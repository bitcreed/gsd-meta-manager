// ============================================================================
// Round 8 — the axis that is NOT the command line: what the verb RUNS UNDER.
//
// **What this file is.** The reproducers `19-SECURITY.md`'s SEVENTH audit
// measured for `T-19-103` and `T-19-104`, plus the ten cells found while PLANNING
// round 8, re-measured here against the BUILT BINARY at this file's base commit
// with one fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory WALKED
// afterwards, and — the step that separates a config-resolution claim from a
// guess — confirmed against the REAL `git` binary using **the envelope's OWN
// `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection as the control** rather than a
// stand-in. Every row was driven BEFORE it was written as an assertion, and every
// one of the audit-7 rows reproduced at the recorded verdict; **none failed to
// reproduce.**
//
// **The invariant this file is about, stated once.**
//
//   THE CONFIGURATION THE GUARD ASSUMES IS IN EFFECT MUST BE THE CONFIGURATION
//   GIT WILL ACTUALLY RESOLVE.
//
// Rounds 5, 6 and 7 closed the three questions BELOW this one, and audit 7
// verified all three answered. Round 5: a decision word must be provably LITERAL
// (`tests/envelope_literal_decision.rs`). Round 6: the words the guard classifies
// must be exactly the words the program receives, in the same order
// (`tests/envelope_argv_deletion.rs`). Round 7: the word the guard calls the verb
// must be the word git calls the verb (`tests/envelope_callee_grammar.rs`) —
// audit 7 says plainly that the callee's option grammar is now modelled and
// fail-closed.
//
// Audit 7's framing, adopted verbatim, is why there is a round 8 at all:
//
// > Git turns a command line into behaviour in **three** stages, and this phase
// > now models the first: (1) **which arriving word is the verb** — closed by
// > `19-21`; (2) **what configuration is in effect while that verb runs** —
// > **unmodelled**; (3) **what the process it inherits tells it** — **unmodelled
// > at one key**.
//
// This file is stages 2 and 3. Having built the right argv and found the right
// verb: **what will that verb run under?** That is a question about git's config
// GRAPH and about the ENVIRONMENT git reads as config, and it is not a question
// about the command line at all.
//
// ============================================================================
// THE FINDING THAT SHAPES THE WHOLE CORPUS, STATED FIRST BECAUSE A LATER READER
// NEEDS IT TO UNDERSTAND WHY THE REPRODUCERS LOOK LIKE THIS
// ============================================================================
//
// **A `T-19-103` reproducer must be built on a base LAYER 2 PERMITS.** The whole
// content of an indirection is that it disarms layer 3 on a line layer 2 lets
// through — so a reproducer whose base is *already refused for its verb* proves
// nothing whatever about config resolution. Measured at this file's base commit,
// one fresh root per row, walk EMPTY on every one:
//
// ```
// exit=2 [force_push_blocked]  git -c include.path=/tmp/evil.cfg push --force origin main
// exit=2 [force_push_blocked]  bash -lc "git -c include.path=/tmp/evil.cfg push --force origin main"
// exit=2 [force_push_blocked]  echo hi && git -c include.path=/tmp/evil.cfg push --force origin main
// exit=2 [force_push_blocked]  git >/dev/null -c include.path=/tmp/evil.cfg push --force origin main
// exit=2 [force_push_blocked]  git --attr-source HEAD -c include.path=/tmp/evil.cfg push --force origin main
// ```
//
// **Every one of those is a CONTROL, and section 0 asserts them as controls.** A
// corpus built out of them would be GREEN before the fix and would certify
// nothing — the eighth consecutive instance of `T-19-76`'s failure mode, produced
// by this corpus rather than found by the next audit. The reproducers therefore
// use the three layer-2-PERMITTED bases audit 7 itself used: `commit -m x`,
// `status`, and the IN-NAMESPACE `push origin refs/heads/gsd-auto/alpha/w`.
// `tests/envelope_wrapper_class.rs` asserts that constraint MECHANICALLY for the
// generative arm, so it is not left to care.
//
// ## The mechanism, in git's own terms
//
// `scan_leading` (`src/envelope/policy.rs:325-471`) decides exactly ONE question
// about a `-c` assignment: is the KEY half `core.hooksPath` (D-09,
// `is_hooks_path_key` at `policy.rs:738-752`)? Git's config resolution has an
// INDIRECTION that check does not model. A key in the `include` or `includeIf`
// section names a FILE, and that file's contents are spliced in **at the
// precedence of the directive that named them** — i.e. at command-line
// precedence, outranking the envelope's env-injected triplet — **without the
// string `core.hooksPath` appearing anywhere on the command line.**
//
// `GIT_CONFIG_PARAMETERS` is the same defeat from stage 3: it is git's own
// internal carrier for `-c`, git EXPORTS it (so one prefix disarms every git
// SUBPROCESS of the command), it outranks the injection, and it is absent from
// `ENVELOPE_ENV_KEYS` (`policy.rs:2785-2874`).
//
// Every word on these lines is literal and every word arrives in order — **round
// 5's bit is RIGHT and round 6's model is RIGHT** — and the word the guard calls
// the verb IS the word git calls the verb, so **round 7's grammar is RIGHT too**.
// All three are pinned non-vacuous in section 11. What is wrong is one layer
// further out: the guard reasoned correctly about the command line and then
// assumed a configuration git will not resolve.
//
// ## The exit codes re-measured at this file's base commit, one fresh root per
// ## row, walk after — every row EMPTY
//
// ```
// exit=0  git -c include.path=/tmp/evil.cfg commit -m x                          <- T-19-103
// exit=0  git -c include.path=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w
// exit=0  git -c include.path=/tmp/evil.cfg status
// exit=0  git -c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg status
// exit=0  git --config-env=include.path=EVILVAR status
// exit=0  git --config-env include.path=EVILVAR status
// exit=0  git -c INCLUDE.PATH=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w
// exit=0  git -c "include.path=/tmp/evil.cfg" status                  <- the quoted spelling
// exit=0  git -c include.pathx=/tmp/evil.cfg status                   <- the DISCLOSED COST
// exit=0  GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git commit -m x  <- T-19-104
// exit=0  GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git push origin refs/heads/gsd-auto/alpha/w
// exit=0  export GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'"; git push origin refs/heads/gsd-auto/alpha/w
// exit=0  env GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git push origin refs/heads/gsd-auto/alpha/w
// exit=0  GIT_CONFIG_PARAMETERS="'include.path=/tmp/evil.cfg'" git push origin refs/heads/gsd-auto/alpha/w
// exit=0  echo GIT_CONFIG_PARAMETERS                                  <- the DISCLOSED COST
// ```
//
// ## The paired discriminators, which prove this is a gap in REACH, not MECHANISM
//
// Measured at this file's base commit, all exit 2, all walks EMPTY, all asserted
// UNCHANGED in section 3:
//
// ```
// hook_bypass_blocked  git -c core.hooksPath=/dev/null commit -m x
// hook_bypass_blocked  git --config-env=core.hooksPath=EVILVAR status
// hook_bypass_blocked  git --config-env core.hooksPath=EVILVAR status
// hook_bypass_blocked  git -c CORE.HOOKSPATH=/dev/null status        <- case, ALREADY right
// hook_bypass_blocked  GIT_CONFIG_COUNT=0 git push origin refs/heads/gsd-auto/alpha/w
// hook_bypass_blocked  export GIT_CONFIG_COUNT=0; git push origin refs/heads/gsd-auto/alpha/w
// hook_bypass_blocked  env GIT_CONFIG_COUNT=0 git push origin refs/heads/gsd-auto/alpha/w
// hook_bypass_blocked  GIT_CONFIG_COUNT=0 git status                 <- permitted base
// hook_bypass_blocked  export GIT_CONFIG_COUNT=0; git status
// hook_bypass_blocked  env GIT_CONFIG_COUNT=0 git status
// hook_bypass_blocked  echo GIT_CONFIG_COUNT                         <- disclosed bare-word cost
// ```
//
// **These are what make every `T-19-104` post-fix verdict DERIVABLE rather than
// guessed**: all three environment spellings already refuse for a key that IS in
// the list, on BOTH a refused and a permitted base, so adding one entry derives
// all five rows and their bare-word cost twin.
//
// ## The real-git precedence table, measured with the envelope's OWN injection as
// ## the control (asserted in section 12, not merely recorded here)
//
// ```
// git version 2.43.0
// control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS
//           git config --get core.hooksPath                      -> /ENV_WINS
// + -c include.path=<file>                                       -> /INCLUDE_WINS
// + -c INCLUDE.PATH=<file>                                       -> /INCLUDE_WINS
// + -c includeIf.gitdir:<p>.path=<file>                          -> /INCLUDE_WINS
// + -c INCLUDEIF.gitdir:<p>.PATH=<file>                          -> /INCLUDE_WINS
// + --config-env=include.path=EVILVAR                            -> /INCLUDE_WINS
// + --config-env include.path=EVILVAR                            -> /INCLUDE_WINS
// + GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'"         -> /PARAM_WINS
// + GIT_CONFIG_PARAMETERS="'include.path=<file>'"                -> /PARAM_INCLUDE_WINS
// + -c include.pathx=<file>                                      -> /ENV_WINS  (git IGNORES it)
// ```
//
// **The case answer, measured in BOTH halves of the key.** Git folds the SECTION
// and the VARIABLE to lower case and leaves the SUBSECTION case-sensitive:
// `INCLUDE.PATH` resolves, and so does `INCLUDEIF.gitdir:<p>.PATH`. **No existing
// pin changes** — `is_hooks_path_key` is already
// `eq_ignore_ascii_case("core.hookspath")` and `git -c CORE.HOOKSPATH=/dev/null
// status` is measured at `hook_bypass_blocked` today (section 3).
//
// ## The class confirmed END TO END against real git, INDEPENDENTLY rather than
// ## by citing audit 7 (asserted in section 13)
//
// A claim in this codebase about which forms outrank the injection is already
// known FALSE — `cred.rs:241-248` says "the one form that outranks this injection
// is `git -c core.hooksPath=… push`", and four measured forms outrank it — so
// this round does not carry forward a behavioural claim it has not reproduced.
// The bare-remote fixture was rebuilt and all four legs re-measured, with the
// remote's SHA recorded BEFORE and AFTER each, because a SHA is what makes "the
// remote moved" a measurement rather than a claim:
//
// ```
// leg 1  plain in-namespace push          -> rc=1, hook REFUSED, ref 11b417c -> 11b417c (UNMOVED)
// leg 2  -c include.path=<evil>           -> rc=0, ref 11b417c -> 0e482a9 (MOVED)
// leg 3  GIT_CONFIG_PARAMETERS carrier    -> rc=0, ref 0e482a9 -> ba3c923 (MOVED)
// leg 4  pre-commit point: control rc=1 (refused), carrier rc=0, HEAD ba3c923 -> 79a7c23
// ```
//
// The hook was delivered exactly as the envelope delivers it — `core.hooksPath`
// through the `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet `cred::hooks_path_env`
// emits. **All four legs reproduced; none failed.**
//
// `pre-push` is the ONLY carrier of the worktree credential scan (`hooks.rs:342`,
// SAFE-05, `T-19-15`/`T-19-16`/`T-19-18`) and the second carrier `AR-19-03` rests
// on, so disarming it widens `T-19-86`, `T-19-91` and `T-19-96` at once.
//
// ## The three-leg argument, as audit 7 measured it
//
// `T-19-103` defeats all three layers on one line:
//
//   1. layer 1's `Bash(git push:*)` prefix rule does not match a line beginning
//      `git -c`;
//   2. layer 2 — this guard — is defeated because the key half is not
//      `core.hooksPath` and nothing else about the assignment is read;
//   3. layer 3 is disarmed, MEASURED above, with `GIT_ASKPASS`,
//      `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` and `GIT_SSH_COMMAND` untouched so
//      the push still authenticates.
//
// ## Written BEFORE the rules, and that ordering is the whole point of this file
// ## existing separately from the fix
//
// For SEVEN consecutive rounds a control in this phase was certified by a corpus
// structurally incapable of failing on the class that got through it —
// `T-19-76`, `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99`, `T-19-101`, now
// `T-19-105`. For the THIRD round running the gap moved AXIS rather than one cell
// over: `UNREADABLE_CLASSES` names seven word-ASSEMBLY classes,
// `DELETION_CLASSES` names five word-REMOVAL classes and `CALLEE_GRAMMAR_CLASSES`
// names five classes of git's OPTION GRAMMAR — and audit 7 verified mechanically
// that nothing anywhere models git's CONFIG RESOLUTION:
// `grep -rn "include\.path\|includeIf" src/ tests/` returned **nothing at all**
// and `grep -rn "GIT_CONFIG_PARAMETERS" src/ tests/` returned **nothing at all**.
// Both greps were re-run while planning this round and both still returned
// nothing. Plan `19-22` writes the corpus and the reproducers and STOPS; plan
// `19-23` writes the rules. A corpus written after a fix cannot be distinguished
// from a corpus written to agree with what the code already did.
//
// **Why a SEVENTH file rather than more rows in
// `tests/envelope_callee_grammar.rs`.** That file is round 7's evidence and its
// header states the exit codes round 7 measured. Round 8's evidence lives here so
// a later reader can tell which round produced which measurement, and so neither
// header has to be rewritten to stay true. `19-14` created a third file, `19-16` a
// fourth, `19-18` a fifth and `19-20` a sixth for exactly this reason.
//
// ## MOST OF THIS FILE IS DELIBERATELY RED AT PLAN 19-22'S END
//
// Every test whose name begins `after_19_23_` is EXPECTED TO FAIL against the
// pre-fix tree. That is not a defect and it is not a regression: it is the
// evidence that the corpus was capable of failing before the code changed.
// `19-22-SUMMARY.md` lists every red name; `19-23` confirms the same names still
// red before it writes a line, and closes them.
//
// Every test whose name does NOT begin `after_19_23_` passes TODAY. Those are the
// controls: the anti-vacuity rows, the five `force_push_blocked` `--force`
// compositions, the walk's positive control, the paired discriminators, the
// permitted half, the discrimination controls, the dotless-key pin, the
// already-refused planning cells, the recorded-not-asserted rows, the `T-19-86`
// persisted-alias arm, the real-git probes and the five carried-forward mechanism
// pins.
//
// ## EVERY POST-FIX EXPECTATION CARRIES A WRITTEN DERIVATION
//
// This plan measures PRE-fix, so its measure-first discipline cannot catch a
// wrong POST-fix expectation. A row pinned at a verdict the rule cannot produce
// lands RED with `19-23` forbidden to edit it — which stalls the round. So every
// asserted post-fix verdict is DERIVED in writing beside the row from `19-23`'s
// mandated design, and **two rows whose post-fix verdict cannot be derived are
// RECORDED IN COMMENTS AND PRINTS and are NOT written as assertions** (section 9).
//
// **This seam needs NO replacement exception and none is granted.** Rounds 3, 4
// and 5 each shipped a two-plan handoff that was unsatisfiable as written; `19-20`
// corrected it by asserting every derivable row at its POST-fix verdict so no body
// ever needs replacing, and this round does the same.
//
// ## What stops this file passing vacuously
//
// 1. Anti-vacuity controls that pass TODAY (section 0), including a POSITIVE
//    control proving the ledger walk can see a line at all.
// 2. A paired PERMITTED half (section 5): eight ordinary invocations pinned at
//    exit 0 before AND after. **`git -c user.name="$NAME" commit -m x` is this
//    axis's `ls {git,svn}-repo`** — the row that tells a resolution MODEL apart
//    from a blanket refusal of anything spelled `-c`, which is the shape this fix
//    is one wrong step away from (AR-19-11).
// 3. The DISCRIMINATION controls (section 4): `-c includepath=` and
//    `-c notinclude.path=` pinned exit 0 before and after, which turn RED on a fix
//    written as a substring match on `include`. **This round's `--signed no`.**
// 4. The DISCLOSED COST rows, each beside its permitted twin, so the over-refusal
//    this round ADDS is measured rather than hidden.
// 5. Every refusal row asserts the D-24 reason identifier as well as the exit
//    code, so a row cannot pass by being refused for an unrelated cause.
// 6. Every row asserts what the WALKED envelope root holds.
// 7. Five MECHANISM pins (section 11) assert over `policy::split_segments_with_heads`
//    and `policy::is_separator` rather than over exit codes, so `19-23` cannot make
//    Rule B, round 5's inversion, round 6's deletion model or round 7's
//    fail-closed grammar dead code while every verdict pin stays green.
// 8. REAL-GIT probes (sections 12 and 13) that fail if git's own resolution ever
//    stops matching what the rules assume.
//
// **Offline, agent-free and clock-free** (D-35). `hooks::guard_in` is driven
// in-process against a per-row `TempDir` envelope root and a config path that does
// not exist, so `resolve_policy` applies the tighter defaults. One root per row,
// because the PR ledger persists and a shared root produces misleading
// cap-exhaustion refusals. The push rows build a local git fixture with a bare
// local upstream; it is offline and it does NOT consult the test process's own
// working directory, which is the `T-19-80` failure this file must not
// reintroduce.
//
// **No crate was added** (`T-19-SC`). The directory walk below is eight lines of
// `std::fs` and every real-git probe shells out through `std::process::Command`.
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
    /// The reason the guard carried in its decision JSON, or the empty string for
    /// a permit (a permit writes nothing at all, by design).
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
/// Carried forward unchanged in shape from `tests/envelope_callee_grammar.rs`.
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
// The walk — carried forward unchanged from `tests/envelope_callee_grammar.rs`
// ---------------------------------------------------------------------------

