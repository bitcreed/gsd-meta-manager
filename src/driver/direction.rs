//! `DIRECTION.md` file primitives: the tool-owned frontmatter splice and the
//! shared body write (D-12, D-13, D-20, D-23).
//!
//! `DIRECTION.md` has two authors. The **body** (`## Direction`, `## Items`) is
//! the human's, and the human edits it with whatever editor they like, saving at
//! any instant. The leading `---` **frontmatter** block carrying
//! `generated_by: driverctl` is the tool's, regenerated as the run progresses.
//! The facts below are each a way the obvious version silently loses a human's
//! save.
//!
//! 1. **Only a marked leading block is the tool's (D-12).** [`split_frontmatter`]
//!    treats a leading `---` block as the tool's only when it contains
//!    [`FM_MARKER`] before its closing `---` line. A human's own unmarked `---`
//!    block is *body*: it is preserved byte-for-byte and the tool block is
//!    prepended above it. Both `---\n` and `---\r\n` openings are recognised, so a
//!    CRLF tool block is *replaced* by one LF block rather than a second block
//!    being stacked on top (D-23).
//! 2. **Read-then-rename loses edits (D-12).** An editor that saves by rename
//!    between the tool's read and its rename is silently reverted. The splice
//!    therefore uses `renameat2(RENAME_EXCHANGE)`: it puts the tool's file in
//!    place **and hands back the inode it displaced**, then compares the
//!    displaced file's *body bytes* with the body it built on. A mismatch means a
//!    human save landed in the window; the tool exchanges **back**, which restores
//!    the human's inode (including an in-place write still in flight into it),
//!    and retries from a fresh read. A human's bytes are never unlinked unseen.
//!    There is no fallback to a plain rename: an `EINVAL`/`ENOSYS` from the
//!    exchange is a typed error, because the fallback is exactly the bug.
//! 3. **Newest human save wins (D-12).** If, after exchanging back, the file now
//!    at the temp name is *not* the tool's own stale bytes, a newer rename-save
//!    replaced ours in the meantime; it is renamed into place before the retry.
//! 4. **An insane body is never written back (D-12).** An in-place editor save
//!    (truncate, then write) can be observed half done. A body missing either
//!    `## Direction` or `## Items` is [`SpliceOutcome::SkippedInsaneBody`] and the
//!    file's bytes are untouched. A run dir with no `DIRECTION.md` is
//!    [`SpliceOutcome::NoDirection`] and nothing is created.
//! 5. **One flock serialises gsd-mm's own writers (D-13, D-20).**
//!    [`direction_lock`] takes `<run-dir>/DIRECTION.lock` with a ~2 s bounded
//!    retry. [`write_body`] is the primitive every gsd-mm edit of the body (Phase
//!    30's steering) must use; it shares the splice's exchange-verify loop, so the
//!    two never race. A *sandboxed* process can hold the lock (flock works on a
//!    read-only descriptor), so on timeout the splice proceeds without it and
//!    reports [`SpliceReport::lock_timed_out`]; the exchange-verify alone prevents
//!    rename-save loss.
//! 6. **The temp is durable and keeps the mode (D-23).** The temp file is
//!    `.DIRECTION.md.fm.<pid>.<n>`, created `O_EXCL`, given the original file's
//!    mode bits and `fsync`ed **before** the exchange, so a power loss between the
//!    write and the exchange cannot surface an empty `DIRECTION.md`.
//!    [`sweep_stale_temps`] removes only such temps older than a threshold.
//!
//! The exchange functions are Linux-only (`renameat2` is a Linux syscall); on
//! other targets they return [`DirectionError::UnsupportedPlatform`]. The pure
//! helpers compile everywhere.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use regex::Regex;

#[cfg(unix)]
use crate::driver::lock::{acquire_at_with_retry, RunLock};
#[cfg(unix)]
use crate::error::LockError;

/// The marker that makes a leading `---` block the tool's.
pub const FM_MARKER: &str = "generated_by: driverctl";
/// The human-owned direction file inside a run directory.
pub const DIRECTION_FILE: &str = "DIRECTION.md";
/// The advisory lock shared by every gsd-mm writer of `DIRECTION.md`.
pub const DIRECTION_LOCK_FILE: &str = "DIRECTION.lock";
/// The temp-file name prefix the splice uses (D-23).
pub const TEMP_PREFIX: &str = ".DIRECTION.md.fm.";
/// How long [`direction_lock`] retries before giving up (D-13).
pub const DIRECTION_LOCK_BUDGET: Duration = Duration::from_secs(2);
/// How many exchange-verify attempts one splice makes.
pub const SPLICE_TRIES: u32 = 5;

