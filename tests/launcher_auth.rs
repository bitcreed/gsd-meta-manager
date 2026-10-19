#![cfg(target_os = "linux")]
// ============================================================================
// `gsd-meta-manager driver auth setup|status` and the token a launch passes,
// through the REAL binary (Phase 28, plan 28-06; LNCH-05, D-21, D-22, T-28-07).
//
// `claude setup-token` is a fake first on `PATH` that prints a banner and exits;
// the token the "user" pastes arrives on the binary's stdin. Every assertion
// that a secret is absent compares against a value this file chose, so a leak
// into stdout, stderr, argv, the run log or the per-run config shows up as a
// plain `contains`.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::process::{Command, Output, Stdio};

use launcher_support::{write_executable, Fixture, RUN_ID, TOKEN, WELL_MANNERED_SCENARIO};
use sha2::{Digest, Sha256};

/// What a real `claude setup-token` prints: a long-lived token.
const PASTED: &str = "sk-ant-oat01-PASTED0123456789abcdefghij";

/// The host credentials the fixture writes: both values are secrets.
const HOST_ACCESS: &str = "host-access-token-0123456789";
const HOST_REFRESH: &str = "host-REFRESH-token-0123456789";

/// A `claude` whose `setup-token` prints a banner (no token) and exits `code`.
fn setup_token_shim(code: i32) -> String {
    format!(
        "#!/bin/sh\n\
         echo \"claude $*\" >> \"$FAKE_SHIM_LOG\"\n\
         if [ \"$1\" = \"setup-token\" ]; then\n\
           echo 'Opening your browser to authorise a long-lived token...'\n\
           exit {code}\n\
         fi\n\
         exit 0\n"
    )
}

fn install_setup_shim(fx: &Fixture, code: i32) {
    write_executable(&fx.shims(), "claude", &setup_token_shim(code));
}

/// The directory `driver auth setup` creates, absent so the 0700 is ours.
fn forget_token_dir(fx: &Fixture) {
    let _ = fs::remove_dir_all(fx.token_path().parent().unwrap());
}

fn host_credentials(fx: &Fixture, expires_in_minutes: i64) -> String {
    let dir = fx.home().join(".claude");
    fs::create_dir_all(&dir).unwrap();
    let expires = chrono::Utc::now() + chrono::Duration::minutes(expires_in_minutes);
    fs::write(
        dir.join(".credentials.json"),
        serde_json::json!({"claudeAiOauth": {
            "accessToken": HOST_ACCESS,
            "refreshToken": HOST_REFRESH,
            "expiresAt": expires.timestamp_millis(),
        }})
        .to_string(),
    )
    .unwrap();
    expires
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        .chars()
        .take(16)
        .collect()
}

fn auth(fx: &Fixture, verb: &str) -> Command {
    let mut cmd = fx.hermetic_command();
    cmd.args(["driver", "auth", verb]);
    cmd
}

/// Run `cmd` with `stdin` as its standard input.
fn run_with_stdin(mut cmd: Command, stdin: &str) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary is built and spawnable");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn sha(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn setup_stores_a_0600_token_and_never_echoes_it() {
    let fx = Fixture::new();
    install_setup_shim(&fx, 0);
    forget_token_dir(&fx);

    let out = run_with_stdin(auth(&fx, "setup"), &format!("{PASTED}\n"));
    assert!(out.status.success(), "setup failed: {}", text(&out));

    let path = fx.token_path();
    let meta = fs::metadata(&path).expect("the token file exists");
    assert_eq!(meta.mode() & 0o777, 0o600, "the token file mode");
    assert_eq!(meta.uid(), rustix::process::getuid().as_raw(), "ours");
    assert_eq!(
        fs::metadata(path.parent().unwrap()).unwrap().mode() & 0o777,
        0o700,
        "a directory setup creates is 0700"
    );
    assert_eq!(fs::read_to_string(&path).unwrap().trim(), PASTED);

    let shown = text(&out);
    assert!(!shown.contains(PASTED), "the token was echoed:\n{shown}");
    assert!(shown.contains("stored at"), "{shown}");
    assert!(shown.contains(&path.display().to_string()), "{shown}");
    assert!(
        fx.shim_log().contains("claude setup-token"),
        "setup runs `claude setup-token`: {}",
        fx.shim_log()
    );
    assert!(
        shown.contains("long-lived") && shown.contains("host login"),
        "setup explains why: {shown}"
    );
}

#[test]
fn setup_failing_setup_token_writes_nothing() {
    let fx = Fixture::new();
    install_setup_shim(&fx, 1);
    forget_token_dir(&fx);

    let out = run_with_stdin(auth(&fx, "setup"), &format!("{PASTED}\n"));
    assert_ne!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!fx.token_path().exists(), "nothing is stored");
    assert!(!text(&out).contains(PASTED));
}

#[test]
fn setup_with_no_claude_on_the_path_writes_nothing() {
    let fx = Fixture::new();
    fs::remove_file(fx.shims().join("claude")).unwrap();
    forget_token_dir(&fx);

    let out = run_with_stdin(auth(&fx, "setup"), &format!("{PASTED}\n"));
    assert_ne!(out.status.code(), Some(0), "{}", text(&out));
    assert!(!fx.token_path().exists());
    assert!(text(&out).contains("claude setup-token"), "{}", text(&out));
}

