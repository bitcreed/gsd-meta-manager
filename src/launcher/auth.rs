//! The OAuth token a launch hands the driver (Phase 28; D-22, LNCH-05, T-28-03).
//!
//! Resolution order is fixed (D-22): the token `claude setup-token` printed and
//! the operator stored (0600, owner-checked), else the host's current
//! `claudeAiOauth.accessToken`, refused under [`MIN_TOKEN_LIFETIME`] to expiry.
//! Nothing else is consulted.
//!
//! **Only the access token is ever read from the host file.** Spike 002 (A2.4)
//! measured that a copied refresh token logs the host out the first time either
//! side refreshes, so [`host_access_token`] parses `accessToken` and `expiresAt`
//! and keeps no other field.
//!
//! **A token reaches the child only as `CLAUDE_CODE_OAUTH_TOKEN` in its
//! environment** (T-28-03): not argv (world-readable in `/proc`), not a file, not
//! the run state. [`LaunchToken`]'s `Debug` prints `<redacted>`, so a stray
//! `{:?}` cannot leak it either. This module has no `Debug` derive on any type
//! that holds a value.
//!
//! The write path (28-06, D-21) is [`validate_pasted_token`] and [`store_token`]
//! behind `driver auth setup`, and [`auth_status`] behind `driver auth status`,
//! which reports presence, path, mode and the host token's expiry and never a
//! value (T-28-07).

use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

/// The stored token's file name under `dirs::config_dir()/gsd-meta-manager/`.
pub const TOKEN_FILE_NAME: &str = "driver-oauth-token";

/// A host access token with less than this left is refused (spike value).
pub const MIN_TOKEN_LIFETIME: Duration = Duration::minutes(5);

/// Where the stored token lives, or `None` when the platform has no config dir.
pub fn token_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("gsd-meta-manager").join(TOKEN_FILE_NAME))
}

/// Where a token came from. The consumers branch on this (28-05's revocation
/// handling does), never on "the host token".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    /// The operator-stored `claude setup-token` token.
    Stored,
    /// The host's `claudeAiOauth.accessToken`.
    Host,
}

/// A token for one launch.
#[derive(Clone, PartialEq, Eq)]
pub struct LaunchToken {
    /// The secret. Never logged, never in argv, never written.
    pub value: String,
    pub source: TokenSource,
    /// When the token expires, when that is known (the host token's is).
    pub expires_at: Option<DateTime<Utc>>,
}

impl std::fmt::Debug for LaunchToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LaunchToken {{ value: <redacted>, source: {:?}, expires_at: {:?} }}",
            self.source, self.expires_at
        )
    }
}

/// Why no token could be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Neither a stored token nor a host credentials file.
    Missing,
    /// The stored token file is not a private regular file of this user.
    Unsafe { detail: String },
    /// The host token has too little life left.
    Expiring { minutes_left: i64 },
    /// A file that does not hold what it should.
    Malformed { detail: String },
    /// A read or write fault.
    Io { detail: String },
    /// A pasted token that is not acceptable. The detail never repeats the value.
    Invalid { detail: String },
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Missing => f.write_str(
                "no driver token: run `gsd-meta-manager driver auth setup` to store one \
                 (or sign in to Claude Code on this host)",
            ),
            AuthError::Unsafe { detail } => write!(f, "refusing the stored token: {detail}"),
            AuthError::Expiring { minutes_left } => write!(
                f,
                "the host access token expires in {minutes_left} min; run any host `claude` \
                 command to refresh it, or run `gsd-meta-manager driver auth setup`"
            ),
            AuthError::Malformed { detail } => write!(f, "unusable token source: {detail}"),
            AuthError::Io { detail } => write!(f, "token file fault: {detail}"),
            AuthError::Invalid { detail } => {
                write!(f, "the pasted token is not acceptable: {detail}")
            }
        }
    }
}

impl std::error::Error for AuthError {}

