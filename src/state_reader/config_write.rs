//! Lock-aware, atomic, read-modify-write saves of `$GSD_HOME/.gsd/defaults.json`
//! (quick 261019-guw, narrowed by quick 261116-kft).
//!
//! # Why this still writes a GSD file
//!
//! GSD files are read-only for this tool: every change goes through gsd-tools
//! or a GSD command (CLAUDE.md, PROJECT.md, commit cf7a246c). Project saves of
//! `.planning/config.json` therefore run gsd-tools `config-set`
//! ([`super::config_set`], quick 261116-kft). gsd-core has no command that
//! writes `defaults.json`: only its installer does (and the /gsd-settings
//! workflow has the agent write it). So the Config tab's Global scope is the
//! one place this build still writes a GSD file itself, a known gap against the
//! read-only rule (261116-kft D-02). It does so under gsd-core's own installer
//! lock and with an atomic replace.
//!
//! # The protocol this replicates
//!
//! `defaults.json` is written by gsd-core's installer under
//! `acquireInstallMigrationLock` (`installer-migrations.cjs:499-597`, lock
//! `gsd-install-migration.lock` next to the file): body
//! `{"pid":N,"acquiredAt":"..."}` plus a newline; a dead pid, our own pid or a
//! non-positive pid is reclaimed by unlink (`:470-479`); an unparseable body is
//! never reclaimed; poll 50 ms. The wait is bounded to 2 s here
//! ([INFERRED I-guw-2]) because the save runs synchronously on the key-handler
//! thread. Release unlinks only a lock that still carries our body
//! ([INFERRED I-guw-9]).
//!
//! The write itself is a same-directory `O_EXCL` temp file, fsynced, given the
//! target's permission bits, and renamed over the target, so a reader sees the
//! old or the new file and never a partial one.
//!
//! Everything here is synchronous on purpose (it is called from sync key
//! handlers); do not add an `async fn` to this module.

#[cfg(unix)]
use std::fs::{self, OpenOptions};
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

#[cfg(unix)]
use rustix::io::Errno;
#[cfg(unix)]
use serde::Serialize;

use super::config_json::GsdConfig;
#[cfg(unix)]
use super::config_json::{parse_gsd_config, serialize_gsd_config};

/// Cap on a lock body read (a lock body is ~100 bytes).
#[cfg(unix)]
const LOCK_BODY_CAP: u64 = 4096;
/// Cap on a config file read inside the lock.
#[cfg(unix)]
pub(crate) const CONFIG_READ_CAP: u64 = 4 * 1024 * 1024;
/// Attempts at a free temp name before giving up.
#[cfg(unix)]
const TEMP_NAME_ATTEMPTS: usize = 16;

#[cfg(unix)]
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How long to wait for a held lock and how often to look again.
#[derive(Debug, Clone, Copy)]
pub struct LockPolicy {
    pub timeout: Duration,
    pub poll: Duration,
}

