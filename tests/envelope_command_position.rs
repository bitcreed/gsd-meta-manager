// ============================================================================
// Round 3 — `T-19-60` narrowed but not closed, and `T-19-81`.
//
// **What this file is.** The reproducers `19-SECURITY.md`'s SECOND audit measured
// against the built binary, written BEFORE the fix, with the exit codes it
// recorded:
//
// ```
// exit=2  env -u SOME_VAR git push --force origin main                        <- the discriminator
// exit=0  env -u git git push --force origin main                             <- FORCE PUSH PERMITTED
// exit=0  env -u git git -c core.hooksPath=/dev/null push --force origin main <- ALL THREE LAYERS CLEARED
// exit=0  env -u gh gh pr create --title x                                    <- PR CAP BYPASSED, no ledger
// exit=0  sudo -u git git push --force origin main                            <- PERMITTED
// exit=0  K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin  <- T-19-81
// ```
//
// Round 1 (`19-11`, `19-12`) replaced `words[0]` with a structural resolver and
// closed all six ORIGINALLY measured lines. The mechanism survives one level up:
// `resolve_program` step 5 returns `Governed { index }` for the first token whose
// basename is governed **without establishing that the token is in a command
// position**, so a wrapper's own option operand captures the index, the real
// program becomes `argv[0]` of the classified argv, `classify_git` reads it as
// the verb, finds `git` absent from the denylist, and answers `Allow`.
//
// **Why a SECOND file rather than more rows in `tests/envelope_wrapper_bypass.rs`.**
// That file is round 2's evidence and its header states the exit codes round 2
// measured. Round 3's evidence lives here so a later reader can tell which round
// produced which measurement, and so neither file's header has to be rewritten to
// stay true.
//
// **Written before the fix, and that ordering is the point.** A corpus written
// after a fix cannot distinguish a control that works from a control written to
// agree with what the code already did — the failure mode 19-08's `T-19-51`
// caught on itself, and the reason this is round 3 rather than round 2. Every
// refusal row below is committed RED, with its verbatim failure output recorded
// in the SUMMARY, before a single production line moves.
//
// **What stops this file passing vacuously.**
//
// 1. Anti-vacuity controls that pass TODAY: the unwrapped force push refuses, and
//    so does `env -u SOME_VAR git push --force origin main` — the audit's own
//    discriminator, whose ONLY difference from the bypass is the spelling of the
//    operand.
// 2. A paired allow corpus (D-32): nine rows that must stay permitted, including
//    the two commit messages and the PR title that would break if the ambiguity
//    rule were written without the head shortcut.
// 3. Every refusal row asserts the D-24 reason identifier as well as the exit
//    code, so a row cannot pass by being refused for an unrelated cause.
//
// **Offline, agent-free and clock-free** (D-35). `hooks::guard_in` is driven
// in-process against a per-test `TempDir` envelope root and a config path that
// does not exist, so `resolve_policy` applies the tighter defaults. One root per
// call, because the PR ledger persists and a shared root produces misleading
// cap-exhaustion refusals.
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

/// Ask the guard about one shell command, against a given envelope root.
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

/// Assert one command is refused with one specific D-24 reason, against an
/// envelope root of its own.
fn refuses(command: &str, reason: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. `19-SECURITY.md`'s second audit measured this \
         line at exit 0 against the built binary: a token that is not the effective \
         program captured `resolve_program`'s index because it is spelled as a governed \
         program in a wrapper's operand slot, and the real command became `argv[0]` of \
         the classified argv. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}`. Asserting the identifier and not \
         merely the exit code is what stops a row from passing because it was refused \
         for an unrelated cause — an unrecoverable split, say, which would refuse the \
         same line for the wrong reason and leave the class exactly as open. Got: {}",
        answer.reason()
    );
}

/// Assert one command is permitted and answers nothing at all.
fn permits(command: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert!(
        answer.code == 0,
        "`{command}` must still be PERMITTED. The guard sees EVERY Bash tool call, so a \
         rule that denied what it could not place would deny this and make a driven run \
         unusable — which is how a safety control gets switched off. stdout: {} \
         stderr: {}",
        answer.stdout,
        answer.stderr
    );
    assert!(
        answer.stdout.is_empty(),
        "a permit answers nothing at all: emitting `allow` would turn a deny-only \
         control into an approval authority. Got: {}",
        answer.stdout
    );
}

// ---------------------------------------------------------------------------
// 0. The anti-vacuity controls — these two pass TODAY
// ---------------------------------------------------------------------------

