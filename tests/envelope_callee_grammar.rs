// ============================================================================
// Round 7 — the CALLEE's grammar, which decides which arriving word is the verb.
//
// **What this file is.** The reproducers `19-SECURITY.md`'s SIXTH audit measured
// for `T-19-100` and `T-19-102`, plus the twelve cells found while PLANNING
// round 7, re-measured here against the BUILT BINARY at this file's base commit
// with one fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory
// WALKED afterwards, and — the step that separates a grammar claim from a guess
// — confirmed against the REAL `git` binary with a two-sided probe. Every row was
// driven BEFORE it was written as an assertion, and every one of the audit-6 rows
// reproduced at the recorded verdict; none failed to reproduce.
//
// **The invariant this file is about, stated once.**
//
//   THE WORD THE GUARD CALLS THE VERB MUST BE THE WORD GIT CALLS THE VERB.
//
// Rounds 5 and 6 closed the two halves of the boundary BELOW this one, and audit
// 6 verified that closure mechanically. Round 5: a decision word must be provably
// LITERAL (`tests/envelope_literal_decision.rs`) — audit 5 enumerated bash's word
// expansions one at a time and found no gap in word ASSEMBLY. Round 6: the words
// the guard classifies must be exactly the words the program receives, in the
// same order (`tests/envelope_argv_deletion.rs`) — audit 6 enumerated bash's
// transformations between the command string and `execve` one at a time (quote
// removal, every expansion, redirection, line continuation, assignment-prefix
// removal, control-operator splitting, here-document delimiters, pipeline and
// group nesting) and found every one modelled or failing closed, with **no
// reordering step in a simple command** for a third rule to miss.
//
// > **The command-line-to-argv boundary is CLOSED.** Audit 6 found no third shell
// > mechanism, and says so plainly.
//
// This file is the layer ABOVE. Given the right words in the right order, **which
// one is the command?** — and that is a question about GIT's grammar, not bash's.
//
// ## The mechanism, in git's own terms
//
// `scan_leading` (`src/envelope/policy.rs:325-423`) walks leading `-`-initial
// tokens and asks `leading_git_option` how many words each occupies. Its answer
// for an option it does not recognise is `(None, 1)` — `policy.rs:488`, and that
// single `1` is the whole of `T-19-100`. The loop advances ONE word, lands on the
// option's VALUE, sees the value does not start with `-`, and BREAKS with that
// value as the verb. `classify_git` then finds a verb in no denylist arm and
// answers `Allow`.
//
// Every word on these lines is literal and `Token.literal` is `true` for all of
// them — **round 5's bit is RIGHT** (pinned in section 8). No word is deleted and
// the order is preserved — **round 6's model is RIGHT** (pinned in section 8).
// What is wrong is the verb INDEX, and it is wrong because of a fact about GIT.
//
// ## The exit codes re-measured at this file's base commit, one fresh root per
// ## row, walk after — every row EMPTY unless noted
//
// ```
// exit=0  git --attr-source HEAD push --force origin main          <- T-19-100, ALL THREE LAYERS
// exit=0  git --attr-source HEAD stash                             <- no second carrier
// exit=0  git --attr-source HEAD update-ref -d refs/heads/main      <- no second carrier
// exit=0  git --attr-source HEAD config core.hooksPath /tmp/x       <- disarms layer 3, no carrier
// exit=0  git --attr-source HEAD reflog delete HEAD@{0}             <- no second carrier
// exit=0  git --attr-source HEAD symbolic-ref HEAD refs/heads/x
// exit=0  git --shallow-file /tmp/s push --force origin main
// exit=0  git --shallow-file /tmp/s stash
// exit=0  git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main  <- LAYER 2
// exit=2  git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w   <- T-19-102
//         (reason: push_outside_namespace — "the refspec `origin` resolves to
//          `refs/heads/origin`, which is outside `refs/heads/gsd-auto/alpha/`")
// ```
//
// The ATTACHED spelling is correctly refused TODAY —
// `git --attr-source=HEAD push --force origin main` -> exit 2 `force_push_blocked`
// — so this is the SEPARATE-VALUE form specifically, and that asymmetry is the
// shape of the fix: git's own grammar makes an attached value self-contained
// whatever the option is, so the structural rule needs no knowledge of git at all.
//
// ## The three-leg argument, as audit 6 measured it
//
// `T-19-100` defeats all three layers on one line:
//
//   1. layer 1's `Bash(git push:*)` prefix rule does not match a line beginning
//      `git --attr-source`;
//   2. layer 2 — this guard — is defeated by the verb-index defect itself;
//   3. layer 3 is disarmed on the same line, because a command-line `-c` outranks
//      the envelope's env-injected `core.hooksPath` (D-09).
//
// **Four of the rows have no second carrier at all** — `stash`, `update-ref -d`,
// `reflog delete` and `config core.hooksPath`, where disarming the hook IS the
// loss of the carrier. Audit 6 confirmed the class by performing a REAL
// destructive force push that rewrote a bare remote's `main`.
//
// ## The two-sided real-git probe, re-measured for this file on git 2.43.0
//
// For each candidate option: `git <opt> version` (if it prints a version, the
// option did NOT consume the next word) versus `git <opt> XVALUE version` (if it
// prints a version, the option DID consume `XVALUE`).
//
// ```
// CONSUMES A SEPARATE WORD:  -c  --git-dir  --work-tree  --namespace
//                            --attr-source  --shallow-file
//                            -C           (via `cannot change to 'version'`)
//                            --config-env (via `invalid config format: version`)
// SELF-CONTAINED (boolean):  --no-pager  -p  --paginate  -P  --bare
//                            --no-replace-objects  --literal-pathspecs
//                            --glob-pathspecs  --noglob-pathspecs
//                            --icase-pathspecs  --no-optional-locks
// TERMINATING (identical):   --exec-path  --html-path  --man-path  --info-path
//                            --version
// NOT ACCEPTED by this git:  --super-prefix  --no-lazy-fetch  --no-advice
//                            --bogus-opt
// UNPROBED, reason stated:   --help  -h   (`git --help XVALUE version` answers
//                            `No manual entry for gitXVALUE`, so no probe of this
//                            shape can classify them)
// ```
//
// **The list is wrong in BOTH directions against the installed git, and that is
// the evidence the list itself is the defect.** `--attr-source` and
// `--shallow-file` DO consume a separate word and are ABSENT from
// `GIT_GLOBAL_VALUE_OPTS`; `--super-prefix`, which the constant CARRIES, prints
// `unknown option: --super-prefix` in both forms. The over-consuming direction is
// the dangerous one and it is demonstrated in section 3:
// `git --super-prefix push --force origin main` -> exit 0, because the guard
// swallows the real verb `push` as `--super-prefix`'s value and reads `origin` as
// the verb. It is not a LIVE bypass only because git itself rejects the option —
// a stale entry for an option git ACCEPTS would be one.
//
// ## Written BEFORE the rule, and that ordering is the whole point of this file
// ## existing separately from the fix
//
// For SIX consecutive rounds a control in this phase was certified by a corpus
// structurally incapable of failing on the class that got through it —
// `T-19-76`, `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99`, now `T-19-101`. This
// time the gap moved AXIS rather than one cell over: `UNREADABLE_CLASSES` names
// seven word-ASSEMBLY classes and `DELETION_CLASSES` names five word-REMOVAL
// classes, and **both are axes of the SHELL's grammar**. Audit 6 verified
// mechanically that nothing anywhere in `tests/` modelled the CALLEE's —
// `grep -rn "attr-source\|shallow-file\|GIT_GLOBAL_VALUE_OPTS" tests/` returned
// nothing at all. Plan `19-20` writes the corpus and the reproducers and STOPS;
// plan `19-21` writes the rule. A corpus written after a fix cannot be
// distinguished from a corpus written to agree with what the code already did.
//
// **Why a SIXTH file rather than more rows in `tests/envelope_argv_deletion.rs`.**
// That file is round 6's evidence and its header states the exit codes round 6
// measured. Round 7's evidence lives here so a later reader can tell which round
// produced which measurement, and so neither header has to be rewritten to stay
// true. `19-14` created a third file, `19-16` a fourth and `19-18` a fifth for
// exactly this reason.
//
// ## MOST OF THIS FILE IS DELIBERATELY RED AT PLAN 19-20'S END
//
// Every test whose name begins `after_19_21_` is EXPECTED TO FAIL against the
// pre-fix tree. That is not a defect and it is not a regression: it is the
// evidence that the corpus was capable of failing before the code changed.
// `19-20-SUMMARY.md` lists every red name; `19-21` confirms the same names still
// red before it writes a line, and closes them.
//
// Every test whose name does NOT begin `after_19_21_` passes TODAY. Those are the
// controls: the anti-vacuity rows, the walk's positive control, the permitted
// half, the already-correct planning cells, the bare-dash pin, the three
// recorded-not-asserted rows, and the four carried-forward mechanism pins.
//
// ## EVERY POST-FIX EXPECTATION CARRIES A WRITTEN DERIVATION
//
// This plan measures PRE-fix, so its measure-first discipline cannot catch a
// wrong POST-fix expectation. A row pinned at a verdict the rule cannot produce
// lands RED with `19-21` forbidden to edit it — which stalls the round. So every
// asserted post-fix verdict is DERIVED in writing beside the row from `19-21`'s
// mandated design (`leading_git_option` answers a three-valued grammar question
// whose third value, *grammar not established*, makes `scan_leading` refuse at
// `ParkReason::EnvelopeAssertionFailed`), and the post-deletion argv each
// derivation names is itself MEASURED in section 0 rather than described. **Three
// rows whose post-fix verdict cannot be derived are RECORDED IN COMMENTS AND
// PRINTS and are NOT written as assertions** (section 6).
//
// **This seam needs NO replacement exception and none is granted.** Three rounds
// running, this phase produced a two-plan handoff that was unsatisfiable as
// written — a pin the second plan had to MOVE under a prohibition allowing only
// additions. The rule adopted here instead: assert the POST-fix verdict wherever
// it is derivable, so the row is RED now and GREEN after and no body ever needs
// replacing; record the PRE-fix measurement in the doc comment, where the
// evidence actually belongs. `T-19-102`'s false refusal is therefore asserted at
// **exit 0**, not at its measured exit 2.
//
// ## What stops this file passing vacuously
//
// 1. Anti-vacuity controls that pass TODAY (section 0), including a POSITIVE
//    control proving the ledger walk can see a line at all.
// 2. A paired PERMITTED half (section 4): twelve ordinary invocations pinned at
//    exit 0. **`git --no-pager status` is this axis's `ls {git,svn}-repo`** — the
//    row that tells a grammar MODEL apart from a blanket refusal of anything
//    beginning with `-`, which is the shape this fix is one wrong step away from.
// 3. The COST rows (section 5), each beside its permitted twin, so the
//    over-refusal this round ADDS is measured rather than hidden.
// 4. The `--signed no` discrimination control (section 2), which turns red on a
//    fix copied from `git push -h`.
// 5. Every refusal row asserts the D-24 reason identifier as well as the exit
//    code, so a row cannot pass by being refused for an unrelated cause.
// 6. Every row asserts what the WALKED envelope root holds.
// 7. Four MECHANISM pins (section 8) assert over `policy::split_segments_with_heads`
//    and `policy::is_separator` rather than over exit codes, so `19-21` cannot
//    make Rule B, round 5's inversion or round 6's deletion model dead code while
//    every verdict pin stays green.
//
// **Offline, agent-free and clock-free** (D-35). `hooks::guard_in` is driven
// in-process against a per-row `TempDir` envelope root and a config path that
// does not exist, so `resolve_policy` applies the tighter defaults. One root per
// row, because the PR ledger persists and a shared root produces misleading
// cap-exhaustion refusals. The `T-19-102` rows build a local git fixture with a
// bare local upstream; it is offline and it does NOT consult the test process's
// own working directory, which is the `T-19-80` failure this file must not
// reintroduce.
//
// **No crate was added** (`T-19-SC`). The directory walk below is eight lines of
// `std::fs` and the real-git probes shell out through `std::process::Command`.
// ============================================================================

