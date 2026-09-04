// ============================================================================
// Round 9 — the REGION the round-8 rule does not reach: a carrier delivered
// inside a config VALUE THE GUARD ITSELF CONFINED.
//
// **What this file is.** The reproducers `19-SECURITY.md`'s EIGHTH audit measured
// for `T-19-108`, plus the cells found while PLANNING round 9, re-measured here
// against the BUILT BINARY at this file's base commit with one fresh
// `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory WALKED afterwards,
// and — the step that separates a config-resolution claim from a guess —
// confirmed against the REAL `git` binary using **the envelope's OWN
// `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection as the control** rather than a
// stand-in. Every row was driven BEFORE it was written as an assertion, and every
// one of the audit-8 rows reproduced at the recorded verdict; **none failed to
// reproduce.**
//
// **This file is an EIGHTH evidence file rather than an appendix to
// `tests/envelope_config_resolution.rs`**, for the reason `19-14` created a third,
// `19-16` a fourth, `19-18` a fifth, `19-20` a sixth and `19-22` a seventh: round
// 8's evidence and round 9's evidence stay attributable to the round that produced
// them. The six carried-forward mechanism pins in section 11 are RE-ASSERTED here
// over the same public functions, not moved and not edited in place.
//
// **This file is RED at this plan's end BY DESIGN.** `19-24` writes the corpus and
// stops; `19-25` writes the rules. If any row here had been green against the
// pre-fix tree it would have been a FINDING — a property green before the fix is a
// property that could not have failed on it — and it would have been reported
// instead of asserted.
//
// **The invariant this file is about, stated once.**
//
//   A VALUE THE GUARD READ, CONFINED AND LET THROUGH IS STILL AN INPUT TO GIT'S
//   CONFIGURATION.
//
// Round 8 closed the question BELOW this one and audit 8 verified every one of its
// rows: an assignment the guard cannot BOUND to the key it names makes the command
// unresolvable. That is the right SHAPE of rule — it asks whether an assignment can
// be bounded, not whether it spells a name. **But it reads config assignments in
// TWO regions — the leading-option region `scan_leading` walks, and the
// environment — and git resolves configuration from a THIRD: a value the guard
// itself CONFINED.** `alias.q` names the section `alias`, which is not an
// indirection, so `config_key_names_an_indirection_section` correctly answers
// `false`, the assignment is CONFINED, and the carrier rides inside its VALUE into
// a position `scan_leading` never reads.
//
// **The axis did not move; the REGION did**, which is audit 8's own framing and the
// reason `CONFIG_RESOLUTION_CLASSES` gains a SIXTH class rather than a fifth axis.
//
// ============================================================================
// THE FINDING THAT SHAPES THE WHOLE CORPUS, STATED FIRST BECAUSE A BLANKET
// `alias.*` REFUSAL WOULD BE UNDISCHARGEABLE
// ============================================================================
//
// **GIT'S SHELL-ALIAS RULE IS A ONE-BYTE FACT, AND A CORPUS THAT DOES NOT FENCE IT
// HANDS `19-25` AN UNDISCHARGEABLE SEAM.** Two evidence files this round may not
// edit pin a `!`-bodied alias PERMITTED as a registered `T-19-86` row:
//
// ```
// tests/envelope_command_position.rs:550       permits("git -c alias.p='!git push --force origin main' p")
// tests/envelope_config_resolution.rs:1539     permits("git config alias.p \"!git push --force origin HEAD:refs/heads/main\"")
// ```
//
// A rule that refused every `alias.*` assignment turns **both permanently red in
// files `19-25` may not edit.** The carve-out is not a concession — it is git's own
// documented rule, and it is MEASURED IN BOTH DIRECTIONS AND AT ITS BOUNDARY at
// this file's base commit against `git version 2.43.0`, with the envelope's own
// injection as the control (`/ENV_WINS`), in section 12:
//
// ```
// control (no carrier)                                          -> /ENV_WINS
// -c alias.a='-c include.path=<f> config --get core.hooksPath'  -> /INCLUDE_WINS   K1: re-parsed IN-PROCESS
// -c alias.b='!git config --get core.hooksPath'                 -> /ENV_WINS       K2: CHILD inherits injection
// -c alias.g='config --get core.hooksPath !x'   (! not first)   -> /ENV_WINS
// -c alias.d=' !git config --get core.hooksPath' (SPACE first)  -> expansion of alias 'd' failed; '' is not a git command
// -c alias.t='<TAB>!git config --get core.hooksPath'            -> expansion of alias 't' failed; '' is not a git command
// -c alias.q='"!git -c include.path=<f> …"'  (QUOTED body)      -> expansion of alias 'q' failed;
//                                                                  '!git -c include.path=<f> …' is not a git command
// -c alias.e=''                       (EMPTY body)              -> expansion of alias 'e' failed; '' is not a git command
// -c alias.o='-c include.path=<f>'    (option only, NO verb)    -> fatal: empty alias for o
// ```
//
// **The QUOTED-body row is the one that shows reading ONE BYTE is not a loophole.**
// Its first byte is `"`, not `!`, so a rule reading the first byte REFUSES it — and
// that refusal is CORRECT, because git re-parses it IN-PROCESS with its own
// `split_cmdline` (the error names the whole dequoted string as "not a git
// command"; it was never handed to a shell). A `!` that is not first is not a shell
// body, and a SPACE or a TAB before `!` makes git refuse to expand at all.
//
// **NO SPELLING WAS FOUND IN WHICH THE FIRST BYTE IS `!` AND GIT NONETHELESS
// RE-PARSES IN-PROCESS**, which is what makes the one-byte rule safe to lean on.
// Section 4 asserts that standing fact; a counterexample is a FINDING to report,
// not a row to add.
//
// **This is `19-22`'s `-c a=b` seam and `19-18`'s `{v}>` blocker one REGION over,
// caught at PLAN time and fenced MECHANICALLY in
// `tests/envelope_wrapper_class.rs` rather than left to care.**
//
// ============================================================================
// THE SECOND CONSTRAINT: A REPRODUCER MUST BE BUILT ON A LAYER-2-PERMITTED BASE
// ============================================================================
//
// `19-22` established this and it still holds. Measured at this file's base commit,
// fresh root per row, walk EMPTY:
//
// ```
// exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg push --force origin main
// exit=2 [force_push_blocked]         git -c a=b push --force origin main
// ```
//
// Every `--force` composition is already refused for its VERB and certifies nothing
// about what a value gets re-parsed into. **Those are CONTROLS.** The reproducers
// use the layer-2-PERMITTED bases: `status`, `commit -m x`, the IN-NAMESPACE
// `push origin refs/heads/gsd-auto/alpha/w`, and the alias INVOCATION forms.
// `tests/envelope_wrapper_class.rs` asserts that constraint MECHANICALLY for the
// generative arm.
//
// ============================================================================
// THE EXIT CODES RE-MEASURED AT THIS FILE'S BASE COMMIT — every walk EMPTY
// ============================================================================
//
// ```
// exit=0  git -c alias.q="-c include.path=/tmp/evil.cfg push --force origin HEAD:refs/heads/main" q   <- T-19-108, ONE line
// exit=0  git -c alias.z="-c core.hooksPath=/dev/null push origin refs/heads/gsd-auto/alpha/w" z      <- reaches 19-02's deny
// exit=0  git -c alias.q="-c include.path=/tmp/evil.cfg status" q
// exit=0  git -c alias.q="-c include.path=/tmp/evil.cfg commit -m x" q
// exit=0  git -c ALIAS.q="-c include.path=/tmp/evil.cfg status" q                                     <- the case row
// exit=0  git --config-env=alias.q=EVILBODY status                                                    <- the second carrier
// exit=0  git config alias.p '-c include.path=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w'  <- the persisted arm
// exit=0  git config --global alias.p '-c include.path=/tmp/evil.cfg push'
// exit=0  git config --worktree|--file|--add|--replace-all alias.p '<non-shell body>'
// exit=0  git -c alias.st=status st                                                                   <- the DISCLOSED COST
// exit=0  git -c alias.lg="log --oneline" lg                                                          <- the DISCLOSED COST
// exit=0  git -c alias.co=checkout co                                                                 <- the DISCLOSED COST
// exit=0  git config alias.co checkout                                                                <- the DISCLOSED COST
// ```
//
// ## The paired discriminators, which prove this is a gap in REACH, not MECHANISM
//
// ```
// exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg push --force origin main
// exit=2 [hook_bypass_blocked]        git -c core.hooksPath=/dev/null status
// exit=2 [hook_bypass_blocked]        git config core.hooksPath /dev/null
// ```
//
// The mechanism is PRESENT at both keys and in both regions the guard reads. What
// is missing is REACH into the value it confined.
//
// ## The PERSISTENCE ASYMMETRY with `include.path`, which `19-25` must not get backwards
//
// Audit 8 measured persisted `git config include.path <evil>` INERT at repo-local,
// `--worktree` and GLOBAL, because the envelope's injection outranks it — so the
// include family correctly needs NO clause at the `git config` write site. Those
// rows are carried forward as ESTABLISHED and are also re-confirmed in section 12.
// **An alias is different IN KIND: it does not have to WIN a precedence contest, it
// only has to EXIST.** Measured while planning and re-asserted in section 12, under
// the envelope's own `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` redirect at a
// generated helper-free file:
//
// ```
// git config --global alias.g '-c include.path=<f> config --get core.hooksPath'; git g -> /INCLUDE_WINS
// git config         alias.l '-c include.path=<f> config --get core.hooksPath'; git l -> /INCLUDE_WINS
// git config         include.path <f>                                                  -> /ENV_WINS  (INERT)
// git config --global include.path <f>                                                 -> /ENV_WINS  (INERT)
// ```
//
// **A clause for `git config include.path` would be INERT and must not be added.**
//
// ## What else git re-parses from a config value — THREE kinds, and only one reaches layer 3
//
// * **K1 — re-parsed as a GIT command line, IN-PROCESS, INCLUDING its leading
//   options.** `alias.<name>` with a non-`!` body. Measurement says it is the ONLY
//   member, and it is the whole of `T-19-108`.
// * **K2 — re-parsed as a SHELL command line, executed as a CHILD that INHERITS the
//   injection.** `alias.<name>` with a `!` body, `core.pager`, `pager.<cmd>`,
//   `core.editor`, `sequence.editor`, `core.askPass`, `credential.helper`,
//   `core.sshCommand`, `core.gitProxy`, `core.fsmonitor`,
//   `core.alternateRefsCommand`, `diff.external`, `diff.<d>.command`,
//   `diff.<d>.textconv`, `merge.<d>.driver`, `mergetool.<t>.cmd`,
//   `difftool.<t>.cmd`, `filter.<n>.clean`/`.smudge`/`.process`, `gpg.program`,
//   `uploadpack.packObjectsHook`, `protocol.<n>.command`,
//   `remote.<n>.uploadpack`/`.receivepack`, `trailer.<t>.command`,
//   `submodule.<n>.update`, `web.browser`/`browser.<t>.cmd`. **Measured INERT for
//   layer 3 in section 13, with the MECHANISM recorded rather than the verdict
//   alone**: the child's own environment still carries `GIT_CONFIG_COUNT=1` /
//   `KEY_0=core.hooksPath` / `VALUE_0=/ENV_WINS`, which is WHY it inherits. Four
//   representatives measured: the `!` alias body, `diff.external`,
//   `credential.helper` and `filter.<n>.clean`.
// * **K3 — re-parsed as a FILE PATH spliced at the directive's precedence.**
//   `include.path`, `includeIf.<cond>.path`. Closed by `19-23`.
//
// ## The DEPTH and QUOTING facts, so `19-25` costs option (a) against measurement
//
// ```
// depth 2: -c alias.d2='-c "alias.inner=-c include.path=<f> config --get core.hooksPath" inner' -> /INCLUDE_WINS
// depth 3: -c alias.d3='-c "alias.d2=…" d2' + -c alias.d2=…                                     -> /INCLUDE_WINS
// dequote: -c alias.m='config --get "core.hooksPath"' m                                         -> /ENV_WINS  (resolves)
// control: git config --get '"core.hooksPath"'                                                  -> error: invalid key
// ```
//
// **Depth 3 is the row that matters**: the recursion is not a depth-2 curiosity, so
// any stated depth bound in option (a) is a residue an attacker reaches by adding
// one more nesting level. And git splits an alias body with its OWN `split_cmdline`
// rules rather than the shell's — the dequoting row resolves where the literal
// quoted key is an `invalid key` error.
//
// ## What this file does NOT close
//
// **`T-19-108` will close only AS SCOPED.** The rule `19-25` writes closes the
// NON-SHELL alias body. It does NOT close audit 7's `!`-bodied destructive pair —
// `git config alias.q '!git -c include.path=<evil> push --force origin
// HEAD:refs/heads/main'` then `git q` — because a `!` body is a whole command line
// handed to a governed program as DATA, which is `T-19-86`, OPEN at `high` and out
// of scope by explicit user decision, and whose two rows are pinned PERMITTED in
// files this round may not edit. **Section 13 measures that pair against a bare
// remote and records that it STILL MOVES THE REF after this round.**
//
// `T-19-86` is additionally recorded MEASURABLY WIDER than the register credits
// (section 10) and is NOT closed, narrowed or re-scoped. `T-19-91` remains open at
// `high`. **Neither this plan nor `19-25` clears `/gsd-secure-phase 19`**, and only
// the WRAPPER-OPERAND sub-class of `T-19-60` is closed.
//
// ============================================================================
// WHAT MAKES THIS CORPUS ABLE TO FAIL IN BOTH DIRECTIONS
// ============================================================================
//
// 1. Anti-vacuity controls that pass TODAY (section 0), including a POSITIVE walk
//    control that leaves exactly ONE ledger line.
// 2. A paired PERMITTED half (sections 5, 6 and 7) — the discrimination controls
//    `aliasx.` and `notalias.`, the dotless fence, round 8's permitted half, and
//    the alias INVOCATION rows.
// 3. The DISCLOSED OVER-REFUSAL family (section 6), every row beside the permitted
//    twin that does the same work.
// 4. Every refusal row asserts the D-24 reason identifier as well as the exit code.
// 5. Every row asserts what the WALKED envelope root holds.
// 6. Six MECHANISM pins (section 11) asserted over `policy::` public functions, so
//    rounds 4-8's rules cannot become dead code with every verdict pin green.
// 7. REAL-GIT probes (sections 12 and 13) that fail if git's own resolution ever
//    stops behaving the way every derivation here assumes.
// 8. TWO rows RECORDED rather than asserted (section 9), because this plan measures
//    PRE-fix and cannot catch a wrong POST-fix expectation.
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
/// Carried forward unchanged in shape from `tests/envelope_config_resolution.rs`.
/// The project root is a parameter rather than the process's working directory on
/// purpose (`T-19-80`).
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

