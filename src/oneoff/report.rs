//! The agent's report: one validator for the `taskctl report` CLI and for the host
//! (C-12, T-f5y-03).
//!
//! The CLI validates before it writes, and the host re-validates whatever
//! `report.json` holds when it reads it ([`read_validated`]): the file sits in a
//! directory the sandboxed agent can write, so it is never trusted as written.
//!
//! Limits (C-12, I-7): the headline is 1..=120 characters on one line; each
//! `--changed`, `--verified` and `--followup` item is 1..=200 characters on one line;
//! each list holds at most 5 items; a `--verified` item reads `<command>: <result>`;
//! control characters and invisible formatting characters are refused everywhere.
//! Lengths are counted in `char`s.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::state_reader::driver_frontmatter::read_run_file_capped;
use crate::text::carries_invisible_formatting;

use super::REPORT_FILE;

/// The report schema version.
pub const SCHEMA: u32 = 1;
/// The longest headline, in characters.
pub const MAX_HEADLINE: usize = 120;
/// The longest list item, in characters.
pub const MAX_ITEM: usize = 200;
/// The most items one list may hold.
pub const MAX_LIST: usize = 5;

/// The agent's verdict on its own work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    /// The task is done
    Done,
    /// Some of it is done
    Partial,
    /// It could not continue without a human
    Blocked,
    /// It failed
    Failed,
}

impl ReportStatus {
    /// The CLI word.
    pub fn as_str(self) -> &'static str {
        match self {
            ReportStatus::Done => "done",
            ReportStatus::Partial => "partial",
            ReportStatus::Blocked => "blocked",
            ReportStatus::Failed => "failed",
        }
    }
}

/// A validated report. Only [`Report::validate`] builds one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema: u32,
    pub status: ReportStatus,
    pub headline: String,
    pub changed: Vec<String>,
    pub verified: Vec<String>,
    pub followups: Vec<String>,
    /// When the report was written, RFC 3339.
    pub at: String,
}

/// Whether `text` holds a control character or an invisible formatting character.
fn has_bad_chars(text: &str) -> bool {
    text.chars().any(char::is_control) || carries_invisible_formatting(text)
}

/// The problem with one single-line text, if any.
fn check_line(flag: &str, text: &str, max: usize) -> Option<String> {
    let count = text.chars().count();
    if text.trim().is_empty() || count > max || text.contains(['\n', '\r']) {
        return Some(format!(
            "{flag}: must be 1-{max} characters on one line (got {count})"
        ));
    }
    if has_bad_chars(text) {
        return Some(format!(
            "{flag}: must not contain control or invisible formatting characters"
        ));
    }
    None
}

/// Check one list flag's items, pushing every problem.
fn check_list(flag: &str, items: &[String], problems: &mut Vec<String>, verified: bool) {
    if items.len() > MAX_LIST {
        problems.push(format!(
            "{flag}: at most {MAX_LIST} items (got {})",
            items.len()
        ));
    }
    for (i, item) in items.iter().enumerate() {
        let name = format!("{flag} #{}", i + 1);
        if let Some(problem) = check_line(&name, item, MAX_ITEM) {
            problems.push(problem);
            continue;
        }
        if verified {
            let well_formed = item
                .split_once(": ")
                .is_some_and(|(cmd, result)| !cmd.trim().is_empty() && !result.trim().is_empty());
            if !well_formed {
                problems.push(format!(
                    "{name}: must read \"<command>: <result>\" (a \": \" between them)"
                ));
            }
        }
    }
}

impl Report {
    /// Judge the five fields; `Err` carries one line per problem, each naming the
    /// flag and the limit. `at` is now.
    pub fn validate(
        status: ReportStatus,
        headline: &str,
        changed: &[String],
        verified: &[String],
        followups: &[String],
    ) -> Result<Report, Vec<String>> {
        Self::validate_at(
            status,
            headline,
            changed,
            verified,
            followups,
            &chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        )
    }

    /// [`validate`](Self::validate) with a given `at`, which must be RFC 3339.
    pub fn validate_at(
        status: ReportStatus,
        headline: &str,
        changed: &[String],
        verified: &[String],
        followups: &[String],
        at: &str,
    ) -> Result<Report, Vec<String>> {
        let mut problems = Vec::new();
        if let Some(problem) = check_line("--headline", headline, MAX_HEADLINE) {
            problems.push(problem);
        }
        check_list("--changed", changed, &mut problems, false);
        check_list("--verified", verified, &mut problems, true);
        check_list("--followup", followups, &mut problems, false);
        if at.len() > 64 || chrono::DateTime::parse_from_rfc3339(at).is_err() {
            problems.push("at: must be an RFC 3339 time".to_owned());
        }
        if !problems.is_empty() {
            return Err(problems);
        }
        Ok(Report {
            schema: SCHEMA,
            status,
            headline: headline.to_owned(),
            changed: changed.to_vec(),
            verified: verified.to_vec(),
            followups: followups.to_vec(),
            at: at.to_owned(),
        })
    }
}

/// Write `report` to `<dir>/report.json` through a temp file in `dir` and a rename,
/// mode 0600. The rename replaces a planted symlink rather than following it. A second
/// write replaces the first (last write wins).
pub fn write_atomic(dir: &Path, report: &Report) -> io::Result<PathBuf> {
    let rendered = serde_json::to_vec_pretty(report)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(&rendered)?;
    tmp.write_all(b"\n")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o600))?;
    }
    let path = dir.join(REPORT_FILE);
    tmp.persist(&path).map_err(|e| e.error)?;
    Ok(path)
}

