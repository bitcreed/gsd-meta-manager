//! The VERIFICATION.md scan: which `gaps_found` files exist on disk right now (D-26).
//!
//! Port of `scan_gaps` from the spike-003 launcher, minus the counter call: the
//! scan only *observes*, callers feed each [`VerificationWrite`] to
//! [`super::counters::observe_gap`]. Counts therefore come only from bytes on
//! disk, never from a value the driver sends.

use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::counters::{digest16, fm_status, phase_key};

/// The longest project-relative path that may be written into a LOG header.
const MAX_PATH_CHARS: usize = 200;

/// One `gaps_found` VERIFICATION.md as observed on disk.
#[derive(Debug, Clone, PartialEq)]
pub struct VerificationWrite {
    /// Phase key (`1`, `26.1`), from the phase directory name.
    pub phase: String,
    /// File mtime as float seconds since the epoch; it belongs to the hashed bytes.
    pub mtime: f64,
    /// `sha256(raw)[:16]` hex.
    pub digest: String,
    /// Path relative to the project; one LOG-header-safe line.
    pub file: String,
}

fn visible_children(dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, std::path::PathBuf)> = rd
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            // glob `*` does not match a leading dot.
            (!name.starts_with('.')).then(|| (name, e.path()))
        })
        .collect();
    out.sort();
    out
}