#[test]
fn setup_refuses_an_unusable_paste_and_stores_nothing() {
    // LNCH-05 empty (explicit): empty, whitespace-only, multi-line, inner
    // whitespace, a control character and a short value all exit non-zero.
    let cases: [(&str, String); 7] = [
        ("empty line", "\n".to_owned()),
        ("whitespace only", "   \t \n".to_owned()),
        ("end of input", String::new()),
        ("inner space", "sk-ant-oat01-abc def0123456789\n".to_owned()),
        (
            "control character",
            "sk-ant-oat01-abc\u{1b}[0mdef012345\n".to_owned(),
        ),
        ("short", "sk-ant-oat01-12345\n".to_owned()),
        ("two lines", format!("{PASTED}\n{PASTED}-second\n")),
    ];
    for (why, stdin) in cases {
        let fx = Fixture::new();
        install_setup_shim(&fx, 0);
        forget_token_dir(&fx);

        let out = run_with_stdin(auth(&fx, "setup"), &stdin);
        assert_ne!(out.status.code(), Some(0), "{why}: {}", text(&out));
        assert!(!fx.token_path().exists(), "{why}: a file was written");
        assert!(
            !text(&out).contains(PASTED),
            "{why}: the paste was echoed:\n{}",
            text(&out)
        );
    }
}

#[test]
fn setup_warns_about_an_unexpected_shape_but_stores_it() {
    let fx = Fixture::new();
    install_setup_shim(&fx, 0);
    forget_token_dir(&fx);

    let odd = "tok-odd-shape-0123456789abc";
    let out = run_with_stdin(auth(&fx, "setup"), &format!("{odd}\n"));
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("warning"), "{}", text(&out));
    assert!(!text(&out).contains(odd));
    assert_eq!(fs::read_to_string(fx.token_path()).unwrap().trim(), odd);
}

#[test]
fn status_reports_presence_without_the_value() {
    let fx = Fixture::new();
    fx.write_token(Some(PASTED));
    let expiry_prefix = host_credentials(&fx, 120);

    let out = auth(&fx, "status").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let shown = text(&out);
    assert!(
        shown.contains(&fx.token_path().display().to_string()),
        "names the path: {shown}"
    );
    assert!(shown.contains("0600"), "names the mode: {shown}");
    assert!(
        shown.contains(&expiry_prefix),
        "names the host expiry ({expiry_prefix}): {shown}"
    );
    for secret in [PASTED, HOST_ACCESS, HOST_REFRESH] {
        assert!(!shown.contains(secret), "status printed {secret}:\n{shown}");
    }
}

#[test]
fn status_with_nothing_stored_says_so_and_names_the_setup_command() {
    let fx = Fixture::new();
    fx.write_token(None);
    let out = auth(&fx, "status").output().unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let shown = text(&out);
    assert!(shown.contains("none"), "{shown}");
    assert!(shown.contains("driver auth setup"), "{shown}");
    assert!(!fx.token_path().exists(), "status writes nothing");
}

#[test]
fn stored_token_with_group_bits_is_refused_at_launch() {
    // LNCH-05 empty/unsafe (explicit): a stored file open to the group is a typed
    // refusal at launch, never a silent fall back to the host token.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    host_credentials(&fx, 120);
    fs::set_permissions(fx.token_path(), fs::Permissions::from_mode(0o640)).unwrap();

    let out = fx.launch(&[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    let shown = text(&out);
    assert!(
        shown.contains(&fx.token_path().display().to_string()),
        "names the file: {shown}"
    );
    assert!(
        fx.recorded("env.txt").is_empty(),
        "the fake claude must never run: no host fallback"
    );
    assert!(fx.shim_log().is_empty(), "nothing was started");

    let status = auth(&fx, "status").output().unwrap();
    assert!(text(&status).contains("REFUSED"), "{}", text(&status));
}

#[test]
fn an_empty_stored_token_is_refused_at_launch() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    host_credentials(&fx, 120);
    fx.write_token(Some("  \n"));

    let out = fx.launch(&[]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(fx.recorded("env.txt").is_empty(), "no host fallback");
}

#[test]
fn stored_token_wins_over_the_host_token() {
    // LNCH-05 adjacency + ordering (explicit): both exist, the stored one is
    // passed alone; ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN are never consulted.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    host_credentials(&fx, 120);

    let mut cmd = fx.launch_command(RUN_ID, &[]);
    cmd.env("ANTHROPIC_AUTH_TOKEN", "auth-token-from-the-parent-env");
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{}", text(&out));

    let passed = fx.recorded("token.txt");
    assert_eq!(sha(&passed), sha(TOKEN), "the stored token was passed");
    assert_ne!(sha(&passed), sha(HOST_ACCESS), "not the host token");

    let env = fx.recorded("env.txt");
    for absent in [
        "ANTHROPIC_API_KEY=",
        "ANTHROPIC_AUTH_TOKEN=",
        "apiKeyHelper",
    ] {
        assert!(!env.contains(absent), "{absent} reached the driver:\n{env}");
    }
    assert_eq!(
        env.lines()
            .filter(|l| l.starts_with("CLAUDE_CODE_OAUTH_TOKEN="))
            .count(),
        1,
        "exactly one token variable"
    );

    // The refresh token and the host access token appear nowhere the launcher wrote.
    let mut written = vec![env, fx.log(), fx.shim_log(), fx.run_file("state.json")];
    written.push(fs::read_to_string(fx.config_dir().join("settings.json")).unwrap_or_default());
    for body in written {
        for secret in [HOST_REFRESH, HOST_ACCESS, "auth-token-from-the-parent-env"] {
            assert!(!body.contains(secret), "{secret} leaked into:\n{body}");
        }
    }
}

#[test]
fn without_a_stored_token_the_host_access_token_is_passed_and_never_the_refresh_token() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    fx.write_token(None);
    host_credentials(&fx, 120);

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", text(&out));
    assert_eq!(sha(&fx.recorded("token.txt")), sha(HOST_ACCESS));
    for body in [fx.recorded("env.txt"), fx.log(), fx.shim_log()] {
        assert!(
            !body.contains(HOST_REFRESH),
            "refresh token leaked:\n{body}"
        );
    }
}