/// The stored token at `path`: `Ok(None)` when there is none.
///
/// A symlink, a non-regular file, a file owned by another user or one readable by
/// group or others is [`AuthError::Unsafe`] (the `socket_path_in` owner/mode
/// rule). The content is trimmed; an empty file is [`AuthError::Malformed`].
pub fn read_stored_token(path: &Path) -> Result<Option<String>, AuthError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(AuthError::Io {
                detail: format!("{}: {e}", path.display()),
            })
        }
    };
    if !meta.file_type().is_file() {
        return Err(AuthError::Unsafe {
            detail: format!("{} is not a regular file", path.display()),
        });
    }
    if meta.uid() != rustix::process::getuid().as_raw() {
        return Err(AuthError::Unsafe {
            detail: format!("{} is not owned by the current user", path.display()),
        });
    }
    if meta.mode() & 0o077 != 0 {
        return Err(AuthError::Unsafe {
            detail: format!(
                "{} must not be accessible to group or others (mode {:o}); chmod 600 it",
                path.display(),
                meta.mode() & 0o777
            ),
        });
    }
    let text = std::fs::read_to_string(path).map_err(|e| AuthError::Io {
        detail: format!("{}: {e}", path.display()),
    })?;
    let token = text.trim();
    if token.is_empty() {
        return Err(AuthError::Malformed {
            detail: format!("{} is empty", path.display()),
        });
    }
    Ok(Some(token.to_owned()))
}

/// The host's current access token, from `<host_claude_dir>/.credentials.json`.
///
/// Reads `claudeAiOauth.accessToken` and `claudeAiOauth.expiresAt` (epoch
/// milliseconds) and nothing else: the refresh token never enters this process's
/// memory as a value. Refused when less than [`MIN_TOKEN_LIFETIME`] remains at
/// `now`.
pub fn host_access_token(
    host_claude_dir: &Path,
    now: DateTime<Utc>,
) -> Result<LaunchToken, AuthError> {
    let path = host_claude_dir.join(".credentials.json");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(AuthError::Missing),
        Err(e) => {
            return Err(AuthError::Io {
                detail: format!("{}: {e}", path.display()),
            })
        }
    };
    let malformed = |what: &str| AuthError::Malformed {
        detail: format!("{}: {what}", path.display()),
    };
    let doc: Value = serde_json::from_slice(&bytes).map_err(|_| malformed("not valid JSON"))?;
    let oauth = doc
        .get("claudeAiOauth")
        .ok_or_else(|| malformed("no claudeAiOauth entry"))?;
    let value = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| malformed("no accessToken"))?
        .trim()
        .to_owned();
    let expires_ms = oauth
        .get("expiresAt")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .ok_or_else(|| malformed("no expiresAt"))?;
    let expires_at = DateTime::<Utc>::from_timestamp_millis(expires_ms)
        .ok_or_else(|| malformed("expiresAt is out of range"))?;
    let left = expires_at - now;
    if left < MIN_TOKEN_LIFETIME {
        return Err(AuthError::Expiring {
            minutes_left: left.num_minutes(),
        });
    }
    Ok(LaunchToken {
        value,
        source: TokenSource::Host,
        expires_at: Some(expires_at),
    })
}

/// The token for a launch at `now`: stored first, else the host's, else
/// [`AuthError::Missing`].
pub fn resolve_launch_token(now: DateTime<Utc>) -> Result<LaunchToken, AuthError> {
    if let Some(stored) = token_path()
        .map(|p| read_stored_token(&p))
        .transpose()?
        .flatten()
    {
        return Ok(LaunchToken {
            value: stored,
            source: TokenSource::Stored,
            expires_at: None,
        });
    }
    let home = dirs::home_dir().ok_or(AuthError::Missing)?;
    host_access_token(&home.join(".claude"), now)
}

/// The shortest pasted token accepted (characters).
pub const MIN_PASTED_LEN: usize = 20;

/// The longest pasted token accepted (characters).
pub const MAX_PASTED_LEN: usize = 512;

/// What a `claude setup-token` token starts with. A value that does not is
/// accepted (the shape is not a published contract) and the caller warns
/// `[inferred — audit]`.
pub const EXPECTED_TOKEN_PREFIX: &str = "sk-ant-oat01-";

/// Whether `token` has the shape `claude setup-token` prints.
pub fn has_expected_prefix(token: &str) -> bool {
    token.starts_with(EXPECTED_TOKEN_PREFIX)
}