/// The shape `report.json` is parsed into before it is judged.
#[derive(Deserialize)]
struct RawReport {
    schema: u32,
    status: ReportStatus,
    headline: String,
    #[serde(default)]
    changed: Vec<String>,
    #[serde(default)]
    verified: Vec<String>,
    #[serde(default)]
    followups: Vec<String>,
    at: String,
}

/// Read `path` capped (1 MiB, symlinks and FIFOs refused) and re-judge it with
/// [`Report::validate_at`]. Anything else (absent, oversized, a symlink, the wrong
/// shape, a field out of bounds) is `Err` with the reason.
pub fn read_validated(path: &Path) -> Result<Report, String> {
    let bytes = read_run_file_capped(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let raw: RawReport = serde_json::from_slice(&bytes)
        .map_err(|e| format!("{}: not a report: {e}", path.display()))?;
    if raw.schema != SCHEMA {
        return Err(format!(
            "{}: schema {} is not {SCHEMA}",
            path.display(),
            raw.schema
        ));
    }
    Report::validate_at(
        raw.status,
        &raw.headline,
        &raw.changed,
        &raw.verified,
        &raw.followups,
        &raw.at,
    )
    .map_err(|problems| format!("{}: {}", path.display(), problems.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn valid() -> Report {
        Report::validate(
            ReportStatus::Done,
            "Fixed the parser",
            &strings(&["src/a.rs", "src/b.rs"]),
            &strings(&["cargo test: 812 passed"]),
            &strings(&["tidy the tests"]),
        )
        .unwrap()
    }

    #[test]
    fn a_valid_report_validates() {
        let report = valid();
        assert_eq!(report.status, ReportStatus::Done);
        assert_eq!(report.changed.len(), 2);
        assert_eq!(report.schema, SCHEMA);
    }

    #[test]
    fn bad_headlines_name_the_flag() {
        let long = "x".repeat(121);
        for headline in ["", long.as_str(), "a\nb", "evil \u{202E}txt"] {
            let problems =
                Report::validate(ReportStatus::Done, headline, &[], &[], &[]).unwrap_err();
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert!(problems[0].starts_with("--headline: "), "{problems:?}");
        }
        let problems = Report::validate(ReportStatus::Done, &long, &[], &[], &[]).unwrap_err();
        assert_eq!(
            problems[0],
            "--headline: must be 1-120 characters on one line (got 121)"
        );
    }

    #[test]
    fn six_changed_items_are_too_many() {
        let six = strings(&["a", "b", "c", "d", "e", "f"]);
        let problems = Report::validate(ReportStatus::Done, "h", &six, &[], &[]).unwrap_err();
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("at most 5"), "{problems:?}");
    }

    #[test]
    fn verified_needs_a_colon_space() {
        let problems =
            Report::validate(ReportStatus::Done, "h", &[], &strings(&["cargo test"]), &[])
                .unwrap_err();
        assert!(problems[0].starts_with("--verified #1: "), "{problems:?}");
    }

    #[test]
    fn a_long_followup_fails() {
        let long = vec!["y".repeat(201)];
        let problems = Report::validate(ReportStatus::Done, "h", &[], &[], &long).unwrap_err();
        assert!(problems[0].starts_with("--followup #1: "), "{problems:?}");
        assert!(problems[0].contains("got 201"));
    }

    #[test]
    fn three_bad_fields_give_three_problems() {
        let problems = Report::validate(
            ReportStatus::Failed,
            "",
            &strings(&["ok", "bad\u{7}"]),
            &strings(&["no colon"]),
            &[],
        )
        .unwrap_err();
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn write_then_read_round_trips_and_a_second_write_replaces_the_first() {
        let dir = TempDir::new().unwrap();
        let first = valid();
        let path = write_atomic(dir.path(), &first).unwrap();
        assert_eq!(read_validated(&path).unwrap(), first);
        let second = Report::validate(ReportStatus::Partial, "Second", &[], &[], &[]).unwrap();
        write_atomic(dir.path(), &second).unwrap();
        assert_eq!(read_validated(&path).unwrap(), second);
    }

    #[test]
    fn a_hand_written_out_of_bounds_report_is_refused() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(REPORT_FILE);
        let headline = "z".repeat(300);
        std::fs::write(
            &path,
            format!(
                r#"{{"schema":1,"status":"done","headline":"{headline}","changed":[],"verified":[],"followups":[],"at":"2026-11-03T10:00:00Z"}}"#
            ),
        )
        .unwrap();
        assert!(read_validated(&path).unwrap_err().contains("--headline"));
    }

    #[test]
    fn a_wrong_shape_is_refused() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(REPORT_FILE);
        std::fs::write(&path, r#"["done", "headline"]"#).unwrap();
        assert!(read_validated(&path).is_err());
        std::fs::write(
            &path,
            r#"{"schema":1,"status":"great","headline":"h","at":"x"}"#,
        )
        .unwrap();
        assert!(read_validated(&path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_report_is_refused() {
        let dir = TempDir::new().unwrap();
        let real = TempDir::new().unwrap();
        let target = write_atomic(real.path(), &valid()).unwrap();
        let link = dir.path().join(REPORT_FILE);
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(read_validated(&link).is_err());
    }
}
