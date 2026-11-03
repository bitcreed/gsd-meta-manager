//! The pure half of a merge landing (quick 261103-hx7, D-04, D-05, C-09..C-25).
//!
//! The I/O (clone, merge, the resolver session, the final fast-forward) is
//! [`super::resolve`]; everything it decides from plain values lives here so it can be
//! unit-tested without git: the resolver's budget, the conflict-marker scan, the parent
//! check of a resolved merge commit, the test command read from the project's config and
//! the merge commit's subject. Nothing here names the agent.

/// A landing is attempted at most this many times in total: the target moved under it
/// each time but the last (C-18).
pub const MAX_LAND_ATTEMPTS: u32 = 3;

/// The resolver's wall clock per attempt, in minutes (C-24).
pub const RESOLVER_CAP_MINUTES: f64 = 15.0;

/// Below this many remaining dollars the resolver is not started (C-22, I-17).
pub const MIN_RESOLVER_BUDGET_USD: f64 = 0.25;

/// The most conflicted file names recorded and shown (I-19).
pub const MAX_CONFLICTED: usize = 50;

/// The longest recorded conflicted file name, in characters (I-19).
pub const MAX_CONFLICTED_NAME: usize = 512;

/// A conflicted file is scanned for markers up to this many bytes (I-16).
pub const MARKER_READ_CAP: u64 = 4 * 1024 * 1024;

/// The merge diff the TUI shows is cut at this many lines (I-22).
pub const DIFF_MAX_LINES: usize = 5000;

/// ... and at this many bytes (I-22).
pub const DIFF_MAX_BYTES: usize = 512 * 1024;

/// The longest headline kept in a merge commit's subject, in characters.
const MAX_SUBJECT_HEADLINE: usize = 120;

/// The longest `workflow.test_command` the host passes on, in characters (I-20).
const MAX_TEST_COMMAND: usize = 200;

/// The dollars the resolver may spend: the run's budget minus what the run already
/// spent (C-22). Refuses when less than [`MIN_RESOLVER_BUDGET_USD`] is left.
pub fn resolver_budget(max_budget_usd: f64, spent_usd: f64) -> Result<f64, String> {
    let remaining = max_budget_usd - spent_usd.max(0.0);
    if remaining.is_finite() && remaining >= MIN_RESOLVER_BUDGET_USD {
        Ok(remaining)
    } else {
        Err(format!(
            "no budget left to resolve conflicts (${:.2} of ${max_budget_usd:.2} left; \
             at least ${MIN_RESOLVER_BUDGET_USD:.2} is needed)",
            remaining.max(0.0)
        ))
    }
}

/// Whether `text` still holds a conflict marker (I-16): a line that starts with exactly
/// seven `<` or seven `>` followed by a space or the end of the line. A bare `=======`
/// is a legal markdown setext underline and never counts.
pub fn has_conflict_markers(text: &str) -> bool {
    text.lines().any(|line| {
        ["<<<<<<<", ">>>>>>>"].iter().any(|marker| {
            line.strip_prefix(marker)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
        })
    })
}

/// Whether `parents_line` (`git rev-list --parents -n 1 HEAD`: the commit, then its
/// parents) names exactly `[first, second]` in that order (C-15).
pub fn merge_parents_ok(parents_line: &str, first: &str, second: &str) -> bool {
    let mut fields = parents_line.split_whitespace();
    let _commit = fields.next();
    let parents: Vec<&str> = fields.collect();
    parents == [first, second]
}

/// `workflow.test_command` of a `.planning/config.json` text, only when it is a string
/// of 1..=200 characters with no control character (I-20).
pub fn test_command_from(config_json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(config_json).ok()?;
    let command = value.get("workflow")?.get("test_command")?.as_str()?;
    let count = command.chars().count();
    let usable = (1..=MAX_TEST_COMMAND).contains(&count)
        && !command.trim().is_empty()
        && !command.chars().any(char::is_control)
        && !crate::text::carries_invisible_formatting(command);
    usable.then(|| command.to_owned())
}

/// The distinct paths of `git ls-files -u -z` output (each entry reads
/// `<mode> <sha> <stage>\t<path>`, NUL-terminated; a conflicted path has up to three
/// stages), in first-seen order. The path follows a tab, so the host's trimming of git's
/// output can never eat a path's own leading or trailing blank.
pub fn parse_unmerged(ls_files_z: &str) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for entry in ls_files_z.split('\0') {
        if let Some((_, path)) = entry.split_once('\t') {
            if !path.is_empty() && !seen.iter().any(|p| p == path) {
                seen.push(path.to_owned());
            }
        }
    }
    seen
}