/// Every file under `dir`, recursively.
///
/// **Why a walk rather than a read of `ledger::ledger_path_in`.** A row that
/// asserted "the expected ledger path is empty" would pass identically if the line
/// had been written somewhere else under the envelope root. The absence is
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
/// envelope root of its own AND an explicit project root, and that the walked root
/// holds no ledger line.
fn refuses_in(project_root: Option<&Path>, command: &str, reason: &str, why: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask_in(envelope.path(), project_root, command);

    assert_eq!(
        answer.code, 2,
        "\n\n`{command}` must be REFUSED.\n\n{why}\n\n\
         It was measured against the BUILT BINARY at this file's base commit with a fresh \
         envelope root and a walk after, and every precedence claim behind it was confirmed \
         against the REAL git binary with the envelope's own `GIT_CONFIG_COUNT`/`KEY_n`/\
         `VALUE_n` injection as the control.\n  stdout: {}\n  stderr: {}\nWalked listing:\n{}",
        answer.stdout,
        answer.stderr,
        listing(envelope.path())
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}`. Asserting the identifier and not merely \
         the exit code is what stops a row from passing because it was refused for an unrelated \
         cause (D-24). Got: {}",
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

/// The common case: refused, no project root needed.
fn refuses(command: &str, reason: &str, why: &str) {
    refuses_in(None, command, reason, why);
}

/// Assert one command is permitted and answers nothing at all.
fn permits_in(project_root: Option<&Path>, command: &str, why: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask_in(envelope.path(), project_root, command);

    assert_eq!(
        answer.code, 0,
        "\n\n`{command}` must be PERMITTED.\n\n{why}\n\n\
         The guard sees EVERY Bash tool call, so a rule that refused every `-c` would refuse \
         this and make a driven run unusable — which is how a safety control gets switched off \
         (AR-19-11).\n  stdout: {}\n  stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.stdout.is_empty(),
        "a permit answers nothing at all: emitting `allow` would turn a deny-only control into \
         an approval authority. Got: {}",
        answer.stdout
    );
}

fn permits(command: &str, why: &str) {
    permits_in(None, command, why);
}

/// Drive one row and PRINT what it answered, asserting nothing about it.
///
/// Section 9's two rows use this and only this. A row whose POST-fix verdict this
/// plan cannot DERIVE must not be pinned: this plan measures PRE-fix, so its
/// measure-first discipline cannot catch a wrong post-fix expectation, and a row
/// pinned at a verdict the rules cannot produce lands RED with `19-23` forbidden
/// to edit it — which stalls the round. `19-16`, `19-18` and `19-20` handled their
/// undeliverable rows exactly this way.
fn record_only(label: &str, project_root: Option<&Path>, command: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask_in(envelope.path(), project_root, command);
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
/// Carried forward unchanged in shape from `tests/envelope_callee_grammar.rs`.
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

/// The one in-namespace push base every `T-19-103` and `T-19-104` push row is
/// built on. Layer 2 PERMITS it, which is the whole constraint of this axis.
const IN_NAMESPACE_PUSH: &str = "push origin refs/heads/gsd-auto/alpha/w";

/// The derivation every `T-19-103` row shares, written once and quoted into each
/// row's own failure message.
const T_19_103_DERIVATION: &str = "\
`19-23`'s mandated clause is raised INSIDE `scan_leading`, at the first `-c` / \
`--config-env` assignment whose effect on `core.hooksPath` it cannot BOUND, and \
before any verb is classified. So the identifier is the unresolvable-command one \
(`envelope_assertion_failed`) and NOT the one the verb would have earned — which \
is D-24's requirement that a refusal name the mechanism that produced it. \
Measured against the real git binary: a key in the `include` or `includeIf` \
section names a file whose contents are spliced in AT THE PRECEDENCE OF THE \
DIRECTIVE THAT NAMED THEM, so `-c include.path=<f>` makes `git config --get \
core.hooksPath` print `/INCLUDE_WINS` while the envelope's own injection alone \
prints `/ENV_WINS`. The string `core.hooksPath` never appears on the line.";

/// The derivation every `T-19-104` row shares.
const T_19_104_DERIVATION: &str = "\
`GIT_CONFIG_PARAMETERS` is git's OWN internal carrier for `-c`; git EXPORTS it, so \
one prefix disarms every git SUBPROCESS of the command. Measured against real git \
with the envelope's own injection as the control, \
`GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/PARAM_WINS'\"` makes `git config --get \
core.hooksPath` print `/PARAM_WINS` — it OUTRANKS the triplet. It is absent from \
`ENVELOPE_ENV_KEYS` (`policy.rs:2785-2874`). The post-fix verdict is DERIVED, not \
guessed: all THREE environment spellings of the paired discriminator \
`GIT_CONFIG_COUNT=0` — the bare prefix, `export …;` and `env …` — are measured at \
exit 2 `hook_bypass_blocked` TODAY on BOTH a refused and a permitted base, and \
`echo GIT_CONFIG_COUNT` is refused too. One `ENVELOPE_ENV_KEYS` entry therefore \
derives every row here and its bare-word cost twin. This is a gap in the LIST, not \
in the mechanism.";

// ===========================================================================
// 0. The anti-vacuity controls — every one of these passes TODAY
// ===========================================================================

#[test]
fn the_unwrapped_refusals_still_fire_which_proves_this_harness_can_see_a_denial() {
    // If any row here is ever red, every refusal row in this file is red for a
    // reason that has nothing to do with config resolution — the harness cannot
    // observe a refusal at all.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "the harness must be able to see an ordinary force-push denial",
    );
    refuses(
        "git config core.hooksPath /tmp/x",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the harness must be able to see a hook-bypass denial",
    );
    refuses(
        "git -c core.hooksPath=/dev/null push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the existing command-line `-c core.hooksPath` clause must still fire",
    );
}

