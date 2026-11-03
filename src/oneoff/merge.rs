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

/// The most files the diff summary lists; the rest become a `... N more files` line
/// (C-20, I-03).
pub const DIFF_MAX_FILES: usize = 2000;

/// The numstat read is cut at this many bytes (I-03).
pub const NUMSTAT_MAX_BYTES: usize = 1024 * 1024;

/// One file of `git diff --numstat -z`: lines added and deleted, `None` for a binary
/// file (`-\t-`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumstatRow {
    pub added: Option<u64>,
    pub deleted: Option<u64>,
    pub path: String,
}

/// The complete rows of `git diff --numstat -z --no-renames` output. Each entry is
/// `<added>\t<deleted>\t<path>` and NUL-terminated; a trailing fragment with no NUL
/// (an output cut at a byte cap) is dropped, and so is an entry that does not parse.
pub fn parse_numstat_z(text: &str) -> Vec<NumstatRow> {
    let mut parts: Vec<&str> = text.split('\0').collect();
    parts.pop();
    parts
        .into_iter()
        .filter_map(|entry| {
            let mut fields = entry.splitn(3, '\t');
            let added = fields.next()?;
            let deleted = fields.next()?;
            let path = fields.next()?;
            if path.is_empty() {
                return None;
            }
            let count = |text: &str| -> Option<Option<u64>> {
                if text == "-" {
                    Some(None)
                } else {
                    text.parse().ok().map(Some)
                }
            };
            Some(NumstatRow {
                added: count(added)?,
                deleted: count(deleted)?,
                path: path.to_owned(),
            })
        })
        .collect()
}

/// The totals of `git diff --shortstat`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shortstat {
    pub files: u64,
    pub insertions: u64,
    pub deletions: u64,
}

/// `" 3 files changed, 10 insertions(+), 2 deletions(-)"`, in any of git's forms (no
/// insertions, no deletions, one file, or an empty string for an empty diff).
pub fn parse_shortstat(text: &str) -> Shortstat {
    let mut stat = Shortstat::default();
    for part in text.split(',') {
        let part = part.trim();
        let Some((count, rest)) = part.split_once(' ') else {
            continue;
        };
        let Ok(count) = count.parse::<u64>() else {
            continue;
        };
        if rest.starts_with("file") {
            stat.files = count;
        } else if rest.starts_with("insertion") {
            stat.insertions = count;
        } else if rest.starts_with("deletion") {
            stat.deletions = count;
        }
    }
    stat
}