/// `text` with control characters replaced by spaces and cut to `max` characters.
pub fn clean_text(text: &str, max: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}

/// The merge commit's subject, `Merge one-off <run-id> (<headline>)` (C-12). The
/// headline is the run's, control-cleaned and cut.
pub fn merge_message(run_id: &str, headline: &str) -> String {
    let headline = clean_text(headline, MAX_SUBJECT_HEADLINE);
    let headline = headline.trim();
    if headline.is_empty() {
        format!("Merge one-off {run_id}")
    } else {
        format!("Merge one-off {run_id} ({headline})")
    }
}

/// The conflicted file names as they are recorded and shown: control-cleaned, each cut
/// to [`MAX_CONFLICTED_NAME`] characters, at most [`MAX_CONFLICTED`] of them (I-19).
pub fn recorded_names(names: &[String]) -> Vec<String> {
    names
        .iter()
        .take(MAX_CONFLICTED)
        .map(|n| clean_text(n, MAX_CONFLICTED_NAME))
        .collect()
}

/// The longest drawn diff line, in characters. Longer lines are cut, not dropped.
pub const DIFF_MAX_LINE_CHARS: usize = 1000;

/// The lines the merge-diff popup shows (quick 261103-hx7, D-06, I-22): a
/// `conflicted: ...` header, a blank line, then `diff` capped at [`DIFF_MAX_LINES`]
/// lines and [`DIFF_MAX_BYTES`] bytes with a closing `... diff truncated` line when it
/// was cut. The file names are control-cleaned like the recorded ones; the diff lines
/// are cut by length only, because the screen escapes whatever it draws.
pub fn diff_lines(conflicted: &[String], diff: &str) -> Vec<String> {
    let mut lines = vec![
        if conflicted.is_empty() {
            "conflicted: none (a clean merge)".to_owned()
        } else {
            format!("conflicted: {}", recorded_names(conflicted).join(", "))
        },
        String::new(),
    ];
    let mut bytes = 0usize;
    let mut truncated = false;
    let mut kept = 0usize;
    for line in diff.lines() {
        let line: String = line.chars().take(DIFF_MAX_LINE_CHARS).collect();
        bytes = bytes.saturating_add(line.len() + 1);
        if kept >= DIFF_MAX_LINES || bytes > DIFF_MAX_BYTES {
            truncated = true;
            break;
        }
        lines.push(line);
        kept += 1;
    }
    if truncated {
        lines.push(format!(
            "... diff truncated (the first {kept} lines are shown; run git diff <merge>^1 <merge> for all of it)"
        ));
    } else if kept == 0 {
        lines.push("(the merge changed nothing against its first parent)".to_owned());
    }
    lines
}

