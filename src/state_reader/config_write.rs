//! Lock-aware, atomic, read-modify-write saves of a GSD config file
//! (quick 261019-guw).
//!
//! Before this module the Defaults tab wrote a cached snapshot straight over
//! `.planning/config.json` with a plain `std::fs::write`: it ignored gsd-core's
//! own lock, could expose a truncated file to a concurrent reader, reset the
//! file's permission bits and overwrote every change GSD made since the tab
//! loaded. [`update_gsd_config`] is the one write path now, and Phase 32's
//! capability toggles reuse it.
//!
//! # The protocol this replicates (measured against gsd-core v1.18.0-6-g8d3d29c46)
//!
//! **`.planning/.lock`** — `withPlanningLock` (`planning-workspace.cjs:313-472`,
//! `src/planning-workspace.cts:425-`). Taken by every gsd-core config write
//! (`config.cjs:569`, `:600`, `:637`). The lock sits next to `config.json`
//! (`planningDir(cwd)/.lock`, `:316`, `:103-122`).
//! - Create: `O_CREAT|O_EXCL`, body `{"pid":N,"cwd":"<root>","acquired":"<ISO-8601 ms Z>"}`
//!   compact, no trailing newline (`:334-342`).
//! - Transient create errors (EPERM, EBUSY, EAGAIN, EINTR, EINVAL, EIO, ENOENT,
//!   ESTALE) wait one poll and retry (`:93-102`, `:369-372`); anything else but
//!   EEXIST is fatal.
//! - EEXIST: a holder is *verified-live* when the body parses, `pid` is an
//!   integer > 0 and `kill(pid, 0)` succeeds or fails with EPERM (`:49-59`,
//!   `:74-86`). Anything else is stealable: re-confirm `(dev, ino, body)`, then
//!   rename the lock aside (only one racer wins) and delete the renamed file
//!   (`:404-450`). An unreadable body is never stolen.
//! - Upstream also steals a verified-live holder once the lock is older than
//!   60 s (the "deadman", `:323`, `:397-403`). **This module does not**
//!   ([INFERRED I-guw-3]): a live holder always ends in a timeout.
//! - Release: unlink (`:343-354`). Here only if the body is still ours
//!   ([INFERRED I-guw-9]).
//!
//! **`$GSD_HOME/.gsd/defaults.json`** is only ever written by gsd-core's
//! installer, under `acquireInstallMigrationLock`
//! (`installer-migrations.cjs:499-597`, lock `gsd-install-migration.lock` at
//! `:37`): body `{"pid":N,"acquiredAt":"..."}` plus a newline; a dead pid, our
//! own pid or a non-positive pid is reclaimed by unlink (`:470-479`); an
//! unparseable body is never reclaimed; poll 50 ms.
//!
//! Both flavours are bounded to a 2 s budget here ([INFERRED I-guw-2]) because
//! the save runs synchronously on the key-handler thread.
//!
//! The write itself is a same-directory `O_EXCL` temp file, fsynced, given the
//! target's permission bits, and renamed over the target, so a reader sees the
//! old or the new file and never a partial one.
//!
//! Everything here is synchronous on purpose (it is called from sync key
//! handlers); do not add an `async fn` to this module.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rustix::io::Errno;
use serde::Serialize;

use super::config_json::{parse_gsd_config, serialize_gsd_config, GsdConfig};

/// Cap on a lock body read (a lock body is ~100 bytes).
const LOCK_BODY_CAP: u64 = 4096;
/// Cap on a config file read inside the lock.
const CONFIG_READ_CAP: u64 = 4 * 1024 * 1024;
/// Attempts at a free temp name before giving up.
const TEMP_NAME_ATTEMPTS: usize = 16;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
static STALE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Which of gsd-core's two lock protocols applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockFlavor {
    /// `.planning/.lock` (`withPlanningLock`).
    Planning,
    /// `$GSD_HOME/.gsd/gsd-install-migration.lock` (`acquireInstallMigrationLock`).
    InstallMigration,
}

/// How long to wait for a held lock and how often to look again.
#[derive(Debug, Clone, Copy)]
pub struct LockPolicy {
    pub timeout: Duration,
    pub poll: Duration,
}

