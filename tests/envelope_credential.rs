// ============================================================================
// SAFE-05 end to end: the user's ambient git identity is UNREACHABLE, not
// merely unused — and pushing still works (D-16, D-17, D-18, D-32).
//
// **The verification has two halves and both are required.** Environment
// assertions alone would pass against an envelope that also broke pushing, and a
// push that succeeds proves nothing about what the child could reach. So this
// file asserts the constructed environment field by field AND performs a real
// `git push` that succeeds with `HOME` pointed at an empty directory.
//
// Every fixture here is offline and agent-free (D-35): a `file://` bare
// repository for the positive case, and — for the fail-closed case — a loopback
// listener that answers `401` and nothing else. No real remote host, no real
// credential, no network beyond `127.0.0.1`, no agent.
//
// `#![cfg(unix)]` for the same reason as `envelope_tracer.rs`: the generated
// stubs are `#!/bin/sh`. The policy they carry is portable and unit-tested in
// `src/envelope/`.
// ============================================================================
#![cfg(unix)]

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use gsd_meta_manager::envelope::{cred, ENVELOPE_ROOT_ENV};
use tempfile::TempDir;

mod common;
use common::{fixture, git, Fixture, BIN};

/// How long a fail-closed push may take before it counts as a hang.
///
/// The failure `GIT_TERMINAL_PROMPT=0` exists to prevent does not look like a
/// failure — it looks like a run that is still going, and the idle cap kills it
/// hours later. So "it terminated" is an assertion here, not an assumption.
const FAIL_CLOSED_BUDGET: Duration = Duration::from_secs(30);

/// Report a skipped fixture instead of skipping silently.
///
/// The fixtures here degrade to a no-op when a sandbox forbids `git init` or
/// binding a loopback port, which is the right behaviour — but a **silent**
/// no-op in a suite whose whole subject is "the credential is unreachable" is
/// the vacuous pass this phase exists to prevent. `cargo test -- --nocapture`
/// now says so out loud.
fn skip_unless<T>(value: Option<T>, name: &str) -> Option<T> {
    if value.is_none() {
        eprintln!(
            "SKIPPED {name}: this sandbox does not permit the fixture, so nothing \
             below was asserted"
        );
    }
    value
}

/// A `HOME` with nothing in it, which is what makes the push assertions mean
/// something: the developer's real `~/.gitconfig`, `~/.git-credentials` and
/// `~/.ssh/` are all out of the picture.
fn empty_home() -> TempDir {
    TempDir::new().expect("a temp directory for an empty HOME")
}

/// Build the driven child's environment for this fixture's alias and work tree.
fn envelope_env(fx: &Fixture) -> cred::EnvelopeEnv {
    cred::build_env_in(&fx.envelope_root, &fx.alias, &fx.work, Path::new(BIN))
        .expect("a plain alias builds an environment")
}

/// A `git` invocation carrying the envelope's environment (or deliberately not)
/// and an emptied `HOME`.
fn git_under(env: Option<&cred::EnvelopeEnv>, home: &Path, dir: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    cmd.env("HOME", home);
    // Inherited from the developer's shell, these would decide the answer before
    // the envelope got a say — in either direction.
    for stray in ["XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM"] {
        cmd.env_remove(stray);
    }
    match env {
        None => {
            // The control run. The injected-config family has to go too, or a
            // stale one would make the control agree with the envelope for a
            // reason that is not the envelope.
            cmd.env_remove("GIT_CONFIG_COUNT");
            cmd.env_remove("GIT_ASKPASS");
        }
        Some(env) => {
            for (key, value) in env.entries() {
                match value {
                    None => cmd.env_remove(key),
                    Some(value) => cmd.env(key, value),
                };
            }
        }
    }
    cmd.output().expect("git is runnable")
}

// ---------------------------------------------------------------------------
// Half one: the constructed environment, field by field
// ---------------------------------------------------------------------------