fn ask(envelope_root: &Path, command: &str) -> Answer {
    ask_in(envelope_root, None, command)
}

// ---------------------------------------------------------------------------
// The walk — carried forward unchanged from `tests/envelope_config_resolution.rs`
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

/// Every line ANYWHERE under the envelope root that is a pull-request ledger entry.
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
/// envelope root of its own, and that the walked root holds no ledger line.
fn refuses_in(project_root: Option<&Path>, command: &str, reason: &str, why: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask_in(envelope.path(), project_root, command);

    assert_eq!(
        answer.code, 2,
        "\n\n`{command}` must be REFUSED.\n\n{why}\n\n\
         It was measured against the BUILT BINARY at this file's base commit with a fresh \
         envelope root and a walk after, and every re-parse claim behind it was confirmed \
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
         The guard sees EVERY Bash tool call, so a rule that refused every `alias.*` would \
         refuse this and make a driven run unusable — which is how a safety control gets \
         switched off (AR-19-11).\n  stdout: {}\n  stderr: {}",
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
/// **Section 9's two rows use this and ONLY this.** A row whose POST-fix verdict
/// this plan cannot DERIVE must not be pinned: this plan measures PRE-fix, so its
/// measure-first discipline cannot catch a wrong post-fix expectation, and a row
/// pinned at a verdict the rules cannot produce lands RED with `19-25` forbidden to
/// edit it — which stalls the round. **`19-22` asserted such a row against its own
/// comment, its own SUMMARY and its own plan-check, and it halted `19-23`
/// mid-plan.** `19-16`, `19-18`, `19-20` and `19-22` handled their undeliverable
/// rows exactly this way.
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
// The git fixtures — offline, and NOT the test process's cwd
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

/// The one in-namespace push base every push row is built on. Layer 2 PERMITS it,
/// which is the whole constraint of this axis.
const IN_NAMESPACE_PUSH: &str = "push origin refs/heads/gsd-auto/alpha/w";

/// The derivation every `-c`-delivery `T-19-108` row shares, written once and
/// quoted into each row's own failure message.
const T_19_108_LEADING_DERIVATION: &str = "\
**THE DERIVATION.** `19-25`'s mandated clause is raised INSIDE `scan_leading`, at \
the first `-c` / `--config-env` assignment whose effect on `core.hooksPath` it \
cannot BOUND, and BEFORE any verb is classified. So the identifier is the \
unresolvable-command one (`envelope_assertion_failed`) and NOT the one the verb \
would have earned — which is D-24's requirement that a refusal name the mechanism \
that produced it. Round 8's clause is raised at exactly that point today, measured: \
`git -c include.path=/tmp/evil.cfg push --force origin main` is exit 2 \
`envelope_assertion_failed` and NOT `force_push_blocked`, even though its VERB \
alone would have earned the latter. \
\n\n**THE MECHANISM, in git's own terms.** Git re-parses a non-`!` \
`alias.<name>` body as a git command line IN-PROCESS, INCLUDING its leading \
options, so the `-c` inside the VALUE is applied by git itself at command-line \
precedence. `alias.q` names the section `alias`, which is not an indirection, so \
`config_key_names_an_indirection_section` correctly answers `false` and round 8's \
clause CONFINES the assignment and lets it through. Measured against real git with \
the envelope's own injection as the control: the control alone prints `/ENV_WINS`, \
and `-c alias.a='-c include.path=<f> config --get core.hooksPath' a` prints \
`/INCLUDE_WINS`.";

/// The derivation every PERSISTED-delivery row shares.
const T_19_108_PERSISTED_DERIVATION: &str = "\
**THE DERIVATION.** `classify_config` ALREADY reads this exact operand with \
`is_hooks_path_key` and refuses `git config core.hooksPath /dev/null` at \
`hook_bypass_blocked` today — measured at this file's base commit — so asking the \
same question at the same operand is the SAME decision region and not a second \
reading site. `19-25`'s clause is raised there before the write is classified, so \
the identifier is `envelope_assertion_failed`. \
\n\n**AND THE PERSISTENCE ASYMMETRY, which `19-25` must not get backwards.** \
Audit 8 measured persisted `git config include.path <evil>` INERT at repo-local, \
`--worktree` and GLOBAL because the envelope's injection OUTRANKS it — re-confirmed \
in section 12 — so a clause for `git config include.path` would be INERT and must \
NOT be added. **An alias is different IN KIND: it does not have to WIN a precedence \
contest, it only has to EXIST.** Measured under the envelope's own \
`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` redirect at a generated helper-free file, \
BOTH the `--global` and the repo-local spellings resolve `/INCLUDE_WINS`.";

// ===========================================================================
// 0. The anti-vacuity controls — every one of these passes TODAY
// ===========================================================================

#[test]
fn the_unwrapped_refusals_still_fire_which_proves_this_harness_can_see_a_denial() {
    // If any row here is ever red, every refusal row in this file is red for a
    // reason that has nothing to do with a re-parsed config value — the harness
    // cannot observe a refusal at all.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "the plainest refusal in the whole envelope. A red here means the harness cannot see a \
         denial and every refusal assertion in this file is vacuous.",
    );
    refuses(
        "git config core.hooksPath /dev/null",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "the `git config` write site's own refusal, which is the decision region section 2's \
         derivation rests on: `classify_config` already reads this operand.",
    );
}

#[test]
fn the_force_push_compositions_are_already_refused_and_are_controls_not_reproducers() {
    // **A `T-19-108` reproducer must be built on a base LAYER 2 PERMITS.** These are
    // CONTROLS. A corpus built out of them would be GREEN before the fix and would
    // certify nothing — the NINTH consecutive instance of `T-19-76`'s failure mode,
    // produced by this corpus rather than found by the next audit.
    refuses(
        "git -c include.path=/tmp/evil.cfg push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "already refused for its CARRIER by round 8's clause — a control, not a reproducer",
    );
    refuses(
        "git -c a=b push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "already refused for its VERB — a control, not a reproducer. It is also the \
         verdict-preserving row that proves the dotless fence does not disarm layer 2.",
    );
}

#[test]
fn the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all() {
    // **Without this row every "the walk was EMPTY" assertion in this file is
    // vacuous.** Measured at this file's base commit: exactly ONE
    // `alpha/pr-ledger.ndjson` line in a walked fresh root.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "gh pr create --title x");
    assert_eq!(
        answer.code,
        0,
        "`gh pr create --title x` must be PERMITTED and COUNTED. stdout: {} stderr: {}",
        answer.stdout,
        answer.stderr
    );
    let written = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        written.len(),
        1,
        "\n\nTHE WALK IS BLIND.\n\nA permitted `gh pr create` writes exactly ONE pull-request \
         ledger line, and the walk must be able to find it. If this is 0 then every \
         empty-walk assertion in this file passes having observed nothing.\n\
         Walked listing:\n{}",
        listing(envelope.path())
    );
    println!(
        "POSITIVE WALK CONTROL — `gh pr create --title x` left {} ledger line(s):\n{}",
        written.len(),
        listing(envelope.path())
    );
}