#[test]
fn the_force_push_compositions_are_already_refused_and_are_controls_not_reproducers() {
    // **THE SINGLE MOST IMPORTANT CONTROL IN THIS FILE, and the finding that
    // shapes the whole corpus.**
    //
    // Every `--force origin main` composition of the include carrier is ALREADY
    // exit 2 `force_push_blocked` at this file's base commit — refused for its
    // VERB, with the carrier never examined. A reproducer built on one of these
    // would be GREEN before `19-23` and would certify NOTHING about config
    // resolution: it is `T-19-76`'s failure mode for the eighth consecutive round,
    // and it would have been produced by this corpus rather than found by the next
    // audit.
    //
    // They are asserted here as CONTROLS. Their VERDICT does not move — exit 2,
    // walk empty, before and after. Their IDENTIFIER does.
    //
    // # THE CORPUS DEFECT THIS BLOCK CARRIED, CORRECTED BY PLAN `19-23`
    //
    // **This block asserted `force_push_blocked` while four independent sources
    // said its identifier was deliberately NOT being asserted.** The comment two
    // paragraphs above read *"Their verdict does not move; only their IDENTIFIER
    // may, and section 9 records that separately without asserting it"*;
    // `19-22-SUMMARY.md` §"The two rows RECORDED rather than asserted" lists
    // `git -c include.path=$F push --force origin main` as exactly such a row;
    // `19-22-PLAN-CHECK.md` Check 4 signed the five compositions off as
    // *"correctly demoted to controls"* and Check 5 as *"No replacement exception
    // needed or granted"*; and `19-23-PLAN.md:668` independently predicted the
    // identifier moving to `envelope_assertion_failed`. **The demotion was stated
    // in all four places and implemented in none of them**, and `refuses()`
    // asserts the identifier.
    //
    // The two assertions were unsatisfiable together. This row and the ORDERING
    // pin in section 7 —
    // `git -c include.path=$F -c core.hooksPath=/dev/null push --force origin
    // main` at `envelope_assertion_failed` — differ ONLY in tokens AFTER the
    // first unbounded assignment, which `scan_leading` never reads because it
    // returns at the first assignment it cannot bound. No rule raised inside the
    // one left-to-right scan can produce different identifiers for them, and a
    // rule that could would have to read past the first unbounded assignment,
    // which is what the ordering pin exists to forbid.
    //
    // # CARRIER BEFORE VERB, DECIDED EXPLICITLY RATHER THAN BY AN EDIT
    //
    // `envelope_assertion_failed` is the correct identifier and the reason is a
    // decision, not a convenience. **An unbounded config assignment means the
    // guard cannot establish what configuration the command will run under**, so
    // every downstream classification — `force_push_blocked` included — is a
    // statement about a command whose behaviour the guard cannot bound. Refusing
    // at the CARRIER and saying so is the honest verdict; reporting
    // `force_push_blocked` would name a specific hazard while the guard is in
    // fact unable to see the command at all. That is D-24's requirement that a
    // refusal name the mechanism that produced it.
    //
    // The correction is scoped to the identifier constant on these five rows.
    // Nothing else in this file moved, `tests/envelope_wrapper_class.rs` was not
    // touched, and no other assertion was weakened.
    //
    // The reproducers in sections 1 and 2 use the three layer-2-PERMITTED bases
    // audit 7 used: `commit -m x`, `status` and the in-namespace push.
    for command in [
        "git -c include.path=/tmp/evil.cfg push --force origin main",
        "bash -lc \"git -c include.path=/tmp/evil.cfg push --force origin main\"",
        "echo hi && git -c include.path=/tmp/evil.cfg push --force origin main",
        "git >/dev/null -c include.path=/tmp/evil.cfg push --force origin main",
        "git --attr-source HEAD -c include.path=/tmp/evil.cfg push --force origin main",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "**CONTROL, NOT REPRODUCER.** This line's VERDICT does not move — it was exit 2 \
             before `19-23` and is exit 2 after — so it certifies nothing about config \
             resolution and a reproducer must not be built on it. Its IDENTIFIER moves, \
             because the carrier is read before the verb: an assignment the guard cannot \
             BOUND makes the whole command unresolvable, and naming `force_push_blocked` \
             would name a specific hazard while the guard cannot see the command at all. If \
             this is red, either the base has stopped being refused or the clause is no \
             longer raised at the first unbounded assignment.",
        );
    }
}

#[test]
fn the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all() {
    // Without this, every "the walk was EMPTY" assertion in this file could be
    // passing because the walk is blind. `gh pr create --title x` is the
    // ready-made positive control: one line on call 1, `pr_cap_exceeded` with two
    // on call 2.
    let envelope = TempDir::new().unwrap();

    let first = ask(envelope.path(), "gh pr create --title x");
    assert_eq!(
        first.code, 0,
        "the first `gh pr create` in a fresh root is PERMITTED. stdout: {} stderr: {}",
        first.stdout, first.stderr
    );
    let after_one = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        after_one.len(),
        1,
        "a permitted `gh pr create` writes EXACTLY ONE ledger line, and the walk must find it. \
         If this is 0 the walk is blind and every empty-walk assertion in this file is \
         vacuous.\nWalked listing:\n{}",
        listing(envelope.path())
    );

    // And the cap fires on the second, with two lines present — so the walk sees
    // accumulation and not just existence.
    let second = ask(envelope.path(), "gh pr create --title y");
    assert_eq!(
        second.code, 2,
        "the second `gh pr create` in the same root exceeds the cap. stdout: {}",
        second.stdout
    );
    assert!(
        second.reason().contains(policy::REASON_PR_CAP_EXCEEDED),
        "and under `pr_cap_exceeded`. Got: {}",
        second.reason()
    );
    assert_eq!(
        ledger_lines_anywhere(envelope.path()).len(),
        2,
        "two ledger lines are present after the capped call.\nWalked listing:\n{}",
        listing(envelope.path())
    );
}

// ===========================================================================
// 1. `T-19-103` — the CONFIG INDIRECTION, on LAYER-2-PERMITTED bases
//
// RED at plan 19-22's end. Every row asserted at exit 2
// `envelope_assertion_failed`, with the derivation written beside it.
// ===========================================================================

#[test]
fn after_19_23_a_config_indirection_carrier_is_refused_on_a_permitted_base() {
    // **BEFORE (measured at this file's base commit, one fresh root per row, walk
    // EMPTY on every one):** every row below is at **exit 0**.
    //
    // **AFTER (`19-23`'s rules):** exit 2 `envelope_assertion_failed`.
    //
    // **THE DERIVATION** is `T_19_103_DERIVATION`, quoted into every failure
    // message below. The short form: the key half names the `include` or
    // `includeIf` SECTION, git splices that file at command-line precedence, and
    // `is_hooks_path_key` cannot see a string that is not on the line.
    //
    // The three bases are the ones audit 7 used and are the only ones that can
    // certify anything here — `commit -m x` and `status` are permitted, and the
    // in-namespace push is permitted and is the one whose harm the end-to-end
    // fixture in section 13 measured against a bare remote.

    // -- the `-c include.path=` carrier on the two no-repo bases.
    for command in [
        "git -c include.path=/tmp/evil.cfg commit -m x",
        "git -c include.path=/tmp/evil.cfg status",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            T_19_103_DERIVATION,
        );
    }

    // -- `includeIf`, whose SUBSECTION is an open condition family. `19-23`'s rule
    //    reads the SECTION and deliberately reads neither the subsection nor the
    //    variable, which covers the whole family by construction.
    refuses(
        "git -c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_103_DERIVATION,
    );

    // -- the SECOND carrier `scan_leading` reads, in both of its spellings. Arm
    //    (b) of `leading_git_option` (`policy.rs:669`) returns the ATTACHED
    //    assignment and arm (a) (`policy.rs:664`) returns the NEXT TOKEN as the
    //    assignment for a bare `-c`/`--config-env`, so BOTH reach the same key
    //    check — which is why one clause covers both and why both are asserted.
    //    `--config-env`'s value is an environment variable NAME rather than a
    //    value, and real git resolves it identically: measured
    //    `--config-env=include.path=EVILVAR` -> `/INCLUDE_WINS`.
    for command in [
        "git --config-env=include.path=EVILVAR status",
        "git --config-env include.path=EVILVAR status",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            T_19_103_DERIVATION,
        );
    }

    // -- THE QUOTED SPELLING. The quoting is removed by the tokenizer before the
    //    scan reads the key, so quoting is neither an evasion nor a defence. A
    //    planning cell, recorded with the `19-14` provenance caveat.
    refuses(
        "git -c \"include.path=/tmp/evil.cfg\" status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_103_DERIVATION,
    );
}