/// Split `text` into the tool's leading frontmatter block and the body.
///
/// Returns `(Some(block), body)` only when `text` starts with `---\n` or
/// `---\r\n`, a closing line that is exactly `---` (terminated by `\n` or
/// `\r\n`) follows, and [`FM_MARKER`] occurs inside the block. The block slice
/// includes its closing delimiter line; the body is everything after it,
/// byte-exact. Anything else is `(None, text)`: an unmarked human block is body.
pub fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let open_len = if text.starts_with("---\n") {
        4
    } else if text.starts_with("---\r\n") {
        5
    } else {
        return (None, text);
    };

    let mut pos = open_len;
    while pos < text.len() {
        let Some(rel) = text[pos..].find('\n') else {
            // The last line has no terminator: not a closing delimiter line.
            break;
        };
        let line_end = pos + rel;
        let line = text[pos..line_end]
            .strip_suffix('\r')
            .unwrap_or(&text[pos..line_end]);
        if line == "---" {
            let block_end = line_end + 1;
            if text[open_len..pos].contains(FM_MARKER) {
                return (Some(&text[..block_end]), &text[block_end..]);
            }
            return (None, text);
        }
        pos = line_end + 1;
    }
    (None, text)
}

/// Whether `body` still has both section headers.
///
/// An in-place (truncate-then-write) editor save can be observed half done; a
/// body without `## Direction` and `## Items` is never written back.
pub fn body_sane(body: &str) -> bool {
    body.contains("## Direction") && body.contains("## Items")
}

/// One `### I-NN` item and its `- budget:` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionItem {
    /// The item id, e.g. `I-01`.
    pub id: String,
    /// `k=v` integer pairs from the item's `- budget:` line; non-integers skipped.
    pub budget: BTreeMap<String, i64>,
}

/// What [`parse_direction`] read out of a body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectionInfo {
    /// The word after `- control:`, lower-cased, if any.
    pub control: Option<String>,
    /// Items in document order.
    pub items: Vec<DirectionItem>,
}

/// The `- control: <word>` matcher. `pub(crate)` because [`parse_direction`] is not
/// its only caller: `driver::steer` reads the same line, and a second regex would let
/// the two disagree about which line is the human's control (WR-01).
pub(crate) fn control_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Deliberately looser than the lint's `- control: <word>`: the human's pause is a
    // safety control, so an indented line, extra spaces or any letter case still read.
    RE.get_or_init(|| {
        Regex::new(r"(?mi)^[ \t]*-[ \t]+control[ \t]*:[ \t]*(\w+)").expect("control regex")
    })
}

/// The hold a `control` word asks for: `Some("pause")` or `Some("stop")` for those
/// words in any letter case and with surrounding whitespace, else `None`.
///
/// The one matcher for the human's pause/stop (D-18, D-19): the store's `dispatched`
/// refusal and the launcher gate both call it, so neither fails open on `Pause`.
pub fn control_hold(word: &str) -> Option<&'static str> {
    match word.trim().to_ascii_lowercase().as_str() {
        "pause" => Some("pause"),
        "stop" => Some("stop"),
        _ => None,
    }
}

/// The `### I-NN` item-heading matcher. `pub(crate)` for the same reason as
/// [`control_re`]: `driver::steer` is the second caller of the one reader (WR-01).
pub(crate) fn item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^### (I-\d+)\b").expect("item regex"))
}

fn budget_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^- budget:\s*(.+)$").expect("budget regex"))
}

/// `text` with every `<!-- ... -->` span replaced by spaces, newlines kept, so the
/// result has the same length, the same line structure and the same char
/// boundaries as `text`. An unterminated comment runs to the end.
///
/// The one comment reader: [`parse_direction`] (the store) and the `driver check`
/// lint both call it, so a commented-out line is invisible to both.
pub fn blank_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        let end = tail.find("-->").map_or(tail.len(), |i| i + "-->".len());
        out.extend(tail[..end].chars().flat_map(|c| {
            // One ASCII space per byte keeps offsets identical to the input.
            let n = if c == '\n' { 0 } else { c.len_utf8() };
            let spaces = std::iter::repeat_n(' ', n);
            let newline = std::iter::once('\n').take(usize::from(c == '\n'));
            newline.chain(spaces)
        }));
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// The `## ` section a DIRECTION body line belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Before the first `## ` heading.
    Preamble,
    /// Under `## Direction`.
    Direction,
    /// Under `## Items`.
    Items,
    /// Under any other `## ` heading.
    Other,
}

/// The section a `## ` heading line opens, or `None` when `line` is not one.
///
/// The one section reader: the store's parser and the lint both use it.
pub fn section_heading(line: &str) -> Option<Section> {
    let line = line.trim_end();
    if !(line.starts_with("## ") || line == "##") {
        return None;
    }
    Some(match line.trim() {
        "## Direction" => Section::Direction,
        "## Items" => Section::Items,
        _ => Section::Other,
    })
}