// ===========================================================================
// 1. `T-19-108` in the `-c` DELIVERY, on LAYER-2-PERMITTED bases
//
// **RED against the pre-fix tree.** Every row measured exit 0 with an EMPTY walk
// at this file's base commit, and asserted at its DERIVED post-fix verdict.
// ===========================================================================

#[test]
fn after_19_25_a_reparsed_alias_value_carrying_an_indirection_is_refused() {
    // **`T-19-108`'S HEADLINE ROW, ON A SINGLE LINE.** Measured exit 0, walk EMPTY.
    // Section 13 measures the same class end to end and it MOVES a bare remote's
    // ref.
    refuses(
        "git -c alias.q=\"-c include.path=/tmp/evil.cfg push --force origin \
         HEAD:refs/heads/main\" q",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_108_LEADING_DERIVATION,
    );
    for base in ["status", "commit -m x", IN_NAMESPACE_PUSH] {
        refuses(
            &format!("git -c alias.q=\"-c include.path=/tmp/evil.cfg {base}\" q"),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            T_19_108_LEADING_DERIVATION,
        );
    }
}

#[test]
fn after_19_25_a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused() {
    // **The row that reaches the by-name `core.hooksPath` deny plan 19-02 built.**
    // `T-19-09` exists to enforce it and `cred.rs` names it as the one form 19-02
    // closes — and a value the guard confined carries it straight past, because the
    // string `core.hooksPath` is inside a VALUE the scan never re-reads.
    //
    // Measured exit 0 today. Its paired discriminator
    // `git -c core.hooksPath=/dev/null status` is exit 2 `hook_bypass_blocked`
    // (section 3), which is what makes this a gap in REACH and not in MECHANISM.
    refuses(
        &format!("git -c alias.z=\"-c core.hooksPath=/dev/null {IN_NAMESPACE_PUSH}\" z"),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_108_LEADING_DERIVATION,
    );
    refuses(
        "git -c alias.z=\"-c core.hooksPath=/dev/null status\" z",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_108_LEADING_DERIVATION,
    );
}

#[test]
fn after_19_25_the_case_varied_and_second_carrier_spellings_are_refused_too() {
    // **The CASE row.** Git folds the config SECTION to lower case, so `ALIAS.q`
    // names the same section as `alias.q`. A rule that compared the section
    // case-SENSITIVELY would leave this open, which is round 8's class-4 lesson one
    // region over.
    refuses(
        "git -c ALIAS.q=\"-c include.path=/tmp/evil.cfg status\" q",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_108_LEADING_DERIVATION,
    );

    // **The SECOND CARRIER, in its ATTACHED spelling.** Arm (b) of
    // `leading_git_option` returns the attached assignment, so it reaches the same
    // key check. Real git resolves it identically: measured `/INCLUDE_WINS` with
    // `BODYVAR` holding the body (section 12).
    //
    // **Its SEPARATE-WORD spelling is section 9's row and is RECORDED, not
    // asserted**, because whether `19-25` can read a first byte at all when the
    // value half is an environment variable NAME is a design decision this plan
    // cannot derive.
    refuses(
        "git --config-env=alias.q=BODYVAR status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        T_19_108_LEADING_DERIVATION,
    );
}

// ===========================================================================
// 2. `T-19-108` in the PERSISTED DELIVERY
//
// **RED against the pre-fix tree.** Every spelling measured exit 0, walk EMPTY.
// ===========================================================================

#[test]
fn after_19_25_a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling() {
    for command in [
        "git config alias.p '-c include.path=/tmp/evil.cfg push origin \
         refs/heads/gsd-auto/alpha/w'",
        "git config --global alias.p '-c include.path=/tmp/evil.cfg push'",
        "git config --worktree alias.p '-c include.path=/tmp/evil.cfg status'",
        "git config --file /tmp/c.cfg alias.p '-c include.path=/tmp/evil.cfg status'",
        "git config --add alias.p '-c include.path=/tmp/evil.cfg status'",
        "git config --replace-all alias.p '-c include.path=/tmp/evil.cfg status'",
        "git config alias.z '-c core.hooksPath=/dev/null status'",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            T_19_108_PERSISTED_DERIVATION,
        );
    }
}

#[test]
fn a_clause_for_a_persisted_include_path_would_be_inert_and_must_not_be_added() {
    // **The asymmetry recorded as a GUARD-SIDE fact, so `19-25` cannot spend a rule
    // on an inert clause.** Section 12 measures the git-side half: a persisted
    // `include.path` LOSES to the envelope's injection at every repository-side
    // level, while a persisted alias only has to EXIST.
    //
    // Nothing about `git config include.path <f>` moves in this round, and that is
    // deliberate rather than an omission. It is measured PERMITTED here and left
    // PERMITTED, because refusing it would be an over-refusal with no measured harm
    // behind it.
    permits(
        "git config include.path /tmp/evil.cfg",
        "**A PERSISTED `include.path` IS INERT** — measured in section 12 at repo-local and at \
         GLOBAL under the envelope's own config posture, it resolves `/ENV_WINS` because the \
         envelope's `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet OUTRANKS every repository-side \
         level. A clause here would refuse a command with no measured harm behind it. **A red \
         here means `19-25` added the inert clause the asymmetry exists to prevent.**",
    );
}

// ===========================================================================
// 3. The PAIRED DISCRIMINATORS, asserted UNCHANGED
//
// GREEN today and after. **These are what make this a gap in REACH rather than in
// MECHANISM**: the same two keys, delivered in a region the guard DOES read, are
// already refused at their own identifiers.
// ===========================================================================

#[test]
fn the_paired_discriminators_already_refuse_and_prove_the_mechanism_present() {
    refuses(
        "git -c include.path=/tmp/evil.cfg push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 8's confinement clause, reached because the carrier is in the LEADING region",
    );
    refuses(
        "git -c core.hooksPath=/dev/null status",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "D-09's by-name deny, reached because the key is in the LEADING region. The identical \
         key INSIDE a confined alias VALUE is exit 0 (section 1) — same key, same mechanism, \
         different REGION.",
    );
    refuses(
        "git config core.hooksPath /dev/null",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "`classify_config` reads the KEY OPERAND with `is_hooks_path_key`. **This is the \
         decision region section 2's derivation rests on** — asking the alias question there is \
         the same reading site, not a second one.",
    );
}

// ===========================================================================
// 4. The SHELL-ALIAS FENCE — the seam this plan exists to fence
//
// GREEN today and after, and the single most load-bearing test in this file.
// ===========================================================================

#[test]
fn a_shell_bodied_alias_stays_permitted_because_gits_rule_is_the_first_byte() {
    // **THE CONSEQUENCE, IN ONE SENTENCE: a rule that refused every `alias.*`
    // assignment would turn TWO EVIDENCE FILES PERMANENTLY RED, and `19-25` may
    // edit neither.**
    //
    //   tests/envelope_command_position.rs:550
    //     permits("git -c alias.p='!git push --force origin main' p")
    //   tests/envelope_config_resolution.rs:1539-1543
    //     permits("git config alias.p \"!git push --force origin HEAD:refs/heads/main\"")
    //
    // Both are registered `T-19-86` rows, OPEN at `high` by explicit user scoping
    // decision. This is `19-18`'s `{v}>` blocker and `19-22`'s `-c a=b` seam one
    // REGION over.
    //
    // **The carve-out is git's own rule, measured in section 12 in both directions
    // and at its boundary**: a body whose FIRST BYTE is `!` is handed to a SHELL and
    // run as a CHILD that INHERITS the envelope's injection (measured `/ENV_WINS`,
    // with the child's own environment dumped in section 13), while a non-`!` body
    // is re-parsed IN-PROCESS at command-line precedence (measured
    // `/INCLUDE_WINS`).
    permits(
        "git -c alias.p='!git push --force origin main' p",
        "**`T-19-86` ROW — mirrors `tests/envelope_command_position.rs:550`, which this round \
         MAY NOT EDIT.** A `!` body is a shell command line run in a CHILD that inherits the \
         envelope's `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet, so layer 3 is intact for it \
         — measured `/ENV_WINS`. A red here is `19-25` having written a BLANKET `alias.*` \
         refusal, which would also turn that file permanently red.",
    );
    permits(
        "git config alias.p \"!git push --force origin HEAD:refs/heads/main\"",
        "**`T-19-86` ROW — mirrors `tests/envelope_config_resolution.rs:1539-1543`, which this \
         round MAY NOT EDIT.** The persisted arm of the same one-byte fact.",
    );
    permits(
        "git -c alias.b='!git status' b",
        "the same one-byte fact on a permitted inner verb, so the fence cannot be satisfied by \
         a rule that happens to key off `--force`",
    );

    // -- **THE STANDING ASSERTION THE WHOLE RULE LEANS ON.** No measured spelling
    //    exists in which the FIRST BYTE IS `!` and git nonetheless re-parses
    //    in-process. Nine spellings were driven against real git while planning, at
    //    plan-check and again at this file's base commit (section 12). **A
    //    counterexample is a FINDING to report, not a row to add.**
    //
    //    This is asserted as a property of the fence's own alphabet rather than as
    //    prose: every command this file pins PERMITTED for the shell-alias reason
    //    carries a body whose first byte IS `!`, and every command it pins REFUSED
    //    for the re-parse reason carries one whose first byte is NOT.
    for permitted_body in [
        "!git push --force origin main",
        "!git push --force origin HEAD:refs/heads/main",
        "!git status",
    ] {
        assert!(
            permitted_body.starts_with('!'),
            "\n\nTHE SHELL-ALIAS FENCE HAS LOST ITS SUBJECT.\n\n\
             `{permitted_body}` is pinned PERMITTED in this test for the SHELL-ALIAS reason, so \
             its first byte MUST be `!`. Git's rule is the first byte and nothing else, \
             measured in nine spellings in section 12.\n\n\
             **NO SPELLING WAS FOUND IN WHICH THE FIRST BYTE IS `!` AND GIT NONETHELESS \
             RE-PARSES IN-PROCESS.** The whole rule leans on that. Finding one is a FINDING to \
             report, not a row to add."
        );
    }
    for refused_body in [
        "-c include.path=/tmp/evil.cfg status",
        "config --get core.hooksPath !x",
        "\"!git -c include.path=/tmp/evil.cfg push --force origin HEAD:refs/heads/main\"",
        "\t!git -c include.path=/tmp/evil.cfg status",
        "status",
    ] {
        assert!(
            !refused_body.starts_with('!'),
            "\n\n`{refused_body}` is pinned REFUSED in this file for the RE-PARSE reason, so its \
             first byte must NOT be `!`. If it is, the corpus is asserting a refusal of a body \
             git hands to a SHELL — which is `T-19-86`, out of scope, and whose two rows are \
             pinned PERMITTED in files this round may not edit."
        );
    }
}

