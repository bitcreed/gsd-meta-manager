// ============================================================================
// SAFE-01 end to end, with the model removed from the equation entirely (D-31).
//
// Every assertion below is made against a REAL `git push` to a REAL remote — a
// `file://` bare repository in a temp directory. No network, no credential, no
// agent, and no prose: the test invokes git directly, so the refusal cannot be
// coming from a model choosing to cooperate. That is the whole point of the
// fixture, and it is what makes ROADMAP criterion 1 provable rather than
// asserted.
//
// `#![cfg(unix)]` because the generated hook stub is `#!/bin/sh` with mode 0755.
// The policy it enforces is portable and is unit-tested in `src/envelope/`; only
// the delivery mechanism proved here is Unix-shaped.
//
// ----------------------------------------------------------------------------
// ETXTBSY: why `run_stub` waits, and what would have been the wrong fix.
//
// `a_relocated_copy_of_the_stub_refuses_instead_of_acting` flaked intermittently
// under contention with `Text file busy` at the `spawn()` inside `run_stub`. The
// obvious suspect is the `std::fs::copy` immediately before the second exec —
// write a file, exec it, lose the race. **That suspect is exonerated by
// measurement, not by argument.** `strace -f -e trace=execve` caught the failing
// syscall, and every time it was the SANCTIONED stub, in the envelope's own hooks
// directory. The relocated copy's path never appeared in a failing `execve` at
// all. What flaked is the CONTROL leg, and the copy is a bystander.
//
// The writer is not ours and is not at this call site. `src/envelope/hooks.rs`'s
// `write_stub` builds the stub through a `NamedTempFile` — write, chmod, persist —
// and while that descriptor is open, any `fork` anywhere in the process gives the
// child a copy of it. This binary runs six `#[test]` fns on six libtest threads
// that spawn subprocesses constantly, so a SIBLING thread's forked child routinely
// inherits it. `O_CLOEXEC` closes it at that child's own `execve`, but not one
// instruction before — and in that window the child is a writer.
//
// ETXTBSY is a per-INODE condition (`i_writecount`), which is what rules out the
// tidy-looking fix. Closing or syncing the `File` before the rename in `write_stub`
// would change nothing: `rename(2)` does not change the inode, and the inode was
// already exposed to a `fork` during the pre-rename write and chmod. The hazard is
// never "the final path had a writer"; it is "this inode had a writer while a
// sibling thread forked". By exec time the offending descriptor lives in a
// short-lived child of another thread and clears on its own in microseconds, so
// the only correct handle on it at the exec site is to wait it out.
//
// The wait is therefore ETXTBSY-only and bounded, and **expiry FAILS rather than
// skips**. This test exists to prove that a relocated stub refuses instead of
// acting; an exec that never happened proves nothing about refusal, so a busy
// inode may never stand in for the refusal being asserted.
//
// The measurements behind all of the above — the contended baseline, the strace
// captures, and the two other eliminations that were considered and rejected —
// are recorded in
// `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md`
// under `# QUICK 260917-lkg`, and are deliberately not duplicated here.
// ============================================================================
#![cfg(unix)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::envelope::{cred, envelope_dir, envelope_dir_in, hooks, ENVELOPE_ROOT_ENV};
use tempfile::TempDir;

// The `file://` bare-remote harness lives in `tests/common/` since plan 19-04,
// so this file and `tests/envelope_credential.rs` drive the same fixture rather
// than two copies of it.
mod common;
use common::{fixture, remote_refs, Fixture, BIN};

/// A ref inside the reserved namespace, and one squarely outside it.
const OUTSIDE_REF: &str = "refs/heads/main";

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