/// The token in one pasted line, or why it is refused (D-21, T-28-07).
///
/// Surrounding whitespace (the newline `read_line` keeps) is trimmed; after that
/// the value must be one line of printable ASCII with no whitespace or control
/// character inside, between [`MIN_PASTED_LEN`] and [`MAX_PASTED_LEN`]. Nothing
/// is written here, and no error text repeats the value.
pub fn validate_pasted_token(raw: &str) -> Result<String, AuthError> {
    let invalid = |detail: &str| AuthError::Invalid {
        detail: detail.to_owned(),
    };
    let token = raw.trim();
    if token.is_empty() {
        return Err(invalid("nothing was pasted"));
    }
    if token.contains('\n') {
        return Err(invalid(
            "more than one line was pasted; paste only the token",
        ));
    }
    if token.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid(
            "it contains whitespace or a control character; paste only the token",
        ));
    }
    if !token.chars().all(|c| c.is_ascii_graphic()) {
        return Err(invalid("it contains a character outside printable ASCII"));
    }
    let len = token.chars().count();
    if !(MIN_PASTED_LEN..=MAX_PASTED_LEN).contains(&len) {
        return Err(AuthError::Invalid {
            detail: format!(
                "its length ({len}) is outside {MIN_PASTED_LEN}..={MAX_PASTED_LEN} characters"
            ),
        });
    }
    Ok(token.to_owned())
}

/// Store `token` at `path`: 0600, atomically, in a directory only this user can
/// write (D-21, T-28-07).
///
/// The parent directory is created 0700 when absent. An existing one must be
/// owned by this user (else [`AuthError::Unsafe`]); group and other write bits are
/// removed from it, but nothing else is changed: the default directory also holds
/// the registry (`config.json`), commonly 0755 or 0775, and the token file's own
/// 0600 is what protects the value `[inferred — audit]`.
///
/// The value goes to a sibling temp file created 0600 (never more permissive, even
/// for an instant, which is what the plan's `umask(0o177)` would buy, without
/// touching process-wide state), is fsynced and then renamed into place, so a
/// reader sees the old file or the whole new one. The value is never logged.
pub fn store_token(path: &Path, token: &str) -> Result<(), AuthError> {
    let token = validate_pasted_token(token)?;
    let io = |what: &str, e: std::io::Error| AuthError::Io {
        detail: format!("{what}: {e}"),
    };
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .ok_or_else(|| AuthError::Io {
            detail: format!("{} has no parent directory", path.display()),
        })?;
    let name = path
        .file_name()
        .ok_or_else(|| AuthError::Io {
            detail: format!("{} has no file name", path.display()),
        })?
        .to_string_lossy()
        .into_owned();

    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| io(&format!("create {}", dir.display()), e))?;
    // `metadata` follows a symlink: a config dir kept in a dotfiles checkout is
    // legitimate, and what matters is who can write the directory it resolves to.
    let meta = std::fs::metadata(dir).map_err(|e| io(&dir.display().to_string(), e))?;
    if !meta.is_dir() {
        return Err(AuthError::Unsafe {
            detail: format!("{} is not a directory", dir.display()),
        });
    }
    if meta.uid() != rustix::process::getuid().as_raw() {
        return Err(AuthError::Unsafe {
            detail: format!("{} is not owned by the current user", dir.display()),
        });
    }
    if meta.mode() & 0o022 != 0 {
        // A group-writable directory is the default under a 0002 umask (Debian
        // and Ubuntu). Others could swap the file by renaming over it, so take
        // the write bits away from a directory this user owns; read and search
        // bits are kept for whatever else lives there (the registry).
        let tightened = meta.mode() & 0o7755;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(tightened)).map_err(|e| {
            io(
                &format!("remove group/other write from {}", dir.display()),
                e,
            )
        })?;
    }

    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let written = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp)?;
        // A stricter umask only narrows the creation mode; this pins exactly 0600.
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(token.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(io(&format!("write {}", path.display()), e));
    }
    if let Ok(dir_file) = std::fs::File::open(dir) {
        let _ = dir_file.sync_all();
    }
    Ok(())
}

/// What `driver auth status` reports. It holds no token value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStatus {
    /// Where the stored token would be, when the platform has a config dir.
    pub path: Option<PathBuf>,
    /// The stored token's path and permission bits, when the file exists.
    pub stored: Option<(PathBuf, u32)>,
    /// Why a stored file would be refused at launch, when it would be.
    pub stored_problem: Option<String>,
    /// The host token's `expiresAt`, when the host credentials file has one.
    pub host_expires_at: Option<DateTime<Utc>>,
    /// Whole minutes the host token has left at the status time (negative when
    /// it has expired).
    pub host_minutes_left: Option<i64>,
}

