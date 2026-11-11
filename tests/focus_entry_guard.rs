// ============================================================================
// The focus-entry audit (debug session `herdr-session-focus-from-tmux`)
//
// The user's rule: focus ALWAYS follows the target session's own multiplexer,
// whatever the TUI's location; the TUI's location only decides where a NEW
// session launches. Two UI paths focus a session:
//
//   * the Sessions panel / main-overview Tab in `src/ui/screens/normal.rs`;
//   * the project tab-switch in `src/ui/screens/detail.rs`.
//
// They must go through the SAME per-session routing function,
// `crate::multiplexer::focus_message`. If either grew its own way to a backend
// (its own `detect_current()` + `focus_with(..)`, its own registry walk), it
// would route from the TUI's environment again and the reported bug would come
// back for that path only, with every test of the other path still green.
//
// The Sessions tab's location column follows the same rule: its resolution
// reaches backends only through `crate::multiplexer::locate_sessions`, called
// from `src/app.rs` on a blocking task; the render path only reads the cache.
//
// Prose cannot enforce that; this file does. Like `spawn_seam_guard.rs` it reads
// the source tree, so it is an integration test and walks `src/` only.
// ============================================================================

use std::path::{Path, PathBuf};

const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// The only files allowed to call the focus entry. Adding a third focus path is
/// a deliberate edit of this list, made in the same commit.
const FOCUS_CALLERS: &[&str] = &["src/ui/screens/detail.rs", "src/ui/screens/normal.rs"];

/// Seam internals no code outside `src/multiplexer/` may call: reaching a
/// backend any other way bypasses the per-session routing.
const SEAM_INTERNALS: &[&str] = &[
    "focus_with",
    "focus_with_procs",
    "focus_message_in",
    "plan_focus",
    "reveal_client",
    "REGISTRY",
    "focus_tty",
    "client_ttys",
    "locate_session",
    "locate_tty",
    "locate_with_procs",
    "locate_sessions_in",
    "reveal_targets",
];

/// The only file allowed to call the location entry: `app.rs` resolves on a
/// blocking task and stores the result for the Sessions tab to read.
const LOCATE_CALLERS: &[&str] = &["src/app.rs"];

type SourceFile = (String, Vec<(usize, String)>);

fn collect(dir: &Path, base: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect(&path, base, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect();
        out.push((relative, lines));
    }
}

fn source_files() -> Vec<SourceFile> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    collect(Path::new(SRC_ROOT), &base, &mut out);
    assert!(
        !out.is_empty(),
        "the audit walked {SRC_ROOT} and found no Rust source at all, which means it is \
         auditing nothing"
    );
    out
}

/// Whether `line` contains `needle` with no identifier character immediately
/// before it, so `REGISTRY` does not match `ABSENT_REGISTRY_OVERLAY` and
/// `focus_with(` does not match `refocus_with(`. `.`, `::` and `(` still count
/// as boundaries. Needles that end in a letter also get a right boundary.
fn contains_token(line: &str, needle: &str) -> bool {
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let needs_right = needle.chars().next_back().is_some_and(is_ident);
    let mut from = 0;
    while let Some(offset) = line[from..].find(needle) {
        let at = from + offset;
        let end = at + needle.len();
        let left_ok = !line[..at].chars().next_back().is_some_and(is_ident)
            || !needle.chars().next().is_some_and(is_ident);
        let right_ok = !needs_right || !line[end..].chars().next().is_some_and(is_ident);
        if left_ok && right_ok {
            return true;
        }
        from = at + needle.len();
    }
    false
}

/// `(path, line number, line)` of every non-comment line containing `needle`.
fn hits(files: &[SourceFile], needle: &str) -> Vec<(String, usize, String)> {
    let mut found = Vec::new();
    for (path, lines) in files {
        for (number, line) in lines {
            if !line.trim_start().starts_with("//") && contains_token(line, needle) {
                found.push((path.clone(), *number, line.trim().to_string()));
            }
        }
    }
    found
}

fn render(hits: &[(String, usize, String)]) -> String {
    hits.iter()
        .map(|(path, number, line)| format!("\n  {path}:{number}: {line}"))
        .collect()
}

/// Both UI paths call the one entry, and no other file does.
#[test]
fn the_sessions_panel_and_the_tab_switch_call_the_same_focus_entry() {
    let files = source_files();
    let callers = hits(&files, "multiplexer::focus_message(");
    let mut paths: Vec<&str> = callers.iter().map(|(path, _, _)| path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(
        paths,
        FOCUS_CALLERS,
        "the files that call crate::multiplexer::focus_message changed. Both focus paths \
         (the Sessions panel and the tab-switch) must call this one function and nobody \
         else may; if a third path is deliberate, add it to FOCUS_CALLERS. Found:{}",
        render(&callers)
    );
}

/// The location column has one resolver entry, called from one file.
#[test]
fn the_location_column_resolves_only_through_locate_sessions() {
    let files = source_files();
    let callers = hits(&files, "multiplexer::locate_sessions(");
    let mut paths: Vec<&str> = callers.iter().map(|(path, _, _)| path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(
        paths,
        LOCATE_CALLERS,
        "the files that call crate::multiplexer::locate_sessions changed. The Sessions tab \
         location column is resolved on a blocking task in app.rs and only READ at render \
         time; if a second resolver is deliberate, add it to LOCATE_CALLERS. Found:{}",
        render(&callers)
    );
}

/// Nothing outside the multiplexer module reaches a backend or the routing
/// internals directly.
#[test]
fn no_ui_code_reaches_a_backend_without_the_per_session_routing() {
    let files = source_files();
    let outside: Vec<SourceFile> = files
        .into_iter()
        .filter(|(path, _)| !path.starts_with("src/multiplexer/"))
        .collect();
    for needle in SEAM_INTERNALS {
        let found = hits(&outside, needle);
        assert!(
            found.is_empty(),
            "{needle} is used outside src/multiplexer/. Focus must reach a backend only \
             through crate::multiplexer::focus_message, which routes by the SESSION's own \
             multiplexer; any other route decides from the TUI's environment and brings back \
             the herdr-session-focus-from-tmux bug for that path:{}",
            render(&found)
        );
    }
}
