#![cfg(target_os = "linux")]
// ============================================================================
// LIVE D-11 probe: how far does the stored `claude setup-token` token reach on
// the server side? (Phase 28.1, plan 28.1-07; T-28.1-31.) Opt-in: `#[ignore]`.
//
//   gsd-meta-manager driver auth setup      # mints and stores the token (a human)
//   cargo test --test launcher_live_token_reach -- --ignored --nocapture
//
// The proxy confines a run to `POST /v1/messages`, so what the account-scoped
// endpoints would answer a setup-token matters only as a second line. This probe
// sends four read-only GETs straight to `api.anthropic.com` with the stored token
// and prints `<path>=<status>`; the same four with the host login token follow as
// `control <path>=<status>` (a positive control: a 2xx there shows the endpoint is
// reachable at all, so a refusal for the setup-token is not vacuous). No token is
// ever printed. With no stored token the probe prints `NOT-RUN: no setup-token`.
//
// Direct HTTP carries none of the client-side gates the forged
// `CLAUDE_CODE_OAUTH_SCOPES` of D-11 would defeat, so it is not set
// `[inferred — audit]`.
// ============================================================================

use gsd_meta_manager::launcher::auth::{host_access_token, read_stored_token, token_path};
use gsd_meta_manager::launcher::egress::upstream::{Connector, Upstream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The four paths: the connectors list, the routines (triggers), the cloud sessions
/// and the profile the Remote Control eligibility check reads. The installed claude
/// bundle (2.1.310) holds no separate eligibility endpoint (its strings name only
/// `/api/oauth/profile`, `/api/oauth/organizations/...` and `/api/oauth/usage`), so
/// the profile path stands in for it `[inferred — audit]`.
const PATHS: [&str; 4] = [
    "/v1/mcp_servers?limit=1",
    "/v1/code/triggers",
    "/v1/code/sessions",
    "/api/oauth/profile",
];

/// `GET path` on `api.anthropic.com` with `Authorization: Bearer <token>`; the HTTP
/// status, or `0` when no status line came back.
async fn get_status(connector: &Connector, path: &str, token: &str) -> u16 {
    let Ok(mut stream) = connector.connect().await else {
        return 0;
    };
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: api.anthropic.com\r\nAuthorization: Bearer {token}\r\n\
         anthropic-beta: oauth-2025-04-20\r\nanthropic-version: 2023-06-01\r\n\
         Accept: application/json\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(request.as_bytes()).await.is_err() {
        return 0;
    }
    let mut head = Vec::new();
    let mut chunk = [0u8; 512];
    // The status line is in the first bytes; stop once it is complete.
    while !head.contains(&b'\n') && head.len() < 4096 {
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => head.extend_from_slice(&chunk[..n]),
        }
    }
    String::from_utf8_lossy(&head)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0)
}

#[test]
#[ignore = "reads the stored setup-token and the host token; read-only GETs, no spend; run with -- --ignored --nocapture"]
fn setup_token_reach_is_inference_only() {
    let stored = token_path().and_then(|path| read_stored_token(&path).ok().flatten());
    let host = dirs::home_dir()
        .and_then(|home| host_access_token(&home.join(".claude"), chrono::Utc::now()).ok())
        .map(|token| token.value);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a tokio runtime");
    let connector = Connector::new(Upstream::anthropic()).expect("the TLS connector");

    let Some(stored) = stored else {
        println!("NOT-RUN: no setup-token");
        // The controls are read-only and cost nothing; they show what the host token
        // reaches while the setup-token half waits for a human-minted token.
        if let Some(host) = host {
            for path in PATHS {
                let status = runtime.block_on(get_status(&connector, path, &host));
                println!("control {path}={status}");
            }
        }
        return;
    };

    let mut setup_statuses = Vec::new();
    for path in PATHS {
        let status = runtime.block_on(get_status(&connector, path, &stored));
        println!("{path}={status}");
        setup_statuses.push((path, status));
    }
    match host {
        Some(host) => {
            for path in PATHS {
                let status = runtime.block_on(get_status(&connector, path, &host));
                println!("control {path}={status}");
            }
        }
        None => println!("control: NOT-RUN: no host token"),
    }
    for (path, status) in setup_statuses {
        assert!(
            !(200..300).contains(&status),
            "the setup-token reached {path} (status {status})"
        );
    }
}
