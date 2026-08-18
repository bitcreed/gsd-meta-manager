// ============================================================================
// SAFE-03 and the `git add -A` worktree sweep, end to end (D-13, D-22, D-33).
//
// Every assertion is made against a REAL git repository and a REAL `file://`
// bare remote in a temp directory. No network, no credential, no agent: the
// tests invoke git directly, so a refusal cannot be coming from a model
// choosing to cooperate.
//
// The first test is the one that matters most, because it is the one the
// obvious implementation fails. A backstop that asks git which files to look
// at honours the repository's ignore rules, and therefore never sees a
// credential written to `secrets/` — which is precisely where a credential
// tends to be written. The fixture plants one there and requires the push to
// be refused.
//
// Every refusal here is paired with an allow. An envelope that blocks
// everything is not a boundary; it is a wall, and a wall passes every refusal
// assertion ever written.
//
// `#![cfg(unix)]` because the generated hook stubs are `#!/bin/sh` with mode
// 0755. The policy they enforce is portable and is unit-tested in
// `src/envelope/`; only the delivery mechanism proved here is Unix-shaped.
// ============================================================================
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use gsd_meta_manager::envelope::{cred, hooks, ENVELOPE_ROOT_ENV};
use tempfile::TempDir;

/// The binary under test, resolved by cargo for this integration target.
const BIN: &str = env!("CARGO_BIN_EXE_gsd-meta-manager");

/// A credential shape planted by these fixtures.
///
/// A PEM block rather than a one-line token, because a PEM block is the case a
/// line-by-line scanner silently misses: `BEGIN` and `END` are never on the
/// same line, so a per-line pass reports a checked-in private key as clean.
const PLANTED_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
                           MIIBOgIBAAJBAKj34GkxFhD9abcdefgh\n\
                           ijklmnopqrstuvwxyz0123456789ABCD\n\
                           -----END RSA PRIVATE KEY-----\n";

/// A path a driven `git add -A` sweeps up, and which must never be committed.
const SWEPT_PATH: &str = ".claude/worktrees/agent-abc/notes.txt";

struct Fixture {
    /// Held for its Drop; every path below lives inside it.
    _tmp: TempDir,
    work: PathBuf,
    bare: PathBuf,
    envelope_root: PathBuf,
    hooks_dir: PathBuf,
    alias: String,
}

impl Fixture {
    fn inside_ref(&self) -> String {
        format!("refs/heads/gsd-auto/{}/sweep", self.alias)
    }
}

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent directory");
    }
    std::fs::write(path, contents).expect("the planted file");
}

/// A work repository with one commit, a `file://` bare remote, and an installed
/// envelope — or `None` when the sandbox forbids `git init`, so the tests skip
/// gracefully rather than failing for a reason that is not about the code.
fn fixture(alias: &str) -> Option<Fixture> {
    let tmp = TempDir::new().ok()?;
    let bare = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    let envelope_root = tmp.path().join("envelope");
    std::fs::create_dir_all(&bare).ok()?;
    std::fs::create_dir_all(&work).ok()?;

    if !git(&bare, &["init", "--bare", "--quiet"]) {
        return None;
    }
    if !git(&work, &["init", "--quiet"]) {
        return None;
    }
    git(&work, &["config", "user.email", "test@example.com"]);
    git(&work, &["config", "user.name", "Test User"]);
    git(&work, &["config", "commit.gpgsign", "false"]);

    // The repository's own ignore rules. `secrets/` is the blind spot D-33
    // names: a backstop that consults these never reads what is under it.
    write(&work, ".gitignore", "secrets/\n*.local\n.env\n");
    write(&work, "tracked.txt", "one\n");
    if !git(&work, &["add", ".gitignore", "tracked.txt"]) {
        return None;
    }
    if !git(&work, &["commit", "-m", "initial commit", "--quiet"]) {
        return None;
    }
    let url = format!("file://{}", bare.display());
    if !git(&work, &["remote", "add", "origin", &url]) {
        return None;
    }

    let hooks_dir = hooks::install_in(&envelope_root, alias, Path::new(BIN))
        .expect("installing hook stubs for a plain alias succeeds");

    Some(Fixture {
        _tmp: tmp,
        work,
        bare,
        envelope_root,
        hooks_dir,
        alias: alias.to_string(),
    })
}