/// `.git/config` bytes plus the `.git/hooks` listing, digest and all.
///
/// The technique `tests/driver_dry_run.rs` established: prove the
/// zero-repository-mutation claim mechanically instead of asserting it. A digest
/// as well as a length, because the most likely unwanted write leaves a file
/// exactly the same size.
fn repo_hook_surface(work: &Path) -> (Vec<u8>, Vec<(String, u64, u64)>) {
    let config = std::fs::read(work.join(".git").join("config")).unwrap_or_default();
    let hooks = work.join(".git").join("hooks");
    let mut listing = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&hooks) {
        for entry in entries.flatten() {
            let bytes = std::fs::read(entry.path()).unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            listing.push((
                entry.file_name().to_string_lossy().into_owned(),
                bytes.len() as u64,
                hasher.finish(),
            ));
        }
    }
    listing.sort();
    (config, listing)
}

// ---------------------------------------------------------------------------
// The proofs
// ---------------------------------------------------------------------------

#[test]
fn a_driven_push_to_main_is_refused_and_the_remote_ref_never_appears() {
    let Some(fx) = fixture("refusal") else {
        return;
    };

    let out = push_under_envelope(&fx, &format!("HEAD:{OUTSIDE_REF}"));
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a push to {OUTSIDE_REF} left the envelope and reached the remote; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(OUTSIDE_REF),
        "the refusal must name the ref it refused, or a human cannot act on it; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("push_outside_namespace"),
        "the refusal must carry its D-24 reason; stderr was:\n{stderr}"
    );
    assert!(
        !remote_refs(&fx).contains(OUTSIDE_REF),
        "the remote grew {OUTSIDE_REF} — the exit code refused but the write happened"
    );
}

#[test]
fn a_driven_push_inside_the_reserved_namespace_reaches_the_remote() {
    let Some(fx) = fixture("allow") else {
        return;
    };

    let inside = fx.inside_ref();
    let out = push_under_envelope(&fx, &format!("HEAD:{inside}"));

    assert!(
        out.status.success(),
        "the envelope refused a push INSIDE its own namespace, which makes it a \
         wall rather than a boundary; stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        remote_refs(&fx).contains(&inside),
        "the push reported success but {inside} is absent from the remote"
    );
}

#[test]
fn neither_push_writes_into_the_driven_repository_config_or_hooks() {
    let Some(fx) = fixture("untouched") else {
        return;
    };

    let (config_before, hooks_before) = repo_hook_surface(&fx.work);
    // A fingerprint that saw nothing would compare equal to itself.
    assert!(
        !config_before.is_empty(),
        "the fingerprint must actually see this repository's .git/config"
    );

    let refused = push_under_envelope(&fx, &format!("HEAD:{OUTSIDE_REF}"));
    assert!(!refused.status.success());
    let allowed = push_under_envelope(&fx, &format!("HEAD:{}", fx.inside_ref()));
    assert!(allowed.status.success());

    let (config_after, hooks_after) = repo_hook_surface(&fx.work);

    assert_eq!(
        config_before, config_after,
        "the envelope rewrote .git/config — core.hooksPath must reach git only \
         through GIT_CONFIG_COUNT/KEY_n/VALUE_n (D-09)"
    );
    assert_eq!(
        hooks_before, hooks_after,
        "the envelope installed into .git/hooks/, which would police the user's \
         own pushes in this repository (D-09)"
    );
    assert!(
        fx.hooks_dir.join("pre-push").is_file(),
        "the hook the pushes ran must live in the envelope directory, outside \
         the repository (D-02)"
    );
    assert!(
        !fx.hooks_dir.starts_with(&fx.work),
        "an envelope artifact inside the driven repository can be swept into a \
         commit by `git add -A` (D-02)"
    );
}