/// Parse the control word and per-item budgets out of a DIRECTION body.
///
/// A port of the spike's `parse_direction` that never fails: an unreadable field
/// is simply absent. It reads the body the way the `driver check` lint does
/// (WR-01): HTML comments are blanked first, and `### I-n` items count only under
/// `## Items`, so a commented-out or misplaced item is not an item to either
/// reader.
pub fn parse_direction(body: &str) -> DirectionInfo {
    let blanked = blank_comments(body);
    // The control word, lower-cased. With more than one `- control:` line (the lint
    // flags that) a `pause` or `stop` on any of them wins over an earlier `run`: an
    // ambiguous file holds the run rather than spending on.
    let mut control: Option<String> = None;
    for caps in control_re().captures_iter(&blanked) {
        let word = caps[1].to_ascii_lowercase();
        let holds = control_hold(&word).is_some();
        if control.is_none() || (holds && control.as_deref().and_then(control_hold).is_none()) {
            control = Some(word);
        }
    }

    let mut items_text = String::new();
    let mut section = Section::Preamble;
    for line in blanked.lines() {
        if let Some(next) = section_heading(line) {
            section = next;
        } else if section == Section::Items {
            items_text.push_str(line);
            items_text.push('\n');
        }
    }
    let body = items_text.as_str();

    let starts: Vec<(usize, usize, String)> = item_re()
        .captures_iter(body)
        .filter_map(|c| {
            let whole = c.get(0)?;
            Some((whole.start(), whole.end(), c.get(1)?.as_str().to_string()))
        })
        .collect();

    let mut items = Vec::with_capacity(starts.len());
    for (i, (_, end, id)) in starts.iter().enumerate() {
        let seg_end = starts.get(i + 1).map_or(body.len(), |next| next.0);
        let segment = &body[*end..seg_end];
        let mut budget = BTreeMap::new();
        if let Some(caps) = budget_re().captures(segment) {
            for kv in caps[1].split(',') {
                if let Some((k, v)) = kv.split_once('=') {
                    if let Ok(n) = v.trim().parse::<i64>() {
                        budget.insert(k.trim().to_string(), n);
                    }
                }
            }
        }
        items.push(DirectionItem {
            id: id.clone(),
            budget,
        });
    }
    DirectionInfo { control, items }
}

/// Where a run directory's `DIRECTION.lock` lives (D-13).
pub fn direction_lock_path(run_dir: &Path) -> PathBuf {
    run_dir.join(DIRECTION_LOCK_FILE)
}

/// Take `<run-dir>/DIRECTION.lock`, retrying for [`DIRECTION_LOCK_BUDGET`] (D-13).
///
/// Every gsd-mm writer of `DIRECTION.md` — driverctl's splice and Phase 30's
/// steering edits — goes through this one lock. The returned guard releases it
/// on drop.
#[cfg(unix)]
pub fn direction_lock(run_dir: &Path) -> Result<RunLock, LockError> {
    acquire_at_with_retry(
        &direction_lock_path(run_dir),
        "direction-edit",
        std::process::id(),
        DIRECTION_LOCK_BUDGET,
    )
}

/// How one splice or body write ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpliceOutcome {
    /// The new content is in place and the human's body bytes were preserved.
    Ok,
    /// The body lacked `## Direction` or `## Items`; the file is untouched.
    SkippedInsaneBody,
    /// Every attempt lost a race with a human save; the human's bytes stand.
    GaveUp,
    /// The run dir has no `DIRECTION.md`; nothing was created.
    NoDirection,
}

impl SpliceOutcome {
    /// The stable string for logs and the race-test output.
    pub fn as_str(self) -> &'static str {
        match self {
            SpliceOutcome::Ok => "ok",
            SpliceOutcome::SkippedInsaneBody => "skipped-insane-body",
            SpliceOutcome::GaveUp => "gave-up",
            SpliceOutcome::NoDirection => "no-direction",
        }
    }
}

/// An outcome plus whether `DIRECTION.lock` could be taken (D-20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpliceReport {
    /// How the operation ended.
    pub outcome: SpliceOutcome,
    /// `true` when the lock budget elapsed and the operation proceeded without
    /// it; the store logs `direction-lock-timeout`.
    pub lock_timed_out: bool,
}

/// A failure of the file layer, distinct from a lost race (which is an outcome).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectionError {
    /// A filesystem or lock fault.
    Io {
        /// What failed.
        detail: String,
    },
    /// The frontmatter block handed to [`splice_frontmatter`] is not a tool block.
    InvalidBlock {
        /// Why it was refused.
        detail: String,
    },
    /// [`write_body`]'s edit produced a body without both section headers.
    InsaneNewBody,
    /// `renameat2(RENAME_EXCHANGE)` is not available on this target.
    UnsupportedPlatform {
        /// What is missing.
        detail: String,
    },
}