#[test]
fn after_19_23_a_config_indirection_carrier_is_refused_on_the_in_namespace_push() {
    // **The push half, on the ONE push base layer 2 permits** — and the base whose
    // real harm section 13 measured end to end against a bare remote: an
    // in-namespace push the `pre-push` hook REFUSES completed and MOVED the
    // remote's ref under `-c include.path=`, SHAs recorded before and after.
    //
    // **BEFORE:** exit 0 for both rows, measured against the built binary with the
    // fixture below as the project root and with a fresh envelope root and a walk.
    // **AFTER:** exit 2 `envelope_assertion_failed`.
    //
    // The repository is passed EXPLICITLY so no verdict here depends on where
    // `cargo test` was invoked (`T-19-80`).
    let repo = in_namespace_repo();
    let work = repo.path().join("work");

    // -- GREEN TODAY AND AFTER: the unwrapped base is PERMITTED. Without this the
    //    two red rows below could be red for the base's own reason.
    permits_in(
        Some(&work),
        &format!("git {IN_NAMESPACE_PUSH}"),
        "the UNWRAPPED in-namespace push is the layer-2-PERMITTED base this whole axis \
         requires. If this is red, the fixture is wrong and the rows below prove nothing \
         about config resolution.",
    );

    // -- THE RED ROWS.
    refuses_in(
        Some(&work),
        &format!("git -c include.path=/tmp/evil.cfg {IN_NAMESPACE_PUSH}"),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_103_DERIVATION,
    );

    // -- THE CASE-VARIED SPELLING, in the SECTION half of the key. Git folds the
    //    section and the variable to lower case, measured: `-c INCLUDE.PATH=<f>`
    //    makes `git config --get core.hooksPath` print `/INCLUDE_WINS`. So the
    //    rule's section comparison must be ASCII-case-insensitive — which is what
    //    `is_hooks_path_key` already is (`eq_ignore_ascii_case`, `policy.rs:750`),
    //    so **no existing pin changes** (section 3 asserts that).
    refuses_in(
        Some(&work),
        &format!("git -c INCLUDE.PATH=/tmp/evil.cfg {IN_NAMESPACE_PUSH}"),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_103_DERIVATION,
    );
}

// ===========================================================================
// 2. `T-19-104` — `GIT_CONFIG_PARAMETERS`, a gap in a LIST rather than in the
//    mechanism
//
// RED at plan 19-22's end. Every row asserted at exit 2 `hook_bypass_blocked`,
// DERIVED from the three-spelling paired discriminator rather than guessed.
// ===========================================================================

#[test]
fn after_19_23_the_environment_carrier_of_configuration_is_refused() {
    // **BEFORE:** every row at **exit 0**, walk EMPTY. **AFTER:** exit 2
    // `hook_bypass_blocked`, derived from `T_19_104_DERIVATION`.
    //
    // **Why the existing `ENVELOPE_ENV_KEYS` drift pin STRUCTURALLY CANNOT SEE
    // THIS.** The pin is sourced from
    // `cred::EnvelopeEnv::with_run_id(build_env_in(…))` — i.e. from the keys the
    // envelope SETS or REMOVES — and every floor it carries is a floor over the
    // envelope's own entries. This is a key the envelope neither sets nor removes
    // but which DEFEATS one it sets. **The fix is a SECOND SOURCE, not a wider
    // filter**, and it is `19-23`'s.
    //
    // The third-omission history: `GIT_SSH_COMMAND`, then `SSH_AUTH_SOCK` /
    // `SSH_AGENT_PID` (`T-19-82`), then `GSD_MM_RUN_ID` (`T-19-90`).
    let repo = in_namespace_repo();
    let work = repo.path().join("work");

    // -- the no-repo bases: the bare assignment prefix on a permitted verb.
    refuses(
        "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git commit -m x",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        T_19_104_DERIVATION,
    );

    // -- ALL THREE SPELLINGS on the in-namespace push, mirroring the three
    //    spellings of the paired discriminator exactly. Because `GIT_CONFIG_COUNT`
    //    is already refused in all three, one list entry derives all three here.
    for spelling in [
        format!("GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git {IN_NAMESPACE_PUSH}"),
        format!(
            "export GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\"; \
             git {IN_NAMESPACE_PUSH}"
        ),
        format!(
            "env GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git {IN_NAMESPACE_PUSH}"
        ),
    ] {
        refuses_in(
            Some(&work),
            &spelling,
            policy::REASON_HOOK_BYPASS_BLOCKED,
            T_19_104_DERIVATION,
        );
    }

    // -- THE COMPOSED ROW: the environment carrier carrying the command-line
    //    carrier's own indirection. `GIT_CONFIG_PARAMETERS="'include.path=<f>'"`
    //    is measured against real git at `/PARAM_INCLUDE_WINS`, so both stages
    //    compose. It is refused by the SAME `ENVELOPE_ENV_KEYS` entry — the key
    //    name is what the guard reads, never the value — which is why this row's
    //    verdict is derivable from the same discriminator.
    refuses_in(
        Some(&work),
        &format!(
            "GIT_CONFIG_PARAMETERS=\"'include.path=/tmp/evil.cfg'\" git {IN_NAMESPACE_PUSH}"
        ),
        policy::REASON_HOOK_BYPASS_BLOCKED,
        T_19_104_DERIVATION,
    );
}

#[test]
fn after_19_23_the_disclosed_bare_word_cost_of_the_new_list_entry_is_refused() {
    // **THE DISCLOSED COST, pinned beside its measured twin rather than found by
    // audit 8.** `tampers_with_envelope_env` (`policy.rs:2901-2928`) matches the
    // KEY NAME as a bare word, so a command that merely NAMES the variable without
    // setting it is refused. That false positive is already DISCLOSED in the
    // production doc and already measured for the existing entries:
    //
    //   echo GIT_CONFIG_COUNT  -> exit 2 hook_bypass_blocked  (TODAY, section 3)
    //
    // Adding `GIT_CONFIG_PARAMETERS` to the list therefore costs exactly one more
    // bare word, in the safe direction, and this row states the cost in the corpus.
    refuses(
        "echo GIT_CONFIG_PARAMETERS",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "DISCLOSED COST. Its twin `echo GIT_CONFIG_COUNT` is measured refused TODAY, so this \
         is the existing, already-disclosed bare-word false positive extended by exactly one \
         key — not a new class of over-refusal.",
    );
}

// ===========================================================================
// 3. The PAIRED DISCRIMINATORS, asserted UNCHANGED
//
// GREEN today and after. These are what make this a gap in REACH rather than in
// MECHANISM: the guard already refuses the direct spelling of the same harm,
// through both carriers and in all three environment spellings.
// ===========================================================================

#[test]
fn the_paired_discriminators_already_refuse_and_prove_the_mechanism_present() {
    // **The command-line half.** Both carriers, and the CASE row.
    refuses(
        "git -c core.hooksPath=/dev/null commit -m x",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the direct `-c core.hooksPath=` spelling on a PERMITTED base is already refused — \
         which is why `T-19-103` is a gap in the check's REACH and not in its existence",
    );
    refuses(
        "git --config-env=core.hooksPath=EVILVAR status",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the attached `--config-env` carrier already reaches the same key check",
    );
    refuses(
        "git --config-env core.hooksPath=EVILVAR status",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "and so does the separate-word spelling, through arm (a) of `leading_git_option`",
    );

    // **THE CASE ROW, AND THE STATEMENT THAT NO EXISTING PIN CHANGES.**
    //
    // `is_hooks_path_key` (`policy.rs:750-752`) is ALREADY
    // `eq_ignore_ascii_case("core.hookspath")`, so the guard's whole-key fold is
    // already right for the section and the variable — and is MORE permissive than
    // git for a subsectioned key, which is the safe direction. This row is green
    // today, and `19-23` must leave it green.
    refuses(
        "git -c CORE.HOOKSPATH=/dev/null status",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "CASE CONTROL. Git folds the config SECTION and VARIABLE to lower case (measured: \
         `-c INCLUDE.PATH=<f>` resolves to `/INCLUDE_WINS`) and `is_hooks_path_key` is already \
         `eq_ignore_ascii_case`. **NO EXISTING PIN CHANGES in this round**, and this row is \
         the mechanical statement of that.",
    );
}

#[test]
fn all_three_environment_spellings_of_the_discriminator_already_refuse_on_both_kinds_of_base() {
    // **This test is the whole derivation of section 2**, and that is why it
    // drives all three spellings on BOTH a refused and a permitted base rather
    // than one spelling on one base. If any row here ever goes green, every
    // `T-19-104` post-fix verdict in section 2 becomes a guess rather than a
    // derivation.
    let repo = in_namespace_repo();
    let work = repo.path().join("work");

    // -- on a base the envelope PERMITS unwrapped (the in-namespace push).
    for spelling in [
        format!("GIT_CONFIG_COUNT=0 git {IN_NAMESPACE_PUSH}"),
        format!("export GIT_CONFIG_COUNT=0; git {IN_NAMESPACE_PUSH}"),
        format!("env GIT_CONFIG_COUNT=0 git {IN_NAMESPACE_PUSH}"),
    ] {
        refuses_in(
            Some(&work),
            &spelling,
            policy::REASON_HOOK_BYPASS_BLOCKED,
            "DERIVATION SOURCE for section 2. `GIT_CONFIG_COUNT` IS in `ENVELOPE_ENV_KEYS`, \
             and all three environment spellings are refused. If this is red, section 2's \
             post-fix verdicts stop being derivable.",
        );
    }

    // -- and on `git status`, the other permitted base, so the derivation does not
    //    rest on push handling.
    for spelling in [
        "GIT_CONFIG_COUNT=0 git status",
        "export GIT_CONFIG_COUNT=0; git status",
        "env GIT_CONFIG_COUNT=0 git status",
    ] {
        refuses(
            spelling,
            policy::REASON_HOOK_BYPASS_BLOCKED,
            "DERIVATION SOURCE for section 2, on the second permitted base",
        );
    }

    // -- and the bare-word cost twin, which is what makes section 2's disclosed
    //    cost row a statement of an EXISTING cost rather than a new one.
    refuses(
        "echo GIT_CONFIG_COUNT",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the disclosed bare-word false positive of `tampers_with_envelope_env`, measured for \
         a key that is ALREADY in the list",
    );
}