use std::path::{Path, PathBuf};

use gsd_meta_manager::envelope::{hooks, policy};
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

/// Ask the guard about one shell command, against a given envelope root and an
/// EXPLICIT project root.
///
/// Carried forward unchanged in shape from `tests/envelope_argv_deletion.rs`.
/// The project root is a parameter rather than the process's working directory on
/// purpose: `push_needs_resolved_dests` makes the guard resolve a push context
/// from a repository, and a test that let that repository be whatever directory
/// `cargo test` happened to run in would have a verdict that depends on the
/// checkout — which is `T-19-80`.
fn ask_in(envelope_root: &Path, project_root: Option<&Path>, command: &str) -> Answer {
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
        &envelope_root.join("no-such-config.json"),
        ALIAS,
        project_root,
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

/// Ask the guard with no project root, which is what every row that does not need
/// a resolved push destination uses.
fn ask(envelope_root: &Path, command: &str) -> Answer {
    ask_in(envelope_root, None, command)
}

// ---------------------------------------------------------------------------
// The walk — carried forward unchanged from `tests/envelope_argv_deletion.rs`
// ---------------------------------------------------------------------------

/// Every file under `dir`, recursively.
///
/// **Why a walk rather than a read of `ledger::ledger_path_in`.** A row that
/// asserted "the expected ledger path is empty" would pass identically if the
/// line had been written somewhere else under the envelope root. The absence is
/// OBSERVED rather than derived from a read of one expected path.
fn envelope_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(envelope_files(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Every line ANYWHERE under the envelope root that is a pull-request ledger
/// entry, paired with the file it was found in.
fn ledger_lines_anywhere(root: &Path) -> Vec<(PathBuf, String)> {
    let mut lines = Vec::new();
    for path in envelope_files(root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if line.contains("\"platform\"") {
                lines.push((path.clone(), line.to_string()));
            }
        }
    }
    lines
}

/// A printable listing of the whole envelope root, for a failure message and for
/// the SUMMARY.
fn listing(root: &Path) -> String {
    let files = envelope_files(root);
    if files.is_empty() {
        return "(the envelope root is empty)".to_string();
    }
    files
        .iter()
        .map(|path| {
            let rel = path.strip_prefix(root).unwrap_or(path);
            let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            format!("  {} ({bytes} bytes)", rel.display())
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// The assertions
// ---------------------------------------------------------------------------

/// Assert one command is refused with one specific D-24 reason, against an
/// envelope root of its own, AND that the walked root holds no ledger line.
fn refuses(command: &str, reason: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. It was measured at exit 0 against the BUILT BINARY at \
         this file's base commit, with a fresh envelope root and a walk after, and the verb \
         git itself reaches was confirmed with a two-sided probe of the real git binary. \
         stdout: {} stderr: {}\nWalked listing:\n{}",
        answer.stdout,
        answer.stderr,
        listing(envelope.path())
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}`. Asserting the identifier and not \
         merely the exit code is what stops a row from passing because it was refused for \
         an unrelated cause (D-24). Got: {}",
        answer.reason()
    );

    let written = ledger_lines_anywhere(envelope.path());
    assert!(
        written.is_empty(),
        "`{command}` was refused, but a pull-request ledger line was written somewhere under \
         the envelope root. Found: {written:?}\nWalked listing:\n{}",
        listing(envelope.path())
    );
}

/// Assert one command is permitted and answers nothing at all.
fn permits(command: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 0,
        "`{command}` must be PERMITTED. The guard sees EVERY Bash tool call, so a rule that \
         refused every leading option would refuse this and make a driven run unusable — \
         which is how a safety control gets switched off (AR-19-11). stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.stdout.is_empty(),
        "a permit answers nothing at all: emitting `allow` would turn a deny-only control \
         into an approval authority. Got: {}",
        answer.stdout
    );
}

/// Drive one row and PRINT what it answered, asserting nothing about it.
///
/// Section 6's three rows use this and only this. A row whose POST-fix verdict
/// this plan cannot DERIVE must not be pinned: this plan measures PRE-fix, so its
/// measure-first discipline cannot catch a wrong post-fix expectation, and a row
/// pinned at a verdict the rule cannot produce lands RED with `19-21` forbidden
/// to edit it — which stalls the round. `19-16` and `19-18` handled their
/// undeliverable rows exactly this way.
fn record_only(label: &str, command: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);
    println!(
        "RECORDED (not asserted) [{label}]\n  command : {command}\n  exit    : {}\n  \
         reason  : {}\n  walk    :\n{}",
        answer.code,
        if answer.reason().is_empty() {
            "(a permit answers nothing at all)".to_string()
        } else {
            answer.reason()
        },
        listing(envelope.path())
    );
}

// ---------------------------------------------------------------------------
// The in-namespace git fixture — offline, and NOT the test process's cwd
// ---------------------------------------------------------------------------

fn git_ok(cwd: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "user.name=fixture",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("git is on PATH");
    assert!(
        status.success(),
        "the fixture command `git {}` failed in {}",
        args.join(" "),
        cwd.display()
    );
}

/// A repository whose current branch is INSIDE the envelope's namespace
/// (`refs/heads/gsd-auto/alpha/…`), with a local bare upstream configured.
///
/// Carried forward unchanged in shape from `tests/envelope_argv_deletion.rs`.
/// Returned as a `TempDir` so the caller owns its lifetime; the repository is
/// passed to the guard EXPLICITLY, so no verdict here depends on where
/// `cargo test` was invoked (`T-19-80`).
fn in_namespace_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let upstream = dir.path().join("upstream.git");
    let work = dir.path().join("work");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::create_dir_all(&work).unwrap();

    git_ok(&upstream, &["init", "--bare", "-q", "."]);
    git_ok(&work, &["init", "-q", "-b", "gsd-auto/alpha/w", "."]);
    std::fs::write(work.join("a"), "x\n").unwrap();
    git_ok(&work, &["add", "a"]);
    git_ok(&work, &["commit", "-qm", "init"]);
    git_ok(
        &work,
        &["remote", "add", "origin", upstream.to_str().unwrap()],
    );
    git_ok(&work, &["push", "-q", "-u", "origin", "gsd-auto/alpha/w"]);

    dir
}

// ===========================================================================
// 0. The anti-vacuity controls — every one of these passes TODAY
// ===========================================================================

#[test]
fn the_unwrapped_refusals_still_fire_which_proves_this_harness_can_see_a_denial() {
    // If any row here is ever red, every refusal row in this file is red for a
    // reason that has nothing to do with a leading git option — the harness
    // cannot observe a refusal at all.
    //
    // **These are also the POST-CONSUMPTION argv of section 1's rows, measured
    // here so each derivation below names a verdict this file has itself
    // observed rather than one it describes.** Once `--attr-source` consumes
    // `HEAD` and `--shallow-file` consumes `/tmp/s`, what is left IS one of the
    // lines below.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses("git stash", policy::REASON_FORCE_PUSH_BLOCKED);
    refuses(
        "git update-ref -d refs/heads/main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses(
        "git config core.hooksPath /tmp/x",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
    refuses(
        "git reflog delete HEAD@{0}",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses(
        "git symbolic-ref HEAD refs/heads/x",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses(
        "git -c core.hooksPath=/dev/null push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
}

#[test]
fn the_attached_value_spelling_is_already_refused_and_is_a_control_not_a_reproducer() {
    // **The asymmetry that is the shape of the fix.** Git's own grammar makes an
    // ATTACHED value self-contained whatever the option is — the `=` is inside
    // the token, so no following word can be consumed — and the guard already
    // gets this right without knowing anything about `--attr-source`. Measured
    // at this file's base commit:
    //
    //   git --attr-source=HEAD push --force origin main
    //     -> exit 2  force_push_blocked
    //
    // This row is therefore a CONTROL and never a reproducer: an alphabet drawn
    // in the attached position would be green before the fix and would certify
    // nothing. It is also the reason `19-21`'s structural rule runs FIRST — an
    // attached `=` needs no list entry at all, so the constants have less to know.
    refuses(
        "git --attr-source=HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all() {
    // **Without this row, every no-ledger-line assertion in this file could be
    // passing because the walk cannot see anything.** "The line is not there" and
    // "this scanner cannot read the file" are indistinguishable without a
    // positive control.
    //
    // `gh pr create --title x` is measured at exit 0 with EXACTLY ONE ledger line
    // at this file's base commit — walked listing
    // `alpha/pr-ledger.ndjson (103 bytes)`.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 0,
        "the first `gh pr create` of a run is PERMITTED. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    let written = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        written.len(),
        1,
        "the walk must be able to SEE a ledger line, or every empty-walk assertion in this \
         file is vacuous. Walked listing:\n{}",
        listing(envelope.path())
    );
}

// ===========================================================================
// 1. T-19-100 — the nine rows audit 6 measured, asserted at their POST-FIX
//    verdict with a DERIVED reason identifier each
// ===========================================================================

#[test]
fn after_19_21_a_leading_git_option_that_consumes_its_value_no_longer_hides_the_verb() {
    // **RED at plan 19-20's end, by construction.** All nine rows are measured at
    // exit 0 against the built binary at this file's base commit, one fresh
    // envelope root each, the whole root walked afterwards and EMPTY in every one.
    //
    // **The mechanism, in git's own terms.** `scan_leading` asks
    // `leading_git_option` how many words `--attr-source` occupies. Neither
    // `--attr-source` nor `--shallow-file` is in `GIT_GLOBAL_VALUE_OPTS`, so the
    // answer is the fail-open default `(None, 1)` at `policy.rs:488`. The loop
    // advances one word, lands on `HEAD` (or `/tmp/s`), sees it does not start
    // with `-`, and BREAKS with that value as the verb. `classify_git` finds
    // `HEAD` in no denylist arm and answers `Allow`.
    //
    // **Confirmed against the REAL git binary, two-sided, on git 2.43.0**:
    //   git --attr-source version         -> usage: git [-v | --version] …
    //   git --attr-source XVALUE version  -> git version 2.43.0
    //   git --shallow-file version        -> usage: git [-v | --version] …
    //   git --shallow-file XVALUE version -> git version 2.43.0
    // Both options DO consume a separate word, so git really does reach the verb
    // the guard did not identify. Confirmed by an actual run in a fixture:
    // `git --attr-source HEAD status` prints `On branch gsd-auto/alpha/w` and
    // `git --shallow-file /tmp/s log --oneline` prints `7347130 init`.
    //
    // **THE THREE-LEG ARGUMENT, as audit 6 measured it.** Layer 1's
    // `Bash(git push:*)` prefix rule does not match a line beginning
    // `git --attr-source`; layer 2 — this guard — is defeated by the verb-index
    // defect itself; layer 3 is disarmed on the same line, because a command-line
    // `-c` outranks the envelope's env-injected `core.hooksPath` (D-09).
    // **FOUR of these rows have no second carrier AT ALL** — `stash`,
    // `update-ref -d`, `reflog delete` and `config core.hooksPath`, where
    // disarming the hook IS the loss of the carrier. Audit 6 confirmed the class
    // by performing a REAL destructive force push that rewrote a bare remote's
    // `main`.
    //
    // **THE DERIVATION for each post-fix identifier.** `19-21` makes
    // `leading_git_option` three-valued and ADDS `--attr-source` and
    // `--shallow-file` to the value-taking constant, so each option consumes its
    // value and the verb index advances by two. The surviving argv is then
    // EXACTLY one of the lines section 0 measures, and the identifier is the one
    // section 0 observed for it. Not described — observed.

    // -- `push --force` -> force_push_blocked (section 0: `git push --force origin main`)
    refuses(
        "git --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses(
        "git --shallow-file /tmp/s push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- `stash` -> force_push_blocked, NO SECOND CARRIER
    //    (section 0: `git stash`)
    refuses(
        "git --attr-source HEAD stash",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses(
        "git --shallow-file /tmp/s stash",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- `update-ref -d` -> force_push_blocked, NO SECOND CARRIER
    //    (section 0: `git update-ref -d refs/heads/main`)
    refuses(
        "git --attr-source HEAD update-ref -d refs/heads/main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- `config core.hooksPath /tmp/x` -> hook_bypass_blocked, NO SECOND
    //    CARRIER: this row DISARMS layer 3, so the loss of the hook IS the loss
    //    of the carrier. Two key operands make it a WRITE of `core.hooksPath`
    //    (section 0: `git config core.hooksPath /tmp/x`).
    refuses(
        "git --attr-source HEAD config core.hooksPath /tmp/x",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );

    // -- `reflog delete` -> force_push_blocked, NO SECOND CARRIER
    //    (section 0: `git reflog delete HEAD@{0}`)
    refuses(
        "git --attr-source HEAD reflog delete HEAD@{0}",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- `symbolic-ref HEAD <ref>` -> force_push_blocked. Two operands make it a
    //    WRITE of HEAD (section 0: `git symbolic-ref HEAD refs/heads/x`).
    refuses(
        "git --attr-source HEAD symbolic-ref HEAD refs/heads/x",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- THE LAYER-2 ROW. Its derivation is DIFFERENT from the eight above and
    //    the comment says so rather than leaving it to be inferred: the refusal
    //    comes from the `-c` KEY CHECK inside `scan_leading`, which fires as soon
    //    as the scan REACHES the `-c` token — it does not depend on the verb at
    //    all. Today the scan never reaches it, because it breaks on `HEAD` first.
    //    (section 0: `git -c core.hooksPath=/dev/null push --force origin main`)
    refuses(
        "git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
}

// ===========================================================================
// 2. T-19-102 — the same enumeration defect in the OVER-REFUSAL direction,
//    asserted at its POST-FIX verdict
// ===========================================================================

#[test]
fn after_19_21_the_in_namespace_push_with_a_value_taking_flag_is_no_longer_falsely_refused() {
    // **RED at plan 19-20's end, and the red assertion is written LAST so every
    // green control in this test is actually exercised today.**
    //
    // **BEFORE (measured at this file's base commit, in the fixture below):**
    //   git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w
    //     -> exit 2  REFUSED (reason: push_outside_namespace) — "the refspec
    //        `origin` resolves to `refs/heads/origin`, which is outside
    //        `refs/heads/gsd-auto/alpha/`"
    //
    // **AFTER (`19-21`'s rule):** exit 0, matching its one-line twin.
    //
    // **THE DERIVATION.** `PUSH_VALUE_OPTS` is `["repo", "push-option",
    // "receive-pack", "exec"]` — `recurse-submodules` is absent, so
    // `push_operands` does not skip `on-demand`, reads it as the REPOSITORY and
    // `origin` as a REFSPEC, and `origin` resolves to `refs/heads/origin`.
    // `19-21` adds `recurse-submodules` and nothing else, so `on-demand` is
    // consumed, `origin` is the repository and `refs/heads/gsd-auto/alpha/w` is
    // the single in-namespace refspec — which is the twin's argv exactly.
    //
    // **REAL GIT runs this line to completion**: in the fixture,
    // `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w`
    // answers `Everything up-to-date`. So it is a FALSE REFUSAL of an ordinary
    // in-namespace push, not a refusal of something that does not run.
    //
    // **ASSERTING THE POST-FIX VERDICT RATHER THAN THE PRE-FIX ONE IS DELIBERATE
    // AND IS THIS PLAN'S CORRECTION TO HOW THE LAST THREE ROUNDS HANDED OFF.** A
    // pin the next plan must MOVE, under a prohibition allowing only additions,
    // is a plan that cannot be executed. This seam therefore grants `19-21` no
    // replacement exception and needs none.
    let repo = in_namespace_repo();
    let work = repo.path().join("work");

    let assert_code = |command: &str, expected: i32, why: &str| {
        let envelope = TempDir::new().unwrap();
        let answer = ask_in(envelope.path(), Some(&work), command);
        assert_eq!(
            answer.code, expected, "{why}\n  command: {command}\n  stdout: {}\n  stderr: {}",
            answer.stdout, answer.stderr
        );
        answer
    };

    // -- GREEN TODAY AND AFTER: the one-line twin the red row must come to match.
    assert_code(
        "git push origin refs/heads/gsd-auto/alpha/w",
        0,
        "the one-line twin is PERMITTED — the refspec is in-namespace and nothing else \
         changed. If this is red, the fixture is wrong and the row below proves nothing.",
    );

    // -- GREEN TODAY AND AFTER: the two short spellings of an option
    //    `PUSH_VALUE_OPTS` ALREADY knows, which is why they are already correct.
    assert_code(
        "git push -o ci.skip origin refs/heads/gsd-auto/alpha/w",
        0,
        "`-o` is `--push-option`'s short spelling and is already handled",
    );
    assert_code(
        "git push --push-option ci.skip origin refs/heads/gsd-auto/alpha/w",
        0,
        "`--push-option` is already in `PUSH_VALUE_OPTS`",
    );

    // -- GREEN TODAY AND AFTER: the ATTACHED form, which needs no list entry
    //    because git's own grammar makes it self-contained.
    assert_code(
        "git push --recurse-submodules=on-demand origin refs/heads/gsd-auto/alpha/w",
        0,
        "the attached spelling is self-contained and is already correct — which is why the \
         separate-value form specifically is the defect",
    );

    // -----------------------------------------------------------------------
    // THE DISCRIMINATION CONTROL THAT MUST STAY REFUSED.
    //
    // `git push -h` spells the two options like this:
    //
    //     --recurse-submodules (check|on-demand|only|no)
    //     --signed[=(yes|no|if-asked)]
    //
    // — a REQUIRED value and an ATTACHED-ONLY optional one, and the help text
    // does not make the difference obvious. **A fix that completed
    // `PUSH_VALUE_OPTS` from that help text would add `signed` and introduce a
    // REAL MIS-PARSE**, because real git does NOT consume `no`: measured in the
    // fixture,
    //
    //     git push --dry-run --signed no origin refs/heads/gsd-auto/alpha/w
    //       -> error: src refspec origin does not match any
    //          error: failed to push some refs to 'no'
    //
    // git reads `no` as the REPOSITORY and `origin` as a refspec. The guard's
    // current refusal is therefore CORRECT for this line and must survive
    // `19-21` unchanged. This row is what turns red on the copied fix.
    // -----------------------------------------------------------------------
    let signed = assert_code(
        "git push --signed no origin refs/heads/gsd-auto/alpha/w",
        2,
        "DISCRIMINATION CONTROL: `--signed` takes an ATTACHED-ONLY optional value, so real \
         git reads `no` as the repository and `origin` as a refspec. A fix that added \
         `signed` to `PUSH_VALUE_OPTS` from `git push -h` would introduce a real mis-parse. \
         The correct response to a red here is to REMOVE `signed` from the constant, never \
         to delete this row.",
    );
    assert!(
        signed.reason().contains(policy::REASON_PUSH_OUTSIDE_NAMESPACE),
        "and under the namespace identifier, so the row cannot pass by being refused for an \
         unrelated cause (D-24). Got: {}",
        signed.reason()
    );

    // -----------------------------------------------------------------------
    // THE RED ROW.
    // -----------------------------------------------------------------------
    assert_code(
        "git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w",
        0,
        "OVER-REFUSAL: this in-namespace push is FALSELY REFUSED at exit 2 \
         `push_outside_namespace` before the rule exists, and `19-21` must move it to exit 0 \
         by adding `recurse-submodules` — and ONLY `recurse-submodules` — to \
         `PUSH_VALUE_OPTS`. Real git runs this line to completion (`Everything up-to-date`), \
         so the refusal is of an ordinary in-namespace push.",
    );
}

// ===========================================================================
// 3. The cells found while PLANNING round 7
//
// Same provenance caveat `19-14`, `19-16` and `19-18` established: found while
// planning, not by an audit. All measured at this file's base commit, all inside
// `T-19-100`'s class rather than new threats. The five COMPOSITION cells live in
// section 7, where their point is made.
// ===========================================================================

#[test]
fn after_19_21_the_stale_and_bundled_planning_cells_are_refused() {
    // **RED at plan 19-20's end.** Three cells, each adding something the nine
    // audit-6 rows do not.

    // -- THE STALE ENTRY, OVER-CONSUMING — the list wrong in the direction that
    //    is a BYPASS rather than an over-refusal, and the measurement that names
    //    the defect as the LIST rather than as a missing row.
    //
    //    Measured: `git --super-prefix push --force origin main` -> exit 0.
    //    `--super-prefix` IS in `GIT_GLOBAL_VALUE_OPTS`, so the guard swallows
    //    the real verb `push` as its value and reads `origin` as the verb. Its
    //    twin `git --super-prefix x push --force origin main` -> exit 2, which is
    //    the two-sided proof that the entry is what moves the index.
    //
    //    **REAL GIT REJECTS IT**: `git --super-prefix version` and
    //    `git --super-prefix XVALUE version` both print
    //    `unknown option: --super-prefix` on git 2.43.0. **This row is therefore
    //    LABELLED a MIS-INDEX OF A COMMAND THAT DOES NOT RUN, never a live
    //    bypass.** A stale entry for an option git ACCEPTS would be one, and that
    //    is precisely why `19-21` pins both constants against the real binary.
    //
    //    DERIVATION: `19-21` REMOVES `--super-prefix` from the value-taking
    //    constant and does not add it to the self-contained one, so it falls to
    //    *grammar not established* and `scan_leading` refuses at
    //    `ParkReason::EnvelopeAssertionFailed`.
    refuses(
        "git --super-prefix push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // -- THE UNKNOWN OPTION AFTER A KNOWN ONE — the scan is a LOOP, so the gap is
    //    not confined to the first slot. Measured: exit 0.
    //
    //    DERIVATION: `-c a=b` occupies two words today and after; `--attr-source`
    //    then consumes `HEAD`; the verb is `push` and `--force` is present, so
    //    the identifier is section 0's `git push --force origin main` verdict.
    refuses(
        "git -c a=b --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- THE SHORT BUNDLE — the verb is read as `user.name=x`. Measured: exit 0.
    //
    //    **REAL GIT REJECTS IT**: `git -pc user.name=x version` prints
    //    `unknown option: -pc` on git 2.43.0 — this git accepts NO short-option
    //    bundling (`-pP`, `-C/tmp` and `-` are all `unknown option` too).
    //    **LABELLED a MIS-INDEX OF A COMMAND THAT DOES NOT RUN.**
    //
    //    DERIVATION: `-pc` does not match `-c`'s prefix arm (the arm requires the
    //    token to START with `-c`), is in neither constant, and so falls to
    //    *grammar not established*.
    refuses(
        "git -pc user.name=x status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_bare_dash_stays_permitted_because_the_rule_must_not_widen_into_non_option_words() {
    // **GREEN TODAY AND GREEN AFTER `19-21` — and this pin is why a bare `-` is
    // deliberately NOT an entry of `GIT_GLOBAL_OPTIONS` in
    // `tests/envelope_wrapper_class.rs`.**
    //
    // Measured at this file's base commit: `git - push --force origin main`
    // -> exit 0, empty walk. `scan_leading` BREAKS on `-` before any option check
    // runs, so the verb is `-` and no denylist arm matches it.
    //
    // **REAL GIT REJECTS IT**: `git - version` prints `unknown option: -`.
    // LABELLED a mis-index of a command that does not run.
    //
    // `19-21` is FORBIDDEN to widen the rule into non-`-`-prefixed words: the
    // breaks on a non-`-` token, on `-` and on `--` are the scan's termination
    // conditions and moving them is a different change with a different blast
    // radius. If an executor had put `-` into the class-5 alphabet, the generative
    // property's refused arm would assert a refusal the rule may never produce —
    // permanently red in a file `19-21` may not edit. That is `19-18`'s `{v}>`
    // blocker one axis over, and this pin is the other half of preventing it.
    permits("git - push --force origin main");
}

#[test]
fn the_already_correct_planning_cells_keep_their_verdicts_as_controls() {
    // **GREEN TODAY AND AFTER.** Three cells that are already right, pinned so
    // `19-21` cannot pay for its rule by moving them.

    // -- THE ATTACHED SHORT `-C`. Already refused: `-C/tmp` does not match the
    //    bare `-C` entry, so the scan does not consume `push`. Real git rejects
    //    `-C/tmp` too (`unknown option: -C/tmp`), so there is no attached short
    //    form to model.
    refuses(
        "git -C/tmp push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- `--` END OF OPTIONS. Already correct: the scan breaks on `--` and the
    //    next word is the verb.
    refuses(
        "git -- push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- THE HOOKS KEY AHEAD OF THE GAP. The `-c` key check fires as soon as the
    //    scan reaches it, BEFORE any verb is identified, so this line is refused
    //    even though the gap is open one token later.
    refuses(
        "git -c core.hooksPath=/dev/null --attr-source HEAD push",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );

    // -- THE TWO-SIDED TWIN of the stale-entry cell, which is what proves the
    //    entry is what moves the verb index.
    refuses(
        "git --super-prefix x push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

// ===========================================================================
// 4. The PERMITTED half — what makes this a grammar MODEL rather than a
//    blanket refusal of anything beginning with `-`
// ===========================================================================

#[test]
fn the_permitted_half_of_the_callee_axis_stays_permitted() {
    // **GREEN TODAY AND GREEN AFTER.** All twelve measured at exit 0 with an
    // empty walk at this file's base commit.
    //
    // **`git --no-pager status` IS THIS AXIS'S `ls {git,svn}-repo`.** A corpus
    // without the permitted half cannot fail on an implementation that refuses
    // every leading option — and that is the shape this fix is ONE WRONG STEP
    // away from. A rule that refused everything beginning with `-` would close
    // `T-19-100` completely and be a guard nobody can use, which is how a safety
    // control gets switched off (AR-19-11).
    //
    // The classification of each option below comes from the two-sided real-git
    // probe recorded in this file's header, not from a guess:
    //   self-contained: --no-pager, --bare, --literal-pathspecs,
    //                   --no-optional-locks
    //   consumes a word: -c, -C, --attr-source, --shallow-file
    //   attached value:  --git-dir=/tmp/g
    //   terminating:     --exec-path, --version
    permits("git --no-pager status");
    permits("git --no-pager log --oneline");
    permits("git -c user.name=\"$NAME\" commit -m x");
    permits("git --git-dir=/tmp/g status");
    permits("git -C /tmp status");
    permits("git --bare status");
    permits("git --literal-pathspecs status");
    permits("git --no-optional-locks status");
    permits("git --exec-path status");
    permits("git --version");

    // **The two that carry their real-git confirmation**, because their whole
    // point is that the value really is consumed and the verb really is reached:
    //   git --attr-source HEAD status        -> On branch gsd-auto/alpha/w
    //   git --shallow-file /tmp/s log --oneline -> 7347130 init
    // both run to completion in the fixture. So `19-21` teaching the guard that
    // these options consume a word must NOT cost these ordinary invocations.
    permits("git --attr-source HEAD status");
    permits("git --shallow-file /tmp/s log --oneline");
}

// ===========================================================================
// 5. The COST rows — the over-refusal this round ADDS, measured rather than
//    hidden, each beside its permitted twin
// ===========================================================================

#[test]
fn after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins() {
    // **RED at plan 19-20's end.** All four measured at exit 0 today.
    //
    // **This is the price of inverting the failure direction and the plan does
    // not hide it.** `19-21` makes *grammar not established* a REFUSAL, so an
    // option the guard's constants do not carry is refused even on an otherwise
    // ordinary line. On the INSTALLED git 2.43.0 the cost is **ZERO**: every
    // option this git accepts is classified by the probe and enumerated, and the
    // only rows moving permitted -> refused are ones git ITSELF rejects, so they
    // are refusals of commands that already do nothing.
    //
    // On a FUTURE git the cost is **one refusal per newly added global option
    // until the constant learns it**. `--no-advice` and `--no-lazy-fetch` are the
    // MEASURED STAND-INS for exactly that: both are real git global options in
    // releases after 2.43, and both print `unknown option:` on the installed git.
    // They are in this corpus because a future cost argued in prose is a cost
    // nobody can check.
    //
    // DERIVATION for all four: none is in either of `19-21`'s two constants, none
    // carries an attached `=`, and none is `-`, `--` or a non-option word — so
    // each falls to *grammar not established* and `scan_leading` refuses at
    // `ParkReason::EnvelopeAssertionFailed`.
    refuses("git --bogus-opt status", policy::REASON_ENVELOPE_ASSERTION_FAILED);
    refuses("git --no-advice status", policy::REASON_ENVELOPE_ASSERTION_FAILED);
    refuses(
        "git --no-lazy-fetch status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // `--super-prefix x` is the same class for a different reason: the entry is
    // being REMOVED from the value-taking constant because the installed git
    // prints `unknown option: --super-prefix`, and it is not added to the
    // self-contained one.
    refuses(
        "git --super-prefix x status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // **THE PERMITTED TWINS, green today and after.** Without them the four rows
    // above would be satisfied by a rule that refused `git status` as well.
    permits("git status");
    permits("git log --oneline");
}

// ===========================================================================
// 6. The THREE rows whose POST-FIX verdict this plan CANNOT DERIVE —
//    RECORDED, never asserted
// ===========================================================================

#[test]
fn the_three_undeliverable_rows_are_recorded_here_and_left_for_19_21_to_pin() {
    // **Nothing here is asserted, and that is the point.** This plan measures
    // PRE-fix, so its measure-first discipline cannot catch a wrong POST-fix
    // expectation; a row pinned at a verdict the rule cannot produce lands RED
    // with `19-21` forbidden to edit this file, which stalls the round. `19-16`
    // and `19-18` handled their undeliverable rows exactly this way.
    //
    // **Measured at this file's base commit — all three at exit 2
    // `envelope_assertion_failed`:**
    //
    //   git --attr-source $T push --force origin main
    //     -> "`$T` is the git verb for this command, and the shell may rewrite it
    //         before the program sees it …" — ROUND 5's rule, firing on what the
    //         guard currently reads as the VERB.
    //   git --attr-source *.x push --force origin main
    //     -> "`*.x` is the git verb for this command …" — the same rule.
    //   git --attr-source HEAD {push,--force} origin main
    //     -> "a brace expansion splices words back into this command after the
    //         guard has answered …" — the whole-command brace rule.
    //
    // **Why the post-fix verdict is not derivable.** After `19-21`, `--attr-source`
    // CONSUMES its value, so `$T` and `*.x` are an OPTION'S OPERAND rather than a
    // decision word, and `{push,--force}` sits after a consumed value rather than
    // in the verb slot. Whether round 5's literalness bit still reaches those
    // positions — and therefore whether the identifier stays
    // `envelope_assertion_failed` or becomes `force_push_blocked` — is a `19-21`
    // DESIGN OUTCOME this plan cannot know. **Asserting merely that they are
    // REFUSED is also forbidden**: that would pin an exit code whose mechanism
    // this plan cannot name, which is the same defect one step weaker.
    //
    // `19-21` measures each against the rule and appends the pin.
    record_only(
        "parameter expansion in the value slot",
        "git --attr-source $T push --force origin main",
    );
    record_only(
        "pathname expansion in the value slot",
        "git --attr-source *.x push --force origin main",
    );
    record_only(
        "brace expansion after a consumed value",
        "git --attr-source HEAD {push,--force} origin main",
    );
}

// ===========================================================================
// 7. The COMPOSITION rows — which prove rounds 5 and 6 are LOAD-BEARING for
//    this round rather than superseded by it
// ===========================================================================

#[test]
fn after_19_21_the_composition_rows_are_refused_which_proves_rounds_5_and_6_load_bearing() {
    // **RED at plan 19-20's end.** All five measured at exit 0 today, empty walk.
    //
    // Each reaches THIS round's gap through a mechanism a PREVIOUS round models.
    // A round that treated its predecessors as superseded would break every one
    // of them.

    // -- DELETION THEN CALLEE GRAMMAR: round 6's model deletes the redirection
    //    and hands the callee gap a clean argv.
    refuses(
        "git >/dev/null --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- CALLEE GRAMMAR THEN DELETION: the other order, same class.
    refuses(
        "git --attr-source HEAD >/dev/null push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- **A CONTINUATION INSIDE THE OPTION NAME — the row that proves round 6
    //    is load-bearing HERE.** The backslash-newline sits between `--attr-so`
    //    and `urce`: it must be DELETED before `--attr-source` is even SPELLED,
    //    and only then does this round's rule have an option to recognise. **This
    //    is the row that turns red if round 6's deletion model is ever removed,
    //    whatever this round's rule does.** The `\` and the newline below are
    //    real bytes in the string, not an escape sequence the guard sees.
    refuses(
        "git --attr-so\\\nurce HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- THE NESTED PAYLOAD: reaches the same place through `NestedPayload`'s
    //    re-split of the quoted `bash -lc` argument.
    refuses(
        "bash -lc \"git --attr-source HEAD push --force origin main\"",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // -- BEHIND A SEQUENCE: past the separator arm, so the scan runs on the
    //    SECOND segment.
    refuses(
        "echo hi && git --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

// ===========================================================================
// 8. The four carried-forward MECHANISM pins — GREEN today and after
//
// Re-asserted here over the same PUBLIC functions rather than moved or edited in
// place, because rounds 4, 5 and 6's evidence stays attributable to the round
// that produced it. This round touches the verb-index decision for the first
// time since round 3, so all four are re-stated in its own evidence.
// ===========================================================================

#[test]
fn rule_b_still_reports_a_severed_head_as_not_a_command_position() {
    // A verdict pin cannot replace this. If Rule B stopped firing, the lines
    // below would STILL be refused — by `resolve_program` step 5's prefix rule —
    // and every verdict pin in the suite would stay green while Rule B quietly
    // became dead code.
    for command in [
        "C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin",
        "K=GIT_SSH; env -u ${K}_COMMAND git fetch origin",
        "env -u ${C} git fetch origin",
    ] {
        let segments = policy::split_segments_with_heads(command)
            .unwrap_or_else(|| panic!("`{command}` splits into segments"));
        let last = segments
            .last()
            .unwrap_or_else(|| panic!("`{command}` has at least one segment"));

        assert!(
            !last.head_is_command_position,
            "\n\nRULE B HAS STOPPED BEING LOAD-BEARING.\n\
             \n  command : {command}\
             \n  last    : {:?}\n\
             \nThe correct response is to restore the flush flag for `}}` and `)`, never to \
             delete this test.",
            last.tokens.iter().map(|t| t.text.clone()).collect::<Vec<_>>(),
        );
    }

    // The other half of Rule B's geometry, so the pin cannot be satisfied by
    // marking EVERY segment severed.
    let segments = policy::split_segments_with_heads("{ git status; }")
        .expect("`{ git status; }` splits into segments");
    let git_segment = segments
        .iter()
        .find(|segment| segment.tokens.first().is_some_and(|t| t.text == "git"))
        .expect("one segment begins with `git`");
    assert!(
        git_segment.head_is_command_position,
        "the first command INSIDE a brace group is a command position. If this is false, \
         Rule B has been widened to closers-and-openers and `{{ git status; }}` is refused \
         — a control failing into unusability. Segments: {segments:?}"
    );
}

#[test]
fn round_5s_literalness_bit_is_non_vacuous_and_is_right_about_every_word_of_the_bypass_line() {
    // **The pin that stops `19-21` from closing `T-19-100` by making round 5's
    // bit WRONG about a word it is RIGHT about.**
    //
    // Every word of `git --attr-source HEAD push --force origin main` is written
    // exactly as the shell hands it over — no expansion, no glob, no tilde — and
    // `Token.literal` is correctly `true` for all of them. **The bit is right
    // about the entire bypass line, and the harm is nothing to do with
    // literalness**: it is that the guard picked the wrong word out of a
    // correctly-read argv. A fix that falsified the bit to obtain a refusal would
    // be round 5's inversion quietly becoming dead code with every verdict pin
    // green — which is the failure mode this whole phase is about.
    //
    // Measured against `policy::split_segments_with_heads` at this file's base
    // commit:
    //
    //   git pus? --force origin main
    //     [("git", true), ("pus?", FALSE), ("--force", true), …]
    //   git --attr-source HEAD push --force origin main
    //     [("git", true), ("--attr-source", TRUE), ("HEAD", TRUE), ("push", TRUE), …]
    //
    // **Asserted as "if the token is still there, its bit is still true"**, so a
    // rule that legitimately restructures the segment is not pinned against.
    let unreadable = policy::split_segments_with_heads("git pus? --force origin main")
        .expect("`git pus? --force origin main` splits into segments");
    let unreadable_tokens: Vec<(String, bool)> = unreadable
        .iter()
        .flat_map(|s| s.tokens.iter())
        .map(|t| (t.text.clone(), t.literal))
        .collect();
    assert!(
        unreadable_tokens
            .iter()
            .any(|(text, literal)| text == "pus?" && !*literal),
        "\n\nROUND 5'S LITERALNESS BIT HAS GONE VACUOUS.\n\
         \n`pus?` is a glob in the git VERB slot: the shell resolves it from the working \
         directory, so the word the guard reads is not provably the word the program \
         receives, and `Token.literal` must be FALSE for it. If this is now true, the \
         inversion has stopped discriminating and every refusal it produces is coming from \
         somewhere else.\n\
         \n  tokens: {unreadable_tokens:?}"
    );

    let bypass =
        policy::split_segments_with_heads("git --attr-source HEAD push --force origin main")
            .expect("the bypass line splits into segments");
    let bypass_tokens: Vec<(String, bool)> = bypass
        .iter()
        .flat_map(|s| s.tokens.iter())
        .map(|t| (t.text.clone(), t.literal))
        .collect();

    for word in ["--attr-source", "HEAD", "push"] {
        for (text, literal) in &bypass_tokens {
            if text == word {
                assert!(
                    *literal,
                    "\n\nTHE INVERSION WAS FALSIFIED INSTEAD OF THE VERB INDEX BEING \
                     CORRECTED.\n\
                     \n`{word}` is a fully LITERAL word and `Token.literal` is correctly \
                     `true` for it. The harm on this line is not that a word is unreadable; \
                     it is that the guard chose the WRONG WORD as the verb out of a \
                     correctly-read argv. If this bit is now `false`, `19-21` closed \
                     `T-19-100` by making round 5's bit WRONG about a word it was right \
                     about — which is the inversion becoming dead code with every verdict \
                     pin green.\n\
                     \n**The correct shape of the fix is to correct the verb INDEX.**\n\
                     \n  tokens: {bypass_tokens:?}"
                );
            }
        }
    }

    // Non-vacuity: the three words really are present, so the loop above is not
    // passing over an empty match set.
    for word in ["--attr-source", "HEAD", "push"] {
        assert!(
            bypass_tokens.iter().any(|(text, _)| text == word),
            "`{word}` must be present as a token, or the assertion above certifies nothing. \
             tokens: {bypass_tokens:?}"
        );
    }
}

#[test]
fn round_6s_deletion_model_is_non_dead_and_this_round_must_not_remove_it() {
    // **The pin that turns red if `19-21` regresses round 6.** Round 6's model
    // deletes a redirection from the argv the guard classifies; three of section
    // 7's rows depend on it having done so before this round's rule sees an
    // option at all.
    //
    // The refused half: `git >/dev/null push --force origin main` — the
    // redirection is deleted, `push` is the verb, `--force` is denied.
    refuses(
        "git >/dev/null push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // The OVER-DELETION control: `x2>/tmp/o` is a REAL argv word (bash treats
    // `x2` as an ordinary word, not an IO_NUMBER), so it must NOT be deleted —
    // and with it present, `push` is not in the verb slot. This is round 6's
    // `ls {git,svn}-repo` and it must stay PERMITTED.
    permits("git x2>/tmp/o push --force origin main");
}

#[test]
fn a_redirection_operator_is_not_a_separator_and_this_round_must_not_make_it_one() {
    // **The mechanical proof that no round in this phase took the route
    // `SEPARATORS`' own doc forbids.** That doc records the decision — a
    // redirection does not start a new command, so treating it as a separator
    // would HIDE the command it redirects — and audit 5 judged the reasoning
    // RIGHT and only its unexamined consequence wrong.
    //
    // `separators_are_named_in_one_list_that_the_predicate_reads` inside
    // `src/envelope/policy.rs` asserts the same fact and must stay GREEN and
    // UNMODIFIED through `19-21`. It is asserted here too so the requirement is
    // visible in this round's own evidence.
    assert!(
        !policy::is_separator(">"),
        "`>` must NOT become a separator. Splitting there would hide the command being \
         redirected, which is worse than the defect it would close."
    );
    assert!(!policy::is_separator("<"), "`<` must NOT become a separator");
    assert!(
        !policy::is_separator(">>"),
        "`>>` must NOT become a separator"
    );

    // Non-vacuity: the predicate is not constant-false.
    assert!(
        policy::is_separator("&"),
        "`&` IS a separator — without this the three assertions above could all pass \
         because `is_separator` answers `false` for everything"
    );
    assert!(policy::is_separator("&&"), "`&&` IS a separator");
}
