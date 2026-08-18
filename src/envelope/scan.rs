//! The full-worktree credential scan (SAFE-03, D-11..D-15).
//!
//! ## Why the walk is hand-rolled and reads everything
//!
//! The obvious backstop — ask git which files are new or changed and scan those
//! — has a named blind spot, and it is precisely the one that matters: git's
//! answer honours the repository's ignore rules, so a credential written to
//! `secrets/`, to `.env`, or to anything matching `*.local` is **never seen**.
//! Those are the paths a secret is *most* likely to be written to. So this walk
//! starts at the repository root, consults no ignore rules at all, and invokes
//! no git plumbing to decide what to read. The only directory it declines to
//! descend into is `.git` itself, whose contents are the object database rather
//! than the worktree.
//!
//! ## Why the scanner is built in (D-11)
//!
//! `gitleaks` is not installed on the development machine, is not a Rust crate,
//! and is not something this project installs on a user's behalf. A push gate
//! that depends on an absent external binary either **fails open** — the worst
//! possible failure for this control — or makes the whole phase unverifiable.
//! So the built-in rules always run, and they are not a second copy of anything:
//! they are the `Credential`-tagged subset of the one already-shipped and
//! already-tested table in [`crate::journal::redact`], reached through
//! [`crate::journal::redact::credential_alternation`]. If a `gitleaks` binary
//! *is* resolvable, it runs **as well** and its non-zero exit is an additional
//! block. Its absence is never a reason to allow, and it is never a substitute.
//!
//! ## What a report may contain, and what it may never contain (D-15)
//!
//! A [`Finding`] carries the file, the line number and the rule name. There is
//! **no field for the matched text**, by construction rather than by discipline.
//! Printing the secret into stderr and the run journal to explain why the secret
//! was blocked is the redact-at-capture bug committed a second time, by the code
//! whose entire job is to prevent it.
//!
//! ## Why the skip list is part of the verdict (D-14)
//!
//! A scanner that reports "clean" while it declined to read forty files is a
//! scanner that lies, and this whole phase is an argument against exactly that.
//! The skip list and the verdict are **one structure**, and [`ScanReport::render`]
//! prints the skip list on **both** outcomes. The nearest existing posture in
//! this codebase is `PushPreview.note`, where "nothing to report" is data carried
//! in the same struct and always printed.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::journal::redact;

/// The directory whose contents are the object database, not the worktree.
const GIT_DIR: &str = ".git";

/// How much of a file's head is probed for a NUL byte before it is called
/// binary. 8 KiB is the size at which git itself makes the same judgement.
const NUL_PROBE_BYTES: usize = 8 * 1024;

/// The rule name a `gitleaks` block is folded into the report under.
///
/// Distinct from every built-in rule name, so a reader can tell which scanner
/// fired without the report having to explain itself.
pub const GITLEAKS_RULE: &str = "gitleaks-external";

/// The external scanner this project deliberately does not depend on.
const GITLEAKS_BIN: &str = "gitleaks";

/// One credential-shaped match: **where**, and **which rule**. Never what.
///
/// There is deliberately no field for the matched text (D-15). A struct with
/// nowhere to put the secret cannot be made to print it by a later edit that
/// only wanted to make the message more helpful — which is the exact shape of
/// the change that would reintroduce the bug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The offending file, relative to the scanned root.
    pub path: PathBuf,
    /// 1-based line number of the match.
    pub line: u32,
    /// The `PARTS` rule name that fired, or [`GITLEAKS_RULE`].
    pub rule: &'static str,
}

/// Why the scan declined to read a path.
///
/// Every variant is **reported**, never silently dropped (D-14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// Larger than [`ScanLimits::max_file_bytes`].
    TooLarge {
        /// The file's size, so a human can judge whether the cap is wrong.
        bytes: u64,
    },
    /// A NUL byte appeared in the first [`NUL_PROBE_BYTES`] bytes.
    Binary,
    /// An I/O or permission error, or a path that is not a regular file.
    Unreadable {
        /// The OS detail. Never file content.
        detail: String,
    },
    /// A symlink, which the walk records rather than follows.
    Symlink,
    /// [`ScanLimits::max_total_bytes`] was already spent when this file came up.
    BudgetExhausted,
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::TooLarge { bytes } => write!(f, "too large ({bytes} bytes)"),
            SkipReason::Binary => write!(f, "binary (NUL byte in the first 8 KiB)"),
            SkipReason::Unreadable { detail } => write!(f, "unreadable ({detail})"),
            SkipReason::Symlink => write!(f, "symlink (not followed)"),
            SkipReason::BudgetExhausted => write!(f, "total byte budget exhausted"),
        }
    }
}