// ===========================================================================
// 4. The DISCRIMINATION CONTROLS — this round's `--signed no`
//
// Two rows GREEN today and pinned GREEN AFTER, and one row whose verdict the
// rule DOES move, pinned at its post-fix verdict so the cost is disclosed here
// rather than discovered by audit 8.
// ===========================================================================

#[test]
fn a_near_miss_key_that_names_no_include_section_stays_permitted_before_and_after() {
    // **THIS ROUND'S `--signed no`, and the two rows are the same control from
    // opposite sides.**
    //
    //   git -c includepath=/tmp/evil.cfg     — no `.` at all, so NO SECTION
    //   git -c notinclude.path=/tmp/evil.cfg — a section, but it is `notinclude`
    //
    // Both are measured **exit 0** today and both are pinned **exit 0 after**. A
    // rule written as `key.contains("include")` turns BOTH red. Only a rule that
    // compares the SECTION — the text before the first `.`, ASCII-case-folded —
    // keeps them green.
    //
    // **This is what stops the fix from being a substring match**, and it is the
    // reason `19-23`'s rule is specified as a section comparison rather than as a
    // search for a word.
    permits(
        "git -c includepath=/tmp/evil.cfg status",
        "DISCRIMINATION CONTROL. `includepath` has no `.` and therefore names NO config \
         section at all — real git answers `error: key does not contain a section` for a \
         dotless key. A red here means the rule became a substring match on `include`, which \
         is a rule that cannot say what it means. The correct response is to compare the \
         SECTION, never to delete this row.",
    );
    permits(
        "git -c notinclude.path=/tmp/evil.cfg status",
        "DISCRIMINATION CONTROL, from the other side. The SECTION here is `notinclude`, which \
         pulls nothing in. A red here means the rule matched letters rather than a section.",
    );
}