impl LockPolicy {
    /// The TUI budget: 2 s, at upstream's poll interval for the flavour
    /// ([INFERRED I-guw-2]).
    pub fn tui(flavor: LockFlavor) -> Self {
        let poll = match flavor {
            LockFlavor::Planning => Duration::from_millis(100),
            LockFlavor::InstallMigration => Duration::from_millis(50),
        };
        LockPolicy {
            timeout: Duration::from_secs(2),
            poll,
        }
    }
}

/// The file a save writes, and thereby which lock protects it.
#[derive(Debug, Clone)]
pub enum ConfigWriteTarget {
    /// A project's `.planning/config.json`; the lock is `<its dir>/.lock`.
    Project {
        config_path: PathBuf,
        project_root: PathBuf,
    },
    /// `$GSD_HOME/.gsd/defaults.json`; the lock is
    /// `<its dir>/gsd-install-migration.lock`.
    GlobalDefaults { path: PathBuf },
}

impl ConfigWriteTarget {
    pub fn config_path(&self) -> &Path {
        match self {
            ConfigWriteTarget::Project { config_path, .. } => config_path,
            ConfigWriteTarget::GlobalDefaults { path } => path,
        }
    }

    pub fn flavor(&self) -> LockFlavor {
        match self {
            ConfigWriteTarget::Project { .. } => LockFlavor::Planning,
            ConfigWriteTarget::GlobalDefaults { .. } => LockFlavor::InstallMigration,
        }
    }

    fn dir(&self) -> &Path {
        self.config_path()
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    }

    pub fn lock_path(&self) -> PathBuf {
        match self {
            ConfigWriteTarget::Project { .. } => self.dir().join(".lock"),
            ConfigWriteTarget::GlobalDefaults { .. } => {
                self.dir().join("gsd-install-migration.lock")
            }
        }
    }
}

/// Why a save did not happen. Carries no path and no config value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigWriteError {
    /// A live process held the lock for the whole budget.
    LockTimeout {
        holder_pid: Option<i64>,
        waited: Duration,
    },
    /// The file on disk exists but does not parse; nothing was written.
    Unparseable,
    /// An I/O failure, reduced to its kind.
    Io { kind: std::io::ErrorKind },
    /// The edited config could not be serialised.
    Serialize,
}

impl ConfigWriteError {
    /// The status-line text. Built ONLY from static text, an integer pid and an
    /// `ErrorKind` — never a path, a config value or serde's `Display`
    /// (T-guw-08, T-jnf-03, I-12).
    pub fn status_text(&self, flavor: LockFlavor) -> String {
        let saved = match flavor {
            LockFlavor::Planning => "Config NOT saved",
            LockFlavor::InstallMigration => "Defaults NOT saved",
        };
        match self {
            ConfigWriteError::LockTimeout { holder_pid, .. } => {
                let who = match holder_pid {
                    Some(pid) => format!("pid {pid}"),
                    None => "another process".to_string(),
                };
                match flavor {
                    LockFlavor::Planning => format!(
                        "{saved}: .planning/.lock is held by {who} (GSD is writing) — retry"
                    ),
                    LockFlavor::InstallMigration => format!(
                        "{saved}: the GSD installer lock is held by {who} — retry after it finishes"
                    ),
                }
            }
            ConfigWriteError::Unparseable => {
                format!("{saved}: the file on disk no longer parses — press r to reload")
            }
            ConfigWriteError::Io { kind } => format!("{saved}: {kind}"),
            ConfigWriteError::Serialize => format!("{saved}: the config could not be serialised"),
        }
    }
}

#[derive(Serialize)]
struct PlanningLockBody<'a> {
    pid: u32,
    cwd: &'a str,
    acquired: String,
}

#[derive(Serialize)]
struct InstallLockBody {
    pid: u32,
    #[serde(rename = "acquiredAt")]
    acquired_at: String,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Read a file safely: it may be planted by anything running in the project
/// (the Phase 27 CR-01 class, [INFERRED I-guw-6]). `O_NONBLOCK` always (a FIFO
/// with no writer cannot hang the open), `O_NOFOLLOW` unless `follow_symlinks`,
/// a regular-file check on the open handle, and a `take(cap + 1)` bound. Any
/// failed check is an error, which callers treat as "unreadable".
fn read_bounded(path: &Path, cap: u64, follow_symlinks: bool) -> std::io::Result<Vec<u8>> {
    let mut flags = rustix::fs::OFlags::NONBLOCK.bits();
    if !follow_symlinks {
        flags |= rustix::fs::OFlags::NOFOLLOW.bits();
    }
    let custom = i32::try_from(flags)
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "open flags"))?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(custom)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.file_type().is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not a regular file",
        ));
    }
    if meta.len() > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file larger than the read cap",
        ));
    }
    let mut raw = Vec::new();
    file.take(cap + 1).read_to_end(&mut raw)?;
    if raw.len() as u64 > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file outgrew the read cap",
        ));
    }
    Ok(raw)
}

