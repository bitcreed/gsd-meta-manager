//! The VERIFICATION.md scan: which `gaps_found` files exist on disk right now (D-26).
//!
//! Port of `scan_gaps` from the spike-003 launcher, minus the counter call: the
//! scan only *observes*, callers feed each [`VerificationWrite`] to
//! [`super::counters::observe_gap`]. Counts therefore come only from bytes on
//! disk, never from a value the driver sends.

use std::fs;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::counters::{digest16, fm_status, phase_key};

/// The longest project-relative path that may be written into a LOG header.
const MAX_PATH_CHARS: usize = 200;

/// The most bytes of one VERIFICATION.md the scan will read (CR-01). A larger file
/// is skipped, not truncated: a truncated read would hash differently from the
/// bytes on disk.
pub const MAX_VERIFICATION_BYTES: u64 = 1 << 20;

/// Read one candidate safely: the sandboxed driver can plant anything at this path.
///
/// Only a regular, non-symlink file no larger than [`MAX_VERIFICATION_BYTES`] is
/// read. The open is `O_NONBLOCK | O_NOFOLLOW`, so a FIFO with no writer cannot hang
/// it; the type is re-checked on the open handle (`fstat`), so a swap between the
/// `lstat` and the open is caught; and the read itself is bounded by `take`, so a
/// `/dev/zero`-like file cannot grow the buffer without limit. Returns the bytes
/// with the mtime before and after the read (`None` for anything skipped).
fn read_regular_bounded(path: &Path) -> Option<(Vec<u8>, SystemTime, SystemTime)> {
    let lmeta = fs::symlink_metadata(path).ok()?;
    if !lmeta.file_type().is_file() || lmeta.len() > MAX_VERIFICATION_BYTES {
        return None;
    }
    let flags = rustix::fs::OFlags::NONBLOCK.bits() | rustix::fs::OFlags::NOFOLLOW.bits();
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(flags).ok()?)
        .open(path)
        .ok()?;
    let before = file.metadata().ok()?;
    if !before.file_type().is_file() || before.len() > MAX_VERIFICATION_BYTES {
        return None;
    }
    let mut raw = Vec::new();
    // One byte over the cap proves the file outgrew it while it was read.
    file.take(MAX_VERIFICATION_BYTES + 1)
        .read_to_end(&mut raw)
        .ok()?;
    if raw.len() as u64 > MAX_VERIFICATION_BYTES {
        return None;
    }
    let after = fs::symlink_metadata(path).ok()?;
    if !after.file_type().is_file() {
        return None;
    }
    Some((raw, before.modified().ok()?, after.modified().ok()?))
}

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
/// A file is skipped (never fatal) when it is not a regular file (a FIFO, a
/// device, a symlink: CR-01), is over [`MAX_VERIFICATION_BYTES`], cannot be read, when its mtime moved
/// while it was read (WR-07: stat, read, stat; the next tick retries), when its
/// frontmatter status is not `gaps_found`, when its phase directory has no
/// numeric key, or when its path could not be written as one LOG header value
/// (contains `|`, CR or LF, or exceeds 200 characters).
pub fn scan_verifications(project: &Path) -> Vec<VerificationWrite> {
    let phases = project.join(".planning").join("phases");
    let mut candidates: Vec<(String, std::path::PathBuf, String)> = Vec::new();
    for (dir_name, dir) in visible_children(&phases) {
        // `symlink_metadata`: a symlinked phase directory is not followed.
        if !fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_dir()) {
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
        let Some((raw, m1, m2)) = read_regular_bounded(&path) else {
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

    /// Run `f` on a thread and fail (rather than hang the suite) if it never returns.
    #[cfg(target_os = "linux")]
    fn within_a_generous_bound<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(Duration::from_secs(30))
            .expect("the scan blocked on a planted file")
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn cr01_a_fifo_named_like_a_verification_does_not_hang_the_scan() {
        let td = TempDir::new().unwrap();
        let p = td.path().to_path_buf();
        put(&p, "01-a", "01-VERIFICATION.md", GAPS, T);
        let fifo = p.join(".planning/phases/02-b/02-VERIFICATION.md");
        fs::create_dir_all(fifo.parent().unwrap()).unwrap();
        rustix::fs::mknodat(
            rustix::fs::CWD,
            &fifo,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::from_raw_mode(0o600),
            0,
        )
        .unwrap();
        let scan_root = p.clone();
        let ws = within_a_generous_bound(move || scan_verifications(&scan_root));
        let phases: Vec<&str> = ws.iter().map(|w| w.phase.as_str()).collect();
        assert_eq!(phases, ["1"], "the FIFO is skipped, the regular file kept");
    }

    #[test]
    fn cr01_a_symlinked_verification_is_not_followed() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        let outside = td.path().join("outside.md");
        fs::write(&outside, GAPS).unwrap();
        let dir = p.join(".planning/phases/01-a");
        fs::create_dir_all(&dir).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("01-VERIFICATION.md")).unwrap();
        assert!(scan_verifications(p).is_empty());
    }

    #[test]
    fn cr01_a_symlinked_phase_directory_is_not_followed() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        let real = td.path().join("elsewhere");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("01-VERIFICATION.md"), GAPS).unwrap();
        let phases = p.join(".planning/phases");
        fs::create_dir_all(&phases).unwrap();
        std::os::unix::fs::symlink(&real, phases.join("01-a")).unwrap();
        assert!(scan_verifications(p).is_empty());
    }

    #[test]
    fn cr01_an_oversize_verification_is_skipped_not_read() {
        let td = TempDir::new().unwrap();
        let p = td.path();
        put(p, "01-a", "01-VERIFICATION.md", GAPS, T);
        // Sparse: the frontmatter says gaps_found, the length is one over the cap.
        let big = p.join(".planning/phases/02-b/02-VERIFICATION.md");
        fs::create_dir_all(big.parent().unwrap()).unwrap();
        fs::write(&big, GAPS).unwrap();
        OpenOptions::new()
            .write(true)
            .open(&big)
            .unwrap()
            .set_len(MAX_VERIFICATION_BYTES + 1)
            .unwrap();
        let ws = scan_verifications(p);
        let phases: Vec<&str> = ws.iter().map(|w| w.phase.as_str()).collect();
        assert_eq!(phases, ["1"]);

        // Exactly at the cap is still read.
        OpenOptions::new()
            .write(true)
            .open(&big)
            .unwrap()
            .set_len(MAX_VERIFICATION_BYTES)
            .unwrap();
        let ws = scan_verifications(p);
        assert_eq!(ws.len(), 2, "{ws:?}");
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