/// What became of the optional external scanner (D-11).
///
/// Recorded rather than inferred, because "the external scanner did not fire" and
/// "the external scanner was never there" are different facts and only one of
/// them is reassuring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalScanner {
    /// This scan did not attempt it. [`scan_worktree`] never does.
    NotAttempted,
    /// No such binary is resolvable. **Never a reason to allow.**
    Absent,
    /// It ran and found nothing.
    Clean,
    /// It ran and exited non-zero, which is folded in as a [`GITLEAKS_RULE`]
    /// finding.
    Blocked,
    /// It exists but could not be run, which is treated as a block: a scanner
    /// that could not complete must not be reported as a clean result.
    Failed {
        /// The OS detail. Deliberately never the tool's own output, which can
        /// carry the secret it found.
        detail: String,
    },
}

/// The block/allow decision and everything the scan declined to read, as one
/// structure (D-14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReport {
    /// Every credential-shaped match. Non-empty means block.
    pub findings: Vec<Finding>,
    /// Every path the scan declined to read, and why.
    pub skipped: Vec<(PathBuf, SkipReason)>,
    /// How many files were actually read.
    pub files_scanned: u32,
    /// How many bytes were actually read.
    pub bytes_scanned: u64,
    /// What became of the optional external scanner.
    pub external: ExternalScanner,
    /// The scan could not read its own root, so it never ran at all.
    ///
    /// Separate from [`ScanReport::skipped`] because it is a different kind of
    /// fact: a skipped *file* is a completed scan with a disclosed gap, whereas
    /// an unreadable *root* is a scan that did not happen — and only the second
    /// one must be prevented from reading as a clean result.
    pub root_unreadable: bool,
}

impl Default for ScanReport {
    fn default() -> Self {
        ScanReport {
            findings: Vec::new(),
            skipped: Vec::new(),
            files_scanned: 0,
            bytes_scanned: 0,
            external: ExternalScanner::NotAttempted,
            root_unreadable: false,
        }
    }
}

impl ScanReport {
    /// Whether the push may proceed as far as **this** control is concerned.
    ///
    /// **Not simply "no findings".** A scan whose root could not be read found
    /// nothing for the same reason a scan that never ran finds nothing, and
    /// reporting that as clean is the fail-open this control exists to refuse.
    /// An individual file the scan declined to read is a different case: it is
    /// disclosed in the skip list and does not block, because a scanner that
    /// blocked every push over one permission-denied file is a scanner that
    /// gets switched off.
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty() && !self.root_unreadable
    }

    /// The whole report as text, with the skip list printed on **both**
    /// outcomes (D-14).
    ///
    /// It cannot contain a matched secret: [`Finding`] has nowhere to hold one,
    /// and [`ExternalScanner`] deliberately never carries the external tool's
    /// output.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("gsd-meta-manager envelope: worktree credential scan\n");
        out.push_str(&format!(
            "  read:     {} files, {} bytes\n",
            self.files_scanned, self.bytes_scanned
        ));

        if self.root_unreadable {
            out.push_str(
                "  VERDICT:  the scan could not read its own root, so it never ran — \
                 refusing rather than reporting clean\n",
            );
        }

        if self.findings.is_empty() {
            out.push_str("  findings: none\n");
        } else {
            out.push_str(&format!("  findings: {}\n", self.findings.len()));
            for finding in &self.findings {
                // File, line, rule — and never the matched text (D-15).
                out.push_str(&format!(
                    "    {}:{}  rule={}\n",
                    finding.path.display(),
                    finding.line,
                    finding.rule
                ));
            }
        }

        out.push_str(&format!("  external: {}\n", self.external_note()));

        // Printed on a clean report too. A scan that declined forty files and
        // said "clean" would be lying by omission.
        if self.skipped.is_empty() {
            out.push_str("  declined to read: nothing\n");
        } else {
            out.push_str(&format!("  declined to read: {} paths\n", self.skipped.len()));
            for (path, reason) in &self.skipped {
                out.push_str(&format!("    {}  {}\n", path.display(), reason));
            }
        }

        out
    }

    fn external_note(&self) -> String {
        match &self.external {
            ExternalScanner::NotAttempted => {
                format!("{GITLEAKS_BIN} not attempted by this entry point")
            }
            ExternalScanner::Absent => format!(
                "{GITLEAKS_BIN} not on PATH — additive only, and its absence is \
                 never a reason to allow"
            ),
            ExternalScanner::Clean => format!("{GITLEAKS_BIN} ran and found nothing"),
            ExternalScanner::Blocked => {
                format!("{GITLEAKS_BIN} exited non-zero — blocking (its own output is \
                         deliberately not reproduced here)")
            }
            ExternalScanner::Failed { detail } => {
                format!("{GITLEAKS_BIN} could not be run ({detail}) — treated as a block")
            }
        }
    }

    fn skip(&mut self, path: PathBuf, reason: SkipReason) {
        self.skipped.push((path, reason));
    }
}