/// The lines the merge-diff popup shows (klf, C-20, I-03): a `conflicted: ...`
/// header; a per-file summary (`+12 -3  path`, `bin  path` for a binary file) of up
/// to [`DIFF_MAX_FILES`] files with a `... N more files` line; a blank line; the
/// hunks; and, when the hunks were cut, a closing `truncated — N more file(s), ~M
/// more line(s)` marker computed from the shortstat totals minus what was shown.
///
/// The file names are control-cleaned like the recorded ones; the hunk lines are cut
/// by length only, because the screen escapes whatever it draws. `hunks_truncated`
/// says the hunk text ends at a byte or line cap, so an unterminated last line is a
/// fragment and is dropped.
pub fn diff_view(
    conflicted: &[String],
    shortstat: &Shortstat,
    rows: &[NumstatRow],
    rows_truncated: bool,
    hunks: &str,
    hunks_truncated: bool,
) -> Vec<String> {
    let mut lines = vec![if conflicted.is_empty() {
        "conflicted: none (a clean merge)".to_owned()
    } else {
        format!("conflicted: {}", recorded_names(conflicted).join(", "))
    }];
    for row in rows.iter().take(DIFF_MAX_FILES) {
        let path = clean_text(&row.path, MAX_CONFLICTED_NAME);
        lines.push(match (row.added, row.deleted) {
            (Some(a), Some(d)) => format!("+{a} -{d}  {path}"),
            _ => format!("bin  {path}"),
        });
    }
    let listed = rows.len().min(DIFF_MAX_FILES) as u64;
    if rows.len() > DIFF_MAX_FILES || rows_truncated {
        let more = shortstat.files.saturating_sub(listed);
        lines.push(if more > 0 {
            format!("... {more} more files")
        } else {
            "... more files".to_owned()
        });
    }
    lines.push(String::new());

    let text = if hunks_truncated && !hunks.ends_with('\n') {
        // The cut fell inside a line: that line is a fragment.
        hunks.rfind('\n').map_or("", |at| &hunks[..=at])
    } else {
        hunks
    };
    let mut files_shown = 0u64;
    let mut body_lines = 0u64;
    let mut in_header = false;
    let mut kept = 0usize;
    for line in text.lines() {
        if kept >= DIFF_MAX_LINES {
            break;
        }
        if line.starts_with("diff --git ") {
            files_shown += 1;
            in_header = true;
        } else if line.starts_with("@@") {
            in_header = false;
        } else if !in_header && (line.starts_with('+') || line.starts_with('-')) {
            body_lines += 1;
        }
        lines.push(line.chars().take(DIFF_MAX_LINE_CHARS).collect());
        kept += 1;
    }
    if hunks_truncated {
        let total_files = shortstat.files.max(rows.len() as u64);
        let more_files = total_files.saturating_sub(files_shown);
        let more_lines = (shortstat.insertions + shortstat.deletions).saturating_sub(body_lines);
        lines.push(format!(
            "truncated — {more_files} more file{}, ~{more_lines} more line{}",
            if more_files == 1 { "" } else { "s" },
            if more_lines == 1 { "" } else { "s" },
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
    fn numstat_rows_parse_text_binary_and_drop_a_cut_fragment() {
        let text = "3\t1\ta.txt\0-\t-\timg.png\0\t\0 5\t2\tcut";
        let rows = parse_numstat_z(text);
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert_eq!(rows[0].added, Some(3));
        assert_eq!(rows[0].deleted, Some(1));
        assert_eq!(rows[0].path, "a.txt");
        assert_eq!(rows[1].added, None);
        assert_eq!(rows[1].path, "img.png");
        assert!(parse_numstat_z("").is_empty());
        let spaced = parse_numstat_z("1\t0\ta b\tc\0");
        assert_eq!(spaced[0].path, "a b\tc");
    }

    #[test]
    fn shortstat_parses_every_form() {
        assert_eq!(
            parse_shortstat(" 1 file changed, 3 insertions(+)\n"),
            Shortstat {
                files: 1,
                insertions: 3,
                deletions: 0
            }
        );
        assert_eq!(
            parse_shortstat(" 2 files changed, 1 deletion(-)"),
            Shortstat {
                files: 2,
                insertions: 0,
                deletions: 1
            }
        );
        assert_eq!(
            parse_shortstat(" 4 files changed, 10 insertions(+), 2 deletions(-)"),
            Shortstat {
                files: 4,
                insertions: 10,
                deletions: 2
            }
        );
        assert_eq!(parse_shortstat(""), Shortstat::default());
    }

    fn row(a: u64, d: u64, path: &str) -> NumstatRow {
        NumstatRow {
            added: Some(a),
            deleted: Some(d),
            path: path.to_owned(),
        }
    }

    #[test]
    fn the_view_is_header_summary_blank_then_hunks() {
        let stat = Shortstat {
            files: 1,
            insertions: 1,
            deletions: 0,
        };
        let view = diff_view(
            &["a.txt".to_owned()],
            &stat,
            &[row(1, 0, "a.txt")],
            false,
            "diff --git a b\n+x\n",
            false,
        );
        assert_eq!(view[0], "conflicted: a.txt");
        assert_eq!(view[1], "+1 -0  a.txt");
        assert_eq!(view[2], "");
        assert_eq!(view[3], "diff --git a b");
        assert_eq!(view.len(), 5, "no marker when nothing was cut: {view:?}");
        let bin = diff_view(
            &[],
            &stat,
            &[NumstatRow {
                added: None,
                deleted: None,
                path: "i\u{1b}.png".to_owned(),
            }],
            false,
            "x\n",
            false,
        );
        assert_eq!(bin[0], "conflicted: none (a clean merge)");
        assert_eq!(bin[1], "bin  i .png");
    }

    #[test]
    fn a_cut_view_ends_with_the_remaining_file_and_line_counts() {
        let stat = Shortstat {
            files: 2,
            insertions: 100,
            deletions: 0,
        };
        let mut hunks = String::from("diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -0,0 +1,3 @@\n");
        hunks.push_str("+one\n+two\n+three\n+frag");
        let view = diff_view(&[], &stat, &[], false, &hunks, true);
        let last = view.last().unwrap();
        assert_eq!(last, "truncated — 1 more file, ~97 more lines", "{view:?}");
        assert!(
            !view.iter().any(|l| l == "+frag"),
            "the fragment is dropped"
        );
        // The `--- a/a` / `+++ b/a` header lines are not body lines.
        let many_files = Shortstat {
            files: 2501,
            insertions: 0,
            deletions: 0,
        };
        let rows: Vec<NumstatRow> = (0..2500).map(|i| row(0, 0, &format!("f{i}"))).collect();
        let view = diff_view(&[], &many_files, &rows, false, "", false);
        assert!(view.contains(&"... 501 more files".to_owned()), "{view:?}");
        assert_eq!(view.iter().filter(|l| l.starts_with("+0 -0")).count(), 2000);
    }

    #[test]
    fn the_hunks_are_capped_by_lines_and_long_lines_are_cut() {
        let many = "+x\n".repeat(DIFF_MAX_LINES + 10);
        let stat = Shortstat {
            files: 1,
            insertions: 20000,
            deletions: 0,
        };
        let capped = diff_view(&[], &stat, &[], false, &many, true);
        assert_eq!(capped.len(), 2 + DIFF_MAX_LINES + 1);
        assert!(capped.last().unwrap().starts_with("truncated — "));
        let wide = format!("+{}\n", "y".repeat(DIFF_MAX_BYTES));
        let cut = diff_view(&[], &Shortstat::default(), &[], false, &wide, false);
        assert_eq!(cut[2].chars().count(), DIFF_MAX_LINE_CHARS);
        let empty = diff_view(&[], &Shortstat::default(), &[], false, "", false);
        assert_eq!(
            empty.last().unwrap(),
            "(the merge changed nothing against its first parent)"
        );
    }
}
