#![cfg(target_os = "linux")]
// ============================================================================
// Quick 261103-hx7 (D-09, C-32, C-33, I-06, I-07): the data and config roots of the
// application are masked with an empty tmpfs when they live outside `$HOME` and
// `/tmp`. The goldens are pure (no bwrap, no systemd).
// ============================================================================

use std::ffi::OsString;
use std::path::PathBuf;

use gsd_meta_manager::launcher::sandbox::{
    bwrap_argv, bwrap_argv_masking, bwrap_argv_with, masks_outside, LaunchPaths, RunDirMode,
    SandboxSpec,
};

const DATA: &str = "/srv/data/gsd-meta-manager";
const CFG: &str = "/srv/cfg/gsd-meta-manager";

fn spec(project: &str, run: &str) -> SandboxSpec {
    SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: LaunchPaths {
            project: PathBuf::from(project),
            run_dir: PathBuf::from(run),
            config_dir: PathBuf::from(
                "/srv/data/gsd-meta-manager/oneoff/ab12cd34/r1/claude-config",
            ),
            socket: PathBuf::from("/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock"),
            egress_socket: PathBuf::from("/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock"),
        },
        guard_binds: Vec::new(),
        home_binds: Vec::new(),
        gitconfig: None,
        null_masks: Vec::new(),
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        etc_entries: Vec::new(),
    }
}

fn specs() -> Vec<(SandboxSpec, RunDirMode)> {
    vec![
        (
            spec("/work/proj", "/work/proj/.planning/meta-manager/drivers/r1"),
            RunDirMode::DriverReadOnly,
        ),
        (
            spec(
                "/srv/data/gsd-meta-manager/oneoff/ab12cd34/r1/work",
                "/srv/data/gsd-meta-manager/oneoff/ab12cd34/r1/report",
            ),
            RunDirMode::ReportWritable,
        ),
    ]
}

fn strings(argv: &[OsString]) -> Vec<String> {
    argv.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn command() -> Vec<OsString> {
    vec![OsString::from("true")]
}

#[test]
fn no_masks_is_byte_identical_to_the_unmasked_argv_for_both_profiles() {
    for (spec, mode) in specs() {
        assert_eq!(
            bwrap_argv_masking(&spec, mode, &[], &command()),
            bwrap_argv_with(&spec, mode, &command()),
            "{mode:?}"
        );
    }
    let (driver, _) = specs().remove(0);
    assert_eq!(
        bwrap_argv(&driver, &command()),
        bwrap_argv_masking(&driver, RunDirMode::DriverReadOnly, &[], &command())
    );
}

#[test]
fn a_root_outside_home_is_masked_right_before_the_project_bind() {
    let masks = [PathBuf::from(DATA), PathBuf::from(CFG)];
    for (spec, mode) in specs() {
        let plain = strings(&bwrap_argv_with(&spec, mode, &command()));
        let masked = strings(&bwrap_argv_masking(&spec, mode, &masks, &command()));
        let project = spec.paths.project.to_string_lossy().into_owned();
        let at = plain
            .windows(3)
            .position(|w| w[0] == "--bind" && w[1] == project && w[2] == project)
            .unwrap_or_else(|| panic!("the project bind is missing: {plain:?}"));
        let mut expected = plain.clone();
        for (offset, root) in [DATA, CFG].iter().enumerate() {
            expected.insert(at + 2 * offset, "--tmpfs".to_owned());
            expected.insert(at + 2 * offset + 1, (*root).to_owned());
        }
        assert_eq!(masked, expected, "{mode:?}");
        // The binds that re-establish this run's paths follow the masks.
        let mask = masked
            .windows(2)
            .position(|w| w[0] == "--tmpfs" && w[1] == DATA)
            .unwrap();
        let config_bind = masked
            .windows(2)
            .position(|w| w[0] == "--bind" && w[1].ends_with("claude-config"))
            .unwrap();
        assert!(mask < config_bind, "{mode:?}");
    }
}

#[test]
fn masks_outside_keeps_only_roots_that_home_and_tmp_do_not_cover() {
    let home = PathBuf::from("/home/u");
    let candidates = [
        PathBuf::from("/home/u/.local/share/gsd-meta-manager"),
        PathBuf::from("/tmp/x/data/gsd-meta-manager"),
        PathBuf::from(DATA),
        PathBuf::from(CFG),
        PathBuf::from(DATA),
        // A lexical prefix of another path is not under it.
        PathBuf::from("/home/uu/gsd-meta-manager"),
        PathBuf::from("/tmpfoo/gsd-meta-manager"),
    ];
    assert_eq!(
        masks_outside(&home, &candidates),
        [
            PathBuf::from(DATA),
            PathBuf::from(CFG),
            PathBuf::from("/home/uu/gsd-meta-manager"),
            PathBuf::from("/tmpfoo/gsd-meta-manager"),
        ]
    );
    assert!(masks_outside(&home, &[]).is_empty());
}

#[test]
fn masks_outside_never_masks_an_ancestor_of_home_or_tmp() {
    // quick 261113-dck, I-7: a tmpfs over `/`, `/home` or `/tmp` would hide the sandbox's
    // own footing; a relative path cannot be a mount point.
    let home = PathBuf::from("/home/u");
    let candidates = [
        PathBuf::from("/"),
        PathBuf::from("/home"),
        PathBuf::from("/home/u"),
        PathBuf::from("/tmp"),
        PathBuf::from("relative/dir"),
        PathBuf::from(DATA),
    ];
    assert_eq!(masks_outside(&home, &candidates), [PathBuf::from(DATA)]);
}
