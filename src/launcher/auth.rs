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
//! This is the read path; the guided `driver auth setup` that writes the stored
//! file is 28-06's.

use std::os::unix::fs::MetadataExt;
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
    /// A read fault.
    Io { detail: String },
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
            AuthError::Io { detail } => write!(f, "could not read the token: {detail}"),
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
}
