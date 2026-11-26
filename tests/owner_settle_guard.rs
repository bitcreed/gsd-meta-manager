//! The settle modules write nothing and start nothing (39 D-13a, OWN-31, SC5).
//!
//! `x` on a check or a phase launches `/gsd-verify-work N <instruction>`; GSD files
//! the outcome. The claim "the app writes no GSD file" therefore reduces to: the
//! settle modules contain no write API and no process API outside their test code.
//! Launching goes through `crate::multiplexer` only (settle never touches the
//! clipboard, quick 261125-khh); it is not spelled in these files.
//!
//! The scan is a lint, not a proof (a helper elsewhere could still write), but the
//! helper the settle modules call is the launch seam, whose own guard
//! (`tests/spawn_seam_guard.rs`) covers it.
//! Its silence is only meaningful if it read the files, so the executable-line
//! floor and the per-file presence check fail it when it did not.
//!
//! Lines inside any `#[cfg(test)]` item are skipped, as are `//` comments.

use std::path::Path;

/// The files the claim covers.
const SETTLE_FILES: [&str; 3] = [
    "src/ui/screens/owner_settle.rs",
    "src/ui/screens/settle_popup.rs",
    "src/ui/screens/settle_pending.rs",
];

/// Write and spawn markers, by the shape they take in source.
const WRITE_OR_SPAWN: [&str; 15] = [
    "fs::write(",
    "File::create(",
    "OpenOptions",
    "create_dir",
    "fs::rename(",
    "remove_file(",
    "remove_dir",
    "fs::copy(",
    "set_permissions(",
    ".write_all(",
    "Command::new(",
    "std::process",
    ".spawn()",
    ".output()",
    ".status()",
];

/// The fewest executable non-test lines the settle files together must show.
const EXECUTABLE_FLOOR: usize = 400;

/// The executable (non-comment, non-`#[cfg(test)]`) lines of `lines`, with their
/// 1-based numbers.
fn executable_lines(lines: &[String]) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut pending_test_attribute = false;
    let mut skipping = false;
    let mut depth: i32 = 0;
    for (index, raw) in lines.iter().enumerate() {
        let trimmed = raw.trim();
        if skipping {
            depth += brace_delta(raw);
            // A braced item ends where its braces balance; a `;` item (a `use`, a
            // `const`) ends on its own line.
            if depth <= 0 && (trimmed.ends_with('}') || trimmed.ends_with(';')) {
                skipping = false;
                depth = 0;
            }
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed == "#[cfg(test)]" {
            pending_test_attribute = true;
            continue;
        }
        if pending_test_attribute {
            if trimmed.starts_with("#[") || trimmed.is_empty() {
                continue;
            }
            // The item the attribute covers starts on this line.
            pending_test_attribute = false;
            depth = brace_delta(raw);
            let one_liner = depth == 0 && (trimmed.ends_with(';') || trimmed.ends_with('}'));
            skipping = !one_liner;
            continue;
        }
        out.push((index + 1, raw.as_str()));
    }
    out
}

/// Opening minus closing braces on `line`, ignoring braces inside string literals
/// and `'{'` char literals well enough for this code base's formatting.
fn brace_delta(line: &str) -> i32 {
    let mut delta = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut previous = '\0';
    for c in line.chars() {
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
        } else if c == '"' && previous != '\'' {
            in_string = true;
        } else if c == '{' && previous != '\'' {
            delta += 1;
        } else if c == '}' && previous != '\'' {
            delta -= 1;
        }
        previous = c;
    }
    delta
}

/// Every executable line of `lines` that carries a marker, as `path:line: text`.
fn hits(path: &str, lines: &[String]) -> Vec<String> {
    executable_lines(lines)
        .into_iter()
        .filter(|(_, line)| WRITE_OR_SPAWN.iter().any(|marker| line.contains(marker)))
        .map(|(number, line)| format!("{path}:{number}: {}", line.trim()))
        .collect()
}

fn read_settle_file(relative: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()))
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_settle_modules_write_nothing_and_spawn_nothing() {
    let mut found = Vec::new();
    for relative in SETTLE_FILES {
        found.extend(hits(relative, &read_settle_file(relative)));
    }
    assert!(
        found.is_empty(),
        "a settle module writes or spawns; launching goes through `crate::multiplexer` \
         only: {found:#?}"
    );
}

fn planted(line: &str) -> Vec<String> {
    vec![line.to_owned()]
}

#[test]
fn every_marker_is_caught_and_a_comment_is_not() {
    for marker in WRITE_OR_SPAWN {
        let line = format!("    let _ = x.{marker};");
        assert_eq!(
            hits("planted.rs", &planted(&line)).len(),
            1,
            "the marker {marker:?} is not caught"
        );
        let comment = format!("    // let _ = x.{marker};");
        assert!(
            hits("planted.rs", &planted(&comment)).is_empty(),
            "a comment naming {marker:?} is not a use"
        );
    }
    // Test code is skipped: a `#[cfg(test)] mod` and a `#[cfg(test)] fn`, and the
    // production line after them is still scanned.
    let source: Vec<String> = [
        "#[cfg(test)]",
        "mod tests {",
        "    fn t() { std::fs::write(\"a\", \"b\"); }",
        "}",
        "#[cfg(test)]",
        "fn seam() {",
        "    Command::new(\"x\");",
        "}",
        "fn production() { std::process::exit(1); }",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let found = hits("planted.rs", &source);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("planted.rs:9"), "{found:?}");
}

#[test]
fn the_scan_read_both_files() {
    let mut total = 0;
    for relative in SETTLE_FILES {
        let lines = read_settle_file(relative);
        let executable = executable_lines(&lines).len();
        assert!(
            executable > 50,
            "{relative}: only {executable} executable lines were scanned, so its silence means nothing"
        );
        total += executable;
    }
    assert!(
        total > EXECUTABLE_FLOOR,
        "the scan examined only {total} executable lines (floor {EXECUTABLE_FLOOR})"
    );
    eprintln!("owner_settle_guard: {total} executable non-test lines scanned");
}
