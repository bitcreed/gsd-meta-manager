// ============================================================================
// T-19-60 — the `PreToolUse` guard decides what a command is by looking at
// `words[0]`, so anything that changes which token is the *effective* program
// without appearing first walks straight through.
//
// **This file was written BEFORE the fix, and that ordering is the point.**
// Every refusal row below is one of the lines `19-SECURITY.md` reproduced
// against the built binary, with the exit code it was *measured* to produce:
//
// ```
// exit=2  git push --force origin main                      <- denied, correct
// exit=0  env git push --force origin main                  <- PERMITTED
// exit=0  GIT_CONFIG_COUNT=0 git push --force origin main   <- PERMITTED
// exit=0  timeout 60 git push --force origin main           <- PERMITTED
// exit=0  command git push --force origin main              <- PERMITTED
// exit=0  gh pr create --title x                            <- permitted, ledger line written
// exit=0  env gh pr create --title x                        <- PERMITTED, NO ledger line
// ```
//
// A corpus written *after* a fix cannot distinguish a control that works from a
// control that was written to agree with what the code already did — the exact
// failure mode 19-08's `T-19-51` caught on itself. So these rows are committed
// red, with their verbatim failure output recorded, before a single production
// line moves.
//
// **Two rows exist so this file cannot pass vacuously.**
//
// 1. The *anti-vacuity control*: the unwrapped `git push --force origin main`
//    refuses TODAY. Without it, the whole file could be red because the harness
//    cannot observe a denial at all rather than because the bypass is real.
// 2. The *paired allow corpus* (D-32): `ls -la`, `echo hi`, `env git status`,
//    `timeout 5 ls`, `gh pr list` and an in-namespace push are permitted and
//    write nothing. The guard sees EVERY Bash tool call, so a fix that made an
//    unrecognised leading token deny would deny `ls` and `cargo test` and make a
//    driven run unusable. These rows fail if the fix becomes "deny everything",
//    which is what stops a green run of the refusal rows from meaning nothing.
//
// Each refusal row asserts the exit code AND the specific D-24 reason, so a row
// cannot pass by being refused for an *unrelated* cause — an unrecoverable
// split, say, which would refuse the same line for the wrong reason and leave
// the class untouched.
//
// **Offline, agent-free and clock-free** (D-35), like `envelope_pr_cap.rs`:
// `hooks::guard_in` is driven in-process against a per-test `TempDir` envelope
// root and a config path that does not exist, so `resolve_policy` applies the
// tighter defaults. One root per test, so the PR-ledger rows cannot contaminate
// each other.
// ============================================================================

use std::path::Path;

use gsd_meta_manager::envelope::{hooks, ledger, policy};
use tempfile::TempDir;

/// The alias every row drives.
const ALIAS: &str = "alpha";

/// One guard answer.
struct Answer {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Answer {
    fn permitted(&self) -> bool {
        self.code == 0
    }

