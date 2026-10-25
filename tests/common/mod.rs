// ============================================================================
// The shared `git` helper for the integration tests that build throwaway
// repositories (`agents_*`, `registry_worktree_guard`).
//
// `#![allow(dead_code)]` because cargo compiles this module into every
// integration target that declares it, and a helper one target does not happen
// to call is not dead code, it is a helper the other target uses.
// ============================================================================
#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false)
}
