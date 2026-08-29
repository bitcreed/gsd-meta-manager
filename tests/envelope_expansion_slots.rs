// ============================================================================
// Round 4, Rule A — the DECISION WORDS of a governed program, assembled by
// shell expansion.
//
// **What this file is.** The reproducers `19-SECURITY.md`'s THIRD audit measured
// against the built binary at `228e4bc`, one fresh `mktemp -d` envelope root per
// row and the envelope directory walked afterwards, with the exit codes it
// recorded:
//
// ```
// exit=0  V=push; git $V --force origin main                                   <- T-19-88
// exit=0  V=stash; git $V                                                      <- T-19-88
// exit=0  V=update-ref; git $V -d refs/heads/main                              <- T-19-88
// exit=0  P=pr; gh $P create --title x                      (no ledger line)   <- T-19-88
// exit=0  gh `true`pr create --title x                      (no ledger line)   <- T-19-88, backtick
// exit=0  git ${X}push --force origin main                                     <- T-19-87
// exit=0  git $(true)push --force origin main                                  <- T-19-87, paren
// exit=0  git ${X}stash                                                        <- T-19-87
// exit=0  git ${X}update-ref -d refs/heads/main                                <- T-19-87
// exit=0  git ${X}config core.hooksPath /tmp/x                                 <- T-19-87
// exit=0  gh ${X}pr create --title x                        (no ledger line)   <- T-19-87
// exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main     <- ALL THREE LAYERS
// exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin                     <- T-19-87, CARRIED
// exit=0  env -u GSD_MM_RUN_ID git fetch origin                                <- T-19-90
// ```
//
// plus the region cells found while CHECKING plan 19-14 rather than by the audit
// — the forge's SECOND subcommand word, the `api` arm's decision words including
// the `-`-initial flag spelling and the endpoint DISPLACED past
// `subcommand_words` by an option value only the `api` scan skips, and
// `classify_config`'s own key operand — each measured at exit 0 by the same
// method and each carried here at that provenance.
//
// **Written BEFORE the fix, and that ordering is the point.** A corpus written
// after a fix cannot distinguish a control that works from a control written to
// agree with what the code already did — the failure mode `T-19-51` caught on
// itself in 19-08, and the reason this is round 4 of gap closure rather than
// round 1. Every refusal row below is committed RED, with its verbatim failure
// output recorded in `19-14-SUMMARY.md`, before a single production line moves.
//
// **Why a THIRD file rather than more rows in `tests/envelope_command_position.rs`.**
// That file is round 3's evidence and its header states the exit codes round 3
// measured. Round 4's evidence lives here so a later reader can tell which round
// produced which measurement, and so neither header has to be rewritten to stay
// true.
//
// ## TWO ROWS IN THIS FILE ARE DELIBERATELY LEFT RED BY PLAN 19-14
//
// `the_two_severed_prefix_rows_are_carried_forward_to_19_15_and_are_expected_red`
// holds
//
// ```
// C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin
// K=GIT_SSH;    env -u ${K}_COMMAND git fetch origin
// ```
//
// Both are RED before Rule A and both are STILL RED after it. The expansion
// lands in a wrapper operand SEVERED from the governed program, so no verb-slot
// rule can reach them; `19-15` closes them positionally. They are left failing
// here so that each rule is shown separately load-bearing rather than one being
// certified by the other's evidence: if `19-15` found them already green, Rule B
// would be a rule nobody had shown was needed, and if `19-14` closed them,
// nobody could tell which mechanism did it. **They are not `#[ignore]`d — an
// ignored test is a test nobody sees go green.**
//
// **What stops this file passing vacuously.**
//
// 1. Anti-vacuity controls that pass TODAY (section 0), including a POSITIVE
//    control that proves the ledger walk can see a line at all.
// 2. A paired allow corpus (D-32, section 11): every row that design option (b)
//    — refusing any expansion-carrying token anywhere in a governed segment —
//    would have charged, pinned PERMITTED. The operand boundary is a behaviour
//    here rather than an intention.
// 3. Every refusal row asserts the D-24 reason identifier as well as the exit
//    code, so a row cannot pass by being refused for an unrelated cause — an
//    unrecoverable split being the obvious one.
// 4. Every forge refusal row asserts that the WALKED envelope root holds no
//    ledger line. A row that only read `ledger::ledger_path_in` would pass
//    identically if the line were written somewhere else.
//
// **Offline, agent-free and clock-free** (D-35). `hooks::guard_in` is driven
// in-process against a per-row `TempDir` envelope root and a config path that
// does not exist, so `resolve_policy` applies the tighter defaults. One root per
// row, because the PR ledger persists and a shared root produces misleading
// cap-exhaustion refusals.
//
// **No crate was added** (`T-19-SC`). The directory walk below is eight lines of
// `std::fs`.
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
        // TIGHTER defaults rather than failing.
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