/// A lock body's `pid` as an integer, if it has one (an integral float counts,
/// as with JS `Number.isInteger`). `None` for every other shape.
fn body_pid(body: &[u8]) -> Option<i64> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let n = v.as_object()?.get("pid")?.as_number()?;
    if let Some(i) = n.as_i64() {
        return Some(i);
    }
    let f = n.as_f64()?;
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        Some(f as i64)
    } else {
        None
    }
}

/// `kill(pid, 0)`: `Some(true)` alive, `Some(false)` ESRCH, `None` for a pid
/// that does not fit a `pid_t` (or is not positive).
fn probe_pid(pid: i64) -> Option<Result<(), Errno>> {
    // `Pid::from_raw` panics on a negative value, so reject it here.
    let raw = i32::try_from(pid).ok().filter(|r| *r > 0)?;
    let p = rustix::process::Pid::from_raw(raw)?;
    Some(rustix::process::test_kill_process(p))
}

/// Planning flavour: verified-live = positive integer pid that fits `pid_t`
/// and answers `kill(pid, 0)` with success or EPERM.
fn planning_verified_live(pid: Option<i64>) -> bool {
    match pid.and_then(probe_pid) {
        Some(Ok(())) => true,
        Some(Err(e)) => e == Errno::PERM,
        None => false,
    }
}

fn is_transient_create_error(e: &std::io::Error) -> bool {
    let Some(raw) = e.raw_os_error() else {
        return false;
    };
    let errno = Errno::from_raw_os_error(raw);
    [
        Errno::PERM,
        Errno::BUSY,
        Errno::AGAIN,
        Errno::INTR,
        Errno::INVAL,
        Errno::IO,
        Errno::NOENT,
        Errno::STALE,
    ]
    .contains(&errno)
}

/// An acquired lock. Dropping it releases the lock, but only if the file still
/// carries the body this guard wrote ([INFERRED I-guw-9]).
struct LockGuard {
    path: PathBuf,
    body: Vec<u8>,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        match read_bounded(&self.path, LOCK_BODY_CAP, false) {
            Ok(now) if now == self.body => {
                if let Err(e) = fs::remove_file(&self.path) {
                    tracing::warn!("config lock release failed: {}", e.kind());
                }
            }
            Ok(_) => tracing::warn!("config lock was replaced while held; leaving it in place"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                tracing::warn!("config lock vanished while held");
            }
            Err(e) => tracing::warn!("config lock not released: {}", e.kind()),
        }
    }
}

enum Next {
    /// Look again immediately.
    RetryNow,
    /// Sleep one poll, then look again.
    Wait,
}

/// Create the lock file `O_EXCL` and write the body through the same handle.
/// A failed body write unlinks the file just created.
fn try_create(path: &Path, body: &[u8]) -> std::io::Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o666)
        .open(path)?;
    if let Err(e) = f.write_all(body) {
        drop(f);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    Ok(())
}

fn lock_body(target: &ConfigWriteTarget) -> Vec<u8> {
    let pid = std::process::id();
    match target {
        ConfigWriteTarget::Project { project_root, .. } => {
            let cwd = project_root.to_string_lossy();
            serde_json::to_vec(&PlanningLockBody {
                pid,
                cwd: &cwd,
                acquired: now_iso(),
            })
            .unwrap_or_default()
        }
        ConfigWriteTarget::GlobalDefaults { .. } => {
            let mut v = serde_json::to_vec(&InstallLockBody {
                pid,
                acquired_at: now_iso(),
            })
            .unwrap_or_default();
            v.push(b'\n');
            v
        }
    }
}