#[test]
fn after_19_25_the_three_non_shell_boundary_spellings_are_refused() {
    // **The RED half of the fence, kept in a test OF ITS OWN so the fence's own
    // permitted half stays GREEN TODAY.** A fence whose green assertions were
    // bundled with post-fix red ones could not be read as evidence that the fence
    // held: its failure output would look identical either way.
    //
    // All three bodies below have a first byte that is NOT `!`, so `19-25`'s rule
    // refuses them — and in all three cases that refusal is CORRECT or costless,
    // which is why the boundary is safe to draw at one byte.

    // -- A `!` that is NOT first is not a shell body. Measured `/ENV_WINS` against
    //    real git, because git re-parses this IN-PROCESS and the trailing `!x` is
    //    just an extra argument.
    refuses(
        "git -c alias.g=\"config --get core.hooksPath !x\" g",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**A `!` THAT IS NOT THE FIRST BYTE IS NOT A SHELL BODY** — measured `/ENV_WINS` against \
         real git, because git re-parses this IN-PROCESS. The rule reads the FIRST byte and \
         nothing else, so this is refused and that is correct.",
    );

    // -- **THE QUOTED-BODY ROW: the case that shows reading ONE BYTE is not a
    //    loophole.** Its first byte is `"`, not `!`. Git re-parses it IN-PROCESS —
    //    measured `expansion of alias 'q' failed; '!git -c include.path=<f> …' is not
    //    a git command`, i.e. git's own `split_cmdline` dequoted it into a single
    //    word and looked for a GIT command, never a shell. So the rule REFUSES it,
    //    and that refusal is CORRECT.
    refuses(
        "git -c alias.q='\"!git -c include.path=/tmp/evil.cfg push --force origin \
         HEAD:refs/heads/main\"' q",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE QUOTED-BODY ROW.** Measured against real git: the first byte is `\"`, git \
         re-parses the body IN-PROCESS with its own `split_cmdline`, and its error names the \
         whole dequoted string as not a git command — it was never handed to a shell. A rule \
         reading the first byte refuses this, and **that is why reading ONE byte is not a \
         loophole**: the quoting that would smuggle a `!` past a naive check also takes the body \
         out of the shell path entirely.",
    );

    // -- **THE TAB SPELLING.** Git refuses to EXPAND it at all — measured
    //    `expansion of alias 't' failed; '' is not a git command`. The first byte is
    //    a TAB, not `!`, so the rule refuses the definition; the disclosed cost of
    //    that is refusing an alias git would itself have refused to run.
    refuses(
        "git -c alias.t='\t!git -c include.path=/tmp/evil.cfg status' t",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE TAB SPELLING.** Measured against real git: a TAB (like a SPACE) before `!` makes \
         git refuse to expand the alias at all — `expansion of alias 't' failed; '' is not a git \
         command`. The first byte is not `!`, so the rule refuses the definition. The cost is \
         refusing a body git would itself have refused to run.",
    );
}

#[test]
fn audit_7s_shell_bodied_destructive_pair_still_works_after_this_round_and_is_not_closed() {
    // **THE HONEST HALF, STATED IN THE SAME TEST FAMILY AS THE FENCE.**
    //
    // Because the `!` body stays permitted, audit 7's destructive pair —
    //
    //   git config alias.q '!git -c include.path=<evil> push --force origin HEAD:refs/heads/main'
    //   git q
    //
    // — which rewrote a bare remote's `main` from `ac303dc` to `9f62444` at
    // `fb43577`, **STILL WORKS AFTER THIS ROUND.** Section 13 re-measures it
    // against a rebuilt bare remote and records the ref moving.
    //
    // That is `T-19-86`: a whole command line handed to a governed program as DATA.
    // It is OPEN at `high`, out of scope by explicit user decision, and **this round
    // does not close, narrow or re-scope it.** `T-19-108` closes only AS SCOPED —
    // the non-shell alias body — and audit 8's own boundary paragraph lists the
    // re-parsed config value and the command-line-as-data as two SEPARATE bullets.
    permits(
        "git config alias.q '!git -c include.path=/tmp/evil.cfg push --force origin \
         HEAD:refs/heads/main'",
        "**`T-19-86`, NOT CLOSED.** Audit 7's destructive pair, first leg. It stays at exit 0 \
         after this round by explicit user scoping decision, and section 13 measures that it \
         still moves a bare remote's ref.",
    );
    permits(
        "git q",
        "**`T-19-86`, NOT CLOSED.** The second, separately-permitted tool call. The guard is \
         stateless and argv-only and cannot correlate the two.",
    );
}

// ===========================================================================
// 5. The DISCRIMINATION CONTROLS — this round's `--signed no`
//
// GREEN today and after. **A rule written as `key.starts_with("alias")` turns both
// of the first two rows RED; only a rule that compares the SECTION keeps them
// green.**
// ===========================================================================

#[test]
fn a_near_miss_key_that_names_no_alias_section_stays_permitted_before_and_after() {
    permits(
        "git -c aliasx.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "**THIS ROUND'S `--signed no`, from one side.** `aliasx` is a SECTION of its own — git \
         re-parses nothing from it — so this must stay at exit 0. A rule written as \
         `key.starts_with(\"alias\")` turns it red, and only a SECTION comparison keeps it \
         green.",
    );
    permits(
        "git -c notalias.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "**THIS ROUND'S `--signed no`, from the other side.** A rule written as \
         `key.contains(\"alias\")` turns this red. Only a SECTION comparison keeps it green.",
    );

    // -- THE DOTLESS ROWS, carried into this round because
    //    `CALLEE_KNOWN_LEADING_PREFIX` splices round 7's WHOLE generative property
    //    behind `-c a=b`. A rule that refused a key it cannot decompose into a
    //    section would turn that property permanently red in a file `19-25` may not
    //    edit.
    permits(
        "git -c a=b status",
        "**THE DOTLESS FENCE.** `CALLEE_KNOWN_LEADING_PREFIX` at \
         `tests/envelope_wrapper_class.rs:5197` is `\"-c a=b\"` and round 7's ENTIRE \
         callee-grammar generative property is spliced behind it. Real git RUNS `git -c a=b \
         version` at rc 0 and errors only when something READS the key — measured. A dotless \
         key names NO SECTION, so it can never name `alias`.",
    );
    permits(
        "git -c a=b version",
        "the same fence on the verb real git was measured running at rc 0",
    );
}

// ===========================================================================
// 6. The DISCLOSED OVER-REFUSAL FAMILY — every row beside its PERMITTED TWIN
//
// **The round's whole cost, disclosed by the corpus that produced it rather than
// found by audit 9.** Every refused row is a DEFINITION of a non-shell alias;
// every permitted twin does the same work; and every INVOCATION row is UNCHANGED.
// ===========================================================================

#[test]
fn after_19_25_defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not() {
    // **THE ROUND'S COST IN ONE SENTENCE: DEFINING a non-shell alias is refused;
    // USING one is not, and every refused form has a permitted twin that does the
    // same work.**
    //
    // Ordinary aliases do NOT keep working, and saying so plainly is the point. All
    // four rows below are measured exit 0 TODAY.
    for (refused, twin, twin_why) in [
        (
            "git -c alias.st=status st",
            "git status",
            "the twin that does the same work",
        ),
        (
            "git -c alias.lg=\"log --oneline\" lg",
            "git log --oneline",
            "the twin that does the same work",
        ),
        (
            "git -c alias.co=checkout co",
            "git checkout",
            "the twin that does the same work",
        ),
        (
            "git config alias.co checkout",
            "git checkout",
            "the persisted form's twin: simply run the command the alias would have run",
        ),
    ] {
        refuses(
            refused,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "**THE DISCLOSED COST.** An ORDINARY alias body is re-parsed by git in-process \
             exactly as a carrier-bearing one is, and the guard cannot establish what the \
             re-parsed line will resolve `core.hooksPath` to without re-entering its own \
             scan. Refusing the whole non-shell class is the safe direction, and this row \
             DISCLOSES the cost rather than leaving it for audit 9. Its permitted twin is \
             pinned in the same test.",
        );
        permits(twin, twin_why);
    }
}

#[test]
fn invoking_an_already_defined_alias_is_unchanged_because_the_guard_is_stateless() {
    // **THE BOUNDARY THAT MAKES THE COST LEGIBLE.** The guard is stateless and
    // argv-only: it cannot see an alias it did not watch being defined. So
    // INVOCATION does not move, in either direction, and a rule that refused these
    // would be refusing every one-word git subcommand it does not recognise.
    for invocation in ["git p", "git co", "git st", "git lg", "git z", "git q"] {
        permits(
            invocation,
            "**INVOCATION IS NOT DEFINITION.** The guard is stateless and argv-only and cannot \
             correlate a definition in one tool call with a use in another. A red here would \
             mean `19-25` started refusing unrecognised git subcommands, which is a far wider \
             blast radius than the class this round is about — and it would ALSO be the \
             `T-19-86` persisted-alias arm being closed by accident.",
        );
    }
}