/// Scan `<project>/.planning/phases/*/*-VERIFICATION.md`, in sorted order.
///
/// A file is skipped (never fatal) when it cannot be read, when its mtime moved
/// while it was read (WR-07: stat, read, stat; the next tick retries), when its
/// frontmatter status is not `gaps_found`, when its phase directory has no
/// numeric key, or when its path could not be written as one LOG header value
/// (contains `|`, CR or LF, or exceeds 200 characters).
pub fn scan_verifications(project: &Path) -> Vec<VerificationWrite> {
    let phases = project.join(".planning").join("phases");
    let mut candidates: Vec<(String, std::path::PathBuf, String)> = Vec::new();
    for (dir_name, dir) in visible_children(&phases) {
        if !dir.is_dir() {
            continue;
        }
        for (file_name, path) in visible_children(&dir) {
            if !file_name.ends_with("-VERIFICATION.md") {
                continue;
            }
            let Ok(rel) = path.strip_prefix(project) else {
                continue;
            };
            let Some(rel) = rel.to_str() else { continue };
            candidates.push((rel.to_owned(), path, dir_name.clone()));
        }
    }
    // The spike sorted the full glob result as strings.
    candidates.sort();

    let mut out = Vec::new();
    for (rel, path, dir_name) in candidates {
        let Ok(st1) = fs::metadata(&path) else {
            continue;
        };
        let Ok(raw) = fs::read(&path) else { continue };
        let Ok(st2) = fs::metadata(&path) else {
            continue;
        };
        let (Ok(m1), Ok(m2)) = (st1.modified(), st2.modified()) else {
            continue;
        };
        if m1 != m2 {
            continue;
        }
        if fm_status(&raw).as_deref() != Some("gaps_found") {
            continue;
        }
        let Some(phase) = phase_key(&dir_name) else {
            continue;
        };
        if rel.contains(['|', '\r', '\n']) || rel.chars().count() > MAX_PATH_CHARS {
            continue;
        }
        let Ok(since) = m2.duration_since(UNIX_EPOCH) else {
            continue;
        };
        out.push(VerificationWrite {
            phase,
            mtime: since.as_secs_f64(),
            digest: digest16(&raw),
            file: rel,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driverctl::counters::{fm_status, observe_gap, GapKind, GapRoundsState};
    use std::fs::OpenOptions;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    fn at(secs: f64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs_f64(secs)
    }

    /// Write `bytes` to `<project>/.planning/phases/<dir>/<file>` and stamp its mtime.
    fn put(project: &Path, dir: &str, file: &str, bytes: &str, mtime: f64) {
        let d = project.join(".planning").join("phases").join(dir);
        fs::create_dir_all(&d).unwrap();
        let p = d.join(file);
        fs::write(&p, bytes).unwrap();
        OpenOptions::new()
            .write(true)
            .open(&p)
            .unwrap()
            .set_modified(at(mtime))
            .unwrap();
    }

    const T: f64 = 1_000_000.0;
    const GAPS: &str = "---\nstatus: gaps_found\n---\n";

    #[test]
    fn scan_then_observe_counts_coalesces_and_dedups() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        let mut st = GapRoundsState::default();
        let mut feed = |p: &Path| {
            let ws = scan_verifications(p);
            assert_eq!(ws.len(), 1, "{ws:?}");
            let w = &ws[0];
            assert_eq!(w.phase, "1");
            assert_eq!(w.file, ".planning/phases/01-slugify/01-VERIFICATION.md");
            observe_gap(&mut st, &w.phase, w.mtime, &w.digest, Some(&w.file))
        };

        put(
            p,
            "01-slugify",
            "01-VERIFICATION.md",
            &format!("{GAPS}body v1\n"),
            T,
        );
        assert_eq!(feed(p), (GapKind::New, 1));

        // The verifier patches the file 6.5 s later: one round, not two.
        put(
            p,
            "01-slugify",
            "01-VERIFICATION.md",
            &format!("{GAPS}body v2\n"),
            T + 6.5,
        );
        assert_eq!(feed(p), (GapKind::Coalesced, 1));

        // The identical bytes again: a duplicate moves nothing.
        put(
            p,
            "01-slugify",
            "01-VERIFICATION.md",
            &format!("{GAPS}body v2\n"),
            T + 30.0,
        );
        assert_eq!(feed(p), (GapKind::Duplicate, 1));

        // A genuine second round, far from the anchor.
        put(
            p,
            "01-slugify",
            "01-VERIFICATION.md",
            &format!("{GAPS}body v3\n"),
            T + 600.0,
        );
        assert_eq!(feed(p), (GapKind::New, 2));

        let json = serde_json::to_string(&st).unwrap();
        let back: GapRoundsState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, st);
        assert_eq!(back.per_phase.get("1"), Some(&2));
        assert_eq!(back.coalesced.get("1"), Some(&1));
    }

    #[test]
    fn quoted_gaps_found_counts() {
        let td = TempDir::new().unwrap();
        put(
            td.path(),
            "02-x",
            "02-VERIFICATION.md",
            "---\nstatus: \"gaps_found\"\n---\n",
            T,
        );
        let ws = scan_verifications(td.path());
        assert_eq!(ws.len(), 1);
        assert_eq!(ws[0].phase, "2");
    }

    #[test]
    fn passed_and_body_status_are_ignored() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        put(
            p,
            "01-a",
            "01-VERIFICATION.md",
            "---\nstatus: passed\n---\n",
            T,
        );
        put(
            p,
            "02-b",
            "02-VERIFICATION.md",
            "---\nphase: 2\n---\nstatus: gaps_found\n",
            T,
        );
        put(p, "03-c", "03-VERIFICATION.md", "status: gaps_found\n", T);
        assert!(scan_verifications(p).is_empty());
    }

    #[test]
    fn non_numeric_phase_dir_is_skipped_not_fatal() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        put(p, "notes", "x-VERIFICATION.md", GAPS, T);
        put(p, "26.1-driver", "26.1-VERIFICATION.md", GAPS, T);
        let ws = scan_verifications(p);
        assert_eq!(ws.len(), 1, "{ws:?}");
        assert_eq!(ws[0].phase, "26.1");
    }

    #[test]
    fn pipe_in_path_is_skipped() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        put(p, "01-a|b", "01-VERIFICATION.md", GAPS, T);
        put(p, "02-ok", "02-VERIFICATION.md", GAPS, T);
        let ws = scan_verifications(p);
        assert_eq!(ws.len(), 1, "{ws:?}");
        assert_eq!(ws[0].phase, "2");
    }

    #[test]
    fn over_long_path_is_skipped() {
        let td = TempDir::new().unwrap();
        let long = format!("01-{}", "a".repeat(190));
        put(td.path(), &long, "01-VERIFICATION.md", GAPS, T);
        assert!(scan_verifications(td.path()).is_empty());
    }

    #[test]
    fn no_phases_dir_scans_empty() {
        let td = TempDir::new().unwrap();
        assert!(scan_verifications(td.path()).is_empty());
        fs::create_dir_all(td.path().join(".planning/phases/01-a")).unwrap();
        assert!(scan_verifications(td.path()).is_empty());
    }

    #[test]
    fn scan_is_sorted_and_carries_float_mtime() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        put(p, "02-b", "02-VERIFICATION.md", GAPS, T + 5.25);
        put(p, "01-a", "01-VERIFICATION.md", GAPS, T);
        let ws = scan_verifications(p);
        let phases: Vec<&str> = ws.iter().map(|w| w.phase.as_str()).collect();
        assert_eq!(phases, ["1", "2"]);
        assert!((ws[1].mtime - (T + 5.25)).abs() < 1e-6, "{}", ws[1].mtime);
    }

    #[test]
    fn fm_status_rejects_bom_prefixed_text() {
        assert_eq!(
            fm_status(b"\xef\xbb\xbf---\nstatus: gaps_found\n---\n"),
            None
        );
        assert_eq!(fm_status(b"\xff\xfe---\nstatus: gaps_found\n"), None);
        assert_eq!(
            fm_status(b"---\nstatus: gaps_found\n---\n").as_deref(),
            Some("gaps_found")
        );
    }
}