#[test]
fn the_agent_is_removed_and_every_config_scope_is_redirected_into_the_envelope() {
    let Some(fx) = skip_unless(fixture("env-shape"), "env-shape") else {
        return;
    };
    let env = envelope_env(&fx);
    let dir = fx.envelope_root.join(&fx.alias);

    for key in ["SSH_AUTH_SOCK", "SSH_AGENT_PID"] {
        assert!(
            env.is_removed(key),
            "{key} must carry the removal marker; an empty value is still a \
             variable an agent can notice and work around (D-16)"
        );
    }
    for key in ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM", "GH_CONFIG_DIR", "GIT_ASKPASS"] {
        let value = env
            .value(key)
            .unwrap_or_else(|| panic!("{key} must be set"));
        assert!(
            Path::new(value).starts_with(&dir),
            "{key} points at {value:?}, outside this alias's envelope {}",
            dir.display()
        );
    }
    assert_eq!(
        env.value("GIT_TERMINAL_PROMPT").map(|v| v.to_string_lossy().into_owned()),
        Some("0".to_string()),
        "a prompt into a detached run's null stdio is a hang, not a prompt (D-16)"
    );
    assert!(
        !fx.envelope_root.starts_with(&fx.work),
        "every artifact asserted above must live outside the driven repository, \
         or `git add -A` can sweep it into a commit (D-02)"
    );
}

#[test]
fn a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope() {
    let Some(fx) = skip_unless(fixture("no-helper"), "no-helper") else {
        return;
    };
    let home = empty_home();

    // A helper the "user" genuinely has. Without it the assertion below would
    // pass against an envelope that does nothing at all — an emptied HOME
    // resolves no helper either, so the control is what makes the refusal
    // load-bearing rather than vacuous.
    std::fs::write(
        home.path().join(".gitconfig"),
        "[credential]\n\thelper = store\n",
    )
    .expect("the fake home is writable");

    let control = git_under(None, home.path(), &fx.work, &["config", "--get", "credential.helper"]);
    if !control.status.success() {
        // git could not see the fake home at all (an unusual sandbox); assert
        // nothing rather than assert something meaningless.
        return;
    }
    assert!(
        String::from_utf8_lossy(&control.stdout).contains("store"),
        "the control must resolve the helper, or the refusal below proves nothing"
    );

    let env = envelope_env(&fx);
    let under = git_under(
        Some(&env),
        home.path(),
        &fx.work,
        &["config", "--get", "credential.helper"],
    );
    assert!(
        !under.status.success(),
        "the envelope still resolves a credential helper ({:?}); the user's \
         keychain, credential store and gh helper are all one helper key away (D-16)",
        String::from_utf8_lossy(&under.stdout)
    );
}

// ---------------------------------------------------------------------------
// Half two: a real push that still works
// ---------------------------------------------------------------------------

#[test]
fn a_push_inside_the_reserved_namespace_succeeds_with_home_emptied() {
    let Some(fx) = skip_unless(fixture("scoped-push"), "scoped-push") else {
        return;
    };
    let home = empty_home();
    let env = envelope_env(&fx);
    let target = fx.inside_ref_named("scoped");

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&fx.work)
        .args(["push", "origin", &format!("HEAD:{target}")]);
    cmd.env("HOME", home.path());
    cmd.env(ENVELOPE_ROOT_ENV, &fx.envelope_root);
    for (key, value) in env.entries() {
        match value {
            None => cmd.env_remove(key),
            Some(value) => cmd.env(key, value),
        };
    }
    let out = cmd.output().expect("git push is runnable");

    assert!(
        out.status.success(),
        "the envelope broke pushing outright. Environment assertions alone would \
         have passed against this, which is why D-32 pairs them with a real push; \
         stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let refs = Command::new("git")
        .args(["ls-remote", "--heads"])
        .arg(format!("file://{}", fx.bare.display()))
        .output()
        .expect("git ls-remote is runnable");
    assert!(
        String::from_utf8_lossy(&refs.stdout).contains(&target),
        "the push reported success but {target} is absent from `git ls-remote`"
    );
}