// ===========================================================================
// 7. The PERMITTED HALF carried forward from round 8, asserted UNCHANGED
//
// GREEN today and after. **Without this section, an implementation that simply
// refused every governed command carrying a `-c` would satisfy every fail-closed
// row above** — while making a driven run unusable, which is how a safety control
// gets switched off (AR-19-11).
// ===========================================================================

#[test]
fn the_permitted_half_of_the_config_axis_stays_permitted() {
    for (command, why) in [
        (
            "git -c user.name=\"Alpha Beta\" commit -m x",
            "**this axis's `ls {git,svn}-repo`.** Setting `user.name` on the command line is \
             exactly what a driven run does to make its commits attributable.",
        ),
        ("git -c core.pager=cat log", "a K2 key, measured INERT for layer 3 in section 13"),
        (
            "git -c includepath=/tmp/evil.cfg status",
            "round 8's discrimination control — a DOTLESS key naming no section at all",
        ),
        (
            "git -c notinclude.path=/tmp/evil.cfg status",
            "round 8's discrimination control — a section that pulls nothing in",
        ),
        ("git --git-dir=/tmp/g status", "an ordinary leading option"),
        ("git -C /tmp status", "an ordinary leading option with a separate value"),
        ("git --no-pager status", "round 7's `ls {git,svn}-repo`"),
    ] {
        permits(command, why);
    }

    // The verdict-PRESERVING refusals: their identifier must not move.
    refuses(
        "git -c a=b push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "a confined dotless assignment does not change a verdict — the base's own identifier",
    );
    refuses(
        "git -c a=b --attr-source HEAD push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "and it composes with round 7's grammar without changing the identifier",
    );
}

// ===========================================================================
// 8. The two ORDERING PINS, at deliberately DIFFERENT post-fix identifiers
//
// Together they are the mechanical proof that `19-25`'s clause is raised inside
// the ONE left-to-right walk and not in a second pass, exactly as round 8's pair
// is for the confinement clause. **Both are exit 2 `hook_bypass_blocked` TODAY**,
// so the FIRST is RED without ever having been exit 0.
// ===========================================================================

#[test]
fn after_19_25_the_scan_order_decides_which_clause_names_a_line_carrying_both() {
    refuses(
        "git -c alias.q=\"-c include.path=/tmp/evil.cfg status\" -c core.hooksPath=/dev/null \
         push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE ALIAS ASSIGNMENT COMES FIRST**, and `scan_leading` is a LEFT-TO-RIGHT walk whose \
         clause is raised at the first assignment it cannot bound. Measured at \
         `hook_bypass_blocked` today — this row is RED without ever having been exit 0, because \
         its IDENTIFIER moves while its verdict does not. **A red here after `19-25` means the \
         clause moved into a SECOND PASS over the leading tokens**, which would also collapse \
         this pin and its twin below into the same identifier.",
    );
    refuses(
        "git -c core.hooksPath=/dev/null -c alias.q=\"-c include.path=/tmp/evil.cfg status\" \
         push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
        "**THE HOOKS KEY COMES FIRST**, so this row is UNCHANGED. If both pins ever report the \
         same identifier, the single left-to-right walk has been replaced by a pass that \
         prioritises one clause over the other regardless of position.",
    );
}

// ===========================================================================
// 9. The two rows RECORDED rather than asserted
//
// **Measured and RECORDED IN A COMMENT AND A PRINT — never written as an
// assertion.** This plan measures PRE-fix and cannot catch a wrong POST-fix
// expectation; a row pinned at a verdict the rules cannot produce lands RED with
// `19-25` forbidden to edit it. **`19-22` asserted such a row against its own
// comment, its own SUMMARY and its own plan-check, and it halted `19-23`
// mid-plan.** Asserting only that they are REFUSED is also forbidden.
// ===========================================================================

#[test]
fn the_two_undeliverable_rows_are_recorded_here_and_left_for_19_25_to_pin() {
    // -- ROW 1: the SEPARATE-WORD `--config-env` spelling. Arm (a) of
    //    `leading_git_option` returns the NEXT token as the assignment, so the key
    //    half reaches the same check as the attached spelling. But the VALUE half is
    //    an environment variable NAME rather than a body, so **whether `19-25` can
    //    read a first byte at all here is a design decision**: the body lives in the
    //    environment, which the guard does not read. Real git resolves it — measured
    //    `/INCLUDE_WINS` through BOTH spellings in section 12 — so the harm is real;
    //    only the identifier is undeliverable.
    //
    //    Measured at this file's base commit: exit 0, walk EMPTY.
    record_only(
        "--config-env SEPARATE-WORD alias delivery: value half is an ENV VAR NAME",
        None,
        "git --config-env alias.q=BODYVAR status",
    );

    // -- ROW 2: `-c alias.q` with NO `=` at all. `config_key_of` returns the WHOLE
    //    token when there is no `=`, so there is NO VALUE HALF to read a first byte
    //    from. Real git accepts it (treats the key as boolean true) and runs the
    //    rest of the line — measured `/ENV_WINS` in section 12, i.e. NO harm. So
    //    **which clause raises the fail-closed answer, and whether one is raised at
    //    all, is a `19-25` measurement.**
    //
    //    Measured at this file's base commit: exit 0, walk EMPTY.
    record_only(
        "-c alias.q with NO `=`: config_key_of returns the whole token, no value half",
        None,
        "git -c alias.q status",
    );
}

// ===========================================================================
// 10. The `T-19-86` rows — RECORDED WIDER and explicitly NOT closing anything
// ===========================================================================

#[test]
fn the_registered_t_19_86_rows_keep_exiting_zero_and_are_recorded_wider_not_closed() {
    // **`T-19-86` is OPEN at `high` by explicit user scoping decision.** Recording
    // it does NOT close, narrow or re-scope it, and this round adds no remedy.
    //
    // **AUDIT 8 FOUND IT MEASURABLY WIDER THAN THE REGISTER CREDITS, AND THIS FILE
    // RECORDS THAT WITHOUT CLOSING IT.** Three documents attribute a layer-3 catch
    // to the closure of `T-19-103`: the `pre-push` hook fires when the alias body
    // runs, so the inner push is caught. **That catch is ABSENT when the alias body
    // carries a carrier of its own** — `!git -c include.path=<evil> push --force …`
    // runs in a child that inherits the injection AND then applies its own
    // command-line carrier on top of it, which outranks the injection inside that
    // child. Section 13 measures the pair moving a bare remote's ref, at this
    // round's base commit, after `19-23` landed.
    for command in [
        "git submodule foreach git push --force origin main",
        "git rebase -x \"git push --force origin main\" HEAD~3",
        "git bisect run sh -c \"git push --force origin main\"",
        "git -c alias.p='!git push --force origin main' p",
    ] {
        permits(
            command,
            "**`T-19-86` REGISTERED ROW — must keep exiting 0.** This plan does not fix, close, \
             narrow or re-scope `T-19-86`, and a red here would be this round having silently \
             taken it on. It is named FIRST in the SUMMARY's 'what remains uncovered' section.",
        );
    }
}

// ===========================================================================
// 11. The SIX carried-forward MECHANISM pins — GREEN today and after
//
// Re-asserted here over the same PUBLIC functions rather than moved or edited in
// place, because rounds 4, 5, 6, 7 and 8's evidence stays attributable to the
// round that produced it. **This round touches the `-c` VALUE for the first time
// in the phase, so all six are re-stated in its own evidence.**
// ===========================================================================

#[test]
fn rule_b_still_reports_a_severed_head_as_not_a_command_position() {
    // A verdict pin cannot replace this. If Rule B stopped firing, the lines below
    // would STILL be refused — by `resolve_program` step 5's prefix rule — and every
    // verdict pin in the suite would stay green while Rule B quietly became dead
    // code.
    for command in [
        "C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin",
        "K=GIT_SSH; env -u ${K}_COMMAND git fetch origin",
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
        "the first command INSIDE a brace group is a command position. If this is false, Rule B \
         has been widened to closers-and-openers and `{{ git status; }}` is refused — a control \
         failing into unusability. Segments: {segments:?}"
    );
}

#[test]
fn round_5s_literalness_bit_is_non_vacuous_and_is_right_about_every_word_of_the_bypass_line() {
    // **The pin that stops `19-25` from closing `T-19-108` by making round 5's bit
    // WRONG about a word it is RIGHT about.**
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
        "\n\nROUND 5'S LITERALNESS BIT HAS GONE VACUOUS.\n\n`pus?` is a glob in the git VERB \
         slot, so `Token.literal` must be FALSE for it.\n  tokens: {unreadable_tokens:?}"
    );
    refuses(
        "git pus? --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 5's inversion, asserted at the verdict as well as at the bit",
    );

    // Every word of the bypass line is written exactly as the shell hands it over,
    // and `Token.literal` is correctly `true` for all of them. **The bit is RIGHT
    // about the entire bypass line and the harm has nothing to do with
    // literalness**: the guard read a correct argv, found the correct verb, decided
    // correctly that the assignment was confined — and then handed git a VALUE it
    // never re-read.
    let bypass =
        policy::split_segments_with_heads("git -c alias.q=\"-c include.path=/tmp/evil.cfg status\" q")
            .expect("the bypass line splits into segments");
    let bypass_tokens: Vec<(String, bool)> = bypass
        .iter()
        .flat_map(|s| s.tokens.iter())
        .map(|t| (t.text.clone(), t.literal))
        .collect();
    for word in ["-c", "q"] {
        assert!(
            bypass_tokens.iter().any(|(text, _)| text == word),
            "`{word}` must be present as a token, or the assertion below certifies nothing. \
             tokens: {bypass_tokens:?}"
        );
    }
    for (text, literal) in &bypass_tokens {
        assert!(
            *literal,
            "\n\nTHE INVERSION WAS FALSIFIED INSTEAD OF THE RE-PARSED VALUE BEING MODELLED.\n\n\
             `{text}` is a fully LITERAL word on `T-19-108`'s bypass line and `Token.literal` is \
             correctly `true` for it. If this bit is now `false`, `19-25` closed `T-19-108` by \
             making round 5's bit WRONG about a word it was right about — the inversion becoming \
             dead code with every verdict pin green.\n\n\
             **The correct shape of the fix is to model what git RE-PARSES from the VALUE.**\n\
             \n  tokens: {bypass_tokens:?}"
        );
    }
}