impl AuthStatus {
    /// Where the next launch would get its token, or why it would not.
    pub fn next_launch(&self) -> &'static str {
        if self.stored_problem.is_some() {
            "none: the stored token file is refused (fix it or rerun `driver auth setup`)"
        } else if self.stored.is_some() {
            "the stored setup-token"
        } else if self
            .host_minutes_left
            .is_some_and(|m| m >= MIN_TOKEN_LIFETIME.num_minutes())
        {
            "the host access token"
        } else {
            "none: run `gsd-meta-manager driver auth setup`"
        }
    }
}

/// The host's token expiry, from `<host_claude_dir>/.credentials.json`.
///
/// Typed on `expiresAt` alone: serde skips every other field, the access and
/// refresh tokens included, without keeping them.
fn host_expiry(host_claude_dir: &Path) -> Option<DateTime<Utc>> {
    #[derive(serde::Deserialize)]
    struct File {
        #[serde(rename = "claudeAiOauth")]
        oauth: Option<Oauth>,
    }
    #[derive(serde::Deserialize)]
    struct Oauth {
        #[serde(rename = "expiresAt")]
        expires_at: Option<f64>,
    }
    let bytes = std::fs::read(host_claude_dir.join(".credentials.json")).ok()?;
    let file: File = serde_json::from_slice(&bytes).ok()?;
    DateTime::<Utc>::from_timestamp_millis(file.oauth?.expires_at? as i64)
}

/// [`auth_status`] with the paths given, for tests.
pub fn auth_status_in(
    token_path: Option<&Path>,
    host_claude_dir: Option<&Path>,
    now: DateTime<Utc>,
) -> AuthStatus {
    let (stored, stored_problem) = match token_path {
        None => (None, None),
        Some(path) => match std::fs::symlink_metadata(path) {
            Err(_) => (None, None),
            Ok(meta) => {
                let found = Some((path.to_owned(), meta.mode() & 0o777));
                match read_stored_token(path) {
                    Ok(_) => (found, None),
                    Err(e) => (found, Some(e.to_string())),
                }
            }
        },
    };
    let host_expires_at = host_claude_dir.and_then(host_expiry);
    AuthStatus {
        path: token_path.map(Path::to_owned),
        stored,
        stored_problem,
        host_expires_at,
        host_minutes_left: host_expires_at.map(|at| (at - now).num_minutes()),
    }
}