impl fmt::Display for DirectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirectionError::Io { detail } => write!(f, "DIRECTION.md I/O failed: {detail}"),
            DirectionError::InvalidBlock { detail } => {
                write!(f, "not a driverctl frontmatter block: {detail}")
            }
            DirectionError::InsaneNewBody => write!(
                f,
                "refusing to write a DIRECTION.md body without both `## Direction` and `## Items`"
            ),
            DirectionError::UnsupportedPlatform { detail } => {
                write!(f, "unsupported platform: {detail}")
            }
        }
    }
}

impl std::error::Error for DirectionError {}

/// Replace only the leading tool block of `<run_dir>/DIRECTION.md` with `fm`.
///
/// `fm` must start with `---\n`, end with `\n---\n` and contain [`FM_MARKER`].
/// See the module header for the exchange-verify protocol.
pub fn splice_frontmatter(run_dir: &Path, fm: &str) -> Result<SpliceReport, DirectionError> {
    if !(fm.starts_with("---\n") && fm.ends_with("\n---\n") && fm.contains(FM_MARKER)) {
        return Err(DirectionError::InvalidBlock {
            detail: format!(
                "a block must start with `---\\n`, end with `\\n---\\n` and contain `{FM_MARKER}`"
            ),
        });
    }
    #[cfg(target_os = "linux")]
    {
        imp::splice(run_dir, fm)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = run_dir;
        Err(unsupported())
    }
}

/// Edit only the body of `<run_dir>/DIRECTION.md`, keeping the tool block verbatim.
///
/// The D-13 primitive: gsd-mm's own DIRECTION writes (Phase 30's steering edits)
/// must go through this so they share `DIRECTION.lock` and the exchange-verify
/// loop with driverctl's splice. `edit` receives the current body and returns the
/// new one; it may be called again on a fresh body when a human save raced the
/// write. A new body missing either section header is [`DirectionError::InsaneNewBody`]
/// and nothing is written.
pub fn write_body<F: FnMut(&str) -> String>(
    run_dir: &Path,
    edit: F,
) -> Result<SpliceReport, DirectionError> {
    #[cfg(target_os = "linux")]
    {
        imp::write_body(run_dir, edit)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (run_dir, edit);
        Err(unsupported())
    }
}

#[cfg(not(target_os = "linux"))]
fn unsupported() -> DirectionError {
    DirectionError::UnsupportedPlatform {
        detail: "renameat2(RENAME_EXCHANGE) is Linux-only".to_string(),
    }
}

/// Remove splice temps older than `older_than` from `run_dir` (D-23).
///
/// Only entries whose name starts with [`TEMP_PREFIX`] are touched.
/// `[inferred — audit]`: the caller picks a threshold long enough that a
/// concurrent Phase 30 [`write_body`] temp in flight is never swept.
pub fn sweep_stale_temps(run_dir: &Path, older_than: Duration) -> std::io::Result<usize> {
    let cutoff = SystemTime::now().checked_sub(older_than);
    let mut removed = 0;
    for entry in std::fs::read_dir(run_dir)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with(TEMP_PREFIX) {
            continue;
        }
        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(modified) => modified,
            Err(_) => continue,
        };
        let stale = cutoff.is_some_and(|cutoff| modified < cutoff);
        if stale && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(target_os = "linux")]
mod imp {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    use rustix::fs::{renameat_with, RenameFlags};

    use super::{
        body_sane, direction_lock, split_frontmatter, DirectionError, SpliceOutcome, SpliceReport,
        DIRECTION_FILE, SPLICE_TRIES, TEMP_PREFIX,
    };
    use crate::error::LockError;

    /// Process-wide counter that makes every temp name unique within a process.
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// What the caller wants written for the body it was just shown.
    enum Plan {
        /// The body is not fit to build on; leave the file alone.
        Skip,
        /// The full new file content.
        Write(String),
    }

    fn io_err(context: impl std::fmt::Display, err: impl std::fmt::Display) -> DirectionError {
        DirectionError::Io {
            detail: format!("{context}: {err}"),
        }
    }

    pub(super) fn splice(run_dir: &Path, fm: &str) -> Result<SpliceReport, DirectionError> {
        locked(run_dir, || {
            run_loop(run_dir, |_block, body| {
                if body_sane(body) {
                    Ok(Plan::Write(format!("{fm}{body}")))
                } else {
                    Ok(Plan::Skip)
                }
            })
        })
    }