#[test]
fn round_6s_deletion_model_is_non_dead_and_this_round_must_not_remove_it() {
    refuses(
        "git >/dev/null push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "round 6's deletion model: the redirection is deleted, `push` is the verb",
    );
    // The OVER-DELETION control: `x2>/tmp/o` is a REAL argv word (bash treats `x2`
    // as an ordinary word, not an IO_NUMBER), so it must NOT be deleted. This is
    // round 6's `ls {git,svn}-repo` and it must stay PERMITTED.
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
    assert!(
        !policy::is_separator(">"),
        "`>` must NOT become a separator. Splitting there would hide the command being \
         redirected, which is worse than the defect it would close."
    );
    assert!(!policy::is_separator("<"), "`<` must NOT become a separator");
    assert!(!policy::is_separator(">>"), "`>>` must NOT become a separator");
    // Non-vacuity: the predicate is not constant-false.
    assert!(
        policy::is_separator("&"),
        "`&` IS a separator — without this the three assertions above could all pass because \
         `is_separator` answers `false` for everything"
    );
    assert!(policy::is_separator("&&"), "`&&` IS a separator");
    assert!(policy::is_separator("|"), "`|` IS a separator");
}

#[test]
fn round_7s_fail_closed_callee_grammar_is_non_dead_and_this_round_must_not_remove_it() {
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
    permits(
        "git --no-pager status",
        "round 7's `ls {git,svn}-repo` — a self-contained option must stay permitted",
    );
    permits(
        "git - push --force origin main",
        "`scan_leading` breaks on a bare `-` before any option check, and round 7 is forbidden \
         to widen into non-`-`-prefixed words",
    );
    refuses(
        "git -- push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
        "and a bare `--` breaks the scan with `push` still the verb",
    );
}

#[test]
fn round_8s_confinement_clause_is_non_dead_and_this_round_must_not_remove_it() {
    // **The pin that turns red if `19-25` regresses round 8.** Round 9's whole
    // finding is that this clause is the RIGHT SHAPE of rule and reaches only two of
    // the three regions — so a round-9 rule that replaced it rather than extending
    // its reach would be trading one open class for another.
    refuses(
        "git -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 8's confinement clause on a PERMITTED base: an assignment the guard cannot bound \
         to the key it names makes the command unresolvable",
    );
    // The two DISCRIMINATION CONTROLS, so the pin cannot be satisfied by a blanket
    // refusal of everything spelled `-c`.
    permits(
        "git -c includepath=/tmp/evil.cfg status",
        "round 8's discrimination control — a dotless key names no section",
    );
    permits(
        "git -c notinclude.path=/tmp/evil.cfg status",
        "round 8's discrimination control — the SECTION is what decides, not the letters",
    );
}

// ===========================================================================
// 12. The REAL-GIT probes — measured with the envelope's OWN injection as the
//     control
//
// GREEN today and after. **This is the step that separates a re-parse claim from a
// guess**, and it is asserted rather than merely recorded because a claim in this
// codebase about which forms outrank the injection has been WRONG TWICE.
// ===========================================================================

/// Run real git with the envelope's OWN `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`
/// injection in the environment, plus any extra environment pairs, and return
/// whatever the command printed (trimmed), stdout and stderr merged.
///
/// The injection is spelled exactly as `cred::hooks_path_env` emits it — count
/// DERIVED from one pair, `KEY_0`, `VALUE_0` — so this is a probe of the REAL
/// control rather than of a stand-in. **stderr is merged deliberately**: the
/// one-byte boundary rows are measured by git's own REFUSAL text, and a probe that
/// discarded stderr could not tell "git refused to expand" from "git printed
/// nothing".
fn git_says(cwd: &Path, extra_env: &[(&str, &str)], args: &[&str]) -> String {
    let mut command = std::process::Command::new("git");
    command
        .current_dir(cwd)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/ENV_WINS");
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let out = command.args(args).output().expect("git is on PATH");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text.trim().to_string()
}

#[test]
fn real_git_reparses_a_non_shell_alias_body_in_process_including_its_leading_options() {
    // **K1, MEASURED. This is `T-19-108`'s whole mechanism.**
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);

    let evil = dir.path().join("evil.cfg");
    std::fs::write(&evil, "[core]\n\thooksPath = /INCLUDE_WINS\n").unwrap();
    let evil = evil.to_str().unwrap().to_string();

    let alias_cfg = dir.path().join("alias.cfg");
    std::fs::write(&alias_cfg, "[core]\n\thooksPath = /ALIAS_WINS\n").unwrap();
    let alias_cfg = alias_cfg.to_str().unwrap().to_string();

    // -- THE CONTROL. If this is not `/ENV_WINS`, every row below is comparing
    //    against nothing and the whole probe is vacuous.
    assert_eq!(
        git_says(&repo, &[], &["config", "--get", "core.hooksPath"]),
        "/ENV_WINS",
        "THE CONTROL. The envelope's own `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet must be \
         what git resolves when nothing else is present. A carrier is a re-parse carrier only \
         if it makes this same command print something else."
    );

    // -- K1: a NON-`!` alias body is re-parsed IN-PROCESS **including its leading
    //    options**, so the `-c` inside the VALUE is applied by git itself at
    //    COMMAND-LINE precedence.
    assert_eq!(
        git_says(
            &repo,
            &[],
            &[
                "-c",
                &format!("alias.a=-c include.path={evil} config --get core.hooksPath"),
                "a",
            ],
        ),
        "/INCLUDE_WINS",
        "\n\n**K1, THE WHOLE OF `T-19-108`.** A non-`!` `alias.<name>` body must be re-parsed by \
         git IN-PROCESS as its own command line INCLUDING its leading options, so a \
         `-c include.path=<f>` inside the VALUE outranks the envelope's injection.\n\n\
         `alias.a` names the section `alias`, which is NOT an indirection, so \
         `config_key_names_an_indirection_section` correctly answers `false` and round 8's \
         clause CONFINES the assignment and lets it through. The carrier rides inside the VALUE \
         into a position `scan_leading` never reads. **If this row ever prints `/ENV_WINS`, \
         `T-19-108` is not the class this file says it is.**"
    );

    // -- and at the OTHER key: the alias body can carry `core.hooksPath` directly,
    //    reaching the by-name deny plan 19-02 built.
    assert_eq!(
        git_says(
            &repo,
            &[],
            &[
                "-c",
                "alias.z=-c core.hooksPath=/ALIAS_WINS config --get core.hooksPath",
                "z",
            ],
        ),
        "/ALIAS_WINS",
        "the alias body reaches the BY-NAME `core.hooksPath` deny directly — the string is on a \
         command line git itself assembles, and never on the one the guard read"
    );
    assert_eq!(
        git_says(
            &repo,
            &[],
            &[
                "-c",
                &format!("alias.w=-c include.path={alias_cfg} config --get core.hooksPath"),
                "w",
            ],
        ),
        "/ALIAS_WINS",
        "and through an include of its own, at the same precedence"
    );

    // -- **THE ONE-BYTE BOUNDARY, MEASURED IN NINE SPELLINGS.** This is the fence
    //    section 4 rests on, and the reason a blanket `alias.*` refusal is
    //    undischargeable.
    assert_eq!(
        git_says(&repo, &[], &["-c", "alias.b=!git config --get core.hooksPath", "b"]),
        "/ENV_WINS",
        "\n\n**K2 — A `!` BODY RUNS IN A CHILD THAT INHERITS THE INJECTION.** This is git's own \
         documented rule and it is a ONE-BYTE fact. Layer 3 is intact for a `!` body, which is \
         why `tests/envelope_command_position.rs:550` and \
         `tests/envelope_config_resolution.rs:1539-1543` both pin one PERMITTED as a registered \
         `T-19-86` row — and why a rule that refused every `alias.*` assignment would turn both \
         PERMANENTLY RED in files `19-25` may not edit."
    );
    assert_eq!(
        git_says(
            &repo,
            &[],
            &["-c", "alias.g=config --get core.hooksPath !x", "g"],
        ),
        "/ENV_WINS",
        "a `!` that is NOT the first byte is not a shell body: git re-parses this IN-PROCESS, \
         and the trailing `!x` is simply an extra argument"
    );
    for (label, body) in [
        ("SPACE before `!`", " !git config --get core.hooksPath"),
        ("TAB before `!`", "\t!git config --get core.hooksPath"),
        ("EMPTY body", ""),
    ] {
        let said = git_says(&repo, &[], &["-c", &format!("alias.d={body}"), "d"]);
        assert!(
            said.contains("expansion of alias 'd' failed"),
            "\n\n**THE BOUNDARY ROW `{label}`.** Git must REFUSE TO EXPAND this alias at all. \
             A leading SPACE or TAB (or an empty body) makes git's own `split_cmdline` produce \
             an empty first word. Got: {said:?}"
        );
    }
    let quoted = git_says(
        &repo,
        &[],
        &[
            "-c",
            &format!("alias.q=\"!git -c include.path={evil} config --get core.hooksPath\""),
            "q",
        ],
    );
    assert!(
        quoted.contains("expansion of alias 'q' failed") && quoted.contains("!git"),
        "\n\n**THE QUOTED-BODY ROW — THE ONE THAT SHOWS READING ONE BYTE IS NOT A LOOPHOLE.**\n\n\
         The body's first byte is `\"`, not `!`. Git re-parses it IN-PROCESS with its own \
         `split_cmdline`, which DEQUOTES it into a single word and then looks for a GIT command \
         of that name — **it is never handed to a shell.** So a rule reading the first byte \
         REFUSES this, and that refusal is CORRECT: the quoting that would smuggle a `!` past a \
         naive check also takes the body out of the shell path entirely.\n\n\
         Got: {quoted:?}"
    );
    let no_verb = git_says(
        &repo,
        &[],
        &["-c", &format!("alias.o=-c include.path={evil}"), "o"],
    );
    assert!(
        no_verb.contains("empty alias for o"),
        "a body that is ONLY an option with no verb is `fatal: empty alias for o`. Got: {no_verb:?}"
    );

    // -- **THE SECOND CARRIER**, in BOTH spellings, whose value is an environment
    //    variable NAME rather than a body. Real git resolves both identically.
    let body = format!("-c include.path={evil} config --get core.hooksPath");
    for args in [
        vec!["--config-env=alias.q=BODYVAR", "q"],
        vec!["--config-env", "alias.q=BODYVAR", "q"],
    ] {
        assert_eq!(
            git_says(&repo, &[("BODYVAR", &body)], &args),
            "/INCLUDE_WINS",
            "`{args:?}` must resolve the ALIAS body's own carrier. The value half is an \
             environment variable NAME — which is exactly why section 9 RECORDS the \
             separate-word guard row rather than asserting it: the body lives in the \
             environment, which the guard does not read."
        );
    }

    // -- **DEPTH 2 AND DEPTH 3**, so `19-25` costs a recursive rule against
    //    measurement rather than against an argument. **Depth 3 is the row that
    //    matters**: the recursion is not a depth-2 curiosity, so any stated depth
    //    bound is a residue an attacker reaches by adding one more nesting level.
    let inner = format!("alias.inner=-c include.path={evil} config --get core.hooksPath");
    assert_eq!(
        git_says(
            &repo,
            &[],
            &["-c", &format!("alias.d2=-c \"{inner}\" inner"), "d2"],
        ),
        "/INCLUDE_WINS",
        "DEPTH 2: an alias body can itself carry `-c alias.<n>=…` and git re-parses that too"
    );
    assert_eq!(
        git_says(
            &repo,
            &[],
            &[
                "-c",
                &format!("alias.d3=-c \"alias.d2=-c \\\"{inner}\\\" inner\" d2"),
                "d3",
            ],
        ),
        "/INCLUDE_WINS",
        "\n\n**DEPTH 3 — THE ROW THAT MATTERS.** The recursion is not a depth-2 curiosity. Any \
         depth bound `19-25` states in a recursive rule is a RESIDUE an attacker reaches by \
         adding one more nesting level, and that cost must be weighed against measurement rather \
         than against an argument."
    );

    // -- **QUOTING: git splits an alias body with its OWN rules, not the shell's**,
    //    with the negative control that proves the dequoting really happened.
    assert!(
        git_says(&repo, &[], &["config", "--get", "\"core.hooksPath\""])
            .contains("invalid key"),
        "THE DEQUOTING CONTROL: a LITERALLY quoted key is an `invalid key` error to git"
    );
    assert_eq!(
        git_says(
            &repo,
            &[],
            &["-c", "alias.m=config --get \"core.hooksPath\"", "m"],
        ),
        "/ENV_WINS",
        "and yet through an ALIAS BODY the same quoted key RESOLVES — so git applied its own \
         `split_cmdline` dequoting to the body. `19-25` cannot assume the shell's rules."
    );

    // -- **THE NO-`=` SPELLING**, section 9's second recorded row, measured HARMLESS
    //    on git's side: git accepts a dotted key with no value and runs the rest.
    assert_eq!(
        git_says(&repo, &[], &["-c", "alias.q", "config", "--get", "core.hooksPath"]),
        "/ENV_WINS",
        "`-c alias.q` with NO `=` carries NO value half at all, and real git resolves the \
         control value — **no harm on git's side**, which is why section 9 records the guard row \
         rather than asserting a refusal for it"
    );
}