    /// The reason the guard carried in its decision JSON, or the empty string
    /// for a permit (a permit writes nothing at all, by design).
    fn reason(&self) -> String {
        let value: serde_json::Value =
            serde_json::from_str(self.stdout.trim()).unwrap_or(serde_json::Value::Null);
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

/// Ask the guard about one shell command, against an envelope root of its own.
fn ask(envelope_root: &Path, command: &str) -> Answer {
    let request = serde_json::json!({
        "session_id": "fixture",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = hooks::guard_in(
        envelope_root,
        // A path with no registry on it: `resolve_policy` degrades to the
        // TIGHTER defaults rather than failing, which is what makes this
        // fixture's namespace and caps the documented ones.
        &envelope_root.join("no-such-config.json"),
        ALIAS,
        None,
        request.as_bytes(),
        &mut out,
        &mut err,
    )
    .expect("the guard answers rather than erroring");

    Answer {
        code,
        stdout: String::from_utf8(out).unwrap(),
        stderr: String::from_utf8(err).unwrap(),
    }
}

/// The non-empty lines of this alias's pull-request ledger.
fn ledger_lines(envelope_root: &Path) -> Vec<String> {
    let path = ledger::ledger_path_in(envelope_root, ALIAS).expect("a plain alias has a ledger");
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// Assert one command is refused with one specific D-24 reason.
fn refuses(command: &str, reason: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 \
         against the built binary: the guard reads `words[0]`, sees a token it has no \
         classifier for, and falls through to a permit. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}` — the same D-24 identifier the \
         command carries when it is written without its prefix. Asserting the reason \
         and not merely the exit code is what stops a row from passing because it was \
         refused for an unrelated cause, such as words that could not be recovered. \
         Got: {}",
        answer.reason()
    );
}

/// Assert one command is permitted and answers nothing at all.
fn permits(command: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert!(
        answer.permitted(),
        "`{command}` must still be PERMITTED. The guard sees every Bash tool call, so a \
         resolution that denied what it did not recognise would deny this and make a \
         driven run unusable — which is how a safety control gets switched off. \
         stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.stdout.is_empty(),
        "a permit answers nothing at all: emitting `allow` would turn a deny-only \
         control into an approval authority. Got: {}",
        answer.stdout
    );
}

// ---------------------------------------------------------------------------
// 0. The anti-vacuity control — this one passes TODAY
// ---------------------------------------------------------------------------

#[test]
fn the_unwrapped_force_push_is_refused_which_proves_this_harness_can_see_a_denial() {
    // `19-SECURITY.md`: exit=2, denied, correct. If this row is ever red, every
    // other row in the file is red for a reason that has nothing to do with
    // T-19-60 — the harness cannot observe a refusal at all.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

// ---------------------------------------------------------------------------
// 1. The six measured bypass lines
// ---------------------------------------------------------------------------

#[test]
fn a_force_push_behind_env_is_refused() {
    // `19-SECURITY.md`: exit=0, PERMITTED. Four letters in front of the command
    // and layer 2 stops seeing it.
    refuses(
        "env git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn an_envelope_config_assignment_prefix_is_refused_on_its_own_account() {
    // `19-SECURITY.md`: exit=0, PERMITTED — and this is the line that makes
    // T-19-60 high rather than medium. ONE token does two things: it stops
    // layer 2 recognising the command (the assignment is `words[0]`), and it
    // neutralises layer 3 (no injected `core.hooksPath`, so no `pre-push`
    // hook runs). The documented D-09 ceiling says an agent past
    // `GIT_CONFIG_COUNT` is past the LAST layer; in practice one token clears
    // layers 2 and 3 together.
    //
    // **The reason is `hook_bypass_blocked`, not `force_push_blocked`**, and
    // that is the design rather than a concession: the assignment is refused on
    // its own account — for what it does to the hook layer — BEFORE program
    // resolution ever reaches the `push --force` behind it. It parks under the
    // same reason `--no-verify` and `core.hooksPath` already park under, which
    // is where a later reader greps for a disarmed enforcement layer.
    refuses(
        "GIT_CONFIG_COUNT=0 git push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );

    // And the refusal names the key, so the row cannot pass by being refused
    // under `hook_bypass_blocked` for some unrelated hook-bypass cause.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "GIT_CONFIG_COUNT=0 git push --force origin main");
    assert!(
        answer.reason().contains("GIT_CONFIG_COUNT"),
        "the refusal must name the key it refused, or a later reader cannot tell which \
         envelope variable the command was reaching for: {}",
        answer.reason()
    );
}

#[test]
fn a_force_push_behind_timeout_is_refused() {
    // `19-SECURITY.md`: exit=0, PERMITTED.
    refuses(
        "timeout 60 git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn a_force_push_behind_command_is_refused() {
    // `19-SECURITY.md`: exit=0, PERMITTED.
    refuses(
        "command git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared() {
    // Not in the measured table, but the same line one `command -v` away.
    // `/usr/bin/env` and `env` are the same program, and a resolution that
    // compared whole strings would be defeated by the spelling `command -v`
    // prints — which is not even an evasion.
    refuses(
        "/usr/bin/env git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused() {
    // **This row is the whole argument of the plan.** The obvious fix is a list
    // of wrapper names — `env`, `timeout`, `nohup`, `command`, `nice`, `sudo` —
    // plus one test row per listed name. That fix is green on the day it lands
    // and silent on the seventh wrapper nobody listed. None of the five names
    // below may appear anywhere in `src/`: the effective program is resolved
    // STRUCTURALLY, by finding the first token whose basename names a program
    // this envelope governs, so there is nothing for a new wrapper to be
    // missing from.
    refuses(
        "nohup nice -n 10 stdbuf -oL setsid git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

// ---------------------------------------------------------------------------
// 2. The SAFE-06 half: the cap is bypassed by a four-letter prefix
// ---------------------------------------------------------------------------

#[test]
fn a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt() {
    // `19-SECURITY.md`: `env gh pr create --title x` exits 0 and writes NO
    // ledger line, while the unwrapped form writes one. That puts SAFE-06 —
    // "no more than N pull requests per project" — behind four letters, with no
    // ledger line and therefore no park and no evidence.
    let envelope = TempDir::new().unwrap();

    let first = ask(envelope.path(), "env gh pr create --title x");
    assert!(
        first.permitted(),
        "the FIRST wrapped attempt is inside the cap and must succeed — an envelope \
         that refused every pull request would satisfy the refusal assertion below and \
         be useless. stderr: {}",
        first.stderr
    );
    assert_eq!(
        ledger_lines(envelope.path()).len(),
        1,
        "the wrapped form must reach the SAME ledger the unwrapped form reaches. Today \
         it writes nothing, which is what makes the cap bypassable by a prefix."
    );

    // The default per-run cap is 1, so the second attempt in the same run is
    // over the bound even though the rolling window still has capacity.
    let second = ask(envelope.path(), "env gh pr create --title y");
    assert_eq!(
        second.code, 2,
        "the second wrapped attempt must be REFUSED: a cap that only counts the \
         unwrapped spelling is not a cap. stdout: {} stderr: {}",
        second.stdout, second.stderr
    );
    assert!(
        second.reason().contains(policy::REASON_PR_CAP_EXCEEDED),
        "the refusal must carry D-24's taxonomy identifier: {}",
        second.reason()
    );
    assert_eq!(
        ledger_lines(envelope.path()).len(),
        2,
        "and the REFUSED attempt is on disk too, because the ledger records before it \
         permits (D-20): the guard is the only point that observes an attempt at all, \
         so an attempt it saw and did not record is one nothing can ever count"
    );
}

// ---------------------------------------------------------------------------
// 3. The paired allow corpus (D-32) — these pass TODAY and must still pass
// ---------------------------------------------------------------------------

#[test]
fn ordinary_commands_are_still_permitted_and_write_nothing() {
    // `Ungoverned` is a PERMIT and it is the ANSWER, not a fall-through. The
    // guard is registered against every Bash tool call, so a rule of the form
    // "an unrecognised leading token denies" would deny each of these.
    for ordinary in [
        "ls -la",
        "echo hi",
        "cargo test --lib",
        "timeout 5 ls",
        "rg -n TODO src/",
        // A `-c` that is not a shell's `-c`. Its following word is an archive
        // name, which names no governed program, so the payload resolves to
        // nothing rather than to a refusal.
        "tar -czf a.tgz dir",
    ] {
        permits(ordinary);
    }
}

#[test]
fn a_wrapped_read_only_git_command_is_permitted() {
    // The fix must make the wrapper transparent in BOTH directions. If `env git
    // status` were refused, the transparency would be a denial rather than a
    // classification, and the first thing a driven run does is read git state.
    permits("env git status");
    permits("timeout 30 git log --oneline -5");
}

#[test]
fn a_forge_read_is_permitted_and_writes_no_ledger_line() {
    let envelope = TempDir::new().unwrap();

    let listed = ask(envelope.path(), "gh pr list --limit 5");
    assert!(listed.permitted(), "{}", listed.stderr);
    assert!(listed.stdout.is_empty(), "{}", listed.stdout);
    assert!(
        ledger_lines(envelope.path()).is_empty(),
        "a cap that counted reads would refuse a run for looking"
    );

    let wrapped = ask(envelope.path(), "env gh pr list --limit 5");
    assert!(wrapped.permitted(), "{}", wrapped.stderr);
    assert!(
        ledger_lines(envelope.path()).is_empty(),
        "resolving through the wrapper must not turn a read into a counted creation"
    );
}

#[test]
fn an_in_namespace_push_is_permitted_and_writes_nothing() {
    permits("git push origin refs/heads/gsd-auto/alpha/w:refs/heads/gsd-auto/alpha/w");
    permits("env git push origin refs/heads/gsd-auto/alpha/w:refs/heads/gsd-auto/alpha/w");
}

#[test]
fn a_quoted_payload_that_classifies_as_allowed_is_permitted() {
    // The quoted-payload rule DISCRIMINATES rather than blanket-denying: the
    // payload is classified, so a search for an allowed git command is allowed.
    // The paired cost is `T-19-75` — a search for a REFUSED git command is
    // refused — which is accepted, confined to driven runs, and legible rather
    // than silent. That cost is stated in the resolver's own doc rather than
    // left for a reader to discover.
    permits("rg \"git status\" src/");
}