#[test]
fn after_19_23_the_disclosed_cost_of_reading_the_section_and_not_the_variable_is_refused() {
    // **THE DISCLOSED COST, stated in the corpus rather than found by audit 8.**
    //
    // `include.pathx` names the `include` SECTION with a variable git IGNORES.
    // Measured against real git with the envelope's own injection as the control:
    //
    //   git -c include.pathx=<file> config --get core.hooksPath  ->  /ENV_WINS
    //
    // i.e. the include was NOT honoured. `19-23`'s mandated rule reads the SECTION
    // and DELIBERATELY does not read the variable, so this key is refused even
    // though git ignores it. That is an over-refusal, and it is in the SAFE
    // direction: `[include]` honours exactly one variable and `includeIf`'s
    // subsection is an open condition family, so not reading either covers both
    // families by construction, and the only cost is the row below.
    //
    // **BEFORE:** exit 0. **AFTER:** exit 2 `envelope_assertion_failed`.
    refuses(
        "git -c include.pathx=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "DISCLOSED COST. Real git IGNORES `include.pathx` (measured: the injection's \
         `/ENV_WINS` still wins), but `19-23`'s rule reads the SECTION and not the variable, \
         so a key git ignores is refused. Over-refusal in the safe direction, stated here \
         rather than discovered later. The correct response to a red is NOT to start reading \
         the variable — that would re-open `includeIf`'s open condition family.",
    );
}

// ===========================================================================
// 5. The PERMITTED HALF — what makes this a resolution MODEL rather than a
//    blanket refusal of anything spelled `-c`
//
// GREEN today and pinned GREEN AFTER.
// ===========================================================================

#[test]
fn the_permitted_half_of_the_config_axis_stays_permitted() {
    // **`git -c user.name="$NAME" commit -m x` is this axis's `ls
    // {git,svn}-repo`** — the row that tells a resolution MODEL apart from a
    // blanket refusal of anything spelled `-c`, which is the shape this fix is ONE
    // WRONG STEP away from. A corpus without this half cannot fail on an
    // implementation that refuses every `-c`, and such an implementation would
    // make a driven run unusable, which is how a safety control gets switched off
    // (AR-19-11). Setting `user.name` on the command line is exactly what a driven
    // run does to make its commits attributable.
    permits(
        "git -c user.name=\"$NAME\" commit -m x",
        "**THE BLANKET-REFUSAL CONTROL.** A confined `-c` assignment whose effect is bounded \
         to the key it names must stay permitted. A red here is the fix having become a \
         refusal of every `-c`.",
    );
    permits(
        "git -c core.pager=cat log",
        "a second confined assignment, on a different verb",
    );
    permits(
        "git -c a=b status",
        "**THE DOTLESS KEY** — see section 6 for why refusing this would turn round 7's whole \
         generative property permanently red",
    );
    permits(
        "git --git-dir=/tmp/g status",
        "a non-`-c` leading option that carries no configuration at all",
    );
    permits(
        "git -C /tmp status",
        "`-C` changes directory; a repo-level include splices at REPO precedence and loses to \
         the envelope's injection (measured: `/ENV_WINS`), so this is not a second escape",
    );
    permits(
        "git --no-pager status",
        "round 7's own `ls {git,svn}-repo`, carried here so this round cannot pay for its \
         rule by narrowing round 7's",
    );
}

#[test]
fn the_verdict_preserving_refusals_of_a_confined_carrier_keep_their_identifier() {
    // The other side of the permitted half: a CONFINED carrier on a REFUSED base
    // must keep the base's own identifier before AND after. If either of these
    // ever landed on `envelope_assertion_failed`, the rule would have widened from
    // indirections to every `-c` — the same defect the permitted half catches,
    // seen from the refused side.
    refuses(
        "git -c a=b push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "a confined carrier is VERDICT-PRESERVING: the base's own force-push denial is what \
         must fire, before and after",
    );
    refuses(
        "git -c a=b --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "and it stays verdict-preserving after round 7's grammar consumes `--attr-source`'s \
         value — which is `CALLEE_KNOWN_LEADING_PREFIX`'s own splice position",
    );
}

// ===========================================================================
// 6. The DOTLESS-KEY PIN — the seam this plan exists to fence
// ===========================================================================

#[test]
fn a_dotless_config_key_names_no_section_and_must_stay_confined() {
    // **THE SEAM DEFECT THAT WOULD HAVE BLOCKED THIS ROUND, caught at plan time
    // and fenced here and mechanically in `tests/envelope_wrapper_class.rs`.**
    //
    // `tests/envelope_wrapper_class.rs:5197` defines
    // `CALLEE_KNOWN_LEADING_PREFIX: &str = "-c a=b"`, and round 7's ENTIRE
    // callee-grammar generative property is spliced behind it — every case of the
    // `AfterAKnownLeadingOption` splice begins `git -c a=b …`.
    //
    // Measured against the REAL git binary (asserted in section 12):
    //
    //   git -c a=b version        -> `git version 2.43.0`, rc 0   (it RUNS)
    //   git -c a=b config --get a -> `error: key does not contain a section: a`, rc 1
    //
    // So a dotless key CANNOT be an indirection — git will not even resolve it —
    // and confining it is CORRECT BY DESIGN, not a concession.
    //
    // **A rule that refused a key it cannot decompose into a section would refuse
    // `-c a=b`, turn round 7's whole property PERMANENTLY RED in a file `19-23`
    // may not edit, and reproduce `19-18`'s `{v}>` blocker one axis over.**
    permits(
        "git -c a=b status",
        "**THE DOTLESS FENCE.** `-c a=b` is `CALLEE_KNOWN_LEADING_PREFIX` \
         (`tests/envelope_wrapper_class.rs:5197`) and round 7's entire callee-grammar \
         generative property is spliced behind it. Refusing it turns that property \
         permanently red in a file `19-23` MAY NOT EDIT. The correct response to a red here \
         is to confine dotless keys, never to edit round 7's property.",
    );
    refuses(
        "git -c a=b push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "and on a refused base it keeps the base's own identifier, which is what \
         verdict-preserving means for `CALLEE_KNOWN_LEADING_PREFIX`",
    );
}

// ===========================================================================
// 7. The rows ALREADY REFUSED TODAY, labelled with the clause that produces
//    them — and the TWO ORDERING PINS
// ===========================================================================

#[test]
fn the_tilde_bearing_include_key_is_already_refused_by_the_rewriting_character_clause() {
    // **A CONTROL, and it must be labelled one.** This row is exit 2
    // `envelope_assertion_failed` TODAY, and NOT by anything this round adds: the
    // `~` in the key half is a character the shell may rewrite, so the existing
    // rewriting-character clause inside `scan_leading`'s `if let
    // Some(assignment)` block (`policy.rs:397-466`) refuses it before
    // `is_hooks_path_key` is consulted at all.
    //
    // The IDENTIFIER is therefore unchanged after `19-23` while the CLAUSE that
    // produces it may not be — which is exactly why this is asserted at its
    // present verdict and labelled a control rather than counted as a reproducer.
    refuses(
        "git -c includeIf.gitdir:~/p/.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**CONTROL, NOT REPRODUCER.** Refused TODAY by the EXISTING rewriting-character clause \
         on the key half, not by this round's rule. Counting it as a `T-19-103` reproducer \
         would be counting a row that was already refused for an unrelated reason.",
    );
}

#[test]
fn after_19_23_the_scan_order_decides_which_clause_names_a_line_carrying_both() {
    // **THE ORDERING PIN, and the two rows are asserted at DIFFERENT post-fix
    // verdicts on purpose.** Together they are the mechanical proof that the two
    // clauses are ordered by the SCAN and not by a priority a reader has to infer,
    // and a `19-23` that raised its refusal in a SECOND PASS over the leading
    // tokens would turn one of them red.
    //
    // Both are exit 2 `hook_bypass_blocked` TODAY, because the include carrier is
    // invisible to the current scan and only the hooks key is seen.

    // -- INDIRECTION FIRST. **RED today**: the verdict does not move but the
    //    IDENTIFIER does.
    //
    //    **THE DERIVATION.** `19-23`'s clause is raised at the FIRST assignment
    //    the loop cannot bound, and `scan_leading` is a single LEFT-TO-RIGHT walk
    //    that returns as soon as it refuses — no assignment after it is read. So a
    //    line carrying both earns the unresolvable identifier and not the
    //    hooks-path one. That is D-24's requirement that a refusal name the
    //    mechanism that produced it.
    refuses(
        "git -c include.path=/tmp/evil.cfg -c core.hooksPath=/dev/null push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "ORDERING PIN. The scan is LEFT TO RIGHT and reaches the INDIRECTION first, so the \
         refusal must name the clause that produced it. A red here means `19-23` raised its \
         clause in a second pass rather than inside the same walk.",
    );

    // -- HOOKS KEY FIRST. **GREEN today and after**, and it is what stops the pin
    //    above from being satisfiable by simply preferring one identifier always.
    refuses(
        "git -c core.hooksPath=/dev/null -c include.path=/tmp/evil.cfg push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "ORDERING PIN, the other direction. The same left-to-right scan reaches the HOOKS KEY \
         first here, so this row's identifier must NOT move. If both rows land on the same \
         identifier, the ordering is not the scan's.",
    );
}

// ===========================================================================
// 8. The COMPOSITION rows — which prove rounds 6 and 7 load-bearing FOR THIS
//    ROUND rather than superseded by it
//
// Built on PERMITTED bases, because that is this axis's whole constraint.
// RED at plan 19-22's end.
// ===========================================================================

#[test]
fn after_19_23_the_composition_rows_are_refused_which_proves_rounds_6_and_7_load_bearing() {
    // Each row was measured FIRST — all four at **exit 0**, walk EMPTY — and each
    // is asserted at the verdict the measurement plus the derivation supports.
    //
    // **These are the rows that turn red if either earlier model is ever removed,
    // whatever this round's rules do.**

    // -- ROUND 6. The redirection is DELETED from the argv the guard classifies
    //    (`tests/envelope_argv_deletion.rs`), and only then is there a
    //    `-c include.path=` in the leading region for this round's rule to read at
    //    all. Pinned non-vacuous in section 11.
    refuses(
        "git >/dev/null -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "COMPOSITION with round 6. If round 6's deletion model were removed, `>/dev/null` \
         would still be an argv word and the carrier would not be in the leading region. This \
         row turns red on that regression whatever this round's rules do.",
    );

    // -- ROUND 7. `--attr-source` CONSUMES `HEAD` under round 7's grammar
    //    (`GIT_GLOBAL_VALUE_OPTS`), so the scan advances TWO words and lands on
    //    `-c include.path=…` rather than treating `HEAD` as the verb. Pinned
    //    non-vacuous in section 11.
    refuses(
        "git --attr-source HEAD -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "COMPOSITION with round 7. The scan reaches the carrier only because round 7's grammar \
         knows `--attr-source` consumes a separate word.",
    );

    // -- THE WRAPPER AXIS. `bash -lc "…"` is unwrapped by `resolve_program` and
    //    the inner command is classified, so the carrier is reached through the
    //    wrapper class rather than around it.
    refuses(
        "bash -lc \"git -c include.path=/tmp/evil.cfg status\"",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "COMPOSITION with the wrapper axis: the inner command is what gets classified",
    );

    // -- THE SEGMENT AXIS. The carrier is in the SECOND segment of a `&&` list.
    refuses(
        "echo hi && git -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "COMPOSITION with the segment axis: every segment is classified, not only the first",
    );
}

// ===========================================================================
// 9. The two rows RECORDED rather than asserted
//
// Each is measured and the measurement is RECORDED IN A COMMENT AND A PRINT —
// **never written as an assertion** — because this plan measures PRE-fix and
// cannot catch a wrong POST-fix expectation, and a row pinned at a verdict the
// rules cannot produce lands RED with `19-23` forbidden to edit it. `19-16`,
// `19-18` and `19-20` handled their undeliverable rows exactly this way. `19-23`
// measures each against the rules and APPENDS the pin as a new fn.
// ===========================================================================

#[test]
fn the_two_undeliverable_rows_are_recorded_here_and_left_for_19_23_to_pin() {
    let repo = in_namespace_repo();
    let work = repo.path().join("work");

    // -- ROW 1: `GIT_CONFIG_NOSYSTEM`.
    //
    // MEASURED at this file's base commit: **exit 0**, walk EMPTY.
    //
    // **It is a measured DEFEAT of a key the envelope SETS**, confirmed against
    // real git: with a system file carrying `credential.helper = evil`,
    //
    //   GIT_CONFIG_SYSTEM=<file> git config --get credential.helper       -> `evil`
    //   + GIT_CONFIG_NOSYSTEM=1                                           -> rc 1, nothing read
    //
    // **But its HARM is INERT and must not be called a bypass.**
    // `cred::write_gitconfig`'s own doc (`cred.rs:253-271`) records that the
    // generated helper-free file is pointed at by **BOTH** `GIT_CONFIG_GLOBAL` and
    // `GIT_CONFIG_SYSTEM`, so suppressing the system read removes a deny that the
    // global pointer duplicates.
    //
    // **Whether it earns an `ENVELOPE_ENV_KEYS` entry is `19-23`'s design
    // decision**, which is exactly why no post-fix verdict is asserted here.
    // Folded into `T-19-104`'s class rather than registered as a new threat ID,
    // with the `19-14` provenance caveat: found while planning, not by an audit.
    record_only(
        "GIT_CONFIG_NOSYSTEM — a measured DEFEAT with an INERT harm; `19-23`'s design decision",
        Some(&work),
        &format!("GIT_CONFIG_NOSYSTEM=1 git {IN_NAMESPACE_PUSH}"),
    );

    // -- ROW 2: the indirection carrier on a `--force` base.
    //
    // MEASURED at this file's base commit: **exit 2 `force_push_blocked`** — it is
    // refused for its VERB, not for its carrier. Its VERDICT therefore does not
    // move, while its IDENTIFIER may: which of the two it lands on is a
    // clause-ordering outcome `19-23` measures, exactly as section 7's ordering
    // pins are. Asserting merely that it is REFUSED would be asserting something
    // already true for an unrelated reason, which certifies nothing.
    record_only(
        "the indirection carrier on a --force base — refused for its VERB; identifier is \
         19-23's to measure",
        None,
        "git -c include.path=$F push --force origin main",
    );
}

// ===========================================================================
// 10. The `T-19-86` PERSISTED-ALIAS ARM — RECORDED, and explicitly NOT closing
//     anything
// ===========================================================================

#[test]
fn the_persisted_alias_arm_of_t_19_86_keeps_exiting_zero_and_is_not_closed_here() {
    // **`T-19-86` is OPEN at `high` by explicit user scoping decision, and
    // recording this arm does NOT close, narrow or re-scope it.** Its four
    // registered rows keep exiting 0; this is a FIFTH arm audit 7 measured that
    // the register does not name.
    //
    // Measured at this file's base commit, two SEPARATELY PERMITTED tool calls
    // that the stateless guard cannot correlate:
    //
    //   git config alias.p "!git push --force origin HEAD:refs/heads/main"  -> exit 0
    //   git p                                                              -> exit 0
    //
    // **Layer 3 catches the inner push TODAY** — the `pre-push` hook fires when
    // the alias body runs, which is `AR-19-03` working exactly as designed. **And
    // that is precisely what `T-19-103` removes**: an alias body invoked under an
    // include carrier runs with no hook at all, which is what section 13's
    // end-to-end fixture measured against a bare remote.
    //
    // **So closing `T-19-103` is a RESTORATION of layer 3's catch, and never a
    // closure of `T-19-86`.**
    permits(
        "git config alias.p \"!git push --force origin HEAD:refs/heads/main\"",
        "`T-19-86` ROW — must keep exiting 0. This plan does not fix, close, narrow or \
         re-scope `T-19-86`, and a red here would be this round having silently taken it on.",
    );
    permits(
        "git p",
        "`T-19-86` ROW — the second, separately-permitted tool call. The guard is stateless \
         and cannot correlate the two; layer 3 is what catches the inner push, and \
         `T-19-103` is what removes layer 3.",
    );
}

// ===========================================================================
// 11. The five carried-forward MECHANISM pins — GREEN today and after
//
// Re-asserted here over the same PUBLIC functions rather than moved or edited in
// place, because rounds 4, 5, 6 and 7's evidence stays attributable to the round
// that produced it. This round touches the `-c` KEY DECISION for the first time
// since round 5, so all five are re-stated in its own evidence.
// ===========================================================================

#[test]
fn rule_b_still_reports_a_severed_head_as_not_a_command_position() {
    // A verdict pin cannot replace this. If Rule B stopped firing, the lines below
    // would STILL be refused — by `resolve_program` step 5's prefix rule — and
    // every verdict pin in the suite would stay green while Rule B quietly became
    // dead code.
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
        "the first command INSIDE a brace group is a command position. If this is false, Rule \
         B has been widened to closers-and-openers and `{{ git status; }}` is refused — a \
         control failing into unusability. Segments: {segments:?}"
    );
}

#[test]
fn round_5s_literalness_bit_is_non_vacuous_and_is_right_about_every_word_of_the_bypass_line() {
    // **The pin that stops `19-23` from closing `T-19-103` by making round 5's bit
    // WRONG about a word it is RIGHT about.**
    //
    // Every word of `git -c include.path=/tmp/evil.cfg status` is written exactly
    // as the shell hands it over — no expansion, no glob, no tilde — and
    // `Token.literal` is correctly `true` for all of them. **The bit is RIGHT
    // about the entire bypass line, and the harm has nothing to do with
    // literalness**: the guard read a correct argv, found the correct verb, and
    // then assumed a configuration git will not resolve. A fix that falsified the
    // bit to obtain a refusal would be round 5's inversion quietly becoming dead
    // code with every verdict pin green.
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
         \n`pus?` is a glob in the git VERB slot, so `Token.literal` must be FALSE for it. If \
         this is now true, the inversion has stopped discriminating and every refusal it \
         produces is coming from somewhere else.\n\
         \n  tokens: {unreadable_tokens:?}"
    );

    let bypass = policy::split_segments_with_heads("git -c include.path=/tmp/evil.cfg status")
        .expect("the bypass line splits into segments");
    let bypass_tokens: Vec<(String, bool)> = bypass
        .iter()
        .flat_map(|s| s.tokens.iter())
        .map(|t| (t.text.clone(), t.literal))
        .collect();

    for word in ["-c", "include.path=/tmp/evil.cfg", "status"] {
        assert!(
            bypass_tokens.iter().any(|(text, _)| text == word),
            "`{word}` must be present as a token, or the assertion below certifies nothing. \
             tokens: {bypass_tokens:?}"
        );
        for (text, literal) in &bypass_tokens {
            if text == word {
                assert!(
                    *literal,
                    "\n\nTHE INVERSION WAS FALSIFIED INSTEAD OF THE CONFIG RESOLUTION BEING \
                     MODELLED.\n\
                     \n`{word}` is a fully LITERAL word and `Token.literal` is correctly \
                     `true` for it. The harm on this line is not that a word is unreadable, \
                     and it is not that the wrong word was called the verb — it is that the \
                     guard assumed a CONFIGURATION git will not resolve. If this bit is now \
                     `false`, `19-23` closed `T-19-103` by making round 5's bit WRONG about a \
                     word it was right about — the inversion becoming dead code with every \
                     verdict pin green.\n\
                     \n**The correct shape of the fix is to model the config INDIRECTION.**\n\
                     \n  tokens: {bypass_tokens:?}"
                );
            }
        }
    }
}