/// The explicit caps, because an unbounded walk on a large repository is a
/// denial of service against the push it is supposed to protect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    /// Per-file cap. Above this the file is [`SkipReason::TooLarge`].
    pub max_file_bytes: u64,
    /// Whole-scan cap. Once spent, every remaining file is
    /// [`SkipReason::BudgetExhausted`].
    pub max_total_bytes: u64,
}

impl Default for ScanLimits {
    fn default() -> Self {
        ScanLimits {
            // 1 MiB. A source file larger than this is a data blob, a minified
            // bundle or a checked-in artifact; a credential shape inside one is
            // possible, so it is reported as skipped rather than pretended away.
            max_file_bytes: 1024 * 1024,
            // 64 MiB. Large enough that an ordinary repository is scanned in
            // full, small enough that a repository carrying a multi-gigabyte
            // dataset does not turn every push into a minutes-long stall — which
            // is how a control gets switched off.
            max_total_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Read every file the walk rules permit and report what was found and what was
/// declined.
///
/// Never panics and never returns an error: an unreadable path is **data** in
/// the report, following `git_ops`' failure-as-data posture. An error return
/// would tempt a caller into treating "the scan broke" as "the scan is not my
/// problem", and that is a fail-open.
pub fn scan_worktree(root: &Path, limits: ScanLimits) -> ScanReport {
    let mut report = ScanReport::default();
    let alternation = redact::credential_alternation();
    let rule_names = redact::credential_rule_names();

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) => {
                if dir == root {
                    report.root_unreadable = true;
                }
                report.skip(
                    relative(root, &dir),
                    SkipReason::Unreadable { detail: err.to_string() },
                );
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    report.skip(
                        relative(root, &dir),
                        SkipReason::Unreadable { detail: err.to_string() },
                    );
                    continue;
                }
            };
            let path = entry.path();

            // `DirEntry::file_type` does not follow symlinks, which is what
            // makes the symlink arm below reachable at all.
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    report.skip(
                        relative(root, &path),
                        SkipReason::Unreadable { detail: err.to_string() },
                    );
                    continue;
                }
            };

            if file_type.is_symlink() {
                // Not followed: a symlink out of the worktree is a way to make
                // the scan read — and report on — a path it has no business in.
                report.skip(relative(root, &path), SkipReason::Symlink);
                continue;
            }

            if file_type.is_dir() {
                // The one directory deliberately out of scope. Its contents are
                // the object database, not the worktree, and this is a scope
                // decision rather than a decline — see the module doc.
                if path.file_name().map(|n| n == GIT_DIR).unwrap_or(false) {
                    continue;
                }
                stack.push(path);
                continue;
            }

            if !file_type.is_file() {
                report.skip(
                    relative(root, &path),
                    SkipReason::Unreadable { detail: "not a regular file".to_string() },
                );
                continue;
            }

            scan_file(&mut report, root, &path, limits, alternation, rule_names);
        }
    }

    report.findings.sort_by(|a, b| {
        (&a.path, a.line, a.rule).cmp(&(&b.path, b.line, b.rule))
    });
    report.skipped.sort_by(|a, b| a.0.cmp(&b.0));
    report
}

/// [`scan_worktree`] plus the optional external scanner (D-11).
///
/// The order is the mechanism: the built-in rules run **first and always**, so
/// there is no arrangement of the external tool's presence or absence under
/// which the scan is skipped.
pub fn scan_with_external(root: &Path, limits: ScanLimits) -> ScanReport {
    let mut report = scan_worktree(root, limits);
    report.external = run_gitleaks(root);
    match report.external {
        ExternalScanner::Blocked | ExternalScanner::Failed { .. } => {
            report.findings.push(Finding {
                path: PathBuf::from("."),
                line: 0,
                rule: GITLEAKS_RULE,
            });
        }
        _ => {}
    }
    report
}

