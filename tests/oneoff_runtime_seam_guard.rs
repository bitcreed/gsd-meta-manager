//! The one-off runtime seam (quick 261103-f5y, C-10): only `src/oneoff/runtime.rs`
//! may name the agent. Every runtime-neutral file below is scanned; a line whose
//! trimmed start is `//` (prose) is skipped. A listed file that is missing fails the
//! test rather than being skipped, so a rename cannot silently retire the guard.

#![cfg(unix)]

use std::path::Path;

/// The runtime-neutral files.
const NEUTRAL_FILES: &[&str] = &[
    "src/oneoff/mod.rs",
    "src/oneoff/report.rs",
    "src/oneoff/prompt.rs",
    "src/oneoff/todos.rs",
    "src/oneoff/store.rs",
    "src/oneoff/launch.rs",
    "src/oneoff/land.rs",
    "src/oneoff/stream.rs",
    "src/oneoff/session.rs",
    "src/oneoff/inbox.rs",
    "src/ui/screens/oneoff.rs",
];

/// The seam itself.
const SEAM_FILE: &str = "src/oneoff/runtime.rs";

/// The forbidden word, assembled at runtime so this file never matches itself.
fn needle() -> String {
    ["cla", "ude"].concat()
}

fn read(rel: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel} must exist: {e}"))
}

fn offending_lines(text: &str) -> Vec<String> {
    let needle = needle();
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| line.to_ascii_lowercase().contains(&needle))
        .map(|(n, line)| format!("{}: {line}", n + 1))
        .collect()
}

#[test]
fn runtime_neutral_files_never_name_the_agent() {
    let mut problems = Vec::new();
    for rel in NEUTRAL_FILES {
        for line in offending_lines(&read(rel)) {
            problems.push(format!("{rel}:{line}"));
        }
    }
    assert!(
        problems.is_empty(),
        "agent specifics outside {SEAM_FILE}:\n{}",
        problems.join("\n")
    );
}

#[test]
fn positive_control_the_seam_names_it_and_the_scan_would_see_it() {
    let seam = read(SEAM_FILE);
    assert!(seam.contains(&format!("{}_argv", needle())));
    assert!(
        !offending_lines(&seam).is_empty(),
        "the scan finds nothing even in the seam, so it certifies nothing"
    );
    assert!(offending_lines("// a comment naming Claude").is_empty());
}