/// Run a hook stub directly, feeding it one ref line, and report its output.
///
/// `current_dir` is the work repository because that is where git runs a hook
/// from — the top of the worktree. Since Phase 19-03 the `pre-push` body scans
/// that directory for credential shapes, so a stub run from the *test process's*
/// cwd would be scanning this project's own checkout: slow, and green or red for
/// reasons that have nothing to do with the fixture.
///
/// The spawn is wrapped in a bounded, **ETXTBSY-only** retry (quick 260917-lkg).
/// See the ETXTBSY section of this file's header for the mechanism. Three things
/// carry it and none of them may be relaxed:
///
/// * only `ErrorKind::ExecutableFileBusy` retries — every other spawn error fails
///   on the spot under the original `the generated stub is executable` wording, so
///   a genuinely non-executable stub (`PermissionDenied` for a bad mode, `ENOEXEC`
///   for a bad shebang) is still a loud, immediate, correctly-attributed failure
///   and the loop cannot become a swallow-all;
/// * expiry PANICS rather than returning a synthesised `Output`, because every
///   caller is about to assert on what the stub did and an exec that never
///   happened proves nothing about it;
/// * the 25ms sleep is INSIDE the poll loop, which is this project's house idiom
///   (`tests/driver_reattach.rs`'s `*_within` helpers) — it returns the moment the
///   inode frees and fails loudly when it never does, unlike a fixed
///   pre-assertion `sleep`, which those headers forbid.
fn run_stub(fx: &Fixture, stub: &Path, ref_line: &str) -> Output {
    // Built once and bound mutably: `Command::spawn` takes `&mut self`, so one
    // builder can be spawned repeatedly with no rebuild and no closure.
    let mut cmd = Command::new(stub);
    cmd.current_dir(&fx.work)
        .env(ENVELOPE_ROOT_ENV, &fx.envelope_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // `Duration::from_secs(30)` inline at the single site, matching
    // `tests/driver_reattach.rs`'s call-site convention rather than adding a
    // named constant for one use. Generous on purpose: the observed pin clears
    // in microseconds, so the limit only ever costs wall time on a real failure.
    let limit = Duration::from_secs(30);
    let deadline = Instant::now() + limit;
    let mut attempts: u32 = 0;

    let mut child = loop {
        attempts += 1;
        match cmd.spawn() {
            Ok(child) => break child,
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                if Instant::now() >= deadline {
                    panic!(
                        "ETXTBSY never cleared for {} after {attempts} spawn attempts over \
                         {limit:?}: the exec NEVER HAPPENED, so nothing whatever was proven \
                         about whether this stub refuses or acts. This is a hard failure, not \
                         a skip, and `Text file busy` is never the refusal this test asserts. \
                         Last error: {e:?}",
                        stub.display()
                    );
                }
                // Emitted so the retry branch is COUNTABLE rather than merely
                // absent: "the flake did not recur" and "the branch fired N
                // times and every run still passed" are different claims.
                eprintln!(
                    "ETXTBSY on {} (attempt {attempts}); the inode is pinned by a writer \
                     inherited into a sibling test thread's forked child — retrying",
                    stub.display()
                );
                std::thread::sleep(Duration::from_millis(25));
            }
            // Verbatim the pre-retry message, so a stub that is genuinely not
            // executable still fails for its own reason and is not masked.
            Err(e) => panic!("the generated stub is executable: {e:?}"),
        }
    };

    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(ref_line.as_bytes())
        .expect("the stub reads its stdin");
    child.wait_with_output().expect("the stub terminates")
}

/// The hostile corpus, reused rather than reinvented.
///
/// The first eleven are exactly the shapes
/// `src/journal/mod.rs::only_a_single_plain_component_is_accepted_as_a_run_id`
/// drives, because D-03 promoted **one** predicate to cover run ids and aliases
/// and a second corpus would be a second place for the two to drift apart. The
/// remainder are the alias-shaped forms an absolute path and a bare `..` cover
/// once the value is a registry key a user types rather than a generated id.
const HOSTILE_ALIASES: &[&str] = &[
    "",
    ".",
    "..",
    "../escaped",
    "../../../../escaped",
    "a/b",
    "/etc/passwd",
    "/",
    "./escaped",
    "escaped/",
    "a/../b",
    "..//..",
    "/absolute/alias",
    "sub/dir/alias",
];