/// Run `gitleaks` over `root` if it is resolvable, and report only its verdict.
///
/// **Its stdout and stderr are deliberately discarded rather than reproduced.**
/// A leak scanner's output names the secrets it found, so folding that output
/// into this report would be the SAFE-04 bug arriving through the one path that
/// looks most like diligence.
fn run_gitleaks(root: &Path) -> ExternalScanner {
    let outcome = std::process::Command::new(GITLEAKS_BIN)
        .arg("detect")
        .arg("--no-git")
        .arg("--no-banner")
        .arg("--redact")
        .arg("--source")
        .arg(root)
        .output();

    match outcome {
        Ok(output) if output.status.success() => ExternalScanner::Clean,
        Ok(_) => ExternalScanner::Blocked,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => ExternalScanner::Absent,
        Err(err) => ExternalScanner::Failed { detail: err.to_string() },
    }
}

/// One file: the caps, the binary probe, and the match pass.
fn scan_file(
    report: &mut ScanReport,
    root: &Path,
    path: &Path,
    limits: ScanLimits,
    alternation: &regex::Regex,
    rule_names: &[&'static str],
) {
    let rel = relative(root, path);

    // Budget first, so a file that comes up after the budget is spent is
    // reported for the reason that actually applies to it.
    if report.bytes_scanned >= limits.max_total_bytes {
        report.skip(rel, SkipReason::BudgetExhausted);
        return;
    }

    let size = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(err) => {
            report.skip(rel, SkipReason::Unreadable { detail: err.to_string() });
            return;
        }
    };
    if size > limits.max_file_bytes {
        report.skip(rel, SkipReason::TooLarge { bytes: size });
        return;
    }

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            report.skip(rel, SkipReason::Unreadable { detail: err.to_string() });
            return;
        }
    };
    if bytes.iter().take(NUL_PROBE_BYTES).any(|byte| *byte == 0) {
        report.skip(rel, SkipReason::Binary);
        return;
    }

    report.files_scanned += 1;
    report.bytes_scanned += bytes.len() as u64;

    let text = String::from_utf8_lossy(&bytes);
    collect_findings(&text, &rel, alternation, rule_names, &mut report.findings);
}