// ---------------------------------------------------------------------------
// The walk — the addition round 3's harness lacks and the audit's method
// requires
// ---------------------------------------------------------------------------

/// Every file under `dir`, recursively.
///
/// **Why a walk rather than a read of `ledger::ledger_path_in`.** A row that
/// asserted "the expected ledger path is empty" would pass identically if the
/// line had been written somewhere else under the envelope root — to another
/// alias's ledger, to a differently-named file, to a subdirectory a later change
/// introduced. Audit 3 measured every no-ledger-line claim by walking a fresh
/// `mktemp -d` root, and this is that method made mechanical: the absence is
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
///
/// A ledger entry is a JSON object carrying `platform` — the field
/// `ledger::LedgerEntry` writes and nothing else under this root does.
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
/// envelope root of its own.
fn refuses(command: &str, reason: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. `19-SECURITY.md`'s third audit measured this line at \
         exit 0 against the built binary: `classify_segments` collapses each `Token` to its \
         `text` before either classifier runs, so `Token.expansion` is structurally \
         unavailable to the words the decision turns on. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}`. Asserting the identifier and not \
         merely the exit code is what stops a row from passing because it was refused for \
         an unrelated cause — an unrecoverable split, say, which would refuse the same line \
         for the wrong reason and leave the class exactly as open. Got: {}",
        answer.reason()
    );
}