impl LockPolicy {
    /// The TUI budget: 2 s, at the installer's 50 ms poll ([INFERRED I-guw-2]).
    pub fn tui() -> Self {
        LockPolicy {
            timeout: Duration::from_secs(2),
            poll: Duration::from_millis(50),
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
    pub fn status_text(&self) -> String {
        const SAVED: &str = "Defaults NOT saved";
        match self {
            ConfigWriteError::LockTimeout { holder_pid, .. } => {
                let who = match holder_pid {
                    Some(pid) => format!("pid {pid}"),
                    None => "another process".to_string(),
                };
                format!(
                    "{SAVED}: the GSD installer lock is held by {who} — retry after it finishes"
                )
            }
            ConfigWriteError::Unparseable => {
                format!("{SAVED}: the file on disk no longer parses — press r to reload")
            }
            ConfigWriteError::Io { kind } => format!("{SAVED}: {kind}"),
            ConfigWriteError::Serialize => format!("{SAVED}: the config could not be serialised"),
        }
    }
}

#[cfg(unix)]
#[derive(Serialize)]
struct InstallLockBody {
    pid: u32,
    #[serde(rename = "acquiredAt")]
    acquired_at: String,
}

#[cfg(unix)]
fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Read a file safely: it may be planted by anything running in the project
/// (the Phase 27 CR-01 class, [INFERRED I-guw-6]). `O_NONBLOCK` always (a FIFO
/// with no writer cannot hang the open), `O_NOFOLLOW` unless `follow_symlinks`,
/// a regular-file check on the open handle, and a `take(cap + 1)` bound. Any
/// failed check is an error, which callers treat as "unreadable".
#[cfg(unix)]
pub(crate) fn read_bounded(
    path: &Path,
    cap: u64,
    follow_symlinks: bool,
) -> std::io::Result<Vec<u8>> {
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
#[cfg(unix)]
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
#[cfg(unix)]
fn probe_pid(pid: i64) -> Option<Result<(), Errno>> {
    // `Pid::from_raw` panics on a negative value, so reject it here.
    let raw = i32::try_from(pid).ok().filter(|r| *r > 0)?;
    let p = rustix::process::Pid::from_raw(raw)?;
    Some(rustix::process::test_kill_process(p))
}

/// An acquired lock. Dropping it releases the lock, but only if the file still
/// carries the body this guard wrote ([INFERRED I-guw-9]).
#[cfg(unix)]
struct LockGuard {
    path: PathBuf,
    body: Vec<u8>,
}

#[cfg(unix)]
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

#[cfg(unix)]
enum Next {
    /// Look again immediately.
    RetryNow,
    /// Sleep one poll, then look again.
    Wait,
}

/// Create the lock file `O_EXCL` and write the body through the same handle.
/// A failed body write unlinks the file just created.
#[cfg(unix)]
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

#[cfg(unix)]
fn lock_body() -> Vec<u8> {
    let mut v = serde_json::to_vec(&InstallLockBody {
        pid: std::process::id(),
        acquired_at: now_iso(),
    })
    .unwrap_or_default();
    v.push(b'\n');
    v
}

/// The directory holding `path`.
#[cfg(unix)]
fn parent_dir(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// The installer lock that guards the defaults file at `path`.
#[cfg(unix)]
fn install_lock_path(path: &Path) -> PathBuf {
    parent_dir(path).join("gsd-install-migration.lock")
}

/// The installer-lock EEXIST decision (`installer-migrations.cjs:499-597`).
#[cfg(unix)]
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

#[cfg(unix)]
fn acquire(config_path: &Path, policy: LockPolicy) -> Result<LockGuard, ConfigWriteError> {
    let path = install_lock_path(config_path);
    fs::create_dir_all(parent_dir(config_path))
        .map_err(|e| ConfigWriteError::Io { kind: e.kind() })?;

    let start = Instant::now();
    let deadline = start + policy.timeout;
    let mut last_holder: Option<i64> = None;
    loop {
        let body = lock_body();
        let next = match try_create(&path, &body) {
            Ok(()) => return Ok(LockGuard { path, body }),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                install_on_exists(&path, &mut last_holder)
            }
            Err(e) => return Err(ConfigWriteError::Io { kind: e.kind() }),
        };
        let now = Instant::now();
        if now >= deadline {
            return Err(ConfigWriteError::LockTimeout {
                holder_pid: last_holder,
                waited: start.elapsed(),
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
#[cfg(unix)]
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

/// Read-modify-write `$GSD_HOME/.gsd/defaults.json` under gsd-core's installer
/// lock.
///
/// Inside the lock the file is re-read, `mutate` is applied to that FRESH copy
/// (as upstream `setConfigValue` does), and the result is written atomically
/// ([INFERRED I-guw-4]):
/// - the file exists but does not parse: `Err(Unparseable)`, nothing written;
/// - the file is absent: `fallback` is the base (create-on-save);
/// - `mutate` returns `false`: `Ok(None)`, nothing written;
/// - otherwise `Ok(Some(fresh))`, the config now on disk.
#[cfg(unix)]
pub fn update_global_defaults(
    path: &Path,
    policy: LockPolicy,
    fallback: &GsdConfig,
    mutate: impl FnOnce(&mut GsdConfig) -> bool,
) -> Result<Option<GsdConfig>, ConfigWriteError> {
    let _guard = acquire(path, policy)?;
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

/// Off Unix there is no gsd-core-compatible lock protocol here (it rests on
/// `O_EXCL` lock files, `kill(pid, 0)` liveness, `O_NOFOLLOW`/`O_NONBLOCK` reads
/// and inode identity), and a save without that protocol could corrupt a file
/// GSD is writing. So this refuses before any I/O: the file is never opened,
/// `mutate` is never called, and the Defaults tab shows
/// `Defaults NOT saved: unsupported` through [`ConfigWriteError::status_text`].
#[cfg(not(unix))]
pub fn atomic_write(_path: &Path, _bytes: &[u8]) -> std::io::Result<()> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

/// Off Unix a defaults save is refused (see [`atomic_write`]): returns
/// `Err(Io { kind: Unsupported })` before any file I/O and never calls `mutate`.
#[cfg(not(unix))]
pub fn update_global_defaults(
    _path: &Path,
    _policy: LockPolicy,
    _fallback: &GsdConfig,
    _mutate: impl FnOnce(&mut GsdConfig) -> bool,
) -> Result<Option<GsdConfig>, ConfigWriteError> {
    Err(ConfigWriteError::Io {
        kind: std::io::ErrorKind::Unsupported,
    })
}

#[cfg(all(test, unix))]
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

    /// `<td>/.gsd/defaults.json`, its directory created.
    fn global(td: &TempDir) -> PathBuf {
        fs::create_dir_all(td.path().join(".gsd")).unwrap();
        td.path().join(".gsd/defaults.json")
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
    fn install_lock_rules() {
        // A dead pid and our own pid are reclaimed.
        for pid in [DEAD_PID, i64::from(std::process::id())] {
            let td = TempDir::new().unwrap();
            let path = global(&td);
            fs::write(
                install_lock_path(&path),
                format!("{{\"pid\":{pid},\"acquiredAt\":\"2026-10-19T00:00:00.000Z\"}}\n"),
            )
            .unwrap();
            update_global_defaults(&path, policy(1000), &GsdConfig::default(), set_commit_docs)
                .unwrap_or_else(|e| panic!("pid {pid} must be reclaimed, got {e:?}"));
            assert_eq!(
                names(&td.path().join(".gsd")),
                vec!["defaults.json".to_string()]
            );
        }

        // An unparseable body is never reclaimed.
        let td = TempDir::new().unwrap();
        let path = global(&td);
        fs::write(install_lock_path(&path), "garbage").unwrap();
        let err =
            update_global_defaults(&path, policy(300), &GsdConfig::default(), set_commit_docs)
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
        assert_eq!(
            fs::read_to_string(install_lock_path(&path)).unwrap(),
            "garbage"
        );
        assert!(!path.exists());

        // A live pid is waited on.
        let td = TempDir::new().unwrap();
        let path = global(&td);
        let live = "{\"pid\":1,\"acquiredAt\":\"2026-10-19T00:00:00.000Z\"}\n";
        fs::write(install_lock_path(&path), live).unwrap();
        let err =
            update_global_defaults(&path, policy(300), &GsdConfig::default(), set_commit_docs)
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
        assert_eq!(fs::read_to_string(install_lock_path(&path)).unwrap(), live);

        // The body we write.
        let td = TempDir::new().unwrap();
        let path = global(&td);
        let lock = install_lock_path(&path);
        let mut seen = String::new();
        update_global_defaults(&path, policy(1000), &GsdConfig::default(), |c| {
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
        let path = global(&td);
        fs::write(
            &path,
            r#"{"mode":"interactive","brand_new_gsd_key":{"a":1}}"#,
        )
        .unwrap();
        let fallback = parse_gsd_config(r#"{"mode":"yolo"}"#).unwrap();
        let fresh = update_global_defaults(&path, policy(1000), &fallback, set_commit_docs)
            .unwrap()
            .expect("written");
        assert_eq!(fresh.mode, "interactive");
        let text = fs::read_to_string(&path).unwrap();
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
        let path = global(&td);
        fs::write(&path, "{not json").unwrap();
        let err =
            update_global_defaults(&path, policy(1000), &GsdConfig::default(), set_commit_docs)
                .unwrap_err();
        assert_eq!(err, ConfigWriteError::Unparseable);
        assert_eq!(fs::read(&path).unwrap(), b"{not json");
        assert!(
            !install_lock_path(&path).exists(),
            "the lock is released on refusal"
        );
    }

    #[test]
    fn update_creates_an_absent_file_from_the_fallback() {
        let td = TempDir::new().unwrap();
        let path = global(&td);
        assert!(!path.exists());
        let fallback = parse_gsd_config(r#"{"mode":"yolo"}"#).unwrap();
        update_global_defaults(&path, policy(1000), &fallback, set_commit_docs).unwrap();
        let on_disk = parse_gsd_config(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(on_disk.mode, "yolo");
        assert_eq!(on_disk.commit_docs, Some(false));
    }

    #[test]
    fn release_does_not_delete_a_lock_it_no_longer_owns() {
        let td = TempDir::new().unwrap();
        let path = global(&td);
        let lock = install_lock_path(&path);
        let foreign = "{\"pid\":1,\"acquiredAt\":\"2026-10-19T00:00:00.000Z\"}\n";
        update_global_defaults(&path, policy(1000), &GsdConfig::default(), |c| {
            fs::remove_file(&lock).unwrap();
            fs::write(&lock, foreign).unwrap();
            set_commit_docs(c)
        })
        .unwrap();
        assert_eq!(fs::read_to_string(&lock).unwrap(), foreign);
    }

    /// gsd-core v1.23.0 (#5096) reads a top-level `"workflow": null` as an
    /// inheritance barrier, so a save must not add a section the file lacks.
    /// Project `config.json` saves go through `gsd-tools config-set` (quick
    /// 261116-kft); the defaults file is the writer this guard still covers.
    #[test]
    fn a_save_does_not_add_sections_the_file_lacks() {
        let td = TempDir::new().unwrap();
        let path = global(&td);
        fs::write(&path, r#"{"mode":"yolo"}"#).unwrap();
        update_global_defaults(&path, policy(1000), &GsdConfig::default(), |c| {
            c.commit_docs = Some(true);
            true
        })
        .unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut keys: Vec<&str> = saved
            .as_object()
            .expect("an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["commit_docs", "mode"], "{saved}");
    }

    #[test]
    fn update_without_a_mutation_writes_nothing() {
        let td = TempDir::new().unwrap();
        let path = global(&td);
        fs::write(&path, r#"{"mode":"yolo"}"#).unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let got =
            update_global_defaults(&path, policy(1000), &GsdConfig::default(), |_| false).unwrap();
        assert!(got.is_none());
        assert_eq!(fs::read(&path).unwrap(), br#"{"mode":"yolo"}"#);
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        assert!(!install_lock_path(&path).exists());
    }

    #[test]
    fn status_text_carries_no_path_and_names_only_the_pid() {
        let timeout = ConfigWriteError::LockTimeout {
            holder_pid: Some(4242),
            waited: Duration::from_secs(2),
        };
        let t = timeout.status_text();
        assert!(
            t.starts_with("Defaults NOT saved") && t.contains("pid 4242"),
            "{t}"
        );
        let unknown = ConfigWriteError::LockTimeout {
            holder_pid: None,
            waited: Duration::ZERO,
        };
        assert!(unknown.status_text().contains("another process"));
        let io = ConfigWriteError::Io {
            kind: std::io::ErrorKind::PermissionDenied,
        };
        assert!(!io.status_text().contains('/'));
    }
}