#[test]
fn a_persisted_alias_only_has_to_exist_while_a_persisted_include_must_win_and_loses() {
    // **THE PERSISTENCE ASYMMETRY — this round's sharpest measured fact, and the one
    // `19-25` must not get backwards.**
    let dir = TempDir::new().unwrap();
    let evil = dir.path().join("evil.cfg");
    std::fs::write(&evil, "[core]\n\thooksPath = /INCLUDE_WINS\n").unwrap();
    let evil = evil.to_str().unwrap().to_string();

    // The envelope's OWN config posture: `cred::write_gitconfig` points BOTH
    // `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` at one generated helper-free file,
    // which is why an alias written at `--global` under the envelope is LIVE.
    let generated = dir.path().join("generated.cfg");
    std::fs::write(&generated, "[credential]\n").unwrap();
    let generated = generated.to_str().unwrap().to_string();

    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);

    let posture: Vec<(&str, &str)> = vec![
        ("GIT_CONFIG_GLOBAL", generated.as_str()),
        ("GIT_CONFIG_SYSTEM", generated.as_str()),
    ];

    assert_eq!(
        git_says(&repo, &posture, &["config", "--get", "core.hooksPath"]),
        "/ENV_WINS",
        "THE CONTROL, under the envelope's own config posture"
    );

    let body = format!("-c include.path={evil} config --get core.hooksPath");

    // -- **AN ALIAS ONLY HAS TO EXIST.** Both persistence levels are LIVE, including
    //    the one the envelope itself controls.
    for (level, args) in [
        ("--global", vec!["config", "--global", "alias.g"]),
        ("repo-local", vec!["config", "alias.l"]),
    ] {
        let mut write = args.clone();
        write.push(&body);
        assert_eq!(
            git_says(&repo, &posture, &write),
            "",
            "writing the {level} alias must succeed silently"
        );
        let name = if level == "--global" { "g" } else { "l" };
        assert_eq!(
            git_says(&repo, &posture, &[name]),
            "/INCLUDE_WINS",
            "\n\n**A PERSISTED ALIAS DOES NOT HAVE TO WIN A PRECEDENCE CONTEST — IT ONLY HAS TO \
             EXIST.** Written at {level} under the envelope's own \
             `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` redirect at a generated helper-free file, \
             invoking it resolves the INCLUDED value. **This is why closing `T-19-108`'s second \
             leg requires the question to be asked at the `git config` KEY OPERAND as well as \
             at the `-c` carrier.**"
        );
    }

    // -- **AN INCLUDE MUST WIN, AND LOSES.** Audit 8's rows, re-confirmed here rather
    //    than carried by citation, at both repository-side levels this round can
    //    reach.
    for (level, args) in [
        ("repo-local", vec!["config", "include.path"]),
        ("--global", vec!["config", "--global", "include.path"]),
    ] {
        let mut write = args.clone();
        write.push(&evil);
        git_says(&repo, &posture, &write);
        assert_eq!(
            git_says(&repo, &posture, &["config", "--get", "core.hooksPath"]),
            "/ENV_WINS",
            "\n\n**A PERSISTED `include.path` AT {level} IS INERT.** It splices at a \
             REPOSITORY-SIDE precedence and LOSES to the envelope's env-injected triplet. \
             **So a clause for `git config include.path` would be INERT and must NOT be \
             added** — that is the asymmetry, and adding an inert clause is the likeliest way \
             `19-25` wastes a rule."
        );
    }
}

// ===========================================================================
// 13. K2 measured INERT with its MECHANISM recorded, and the class confirmed
//     END TO END against real git with a BARE REMOTE
//
// GREEN today and after. **Not inherited by citation from audit 8** — the fixture
// is rebuilt here and every leg is re-measured with the remote's SHA before and
// after and a CONTROL beside each, because claims in this codebase about what
// outranks the injection have been WRONG TWICE.
// ===========================================================================