#[test]
fn the_unwrapped_force_push_is_refused_which_proves_this_harness_can_see_a_denial() {
    // If this row is ever red, every other row in the file is red for a reason
    // that has nothing to do with command position — the harness cannot observe
    // a refusal at all.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn the_audits_own_discriminator_is_refused_today_and_must_stay_refused() {
    // `19-SECURITY.md`: exit=2. This is the line the bypass differs from by
    // exactly ONE word — `SOME_VAR` instead of `git` — which is what makes the
    // pair a measurement of command position rather than of wrapper handling.
    // One candidate, behind a prefix, resolves as it always did.
    refuses(
        "env -u SOME_VAR git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

// ---------------------------------------------------------------------------
// 1. `T-19-60`, the wrapper-operand sub-class — the audit-2 command-position rows
// ---------------------------------------------------------------------------

#[test]
fn a_force_push_behind_a_governed_decoy_operand_is_refused() {
    // `19-SECURITY.md`: exit=0, FORCE PUSH PERMITTED. `env -u git` puts the word
    // `git` in the wrapper's own operand slot; step 5 takes it as the program,
    // `classify_git` is then handed the argv `git push --force origin main` and
    // reads its VERB as `git`, which the denylist's default arm allows.
    //
    // The reason is `envelope_assertion_failed` rather than `force_push_blocked`,
    // and that is the design rather than a concession: two governed names in one
    // simple command with a wrapper between them is a command POSITION that
    // cannot be established without knowing the wrapper's flag grammar, which is
    // the knowledge this resolver refuses to encode. It is refused for being
    // unplaceable, not for what the thing behind it turned out to be.
    refuses(
        "env -u git git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_decoy_that_clears_all_three_layers_with_the_credential_intact_is_refused() {
    // `19-SECURITY.md`: exit=0, ALL THREE LAYERS CLEARED. Strictly worse than the
    // original `T-19-60`, whose D-09 note at least recorded that the escape cost
    // the agent its token: here `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and
    // `GIT_SSH_COMMAND` all survive, `-c core.hooksPath=/dev/null` outranks the
    // injected hooks path, and the push authenticates.
    refuses(
        "env -u git git -c core.hooksPath=/dev/null push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_decoy_operand_spelled_as_an_absolute_path_is_refused_too() {
    // Basename normalisation must be exercised on the DECOY as well as on the
    // program. A rule that compared raw token text would see `/usr/bin/git` as
    // an ordinary word and count one candidate instead of two.
    refuses(
        "env -u /usr/bin/git git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn a_second_wrapper_between_the_decoy_and_the_program_is_refused() {
    // **This row is the measurement, not an extra spelling.** The auditor's named
    // one-function fix — `classify_git` refuses an argv whose verb is itself
    // governed — closes the two rows above and NOT this one: here the index is
    // captured at `git` (the decoy), the argv handed to `classify_git` begins
    // `timeout 5 git push …`, and the verb that arm would see is `timeout`, which
    // the denylist allows.
    //
    // So the sub-class is closed by establishing command position, and the named
    // fix is kept as a SECOND, independent layer with its own unit pin rather
    // than as the fix.
    refuses(
        "env -u git timeout 5 git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_sudo_spelling_of_the_decoy_is_refused() {
    // `19-SECURITY.md`'s row 4, and the one a reader is most likely to type by
    // accident: `sudo -u git` names a UNIX ACCOUNT called `git`, which is a real
    // and ordinary thing on a git server. The resolver cannot tell that operand
    // from the decoy, and the honest answer to "I cannot tell" is a refusal — see
    // the disclosed-cost rows in `tests/envelope_wrapper_class.rs`, where
    // `sudo -u git git status` is pinned as a COST of this fix beside the
    // `git status` that still works.
    refuses(
        "sudo -u git git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_decoy_shape_is_refused_even_where_no_second_carrier_exists() {
    // `git stash` has no `pre-push` and no `pre-commit` hook behind it — nothing
    // observes it after the fact, so for this shape the guard is the ONLY
    // control. The rows above are backstopped by layer 3 when the injected hooks
    // path survives; this one is not backstopped by anything.
    refuses(
        "env -u git git stash",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_forge_decoy_is_refused_before_any_ledger_line_is_written() {
    // `19-SECURITY.md`: exit=0, PR CAP BYPASSED, **no ledger line**. The forge
    // half is not reached by `classify_git` at all — a `gh` command is decided by
    // `pr_command_label`, which returns `None` here because `subcommand_words`
    // yields `["gh", "pr", "create", "x"]` and the match wants `["pr", "create",
    // ..]`. So the cap is not exceeded; it is never counted.
    //
    // The ledger assertion is made in the direction that DISCRIMINATES. Today the
    // row exits 0 and writes nothing, so "no ledger line" is true for the wrong
    // reason — the command was never recognised. After the fix it must still be
    // true, and for the right reason: the refusal happens BEFORE the ledger
    // write, so a decoy never consumes cap budget. Pairing it with the exit code
    // is what tells the two apart.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "env -u gh gh pr create --title x");

    assert_eq!(
        answer.code, 2,
        "`env -u gh gh pr create --title x` must be REFUSED. Measured at exit 0: two \
         `gh` candidates, the first captured the index, and `pr_command_label` then read \
         a subcommand chain beginning with `gh` and recognised nothing. stdout: {} \
         stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer
            .reason()
            .contains(policy::REASON_ENVELOPE_ASSERTION_FAILED),
        "the forge decoy must be refused for an unplaceable command position, not for a \
         cap it was never counted against. Got: {}",
        answer.reason()
    );
    assert!(
        ledger_lines(envelope.path()).is_empty(),
        "a refused decoy must not consume cap budget: the refusal has to happen BEFORE \
         the ledger write, or an agent can exhaust a run's PR cap with commands that \
         never open a pull request. Ledger held: {:?}",
        ledger_lines(envelope.path())
    );
}

// ---------------------------------------------------------------------------
// 2. `T-19-81` — the second blocking threat, on both halves
// ---------------------------------------------------------------------------

#[test]
fn the_measured_ssh_agent_restoration_line_is_refused() {
    // `19-SECURITY.md`: exit=0. One tool call removes the `GIT_SSH_COMMAND`
    // braces (`IdentitiesOnly=yes`, `IdentityAgent=none`, `-F /dev/null`) and
    // restores `SSH_AUTH_SOCK`, putting the user's own ssh-agent back inside a
    // driven run — which is what D-16 and `T-19-23` exist to prevent.
    //
    // The FIRST segment is what must be refused, and the reason its check has to
    // sit before the `NoProgram` return: `K=GIT_SSH_COMMAND` runs no program at
    // all, so a value check placed after resolution — where step 7's
    // `GOVERNED_PROGRAMS` check sits — is never reached for it.
    refuses(
        "K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
}

#[test]
fn the_indirect_spelling_no_literal_match_can_find_is_refused_too() {
    // **Why this row exists.** A fix that matched the string the audit happened
    // to type would leave the class open for round 4. Here the key name is
    // assembled across a variable binding and a brace expansion, so it is spelled
    // NOWHERE — `K=GIT_SSH` is not an envelope key, and `${K}_COMMAND` is a token
    // whose value is unknowable before it runs.
    //
    // It is closed by the CLASS-level half instead: an expansion-carrying token
    // strictly between the head and the resolved governed index makes the wrapper
    // prefix unresolvable, because what that prefix does to the environment — and
    // therefore which program runs and with what — is decided after the guard has
    // answered. The guard never has to know that `env`'s `-u` removes a variable;
    // a fix that knew what `-u` meant would be a wrapper-name list wearing a
    // flag's clothes.
    refuses(
        "K=GIT_SSH; env -u ${K}_COMMAND git fetch origin",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_enabling_half_of_t_19_82_is_permitted_today() {
    // **This row is scheduled to FLIP.** `SSH_AUTH_SOCK` is one of
    // `cred::build_env_in`'s two REMOVAL entries — the belt D-16 relies on — and
    // it is absent from `ENVELOPE_ENV_KEYS` because the drift pin filtered
    // removals out with `value.is_some()`. So reassigning it is permitted today,
    // which is the enabling half of the line above.
    //
    // Asserted as PERMITTED here, against the unfixed tree, so the fix is
    // measured as a CHANGE rather than asserted about. Task 3 of plan 19-13 turns
    // this row into a refusal under `hook_bypass_blocked` and names the flip in
    // the SUMMARY.
    permits("SSH_AUTH_SOCK=/tmp/evil git fetch origin");
}

// ---------------------------------------------------------------------------
// 3. The disclosed cost of the leading-assignment VALUE check
// ---------------------------------------------------------------------------

#[test]
fn a_leading_assignment_naming_an_envelope_key_is_refused_and_the_boundary_is_pinned_beside_it() {
    // **A cost that is not pinned is a cost nobody can tell has grown.** The
    // value check refuses any LEADING assignment whose value names an envelope
    // key, which catches `FOO=GIT_ASKPASS echo hi` along with the `T-19-81` line
    // it exists for. That is the bill, and it is paid here rather than
    // discovered.
    //
    // The two permitted rows are the BOUNDARY, and the second is the load-bearing
    // one: `echo FOO=GIT_ASKPASS` carries the same word in an ARGUMENT rather
    // than in a leading assignment, and a check that fired on it would refuse
    // every command that mentions an envelope key in passing.
    refuses("FOO=GIT_ASKPASS echo hi", policy::REASON_HOOK_BYPASS_BLOCKED);
    permits("FOO=bar echo hi");
    permits("echo FOO=GIT_ASKPASS");
}

// ---------------------------------------------------------------------------
// 4. The paired allow corpus (D-32) — and the head shortcut, pinned
// ---------------------------------------------------------------------------

#[test]
fn a_commit_message_quoting_a_refused_git_command_still_commits() {
    // **These two rows are what the head shortcut costs nothing FOR.** Both are
    // ordinary commands this project types constantly, and `19-SECURITY.md`'s own
    // blast-radius table measured both at exit 0.
    //
    // The second row has TWO governed candidates — `git` at index 0 and the
    // quoted message whose first word is `git` — so an ambiguity rule written
    // WITHOUT the head shortcut would refuse it, and with it every commit message
    // that quotes a git command. That is `T-19-75` widened from `rg` to every
    // commit, and a control that fails into unusability is a control that gets
    // switched off (AR-19-11).
    //
    // The head IS the command position: nothing precedes it but assignments, so
    // no wrapper grammar is in play and there is no ambiguity to resolve.
    permits("git commit -m \"fix: stop git push --force bypassing the guard\"");
    permits("git commit -m \"git push --force is now blocked\"");
}

#[test]
fn a_pull_request_title_quoting_a_refused_git_command_still_opens_the_pull_request() {
    // The forge side of the same shortcut, and the first PR of its run so the cap
    // is not what decides it. Two candidates in the second row — `gh` at the head
    // and the title whose first word is `git`.
    permits("gh pr create --title \"block git push --force\" --body x");
    permits("gh pr create --title \"git push --force is now blocked\" --body x");
}

#[test]
fn a_decoy_operand_with_nothing_governed_behind_it_still_runs() {
    // **The row that tells "establishing position" apart from "blanket-denying
    // the shape".** `env -u git ls` has exactly ONE governed candidate — the
    // decoy — and the command that actually runs is `ls`. Refusing it would be
    // denying the SHAPE rather than answering about the position, and this row is
    // what makes the difference measurable.
    permits("env -u git ls");
}

#[test]
fn the_ordinary_corpus_is_untouched() {
    permits("ls -la");
    permits("echo hi");
    permits("env git status");
    permits("timeout 5 ls");
    permits("git push origin refs/heads/gsd-auto/alpha/w:refs/heads/gsd-auto/alpha/w");
}

// ===========================================================================
// 5. `T-19-86` — the member of `T-19-60`'s class this round does NOT close
// ===========================================================================

#[test]
fn the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted() {
    // **Everything in this test asserts an UNCOVERED RESIDUAL, not a desirable
    // behaviour.** It is the same disclosure shape `T-19-74`'s pins already use
    // in `tests/envelope_wrapper_class.rs`, and it is here so the boundary of
    // what round 3 closed is bounded on BOTH sides rather than merely asserted.
    //
    // **The shape.** A GOVERNED program's own operand naming a governed command.
    // These resolve at the head — correctly, because the head IS the command
    // position — and are then permitted by `classify_git`'s denylist default arm,
    // whose verbs here are `submodule`, `rebase`, `bisect` and (after
    // `scan_leading` consumes `-c alias.p=…`) `p`. They were permitted before
    // this plan and they are permitted after it.
    //
    // **Why they are not closed here.** They were found while PLANNING this
    // round. A plan cannot both discover a threat and be the plan that measured
    // it fail first, and closing it inside this plan would make round 3 the third
    // consecutive round in which a corpus certified a claim it could not have
    // failed on. Narrowing the closure claim was the correct response to finding
    // them; closing them inside this plan was not.
    //
    // So: `T-19-60` is closed for the WRAPPER-OPERAND sub-class only. The shape
    // below is registered as **`T-19-86`** in `19-SECURITY.md` and
    // `deferred-items.md`, disclosed in `resolve_program`'s own doc beside
    // `T-19-74` and `T-19-75`, and left for a later round.
    //
    // **A future change that moves this boundary must DELETE these rows
    // deliberately rather than discover them failing.**
    permits("git submodule foreach git push --force origin main");
    permits("git rebase -x \"git push --force origin main\" HEAD~3");
    permits("git bisect run sh -c \"git push --force origin main\"");
    permits("git -c alias.p='!git push --force origin main' p");
}