/// The Planning-flavour EEXIST decision (`planning-workspace.cjs:384-457`).
fn planning_on_exists(path: &Path, last_holder: &mut Option<i64>) -> Next {
    let Ok(snap_meta) = fs::symlink_metadata(path) else {
        // A failed lstat means the lock moved under us: look again at once.
        return Next::RetryNow;
    };
    let snap_id = (snap_meta.dev(), snap_meta.ino());
    let snap_body = read_bounded(path, LOCK_BODY_CAP, false).ok();
    let pid = snap_body.as_deref().and_then(body_pid);
    if let Some(p) = pid.filter(|p| *p > 0) {
        *last_holder = Some(p);
    }
    let Some(snap_body) = snap_body else {
        // Unreadable (symlink, FIFO, oversized, ...): never stolen.
        return Next::Wait;
    };
    if planning_verified_live(pid) {
        // No deadman steal (I-guw-3).
        return Next::Wait;
    }
    // Not verified-live: re-confirm (dev, ino, body), then rename aside.
    let Ok(again_meta) = fs::symlink_metadata(path) else {
        return Next::RetryNow;
    };
    if (again_meta.dev(), again_meta.ino()) != snap_id {
        return Next::Wait;
    }
    match read_bounded(path, LOCK_BODY_CAP, false) {
        Ok(again_body) if again_body == snap_body => {}
        _ => return Next::Wait,
    }
    let seq = STALE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let mut aside = path.as_os_str().to_os_string();
    aside.push(format!(".stale-{}-{}-{}", std::process::id(), ms, seq));
    let aside = PathBuf::from(aside);
    match fs::rename(path, &aside) {
        Ok(()) => {
            let _ = fs::remove_file(&aside);
            Next::RetryNow
        }
        Err(_) => Next::Wait,
    }
}

/// The InstallMigration-flavour EEXIST decision
/// (`installer-migrations.cjs:499-597`).
fn install_on_exists(path: &Path, last_holder: &mut Option<i64>) -> Next {
    let Some(pid) = read_bounded(path, LOCK_BODY_CAP, false)
        .ok()
        .as_deref()
        .and_then(body_pid)
    else {
        // Unparseable or unreadable: never reclaimed.
        return Next::Wait;
    };
    let dead = pid == i64::from(std::process::id())
        || pid <= 0
        || matches!(probe_pid(pid), Some(Err(e)) if e == Errno::SRCH);
    if dead {
        if fs::remove_file(path).is_ok() {
            return Next::RetryNow;
        }
        return Next::Wait;
    }
    *last_holder = Some(pid);
    Next::Wait
}