#[test]
fn round_6s_deletion_model_is_non_dead_and_this_round_must_not_remove_it() {
    // **The pin that turns red if `19-23` regresses round 6.** Section 8's
    // redirection row depends on the model having deleted `>/dev/null` before this
    // round's rule sees an assignment at all.
    refuses(
        "git >/dev/null push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "round 6's deletion model: the redirection is deleted, `push` is the verb",
    );

    // The OVER-DELETION control: `x2>/tmp/o` is a REAL argv word (bash treats `x2`
    // as an ordinary word, not an IO_NUMBER), so it must NOT be deleted — and with
    // it present, `push` is not in the verb slot. This is round 6's
    // `ls {git,svn}-repo` and it must stay PERMITTED.
    permits(
        "git x2>/tmp/o push --force origin main",
        "round 6's over-deletion control — `x2` is an ordinary word and must not be deleted",
    );
}

#[test]
fn a_redirection_operator_is_not_a_separator_and_this_round_must_not_make_it_one() {
    // **The mechanical proof that no round in this phase took the route
    // `SEPARATORS`' own doc forbids.** A redirection does not start a new command,
    // so treating it as a separator would HIDE the command it redirects.
    //
    // `separators_are_named_in_one_list_that_the_predicate_reads` inside
    // `src/envelope/policy.rs` asserts the same fact and must stay GREEN and
    // UNMODIFIED through `19-23`.
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
        "`&` IS a separator — without this the three assertions above could all pass because \
         `is_separator` answers `false` for everything"
    );
    assert!(policy::is_separator("&&"), "`&&` IS a separator");
}

#[test]
fn round_7s_fail_closed_callee_grammar_is_non_dead_and_this_round_must_not_remove_it() {
    // **The pin that turns red if `19-23` regresses round 7.** Section 8's
    // `--attr-source HEAD` composition depends on the grammar consuming a separate
    // word, and section 5's `git -c a=b --attr-source HEAD push --force …` row
    // depends on it too.
    refuses(
        "git --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "round 7's grammar: `--attr-source` consumes `HEAD`, so `push` is the verb",
    );
    refuses(
        "git --bogus-opt status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 7's FAIL-CLOSED half: an option whose grammar the guard cannot establish is a \
         refusal and not a guess",
    );

    // The two permitted controls, so the pin cannot be satisfied by refusing every
    // leading option.
    permits(
        "git --no-pager status",
        "round 7's `ls {git,svn}-repo` — a self-contained option must stay permitted",
    );
    permits(
        "git - push --force origin main",
        "`scan_leading` breaks on a bare `-` before any option check, and round 7 is \
         forbidden to widen into non-`-`-prefixed words",
    );
}

// ===========================================================================
// 12. The REAL-GIT precedence probes — measured with the envelope's OWN
//     injection as the control
//
// GREEN today and after. **This is the step that separates a resolution claim
// from a guess**, and it is asserted rather than merely recorded because a claim
// in this codebase about which forms outrank the injection is already KNOWN
// FALSE: `cred.rs:241-248` says "the one form that outranks this injection is
// `git -c core.hooksPath=… push`", and four measured forms outrank it.
// (Correcting that doc is `19-23`'s, not this plan's.)
// ===========================================================================

/// Run real git with the envelope's OWN `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`
/// injection in the environment, plus any extra environment pairs, and return the
/// value git resolves for `core.hooksPath`.
///
/// The injection is spelled exactly as `cred::hooks_path_env` emits it — count
/// DERIVED from one pair, `KEY_0`, `VALUE_0` — so this is a probe of the real
/// control rather than of a stand-in.
fn resolved_hooks_path(cwd: &Path, extra_env: &[(&str, &str)], args: &[&str]) -> String {
    let mut command = std::process::Command::new("git");
    command
        .current_dir(cwd)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/ENV_WINS");
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let out = command
        .args(args)
        .args(["config", "--get", "core.hooksPath"])
        .output()
        .expect("git is on PATH");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn real_git_resolves_an_include_directive_above_the_envelopes_own_injection() {
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);

    let evil = dir.path().join("evil.cfg");
    std::fs::write(&evil, "[core]\n\thooksPath = /INCLUDE_WINS\n").unwrap();
    let evil = evil.to_str().unwrap().to_string();

    let param_include = dir.path().join("param-include.cfg");
    std::fs::write(
        &param_include,
        "[core]\n\thooksPath = /PARAM_INCLUDE_WINS\n",
    )
    .unwrap();
    let param_include = param_include.to_str().unwrap().to_string();

    // -- THE CONTROL. If this is not `/ENV_WINS`, every row below is comparing
    //    against nothing and the whole probe is vacuous.
    assert_eq!(
        resolved_hooks_path(&repo, &[], &[]),
        "/ENV_WINS",
        "THE CONTROL. The envelope's own `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet must be \
         what git resolves when nothing else is present. A carrier is an INDIRECTION only if \
         it makes this same command print something else."
    );

    // -- THE COMMAND-LINE CARRIERS, both sections and both spellings.
    for (label, args) in [
        ("-c include.path=<file>", vec![format!("include.path={evil}")]),
        ("-c INCLUDE.PATH=<file>", vec![format!("INCLUDE.PATH={evil}")]),
        (
            "-c includeIf.gitdir:<p>.path=<file>",
            vec![format!(
                "includeIf.gitdir:{}/.path={evil}",
                repo.to_str().unwrap()
            )],
        ),
        (
            "-c INCLUDEIF.gitdir:<p>.PATH=<file>",
            vec![format!(
                "INCLUDEIF.gitdir:{}/.PATH={evil}",
                repo.to_str().unwrap()
            )],
        ),
    ] {
        let refs: Vec<&str> = std::iter::once("-c")
            .chain(args.iter().map(|s| s.as_str()))
            .collect();
        assert_eq!(
            resolved_hooks_path(&repo, &[], &refs),
            "/INCLUDE_WINS",
            "`{label}` must OUTRANK the envelope's injection. If it does not, `T-19-103` is \
             not the class this file says it is.\n\n\
             **The CASE rows are the point**: git folds the config SECTION and the VARIABLE to \
             lower case and leaves the SUBSECTION case-sensitive, which is why the rule's \
             section comparison must be ASCII-case-insensitive and why the subsection must not \
             be read at all."
        );
    }

    // -- THE `--config-env` CARRIER, whose value is a VARIABLE NAME.
    for args in [
        vec!["--config-env=include.path=EVILVAR"],
        vec!["--config-env", "include.path=EVILVAR"],
    ] {
        assert_eq!(
            resolved_hooks_path(&repo, &[("EVILVAR", &evil)], &args),
            "/INCLUDE_WINS",
            "`{args:?}` must OUTRANK the injection — the second carrier `scan_leading` reads, \
             in both of its spellings"
        );
    }

    // -- THE ENVIRONMENT CARRIER, `T-19-104`'s whole subject.
    assert_eq!(
        resolved_hooks_path(
            &repo,
            &[("GIT_CONFIG_PARAMETERS", "'core.hooksPath=/PARAM_WINS'")],
            &[]
        ),
        "/PARAM_WINS",
        "`GIT_CONFIG_PARAMETERS` must OUTRANK the envelope's own triplet. It is git's internal \
         carrier for `-c` and git EXPORTS it, so one prefix disarms every git subprocess."
    );
    assert_eq!(
        resolved_hooks_path(
            &repo,
            &[(
                "GIT_CONFIG_PARAMETERS",
                &format!("'include.path={param_include}'")
            )],
            &[]
        ),
        "/PARAM_INCLUDE_WINS",
        "and the two stages COMPOSE: the environment carrier carrying the command-line \
         carrier's own indirection"
    );

    // -- THE DISCLOSED COST, measured rather than argued: git IGNORES a variable
    //    that is not `path` in the `include` section, so refusing `include.pathx`
    //    is an over-refusal in the safe direction.
    assert_eq!(
        resolved_hooks_path(&repo, &[], &["-c", &format!("include.pathx={evil}")]),
        "/ENV_WINS",
        "real git IGNORES `include.pathx` — the injection still wins. That is what makes \
         section 4's refusal of it a DISCLOSED COST rather than a necessity, and it is stated \
         in the corpus rather than found by audit 8."
    );

    // -- THE DISCRIMINATION CONTROLS, from git's own side: neither near-miss key
    //    pulls anything in, which is why both stay PERMITTED before and after.
    assert_eq!(
        resolved_hooks_path(&repo, &[], &["-c", &format!("notinclude.path={evil}")]),
        "/ENV_WINS",
        "`notinclude.path` names a section that pulls nothing in — the SECTION is what \
         decides, not the letters"
    );

    // -- AND THE PERSISTED `git config include.path`, checked so this round does
    //    not claim the `git config` subcommand as a second escape. It splices at
    //    REPO precedence and LOSES to the injection.
    git_ok(&repo, &["config", "include.path", &evil]);
    assert_eq!(
        resolved_hooks_path(&repo, &[], &[]),
        "/ENV_WINS",
        "a PERSISTED `include.path` splices at REPOSITORY precedence and loses to the \
         envelope's env-injected triplet, so the `git config` subcommand is NOT a second \
         escape and correctly needs no clause. Measured, not assumed."
    );
}