    pub(super) fn write_body<F: FnMut(&str) -> String>(
        run_dir: &Path,
        mut edit: F,
    ) -> Result<SpliceReport, DirectionError> {
        locked(run_dir, || {
            run_loop(run_dir, |block, body| {
                let new_body = edit(body);
                if !body_sane(&new_body) {
                    return Err(DirectionError::InsaneNewBody);
                }
                Ok(Plan::Write(format!(
                    "{}{new_body}",
                    block.unwrap_or_default()
                )))
            })
        })
    }

    /// Run `body` holding `DIRECTION.lock` when it can be had (D-20).
    fn locked(
        run_dir: &Path,
        body: impl FnOnce() -> Result<SpliceOutcome, DirectionError>,
    ) -> Result<SpliceReport, DirectionError> {
        // An absent file creates nothing, not even the lock.
        if !run_dir.join(DIRECTION_FILE).exists() {
            return Ok(SpliceReport {
                outcome: SpliceOutcome::NoDirection,
                lock_timed_out: false,
            });
        }
        let (guard, lock_timed_out) = match direction_lock(run_dir) {
            Ok(guard) => (Some(guard), false),
            // A sandboxed process may be holding it (T-27-02): proceed without.
            Err(LockError::HeldBy { .. } | LockError::HeldByUnknownRun) => (None, true),
            Err(other) => return Err(io_err("DIRECTION.lock", other)),
        };
        let outcome = body()?;
        drop(guard);
        Ok(SpliceReport {
            outcome,
            lock_timed_out,
        })
    }

    fn exchange(dir: &File, a: &str, b: &str) -> rustix::io::Result<()> {
        renameat_with(dir, a, dir, b, RenameFlags::EXCHANGE)
    }