/// `git push` under the envelope's environment, and nothing else changed.
fn push_under_envelope(fx: &Fixture, refspec: &str) -> Output {
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&fx.work)
        .args(["push", "origin", refspec]);
    for (key, value) in cred::hooks_path_env(&fx.hooks_dir) {
        cmd.env(key, value);
    }
    cmd.env(ENVELOPE_ROOT_ENV, &fx.envelope_root);
    cmd.output().expect("git push is runnable")
}

/// The `pre-commit` stub, run the way git runs it: from the top of the
/// worktree, with no arguments and no stdin.
fn run_pre_commit(fx: &Fixture) -> Output {
    Command::new(fx.hooks_dir.join(hooks::PRE_COMMIT_HOOK))
        .current_dir(&fx.work)
        .env(ENVELOPE_ROOT_ENV, &fx.envelope_root)
        .output()
        .expect("the generated stub is executable")
}

/// The remote's refs, read from the bare repository itself.
fn remote_refs(fx: &Fixture) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(&fx.bare)
        .args(["for-each-ref", "--format=%(refname)"])
        .output()
        .expect("git for-each-ref is runnable");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---------------------------------------------------------------------------
// SAFE-03
// ---------------------------------------------------------------------------

#[test]
fn a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push() {
    let Some(fx) = fixture("gitignored") else {
        return;
    };

    // Ignored by `.gitignore`, so git will never report it as a changed or
    // untracked file — and it is exactly the kind of path a secret gets
    // written to. This is D-33's named blind spot, planted deliberately.
    write(&fx.work, "secrets/prod.pem", PLANTED_PEM);
    assert!(
        git(&fx.work, &["check-ignore", "-q", "secrets/prod.pem"]),
        "the fixture must plant the secret on a path the repository's ignore \
         rules actually cover, or this test proves nothing"
    );

    let inside = fx.inside_ref();
    let out = push_under_envelope(&fx, &format!("HEAD:{inside}"));
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a push carrying a credential on an ignored path left the machine; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("secrets/prod.pem"),
        "the refusal must name the file, or a human cannot act on it; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("rule=pem"),
        "the refusal must name the rule that fired; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("secret_detected"),
        "the refusal must carry its D-24 park reason; stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("MIIBOgIBAAJBAKj34GkxFhD9abcdefgh"),
        "the report reproduced the secret it blocked, which is the \
         redact-at-capture bug committed by the code meant to prevent it; \
         stderr was:\n{stderr}"
    );
    assert!(
        !remote_refs(&fx).contains(&inside),
        "the remote grew {inside} — the exit code refused but the write happened"
    );
}

#[test]
fn a_clean_worktree_pushes_inside_the_namespace_and_succeeds() {
    let Some(fx) = fixture("cleanpush") else {
        return;
    };

    // The paired allow. Without it the scan could refuse every push on earth
    // and still satisfy every refusal assertion above.
    let inside = fx.inside_ref();
    let out = push_under_envelope(&fx, &format!("HEAD:{inside}"));

    assert!(
        out.status.success(),
        "the envelope refused a clean push inside its own namespace, which \
         makes it a wall rather than a boundary; stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        remote_refs(&fx).contains(&inside),
        "the push reported success but {inside} is absent from the remote"
    );
}