#[test]
fn a_hostile_alias_is_refused_before_any_path_is_joined() {
    let tmp = TempDir::new().expect("a temp directory");
    let root = tmp.path().join("envelope");
    std::fs::create_dir_all(&root).expect("the envelope root");
    let outside = tmp.path().join("outside-marker");

    for hostile in HOSTILE_ALIASES {
        assert!(
            envelope_dir_in(&root, hostile).is_none(),
            "{hostile:?} must not be joined into an envelope path"
        );
        assert!(
            envelope_dir(hostile).is_none(),
            "{hostile:?} must yield no envelope directory at all"
        );

        let refused = hooks::install_in(&root, hostile, Path::new(BIN));
        assert!(
            refused.is_err(),
            "{hostile:?} must not reach the point where a hook is installed"
        );
    }

    assert_eq!(
        std::fs::read_dir(&root)
            .expect("the envelope root is readable")
            .count(),
        0,
        "a refused alias created something inside the envelope root"
    );
    assert!(
        !outside.exists(),
        "a refused alias escaped the envelope root entirely"
    );
    // The temp root holds exactly what this test put there and nothing a
    // traversal walked back into.
    let entries: Vec<String> = std::fs::read_dir(tmp.path())
        .expect("the temp root is readable")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries,
        vec!["envelope".to_string()],
        "a refused alias created a sibling of the envelope root: {entries:?}"
    );
}

#[test]
fn a_relocated_copy_of_the_stub_refuses_instead_of_acting() {
    let Some(fx) = fixture("provenance") else {
        return;
    };

    // A ref INSIDE the namespace, so a refusal below cannot be the policy
    // talking. Whatever refuses the copy has to be the provenance check.
    let allowed_line = format!(
        "refs/heads/x 1111111111111111111111111111111111111111 {} 2222222222222222222222222222222222222222\n",
        fx.inside_ref()
    );

    let sanctioned = fx.hooks_dir.join("pre-push");
    let control = run_stub(&fx, &sanctioned, &allowed_line);
    assert!(
        control.status.success(),
        "the sanctioned hook must ALLOW this ref, or the relocation assertion \
         below proves nothing; stderr was:\n{}",
        String::from_utf8_lossy(&control.stderr)
    );

    let elsewhere = fx.envelope_root.parent().unwrap().join("relocated-hooks");
    std::fs::create_dir_all(&elsewhere).expect("a directory outside the envelope");
    let copy = elsewhere.join("pre-push");
    std::fs::copy(&sanctioned, &copy).expect("the stub is copyable, which is the hazard");

    let out = run_stub(&fx, &copy, &allowed_line);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a copy of the hook relocated to {} became the sanctioned one (D-10); \
         stderr was:\n{stderr}",
        elsewhere.display()
    );
    assert!(
        stderr.contains("provenance"),
        "the refusal must name the provenance failure, so a human does not read \
         it as a policy verdict; stderr was:\n{stderr}"
    );
}

#[test]
fn the_generated_stub_carries_no_policy_logic() {
    let Some(fx) = fixture("stub") else {
        return;
    };

    let body = std::fs::read_to_string(fx.hooks_dir.join("pre-push"))
        .expect("the installed stub is readable");

    assert!(
        body.lines().count() <= 5,
        "the stub grew past five lines, which is where policy starts living in \
         shell instead of in Rust (D-04):\n{body}"
    );
    // Token-level, not `contains`: a substring check would fire on any path that
    // happens to spell one of these words, and pass or fail for the wrong reason.
    for token in body.split_whitespace() {
        assert!(
            !matches!(token, "if" | "case" | "esac" | "fi" | "test" | "[" | "eval"),
            "the stub contains the shell construct {token:?}, so it carries \
             policy an agent can rewrite invisibly (D-04):\n{body}"
        );
    }
    assert!(
        body.contains(BIN),
        "the stub must exec the absolute binary path captured at generation \
         time, never a name resolved from PATH:\n{body}"
    );
}