    /// Create the temp `O_EXCL`, write `content`, copy `mode`, fsync (D-23).
    fn write_temp(path: &Path, content: &str, mode: u32) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        file.set_permissions(std::fs::Permissions::from_mode(mode))?;
        file.sync_all()
    }

    /// The exchange-verify loop shared by the splice and the body write.
    fn run_loop<P>(run_dir: &Path, mut plan: P) -> Result<SpliceOutcome, DirectionError>
    where
        P: FnMut(Option<&str>, &str) -> Result<Plan, DirectionError>,
    {
        let target = run_dir.join(DIRECTION_FILE);
        let dir = File::open(run_dir).map_err(|e| io_err(run_dir.display(), e))?;

        for attempt in 0..SPLICE_TRIES {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(5 * u64::from(attempt)));
            }
            let bytes = match std::fs::read(&target) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(SpliceOutcome::NoDirection)
                }
                Err(e) => return Err(io_err(target.display(), e)),
            };
            // A non-UTF-8 read is most likely an in-place save observed half
            // done; retry rather than guess at bytes (the file is untouched).
            let Ok(text) = String::from_utf8(bytes) else {
                continue;
            };
            let (block, body) = split_frontmatter(&text);
            let new_content = match plan(block, body)? {
                Plan::Skip => return Ok(SpliceOutcome::SkippedInsaneBody),
                Plan::Write(content) => content,
            };
            let mode = match std::fs::metadata(&target) {
                Ok(meta) => meta.permissions().mode() & 0o7777,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(SpliceOutcome::NoDirection)
                }
                Err(e) => return Err(io_err(target.display(), e)),
            };

            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let temp_name = format!("{TEMP_PREFIX}{}.{n}", std::process::id());
            let temp_path = run_dir.join(&temp_name);
            if let Err(e) = write_temp(&temp_path, &new_content, mode) {
                let _ = std::fs::remove_file(&temp_path);
                return Err(io_err(temp_path.display(), e));
            }

            // Our file goes in place AND the displaced inode comes back to the
            // temp name. No fallback to a plain rename (module fact 2).
            match exchange(&dir, &temp_name, DIRECTION_FILE) {
                Ok(()) => {}
                Err(errno) if errno == rustix::io::Errno::NOENT => {
                    let _ = std::fs::remove_file(&temp_path);
                    return Ok(SpliceOutcome::NoDirection);
                }
                Err(errno) => {
                    let _ = std::fs::remove_file(&temp_path);
                    return Err(io_err("renameat2(RENAME_EXCHANGE)", errno));
                }
            }

            // D-23: compare BODY BYTES, not inode identity. An unreadable or
            // non-UTF-8 displaced file is a mismatch: give it back.
            let displaced_matches = std::fs::read(&temp_path)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .is_some_and(|displaced| split_frontmatter(&displaced).1 == body);
            if displaced_matches {
                let _ = std::fs::remove_file(&temp_path);
                return Ok(SpliceOutcome::Ok);
            }

            // A human save raced us: give their inode back. After this the
            // temp name holds whatever was at DIRECTION.md just before.
            if let Err(errno) = exchange(&dir, &temp_name, DIRECTION_FILE) {
                // Never unlink here: the temp name may hold a human's bytes.
                return Err(io_err(
                    format!(
                        "could not exchange back; a displaced file was left at {}",
                        temp_path.display()
                    ),
                    errno,
                ));
            }
            match std::fs::read(&temp_path) {
                Ok(bytes) if bytes == new_content.as_bytes() => {
                    // Our own stale file: drop it and try again.
                    let _ = std::fs::remove_file(&temp_path);
                }
                Ok(_) => {
                    // An even newer rename-save replaced ours in the meantime:
                    // the newest human bytes win, then retry.
                    std::fs::rename(&temp_path, &target)
                        .map_err(|e| io_err(target.display(), e))?;
                }
                Err(e) => return Err(io_err(temp_path.display(), e)),
            }
        }
        Ok(SpliceOutcome::GaveUp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "# Driver direction: t\n\n## Direction\n\n- control: run\n\n## Items\n\n### I-01 first\n\n- budget: dispatches=3, tokens=1000, bad=x\n\n### I-02 second\n\n- budget: dispatches=1\n";

    fn block(n: u32) -> String {
        format!("---\n{FM_MARKER}\nn: {n}\n---\n")
    }

    #[test]
    fn split_frontmatter_table_cases() {
        let marked = format!("---\n{FM_MARKER}\n---\nbody\n");
        let (b, rest) = split_frontmatter(&marked);
        assert_eq!(
            b,
            Some(&marked[..marked.len() - 5]),
            "block includes its closing line"
        );
        assert_eq!(rest, "body\n");

        let crlf = format!("---\r\n{FM_MARKER}\r\n---\r\nbody\r\n");
        let (b, rest) = split_frontmatter(&crlf);
        assert!(b.is_some(), "a CRLF opening with the marker is the tool's");
        assert_eq!(rest, "body\r\n", "the CRLF body is returned byte-exact");

        let human = "---\ntitle: x\n---\n## Direction\n";
        assert_eq!(split_frontmatter(human), (None, human), "no marker: body");

        let no_close = format!("---\n{FM_MARKER}\nbody\n");
        assert_eq!(
            split_frontmatter(&no_close).0,
            None,
            "no closing line: body"
        );

        let unterminated = format!("---\n{FM_MARKER}\n---");
        assert_eq!(
            split_frontmatter(&unterminated).0,
            None,
            "a closing `---` with no newline is not a delimiter line"
        );

        let plain = "# hello\n";
        assert_eq!(split_frontmatter(plain), (None, plain));

        // The marker after the closing line belongs to the body.
        let late = format!("---\nx: 1\n---\n{FM_MARKER}\n");
        assert_eq!(split_frontmatter(&late).0, None, "marker outside the block");

        // Only the FIRST closing line ends the block.
        let two = format!("---\n{FM_MARKER}\n---\nmid\n---\nend\n");
        assert_eq!(split_frontmatter(&two).1, "mid\n---\nend\n");
    }

    #[test]
    fn body_sane_needs_both_headers() {
        assert!(body_sane(BODY));
        assert!(!body_sane("## Direction\n"));
        assert!(!body_sane("## Items\n"));
        assert!(!body_sane(""));
    }

    #[test]
    fn parse_direction_reads_control_items_and_budgets() {
        let info = parse_direction(BODY);
        assert_eq!(info.control.as_deref(), Some("run"));
        assert_eq!(info.items.len(), 2, "items in document order");
        assert_eq!(info.items[0].id, "I-01");
        assert_eq!(info.items[0].budget.get("dispatches"), Some(&3));
        assert_eq!(info.items[0].budget.get("tokens"), Some(&1000));
        assert!(
            !info.items[0].budget.contains_key("bad"),
            "a non-integer budget value is skipped"
        );
        assert_eq!(info.items[1].id, "I-02");
        assert_eq!(info.items[1].budget.get("dispatches"), Some(&1));

        assert_eq!(parse_direction("nothing here"), DirectionInfo::default());
    }

    #[test]
    fn control_words_are_lower_cased_and_a_hold_on_any_line_wins() {
        let body = |line: &str| format!("## Direction\n\n{line}\n\n## Items\n");
        assert_eq!(
            parse_direction(&body("- control:   PaUsE  "))
                .control
                .as_deref(),
            Some("pause")
        );
        assert_eq!(
            parse_direction(&body("  -  CONTROL :stop"))
                .control
                .as_deref(),
            Some("stop")
        );
        // Duplicates: the first word, unless a later line holds the run.
        assert_eq!(
            parse_direction(&body("- control: run\n- control: other"))
                .control
                .as_deref(),
            Some("run")
        );
        assert_eq!(
            parse_direction(&body("- control: run\n- control: Stop\n- control: pause"))
                .control
                .as_deref(),
            Some("stop")
        );
        // A commented-out hold is not a hold.
        assert_eq!(
            parse_direction(&body("- control: run\n<!-- - control: pause -->"))
                .control
                .as_deref(),
            Some("run")
        );
        assert_eq!(control_hold(" Pause\t"), Some("pause"));
        assert_eq!(control_hold("run"), None);
    }

    #[test]
    fn direction_lock_path_is_beside_direction_md() {
        assert_eq!(
            direction_lock_path(Path::new("/r/run")),
            Path::new("/r/run/DIRECTION.lock")
        );
    }

    #[test]
    fn invalid_block_is_refused_before_touching_anything() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        for bad in [
            "",
            "no frontmatter",
            "---\nx: 1\n---\n",
            "---\ngenerated_by: driverctl\n",
        ] {
            assert!(
                matches!(
                    splice_frontmatter(dir.path(), bad),
                    Err(DirectionError::InvalidBlock { .. })
                ),
                "{bad:?} is not a tool block"
            );
        }
        assert_eq!(std::fs::read_dir(dir.path()).expect("read_dir").count(), 0);
    }

    #[test]
    fn sweep_removes_only_old_prefixed_temps() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let old = dir.path().join(format!("{TEMP_PREFIX}1.1"));
        let fresh = dir.path().join(format!("{TEMP_PREFIX}1.2"));
        let other_old = dir.path().join("DIRECTION.md");
        for p in [&old, &fresh, &other_old] {
            std::fs::write(p, "x").expect("write");
        }
        let long_ago = SystemTime::now() - Duration::from_secs(3600);
        for p in [&old, &other_old] {
            std::fs::File::options()
                .write(true)
                .open(p)
                .expect("open")
                .set_modified(long_ago)
                .expect("set_modified");
        }

        let removed = sweep_stale_temps(dir.path(), Duration::from_secs(600)).expect("sweep");
        assert_eq!(removed, 1, "exactly the old prefixed temp goes");
        assert!(!old.exists(), "the stale temp is removed");
        assert!(fresh.exists(), "a recent temp may belong to a live writer");
        assert!(other_old.exists(), "a non-temp file is never touched");
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;
        use std::os::unix::fs::PermissionsExt;

        fn run_dir_with(contents: &str) -> tempfile::TempDir {
            let dir = tempfile::TempDir::new().expect("temp dir");
            std::fs::write(dir.path().join(DIRECTION_FILE), contents).expect("seed DIRECTION.md");
            dir
        }

        fn read(dir: &tempfile::TempDir) -> String {
            std::fs::read_to_string(dir.path().join(DIRECTION_FILE)).expect("read DIRECTION.md")
        }

        fn no_temps_left(dir: &tempfile::TempDir) -> bool {
            std::fs::read_dir(dir.path())
                .expect("read_dir")
                .filter_map(Result::ok)
                .all(|e| !e.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
        }

        #[test]
        fn splice_prepends_then_replaces_the_tool_block_and_keeps_body_bytes() {
            let dir = run_dir_with(BODY);

            let first = splice_frontmatter(dir.path(), &block(1)).expect("first splice");
            assert_eq!(first.outcome, SpliceOutcome::Ok);
            assert!(
                !first.lock_timed_out,
                "an uncontended lock is not a timeout"
            );
            assert_eq!(
                read(&dir),
                format!("{}{BODY}", block(1)),
                "a first splice prepends the block above the untouched body"
            );

            let second = splice_frontmatter(dir.path(), &block(2)).expect("second splice");
            assert_eq!(second.outcome, SpliceOutcome::Ok);
            let after = read(&dir);
            assert_eq!(
                after.matches(FM_MARKER).count(),
                1,
                "a second splice replaces the block, it never stacks a second one"
            );
            assert_eq!(after, format!("{}{BODY}", block(2)));
            assert_eq!(
                split_frontmatter(&after).1,
                BODY,
                "the bytes after the block equal the original body"
            );
            assert!(
                direction_lock_path(dir.path()).exists(),
                "the splice ran under DIRECTION.lock"
            );
            assert!(no_temps_left(&dir), "no temp survives a successful splice");
        }

        #[test]
        fn insane_body_is_never_written_back() {
            let truncated = "## Direction\n- control: run\n";
            let dir = run_dir_with(truncated);

            let report = splice_frontmatter(dir.path(), &block(1)).expect("splice");

            assert_eq!(report.outcome, SpliceOutcome::SkippedInsaneBody);
            assert_eq!(
                read(&dir),
                truncated,
                "an insane body's bytes are unchanged"
            );
            assert!(no_temps_left(&dir));
        }

        #[test]
        fn missing_direction_is_no_direction_and_creates_nothing() {
            let dir = tempfile::TempDir::new().expect("temp dir");

            let report = splice_frontmatter(dir.path(), &block(1)).expect("splice");

            assert_eq!(report.outcome, SpliceOutcome::NoDirection);
            assert_eq!(
                std::fs::read_dir(dir.path()).expect("read_dir").count(),
                0,
                "no DIRECTION.md, no lock, no temp"
            );
        }

        #[test]
        fn crlf_tool_block_is_replaced_not_duplicated() {
            let crlf_body = BODY.replace('\n', "\r\n");
            let seeded = format!("---\r\n{FM_MARKER}\r\nn: 0\r\n---\r\n{crlf_body}");
            let dir = run_dir_with(&seeded);

            let report = splice_frontmatter(dir.path(), &block(7)).expect("splice");

            assert_eq!(report.outcome, SpliceOutcome::Ok);
            let after = read(&dir);
            assert_eq!(after.matches(FM_MARKER).count(), 1, "one block, not two");
            assert_eq!(
                after,
                format!("{}{crlf_body}", block(7)),
                "one LF block and the CRLF body byte-identical"
            );
        }

        #[test]
        fn unmarked_human_block_is_body() {
            let human = format!("---\ntitle: mine\n---\n{BODY}");
            let dir = run_dir_with(&human);

            let report = splice_frontmatter(dir.path(), &block(1)).expect("splice");

            assert_eq!(report.outcome, SpliceOutcome::Ok);
            assert_eq!(
                read(&dir),
                format!("{}{human}", block(1)),
                "the tool block goes above the human's block, which is preserved byte-for-byte"
            );
        }

        #[test]
        fn file_mode_is_kept() {
            let dir = run_dir_with(BODY);
            let path = dir.path().join(DIRECTION_FILE);
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                .expect("chmod 600");

            splice_frontmatter(dir.path(), &block(1)).expect("splice");

            let mode = std::fs::metadata(&path).expect("stat").permissions().mode() & 0o7777;
            assert_eq!(mode, 0o600, "the original mode survives the exchange");
        }

        #[test]
        fn write_body_keeps_the_tool_block_verbatim() {
            let dir = run_dir_with(&format!("{}{BODY}", block(4)));

            let report = write_body(dir.path(), |old| {
                old.replace("control: run", "control: pause")
            })
            .expect("write_body");

            assert_eq!(report.outcome, SpliceOutcome::Ok);
            let after = read(&dir);
            assert!(
                after.starts_with(&block(4)),
                "the tool block is byte-identical"
            );
            assert!(after.contains("- control: pause"), "the edit landed");
            assert_eq!(after.matches(FM_MARKER).count(), 1);
        }

        #[test]
        fn write_body_leaves_the_file_alone_when_the_body_is_unchanged() {
            use std::os::unix::fs::MetadataExt;
            let seeded = format!("{}{BODY}", block(4));
            let dir = run_dir_with(&seeded);
            let path = dir.path().join(DIRECTION_FILE);
            let before = std::fs::metadata(&path).expect("stat");

            let report = write_body(dir.path(), |live| live.to_string()).expect("write_body");

            assert_eq!(report.outcome, SpliceOutcome::Ok);
            let after = std::fs::metadata(&path).expect("stat");
            assert_eq!(
                after.ino(),
                before.ino(),
                "same inode: nothing was exchanged"
            );
            assert_eq!(after.modified().unwrap(), before.modified().unwrap());
            assert_eq!(read(&dir), seeded);
            assert!(no_temps_left(&dir), "no .DIRECTION.md.fm.* temp was made");
        }

        #[test]
        fn write_body_refuses_an_insane_body() {
            let seeded = format!("{}{BODY}", block(4));
            let dir = run_dir_with(&seeded);

            let result = write_body(dir.path(), |_| "wiped\n".to_string());

            assert_eq!(result, Err(DirectionError::InsaneNewBody));
            assert_eq!(read(&dir), seeded, "nothing is written for an insane body");
            assert!(no_temps_left(&dir));
        }

        #[test]
        fn a_held_direction_lock_times_out_and_the_splice_still_lands() {
            let dir = run_dir_with(BODY);
            let _held = direction_lock(dir.path()).expect("the test takes the lock first");

            let report = splice_frontmatter(dir.path(), &block(1)).expect("splice");

            assert!(
                report.lock_timed_out,
                "D-20: a timeout is reported, not hidden"
            );
            assert_eq!(report.outcome, SpliceOutcome::Ok);
            assert_eq!(read(&dir), format!("{}{BODY}", block(1)));
        }
    }
}