// ---------------------------------------------------------------------------
// The fail-closed case: no credential means no push, promptly and legibly
// ---------------------------------------------------------------------------

/// A loopback listener that answers every request with `401` and closes.
///
/// It stands in for "a remote that wants credentials" without being one: no
/// name resolution, no route off the machine, no repository behind it. Returns
/// `None` when the sandbox forbids binding, so the test skips rather than fails
/// for a reason that is not about the code.
fn spawn_challenging_listener() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(8) {
            let Ok(mut stream) = stream else { break };
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(
                b"HTTP/1.1 401 Unauthorized\r\n\
                  WWW-Authenticate: Basic realm=\"gsd-meta-manager test\"\r\n\
                  Content-Length: 0\r\n\
                  Connection: close\r\n\r\n",
            );
            let _ = stream.flush();
        }
    });
    Some(port)
}

#[test]
fn an_unconfigured_credential_fails_closed_promptly_instead_of_hanging() {
    // A deliberately unrepeatable alias: the responder resolves its credential
    // from the registry at the default path, and this test's whole claim is that
    // it finds nothing there.
    let Some(fx) = skip_unless(fixture("unconfigured-19-04"), "unconfigured-19-04") else {
        return;
    };
    let Some(port) = spawn_challenging_listener() else {
        return;
    };
    let home = empty_home();

    // Point the remote at the challenging listener BEFORE building the
    // environment, so the responder's baked host is this one.
    assert!(
        git(
            &fx.work,
            &["remote", "set-url", "origin", &format!("http://127.0.0.1:{port}/repo.git")]
        ),
        "the fixture's remote is re-pointable"
    );
    let env = envelope_env(&fx);

    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(&fx.work).args([
        "push",
        "origin",
        &format!("HEAD:{}", fx.inside_ref_named("fail-closed")),
    ]);
    cmd.env("HOME", home.path());
    cmd.env(ENVELOPE_ROOT_ENV, &fx.envelope_root);
    for (key, value) in env.entries() {
        match value {
            None => cmd.env_remove(key),
            Some(value) => cmd.env(key, value),
        };
    }
    // Null stdin is the driven run's real stdio (Phase 17 D-01), and it is what
    // makes "a prompt is a hang" true rather than theoretical.
    cmd.stdin(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::piped());

    let started = Instant::now();
    let mut child = cmd.spawn().expect("git push is spawnable");
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child is waitable") {
            break status;
        }
        if started.elapsed() > FAIL_CLOSED_BUDGET {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "the push was still running after {FAIL_CLOSED_BUDGET:?}. An \
                 authentication prompt into a detached run's null stdio is not a \
                 prompt — it is the hang GIT_TERMINAL_PROMPT=0 exists to prevent, \
                 and it surfaces hours later as an idle-cap kill (D-16)"
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_string(&mut stderr);
    }

    assert!(
        !status.success(),
        "a push with no configured credential succeeded, which means something \
         supplied one — and the only credentials on this machine are the user's \
         (D-18); stderr was:\n{stderr}"
    );
    assert!(
        started.elapsed() < FAIL_CLOSED_BUDGET,
        "the failure has to be prompt, not eventual"
    );
    // Both halves of the fail-closed chain, asserted rather than assumed. The
    // first proves the ENVELOPE refused — not merely that the push happened to
    // fail — and the second proves what caught the fall: git dropped to a
    // terminal prompt and `GIT_TERMINAL_PROMPT=0` turned that into an immediate
    // error instead of a block on a null stdio.
    assert!(
        stderr.contains("credential_unavailable"),
        "the refusal must name its D-24 reason, or a reader cannot tell an \
         unconfigured run from a broken one; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("terminal prompts disabled"),
        "git fell back to a prompt and something other than GIT_TERMINAL_PROMPT \
         ended the run; stderr was:\n{stderr}"
    );
}