/// Match over the **whole file text**, not line by line, and derive the line
/// number from the match offset.
///
/// Line-by-line would be simpler and would silently miss the single most
/// important rule in the table: a PEM private key block spans many lines, so a
/// per-line pass never sees a `BEGIN`/`END` pair and a checked-in private key
/// scans clean.
fn collect_findings(
    text: &str,
    rel: &Path,
    alternation: &regex::Regex,
    rule_names: &[&'static str],
    out: &mut Vec<Finding>,
) {
    let mut seen: HashSet<(u32, &'static str)> = HashSet::new();
    // Matches arrive in offset order, so the newline count is carried forward
    // rather than recomputed from the start of the file for every match.
    let mut cursor = 0usize;
    let mut line = 1u32;

    for caps in alternation.captures_iter(text) {
        let Some(whole) = caps.get(0) else { continue };
        let start = whole.start();
        line += text[cursor..start].matches('\n').count() as u32;
        cursor = start;

        let Some(rule) = rule_names.iter().find(|name| caps.name(name).is_some()) else {
            continue;
        };
        if seen.insert((line, *rule)) {
            out.push(Finding { path: rel.to_path_buf(), line, rule });
        }
    }
}

/// `path` relative to `root`, so no report line carries an absolute home path.
fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A credential shape planted by the tests. Its literal bytes must never
    /// appear in a rendered report.
    const PLANTED_AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";

    const PLANTED_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
                               MIIBOgIBAAJBAKj34GkxFhD9\n\
                               abcdefgh\n\
                               -----END RSA PRIVATE KEY-----\n";

    fn tempdir() -> tempfile::TempDir {
        tempfile::TempDir::new().expect("a temp directory")
    }

    fn write(root: &Path, rel: &str, contents: &[u8]) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("the parent directory");
        }
        std::fs::write(path, contents).expect("the planted file");
    }

    #[test]
    fn a_planted_private_key_blocks_and_names_its_file_and_line() {
        let tmp = tempdir();
        write(tmp.path(), "src/ok.rs", b"fn main() {}\n");
        write(
            tmp.path(),
            "deploy/id_rsa",
            format!("# a note\n{PLANTED_PEM}").as_bytes(),
        );

        let report = scan_worktree(tmp.path(), ScanLimits::default());

        assert!(!report.is_clean(), "a checked-in private key must block");
        let finding = report
            .findings
            .iter()
            .find(|f| f.rule == "pem")
            .unwrap_or_else(|| panic!("the pem rule must be the one that fired: {report:?}"));
        assert_eq!(finding.path, PathBuf::from("deploy/id_rsa"));
        assert_eq!(
            finding.line, 2,
            "the line number must point at the BEGIN line, not at the file"
        );
    }

    #[test]
    fn the_rendered_report_never_contains_the_planted_secret() {
        let tmp = tempdir();
        write(
            tmp.path(),
            "config/aws.txt",
            format!("aws_access_key_id = {PLANTED_AWS_KEY}\n").as_bytes(),
        );

        let report = scan_worktree(tmp.path(), ScanLimits::default());
        assert!(!report.is_clean());

        let rendered = report.render();
        assert!(
            !rendered.contains(PLANTED_AWS_KEY),
            "the report reproduced the secret it blocked, which is the \
             redact-at-capture bug committed by the code meant to prevent it:\n{rendered}"
        );
        assert!(
            rendered.contains("config/aws.txt"),
            "the report must still name the file:\n{rendered}"
        );
        assert!(
            rendered.contains("rule=aws"),
            "the report must still name the rule:\n{rendered}"
        );
    }

    #[test]
    fn the_skip_list_is_printed_on_a_clean_report_as_well_as_a_blocking_one() {
        let tmp = tempdir();
        write(tmp.path(), "notes.txt", b"nothing to see\n");
        write(tmp.path(), "blob.bin", b"head\0tail\n");

        let clean = scan_worktree(tmp.path(), ScanLimits::default());
        assert!(clean.is_clean(), "this worktree carries no credential");
        let rendered = clean.render();
        assert!(
            rendered.contains("declined to read: 1 paths") && rendered.contains("blob.bin"),
            "a clean verdict must still disclose what the scan did not read \
             (D-14):\n{rendered}"
        );

        write(
            tmp.path(),
            "leak.txt",
            format!("key {PLANTED_AWS_KEY}\n").as_bytes(),
        );
        let blocking = scan_worktree(tmp.path(), ScanLimits::default());
        assert!(!blocking.is_clean());
        let rendered = blocking.render();
        assert!(
            rendered.contains("blob.bin"),
            "and so must a blocking one:\n{rendered}"
        );
    }

    #[test]
    fn a_file_over_the_per_file_cap_is_reported_as_skipped_rather_than_omitted() {
        let tmp = tempdir();
        write(tmp.path(), "huge.txt", &vec![b'a'; 4096]);

        let limits = ScanLimits { max_file_bytes: 1024, ..ScanLimits::default() };
        let report = scan_worktree(tmp.path(), limits);

        let (path, reason) = report
            .skipped
            .iter()
            .find(|(path, _)| path == Path::new("huge.txt"))
            .expect("an over-cap file must be IN the report, not absent from it");
        assert_eq!(path, Path::new("huge.txt"));
        assert_eq!(*reason, SkipReason::TooLarge { bytes: 4096 });
        assert_eq!(report.files_scanned, 0);
    }

    #[test]
    fn a_file_with_a_nul_byte_in_its_first_8_kib_is_recorded_binary() {
        let tmp = tempdir();
        let mut blob = vec![b'x'; 100];
        blob.push(0);
        blob.extend_from_slice(PLANTED_AWS_KEY.as_bytes());
        write(tmp.path(), "obj.bin", &blob);

        let report = scan_worktree(tmp.path(), ScanLimits::default());

        assert_eq!(
            report.skipped,
            vec![(PathBuf::from("obj.bin"), SkipReason::Binary)]
        );
        assert!(report.is_clean(), "a binary file is skipped, not scanned");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_is_recorded_and_not_followed() {
        let tmp = tempdir();
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&outside).expect("a directory outside the scanned root");
        std::fs::write(
            outside.join("secret.txt"),
            format!("key {PLANTED_AWS_KEY}\n"),
        )
        .expect("the file the symlink points at");

        let root = tmp.path().join("repo");
        std::fs::create_dir_all(&root).expect("the scanned root");
        std::os::unix::fs::symlink(outside.join("secret.txt"), root.join("link.txt"))
            .expect("a symlink");

        let report = scan_worktree(&root, ScanLimits::default());

        assert_eq!(
            report.skipped,
            vec![(PathBuf::from("link.txt"), SkipReason::Symlink)]
        );
        assert!(
            report.is_clean(),
            "following the symlink would let the scan read outside its own root"
        );
    }

    #[test]
    fn a_path_the_repositorys_ignore_rules_cover_is_scanned_anyway() {
        // D-13's whole reason, at unit scale: a backstop that asks git which
        // files to look at never sees these, and these are exactly where a
        // secret gets written.
        let tmp = tempdir();
        write(tmp.path(), ".gitignore", b"secrets/\n*.local\n.env\n");
        write(
            tmp.path(),
            "secrets/prod.pem",
            PLANTED_PEM.as_bytes(),
        );
        write(
            tmp.path(),
            ".env",
            format!("AWS_ACCESS_KEY={PLANTED_AWS_KEY}\n").as_bytes(),
        );

        let report = scan_worktree(tmp.path(), ScanLimits::default());

        assert!(!report.is_clean());
        let paths: Vec<&Path> = report.findings.iter().map(|f| f.path.as_path()).collect();
        assert!(
            paths.contains(&Path::new("secrets/prod.pem")),
            "an ignored DIRECTORY must still be read: {paths:?}"
        );
        assert!(
            paths.contains(&Path::new(".env")),
            "an ignored FILE must still be read: {paths:?}"
        );
    }

    #[test]
    fn an_exhausted_total_budget_marks_the_rest_skipped_rather_than_ending_the_scan() {
        let tmp = tempdir();
        for index in 0..5 {
            write(tmp.path(), &format!("f{index}.txt"), &[b'a'; 200]);
        }

        let limits = ScanLimits { max_file_bytes: 1024, max_total_bytes: 300 };
        let report = scan_worktree(tmp.path(), limits);

        assert_eq!(report.files_scanned, 2, "the budget stops the reading");
        assert_eq!(
            report.skipped.len(),
            3,
            "and every remaining file is NAMED rather than silently dropped: {:?}",
            report.skipped
        );
        for (_, reason) in &report.skipped {
            assert_eq!(*reason, SkipReason::BudgetExhausted);
        }
    }

    #[test]
    fn the_git_directory_is_not_walked() {
        let tmp = tempdir();
        write(
            tmp.path(),
            ".git/config-backup",
            format!("token {PLANTED_AWS_KEY}\n").as_bytes(),
        );
        write(tmp.path(), "README.md", b"hello\n");

        let report = scan_worktree(tmp.path(), ScanLimits::default());

        assert!(report.is_clean(), "the object database is out of scope: {report:?}");
        assert_eq!(report.files_scanned, 1);
    }

    #[test]
    fn one_line_carrying_two_rules_yields_one_finding_per_rule() {
        let tmp = tempdir();
        write(
            tmp.path(),
            "both.txt",
            format!("aws {PLANTED_AWS_KEY} and slack xoxb-1234567890-abcdefghijkl\n").as_bytes(),
        );

        let report = scan_worktree(tmp.path(), ScanLimits::default());
        let rules: Vec<&str> = report.findings.iter().map(|f| f.rule).collect();

        assert!(rules.contains(&"aws"), "{rules:?}");
        assert!(rules.contains(&"slack"), "{rules:?}");
        assert!(report.findings.iter().all(|f| f.line == 1));
    }

    #[test]
    fn a_scan_that_could_not_read_its_root_is_never_reported_as_clean() {
        // The fail-open this control exists to refuse: a scan that never ran
        // finds nothing, and "found nothing" must not become "allow".
        let tmp = tempdir();
        let missing = tmp.path().join("does-not-exist");

        let report = scan_worktree(&missing, ScanLimits::default());

        assert_eq!(report.files_scanned, 0);
        assert!(report.root_unreadable);
        assert!(
            !report.is_clean(),
            "a scan that could not read its own root reported clean: {report:?}"
        );
        assert!(matches!(report.skipped[0].1, SkipReason::Unreadable { .. }));
        // And the rendered report says so in as many words.
        assert!(report.render().contains("it never ran"));
    }
}
