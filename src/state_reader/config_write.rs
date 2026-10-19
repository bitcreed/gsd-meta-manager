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
    let raw = i32::try_from(pid).ok()?;
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