/// The success line of a merge landing (C-21).
pub fn landed_line(
    run_id: &str,
    branch: &str,
    merge_sha: &str,
    resolved: bool,
    conflicted: &[String],
) -> String {
    let short = &merge_sha[..merge_sha.len().min(12)];
    if resolved {
        format!(
            "merged {run_id} into {branch} as {short} (resolved by agent; conflicted: {})",
            conflicted.join(", ")
        )
    } else {
        format!("merged {run_id} into {branch} as {short} (merge commit)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_resolver_gets_the_remaining_budget_or_is_refused() {
        assert_eq!(resolver_budget(10.0, 1.5), Ok(8.5));
        for (max, spent) in [(10.0, 9.8), (0.2, 0.0), (10.0, 10.0), (10.0, 12.0)] {
            let why = resolver_budget(max, spent).unwrap_err();
            assert!(why.contains("no budget left to resolve conflicts"), "{why}");
        }
        assert_eq!(
            resolver_budget(0.25, 0.0),
            Ok(0.25),
            "the floor is inclusive"
        );
        assert!(resolver_budget(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn only_a_real_conflict_marker_counts() {
        assert!(has_conflict_markers(
            "a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> b\n"
        ));
        assert!(has_conflict_markers(">>>>>>>\n"));
        assert!(has_conflict_markers("<<<<<<< "));
        assert!(!has_conflict_markers("Title\n=======\n"));
        assert!(!has_conflict_markers("x <<<<<<< y"));
        assert!(!has_conflict_markers("<<<<<<<< eight\n"));
        assert!(!has_conflict_markers("<<<<<<<x\n"));
        assert!(!has_conflict_markers(""));
    }

    #[test]
    fn a_resolved_merge_has_exactly_the_target_then_the_run_head() {
        assert!(merge_parents_ok("m t h", "t", "h"));
        for bad in ["m h t", "m t", "m t h x", "m", "", "m t t"] {
            assert!(!merge_parents_ok(bad, "t", "h"), "{bad:?}");
        }
    }

    #[test]
    fn the_test_command_is_a_short_clean_string() {
        let ok = r#"{"workflow":{"test_command":"cargo test"}}"#;
        assert_eq!(test_command_from(ok).as_deref(), Some("cargo test"));
        let long = format!(r#"{{"workflow":{{"test_command":"{}"}}}}"#, "x".repeat(201));
        let just = format!(r#"{{"workflow":{{"test_command":"{}"}}}}"#, "x".repeat(200));
        assert_eq!(test_command_from(&long), None);
        assert!(test_command_from(&just).is_some());
        for bad in [
            r#"{"workflow":{"test_command":""}}"#,
            r#"{"workflow":{"test_command":"a\nb"}}"#,
            r#"{"workflow":{"test_command":"a\u001bb"}}"#,
            r#"{"workflow":{"test_command":5}}"#,
            r#"{"workflow":{}}"#,
            r#"{}"#,
            "not json",
        ] {
            assert_eq!(test_command_from(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_merge_subject_names_the_run_and_a_clean_headline() {
        assert_eq!(
            merge_message("oneoff-x", "Fix a\tb"),
            "Merge one-off oneoff-x (Fix a b)"
        );
        assert_eq!(merge_message("oneoff-x", "  "), "Merge one-off oneoff-x");
        let long = merge_message("oneoff-x", &"h".repeat(500));
        assert_eq!(
            long.chars().count(),
            "Merge one-off oneoff-x ()".len() + 120
        );
    }

    #[test]
    fn recorded_names_are_capped_and_clean() {
        let many: Vec<String> = (0..80).map(|i| format!("f{i}\u{7}")).collect();
        let names = recorded_names(&many);
        assert_eq!(names.len(), MAX_CONFLICTED);
        assert_eq!(names[0], "f0 ");
        let long = recorded_names(&["a".repeat(900)]);
        assert_eq!(long[0].chars().count(), MAX_CONFLICTED_NAME);
    }

    #[test]
    fn the_success_line_says_how_it_landed() {
        let sha = "0123456789abcdef0123";
        assert_eq!(
            landed_line("r", "main", sha, false, &[]),
            "merged r into main as 0123456789ab (merge commit)"
        );
        let line = landed_line("r", "main", sha, true, &["a".into(), "b".into()]);
        assert!(
            line.contains("resolved by agent; conflicted: a, b"),
            "{line}"
        );
    }

    #[test]
    fn unmerged_paths_are_distinct_and_keep_their_blanks() {
        let out =
            "100644 aaa 1\t shared.txt\x00100644 bbb 2\t shared.txt\x00100644 ccc 3\t shared.txt\0\
                   100644 ddd 2\ta b.md\0";
        assert_eq!(parse_unmerged(out), [" shared.txt", "a b.md"]);
        assert!(parse_unmerged("").is_empty());
    }
    #[test]
    fn the_diff_is_headed_by_the_conflicted_files_and_capped() {
        let small = diff_lines(&["a.txt".to_owned()], "diff --git a b\n+x\n");
        assert_eq!(small[0], "conflicted: a.txt");
        assert_eq!(small[1], "");
        assert_eq!(small[2], "diff --git a b");
        assert_eq!(small.len(), 4);
        assert_eq!(diff_lines(&[], "+x")[0], "conflicted: none (a clean merge)");

        let many = "+x\n".repeat(DIFF_MAX_LINES + 10);
        let capped = diff_lines(&[], &many);
        assert_eq!(capped.len(), 2 + DIFF_MAX_LINES + 1);
        assert!(capped.last().unwrap().starts_with("... diff truncated"));

        let wide = format!("+{}\n", "y".repeat(DIFF_MAX_BYTES));
        let cut = diff_lines(&[], &wide);
        assert_eq!(cut[2].chars().count(), DIFF_MAX_LINE_CHARS);

        let bytes = format!("+{}\n", "z".repeat(900)).repeat(DIFF_MAX_BYTES / 900 + 5);
        let by_size = diff_lines(&[], &bytes);
        assert!(by_size.last().unwrap().starts_with("... diff truncated"));
        assert!(by_size.len() < DIFF_MAX_LINES);
    }
}