/// The token situation on this host at `now`: the stored file and the host
/// token's expiry. Read-only, and no value is kept.
pub fn auth_status(now: DateTime<Utc>) -> AuthStatus {
    let path = token_path();
    let host = dirs::home_dir().map(|h| h.join(".claude"));
    auth_status_in(path.as_deref(), host.as_deref(), now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn write_mode(dir: &TempDir, name: &str, body: &str, mode: u32) -> PathBuf {
        let path = dir.path().join(name);
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn creds(dir: &TempDir, expires_in: Duration) -> PathBuf {
        let expires = (now() + expires_in).timestamp_millis();
        std::fs::write(
            dir.path().join(".credentials.json"),
            serde_json::json!({"claudeAiOauth": {
                "accessToken": "at-secret",
                "refreshToken": "rt-must-not-be-read",
                "expiresAt": expires,
            }})
            .to_string(),
        )
        .unwrap();
        dir.path().to_owned()
    }

    #[test]
    fn an_absent_stored_token_is_none() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_stored_token(&dir.path().join("nope")).unwrap(), None);
    }

    #[test]
    fn a_private_owned_token_is_read_and_trimmed() {
        let dir = TempDir::new().unwrap();
        let path = write_mode(&dir, "t", "  tok-1234  \n", 0o600);
        assert_eq!(
            read_stored_token(&path).unwrap().as_deref(),
            Some("tok-1234")
        );
    }

    #[test]
    fn a_group_or_world_readable_token_is_refused() {
        let dir = TempDir::new().unwrap();
        for mode in [0o640, 0o604, 0o644, 0o660] {
            let path = write_mode(&dir, "t", "tok", mode);
            assert!(
                matches!(read_stored_token(&path), Err(AuthError::Unsafe { .. })),
                "mode {mode:o}"
            );
        }
    }

    #[test]
    fn a_symlinked_or_directory_token_is_refused() {
        let dir = TempDir::new().unwrap();
        let real = write_mode(&dir, "real", "tok", 0o600);
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert!(matches!(
            read_stored_token(&link),
            Err(AuthError::Unsafe { .. })
        ));
        assert!(matches!(
            read_stored_token(dir.path()),
            Err(AuthError::Unsafe { .. })
        ));
    }

    #[test]
    fn an_empty_stored_token_is_malformed() {
        let dir = TempDir::new().unwrap();
        let path = write_mode(&dir, "t", " \n", 0o600);
        assert!(matches!(
            read_stored_token(&path),
            Err(AuthError::Malformed { .. })
        ));
    }

    #[test]
    fn the_host_token_is_read_with_its_expiry_and_nothing_else() {
        let dir = TempDir::new().unwrap();
        let host = creds(&dir, Duration::hours(2));
        let token = host_access_token(&host, now()).unwrap();
        assert_eq!(token.value, "at-secret");
        assert_eq!(token.source, TokenSource::Host);
        assert_eq!(token.expires_at, Some(now() + Duration::hours(2)));
        let shown = format!("{token:?}");
        assert!(shown.contains("<redacted>"));
        assert!(!shown.contains("at-secret") && !shown.contains("rt-must-not-be-read"));
    }

    #[test]
    fn a_host_token_under_five_minutes_is_refused() {
        let dir = TempDir::new().unwrap();
        let host = creds(&dir, Duration::minutes(4));
        match host_access_token(&host, now()) {
            Err(AuthError::Expiring { minutes_left }) => assert_eq!(minutes_left, 4),
            other => panic!("expected Expiring, got {other:?}"),
        }
        let dir = TempDir::new().unwrap();
        let host = creds(&dir, Duration::minutes(6));
        assert!(host_access_token(&host, now()).is_ok());
    }

    #[test]
    fn a_missing_credentials_file_is_missing_and_names_the_setup_command() {
        let dir = TempDir::new().unwrap();
        let err = host_access_token(dir.path(), now()).unwrap_err();
        assert_eq!(err, AuthError::Missing);
        assert!(err
            .to_string()
            .contains("gsd-meta-manager driver auth setup"));
    }

    #[test]
    fn a_credentials_file_without_the_oauth_entry_is_malformed() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join(".credentials.json"), "{}").unwrap();
        assert!(matches!(
            host_access_token(dir.path(), now()),
            Err(AuthError::Malformed { .. })
        ));
    }
    // ----------------------------------------------------- the write path (28-06)

    const GOOD: &str = "sk-ant-oat01-abcdefghij0123456789";

    #[test]
    fn a_pasted_token_is_trimmed() {
        assert_eq!(
            validate_pasted_token(&format!("  {GOOD}  \n")).unwrap(),
            GOOD
        );
    }

    #[test]
    fn a_pasted_token_that_is_not_one_clean_line_is_refused_without_echoing_it() {
        let long = "a".repeat(MAX_PASTED_LEN + 1);
        let cases: Vec<(String, &str)> = vec![
            (String::new(), "empty"),
            ("   \n".to_owned(), "whitespace only"),
            ("sk-ant-oat01-abc def0123456789".to_owned(), "inner space"),
            (format!("{GOOD}\r"), "trailing CR is trimmed so this passes"),
            ("sk-ant-oat01-abc\rdef0123456789".to_owned(), "inner CR"),
            (
                "sk-ant-oat01-abc\x1b[0mdef012345".to_owned(),
                "escape sequence",
            ),
            (format!("{GOOD}\nsecond-line-is-here-too"), "two lines"),
            ("sk-ant-oat01-12345".to_owned(), "19 characters"),
            ("sk-ant-oat01-éééééééééééé".to_owned(), "non-ascii"),
            (long, "too long"),
        ];
        for (raw, why) in cases {
            let result = validate_pasted_token(&raw);
            if why.starts_with("trailing CR") {
                assert_eq!(result.unwrap(), GOOD, "{why}");
                continue;
            }
            let err = result.expect_err(why);
            assert!(matches!(err, AuthError::Invalid { .. }), "{why}: {err:?}");
            let shown = err.to_string();
            let secret = raw.trim();
            assert!(
                secret.is_empty() || !shown.contains(secret),
                "{why}: the refusal repeats the value"
            );
        }
    }

    #[test]
    fn a_nineteen_character_token_is_refused_and_twenty_is_accepted() {
        assert!(validate_pasted_token(&"x".repeat(19)).is_err());
        assert!(validate_pasted_token(&"x".repeat(20)).is_ok());
        assert!(validate_pasted_token(&"x".repeat(MAX_PASTED_LEN)).is_ok());
    }

    #[test]
    fn the_expected_prefix_is_recognised_but_not_required() {
        assert!(has_expected_prefix(GOOD));
        assert!(!has_expected_prefix("tok-test-0123456789abcdef"));
        assert!(validate_pasted_token("tok-test-0123456789abcdef").is_ok());
    }

    #[test]
    fn a_stored_token_is_0600_and_reads_back_through_the_read_path() {
        let dir = TempDir::new().unwrap();
        let path = dir
            .path()
            .join("cfg/gsd-meta-manager")
            .join(TOKEN_FILE_NAME);
        store_token(&path, GOOD).unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(path.parent().unwrap()), 0o700, "a created dir is 0700");
        assert_eq!(read_stored_token(&path).unwrap().as_deref(), Some(GOOD));
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, [TOKEN_FILE_NAME], "no temp file is left behind");
    }

    #[test]
    fn storing_replaces_an_older_token_and_resets_its_mode() {
        let dir = TempDir::new().unwrap();
        // Whatever the test runner's umask made of the temp dir.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = write_mode(&dir, TOKEN_FILE_NAME, "old-token-0123456789abcdef", 0o644);
        store_token(&path, GOOD).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert_eq!(read_stored_token(&path).unwrap().as_deref(), Some(GOOD));
    }

    #[test]
    fn an_invalid_token_writes_nothing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t");
        assert!(store_token(&path, "  \n").is_err());
        assert!(store_token(&path, "has a space inside 0123456789").is_err());
        assert!(!path.exists());
    }

    #[test]
    fn a_directory_others_can_write_to_loses_those_bits_and_nothing_else() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("shared");
        std::fs::create_dir(&sub).unwrap();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o777)).unwrap();
        let path = sub.join("t");
        store_token(&path, GOOD).unwrap();
        assert_eq!(std::fs::metadata(&sub).unwrap().mode() & 0o777, 0o755);
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn a_path_whose_parent_is_a_file_is_an_io_error_and_writes_nothing() {
        let dir = TempDir::new().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        assert!(matches!(
            store_token(&blocker.join("t"), GOOD),
            Err(AuthError::Io { .. })
        ));
    }

    #[test]
    fn an_existing_0755_directory_is_accepted_because_it_also_holds_the_registry() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("gsd-meta-manager");
        std::fs::create_dir(&sub).unwrap();
        std::fs::set_permissions(&sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let path = sub.join("t");
        store_token(&path, GOOD).unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    }

    #[test]
    fn status_reports_presence_path_mode_and_host_expiry_and_no_value() {
        let dir = TempDir::new().unwrap();
        let path = write_mode(&dir, "t", GOOD, 0o600);
        let host = creds(&dir, Duration::hours(2));
        let status = auth_status_in(Some(&path), Some(&host), now());
        assert_eq!(status.stored, Some((path.clone(), 0o600)));
        assert_eq!(status.stored_problem, None);
        assert_eq!(status.host_expires_at, Some(now() + Duration::hours(2)));
        assert_eq!(status.host_minutes_left, Some(120));
        assert_eq!(status.next_launch(), "the stored setup-token");
        let shown = format!("{status:?}");
        for secret in [GOOD, "at-secret", "rt-must-not-be-read"] {
            assert!(!shown.contains(secret), "status holds {secret}");
        }
    }

    #[test]
    fn status_with_only_a_host_token_says_the_host_token_is_next() {
        let dir = TempDir::new().unwrap();
        let host = creds(&dir, Duration::hours(1));
        let status = auth_status_in(Some(&dir.path().join("absent")), Some(&host), now());
        assert_eq!(status.stored, None);
        assert_eq!(status.next_launch(), "the host access token");
        let host = creds(&dir, Duration::minutes(3));
        let status = auth_status_in(None, Some(&host), now());
        assert!(status.next_launch().starts_with("none"));
    }

    #[test]
    fn status_flags_a_stored_file_a_launch_would_refuse() {
        let dir = TempDir::new().unwrap();
        let path = write_mode(&dir, "t", GOOD, 0o640);
        let status = auth_status_in(Some(&path), None, now());
        assert_eq!(status.stored, Some((path, 0o640)));
        assert!(status.stored_problem.is_some());
        assert!(status.next_launch().starts_with("none"));
        let shown = format!("{status:?}");
        assert!(!shown.contains(GOOD));
    }
}