/// Assert one command is refused with one specific D-24 reason AND that the
/// walked envelope root holds no pull-request ledger line.
///
/// Every forge row uses this rather than [`refuses`], because the harm
/// `T-19-88` names on the forge path is not only that the command runs: it is
/// that `pr_command_label` matches no arm, so the creation is never COUNTED and
/// the SAFE-06 cap is bypassed rather than exceeded.
fn refuses_and_writes_no_ledger_line(command: &str, reason: &str) {
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), command);

    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    assert!(
        answer.reason().contains(reason),
        "`{command}` must be refused UNDER `{reason}`. Got: {}",
        answer.reason()
    );

    let written = ledger_lines_anywhere(envelope.path());
    assert!(
        written.is_empty(),
        "`{command}` was refused, but a pull-request ledger line was written somewhere \
         under the envelope root. A refusal taken AFTER the ledger write consumes cap \
         budget for a command that never opens a pull request, which is the SAFE-06 cap \
         charged for nothing. Found: {written:?}\nWalked listing:\n{}",
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
         denied what it could not read would deny this and make a driven run unusable — \
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

// ===========================================================================
// 0. The anti-vacuity controls — every one of these passes TODAY
// ===========================================================================

#[test]
fn the_unwrapped_refusals_still_fire_which_proves_this_harness_can_see_a_denial() {
    // If either row is ever red, every refusal row in this file is red for a
    // reason that has nothing to do with a decision word — the harness cannot
    // observe a refusal at all.
    refuses(
        "git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses("git stash", policy::REASON_FORCE_PUSH_BLOCKED);

    // Round 3's own closure, re-asserted here so this file fails loudly if it is
    // ever regressed by round 4's work.
    refuses(
        "env -u git git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all() {
    // **Without this row, every `refuses_and_writes_no_ledger_line` assertion
    // above could be passing because the walk cannot see anything.** An absence
    // assertion needs a positive control for the same reason an `include_str!`
    // absence control does: "the line is not there" and "this scanner cannot
    // read the file" are indistinguishable without one.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 0,
        "the first `gh pr create` of a run is PERMITTED — the cap is 3/1 and this is the \
         first. stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );

    let written = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        written.len(),
        1,
        "a permitted pull-request creation writes EXACTLY ONE ledger line, and the walk \
         must be able to find it. If this is 0 the walk is blind and every no-ledger-line \
         assertion in this file is vacuous. Walked listing:\n{}",
        listing(envelope.path())
    );

    // Printed so the SUMMARY records a listing that was MEASURED rather than
    // described. `cargo test -- --nocapture` shows it.
    println!(
        "positive control — walked envelope root after a PERMITTED `gh pr create`:\n{}\n  \
         ledger lines found: {}",
        listing(envelope.path()),
        written.len()
    );
}

// ===========================================================================
// 1. `T-19-88` — the git VERB slot
// ===========================================================================

#[test]
fn a_git_verb_assembled_by_expansion_is_refused_in_every_measured_spelling() {
    // The three `T-19-88` lines exactly as audit 3 measured them. No braces and
    // no parentheses are needed: `$V` survives as ONE token carrying
    // `Token.expansion`, and `classify_git` reads its verb as `$V`, finds it
    // absent from the denylist, and answers `Allow` from the default arm.
    refuses(
        "V=push; git $V --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses("V=stash; git $V", policy::REASON_ENVELOPE_ASSERTION_FAILED);
    refuses(
        "V=update-ref; git $V -d refs/heads/main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_verbs_with_no_second_carrier_are_the_reason_this_class_is_rated_high() {
    // **What the two rows below cost, stated rather than left to be inferred.**
    // `git push --force` has a `pre-push` hook behind it: layer 3 sees what git
    // actually does regardless of how git was invoked. `git stash` and
    // `git update-ref` have NO `pre-push` and NO `pre-commit` behind them — git
    // runs no hook for either — so for these two shapes the `PreToolUse` guard
    // is the ONLY control, and a verb it cannot read is a control that is not
    // there. `git stash` removes the human's uncommitted work from the tree,
    // where `git fsck --lost-found` is the only recovery.
    refuses("V=stash; git $V", policy::REASON_ENVELOPE_ASSERTION_FAILED);
    refuses(
        "V=update-ref; git $V -d refs/heads/main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 2. `T-19-88` — the forge's FIRST subcommand word
// ===========================================================================

#[test]
fn a_forge_first_subcommand_word_assembled_by_expansion_is_refused_and_writes_no_ledger_line() {
    // `pr_command_label` matches on `["pr", "create", ..]`. An expansion in the
    // first word leaves no arm matching, so `None` is returned: the command is
    // neither refused NOR counted, and the SAFE-06 cap is bypassed rather than
    // exceeded. The cap has no second carrier (`T-19-35`).
    refuses_and_writes_no_ledger_line(
        "P=pr; gh $P create --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // The backtick spelling, which the audit measured separately because it
    // reaches the same slot through a different tokenizer arm: a backtick sets
    // `Token.expansion` without being a separator, so `` `true`pr `` survives as
    // one word.
    refuses_and_writes_no_ledger_line(
        "gh `true`pr create --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // The GitLab sibling, whose arm is `["mr", "create", ..]`.
    refuses_and_writes_no_ledger_line(
        "P=mr; glab $P create --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 3. The forge's SECOND subcommand word — the cell a ONE-WORD region leaves
//    open
// ===========================================================================

#[test]
fn a_forge_second_subcommand_word_assembled_by_expansion_is_refused_and_writes_no_ledger_line() {
    // **Found while CHECKING plan 19-14, not by audit 3, and carried here at
    // that provenance.** `pr_command_label` matches on TWO words —
    // `["pr", "create", ..]` and `["mr", "create", ..]` — so an expansion in the
    // SECOND word leaves no arm matching just as surely as one in the first: no
    // arm, `None` returned, no refusal, no cap charge and NO LEDGER LINE. A
    // decision region defined as "the first subcommand word" would leave this
    // exact shape open, which is `T-19-88` recurring inside the fix for
    // `T-19-88`.
    refuses_and_writes_no_ledger_line(
        "P=create; gh pr $P --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses_and_writes_no_ledger_line(
        "M=create; glab mr $M --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 4. The `api` arm's decision words — five spellings, in five groups
// ===========================================================================

#[test]
fn an_api_endpoint_assembled_by_expansion_is_refused_and_writes_no_ledger_line() {
    // `endpoint_is_pulls` compares the last path segment against `"pulls"`. It
    // is `$P`, so the comparison is false, `posts && false` is false, the label
    // is `None`, and the pull request opens with no refusal and no ledger line.
    refuses_and_writes_no_ledger_line(
        "gh api /repos/o/r/$P -X POST -f title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn an_api_endpoint_displaced_out_of_the_first_two_subcommand_words_is_still_refused() {
    // **The blocker plan-check round 3 found, and the reason there is a THIRD
    // index primitive rather than two.** The two forge scans DISAGREE about
    // which words are flags: `subcommand_words` skips only `FORGE_VALUE_OPTS`
    // (`-R`, `--repo`, `--hostname`), while `gh_api_posts_a_pull_request` skips
    // all of `GH_API_VALUE_OPTS` (`-f`, `-F`, `--field`, `-H`, `--header`, `-q`,
    // `-t`, `--input`, …).
    //
    // So `-f`'s VALUE `title=x` is an ordinary non-flag word to the first scan
    // and displaces the endpoint to the THIRD position:
    //
    //   subcommand_words = ["api", "title=x", "repos/o/r/$E"]
    //
    // The endpoint is therefore in NO part of a region derived from that scan —
    // it is not one of the first two words, it is not the method value, it does
    // not begin with `-`, and it does not begin with an expansion marker.
    // Meanwhile the arm's OWN scan skips the whole `-f` pair, sets
    // `implies_post`, takes `path = repos/o/r/$E`, and compares `$E` against
    // `"pulls"` — so the pull request opens uncounted with no ledger line.
    //
    // **A region computed by a second scan is the defect this whole round is
    // about.** It has now sat one slot over four times: the wrapper operand, the
    // governed program's own operand, the verb slot, and here. Every index in
    // the region must be reported by the scan whose answer it guards.
    refuses_and_writes_no_ledger_line(
        "E=pulls; gh api -f title=x repos/o/r/$E",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // The same hole with a different displacing option. `-H accept:x` is one
    // more `GH_API_VALUE_OPTS` pair that `subcommand_words` does not know about.
    refuses_and_writes_no_ledger_line(
        "E=pulls; gh api -H accept:x repos/o/r/$E -f title=y",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn an_api_method_assembled_by_expansion_is_refused_and_writes_no_ledger_line() {
    // `gh api` decides POST-ness on the method when one is given. A method it
    // cannot read is a decision it cannot make.
    refuses_and_writes_no_ledger_line(
        "gh api repos/o/r/pulls -X $M -f title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    // The `=`-attached spelling, which reaches `method` through a different
    // branch of the same scan.
    refuses_and_writes_no_ledger_line(
        "gh api repos/o/r/pulls -X=$M -f title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn an_api_flag_whose_flag_ness_cannot_be_read_is_refused_and_writes_no_ledger_line() {
    // **`gh api` decides on whether a token IS a flag.**
    // `gh_api_posts_a_pull_request` sets `implies_post` by WHOLE-TOKEN
    // membership in `GH_API_IMPLIES_POST`, so a token it cannot read yields
    // `posts == false`, the label is `None`, and a pull request opens with no
    // refusal, no ledger line and no cap charge.
    //
    // The marker-initial spelling: `$F` expands to `-f` at run time.
    refuses_and_writes_no_ledger_line(
        "F=-f; gh api repos/o/r/pulls $F title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn an_api_flag_that_is_dash_initial_with_an_unreadable_key_half_is_a_separate_row_on_purpose() {
    // **The same hole ONE CHARACTER TO THE LEFT, and it is a separate row
    // because a clause written only for the marker-initial form leaves it in NO
    // part of the region.** `-$F` begins with `-`, so it is not one of the first
    // two subcommand words (it is a flag to `subcommand_words`), it is not the
    // method value, and it is not marker-initial. The clause therefore has to be
    // "marker-initial OR `-`-initial with an expansion before the first `=`",
    // which is the same key-half readability test the `git -c` check applies.
    refuses_and_writes_no_ledger_line(
        "F=f; gh api repos/o/r/pulls -$F title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    // The brace spelling of the same word. The tokenizer flushes at `{`, so the
    // segment's flag token is the bare `-$`.
    refuses_and_writes_no_ledger_line(
        "F=f; gh api repos/o/r/pulls -${F} title=x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 5. `T-19-87` — the brace and paren spellings, SIX of eight rows
// ===========================================================================

#[test]
fn the_brace_and_paren_spellings_that_land_in_a_verb_slot_are_refused() {
    // **These close with NO change to what `{`, `}`, `(` and `)` do.** The
    // tokenizer flushes the current word at each of those characters and emits
    // an operator, so `git ${X}push --force origin main` becomes the segments
    //
    //   `git $`   |   `X`   |   `push --force origin main`
    //
    // The FIRST segment resolves `git` at the head, and its VERB SLOT is the
    // flushed `$` — a token that survives carrying `Token.expansion`. A
    // verb-slot rule therefore reaches every one of these without `SEPARATORS`,
    // `split_segments` or the tokenizer's separator arm being touched at all.
    //
    // **This is SIX of `T-19-87`'s eight measured rows, not eight.** The two
    // where the flush lands in a wrapper prefix SEVERED from the governed
    // program are in
    // `the_two_severed_prefix_rows_are_carried_forward_to_19_15_and_are_expected_red`
    // below, and `T-19-87` is NOT claimed closed by plan 19-14.
    refuses(
        "git ${X}push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "git $(true)push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses("git ${X}stash", policy::REASON_ENVELOPE_ASSERTION_FAILED);
    refuses(
        "git ${X}update-ref -d refs/heads/main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "git ${X}config core.hooksPath /tmp/x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_brace_spelling_on_the_forge_path_is_refused_and_writes_no_ledger_line() {
    // The sixth of the six, kept in its own test because the harm it names is
    // the ledger line rather than the exit code: audit 3 measured this row at
    // exit 0 with NO ledger line and no park, which is SAFE-06 bypassed rather
    // than exceeded.
    refuses_and_writes_no_ledger_line(
        "gh ${X}pr create --title x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 6. The composed three-layer line
// ===========================================================================

#[test]
fn the_composed_three_layer_line_is_refused_through_its_verb_slot_half() {
    // **What this line is in bash**, which is why audit 3 rated it the way it
    // rated `T-19-60`: it runs `env -u GIT_CONFIG_COUNT git push --force origin
    // main`. Layer 1's permission prefix does not match `Bash(git push:*)`;
    // layer 2 permits; layer 3's `core.hooksPath` carrier is removed by the
    // `-u`, so the `pre-push` hook does not run; and `GIT_ASKPASS`,
    // `GIT_CONFIG_GLOBAL` and `GIT_SSH_COMMAND` are untouched, so the push
    // authenticates. All three layers cleared by one tool call.
    //
    // Rule A reaches it through the `${X}push` half: the fragment
    // `_COUNT git $` resolves `git` behind a one-word ungoverned head and its
    // verb slot is the flushed `$`. The `${C}_COUNT` half is NOT reached by Rule
    // A — see the carry-forward test below.
    refuses(
        "C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 7. The `-c` key and `classify_config`'s own key operand
// ===========================================================================

#[test]
fn a_leading_config_option_whose_key_cannot_be_read_is_refused() {
    // **The same root cause one slot to the LEFT of the verb.** `scan_leading`
    // decides whether `core.hooksPath` is being set at command-line precedence —
    // the one form that outranks the envelope's own env-injected setting (D-09)
    // — and a key it cannot read is a decision it cannot make.
    //
    // The check is on the KEY HALF only, and that is what keeps
    // `git -c user.name="$NAME" commit -m x` working: a VALUE carrying an
    // expansion changes what the setting IS, not WHICH setting it is.
    refuses(
        "git -c $K commit -m x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "git -c ${K}=/tmp/x commit -m x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn a_config_key_operand_assembled_by_expansion_is_refused() {
    // **Found while checking plan 19-14, and it is the one `T-19-91` cell this
    // plan closes.** `classify_config` walks its options and operands and then
    // tests the first key operand with `is_hooks_path_key`. Handed `$K`, that
    // test is false, the arm falls through to `Allow`, and
    // `git config $K /tmp/x` disarms layer 3 exactly as
    // `git config core.hooksPath /tmp/x` would.
    //
    // It is closed HERE — rather than registered like the rest of `T-19-91` —
    // because `git ${X}config core.hooksPath /tmp/x` is a row in audit 3's OWN
    // measured bypass list, and a region principle that stopped at the verb
    // while `config`'s key operand stayed unreadable would not be coherent.
    refuses(
        "git config $K /tmp/x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "git config ${K} /tmp/x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "git config set $K /tmp/x",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 8. `T-19-90` — the run-id key
// ===========================================================================

#[test]
fn removing_the_run_id_key_is_refused() {
    // **What its removal costs, named rather than left to be inferred.**
    // `hooks::current_run_id` falls back to the `"unattributed-run"` placeholder
    // bucket when `GSD_MM_RUN_ID` is absent, so every park this run produces is
    // attributed to a run nobody can find. D-24 requires every envelope refusal
    // to park and D-25 requires the park to land where a later reader can find
    // it, so this entry protects the EVIDENCE rather than the containment — the
    // same class as `T-19-62` and `T-19-70`, and the same reasoning 19-13
    // recorded for the sibling locator `GSD_MM_ENVELOPE_PROJECT_ROOT`.
    refuses(
        "env -u GSD_MM_RUN_ID git fetch origin",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
    refuses(
        "GSD_MM_RUN_ID=other git fetch origin",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
}

// ===========================================================================
// 9. THE CARRY-FORWARD — two rows deliberately left RED by plan 19-14
// ===========================================================================

/// **This test is EXPECTED TO FAIL at the end of plan 19-14. Do not fix it
/// here, do not weaken it, and do not mark it `#[ignore]`.**
///
/// Both rows are measured `T-19-87` bypasses. Both are RED before Rule A and
/// both are STILL RED after it, because the expansion lands in a wrapper operand
/// **severed** from the governed program:
///
/// ```text
/// C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin
///   ->  `C=GIT_CONFIG`  |  `env -u $`  |  `C`  |  `_COUNT git fetch origin`
/// ```
///
/// The last fragment resolves `git` behind a clean one-word prefix and its verb
/// is the LITERAL `fetch`. No verb-slot rule can see the shape, because by the
/// time resolution runs the shape is gone. `19-15` closes both positionally, by
/// recording which segment boundaries were created by a word-splitting flush —
/// with `SEPARATORS` untouched there too.
///
/// **The carry-forward is a MECHANISM, not an omission.** If `19-15` found these
/// already green, Rule B would be a rule nobody had shown was needed; if `19-14`
/// closed them, nobody could tell which mechanism did it. Three rounds of this
/// phase have been certified by corpora that could not fail on their own class,
/// and separating the two rules is how this round avoids being the fourth.
///
/// The harm is `T-19-81`'s: the removal of `GIT_CONFIG_COUNT` takes away the
/// `core.hooksPath` carrier layer 3 depends on, and the removal of
/// `GIT_SSH_COMMAND` puts the user's own agent and default identity back within
/// reach of the run (D-16).
#[test]
fn the_two_severed_prefix_rows_are_carried_forward_to_19_15_and_are_expected_red() {
    refuses(
        "C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    refuses(
        "K=GIT_SSH; env -u ${K}_COMMAND git fetch origin",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

// ===========================================================================
// 10. `T-19-91` — measured, pinned and REGISTERED OPEN rather than closed
// ===========================================================================

/// **Everything in this test asserts an UNCOVERED RESIDUAL, not a desirable
/// behaviour.** The git decision region plan 19-14 implements covers the VERB
/// and, when that verb is `config`, the one operand `classify_config` tests with
/// `is_hooks_path_key`. Three other git classifiers read a decision operand of
/// their own, and two of them answer `Allow` on an operand they cannot read.
///
/// **Why `config` is closed and these are not, stated as the true reason and no
/// other.** It is ROUND DISCIPLINE plus provenance:
/// `git ${X}config core.hooksPath /tmp/x` is a row in **audit 3's own measured
/// bypass list**, so closing it is part of making the region principle coherent
/// over rows the audit already established. These three were found while
/// CHECKING plan 19-14, and a plan cannot both discover a threat and be the plan
/// that measured it fail first.
///
/// **It is NOT a second-carrier argument, and the registration says so.**
/// `reflog` and `symbolic-ref` are LISTED verbs that reach their own classifiers
/// and fall to an `Allow` arm on an operand those classifiers cannot read —
/// structurally identical to the `config` cell. Neither has a `pre-push` or a
/// `pre-commit` behind it: git runs no hook for either, and
/// `classify_reflog`'s own refusal text records that the reflog is **the
/// recovery path for every other destructive git operation**. Only
/// `git push $REF` has a hook behind it. A reader who took the asymmetry for a
/// blast-radius judgement would read a narrowed threat as a covered one.
///
/// A future change that closes `T-19-91` must DELETE these rows deliberately
/// rather than discover them failing.
#[test]
fn the_t_19_91_residual_is_measured_and_pinned_rather_than_closed() {
    // --- PERMITTED today, and registered OPEN as `T-19-91` ----------------
    //
    // `classify_reflog` looks for the first non-flag token and matches it
    // against `delete`, `expire` and `drop`. Handed `$S`, no arm matches and it
    // answers `Allow` — so `S=delete; git reflog $S` destroys the reflog.
    permits("git reflog $S");
    permits("git reflog show $S");

    // `classify_symbolic_ref` counts operands and looks for `-d`/`--delete`.
    // With ONE unreadable operand it reads a plain read and answers `Allow` —
    // so `S=-d; git symbolic-ref $S` deletes the ref, and `git symbolic-ref $S`
    // where `$S` expands to two words repoints HEAD.
    permits("git symbolic-ref $S");

    // --- REFUSED today, measured rather than assumed ----------------------
    //
    // Not every unreadable decision operand fails open, and pinning the ones
    // that already fail CLOSED is what keeps the registration honest about how
    // wide `T-19-91` actually is.
    //
    // `classify_symbolic_ref` with TWO operands is a write whatever they say.
    refuses(
        "git symbolic-ref HEAD $R",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    // `classify_push` qualifies each refspec and tests it against the namespace
    // prefix. An unreadable refspec does not carry the prefix, so the namespace
    // test refuses it — the push classifier fails CLOSED on this operand.
    refuses("git push origin $REF", policy::REASON_PUSH_OUTSIDE_NAMESPACE);

    // **`git push $REF` — measured, and deliberately NOT pinned as a live row
    // here.** Measured against the unfixed tree at `c595141` it is refused
    // (`push_outside_namespace`), because with a single operand it carries no
    // refspec and `classify_push` falls to `ctx.resolved_push_dests` instead.
    // That is the ONE shape `policy::push_needs_resolved_dests` answers `true`
    // for, which makes the guard shell out to `git` in the test's own working
    // directory — so its verdict depends on the repository the test happens to
    // run in, which is `T-19-80`. Its measured verdict is recorded in
    // `19-14-SUMMARY.md` and in the `T-19-91` registration instead; the
    // repository-free half of the same question is the `git push origin $REF`
    // row above.
}

// ===========================================================================
// 11. The paired allow corpus (D-32) — the bill design option (b) would have
//     charged, and the operand boundary as a BEHAVIOUR
// ===========================================================================

#[test]
fn a_commit_message_carrying_an_expansion_still_commits() {
    // **This is the bill for design option (b)**, which would have refused any
    // expansion-carrying token anywhere in a governed segment. It refuses these,
    // which are ordinary commands this project types — `T-19-75` widened from
    // `rg` to every commit. A control that fails into unusability gets switched
    // off (AR-19-11), which is why option (a), a decision REGION, was chosen.
    //
    // Everything to the right of the decision words is an OPERAND and stays
    // free. That boundary is the whole of option (a)'s cost containment, and
    // these rows pin it as a behaviour rather than as an intention.
    permits("git commit -m \"use ${HOME} here\"");
    permits("git commit -m \"$MSG\"");
    permits("git commit -m \"fix: stop git push --force bypassing the guard\"");
}

#[test]
fn a_pull_request_title_carrying_an_expansion_still_opens_the_pull_request() {
    // The forge side of the same boundary. Each row is the FIRST pull request of
    // its own run, so the 3/1 cap is not what decides it — `permits` takes a
    // fresh envelope root per call.
    permits("gh pr create --title 'fix $PATH handling' --body x");
    permits("gh pr create --title \"$MSG\" --body x");
}

#[test]
fn an_api_call_whose_flags_are_all_readable_still_opens_the_pull_request() {
    // **The row that shows the `api` clause is a readability test rather than a
    // ban on `$`.** `-f` has a readable key half, so it stays a countable flag;
    // `title=$T` neither begins with `-` nor with an expansion marker, so it is
    // an operand; and `repos/o/r/pulls` is the endpoint the arm's own scan
    // reports, which is literal. The creation is permitted AND counted.
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "gh api repos/o/r/pulls -f title=\"$T\"");
    assert_eq!(
        answer.code, 0,
        "an `api` creation whose decision words are all readable must be PERMITTED. \
         stdout: {} stderr: {}",
        answer.stdout, answer.stderr
    );
    let written = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        written.len(),
        1,
        "...and COUNTED. A rule that refused this would not only over-refuse; it would \
         also stop the cap seeing a creation it is meant to bound. Walked listing:\n{}",
        listing(envelope.path())
    );
}

#[test]
fn a_config_option_whose_key_is_readable_still_runs_however_its_value_is_spelled() {
    // The key-half boundary, pinned from the permitted side. `user.name` and
    // `user.email` are readable keys; their VALUES carry expansions and change
    // what the setting is rather than which setting it is.
    permits("git -c user.name=\"$NAME\" commit -m x");
    permits("git config user.email \"$EMAIL\"");
}

#[test]
fn the_contents_of_a_substitution_are_not_a_decision_word() {
    // A command substitution in an UNGOVERNED command, and an ordinary `cd`.
    // These are the rows `resolve_program`'s doc names as the bill for closing
    // `T-19-74`, and they are re-asserted here because option (b) would have
    // charged them too.
    permits("echo $(git rev-parse HEAD)");
    permits("git log --format=%h $(git rev-parse HEAD)");
    permits("cd \"$HOME\"");
}

#[test]
fn the_two_core_t_19_74_spellings_this_plan_does_not_move_are_still_permitted() {
    // **AR-19-10's core, unmoved.** `env $X push --force origin main` reaches
    // ZERO governed candidates and stays `Ungoverned`;
    // `X=git; env $X push --force origin main` returns `NoProgram` for the
    // binding segment and `Ungoverned` for the command segment.
    //
    // Plan 19-14 narrows the accepted residual at exactly ONE other spelling —
    // the DECOY form `env -u git $X push --force origin main`, pinned in
    // `tests/envelope_wrapper_class.rs` — and that flip is disclosed with its
    // old and new verdicts in `19-14-SUMMARY.md` rather than absorbed.
    permits("env $X push --force origin main");
    permits("X=git; env $X push --force origin main");
}