fn acquire(target: &ConfigWriteTarget, policy: LockPolicy) -> Result<LockGuard, ConfigWriteError> {
    let flavor = target.flavor();
    let path = target.lock_path();
    fs::create_dir_all(target.dir()).map_err(|e| ConfigWriteError::Io { kind: e.kind() })?;

    let start = Instant::now();
    let deadline = start + policy.timeout;
    let mut last_holder: Option<i64> = None;
    let mut last_transient: Option<std::io::ErrorKind> = None;
    loop {
        let body = lock_body(target);
        let next = match try_create(&path, &body) {
            Ok(()) => return Ok(LockGuard { path, body }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => match flavor {
                LockFlavor::Planning => planning_on_exists(&path, &mut last_holder),
                LockFlavor::InstallMigration => install_on_exists(&path, &mut last_holder),
            },
            Err(e) if flavor == LockFlavor::Planning && is_transient_create_error(&e) => {
                last_transient = Some(e.kind());
                Next::Wait
            }
            Err(e) => return Err(ConfigWriteError::Io { kind: e.kind() }),
        };
        let now = Instant::now();
        if now >= deadline {
            return Err(match (last_holder, last_transient) {
                (None, Some(kind)) => ConfigWriteError::Io { kind },
                (holder_pid, _) => ConfigWriteError::LockTimeout {
                    holder_pid,
                    waited: start.elapsed(),
                },
            });
        }
        if let Next::Wait = next {
            std::thread::sleep(policy.poll.min(deadline - now));
        }
    }
}

/// Write `bytes` over `path` atomically: a same-directory `O_EXCL` temp file
/// (`<target>.tmp-<pid>-<n>`), the target's permission bits, `fsync`, rename.
/// A foreign file at a temp name is never touched (the next counter is tried).
/// On any failure the temp file is removed.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?
        .to_string_lossy()
        .into_owned();

    let mut created = None;
    for _ in 0..TEMP_NAME_ATTEMPTS {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = dir.join(format!("{name}.tmp-{}-{n}", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o666)
            .open(&tmp)
        {
            Ok(f) => {
                created = Some((tmp, f));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    let Some((tmp, mut file)) = created else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "no free temp file name",
        ));
    };

    let result = (|| -> std::io::Result<()> {
        file.write_all(bytes)?;
        if let Ok(meta) = fs::metadata(path) {
            if meta.is_file() {
                file.set_permissions(fs::Permissions::from_mode(meta.mode() & 0o7777))?;
            }
        }
        file.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if let Err(e) = result {
        drop(file);
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    // Best effort: make the rename itself durable.
    if let Ok(d) = fs::File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

/// Read-modify-write one config file under gsd-core's lock.
///
/// Inside the lock the file is re-read, `mutate` is applied to that FRESH copy
/// (as upstream `setConfigValue` does), and the result is written atomically
/// ([INFERRED I-guw-4]):
/// - the file exists but does not parse: `Err(Unparseable)`, nothing written;
/// - the file is absent: `fallback` is the base (create-on-save);
/// - `mutate` returns `false`: `Ok(None)`, nothing written;
/// - otherwise `Ok(Some(fresh))`, the config now on disk.
pub fn update_gsd_config(
    target: &ConfigWriteTarget,
    policy: LockPolicy,
    fallback: &GsdConfig,
    mutate: impl FnOnce(&mut GsdConfig) -> bool,
) -> Result<Option<GsdConfig>, ConfigWriteError> {
    let _guard = acquire(target, policy)?;
    let path = target.config_path();
    let mut fresh = match read_bounded(path, CONFIG_READ_CAP, true) {
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| ConfigWriteError::Unparseable)?;
            parse_gsd_config(&text).ok_or(ConfigWriteError::Unparseable)?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => fallback.clone(),
        Err(e) => return Err(ConfigWriteError::Io { kind: e.kind() }),
    };
    if !mutate(&mut fresh) {
        return Ok(None);
    }
    let json = serialize_gsd_config(&fresh).map_err(|_| ConfigWriteError::Serialize)?;
    atomic_write(path, json.as_bytes()).map_err(|e| ConfigWriteError::Io { kind: e.kind() })?;
    Ok(Some(fresh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tempfile::TempDir;

    const DEAD_PID: i64 = i32::MAX as i64;

    fn policy(timeout_ms: u64) -> LockPolicy {
        LockPolicy {
            timeout: Duration::from_millis(timeout_ms),
            poll: Duration::from_millis(20),
        }
    }

    fn project(td: &TempDir) -> ConfigWriteTarget {
        fs::create_dir_all(td.path().join(".planning")).unwrap();
        ConfigWriteTarget::Project {
            config_path: td.path().join(".planning/config.json"),
            project_root: td.path().to_path_buf(),
        }
    }

    fn global(td: &TempDir) -> ConfigWriteTarget {
        fs::create_dir_all(td.path().join(".gsd")).unwrap();
        ConfigWriteTarget::GlobalDefaults {
            path: td.path().join(".gsd/defaults.json"),
        }
    }

    fn set_commit_docs(c: &mut GsdConfig) -> bool {
        c.commit_docs = Some(false);
        true
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn atomic_write_never_exposes_a_partial_file() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("config.json");
        let a = vec![b'a'; 512 * 1024];
        let b = vec![b'b'; 300 * 1024];
        atomic_write(&path, &a).unwrap();
        let done = Arc::new(AtomicBool::new(false));
        let writer = {
            let (path, a, b, done) = (path.clone(), a.clone(), b.clone(), done.clone());
            std::thread::spawn(move || {
                for i in 0..200 {
                    atomic_write(&path, if i % 2 == 0 { &b } else { &a }).unwrap();
                }
                done.store(true, Ordering::SeqCst);
            })
        };
        let mut reads = 0;
        while reads < 2000 || !done.load(Ordering::SeqCst) {
            let got = fs::read(&path).unwrap();
            assert!(
                got == a || got == b,
                "a reader saw a partial file ({} bytes)",
                got.len()
            );
            reads += 1;
        }
        writer.join().unwrap();
    }

    #[test]
    fn atomic_write_preserves_existing_permission_bits() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("config.json");
        for mode in [0o600, 0o640] {
            fs::write(&path, b"old").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            atomic_write(&path, b"new").unwrap();
            assert_eq!(fs::read(&path).unwrap(), b"new");
            assert_eq!(
                fs::metadata(&path).unwrap().mode() & 0o7777,
                mode,
                "mode {mode:o} must survive the save"
            );
        }
    }

    #[test]
    fn atomic_write_leaves_no_temp_file_behind() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("config.json");
        for i in 0..50 {
            atomic_write(&path, format!("v{i}").as_bytes()).unwrap();
        }
        assert_eq!(names(td.path()), vec!["config.json".to_string()]);
    }

    #[test]
    fn atomic_write_never_touches_a_foreign_file_at_a_temp_name() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("config.json");
        // Plant a symlink at every name the next write could pick.
        let next = TEMP_COUNTER.load(Ordering::Relaxed);
        let victim = td.path().join("victim");
        fs::write(&victim, b"precious").unwrap();
        for n in next..next + 4 {
            let planted = td
                .path()
                .join(format!("config.json.tmp-{}-{n}", std::process::id()));
            let _ = symlink(&victim, planted);
        }
        atomic_write(&path, b"data").unwrap();
        assert_eq!(fs::read(&victim).unwrap(), b"precious");
        assert_eq!(fs::read(&path).unwrap(), b"data");
    }

    #[test]
    fn planning_lock_live_holder_times_out_without_stealing() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let lock = target.lock_path();
        fs::write(
            &lock,
            r#"{"pid":1,"cwd":"/x","acquired":"2026-10-19T00:00:00.000Z"}"#,
        )
        .unwrap();
        fs::write(target.config_path(), r#"{"mode":"yolo"}"#).unwrap();
        let lock_before = fs::read(&lock).unwrap();
        let started = Instant::now();
        let err = update_gsd_config(&target, policy(300), &GsdConfig::default(), set_commit_docs)
            .unwrap_err();
        assert!(started.elapsed() >= Duration::from_millis(300));
        match err {
            ConfigWriteError::LockTimeout { holder_pid, .. } => assert_eq!(holder_pid, Some(1)),
            other => panic!("expected LockTimeout, got {other:?}"),
        }
        assert_eq!(
            fs::read(&lock).unwrap(),
            lock_before,
            "a live lock is never stolen"
        );
        assert_eq!(
            fs::read(target.config_path()).unwrap(),
            br#"{"mode":"yolo"}"#
        );
    }

    #[test]
    fn planning_lock_released_mid_wait_is_acquired() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let lock = target.lock_path();
        fs::write(&lock, r#"{"pid":1}"#).unwrap();
        let remover = {
            let lock = lock.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(200));
                fs::remove_file(lock).unwrap();
            })
        };
        let started = Instant::now();
        let got = update_gsd_config(
            &target,
            policy(2000),
            &GsdConfig::default(),
            set_commit_docs,
        )
        .unwrap();
        remover.join().unwrap();
        assert!(started.elapsed() >= Duration::from_millis(200));
        assert!(got.is_some());
        let on_disk = parse_gsd_config(&fs::read_to_string(target.config_path()).unwrap()).unwrap();
        assert_eq!(on_disk.commit_docs, Some(false));
        assert!(!lock.exists());
    }

    #[test]
    fn planning_lock_dead_holder_is_stolen() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        fs::write(
            target.lock_path(),
            format!(r#"{{"pid":{DEAD_PID},"cwd":"/x","acquired":"2026-10-19T00:00:00.000Z"}}"#),
        )
        .unwrap();
        let started = Instant::now();
        update_gsd_config(
            &target,
            policy(2000),
            &GsdConfig::default(),
            set_commit_docs,
        )
        .unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(1000),
            "stolen at once"
        );
        assert_eq!(
            names(&td.path().join(".planning")),
            vec!["config.json".to_string()]
        );
    }

    #[test]
    fn planning_lock_garbage_empty_or_pidless_body_is_stolen() {
        for body in [
            "not json",
            "",
            r#"{"cwd":"x"}"#,
            r#"{"pid":0}"#,
            r#"{"pid":-5}"#,
            r#"{"pid":"12"}"#,
        ] {
            let td = TempDir::new().unwrap();
            let target = project(&td);
            fs::write(target.lock_path(), body).unwrap();
            update_gsd_config(
                &target,
                policy(1000),
                &GsdConfig::default(),
                set_commit_docs,
            )
            .unwrap_or_else(|e| panic!("body {body:?} must be stolen, got {e:?}"));
            assert_eq!(
                names(&td.path().join(".planning")),
                vec!["config.json".to_string()],
                "body {body:?}"
            );
        }
    }

    #[test]
    fn planning_lock_unreadable_body_is_never_stolen() {
        // A symlink at .lock.
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let real = td.path().join("real-lock");
        fs::write(&real, format!(r#"{{"pid":{DEAD_PID}}}"#)).unwrap();
        symlink(&real, target.lock_path()).unwrap();
        let err = update_gsd_config(&target, policy(300), &GsdConfig::default(), set_commit_docs)
            .unwrap_err();
        assert!(
            matches!(err, ConfigWriteError::LockTimeout { .. }),
            "{err:?}"
        );
        assert!(fs::symlink_metadata(target.lock_path())
            .unwrap()
            .file_type()
            .is_symlink());

        // A lock larger than the 4 KiB cap.
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let big = format!(r#"{{"pid":{DEAD_PID},"pad":"{}"}}"#, "x".repeat(5000));
        fs::write(target.lock_path(), &big).unwrap();
        let err = update_gsd_config(&target, policy(300), &GsdConfig::default(), set_commit_docs)
            .unwrap_err();
        assert!(
            matches!(err, ConfigWriteError::LockTimeout { .. }),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(target.lock_path()).unwrap(), big);
    }

    #[test]
    fn planning_lock_body_matches_gsd_core_shape() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let lock = target.lock_path();
        let mut seen = String::new();
        update_gsd_config(&target, policy(1000), &GsdConfig::default(), |c| {
            seen = fs::read_to_string(&lock).unwrap();
            set_commit_docs(c)
        })
        .unwrap();
        let prefix = format!(
            r#"{{"pid":{},"cwd":"{}","acquired":""#,
            std::process::id(),
            td.path().display()
        );
        assert!(seen.starts_with(&prefix), "{seen}");
        assert!(
            seen.ends_with(r#"Z"}"#),
            "compact, no trailing newline: {seen:?}"
        );
        let acquired = &seen[prefix.len()..seen.len() - 2];
        assert_eq!(acquired.len(), 24, "ISO-8601 with milliseconds: {acquired}");
        assert!(chrono::DateTime::parse_from_rfc3339(acquired).is_ok());
        let v: serde_json::Value = serde_json::from_str(&seen).unwrap();
        assert_eq!(v.as_object().unwrap().len(), 3);
        assert!(!lock.exists());
    }

    #[test]
    fn install_lock_rules() {
        // A dead pid and our own pid are reclaimed.
        for pid in [DEAD_PID, i64::from(std::process::id())] {
            let td = TempDir::new().unwrap();
            let target = global(&td);
            fs::write(
                target.lock_path(),
                format!("{{\"pid\":{pid},\"acquiredAt\":\"2026-10-19T00:00:00.000Z\"}}\n"),
            )
            .unwrap();
            update_gsd_config(
                &target,
                policy(1000),
                &GsdConfig::default(),
                set_commit_docs,
            )
            .unwrap_or_else(|e| panic!("pid {pid} must be reclaimed, got {e:?}"));
            assert_eq!(
                names(&td.path().join(".gsd")),
                vec!["defaults.json".to_string()]
            );
        }

        // An unparseable body is never reclaimed.
        let td = TempDir::new().unwrap();
        let target = global(&td);
        fs::write(target.lock_path(), "garbage").unwrap();
        let err = update_gsd_config(&target, policy(300), &GsdConfig::default(), set_commit_docs)
            .unwrap_err();
        assert!(
            matches!(
                err,
                ConfigWriteError::LockTimeout {
                    holder_pid: None,
                    ..
                }
            ),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(target.lock_path()).unwrap(), "garbage");
        assert!(!target.config_path().exists());

        // A live pid is waited on.
        let td = TempDir::new().unwrap();
        let target = global(&td);
        let live = "{\"pid\":1,\"acquiredAt\":\"2026-10-19T00:00:00.000Z\"}\n";
        fs::write(target.lock_path(), live).unwrap();
        let err = update_gsd_config(&target, policy(300), &GsdConfig::default(), set_commit_docs)
            .unwrap_err();
        assert!(
            matches!(
                err,
                ConfigWriteError::LockTimeout {
                    holder_pid: Some(1),
                    ..
                }
            ),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(target.lock_path()).unwrap(), live);

        // The body we write.
        let td = TempDir::new().unwrap();
        let target = global(&td);
        let lock = target.lock_path();
        let mut seen = String::new();
        update_gsd_config(&target, policy(1000), &GsdConfig::default(), |c| {
            seen = fs::read_to_string(&lock).unwrap();
            set_commit_docs(c)
        })
        .unwrap();
        let prefix = format!("{{\"pid\":{},\"acquiredAt\":\"", std::process::id());
        assert!(seen.starts_with(&prefix), "{seen}");
        assert!(seen.ends_with("Z\"}\n"), "{seen:?}");
        assert!(!lock.exists());
    }

    #[test]
    fn update_applies_the_edit_to_the_on_disk_config_not_the_cached_snapshot() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        fs::write(
            target.config_path(),
            r#"{"mode":"interactive","brand_new_gsd_key":{"a":1}}"#,
        )
        .unwrap();
        let fallback = parse_gsd_config(r#"{"mode":"yolo"}"#).unwrap();
        let fresh = update_gsd_config(&target, policy(1000), &fallback, set_commit_docs)
            .unwrap()
            .expect("written");
        assert_eq!(fresh.mode, "interactive");
        let text = fs::read_to_string(target.config_path()).unwrap();
        let on_disk = parse_gsd_config(&text).unwrap();
        assert_eq!(
            on_disk.mode, "interactive",
            "the disk mode wins over the snapshot"
        );
        assert_eq!(on_disk.commit_docs, Some(false));
        assert!(
            text.contains("brand_new_gsd_key"),
            "unknown keys survive: {text}"
        );
    }

    #[test]
    fn update_refuses_an_unparseable_config_on_disk() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        fs::write(target.config_path(), "{not json").unwrap();
        let err = update_gsd_config(
            &target,
            policy(1000),
            &GsdConfig::default(),
            set_commit_docs,
        )
        .unwrap_err();
        assert_eq!(err, ConfigWriteError::Unparseable);
        assert_eq!(fs::read(target.config_path()).unwrap(), b"{not json");
        assert!(
            !target.lock_path().exists(),
            "the lock is released on refusal"
        );
    }

    #[test]
    fn update_creates_an_absent_file_from_the_fallback() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        assert!(!target.config_path().exists());
        let fallback = parse_gsd_config(r#"{"mode":"yolo"}"#).unwrap();
        update_gsd_config(&target, policy(1000), &fallback, set_commit_docs).unwrap();
        let on_disk = parse_gsd_config(&fs::read_to_string(target.config_path()).unwrap()).unwrap();
        assert_eq!(on_disk.mode, "yolo");
        assert_eq!(on_disk.commit_docs, Some(false));
    }

    #[test]
    fn release_does_not_delete_a_lock_it_no_longer_owns() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        let lock = target.lock_path();
        let foreign = r#"{"pid":1,"cwd":"/other","acquired":"2026-10-19T00:00:00.000Z"}"#;
        update_gsd_config(&target, policy(1000), &GsdConfig::default(), |c| {
            fs::remove_file(&lock).unwrap();
            fs::write(&lock, foreign).unwrap();
            set_commit_docs(c)
        })
        .unwrap();
        assert_eq!(fs::read_to_string(&lock).unwrap(), foreign);
    }

    #[test]
    fn update_without_a_mutation_writes_nothing() {
        let td = TempDir::new().unwrap();
        let target = project(&td);
        fs::write(target.config_path(), r#"{"mode":"yolo"}"#).unwrap();
        let before = fs::metadata(target.config_path())
            .unwrap()
            .modified()
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let got =
            update_gsd_config(&target, policy(1000), &GsdConfig::default(), |_| false).unwrap();
        assert!(got.is_none());
        assert_eq!(
            fs::read(target.config_path()).unwrap(),
            br#"{"mode":"yolo"}"#
        );
        assert_eq!(
            fs::metadata(target.config_path())
                .unwrap()
                .modified()
                .unwrap(),
            before
        );
        assert!(!target.lock_path().exists());
    }

    #[test]
    fn status_text_carries_no_path_and_names_only_the_pid() {
        let timeout = ConfigWriteError::LockTimeout {
            holder_pid: Some(4242),
            waited: Duration::from_secs(2),
        };
        let t = timeout.status_text(LockFlavor::Planning);
        assert!(
            t.starts_with("Config NOT saved") && t.contains("pid 4242"),
            "{t}"
        );
        let unknown = ConfigWriteError::LockTimeout {
            holder_pid: None,
            waited: Duration::ZERO,
        };
        assert!(unknown
            .status_text(LockFlavor::InstallMigration)
            .contains("another process"));
        let io = ConfigWriteError::Io {
            kind: std::io::ErrorKind::PermissionDenied,
        };
        assert!(!io.status_text(LockFlavor::Planning).contains('/'));
    }
}