/// A child that DUMPS ITS OWN ENVIRONMENT and then asks git what it resolves.
///
/// **The inertness of K2 is EXPLAINED rather than observed**: the child's own
/// environment still carries `GIT_CONFIG_COUNT=1` / `KEY_0=core.hooksPath` /
/// `VALUE_0=/ENV_WINS`, which is WHY it inherits.
fn write_dumper(path: &Path) {
    std::fs::write(
        path,
        "#!/bin/sh\n\
         echo \"CHILD_ENV COUNT=${GIT_CONFIG_COUNT:-unset} KEY_0=${GIT_CONFIG_KEY_0:-unset} \
         VALUE_0=${GIT_CONFIG_VALUE_0:-unset}\" >&2\n\
         echo \"CHILD_RESOLVES $(git config --get core.hooksPath)\" >&2\n\
         exit 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn every_k2_representative_is_inert_because_the_child_inherits_the_injection() {
    // **K2 IS INERT FOR LAYER 3, AND THAT IS A MEASUREMENT, NOT A CATEGORY
    // ARGUMENT.** This is audit 8's `GIT_PAGER` / `GIT_EDITOR` / `GIT_SSH` finding
    // restated on the CONFIG side. **A K2 member that does NOT inherit would be a
    // FINDING to report, not a row to add** — none was found.
    let dir = TempDir::new().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main", "."]);
    std::fs::write(repo.join("f"), "hi\n").unwrap();
    std::fs::write(repo.join(".gitattributes"), "f filter=x\n").unwrap();
    git_ok(&repo, &["add", "-A"]);
    git_ok(&repo, &["commit", "-qm", "init"]);
    std::fs::write(repo.join("f"), "bye\n").unwrap();

    let dumper = dir.path().join("dump.sh");
    write_dumper(&dumper);
    let dumper = dumper.to_str().unwrap().to_string();

    for (representative, args) in [
        (
            "alias.<n> with a `!` body",
            vec!["-c".to_string(), format!("alias.b=!{dumper}"), "b".to_string()],
        ),
        (
            "diff.external",
            vec!["-c".to_string(), format!("diff.external={dumper}"), "diff".to_string()],
        ),
        (
            "filter.<n>.clean",
            vec![
                "-c".to_string(),
                format!("filter.x.clean={dumper}"),
                "add".to_string(),
                "--renormalize".to_string(),
                "f".to_string(),
            ],
        ),
    ] {
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let said = git_says(&repo, &[], &refs);
        assert!(
            said.contains("CHILD_ENV COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS"),
            "\n\n**K2 REPRESENTATIVE `{representative}` DID NOT RUN, OR DID NOT INHERIT.**\n\n\
             The whole K2 finding is that a config value git re-parses as a SHELL command line \
             is executed as a CHILD that INHERITS the envelope's \
             `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet — so it is INERT for layer 3. **The \
             MECHANISM is what is asserted here, not just the verdict**: the child dumps its own \
             environment, and that dump is the explanation.\n\n\
             If the child ran but did NOT carry the triplet, that is a FINDING — a K2 member \
             that does not inherit would be a live class of its own — and it must be REPORTED \
             rather than folded into a row.\n\n  got: {said:?}"
        );
        assert!(
            said.contains("CHILD_RESOLVES /ENV_WINS"),
            "and the child must RESOLVE the injected value, which is what INERT means for layer \
             3. got: {said:?}"
        );
        println!("K2 INERT — {representative}: {said}");
    }

    // -- **A FOURTH REPRESENTATIVE, `credential.helper`, which needs STDIN to make
    //    git invoke the child at all.** It is measured here rather than in the loop
    //    above because `git credential fill` reads a request from stdin; a probe
    //    that merely READ the key (`git config --get credential.helper`) would print
    //    the value and never run the helper, which is a probe measuring nothing.
    let mut child = std::process::Command::new("git")
        .current_dir(&repo)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "core.hooksPath")
        .env("GIT_CONFIG_VALUE_0", "/ENV_WINS")
        .args(["-c", &format!("credential.helper=!{dumper}"), "credential", "fill"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("git is on PATH");
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(b"protocol=https\nhost=example.invalid\n\n")
            .unwrap();
    }
    let out = child.wait_with_output().expect("git answers");
    let said = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        said.contains("CHILD_ENV COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS")
            && said.contains("CHILD_RESOLVES /ENV_WINS"),
        "\n\n**K2 REPRESENTATIVE `credential.helper` DID NOT RUN, OR DID NOT INHERIT.**\n\n\
         A `!`-prefixed credential helper is a SHELL command line run as a CHILD, and the child \
         must carry the envelope's `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet. A member that \
         did not inherit would be a FINDING to report rather than a row.\n\n  got: {said:?}"
    );
    println!("K2 INERT — credential.helper: {said}");
}

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
fn a_reparsed_alias_value_carries_a_refused_push_past_the_hook_and_moves_a_bare_remotes_ref() {
    // **THE BEHAVIOURAL CLAIM, REPRODUCED RATHER THAN REPEATED.** The fixture is
    // rebuilt and every leg re-measured, with a CONTROL beside each, because claims
    // in this codebase about what outranks the injection have been wrong twice.
    //
    // The hook is delivered EXACTLY as the envelope delivers it — `core.hooksPath`
    // through the `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet `cred::hooks_path_env`
    // emits — and the remote is a LOCAL BARE repository, so this is offline.
    let dir = TempDir::new().unwrap();
    let upstream = dir.path().join("upstream.git");
    let work = dir.path().join("work");
    let hooks_dir = dir.path().join("hooks");
    let generated = dir.path().join("generated.cfg");
    std::fs::create_dir_all(&upstream).unwrap();
    std::fs::create_dir_all(&work).unwrap();
    std::fs::create_dir_all(&hooks_dir).unwrap();
    std::fs::write(&generated, "[credential]\n").unwrap();

    git_ok(&upstream, &["init", "--bare", "-q", "."]);
    git_ok(&work, &["init", "-q", "-b", "gsd-auto/alpha/w", "."]);
    std::fs::write(work.join("a"), "x\n").unwrap();
    git_ok(&work, &["add", "a"]);
    git_ok(&work, &["commit", "-qm", "init"]);
    git_ok(&work, &["remote", "add", "origin", upstream.to_str().unwrap()]);
    git_ok(&work, &["push", "-q", "-u", "origin", "gsd-auto/alpha/w"]);

    for name in ["pre-push", "pre-commit"] {
        let path = hooks_dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\necho '{name}: REFUSED' >&2\nexit 1\n")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    let evil = dir.path().join("evil.cfg");
    std::fs::write(&evil, "[core]\n\thooksPath = /nonexistent-hooks-dir\n").unwrap();
    let evil = evil.to_str().unwrap().to_string();

    // Every leg runs under the envelope's own config posture AND its own injection.
    let run = |args: &[&str]| -> bool {
        std::process::Command::new("git")
            .current_dir(&work)
            .env("GIT_CONFIG_GLOBAL", &generated)
            .env("GIT_CONFIG_SYSTEM", &generated)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", hooks_dir.to_str().unwrap())
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .args(args)
            .output()
            .expect("git is on PATH")
            .status
            .success()
    };

    let mut trail: Vec<String> = Vec::new();

    // -- LEG 1 CONTROL: the plain in-namespace push. The hook REFUSES and the ref
    //    does not move. **Without this leg every leg below proves nothing**, because
    //    a ref that was going to move anyway is not evidence of a disarmed hook.
    std::fs::write(work.join("a"), "x2\n").unwrap();
    git_ok(&work, &["commit", "-qam", "c2", "--no-verify"]);
    let before = remote_sha(&upstream);
    let ok = run(&["push", "origin", "refs/heads/gsd-auto/alpha/w"]);
    let after = remote_sha(&upstream);
    trail.push(format!("LEG 1 CONTROL plain push: ok={ok} {before} -> {after}"));
    assert!(
        !ok,
        "LEG 1 CONTROL: the plain in-namespace push must be REFUSED by the `pre-push` hook the \
         envelope installed. If it is not, the fixture's hook is not being read and every leg \
         below is measuring nothing."
    );
    assert_eq!(
        before, after,
        "LEG 1 CONTROL: and the bare remote's ref must NOT MOVE. before={before} after={after}"
    );

    // -- LEG 2 CONTROL: the SAME ALIAS MECHANISM with NO carrier. **This is the
    //    sharpest control in the fixture**: it isolates the carrier from the alias.
    run(&["config", "alias.noinc", "push origin refs/heads/gsd-auto/alpha/w"]);
    let before = remote_sha(&upstream);
    let ok = run(&["noinc"]);
    let after = remote_sha(&upstream);
    trail.push(format!("LEG 2 CONTROL alias, no carrier: ok={ok} {before} -> {after}"));
    assert!(
        !ok,
        "LEG 2 CONTROL: a persisted NON-SHELL alias WITHOUT the include must still be REFUSED by \
         the hook. **This is what isolates the CARRIER from the ALIAS**: if this leg passed, the \
         harm would be 'aliases evade hooks' rather than 'a re-parsed value carries a config \
         indirection', and the whole class would be mis-attributed."
    );
    assert_eq!(before, after, "LEG 2 CONTROL: and the ref must NOT MOVE");

    // -- LEG 3 CARRIER: the persisted NON-SHELL alias CARRYING the include, which
    //    the guard PERMITS at exit 0 (section 2). The hook is gone and the ref MOVES.
    run(&[
        "config",
        "alias.p",
        &format!("-c include.path={evil} push origin refs/heads/gsd-auto/alpha/w"),
    ]);
    let before = remote_sha(&upstream);
    let ok = run(&["p"]);
    let after = remote_sha(&upstream);
    trail.push(format!("LEG 3 CARRIER persisted alias: ok={ok} {before} -> {after}"));
    assert!(ok, "LEG 3: the carrier-bearing persisted alias must carry the push to COMPLETION");
    assert_ne!(
        before, after,
        "\n\nLEG 3: **THE BARE REMOTE'S REF MUST MOVE.** before={before} after={after}\n\n\
         This is `T-19-108`'s harm as a MEASUREMENT rather than an argument: a push the \
         `pre-push` hook refuses completes and rewrites a remote ref, through two separately \
         PERMITTED tool calls the stateless guard cannot correlate. `pre-push` is the ONLY \
         carrier of the worktree credential scan (`hooks.rs:342`, SAFE-05) and the second \
         carrier `AR-19-03` rests on."
    );

    // -- LEG 4 CARRIER: the `-c alias.<n>=` delivery, on ONE line.
    std::fs::write(work.join("a"), "x3\n").unwrap();
    git_ok(&work, &["commit", "-qam", "c3", "--no-verify"]);
    let before = remote_sha(&upstream);
    let ok = run(&[
        "-c",
        &format!("alias.q=-c include.path={evil} push origin refs/heads/gsd-auto/alpha/w"),
        "q",
    ]);
    let after = remote_sha(&upstream);
    trail.push(format!("LEG 4 CARRIER -c alias delivery: ok={ok} {before} -> {after}"));
    assert!(ok, "LEG 4: the `-c alias.<n>=` delivery must carry the push to COMPLETION");
    assert_ne!(
        before, after,
        "LEG 4: and the ref must MOVE, on a SINGLE line the guard permits at exit 0. \
         before={before} after={after}"
    );

    // -- LEG 5: the `pre-commit` enforcement point falls the same way, control first.
    std::fs::write(work.join("a"), "x4\n").unwrap();
    let head_before = git_says(&work, &[], &["rev-parse", "HEAD"]);
    assert!(
        !run(&["commit", "-qam", "c4"]),
        "LEG 5 CONTROL: the control commit must be REFUSED by the `pre-commit` hook"
    );
    assert!(
        run(&["-c", &format!("include.path={evil}"), "commit", "-qam", "c4"]),
        "LEG 5: and the same commit under the carrier must SUCCEED"
    );
    let head_after = git_says(&work, &[], &["rev-parse", "HEAD"]);
    trail.push(format!("LEG 5 pre-commit: {head_before} -> {head_after}"));
    assert_ne!(
        head_before, head_after,
        "LEG 5: HEAD must have MOVED. before={head_before} after={head_after}"
    );

    // -- **THE `T-19-86` RECORD: audit 7's `!`-BODIED DESTRUCTIVE PAIR STILL WORKS
    //    AFTER THIS ROUND.** It is measured here so the SUMMARY states it as a fact
    //    rather than a caveat. It is NOT closed, narrowed or re-scoped, and closing
    //    it would mean taking on `T-19-86`, which is out of scope by explicit user
    //    decision and whose two rows are pinned PERMITTED in files this round may
    //    not edit.
    std::fs::write(work.join("a"), "x5\n").unwrap();
    git_ok(&work, &["commit", "-qam", "c5", "--no-verify"]);
    run(&[
        "config",
        "alias.bang",
        &format!("!git -c include.path={evil} push --force origin HEAD:refs/heads/gsd-auto/alpha/w"),
    ]);
    let before = remote_sha(&upstream);
    let ok = run(&["bang"]);
    let after = remote_sha(&upstream);
    trail.push(format!("T-19-86 `!` pair (NOT CLOSED): ok={ok} {before} -> {after}"));
    assert!(
        ok && before != after,
        "\n\n**`T-19-86` IS RECORDED, NOT CLOSED — AND THIS ROW SAYS SO MECHANICALLY.**\n\n\
         Audit 7's `!`-bodied destructive pair must STILL move the bare remote's ref after this \
         round. A `!` body runs in a CHILD that inherits the injection, and then applies its OWN \
         command-line carrier on top of it inside that child — which is why the layer-3 catch \
         three documents attribute to closing `T-19-103` is ABSENT here. **This is audit 8's \
         finding that `T-19-86` is measurably WIDER than the register credits, asserted rather \
         than described.**\n\n\
         If this ever becomes false, `T-19-108`'s scoped closure has silently taken on \
         `T-19-86` — which would ALSO turn `tests/envelope_command_position.rs:550` and \
         `tests/envelope_config_resolution.rs:1539-1543` red. before={before} after={after}"
    );

    println!("END-TO-END, reproduced independently of audit 8:\n  {}", trail.join("\n  "));
}