#[test]
fn real_git_runs_a_dotless_config_key_and_refuses_only_to_resolve_it() {
    // **The measurement behind section 6's fence and behind
    // `tests/envelope_wrapper_class.rs`'s mechanical dotless assertion.**
    let dir = TempDir::new().unwrap();
    let repo = dir.path().to_path_buf();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);

    let runs = std::process::Command::new("git")
        .current_dir(&repo)
        .args(["-c", "a=b", "version"])
        .output()
        .expect("git is on PATH");
    assert!(
        runs.status.success(),
        "**real git RUNS `git -c a=b version`** — a dotless key is not an error until \
         something READS it. This is why refusing `-c a=b` would be refusing a command git \
         itself accepts, and why `CALLEE_KNOWN_LEADING_PREFIX` can safely be `-c a=b`. \
         stderr: {}",
        String::from_utf8_lossy(&runs.stderr)
    );
    assert!(
        String::from_utf8_lossy(&runs.stdout).contains("git version"),
        "and it prints a version. Got: {}",
        String::from_utf8_lossy(&runs.stdout)
    );

    let reads = std::process::Command::new("git")
        .current_dir(&repo)
        .args(["-c", "a=b", "config", "--get", "a"])
        .output()
        .expect("git is on PATH");
    assert!(
        !reads.status.success(),
        "and `git -c a=b config --get a` FAILS — a dotless key names no section, so it can \
         never be an indirection. Confining it is correct by design, not a concession."
    );
    assert!(
        String::from_utf8_lossy(&reads.stderr).contains("does not contain a section"),
        "with git's own words. Got: {}",
        String::from_utf8_lossy(&reads.stderr)
    );
}

// ===========================================================================
// 13. The class confirmed END TO END against real git with a BARE REMOTE
//
// GREEN today and after. **Not inherited by citation from audit 7** — the
// fixture is rebuilt here and all four legs are re-measured, with the remote's
// SHA recorded BEFORE and AFTER each, because a SHA is what makes "the remote
// moved" a measurement rather than a claim.
// ===========================================================================

/// The remote's SHA for the in-namespace branch, or `(absent)`.
fn remote_sha(upstream: &Path) -> String {
    let out = std::process::Command::new("git")
        .arg("--git-dir")
        .arg(upstream)
        .args(["rev-parse", "refs/heads/gsd-auto/alpha/w"])
        .output()
        .expect("git is on PATH");
    if out.status.success() {
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    } else {
        "(absent)".to_string()
    }
}

#[test]
fn a_config_indirection_carries_a_refused_push_past_the_hook_and_moves_a_bare_remotes_ref() {
    // **THE BEHAVIOURAL CLAIM, REPRODUCED RATHER THAN REPEATED.** A claim in this
    // codebase about which forms outrank the injection is already known false, so
    // this round does not carry forward a behavioural claim it has not measured.
    //
    // The hook is delivered EXACTLY as the envelope delivers it — `core.hooksPath`
    // through the `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet
    // `cred::hooks_path_env` emits — and the remote is a LOCAL BARE repository, so
    // this is offline.
    let dir = TempDir::new().unwrap();
    let upstream = dir.path().join("upstream.git");
    let work = dir.path().join("work");
    let hooks = dir.path().join("hooks");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&hooks).unwrap();

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

    for name in ["pre-push", "pre-commit"] {
        let path = hooks.join(name);
        std::fs::write(&path, format!("#!/bin/sh\necho '{name}: REFUSED' >&2\nexit 1\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    let evil = dir.path().join("evil.cfg");
    std::fs::write(&evil, "[core]\n\thooksPath = /nonexistent-hooks-dir\n").unwrap();

    // Every leg runs with the envelope's own injection in the environment.
    let run = |extra_env: &[(&str, &str)], args: &[&str]| {
        let mut command = std::process::Command::new("git");
        command
            .current_dir(&work)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", hooks.to_str().unwrap())
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid");
        for (key, value) in extra_env {
            command.env(key, value);
        }
        command
            .args(args)
            .output()
            .expect("git is on PATH")
            .status
            .success()
    };

    // -- LEG 1: the plain in-namespace push. The hook REFUSES and the ref does not
    //    move. Without this leg the two below prove nothing, because a ref that
    //    was going to move anyway is not evidence of a disarmed hook.
    std::fs::write(work.join("a"), "x2\n").unwrap();
    git_ok(&work, &["commit", "-qam", "c2", "--no-verify"]);
    let before = remote_sha(&upstream);
    let ok = run(&[], &["push", "origin", "refs/heads/gsd-auto/alpha/w"]);
    let after = remote_sha(&upstream);
    assert!(
        !ok,
        "LEG 1: the plain in-namespace push must be REFUSED by the `pre-push` hook the \
         envelope installed. If it is not, the fixture's hook is not being read and every \
         leg below is measuring nothing."
    );
    assert_eq!(
        before, after,
        "LEG 1: and the bare remote's ref must NOT MOVE. before={before} after={after}"
    );

    // -- LEG 2: the same push under `-c include.path=`, which the guard PERMITS at
    //    exit 0 (section 1). The hook is gone and the ref MOVES.
    let before = remote_sha(&upstream);
    let ok = run(
        &[],
        &[
            "-c",
            &format!("include.path={}", evil.to_str().unwrap()),
            "push",
            "origin",
            "refs/heads/gsd-auto/alpha/w",
        ],
    );
    let after = remote_sha(&upstream);
    assert!(
        ok,
        "LEG 2: `-c include.path=<evil>` must carry the push to COMPLETION. This is the line \
         the guard permits at exit 0."
    );
    assert_ne!(
        before, after,
        "\n\nLEG 2: **THE BARE REMOTE'S REF MUST MOVE.** before={before} after={after}\n\n\
         This is `T-19-103`'s harm as a MEASUREMENT rather than an argument: a push the \
         `pre-push` hook refuses completes and rewrites a remote ref, on a command line the \
         guard permits at exit 0. `pre-push` is the ONLY carrier of the worktree credential \
         scan (`hooks.rs:342`, SAFE-05) and the second carrier `AR-19-03` rests on."
    );

    // -- LEG 3: the ENVIRONMENT carrier, `T-19-104`'s subject, on the same fixture.
    std::fs::write(work.join("a"), "x3\n").unwrap();
    git_ok(&work, &["commit", "-qam", "c3", "--no-verify"]);
    let before = remote_sha(&upstream);
    let ok = run(
        &[("GIT_CONFIG_PARAMETERS", "'core.hooksPath=/nonexistent-hooks-dir'")],
        &["push", "origin", "refs/heads/gsd-auto/alpha/w"],
    );
    let after = remote_sha(&upstream);
    assert!(ok, "LEG 3: the `GIT_CONFIG_PARAMETERS` carrier must carry the push to completion");
    assert_ne!(
        before, after,
        "LEG 3: and the bare remote's ref must MOVE. before={before} after={after}\n\n\
         Git EXPORTS this variable, so one prefix disarms every git SUBPROCESS of the command \
         — which is how `T-19-104` composes with `T-19-86`."
    );

    // -- LEG 4: the `pre-commit` enforcement point falls the same way.
    std::fs::write(work.join("a"), "x4\n").unwrap();
    let head_before = std::process::Command::new("git")
        .current_dir(&work)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git is on PATH");
    let head_before = String::from_utf8_lossy(&head_before.stdout).trim().to_string();
    assert!(
        !run(&[], &["commit", "-qam", "c4"]),
        "LEG 4: the control commit must be REFUSED by the `pre-commit` hook"
    );
    assert!(
        run(
            &[],
            &[
                "-c",
                &format!("include.path={}", evil.to_str().unwrap()),
                "commit",
                "-qam",
                "c4",
            ],
        ),
        "LEG 4: and the same commit under the carrier must SUCCEED"
    );
    let head_after = std::process::Command::new("git")
        .current_dir(&work)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git is on PATH");
    let head_after = String::from_utf8_lossy(&head_after.stdout).trim().to_string();
    assert_ne!(
        head_before, head_after,
        "LEG 4: HEAD must have MOVED. before={head_before} after={head_after}"
    );

    println!(
        "END-TO-END: all four legs reproduced independently of audit 7. \
         Final remote ref: {}",
        remote_sha(&upstream)
    );
}

#[test]
fn git_config_nosystem_defeats_a_key_the_envelope_sets_and_the_harm_is_inert() {
    // **RECORDED AS A MEASURED DEFEAT WITH AN INERT EFFECT — never as a bypass.**
    //
    // The DEFEAT is real and is asserted here. The HARM is not, and the reason is
    // in `cred::write_gitconfig`'s own doc (`cred.rs:253-271`): the generated
    // helper-free file is pointed at by **BOTH** `GIT_CONFIG_GLOBAL` and
    // `GIT_CONFIG_SYSTEM`, so suppressing the system read removes a deny the
    // global pointer duplicates.
    //
    // Whether it earns an `ENVELOPE_ENV_KEYS` entry is `19-23`'s design decision;
    // the guard-side row is RECORDED and not asserted (section 9).
    let dir = TempDir::new().unwrap();
    let system = dir.path().join("system.cfg");
    std::fs::write(&system, "[credential]\n\thelper = evil\n").unwrap();
    let repo = dir.path().join("r");
    std::fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);

    let helper = |extra: &[(&str, &str)]| {
        let mut command = std::process::Command::new("git");
        command
            .current_dir(&repo)
            .env("GIT_CONFIG_SYSTEM", &system);
        for (key, value) in extra {
            command.env(key, value);
        }
        let out = command
            .args(["config", "--get", "credential.helper"])
            .output()
            .expect("git is on PATH");
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).trim().to_string(),
        )
    };

    assert_eq!(
        helper(&[]),
        (true, "evil".to_string()),
        "the control: with `GIT_CONFIG_SYSTEM` pointing at a file that names a helper, git \
         reads it"
    );
    assert_eq!(
        helper(&[("GIT_CONFIG_NOSYSTEM", "1")]).0,
        false,
        "**THE DEFEAT.** `GIT_CONFIG_NOSYSTEM=1` makes git read NOTHING from the system file \
         — a key the envelope does not set that suppresses one it does.\n\n\
         **This is NOT a bypass**, and this round does not call it one: \
         `cred::write_gitconfig` points BOTH `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` at \
         the same helper-free file, so the deny this removes is duplicated by the global \
         pointer. Recorded as a measured defeat with an INERT effect, folded into \
         `T-19-104`'s class rather than registered as a new threat ID, with the `19-14` \
         provenance caveat: found while planning, not by an audit."
    );
}