#[test]
fn the_skip_list_reaches_a_human_on_the_allowing_path_too() {
    let Some(fx) = fixture("skiplist") else {
        return;
    };

    // A binary file the scan will decline to read. The push still succeeds —
    // one unreadable file does not block — but the report must SAY so, because
    // a scanner that reports clean while declining to read is a scanner that
    // lies (D-14).
    std::fs::write(fx.work.join("blob.bin"), [b'x', 0, b'y']).expect("the binary file");

    let out = push_under_envelope(&fx, &format!("HEAD:{}", fx.inside_ref()));
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(out.status.success(), "stderr was:\n{stderr}");
    assert!(
        stderr.contains("declined to read") && stderr.contains("blob.bin"),
        "an allowed push must still disclose what the scan did not read; \
         stderr was:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// The `git add -A` sweep (D-22)
// ---------------------------------------------------------------------------

#[test]
fn staging_a_worktree_path_is_refused_by_the_commit_hook() {
    let Some(fx) = fixture("precommit") else {
        return;
    };

    // The paired allow FIRST, so a refusal below cannot be the hook simply
    // refusing everything.
    write(&fx.work, "src/main.rs", "fn main() {}\n");
    assert!(git(&fx.work, &["add", "src/main.rs"]));
    let allowed = run_pre_commit(&fx);
    assert!(
        allowed.status.success(),
        "an ordinary staged file must commit; stderr was:\n{}",
        String::from_utf8_lossy(&allowed.stderr)
    );

    write(&fx.work, SWEPT_PATH, "swept by git add -A\n");
    assert!(git(&fx.work, &["add", SWEPT_PATH]));
    let refused = run_pre_commit(&fx);
    let stderr = String::from_utf8_lossy(&refused.stderr);

    assert!(
        !refused.status.success(),
        "a staged {SWEPT_PATH} reached a commit; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(SWEPT_PATH),
        "the refusal must name the offending path; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("force_push_blocked"),
        "the refusal must carry a D-24 park reason; stderr was:\n{stderr}"
    );
}

#[test]
fn a_commit_made_with_verification_suppressed_is_refused_by_the_push_hook() {
    let Some(fx) = fixture("nobypass") else {
        return;
    };

    // The backstop's whole reason for existing: this commit is made with the
    // verification step suppressed, so the commit hook by construction never
    // saw it. The push hook re-checks the paths the introduced commits touch.
    write(&fx.work, SWEPT_PATH, "swept by git add -A\n");
    assert!(git(&fx.work, &["add", SWEPT_PATH]));
    assert!(git(
        &fx.work,
        &["commit", "--no-verify", "-m", "sweep", "--quiet"]
    ));

    let inside = fx.inside_ref();
    let out = push_under_envelope(&fx, &format!("HEAD:{inside}"));
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a commit that skipped the commit hook reached the remote; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(SWEPT_PATH),
        "the backstop must name the path it refused; stderr was:\n{stderr}"
    );
    assert!(
        !remote_refs(&fx).contains(&inside),
        "the remote grew {inside} — the exit code refused but the write happened"
    );
}

#[test]
fn the_exclude_block_is_idempotent_and_survives_content_already_present() {
    let Some(fx) = fixture("exclude") else {
        return;
    };

    let exclude = fx.work.join(".git").join("info").join("exclude");
    std::fs::create_dir_all(exclude.parent().unwrap()).expect("the info directory");
    std::fs::write(&exclude, "# the user's own rules\nlocal-scratch/\n")
        .expect("pre-existing content");

    hooks::write_exclude_block(&fx.work).expect("the first write");
    let once = std::fs::read(&exclude).expect("readable");
    hooks::write_exclude_block(&fx.work).expect("the second write");
    let twice = std::fs::read(&exclude).expect("readable");

    assert_eq!(
        once, twice,
        "the block is appended rather than rewritten in place, so every run \
         grows the file (D-23)"
    );

    let text = String::from_utf8(twice).expect("utf-8");
    assert!(
        text.contains("local-scratch/"),
        "unrelated content already in the file was destroyed:\n{text}"
    );
    assert_eq!(
        text.matches(hooks::EXCLUDE_BLOCK_START).count(),
        1,
        "exactly one block, no matter how many runs:\n{text}"
    );
    assert_eq!(text.matches(hooks::EXCLUDE_BLOCK_END).count(), 1, "{text}");
    assert!(text.contains(".claude/worktrees/"), "{text}");

    // And the rule is live: git itself now ignores the swept directory.
    write(&fx.work, SWEPT_PATH, "swept\n");
    assert!(
        git(&fx.work, &["check-ignore", "-q", SWEPT_PATH]),
        "the block was written but git does not honour it, which makes it \
         decoration rather than prevention"
    );
}

#[test]
fn both_hooks_are_generated_and_neither_carries_policy_logic() {
    let Some(fx) = fixture("stubs") else {
        return;
    };

    for hook in [hooks::PRE_PUSH_HOOK, hooks::PRE_COMMIT_HOOK] {
        let body = std::fs::read_to_string(fx.hooks_dir.join(hook))
            .unwrap_or_else(|err| panic!("the {hook} stub must exist: {err}"));
        assert!(
            body.lines().count() <= 5,
            "the {hook} stub grew past five lines, which is where policy starts \
             living in shell instead of in Rust (D-04):\n{body}"
        );
        for token in body.split_whitespace() {
            assert!(
                !matches!(token, "if" | "case" | "esac" | "fi" | "test" | "[" | "eval"),
                "the {hook} stub contains the shell construct {token:?}, so it \
                 carries policy an agent can rewrite invisibly (D-04):\n{body}"
            );
        }
        assert!(body.contains(BIN), "the {hook} stub must exec the binary:\n{body}");
    }
}