#[test]
fn the_responder_answers_the_configured_host_and_refuses_every_other_one() {
    // The whole D-17 contract, through the REAL binary rather than through the
    // library: the token reaches stdout for the configured host, and an agent
    // that adds a second remote gets an authentication failure instead.
    let Some(fx) = skip_unless(fixture("askpass-e2e"), "askpass-e2e") else {
        return;
    };
    const HOST: &str = "git.example.com";
    const SECRET: &str = "tok_A1b2C3d4E5f6";

    let config_path = fx.tmp().join("registry.json");
    std::fs::write(
        &config_path,
        format!(
            r#"{{
  "version": 2,
  "projects": {{
    "{alias}": {{
      "path": "{path}",
      "added": "2026-08-18T00:00:00Z",
      "driver_opt_in": {{
        "opted_in_at": "2026-08-18T00:00:00Z",
        "claude_md_digest": "fnv1a:0123456789abcdef",
        "credential": {{ "source": "command", "argv": ["printf", "{secret}"] }}
      }}
    }}
  }},
  "preferences": {{}}
}}"#,
            alias = fx.alias,
            path = fx.work.display(),
            secret = SECRET,
        ),
    )
    .expect("the registry is writable");

    let responder = |host: &str, prompt: &str| -> Output {
        Command::new(BIN)
            .args(["envelope", "askpass", &fx.alias, "--host", host])
            .arg("--config")
            .arg(&config_path)
            .args(["--", prompt])
            .env(ENVELOPE_ROOT_ENV, &fx.envelope_root)
            .output()
            .expect("the envelope binary is runnable")
    };

    let answered = responder(HOST, &format!("Password for 'https://{HOST}': "));
    assert!(
        answered.status.success(),
        "the configured host must be answered; stderr was:\n{}",
        String::from_utf8_lossy(&answered.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&answered.stdout).trim(),
        SECRET,
        "the credential must reach git through stdout, and reach it intact"
    );
    assert!(
        !String::from_utf8_lossy(&answered.stderr).contains(SECRET),
        "the credential appeared on stderr, which is a log destination"
    );

    let refused = responder(HOST, "Password for 'https://evil.example.net/repo.git': ");
    assert!(
        !refused.status.success(),
        "a remote the run was not configured against must get an authentication \
         failure, never the token (D-17)"
    );
    assert!(
        refused.stdout.is_empty(),
        "a second remote received {:?} on stdout",
        String::from_utf8_lossy(&refused.stdout)
    );
    assert!(
        !String::from_utf8_lossy(&refused.stderr).contains(SECRET),
        "the refusal leaked the credential it was refusing to emit"
    );
}

#[test]
fn nothing_the_envelope_wrote_holds_the_credential() {
    // A behavior assertion over the directory contents rather than a source
    // grep: the claim is that no file this run created is a secret at rest, and
    // only reading what was written can settle it (D-17).
    let Some(fx) = skip_unless(fixture("at-rest"), "at-rest") else {
        return;
    };
    const SECRET: &str = "tok_Z9y8X7w6V5u4";

    let _ = envelope_env(&fx);
    let mut out = Vec::new();
    cred::askpass_into(
        &mut out,
        "Password for 'https://git.example.com': ",
        "git.example.com",
        Some(SECRET),
    )
    .expect("the responder writes");
    assert!(
        String::from_utf8_lossy(&out).contains(SECRET),
        "the fixture must have emitted the secret, or the walk below proves nothing"
    );

    let mut seen = 0usize;
    let mut stack: Vec<PathBuf> = vec![fx.envelope_root.clone()];
    while let Some(next) = stack.pop() {
        for entry in std::fs::read_dir(&next).expect("the envelope root is readable") {
            let path = entry.expect("a readable entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            seen += 1;
            let bytes = std::fs::read(&path).expect("a readable file");
            assert!(
                !String::from_utf8_lossy(&bytes).contains(SECRET),
                "{} holds the credential; a secret at rest under the envelope has \
                 no protection posture at all (D-17)",
                path.display()
            );
        }
    }
    assert!(seen > 0, "the walk saw no files, so it asserted nothing");
}
