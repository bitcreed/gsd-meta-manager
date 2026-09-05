// ============================================================================
// Round 13 — the words the reader never receives, and the paths the comparison
// never protects: the OTHER TWO ways a boundary can be silent.
//
// **What this file is.** The TWELFTH evidence file. Audit 12 adopted round 12's
// boundary — `/`-anchored substrings rather than an `=`-split or a character
// list — and closed `T-19-119` and `T-19-120`. What it then found is that
// `protected_carrier_named`'s residue condition (`policy.rs:5900-5905`)
// **enumerates the silences of a READING**, and that a boundary has THREE ways
// to be silent:
//
// ```text
// AXIS 1  WHICH WORDS REACH THE READER    <- T-19-122, unnamed until audit 12
// AXIS 2  WHAT THE READER SEES IN A WORD  <- the condition round 12 wrote
// AXIS 3  WHICH PATHS THE COMPARISON      <- T-19-123, unnamed until audit 12
//         PROTECTS
// ```
//
// The condition names the word set (*"in either word class"*) and the path set
// (*"against both paths"*) as PREMISES and then says nothing about either.
//
// **This file is a TWELFTH evidence file rather than an appendix**, for the
// reason `19-14` created a third, `19-16` a fourth, `19-18` a fifth, `19-20` a
// sixth, `19-22` a seventh, `19-24` an eighth, `19-26` a ninth, `19-28` a tenth
// and `19-30` an eleventh: round 12's evidence and round 13's evidence stay
// attributable to the round that produced them. The mechanism pins from rounds 4
// through 12 are RE-ASSERTED here over the same public functions, not moved and
// not edited in place.
//
// **This file is RED at this plan's end BY DESIGN.** `19-32` writes the corpus
// and the reproducers and STOPS; `19-33` writes the rules and the honesty
// repairs. Every commit producing this file shows ZERO `src/` hunks.
//
// ============================================================================
// THE NAMING CONTRACT, WHICH IS WHAT MAKES THIS FILE'S RED GATE ASSERT ROWS
// RATHER THAN THE MERE PRESENCE OF FAILURE
// ============================================================================
//
// **Every `#[test]` in this file that is asserted at a DERIVED POST-FIX verdict
// — and only those — is named with the prefix `after_19_33_`.** Every
// carried-forward pin, every control, every verdict-PRESERVING row and every
// `record_only` row is named WITHOUT it.
//
// The consequence is mechanical: at this plan's end the failing set is EXACTLY
// the `after_19_33_*` fns, so a broken fixture, a mis-quoted argv, a mis-derived
// floor or a class collision shows up as a failure WITHOUT the prefix and the
// gate catches it — where a bare *"something failed"* check would have passed on
// any of the four. `19-31`'s handoff used this convention for eight names; this
// plan makes it a gate rather than a habit.
//
// ============================================================================
// THE EXIT CODES MEASURED AT THIS FILE'S BASE COMMIT (`4e77a0e`), AGAINST THE
// BUILT BINARY, ONE FRESH `GSD_MM_ENVELOPE_ROOT` PER ROW, EVERY WALK EMPTY
// ============================================================================
//
// `git diff --numstat f06d153..8659a18 -- src/ tests/` was confirmed EMPTY
// before the first measurement, and `git diff --numstat 8659a18..4e77a0e --
// src/ tests/` is EMPTY too, so this is the tree audit 12 measured against.
//
// ```text
// AXIS 1 — THE WORD SET (`T-19-122`)
// exit=0  xargs rm -rf <<< <ENV>/alpha                    REACHES  (dir GONE)
// exit=2    TWIN rm -rf <ENV>/alpha                       envelope_assertion_failed
// exit=0  xargs rm -f  <<< <ENV>/alpha/pr-ledger.ndjson   REACHES  (resets a FIRED cap)
// exit=2    TWIN rm -f <ledger>                           envelope_assertion_failed
// exit=0  xargs cp /bin/true <<< <BINARY>                 the BINARY half
// exit=2    TWIN cp /bin/true <BINARY>                    envelope_assertion_failed
// exit=0  xargs -n1 rm -rf <<< <ENV>/alpha/hooks          a second `<<<` spelling
// exit=0  xargs rm -rf <<< of=<ENV>/alpha                 a NON-ZERO index inside the word
// exit=0  : >&<ledger>                                    `>&`, a SECOND operator, REACHES
// exit=0  echo evil >&<ledger>                            `>&` again, REACHES (truncates)
// exit=2    TWIN : > <ledger>                             envelope_assertion_failed
//
// exit=2  xargs rm -rf <<EOF\n<ENV>/alpha\nEOF            **A CORRECTION TO AUDIT 12**
// exit=0  xargs rm -rf <<<ENV>/alpha  (delimiter=path)    permitted, but DOES NOT REACH
// exit=0  cat <&<ledger>                                  permitted, `ambiguous redirect`
//
// AXIS 3 — THE PATH SET (`T-19-123`)
// exit=0  rm -rf <ENV>                                    REACHES  (root GONE, alpha GONE)
// exit=2    TWIN rm -rf <ENV>/alpha                       envelope_assertion_failed
// exit=0  rm -rf <ENV>/  /  mv <ENV> /tmp/gone            REACH
// exit=0  find <ENV> -delete  /  chmod 000 <ENV>          REACH
// exit=0  tar -C<ENV> --create --file /tmp/t .            REACHES (archives ./alpha/…)
// exit=0  cd <ENV> && rm -rf alpha                        REACHES — defeats the doc's own claim
// exit=2    THE DOC'S OWN: cd <ENV>/alpha && rm -f pr-ledger.ndjson
//
// THE SCAN'S COST (`T-19-124`), HEAD against a rebuilt `dd17bfb` control binary
//   1 000 slashes     68 ms  /   5 ms
//   2 000 slashes    249 ms  /   5 ms
//   5 000 slashes  1 524 ms  /   7 ms
//   8 000 slashes  3 904 ms  /   8 ms      ratio t(8000)/t(2000) = 15.7
//   9 000 slashes  4 957 ms              <- GUARD_TIMEOUT_SECS = 5 crossed between
//  10 000 slashes  6 099 ms                 18 KB and 20 KB on THIS machine
//  20 000 slashes 24 425 ms  /  14 ms
//  50 000 slashes 151 249 ms /  28 ms
// 400 000 slashes  DID NOT ANSWER in 300 s (hard external timeout, exit 124)
// 100 000 chars, NO slashes  29 ms / 21 ms  <- the DECISIVE control: the SCAN, not the LENGTH
// ```
//
// ## The rows that must NOT move (exit 0 before AND after)
//
// ```text
// exit=0  cat <<<x  /  cat <<< /tmp/x        <- class 13 is a PATH class, not a here-string ban
// exit=2  git <<<x push --force origin main  <- T-19-97 / round 6, force_push_blocked
// exit=0  ls <ENV>/unrelated-sibling         <- the ANCESTOR-vs-PREFIX discriminator
// exit=0  ls <ENV-PARENT>  /  ls /tmp        <- the ancestor STOP; no rule above the root
// exit=0  ls <BINPAR>  /  cp /bin/true <BINPAR>/some-other-file  <- the EXACT-PATH controls
// exit=0  git --git-dir=/tmp/g status        <- pinned PERMITTED in FOUR other files
// exit=0  a 100 000-character word with NO slashes, answered in 29 ms
// exit=0  a symlinked-to-REGULAR ledger, PERMITTED and COUNTED; a FRESH root
// exit=0  git -c alias.p='!git push --force origin main' p        <- T-19-86, may not move
// ```
//
// ## The rows that are RECORDED and never asserted
//
// Because `19-33` writes **no rule** for them, a row asserted REFUSED lands
// permanently red in a file `19-33` cannot satisfy, and a row asserted PERMITTED
// pins a live bypass as correct. **`19-22` asserted such a row against its own
// comment, its SUMMARY and its plan-check, and it halted `19-23` mid-plan.**
// Section 14 uses `record_only` and ONLY `record_only`, and
// `no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically`
// proves it by reading this file's own text.
//
// ============================================================================

use std::path::{Path, PathBuf};

use gsd_meta_manager::envelope::{advisory, cred, hooks, policy};
use tempfile::TempDir;

/// The alias every row drives.
const ALIAS: &str = "alpha";

/// The production sources this file reasons about, resolved at compile time.
const POLICY_SOURCE: &str = include_str!("../src/envelope/policy.rs");
const HOOKS_SOURCE: &str = include_str!("../src/envelope/hooks.rs");
const CRED_SOURCE: &str = include_str!("../src/envelope/cred.rs");
const LEDGER_SOURCE: &str = include_str!("../src/envelope/ledger.rs");

/// This file's own text, for the mechanical self-assertion in section 14.
const THIS_FILE: &str = include_str!("envelope_word_set.rs");

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

// ---------------------------------------------------------------------------
// The harness — REBUILT here rather than cited, in the shape
// `tests/envelope_interior_path.rs:190-410` carries it forward
// ---------------------------------------------------------------------------

/// Ask the guard about one shell command, against an explicit envelope root.
fn ask_full(
    envelope_root: &Path,
    config_path: &Path,
    project_root: Option<&Path>,
    command: &str,
) -> Answer {
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
        config_path,
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
    ask_full(
        envelope_root,
        &envelope_root.join("no-such-config.json"),
        None,
        command,
    )
}

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
/// entry.
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

/// A printable listing of the whole envelope root, for a failure message.
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

/// The derivation every AXIS-1 (WORD SET) row shares, written once and quoted
/// into each row's own failure message.
const WORD_SET_DERIVATION: &str = "\
**THE DERIVATION.** `19-33` carries LITERAL NON-PATHNAME redirection target \
words on a SECOND `Segment` field and chains them as a THIRD word class at the \
SAME reading site `protected_carrier_named` already iterates \
(`policy.rs:6118-6128`). The EXISTING literal filter, the EXISTING \
`slash_anchored_candidates` scan, the EXISTING `lexical_absolute_components` \
normaliser and the EXISTING two comparisons are all unchanged; only the SET OF \
WORDS they are applied to grows, and it grows by a set that was already walked \
— `skip_redirection_target` computes `text` and `literal` as BY-PRODUCTS of the \
walk that finds the target's end (`policy.rs:3231-3244`). \
**DESIGN (A) IS FORBIDDEN AND THE REASON IS A MEASURED FENCE RATHER THAN A \
PREFERENCE.** Widening `RedirectionOperator::pathname_target` so the five \
non-pathname operators record onto `Segment::redirection_targets` turns \
`only_a_literal_pathname_target_reaches_the_segment_and_the_tokens_do_not_move` \
(`policy.rs:10693-10727`, which asserts `targets_of(\"ls <<<here\")`, \
`(\"ls <<EOF\")`, `(\"ls <<-EOF\")`, `(\"ls >&2\")` and `(\"ls <&0\")` ALL EMPTY) \
and the twelve-operator grammar pin (`:10586`, `:10603`) RED. Design (B) turns \
NEITHER red, because `redirection_targets`, `pathname_target`, \
`redirection_operator`, `skip_redirection_target`, `segment.tokens`, \
`SEPARATORS` and `is_separator` all stay byte-identical. \
**THE THIRD DESIGN WAS CONSIDERED AND IS REJECTED IN WRITING:** treating every \
deleted-and-unrecorded word as UNRESOLVABLE would deny `cat <<< hello`, \
`sort <<< \"$x\"` and every fd duplication on the line, which is an outage \
rather than a boundary (AR-19-11). \
**THE PRINCIPLED RESTATEMENT:** the predicate's subject is what a line NAMES, \
not what the shell does with a word — so the word set is *every LITERAL word \
the line contains, whether it survives into argv or is consumed by a \
redirection, pathname or not.* \
The clause is raised at the SAME ONE SITE, BEFORE the resolution match, so the \
reason identifier is the GENERAL unresolvable one the sibling refusals already \
carry, `envelope_assertion_failed`, and NOT one a classifier would have earned \
(D-24).";

/// The derivation every AXIS-3 (PATH SET) row shares.
const PATH_SET_DERIVATION: &str = "\
**THE DERIVATION.** `19-33`'s protected path set becomes: a candidate UNDER \
`<root>/<alias>` component-wise (the EXISTING prefix, unchanged); OR a \
candidate that is a proper ANCESTOR of `<root>/<alias>` and is itself AT OR \
UNDER `<root>` (the NEW clause); OR a candidate EQUAL to `current_exe()` (the \
EXISTING equality, unchanged). \
**IT GROWS BY EXACTLY ONE PATH.** `envelope_dir_in(root, alias)` is \
`<root>/<alias>` and the alias is a PLAIN SINGLE PATH COMPONENT — \
`ledger_path_in` refuses anything else outright — so the ancestor set of the \
envelope directory that lies at or under the root is exactly `{<root>}`. The \
clause is nevertheless written in its GENERAL form so it stays correct if the \
directory ever becomes deeper. \
**IT STOPS AT `<root>`, AND THE REASON IS OWNERSHIP RATHER THAN TASTE.** \
`<root>` is `GSD_MM_ENVELOPE_ROOT`, or the default \
`~/.local/share/gsd-meta-manager/envelope`; it is created by this tool and \
contains only alias directories this tool created. Above it the chain is \
`~/.local/share`, `$HOME`, `/tmp`, `/` — directories shared with everything the \
user has. A boundary reaching them would refuse `ls /`, `df /`, `du -sh $HOME` \
and `ls /tmp`, which is not a boundary but an OUTAGE (AR-19-11: a refusal a \
user cannot act on is a control that gets switched off). It is the same \
argument `word_is_exactly` already gives one path over for keeping the binary \
half an EQUALITY. \
**IT IS AN ANCESTOR CLAUSE AND NOT THE EXISTING PREFIX WIDENED TO `<root>`, \
AND THE CORPUS DRAWS THE ROW THAT TELLS THEM APART.** A prefix over `<root>` \
refuses every sibling alias's directory and every file directly under the root; \
and because `GSD_MM_ENVELOPE_ROOT` is USER-SETTABLE, a user who sets it to \
`/tmp` gets the whole `/tmp` subtree refused, while the ancestor clause under \
the same configuration refuses only the single word `/tmp`. \
`ls <ENV>/unrelated-sibling` is pinned PERMITTED before AND after in section 5, \
so a prefix-widened implementation turns THIS FILE red rather than turning a \
driven run unusable. \
The mechanism the clause repairs is `word_is_within`'s `word.len() < dir.len()` \
early return over COMPONENT VECTORS (`policy.rs:5706`), which answers `false` \
for every ANCESTOR because an ancestor is SHORTER.";

/// Assert one command is refused with one specific D-24 reason, against an
/// envelope root of its own, and that the walked root holds no ledger line.
fn refuses_carrier(make: impl Fn(&Path) -> String, reason: &str, why: &str) {
    let envelope = TempDir::new().unwrap();
    let command = make(envelope.path());
    let answer = ask(envelope.path(), &command);

    assert_eq!(
        answer.code, 2,
        "\n\n`{command}` must be REFUSED.\n\n{why}\n\n\
         It was measured against the BUILT BINARY at this file's base commit with a fresh \
         envelope root and a walk after.\n  stdout: {}\n  stderr: {}\nWalked listing:\n{}",
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

/// Assert one command is permitted and answers nothing at all.
fn permits_carrier(make: impl Fn(&Path) -> String, why: &str) {
    let envelope = TempDir::new().unwrap();
    let command = make(envelope.path());
    let answer = ask(envelope.path(), &command);

    assert_eq!(
        answer.code, 0,
        "\n\n`{command}` must be PERMITTED.\n\n{why}\n\n\
         The guard sees EVERY Bash tool call, so a rule that refused more than its stated \
         boundary would make a driven run unusable — which is how a safety control gets \
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

/// [`permits_carrier`] for a command that names no envelope path.
fn permits(command: &str, why: &str) {
    permits_carrier(|_| command.to_string(), why);
}

/// [`refuses_carrier`] for a command that names no envelope path.
fn refuses(command: &str, reason: &str, why: &str) {
    refuses_carrier(|_| command.to_string(), reason, why);
}

/// Drive one row and PRINT what it answered, asserting nothing about it.
///
/// **Section 14 uses this and ONLY this**, for the reason stated in this file's
/// header: `19-33` writes no rule for `C-11` … `C-15`, for `C-08`'s behavioural
/// half, for `T-19-124`'s behavioural half, for `T-19-116`'s residues, for the
/// ancestor ABOVE the root, or for the alias-body credential route.
fn record_only(label: &str, make: impl Fn(&Path) -> String) {
    let envelope = TempDir::new().unwrap();
    let command = make(envelope.path());
    let answer = ask(envelope.path(), &command);
    println!(
        "RECORDED (not asserted) [{label}]\n  command : {command}\n  exit    : {}\n  \
         reason  : {}\n  walk    :\n{}",
        answer.code,
        if answer.reason().is_empty() {
            "(a permit answers nothing at all)".to_string()
        } else {
            answer
                .reason()
                .lines()
                .next()
                .unwrap_or_default()
                .to_string()
        },
        listing(envelope.path())
    );
}

/// The fixture command runner, offline and never the test process's cwd.
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

/// `git rev-parse <rev>` in a bare repository, as a string.
fn rev(git_dir: &Path, revision: &str) -> String {
    let out = std::process::Command::new("git")
        .args([
            "--git-dir",
            &git_dir.display().to_string(),
            "rev-parse",
            revision,
        ])
        .output()
        .expect("git is on PATH");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// **THE BINARY THIS GUARD IS RUNNING AS — `C-10` ITSELF.**
fn this_binary() -> PathBuf {
    std::env::current_exe().expect("the running test binary has a path")
}

/// The DIRECTORY the binary lives in — a shared directory, and therefore an
/// EXACT-PATH boundary rather than a prefix one. **No ancestor clause is written
/// for it and that is stated rather than left to be noticed.**
fn this_binary_parent() -> PathBuf {
    this_binary()
        .parent()
        .expect("the running binary has a parent directory")
        .to_path_buf()
}

/// The PRODUCT binary, which is what a hook stub must exec and what every
/// LATENCY row is measured against.
const PRODUCT_BIN: &str = env!("CARGO_BIN_EXE_gsd-meta-manager");

/// Run one shell command under real `bash`, from a cwd that is never the test
/// process's own, and report its exit code.
///
/// **This is the REACH leg, and it is what tells a bypass from a shape.** A
/// spelling the guard permits but the shell does not resolve to the file is not
/// a bypass, and every asserted row in section 1 has this leg beside it.
fn bash_reaches(command: &str) -> i32 {
    std::process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir("/tmp")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("bash is on PATH")
        .code()
        .unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// THE SIMULATION — `19-33`'s mandated designs, computed HERE so this file can
// check its own derived expectations against a tree that does not have them
// ---------------------------------------------------------------------------
//
// **Why a simulation is legitimate here and is not a second implementation.**
// Every derived-post-fix verdict in this file is a claim about what `19-33`'s
// MANDATED design answers. A claim nobody can compute is a claim nobody can
// check, and `19-27` measured a plan mandating a row that certified nothing.
// These functions are the designs' own definitions written as code, and
// section 9 uses them to prove that every fenced row in every file `19-33` may
// not edit keeps its verdict. **They are test-local and no production line
// depends on them.**

/// EVERY `/`-anchored substring of `word` — round 12's candidate set,
/// UNCHANGED by this round.
fn slash_anchored_candidates(word: &str) -> Vec<&str> {
    word.char_indices()
        .filter(|(_, character)| *character == '/')
        .map(|(index, _)| &word[index..])
        .collect()
}

/// `lexical_absolute_components` (`policy.rs:5587-5606`), mirrored byte for byte
/// so the simulation runs the SAME normalisation the rule already uses.
fn components(word: &str) -> Option<Vec<&str>> {
    let word = word.trim_start_matches("./");
    if !word.starts_with('/') {
        return None;
    }
    let mut out: Vec<&str> = Vec::new();
    for part in word.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// TODAY's path set, over the candidate scan: UNDER `dir`, or EQUAL to `binary`.
fn protected_today(word: &str, dir: Option<&str>, binary: Option<&str>) -> bool {
    for candidate in slash_anchored_candidates(word) {
        let Some(parts) = components(candidate) else {
            continue;
        };
        if let Some(dir) = dir {
            if let Some(dir_parts) = components(dir) {
                if !dir_parts.is_empty()
                    && parts.len() >= dir_parts.len()
                    && parts[..dir_parts.len()] == dir_parts[..]
                {
                    return true;
                }
            }
        }
        if let Some(binary) = binary {
            if let Some(binary_parts) = components(binary) {
                if !binary_parts.is_empty() && parts == binary_parts {
                    return true;
                }
            }
        }
    }
    false
}

/// `19-33`'s path set: today's, PLUS a proper ANCESTOR of `dir` that is itself
/// AT OR UNDER `root`.
///
/// **Written in its GENERAL form** — *a proper ancestor at or under the root* —
/// rather than as the one-path special case, so it stays correct if the envelope
/// directory ever becomes deeper and so a reader can check it against the code
/// rather than against an arithmetic.
fn protected_after_19_33(
    word: &str,
    dir: Option<&str>,
    root: Option<&str>,
    binary: Option<&str>,
) -> bool {
    if protected_today(word, dir, binary) {
        return true;
    }
    let (Some(dir), Some(root)) = (dir, root) else {
        return false;
    };
    let (Some(dir_parts), Some(root_parts)) = (components(dir), components(root)) else {
        return false;
    };
    if dir_parts.is_empty() || root_parts.is_empty() {
        return false;
    }
    slash_anchored_candidates(word).into_iter().any(|candidate| {
        let Some(parts) = components(candidate) else {
            return false;
        };
        // A PROPER ancestor of the envelope directory …
        let proper_ancestor = parts.len() < dir_parts.len() && parts[..] == dir_parts[..parts.len()];
        // … that is itself AT OR UNDER the envelope root.
        let at_or_under_root =
            parts.len() >= root_parts.len() && parts[..root_parts.len()] == root_parts[..];
        proper_ancestor && at_or_under_root
    })
}

// ===========================================================================
// SECTION 0 — the walk's NON-BLINDNESS control
// ===========================================================================

#[test]
fn the_positive_control_proves_the_walk_can_see_a_ledger_line_at_all() {
    // **Without this, every "the walk found nothing" below is unfalsifiable.**
    let envelope = TempDir::new().unwrap();
    let answer = ask(envelope.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 0,
        "`gh pr create --title x` is PERMITTED on the first attempt (the default per-run cap \
         is 1). stdout: {}",
        answer.stdout
    );
    let written = ledger_lines_anywhere(envelope.path());
    assert_eq!(
        written.len(),
        1,
        "the positive control must leave EXACTLY ONE pull-request ledger line under the walked \
         envelope root, which is what proves the walk is not blind. Walked listing:\n{}",
        listing(envelope.path())
    );
}

// ===========================================================================
// SECTION 1 — AXIS 1, THE WORD SET: the measurement that decides where the
// FIRST new boundary is
//
// **Every row here was driven at the guard AND probed under real `bash` before
// it was written.** A spelling the guard permits but the shell does not reach is
// not a bypass, and the reach leg is what tells them apart — so the rows whose
// reach was DEMONSTRATED are ASSERTED at their derived post-fix verdict and the
// rows whose reach was NOT demonstrated are RECORDED in section 14.
//
// **THE MECHANISM, READ OUT OF THE SOURCE RATHER THAN INFERRED:**
//
// ```text
// policy.rs:3564   let pathname_target = operator.is_some_and(|op| op.pathname_target);
// policy.rs:3578   match skip_redirection_target(&chars, op_start + op_len) {
// policy.rs:3581       if pathname_target && target.literal { redirections.push(...) }
// policy.rs:3205       (Some('<'), Some('<')) => (3, false),   // <<<   here-string
// ```
//
// The target word emits **no `Token`** — that is round 6's deletion model,
// unchanged and correct — and it is pushed onto `Segment::redirection_targets`
// only when the operator takes a PATHNAME. `<<<`, `<<`, `<<-`, `>&` and `<&` all
// answer `false`. **The word is right there in the text the guard read, and it
// is in NEITHER of the two classes `protected_carrier_named` iterates.**
//
// **THE REACH TABLE, MEASURED under real `bash` at this file's base:**
//
// ```text
// xargs rm -rf <<< <ENV>/alpha                REACHES  (the envelope directory was deleted)
// xargs rm -f  <<< <ledger>                   REACHES  (the ledger was deleted)
// xargs -I{} rm -rf {} <<< <ENV>/alpha        REACHES
// xargs rm -rf <<< "x <ENV>/alpha"            REACHES  (xargs splits the here-string on space)
// echo evil >&<ledger>                        REACHES  — the file was TRUNCATED and rewritten
// : >&<ledger>                                REACHES  — truncated to 0 bytes
// xargs rm -rf <<EOF\n<ENV>/alpha\nEOF        REACHES  — **but the guard ALREADY REFUSES IT**
// xargs rm -rf <<<ENV>/alpha  (delimiter)     DOES NOT — bash warns and the body is empty
// xargs rm -rf <<-<ENV>/alpha (delimiter)     DOES NOT — the same
// cat <&<ledger>                              DOES NOT — bash answers `ambiguous redirect`
// ```
//
// **THE HEREDOC BODY FORM IS A CORRECTION TO AUDIT 12, WRITTEN BESIDE RATHER
// THAN AS AN EDIT.** Audit 12 recorded that `xargs rm -rf <<EOF\n<ENV>/alpha\nEOF`
// deleted the directory under real `bash` — a claim about the HARM, with no
// guard verdict beside it. **Driven here, the guard answers exit 2
// `envelope_assertion_failed`.** `\n` is in `SEPARATORS` (`policy.rs:2422`), so
// the body's words are ordinary tokens of a SEGMENT OF THEIR OWN and round 10's
// clause already reaches them. The harm audit 12 measured is real; the guard
// verdict it did not measure is a REFUSAL. Audit 12's own subsection is not
// edited — this is recorded beside it, which is the discipline audit 12 used for
// its own three corrections.
//
// **THE PROCESS FINDING, ALSO WRITTEN BESIDE.** Audit 11 looked at this exact
// cell and recorded the five non-pathname operators as *"correctly NOT recorded
// as carriers, so the round bought its rule without an over-refusal"*, with
// `cat <<<X` as the witness. **That is right about `cat` and wrong as a general
// statement: a here-string is a DATA channel, and `xargs` is a program that
// turns data into argv.** It is the same shape as `19-28` seeing `T-19-119`'s
// spelling and calling it *"Not a defect"* — **a recorded-and-misgeneralised
// observation is how a finding survives a round that already saw it** — and
// neither subsection is edited to say so.
//
// **THE DISCLOSED COST OF THE THIRD WORD CLASS, STATED IN ADVANCE:**
// `cat <<< <ENV>/alpha/pr-ledger.ndjson` becomes REFUSED. It is the same family
// as the already-disclosed refusal of `cat <ledger>`, it fails CLOSED, and it is
// bounded to words naming this run's own two paths. `cat <<<x` and
// `cat <<< /tmp/x` stay permitted and both are pinned below.
// ===========================================================================

#[test]
fn after_19_33_a_here_string_word_naming_the_envelope_directory_is_refused() {
    // **THE WHOLE FINDING IN TWO LINES.** `rm -rf <ENV>/alpha` is exit 2 today;
    // `xargs rm -rf <<< <ENV>/alpha` — the same program-free deletion, differing
    // ONLY in whether the path stands as an OPERAND or after a `<<<` — is exit 0,
    // and section 2 drives it to a moved bare remote.
    //
    // **THE CONTROL AND WHAT MAKES THE PAIR DISCRIMINATING:** the twin differs in
    // exactly one thing — whether the word stands as an operand or after a
    // NON-PATHNAME redirection operator. The twin is asserted REFUSED in
    // section 13 as round 10's clause, so it cannot pass vacuously here.
    refuses_carrier(
        |root| format!("xargs rm -rf <<< {}/{ALIAS}", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-122` — A WORD IN NEITHER WORD CLASS, over the ENVELOPE DIRECTORY.**\n\n\
             **REACH, measured under real `bash`:** the envelope directory was DELETED.\n\n\
             **THE CONTROL:** `rm -rf <ENV>/alpha` is ALREADY exit 2 \
             `envelope_assertion_failed` today (round 10's clause, re-asserted in section 13). \
             The pair differs in exactly one property — operand position versus \
             non-pathname-redirection-target position.\n\n{WORD_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("xargs rm -f <<< {}/{ALIAS}/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-122` over the LEDGER — the same class, `T-19-112`'s harm by a FIFTH \
             route.** Section 3 drives it from a FIRED cap in one persistent root.\n\n\
             **THE CONTROL:** `rm -f <ledger>` is already exit 2 today.\n\n\
             {WORD_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("xargs -n1 rm -rf <<< {}/{ALIAS}/hooks", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**A SECOND `<<<` SPELLING, over the hook-stub directory** — SAFE-05's only \
             carrier and `AR-19-03`'s backstop. Drawn under a different `xargs` invocation so \
             the row cannot pass because of one program's argument grammar.\n\n\
             {WORD_SET_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_33_a_here_string_word_naming_the_guards_own_binary_is_refused() {
    // **THE BINARY HALF — `C-10`, layers 2 AND 3 in one permitted call.**
    // `word_is_exactly` is an EQUALITY and the third word class does not widen
    // it: the word must still normalise to the binary's own full path.
    //
    // **THE CONTROL:** `cp /bin/true <BINARY>` is already exit 2 today (round
    // 11's exact-path clause, re-asserted in section 13), and
    // `xargs cp /bin/true <<< <BINPAR>/some-other-file` is exit 0 before AND
    // after (section 5). The three rows differ in exactly one thing each: the
    // word position, and whether the path IS the binary.
    let binary = this_binary().display().to_string();
    refuses(
        &format!("xargs cp /bin/true <<< {binary}"),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-122` over the BINARY.** Audit 12 measured this at exit 0 and it \
             reproduces.\n\n\
             **`T-19-116` GAINS TWO ROUTES THIS AUDIT and closing `T-19-122` closes only \
             ONE of them.** This spelling is closed by the third word class; \
             `rm -rf <binary-parent>` is NOT, because no ancestor clause is written for the \
             binary and `word_is_exactly` stays an EQUALITY. That row is RECORDED in \
             section 14 and asserted in NEITHER direction.\n\n{WORD_SET_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_33_a_non_pathname_operator_other_than_a_here_string_reaches_the_same_way() {
    // **THE ROW THAT STOPS `19-33` WRITING A HERE-STRING-ONLY RULE.** An alphabet
    // and a corpus that only ever drew `<<<` would certify a rule keyed to one
    // operator — which is D-08's defect one level over, and exactly the shape
    // round 12 refused when it declined an `=`-keyed rule.
    //
    // **`>&` IS MEASURED, NOT ASSUMED.** Bash's `[n]>&word` duplicates a file
    // descriptor when `word` is digits — but when it is a PATH, bash treats the
    // redirection as `&>word` and OPENS THE FILE FOR WRITING. Measured here: a
    // three-line 18-byte ledger became a 5-byte file holding `evil`, and
    // `: >&<ledger>` truncated it to ZERO bytes. **That is `T-19-112`'s cap reset
    // under an operator `redirection_operator` correctly answers
    // `pathname_target = false` for**, because the GRAMMAR says it takes an fd
    // number — and bash's fallback is what makes it reach a file anyway.
    //
    // **THE CONTROL:** `: > <ledger>` — the SAME truncation through a PATHNAME
    // operator — is already exit 2 today (round 11's redirection clause,
    // re-asserted in section 13), and `echo x >&/tmp/plain-outside` is exit 0
    // before AND after (section 5). The pair differs in exactly one thing: which
    // side of bash's twelve-operator split the operator falls on.
    refuses_carrier(
        |root| format!(": >&{}/{ALIAS}/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-122` UNDER `>&` — a SECOND non-pathname operator, with a DRIVEN reach.**\n\n\
             **REACH, measured under real `bash`:** the ledger was truncated to ZERO bytes.\n\n\
             {WORD_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("echo evil >&{}/{ALIAS}/askpass", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`>&` again, over a SECOND carrier file.** Measured under real `bash`: the file \
             was truncated and rewritten with the redirected text.\n\n{WORD_SET_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_33_a_here_string_word_carrying_the_path_at_a_non_zero_index_is_refused() {
    // **THE COMPOSITION WITH ROUND 12's INTERIOR SCAN, DRAWN RATHER THAN
    // ASSUMED.** The third word class and the `/`-anchored scan must compose at
    // the SAME site rather than being two rules: the class decides WHICH WORDS
    // reach the reader, and the scan decides WHAT THE READER SEES IN one. A word
    // that is both a non-pathname redirection target AND carries its path at a
    // non-zero index exercises both at once.
    //
    // **ITS REACH IS NOT CLAIMED, AND THAT IS STATED RATHER THAN LEFT AS A GAP.**
    // `xargs` hands `of=<ENV>/alpha` to `rm -rf` as a literal filename, so no
    // file under the envelope directory is opened. **This row fences the rule
    // SHAPE rather than a measured bypass**, exactly as
    // `CONTROL_CARRIER_INTERIOR_PATH`'s entry 11 already does one axis over: a
    // rule that read only the WHOLE here-string word would miss it, and the
    // mandated design — the existing `/`-anchored scan applied to the new word
    // class — does not.
    //
    // **THE CONTROL:** `xargs rm -rf <<< of=/tmp/g/x` is exit 0 before AND after
    // (section 5). The pair differs only in whether the interior path is the
    // protected one.
    refuses_carrier(
        |root| format!("xargs rm -rf <<< of={}/{ALIAS}", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-122` × `T-19-119`, COMPOSED.** The word is a NON-PATHNAME redirection \
             target (axis 1) whose carrier path sits at a NON-ZERO index (axis 2). If this \
             row is red after `19-33` while the index-zero rows are green, the third word \
             class was chained SOMEWHERE OTHER than the site the `/`-anchored scan already \
             runs at — which is a finding about the DESIGN and is reported rather than \
             relaxed.\n\n{WORD_SET_DERIVATION}"
        ),
    );
}

#[test]
fn the_heredoc_body_form_is_already_refused_and_that_corrects_audit_12_beside_its_row() {
    // **GREEN today and after. This is a CORRECTION, not this round's RED.**
    //
    // Audit 12 records that `xargs rm -rf <<EOF\n<ENV>/alpha\nEOF` deleted the
    // directory under real `bash`, with no guard verdict beside it. **Driven
    // here it is exit 2 `envelope_assertion_failed`**, because `\n` is in
    // `SEPARATORS` (`policy.rs:2422`, ONE commit in the phase, `84a9b05`) — so
    // the body's words are ordinary tokens of a SEGMENT OF THEIR OWN and round
    // 10's clause already reaches them.
    //
    // **The harm audit 12 measured is real and the verdict it did not measure is
    // a REFUSAL.** Both halves are true and this file says so beside audit 12's
    // row rather than editing it.
    refuses_carrier(
        |root| format!("xargs rm -rf <<EOF\n{}/{ALIAS}\nEOF", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE HEREDOC BODY FORM.** `\\n` is a `SEPARATORS` entry, so the body is a segment \
         of its own carrying `<ENV>/alpha` as an ORDINARY OPERAND — round 10's class, not \
         round 13's. **A red here means `19-33` changed `SEPARATORS` or `is_separator`, \
         which is forbidden**, and the correct response is to report it as a finding about \
         the FIX rather than to relax this row.",
    );
    permits_carrier(
        |_| "xargs rm -rf <<EOF\n/tmp/plain-outside\nEOF".to_string(),
        "**THE CONTROL, and what makes the pair discriminating.** The same two-segment shape \
         with a body naming a path OUTSIDE the envelope root. Without it the row above would \
         pass on any rule that refused every heredoc.",
    );
}

#[test]
fn the_here_string_class_is_a_path_class_and_not_a_here_string_ban() {
    // **GREEN today and after — and this is the half that stops `19-33` writing
    // the THIRD, REJECTED design.** *Treat every deleted-and-unrecorded word as
    // UNRESOLVABLE* would deny `cat <<< hello` and `sort <<< "$x"`, which is an
    // outage rather than a boundary (AR-19-11). These rows turn THIS file red if
    // that design is taken.
    permits(
        "cat <<<x",
        "a here-string carrying NO path at all. **A rule that refused this is a here-string \
         BAN and not a path class**, and `19-33`'s mandated design cannot produce it: the new \
         word class goes through the SAME `lexical_absolute_components` normaliser and the \
         SAME two comparisons, and `x` normalises to nothing.",
    );
    permits(
        "cat <<< /tmp/x",
        "a here-string carrying an ABSOLUTE path that is NOT protected. **This is the row \
         that proves the class is bounded by the PATH SET rather than by the operator**, and \
         it is the exact analogue of `--git-dir=/tmp/g` one axis over.",
    );
    refuses(
        "git <<<x push --force origin main",
        "force_push_blocked",
        "**`T-19-97` / round 6's own row, which must not move by ONE CHARACTER.** The \
         redirection is DELETED and the surviving argv is still a force push. A rule that \
         pushed the here-string target into the token stream would split this segment and \
         turn round 6's headline refusal into a PERMIT — which is why the mandated design \
         carries the word on a SECOND `Segment` FIELD and never as a `Token`.",
    );
}

// ===========================================================================
// SECTION 2 — `T-19-122` DRIVEN END TO END, WITH A CONTROL BESIDE EVERY LEG
//
// **The bare-remote fixture is REBUILT here rather than cited.** Audit 12's
// numbers are its own; this file measures its own. Both remote SHAs are recorded
// and the envelope directory's absence is proved by a WALK.
//
// **GREEN today and after**, because it drives the PRE-FIX harm directly: the
// deletion is performed by this test, not by the guard. The guard row `19-33`
// turns green is section 1's.
// ===========================================================================

#[test]
fn the_t_19_122_here_string_deletion_moves_a_bare_remote_with_a_control_on_both_sides() {
    let fixture = TempDir::new().unwrap();
    let root = fixture.path();

    // A private copy of the product binary. NEVER the build artefact itself.
    let binary = root.join("gsd-meta-manager");
    std::fs::copy(PRODUCT_BIN, &binary).expect("the product binary copies");

    let envelope = root.join("env");
    let hooks_dir = hooks::install_in(&envelope, ALIAS, &binary)
        .expect("the fixture installs both stubs naming the copied binary");
    let stub = std::fs::read_to_string(hooks_dir.join("pre-push")).unwrap();
    assert!(
        stub.contains("envelope pre-push"),
        "the fixture's stub must be the real `stub_body` shape, got:\n{stub}"
    );

    let remote = root.join("remote.git");
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    git_ok(root, &["init", "--bare", "-q", &remote.display().to_string()]);
    git_ok(root, &["init", "-q", "-b", "main", &work.display().to_string()]);
    git_ok(&work, &["commit", "-q", "--allow-empty", "-m", "one"]);
    git_ok(&work, &["commit", "-q", "--allow-empty", "-m", "two"]);
    git_ok(
        &work,
        &["remote", "add", "origin", &remote.display().to_string()],
    );
    git_ok(&work, &["push", "-q", "origin", "main"]);
    git_ok(&work, &["checkout", "-q", "-b", "gsd-auto/alpha/w"]);
    git_ok(&work, &["commit", "-q", "--allow-empty", "-m", "three"]);

    // The hooks are delivered exactly as the envelope delivers them, through the
    // `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0` / `GIT_CONFIG_VALUE_0` triplet.
    let force_push = |label: &str| -> bool {
        let status = std::process::Command::new("git")
            .args(["push", "--force", "origin", "HEAD:refs/heads/main"])
            .current_dir(&work)
            .env("GSD_MM_ENVELOPE_ROOT", &envelope)
            .env("GIT_CONFIG_COUNT", "1")
            .env("GIT_CONFIG_KEY_0", "core.hooksPath")
            .env("GIT_CONFIG_VALUE_0", &hooks_dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("git is on PATH");
        println!("  [{label}] git push --force exit={:?}", status.code());
        status.success()
    };

    // -- CONTROL, before: the stubs in place.
    let before = rev(&remote, "main");
    assert!(
        !force_push("CONTROL hooks present"),
        "the CONTROL leg must be REFUSED by the pre-push hook. If it is not, the fixture is not \
         delivering the hook and every leg below measures nothing."
    );
    assert_eq!(
        rev(&remote, "main"),
        before,
        "the CONTROL leg must leave the remote's `main` UNMOVED. before={before}"
    );

    // -- **THE TWO-LINE FINDING, DRIVEN AT THE GUARD.**
    let dir = envelope.join(ALIAS);
    let here_string = ask(&envelope, &format!("xargs rm -rf <<< {}", dir.display()));
    let separated = ask(&envelope, &format!("rm -rf {}", dir.display()));
    println!(
        "  [TWIN PAIR] `xargs rm -rf <<< <ENV>/alpha` exit={}   `rm -rf <ENV>/alpha` exit={}",
        here_string.code, separated.code
    );
    println!(
        "  RECORDED (not asserted) [the here-string verdict itself]: exit={} at this file's \
         base. **It is ASSERTED at its DERIVED post-fix verdict in section 1**, not here — a \
         row asserted at the PRE-fix verdict in a file `19-33` may only ADD to would land \
         permanently red the day the rule lands. The deletion below is performed directly, so \
         THIS test's harm keeps reproducing after the rule closes the spelling.",
        here_string.code
    );
    // **BOTH LEGS ARE DRIVEN AGAINST THE FIXTURE'S OWN ENVELOPE ROOT, and that
    // is the only way the pair means anything.** `word_is_within` compares
    // against the directory THE GUARD WAS HANDED — `<GSD_MM_ENVELOPE_ROOT>/alpha`
    // — so a pair driven against a DIFFERENT root would measure two silences
    // rather than one refusal and one permit. The twin's REFUSAL is the control
    // and it is asserted; the here-string's verdict is RECORDED here and
    // ASSERTED at its derived post-fix verdict in section 1.
    assert_eq!(
        separated.code, 2,
        "\n\n**THE OPERAND TWIN MUST BE REFUSED.** It is round 10's clause and it is the \
         control this whole finding is measured against. Without it, the here-string leg's \
         exit 0 could be explained by the guard being silent about the path altogether. \
         stdout: {}",
        separated.stdout
    );

    // -- LEG B: the envelope deleted THROUGH THE PERMITTED SPELLING, under real
    //    `bash`, exactly as the guard let it through.
    let performed = bash_reaches(&format!("xargs rm -rf <<< {}", dir.display()));
    assert_eq!(
        performed, 0,
        "the permitted here-string deletion must succeed under real `bash`"
    );
    let walked = envelope_files(&envelope);
    assert!(
        walked.is_empty(),
        "\n\n**THE ENVELOPE DIRECTORY MUST BE GONE, PROVED BY A WALK** rather than by a read \
         of one expected path. Found: {walked:?}"
    );
    println!("  [LEG B] the envelope directory was deleted; the walk found nothing");

    assert!(
        force_push("LEG B envelope deleted"),
        "\n\n**`T-19-122` DID NOT REPRODUCE END TO END.**\n\n\
         With the hook stubs deleted through the permitted here-string spelling, the force \
         push must COMPLETE. A refusal here means the fixture is not exercising the carrier \
         and the finding must be RE-MEASURED rather than asserted at audit 12's number."
    );
    let after = rev(&remote, "main");
    assert_ne!(
        after, before,
        "\n\n**THE REMOTE MUST HAVE MOVED.**\n  before : {before}\n  after  : {after}\n\n\
         The deletion removes LAYER 3 without touching `GIT_CONFIG_COUNT` at all: \
         `core.hooksPath` still points at a directory, and the directory is no longer there."
    );
    println!("  [LEG B] remote main {before} -> {after}  MOVED");

    // -- CONTROL, after: reinstall the stubs, rewind the remote, same push.
    hooks::install_in(&envelope, ALIAS, &binary).expect("the stubs reinstall");
    git_ok(
        root,
        &[
            "--git-dir",
            &remote.display().to_string(),
            "update-ref",
            "refs/heads/main",
            &before,
        ],
    );
    assert!(
        !force_push("CONTROL hooks restored"),
        "restoring the stubs must restore the refusal. Without this leg, LEG B could be \
         explained by anything that happened between the two pushes."
    );
    assert_eq!(
        rev(&remote, "main"),
        before,
        "the restored-hooks control must leave the remote's `main` UNMOVED again"
    );
}

// ===========================================================================
// SECTION 3 — `T-19-122` RE-DRIVEN ON THE LEDGER, FROM A FIRED CAP
//
// **ONE PERSISTENT ROOT across all five calls**, because a fresh root per row
// would destroy the measurement: the cap only fires against a ledger that
// already holds this run's own attempts.
//
// **Measured at this file's base:**
//
// ```text
// call 1  gh pr create --title x   -> exit 0 (permit),        ledger 1 line
// call 2  gh pr create --title x   -> exit 2 pr_cap_exceeded, ledger 2 lines
// call 3  gh pr create --title x   -> exit 2 pr_cap_exceeded, ledger 3 lines
// call 4  gh pr create --title x   -> exit 2 pr_cap_exceeded, ledger 4 lines
// GUARD   rm -f <ledger>           -> exit 2   (the TWIN)
// GUARD   xargs rm -f <<< <ledger> -> exit 0
// GUARD   : >&<ledger>             -> exit 0   (the SECOND operator)
// GUARD   rm -rf <ENV>             -> exit 0   (the ANCESTOR, section 4's class)
// (the permitted spelling performed) ledger GONE
// call 5  gh pr create --title x   -> exit 0 (PERMITTED AGAIN), ledger 1 line
// ```
//
// **GREEN today and after**, for the same reason as section 2.
// ===========================================================================

#[test]
fn the_t_19_122_here_string_deletion_resets_a_fired_pr_cap_in_one_persistent_root() {
    let envelope = TempDir::new().unwrap();
    let root = envelope.path();
    let ledger = root.join(ALIAS).join("pr-ledger.ndjson");
    let lines = || {
        std::fs::read_to_string(&ledger)
            .map(|text| text.lines().count())
            .unwrap_or(0)
    };

    let mut trail: Vec<String> = Vec::new();
    for call in 1..=4 {
        let answer = ask(root, "gh pr create --title x");
        trail.push(format!(
            "call {call}: exit={} lines={} reason={}",
            answer.code,
            lines(),
            answer.reason().lines().next().unwrap_or_default()
        ));
        if call == 1 {
            assert_eq!(answer.code, 0, "call 1 must be PERMITTED: {trail:?}");
        } else {
            assert_eq!(answer.code, 2, "call {call} must be REFUSED: {trail:?}");
            assert!(
                answer.reason().contains("pr_cap_exceeded"),
                "call {call} must be refused UNDER `pr_cap_exceeded`, not some other cause \
                 (D-24): {trail:?}"
            );
        }
    }
    assert_eq!(
        lines(),
        4,
        "four attempts must have left four ledger lines. The cap has FIRED. {trail:?}"
    );

    // -- **ALL FOUR SPELLINGS, DRIVEN AGAINST THIS SAME ROOT BEFORE ANY IS
    //    PERFORMED.** The operand twin is the CONTROL and it is asserted; the
    //    three permitted spellings are RECORDED here and ASSERTED at their
    //    DERIVED post-fix verdicts in sections 1 and 4.
    let separated = ask(root, &format!("rm -f {}", ledger.display()));
    let here_string = ask(root, &format!("xargs rm -f <<< {}", ledger.display()));
    let fd_dup = ask(root, &format!(": >&{}", ledger.display()));
    let ancestor = ask(root, &format!("rm -rf {}", root.display()));
    println!(
        "  [FOUR SPELLINGS] `rm -f <ledger>` exit={}   `xargs rm -f <<< <ledger>` exit={}   \
         `: >&<ledger>` exit={}   `rm -rf <ENV>` exit={}",
        separated.code, here_string.code, fd_dup.code, ancestor.code
    );
    assert_eq!(
        separated.code, 2,
        "the OPERAND twin must be REFUSED — round 10's clause. It is the control all three \
         permitted spellings are measured against, and without it their exit 0 could be \
         explained by the guard being silent about the ledger altogether."
    );
    println!(
        "  RECORDED (not asserted) [the three permitted spellings' verdicts themselves]: \
         here-string exit={}, `>&` exit={}, ancestor exit={} at this file's base. **Each is \
         ASSERTED at its DERIVED post-fix verdict in section 1 or section 4**, never here.",
        here_string.code, fd_dup.code, ancestor.code
    );

    // -- Perform the permitted spelling, exactly as the guard let it through.
    let performed = bash_reaches(&format!("xargs rm -f <<< {}", ledger.display()));
    assert_eq!(performed, 0, "the permitted here-string deletion succeeds");
    assert!(
        !ledger.exists(),
        "the ledger must be gone after the permitted deletion"
    );

    let fifth = ask(root, "gh pr create --title x");
    println!(
        "  call 5 after the deletion: exit={} lines={}",
        fifth.code,
        lines()
    );
    assert_eq!(
        fifth.code, 0,
        "\n\n**THE CAP MUST BE RESET.**\n\n\
         After the deletion a fifth `gh pr create --title x` must be PERMITTED again — the \
         run has reset its own limit through a command the guard allowed. That is \
         `T-19-112`'s harm reached by a FIFTH route, and closing `T-19-122` narrows it \
         without closing it. Trail: {trail:?}"
    );
    assert_eq!(
        lines(),
        1,
        "and the fresh ledger must hold exactly one line. Walked listing:\n{}",
        listing(root)
    );
}

// ===========================================================================
// SECTION 4 — AXIS 3, THE PATH SET: `T-19-123`, a word naming an ANCESTOR
//
// `rm -rf <ENV>` is **exit 0**; `rm -rf <ENV>/alpha` is **exit 2**.
// `word_is_within` (`policy.rs:5685-5711`) compares COMPONENT VECTORS and
// returns early on `word.len() < dir.len()` (`:5706`), so every ANCESTOR of
// `<root>/<alias>` — being SHORTER — answers `false`.
// **`rm -rf <root>` takes the same nine carriers in ONE call that the prefix
// boundary exists to protect**, one component up.
//
// **AND THE COMPOSITE `protected_carrier_named`'s OWN DOC CLAIMS AS A NARROWING
// IS DEFEATED BY IT.** The doc says `cd <env>/<alias> && rm -f pr-ledger.ndjson`
// *"is refused BY SEGMENT ONE"* — measured TRUE here — but
// `cd <root> && rm -rf alpha` is exit 0, because segment one names the ANCESTOR.
// The pair is the finding and both legs are driven side by side.
//
// **THE ROOT IS ONE `dirname` FROM THE REFUSAL MESSAGE ITSELF.**
// `git config --get core.hooksPath` is PERMITTED and reports `<root>/<alias>/hooks`,
// and `envelope_carrier_refusal` names `<root>/<alias>` in the refusal text.
//
// **THIS IS NOT A BUG FOUND. IT IS A DECISION WHOSE HARM WAS NEVER MODELLED**,
// and it is PINNED PERMITTED BY NAME in TWO places:
//
// ```text
// policy.rs:10022-10026        ("/tmp/envroot", "the PARENT of the envelope directory is not
//                               under it. The boundary is the directory this run owns, not
//                               everything beside it")
// tests/envelope_interior_path.rs:2341
//                              ("/tmp/envroot", "the PARENT of the envelope directory is not
//                               under it")
// ```
//
// **BOTH ARE GREEN AND NON-VACUOUS TODAY AND THIS PLAN MOVES NEITHER.** Audit 12
// says plainly that moving them is a DESIGN CHANGE and not a test edit, so
// `19-33` moves them as a NAMED, BOUNDED, CROSS-FENCE exception with each pin's
// reasoning REWRITTEN rather than deleted — the shape round 11 used for
// `direction_i_…_stays_permitted`. **This plan names the exception in advance and
// performs none of it.**
// ===========================================================================

#[test]
fn after_19_33_a_word_naming_the_ancestor_of_the_envelope_directory_is_refused() {
    // **THE WHOLE FINDING IN TWO LINES.** `rm -rf <ENV>/alpha` is exit 2;
    // `rm -rf <ENV>` — one component up, the same program, the same operand
    // position — is exit 0, and it deletes strictly MORE.
    //
    // **THE CONTROL AND WHAT MAKES THE PAIR DISCRIMINATING:** the twin differs in
    // exactly one path component — whether the word names the envelope DIRECTORY
    // or its PARENT. The twin is asserted REFUSED in section 13 as round 10's
    // clause, and `ls <ENV>/unrelated-sibling` is asserted PERMITTED in section 5,
    // so the clause cannot be satisfied by a prefix widened to the root.
    refuses_carrier(
        |root| format!("rm -rf {}", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-123` — THE ANCESTOR, over the ENVELOPE ROOT.**\n\n\
             **REACH, measured under real `bash`:** the root was deleted and the envelope \
             directory with it.\n\n\
             **THE CONTROL:** `rm -rf <ENV>/alpha` is ALREADY exit 2 today. The pair differs \
             in exactly ONE PATH COMPONENT.\n\n{PATH_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("rm -rf {}/", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**THE TRAILING-SLASH SPELLING.** `lexical_absolute_components` collapses a \
             trailing `/` to the empty component and drops it, so this normalises to exactly \
             the same component vector as the row above — which is what makes it a spelling \
             rather than a second case.\n\n{PATH_SET_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_33_the_ancestor_spellings_that_are_not_rm_are_refused_the_same_way() {
    // **THE RULE READS A PATH AND NEVER A PROGRAM NAME**, and drawing the
    // ancestor under four further programs is the corpus's own proof of it.
    // `mv`, `find`, `chmod` and `tar` are all absent from both production halves
    // (`the_carrier_rule_still_reads_a_path_and_never_a_program_name` in
    // section 13 counts that rather than trusting this comment).
    //
    // **EVERY ONE OF THE FOUR REACHES, measured under real `bash`:** `mv` moved
    // the whole root away; `find -delete` emptied it; `chmod 000` made the
    // envelope directory inaccessible; and `tar -C<ENV> --create` archived
    // `./alpha/pr-ledger.ndjson` — a READ of the whole envelope.
    //
    // **THE CONTROL for each:** the identical spelling one component deeper is
    // ALREADY exit 2 today, and the identical spelling over `/tmp/g` is exit 0
    // before AND after (section 5).
    for (template, why) in [
        (
            "mv {} /tmp/gone",
            "a MOVE rather than a delete: the carriers survive, at a path nothing this run \
             holds points at",
        ),
        (
            "find {} -delete",
            "a second deletion program the guard has never heard of",
        ),
        (
            "chmod 000 {}",
            "a DENIAL rather than a deletion — the envelope directory becomes unreadable and \
             every control under it stops answering",
        ),
        (
            "tar -C{} --create --file /tmp/t .",
            "**THE ANCESTOR COMPOSED WITH ROUND 12's INTERIOR SCAN** — the ancestor path sits \
             at a NON-ZERO index behind an attached short option with no `=` at all. If this \
             row is red after `19-33` while the index-zero rows are green, the ancestor clause \
             was written OUTSIDE the per-candidate walk, which is a finding about the DESIGN",
        ),
    ] {
        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            &format!("**`T-19-123`:** {why}.\n\n{PATH_SET_DERIVATION}"),
        );
    }
}

#[test]
fn after_19_33_the_two_segment_composite_naming_the_ancestor_is_refused() {
    // **THE ROW THAT DEFEATS THE NARROWING `protected_carrier_named`'s OWN DOC
    // CLAIMS, WITH THE DOC'S OWN ROW DRIVEN BESIDE IT.** The doc says
    // `cd <env>/<alias> && rm -f pr-ledger.ndjson` *"is refused BY SEGMENT ONE"*.
    // True — and one component up, segment one names the ancestor and the whole
    // composite is permitted.
    //
    // **THE PAIR IS THE FINDING**, so both legs are driven here rather than in
    // separate tests.
    refuses_carrier(
        |root| format!("cd {}/{ALIAS} && rm -f pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE DOC'S OWN CLAIMED NARROWING, DRIVEN.** `protected_carrier_named`'s doc says \
         this composite is refused by SEGMENT ONE, and it is — GREEN today and after. It is \
         the CONTROL for the row below, and what makes the pair discriminating is exactly one \
         path component in segment one.",
    );
    refuses_carrier(
        |root| format!("cd {} && rm -rf {ALIAS}", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-123` DEFEATS THE COMPOSITE NARROWING.** Segment one names the ANCESTOR, \
             so nothing refuses it; segment two is a RELATIVE operand the guard has no cwd \
             for. **The narrowing the doc claims holds one component deeper and not here**, \
             and that sentence is `19-33`'s to repair.\n\n{PATH_SET_DERIVATION}"
        ),
    );
}

// ===========================================================================
// SECTION 5 — THE ANCESTOR STOP, PINNED FROM BOTH SIDES
//
// **This is what makes the design a BOUNDARY rather than an ESCALATION.** Every
// row here is exit 0 today and asserted exit 0 AFTER, so a rule that reached
// past the root — or that widened the existing PREFIX to the root instead of
// adding an ANCESTOR clause beside it — turns THIS FILE red rather than turning
// a driven run unusable (AR-19-11).
//
// ```text
//                                     ancestor clause (mandated)  prefix widened to <root> (forbidden)
// rm -rf <ENV>                        REFUSED                     REFUSED
// rm -rf <ENV>/alpha                  REFUSED                     REFUSED
// ls <ENV>/unrelated-sibling          PERMITTED                   REFUSED   <- the discriminator
// GSD_MM_ENVELOPE_ROOT set by the
// user to a shared directory;
// ls <that>/anything-at-all           PERMITTED                   REFUSED   <- the whole subtree
// ```
//
// **`GSD_MM_ENVELOPE_ROOT` IS USER-SETTABLE, so the second row is reachable
// CONFIGURATION rather than a thought experiment**, and it is driven below.
//
// **NO ANCESTOR CLAUSE IS WRITTEN FOR THE BINARY, and that is stated rather than
// left to be noticed.** `word_is_exactly`'s permit is explicitly reasoned — a
// prefix over the binary's parent would refuse `ls <parent>` and every
// `cargo install` — **but the reasoning argues only that a PREFIX would be wrong;
// it never states that the parent's own DELETION removes the binary.** That is
// an HONESTY gap rather than a rule gap: `rm -rf <binary-parent>` is exit 0, it
// is a `T-19-116` route audit 12 added, and it is RECORDED in section 14 and
// asserted in NEITHER direction.
// ===========================================================================

#[test]
fn the_ancestor_stop_is_pinned_from_both_sides_before_and_after() {
    // -- **THE ANCESTOR-vs-PREFIX DISCRIMINATOR.** A prefix widened from
    //    `<ENV>/alpha` to `<ENV>` refuses these; the mandated ancestor clause
    //    does not, because a SIBLING under the root is neither UNDER the envelope
    //    directory nor an ANCESTOR of it.
    for (template, why) in [
        (
            "ls {}/unrelated-sibling",
            "a file directly under the root that is not an alias directory. **A prefix widened \
             to the root refuses this and the ancestor clause does not** — that is the whole \
             difference between the two designs, in one row",
        ),
        (
            "rm -f {}/unrelated-sibling",
            "the same discriminator as a WRITE, so the row cannot pass because reads happen to \
             be treated differently",
        ),
        (
            "ls {}/beta",
            "a SECOND alias directory under the same root. This run owns `<root>/alpha`; a \
             concurrent run owns `<root>/beta`, and refusing it would make two envelopes on \
             one machine deny each other",
        ),
        (
            "rm -rf {}/beta",
            "the same, as a deletion",
        ),
    ] {
        let template = template.to_string();
        permits_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            &format!(
                "**THE ANCESTOR-vs-PREFIX DISCRIMINATOR:** {why}.\n\n\
                 A red here means `19-33` widened the existing PREFIX to `<root>` instead of \
                 adding an ANCESTOR clause beside it. **The correct response is to change the \
                 RULE, never to relax this row.**"
            ),
        );
    }

    // -- **THE STOP ITSELF.** No rule is written above the root, and these rows
    //    are what a clause that walked the whole ancestor chain to `/` would turn
    //    red. `ls /`, `df /`, `du -sh $HOME` and `ls /tmp` must all stay
    //    permitted, and a boundary that refused them is not a boundary but an
    //    OUTAGE (AR-19-11).
    for command in ["ls /tmp", "ls /", "df /", "ls /home"] {
        permits(
            command,
            "**THE ANCESTOR STOP.** The clause stops at `<root>` and the reason is OWNERSHIP: \
             `<root>` is created by this tool and holds only alias directories it created, \
             while `~/.local/share`, `$HOME`, `/tmp` and `/` are shared with everything the \
             user has. **A clause that walked the chain past the root turns this row red.**",
        );
    }
    permits_carrier(
        |root| {
            format!(
                "ls {}",
                root.parent()
                    .expect("a temporary envelope root has a parent")
                    .display()
            )
        },
        "**THE ROOT'S OWN PARENT** — the first path ABOVE the stop, driven by name rather \
         than by a literal. Its refusal is `T-19-123`'s RESIDUE: an ancestor above `<root>` \
         reaches the same nine carriers and NO RULE IS WRITTEN FOR IT. It is registered, \
         disclosed and unaccepted, and named on axis 3 of the restated condition.",
    );

    // -- **THE USER-SET-ROOT CASE, which is reachable CONFIGURATION.** The guard
    //    is driven with `GSD_MM_ENVELOPE_ROOT` pointed at a directory that also
    //    holds unrelated files, and a word naming one of them stays permitted.
    //    **A prefix-to-the-root implementation refuses the whole subtree here.**
    let shared = TempDir::new().unwrap();
    std::fs::write(shared.path().join("someone-elses-file"), "x").unwrap();
    std::fs::create_dir_all(shared.path().join("someone-elses-dir")).unwrap();
    for command in ["ls {}/someone-elses-file", "rm -rf {}/someone-elses-dir"] {
        let full = command.replace("{}", &shared.path().display().to_string());
        let answer = ask(shared.path(), &full);
        assert_eq!(
            answer.code, 0,
            "\n\n**THE USER-SET-ROOT CASE MUST STAY PERMITTED.**\n  command: {full}\n\n\
             `GSD_MM_ENVELOPE_ROOT` is user-settable, so a user may point it at a directory \
             that already holds unrelated files — `/tmp` being the extreme case. **The \
             ancestor clause under that configuration refuses only the single word naming the \
             root; a prefix widened to the root refuses the whole subtree.** This is the row \
             that turns red rather than a driven run.\n  stdout: {}",
            answer.stdout
        );
    }

    // -- **THE BINARY's EXACT-PATH CONTROLS, UNCHANGED because no ancestor
    //    clause is written for the binary.**
    permits(
        &format!(
            "cp /bin/true {}/some-other-file",
            this_binary_parent().display()
        ),
        "a SIBLING of the binary in the same shared directory. **`word_is_exactly` is an \
         EQUALITY and this round does not widen it**, and a clause written as a directory \
         prefix over the binary's parent turns this red.",
    );
    permits(
        &format!("ls {}", this_binary_parent().display()),
        "a READ of the binary's own directory — under `cargo test` that is \
         `target/debug/deps`. **A prefix over it would refuse `ls ~/.cargo/bin` and every \
         `cargo install`**, which is how a safety control gets switched off (AR-19-11).",
    );

    // -- **THE WORD-SET half's own cost controls**, so the third word class is
    //    bounded by the PATH SET rather than by the OPERATOR.
    permits(
        "xargs rm -rf <<< of=/tmp/g/x",
        "a here-string carrying an interior path OUTSIDE the path set — the \
         outside-the-path-set twin of section 1's non-zero-index row, in the SAME attachment \
         and the SAME word position.",
    );
    permits(
        "echo x >&/tmp/plain-outside",
        "a `>&` target naming an unprotected path — the outside-the-path-set twin of section \
         1's `>&` rows. **A rule that refused this would be an OPERATOR ban rather than a \
         path class.**",
    );
    permits(
        "git --git-dir=/tmp/g status",
        "pinned PERMITTED in FOUR other files and named in FIVE `policy.rs` doc sites. A rule \
         that turned it red would turn five files red at once.",
    );
}

// ===========================================================================
// SECTION 6 — `T-19-124`: ROUND 12's OWN MECHANISM BOUNDING THE WRONG QUANTITY
//
// `slash_anchored_candidates` (`policy.rs:5658-5662`) is applied PER CANDIDATE
// inside `word_is_within` (`:5697`) and `word_is_exactly` (`:5760`), so
// `lexical_absolute_components` used to run ONCE PER WORD and now runs ONCE PER
// `/`. **The scan is QUADRATIC on the guard's own critical path.**
//
// **MEASURED HERE, HEAD against a REBUILT PRE-ROUND-12 CONTROL BINARY at
// `dd17bfb`** (extracted with `git archive dd17bfb | tar -x -C <tmpdir>` OUTSIDE
// the repository and built there — the working tree was never checked out, reset
// or stashed), same input, same machine, one word of the form `/a/a/a…`:
//
// ```text
// slashes        bytes        HEAD        dd17bfb
//   1 000        2 000          68 ms       5 ms
//   2 000        4 000         249 ms       5 ms
//   5 000       10 000       1 524 ms       7 ms
//   8 000       16 000       3 904 ms       8 ms
//   9 000       18 000       4 957 ms       —      <- GUARD_TIMEOUT_SECS = 5 is crossed
//  10 000       20 000       6 099 ms       —         between 18 KB and 20 KB HERE
//  20 000       40 000      24 425 ms      14 ms
//  50 000      100 000     151 249 ms      28 ms
// 400 000      800 000   DID NOT ANSWER in 300 s (exit 124, hard external timeout)
// 100 000 chars, NO slashes    29 ms      21 ms  <- the DECISIVE control
// ```
//
// **`MAX_GUARD_REQUEST_BYTES = 1 MiB` (`hooks.rs:733`) BOUNDS THE INPUT AND
// BOUNDS NO WORK AT ALL** — which is `T-19-120`'s own shape, a bound over the
// wrong property, in the round that fixed `T-19-120`. Measured: a request whose
// command is 1 MiB of `/a` is refused in 20 ms by that bound, so the WORST CASE
// is a request just UNDER it — and 800 KB did not answer in five minutes.
//
// **AUDIT 12's OWN PARENTHETICAL REMEDY IS INSUFFICIENT, AND THE ARITHMETIC IS
// SHORT ENOUGH TO WRITE OUT.** The suggestion is *"a cap on the number of
// `/`-anchored candidates per word (or an early return once a candidate is
// shorter than the shortest protected path, which is the same
// `word.len() < dir.len()` test hoisted to the byte level)"*.
//
// * **The parenthetical half does not restore linearity.** A byte floor at the
//   shortest protected path removes only the candidates SHORTER than that floor.
//   For a 16 KB word `/a/a/a…` with 8 000 slashes, candidate `i` has length
//   `2 * (8000 - i)`; with an envelope directory of about 18 bytes the floor
//   removes exactly those `i` with `2 * (8000 - i) < 18`, i.e. `i > 7991` —
//   **NINE candidates out of EIGHT THOUSAND.** The other 7 991 each still cost
//   O(word length) and the measured 3 904 ms does not move. **A floor prunes the
//   CHEAP END of a quadratic.**
// * **A candidate CAP is a bound on work and does fail closed — but it must be
//   over a quantity the request cannot MULTIPLY.** A per-WORD cap is amplified by
//   the word count and a per-SEGMENT cap by the segment count: a 1 MiB request of
//   many medium words, each spending exactly the cap, defeats either.
//
// **SO THE PROPERTY `19-33` MUST SATISFY, AND THE ONE THIS SECTION PINS, IS
// LINEARITY — with a FAIL-CLOSED WORK CEILING beside it as the backstop**,
// refusing at `ParkReason::EnvelopeAssertionFailed` when a future edit reinstates
// a super-linear scan rather than overrunning the deadline. `19-33` chooses the
// implementation and measures it against this curve on all three trees; **this
// section writes the property and asserts no implementation.**
//
// **THE BEHAVIOURAL HALF — what the agent CLI does with a `PreToolUse` hook that
// OVERRUNS its timeout — is UNMEASURED and is claimed in NEITHER direction**, on
// the same discipline `C-08`'s, `T-19-117`'s and `T-19-120`'s behavioural halves
// are held to. It is recorded in section 14.
// ===========================================================================

/// Drive the PRODUCT binary under a HARD external timeout and report both its
/// exit code and its wall time.
///
/// **The PRODUCT binary rather than `guard_in` in-process**, because the claim
/// is about what a `PreToolUse` hook costs the agent CLI, and because a scan
/// that never returned would hang the whole test binary.
fn ask_binary_under_timeout(root: &Path, command: &str, seconds: u32) -> (i32, u128) {
    use std::io::Write as _;
    let request = serde_json::json!({
        "session_id": "fixture",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string();
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new("timeout")
        .arg(seconds.to_string())
        .arg(PRODUCT_BIN)
        .args(["envelope", "guard", ALIAS])
        .env("GSD_MM_ENVELOPE_ROOT", root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("`timeout` and the product binary are on PATH");
    let _ = child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(request.as_bytes());
    let code = child.wait().expect("the child exits").code().unwrap_or(-1);
    (code, started.elapsed().as_millis())
}

/// One word of the form `/a/a/a…` carrying exactly `slashes` slashes.
fn slash_dense_word(slashes: usize) -> String {
    "/a".repeat(slashes)
}

/// The ratio's threshold. **Derived rather than chosen:** four times the input
/// predicts about **4×** under a LINEAR scan and about **16×** under a QUADRATIC
/// one, so **8** sits halfway between them on a log scale and leaves a factor of
/// two of margin in BOTH directions. Measured **15.7** at this file's base.
const LINEARITY_RATIO_CEILING: f64 = 8.0;

#[test]
fn after_19_33_the_candidate_scan_is_linear_in_the_words_length() {
    // ===================================================================
    // **THE SEVERANCE-RETIRABLE ROW. IT CARRIES THIS ASSERTION AND NOTHING
    // ELSE, AND THAT IS A CONTRACT RATHER THAN A CONVENTION.**
    //
    // `19-33`'s work-bound task is SEVERABLE — `T-19-124` is `medium` and
    // non-blocking — and **a severance that cannot be taken is not a
    // severance.** So this fn carries the linearity ratio and no other row:
    // the no-slash control, the curve record and every other `T-19-124` row
    // live in SEPARATE fns, **so that under a severance the failure set is
    // EXACTLY this one name** and `19-33`'s two gates can each branch on it.
    //
    // This is the seam that halted `19-23` and needed a human authorisation
    // twice since. It earns a named contract rather than a precedent.
    // ===================================================================
    //
    // **THE PROPERTY IS LINEARITY, NOT SPEED, because speed is a fact about a
    // MACHINE and linearity is a fact about the CODE.** The row is a RATIO
    // between two input sizes on the same machine in the same run.
    //
    // **PROCESS STARTUP IS A FIXED ADDITIVE TERM IN BOTH MEASUREMENTS AND IT
    // ONLY EVER MOVES THE RATIO DOWN, so it cannot manufacture a red.** With
    // `t(n) = spawn + work(n)`, adding the same `spawn` to numerator and
    // denominator strictly reduces the quotient. Measured here: `spawn` is
    // about 5 ms (the `dd17bfb` control binary's whole answer at 2 000
    // slashes) against a 249 ms denominator.
    let envelope = TempDir::new().unwrap();
    let (small_code, small) = ask_binary_under_timeout(
        envelope.path(),
        &format!("ls {}", slash_dense_word(2_000)),
        120,
    );
    let (large_code, large) = ask_binary_under_timeout(
        envelope.path(),
        &format!("ls {}", slash_dense_word(8_000)),
        120,
    );
    let ratio = large as f64 / small.max(1) as f64;
    println!(
        "  LINEARITY: t(2000 slashes) = {small} ms (exit {small_code}), \
         t(8000 slashes) = {large} ms (exit {large_code}), ratio = {ratio:.2} \
         (ceiling {LINEARITY_RATIO_CEILING})"
    );
    assert!(
        ratio < LINEARITY_RATIO_CEILING,
        "\n\n**THE CANDIDATE SCAN IS SUPER-LINEAR IN THE WORD'S LENGTH.**\n\
         \n  t(2 000 slashes) : {small} ms\
         \n  t(8 000 slashes) : {large} ms\
         \n  ratio            : {ratio:.2}   (ceiling {LINEARITY_RATIO_CEILING})\n\n\
         **FOUR TIMES THE INPUT PREDICTS ABOUT 4× UNDER A LINEAR SCAN AND ABOUT 16× UNDER A \
         QUADRATIC ONE.** The ceiling sits halfway between, leaving a factor of two of margin \
         in both directions. **Measured 15.7 OUT OF BAND and 13.19 UNDER FULL SUITE LOAD at \
         this file's base** — load moves the ratio DOWN, because it inflates the fixed \
         additive term in both measurements, and the number stays far above 8 either way. \
         The rebuilt `dd17bfb` control binary answers both sizes in 5 ms and 8 ms.\n\n\
         **THE MECHANISM.** `slash_anchored_candidates` (`policy.rs:5658-5662`) is applied PER \
         CANDIDATE inside `word_is_within` (`:5697`) and `word_is_exactly` (`:5760`), so \
         `lexical_absolute_components` runs ONCE PER `/` over a suffix whose length is itself \
         O(word). `MAX_GUARD_REQUEST_BYTES = 1 MiB` bounds the INPUT and bounds no work at \
         all — `T-19-120`'s own shape, in the round that fixed `T-19-120`.\n\n\
         **THE BOUND MUST BE ON WORK.** A byte floor at the shortest protected path removes \
         NINE candidates of EIGHT THOUSAND for the 16 KB word and does not move the number \
         above; a per-WORD cap is amplified by the word count and a per-SEGMENT cap by the \
         segment count. **`19-33` chooses the implementation; this row states the property \
         and measures it.**\n\n\
         **THIS IS THE SEVERANCE-RETIRABLE ROW.** If `19-33`'s work-bound task is SEVERED, \
         this is the ONE name that may stay red — it carries this assertion and nothing else, \
         and `19-33`'s two gates each branch on exactly this name and no other."
    );
}

#[test]
fn the_no_slash_control_attributes_the_cost_to_the_scan_and_not_to_the_length() {
    // **AUDIT 12's DECISIVE CONTROL, and without it the ratio row above could be
    // satisfied by a rule that simply REFUSED LONG WORDS.** A 100 000-character
    // word with NO slashes yields NO candidates at all, so the scan does no work
    // — and it answers in 29 ms at HEAD against 21 ms on the `dd17bfb` control
    // binary. **The cost is the SCAN, not the LENGTH.**
    //
    // **Asserted exit 0 and FAST, BEFORE AND AFTER.**
    let envelope = TempDir::new().unwrap();
    let word = format!("/{}", "a".repeat(99_999));
    assert_eq!(
        word.matches('/').count(),
        1,
        "the mechanism, pinned before the verdict: the control word must carry exactly ONE \
         slash, so it yields exactly one candidate and the scan cannot be quadratic in it"
    );
    let (code, millis) = ask_binary_under_timeout(envelope.path(), &format!("ls {word}"), 60);
    println!("  NO-SLASH CONTROL: {} chars, exit={code} after {millis} ms", word.len());
    assert_eq!(
        code, 0,
        "\n\n**A 100 000-CHARACTER WORD WITH NO SLASHES MUST BE PERMITTED.** It names no path \
         the guard protects, and a rule that refused it would be bounding LENGTH rather than \
         WORK. Measured exit 0 in 29 ms at this file's base."
    );
    assert!(
        millis < 5_000,
        "\n\n**AND IT MUST ANSWER WELL INSIDE `GUARD_TIMEOUT_SECS = 5`.** Got {millis} ms. \
         **This is what attributes the quadratic to the SCAN rather than to the LENGTH**: the \
         same number of bytes, no slashes, and the cost vanishes. A red here means the guard \
         became slow in the word's LENGTH, which is a different defect from the one this \
         section is about and is reported as a finding rather than absorbed."
    );
}

#[test]
fn the_scans_cost_curve_is_recorded_on_two_trees_and_the_extremes_are_not_asserted() {
    // **RECORDED, NOT ASSERTED — because a test that is RED for hours is not a
    // test.** The 50 000-slash row took 151 s at this file's base and the
    // 400 000-slash row did not answer in 300 s. Both are handed to `19-33` to
    // assert once they are fast.
    //
    // **The two constants this round's cost claim is measured against**, read
    // from the production source rather than retyped:
    assert!(
        HOOKS_SOURCE.contains("MAX_GUARD_REQUEST_BYTES"),
        "the `hooks.rs` include must reach the real file"
    );
    assert!(
        HOOKS_SOURCE.contains("GUARD_TIMEOUT_SECS"),
        "the `hooks.rs` include must reach the real file"
    );
    println!(
        "  `MAX_GUARD_REQUEST_BYTES` bounds the INPUT at 1 MiB and bounds NO WORK; \
         `GUARD_TIMEOUT_SECS` is 5. **The second is what the first fails to protect.**"
    );

    let envelope = TempDir::new().unwrap();
    for slashes in [1_000usize, 2_000, 5_000, 8_000] {
        let word = slash_dense_word(slashes);
        let (code, millis) =
            ask_binary_under_timeout(envelope.path(), &format!("ls {word}"), 120);
        println!(
            "  RECORDED (not asserted) [curve] {slashes:>6} slashes ({:>7} bytes): \
             exit={code} after {millis:>7} ms",
            word.len()
        );
    }
    println!(
        "  RECORDED (not asserted) [curve, MEASURED ONCE OUT OF BAND under a hard external \
         timeout] 20 000 slashes: 24 425 ms; 50 000 slashes: 151 249 ms; 400 000 slashes \
         (800 000 bytes): DID NOT ANSWER in 300 s (exit 124). A 1 MiB command is refused in \
         20 ms by `MAX_GUARD_REQUEST_BYTES`, so the WORST CASE is a request just under it. \
         **These are handed to `19-33` to assert once they are fast.**"
    );
    println!(
        "  RECORDED (not asserted) [the pre-round-12 control binary, rebuilt at `dd17bfb` \
         from a `git archive` extraction OUTSIDE the repository] the same inputs answered in \
         5, 5, 7, 8, 14 and 28 ms at 1 000 / 2 000 / 5 000 / 8 000 / 20 000 / 50 000 slashes. \
         **The quadratic is round 12's own, and it is PRE-EXISTING only in the sense that \
         round 12 introduced it — the `dd17bfb` tree does not have it.**"
    );
    println!(
        "  RECORDED (not asserted) [`T-19-124`'s BEHAVIOURAL half] what the agent CLI does \
         with a `PreToolUse` hook that OVERRUNS its registered timeout is a property of a \
         CLOSED-SOURCE BINARY. It is UNMEASURED and is claimed in NEITHER direction — exactly \
         as `C-08`'s and `T-19-120`'s behavioural halves are."
    );
}

// ---------------------------------------------------------------------------
// SECTION 6b — `19-33`'s OWN ROWS: the ceiling DRIVEN, and the two rows `19-32`
// could only RECORD, now asserted
//
// **A ceiling that cannot fire is a control that cannot fail** — the exact
// defect `T-19-126`(i) records one file over, and this round is not going to
// ship a second one. `CANDIDATE_SCAN_WORK_CEILING` is therefore driven END TO
// END through the BUILT BINARY, with a control beside it that makes the row a
// measurement of the SCAN's work rather than of the input's LENGTH.
// ---------------------------------------------------------------------------

#[test]
fn after_19_33_the_fail_closed_work_ceiling_can_fire_and_this_row_drives_it() {
    // **THE BACKSTOP, DRIVEN.** The BOUND is the linearity the row above pins;
    // this is the fail-closed ceiling beside it, which exists so that a later
    // edit reinstating a super-linear scan REFUSES rather than overrunning
    // `GUARD_TIMEOUT_SECS`.
    //
    // **THE DISCRIMINATOR, and it is what makes this a measurement rather than
    // a coincidence.** Three rows of the SAME ORDER OF MAGNITUDE of bytes:
    //
    //   500 KB, slash-DENSE  -> REFUSED at `envelope_assertion_failed`   <- the ceiling
    //   500 KB, NO slashes   -> PERMITTED                                <- same bytes, no work
    //   1.5 MB, one slash    -> refused by `MAX_GUARD_REQUEST_BYTES`, a
    //                           DIFFERENT message naming the byte bound
    //
    // Without the second row this test would pass against a rule that simply
    // refused long words — bounding LENGTH rather than WORK, which is the
    // defect `T-19-124` is about. Without the third it could not tell the
    // ceiling from the input bound.
    let envelope = TempDir::new().unwrap();

    let dense = slash_dense_word(250_000);
    assert_eq!(dense.len(), 500_000, "the mechanism, pinned before the verdict");
    let (code, millis) =
        ask_binary_under_timeout(envelope.path(), &format!("ls {dense}"), 60);
    println!("  CEILING DRIVEN: 250 000 slashes ({} bytes), exit={code} after {millis} ms", dense.len());
    assert_eq!(
        code, 2,
        "\n\n**THE FAIL-CLOSED WORK CEILING MUST FIRE.**\n\n\
         `CANDIDATE_SCAN_WORK_CEILING` bounds the work ONE `protected_carrier_named` \
         invocation may spend. When it is exhausted the predicate has NOT established that no \
         protected path was named, so the segment is REFUSED at \
         `ParkReason::EnvelopeAssertionFailed` — never permitted, never a new park reason. \
         **A ceiling no test drives is a control that cannot fail.** Got exit {code} after \
         {millis} ms."
    );
    assert!(
        millis < 5_000,
        "\n\n**AND IT MUST REFUSE INSIDE `GUARD_TIMEOUT_SECS = 5`.** Got {millis} ms. A \
         ceiling that fires only AFTER the deadline has already been overrun is not a \
         backstop."
    );

    // -- **THE CONTROL: the same order of bytes with NO slashes.** It yields no
    //    candidates, so the scan does no work, and it is PERMITTED.
    let flat = format!("/{}", "a".repeat(499_999));
    assert_eq!(
        flat.matches('/').count(),
        1,
        "the control word must carry exactly ONE slash, so the scan cannot be quadratic in it"
    );
    let (flat_code, flat_millis) =
        ask_binary_under_timeout(envelope.path(), &format!("ls {flat}"), 60);
    println!("  CEILING CONTROL: {} chars, ONE slash, exit={flat_code} after {flat_millis} ms", flat.len());
    assert_eq!(
        flat_code, 0,
        "\n\n**THE SAME NUMBER OF BYTES WITH NO SLASHES MUST STAY PERMITTED.** This is what \
         attributes the refusal above to the SCAN's WORK rather than to the input's LENGTH. A \
         red here means the ceiling became a cap on word length, which is a bound over the \
         wrong property — `T-19-120`'s own shape."
    );

    // -- **AND THE INPUT BOUND IS A DIFFERENT CONTROL WITH A DIFFERENT MESSAGE**,
    //    so the ceiling cannot be credited with a refusal `MAX_GUARD_REQUEST_BYTES`
    //    produced. `MAX_GUARD_REQUEST_BYTES` keeps its value: a bound over the
    //    wrong property is not corrected by moving a number.
    let over = format!("/{}", "a".repeat(1_500_000));
    let answer = ask(envelope.path(), &format!("ls {over}"));
    assert_eq!(answer.code, 2, "a request over 1 MiB is refused by the INPUT bound");
    assert!(
        answer.reason().contains("larger than"),
        "\n\n**THE INPUT BOUND MUST STILL BE DISTINGUISHABLE FROM THE WORK CEILING.** \
         `MAX_GUARD_REQUEST_BYTES` names the byte bound in its own message; the ceiling \
         refuses through the carrier predicate. Attributing one refusal to the other \
         mechanism is D-24. Got: {}",
        answer.reason()
    );
}

#[test]
fn after_19_33_the_fifty_thousand_slash_row_is_promoted_from_recorded_to_asserted() {
    // **`19-32` RECORDED this row at 151 249 ms and could not assert it — a test
    // that is RED for hours is not a test.** It is asserted here because the
    // scan is now linear.
    //
    // **ITS CONTROL is the 100 000-character no-slash word** in
    // `the_no_slash_control_attributes_the_cost_to_the_scan_and_not_to_the_length`:
    // the same order of bytes, no candidates, and it was ALREADY fast — so this
    // row measures the SCAN and not the machine.
    let envelope = TempDir::new().unwrap();
    let word = slash_dense_word(50_000);
    assert_eq!(word.len(), 100_000, "the mechanism, pinned before the verdict");
    let (code, millis) = ask_binary_under_timeout(envelope.path(), &format!("ls {word}"), 60);
    println!("  PROMOTED: 50 000 slashes (100 000 bytes), exit={code} after {millis} ms (19-32 RECORDED 151 249 ms)");
    assert_eq!(
        code, 0,
        "\n\n**A 50 000-SLASH WORD NAMING NO PROTECTED PATH MUST BE PERMITTED**, and it must \
         stay UNDER the work ceiling — the ceiling is a backstop against a super-linear scan, \
         not a limit ordinary input meets. Got exit {code}."
    );
    assert!(
        millis < 5_000,
        "\n\n**AND IT MUST ANSWER INSIDE `GUARD_TIMEOUT_SECS = 5`.** Got {millis} ms against \
         `19-32`'s recorded 151 249 ms on the quadratic scan. **A red here means the \
         reformulation regressed**, and it is reported as a finding rather than relaxed."
    );
}

#[test]
fn after_19_33_the_slash_dense_word_just_under_the_input_bound_answers_inside_the_deadline() {
    // **`19-32` RECORDED this row as DID NOT ANSWER IN 300 s (exit 124, a hard
    // external timeout).** It is asserted here on the property that matters —
    // that it ANSWERS, inside `GUARD_TIMEOUT_SECS` — with the VERDICT recorded
    // rather than asserted, because which of the two bounds produces it is a
    // fact about the ceiling's value and not about the reformulation.
    //
    // **This is the WORST CASE `19-32` derived**: `MAX_GUARD_REQUEST_BYTES`
    // refuses a 1 MiB request in 20 ms, so the most work a request can buy is
    // just UNDER that bound.
    let envelope = TempDir::new().unwrap();
    let word = slash_dense_word(400_000);
    assert_eq!(word.len(), 800_000, "the mechanism, pinned before the verdict");
    let (code, millis) = ask_binary_under_timeout(envelope.path(), &format!("ls {word}"), 60);
    println!("  PROMOTED: 400 000 slashes (800 000 bytes), exit={code} after {millis} ms (19-32 RECORDED: no answer in 300 s)");
    assert!(
        millis < 5_000,
        "\n\n**THE WORST CASE UNDER `MAX_GUARD_REQUEST_BYTES` MUST ANSWER INSIDE \
         `GUARD_TIMEOUT_SECS = 5`.** Got {millis} ms against `19-32`'s recorded NO ANSWER in \
         300 s. **That is the whole of `T-19-124`**: the input bound now bounds work, because \
         the work is linear in the input."
    );
    println!(
        "  RECORDED (not asserted) [which bound produced it] exit={code}. At \
         `CANDIDATE_SCAN_WORK_CEILING`'s current value this word exhausts the WORK ceiling \
         and is refused at `envelope_assertion_failed`; the disclosed over-refusal is stated \
         in `protected_carrier_named`'s own cost section. **The verdict is recorded rather \
         than asserted because it is a fact about the ceiling's VALUE, and the value is \
         chosen to be reachable rather than to make a row green.**"
    );
}

#[test]
fn after_19_33_the_post_fix_curve_is_recorded_on_three_trees() {
    // **RECORDED, not asserted** — the assertions are the ratio row and the two
    // promoted rows above. This prints the curve so the record carries the
    // measurement rather than a claim about it.
    let envelope = TempDir::new().unwrap();
    for slashes in [1_000usize, 2_000, 5_000, 8_000, 20_000, 50_000, 100_000] {
        let word = slash_dense_word(slashes);
        let (code, millis) = ask_binary_under_timeout(envelope.path(), &format!("ls {word}"), 60);
        println!(
            "  RECORDED (not asserted) [post-fix curve] {slashes:>7} slashes ({:>7} bytes): \
             exit={code} after {millis:>6} ms",
            word.len()
        );
    }
    println!(
        "  RECORDED (not asserted) [the three trees] `19-32` measured HEAD-before at 68 / 249 \
         / 1 524 / 3 904 / 24 425 / 151 249 ms and the rebuilt `dd17bfb` control binary at 5 \
         / 5 / 7 / 8 / 14 / 28 ms, at 1 000 / 2 000 / 5 000 / 8 000 / 20 000 / 50 000 \
         slashes. **The post-fix tree is on the CONTROL's curve rather than on round 12's.**"
    );
}

// ===========================================================================
// SECTION 7 — `T-19-125`: A DISCLOSED COST THAT IS ONE OPERATION SHORT
//
// `protected_carrier_named`'s disclosed cost (`policy.rs:6044-6048`) says
// *"any word whose text contains THIS RUN'S OWN envelope directory or THIS RUN'S
// OWN binary path **as a `/`-anchored substring**"*.
//
// **THE COMPARISON NORMALISES, so the real surface is WIDER than the sentence.**
// `dd of=<ENV>/./alpha/x` and `dd of=<ENV>/zzz/../alpha/x` are both exit 2 and
// **neither word contains `<ENV>/alpha` as a substring at all** — verified
// MECHANICALLY below rather than claimed. The true surface is *any word one of
// whose `/`-anchored substrings NORMALISES to a path under this run's directory
// or equal to its binary.*
//
// It fails CLOSED and the gap is small — **and this phase has already shipped a
// "FIVE forms" claim, a "four directions" claim and a "SEVEN spellings" claim
// that were each wrong when written, and the round that removed the count
// replaced it with a wording that is one operation short.**
//
// **RECORDED BESIDE IT:** `envelope_carrier_refusal`'s *"To proceed: name a path
// outside that directory"* (`policy.rs:6168`, `:6180`) is STALE for exactly this
// case, where the word merely CONTAINS the directory and the program would never
// have opened it. **Both repairs are `19-33`'s and neither is performed here.**
// ===========================================================================

#[test]
fn the_normalising_instances_are_refused_before_and_after_and_the_wording_is_recorded() {
    // **GREEN today and after. This is a WORDING finding, not this round's RED.**
    for (template, why) in [
        (
            "dd of={}/./alpha/x",
            "a `/.` segment the normaliser COLLAPSES",
        ),
        (
            "dd of={}/zzz/../alpha/x",
            "a `..` walk that lands back INSIDE the envelope directory",
        ),
    ] {
        // -- **THE MECHANICAL HALF, ASSERTED RATHER THAN CLAIMED.** The word must
        //    NOT contain the envelope directory as a substring at all, or the row
        //    says nothing about normalisation.
        let root = "/tmp/envroot";
        let word = template.replace("{}", root);
        let dir = format!("{root}/{ALIAS}");
        assert!(
            !word.contains(&dir),
            "\n\n**THE WHOLE POINT OF THIS ROW IS THAT `{word}` DOES NOT CONTAIN `{dir}` AS A \
             SUBSTRING.** If it does, the row measures the disclosed wording rather than the \
             gap in it."
        );
        assert!(
            protected_today(&word, Some(&dir), None),
            "and yet the simulation of TODAY's rule answers `true` for it, because the \
             comparison NORMALISES. **That pair — no substring, and a refusal — is the whole \
             of `T-19-125`.**"
        );

        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            &format!(
                "**`T-19-125` — the disclosed cost's wording is one operation short:** {why}.\n\n\
                 **THE CONTROL AND WHAT MAKES THE PAIR DISCRIMINATING:** its non-normalising \
                 twin `dd of=<ENV>/alpha/x` is refused too (below), and \
                 `dd of=/tmp/g/./x` — the SAME normalising shape one path over — is exit 0 \
                 before AND after. The three differ in exactly one thing each: whether the \
                 normalisation is needed, and whether the normalised path is the protected one.\n\n\
                 **The BEHAVIOUR is asserted here and the WORDING is RECORDED. `19-33` \
                 repairs the sentence whether or not any rule lands.**"
            ),
        );
    }

    // -- **THE NON-NORMALISING TWIN**, so the pair is discriminating.
    refuses_carrier(
        |root| format!("dd of={}/{ALIAS}/x", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "the NON-NORMALISING twin: the word DOES contain the envelope directory as a \
         `/`-anchored substring, so it is exactly what the disclosed wording describes.",
    );
    // -- **THE OUTSIDE-THE-PATH-SET CONTROL**, in the same normalising shape.
    permits(
        "dd of=/tmp/g/./x",
        "the same `/.`-collapsing shape over a path OUTSIDE the path set. **Without it, the \
         two rows above would pass on any rule that refused every word containing `/./`.**",
    );

    // -- **THE TWO WORDINGS, READ OUT OF THE PRODUCTION SOURCE RATHER THAN
    //    RETYPED.** A quotation a test retypes is a quotation that can drift from
    //    the file it claims to be quoting.
    println!(
        "RECORDED (not asserted) [`T-19-125`, the DISCLOSED COST's wording]\n  \
         `policy.rs:6044-6048`'s phrase `as a `/`-anchored substring` is {} in the production \
         source at this run. **Measured one operation short**: two words containing NO such \
         substring are refused, because the comparison NORMALISES. The true surface is *any \
         word one of whose `/`-anchored substrings NORMALISES to a path under this run's \
         directory or equal to its binary*. **`19-33`'s to repair.**",
        if POLICY_SOURCE.contains("as a `/`-anchored substring") {
            "PRESENT"
        } else {
            "ABSENT (repaired)"
        }
    );
    println!(
        "RECORDED (not asserted) [`T-19-125`, the REFUSAL's recovery line]\n  \
         `envelope_carrier_refusal`'s `To proceed: name a path outside that directory` \
         (`policy.rs:6168`, `:6180`) is {} in the production source. **It is STALE for \
         exactly this case**, where the word merely CONTAINS the directory and the program \
         would never have opened that substring as a path — so `name a path outside that \
         directory` is advice the user cannot act on (AR-19-11). **`19-33`'s to repair.**",
        if POLICY_SOURCE.contains("name \\\n             a path outside that directory")
            || POLICY_SOURCE.contains("a path outside that directory")
        {
            "PRESENT"
        } else {
            "ABSENT (repaired)"
        }
    );
}

// ===========================================================================
// SECTION 8 — `T-19-126`(i): TWO REAL STANDING PINS FOR A PROPERTY A ONE-SHOT
// GREP GATE STOOD IN FOR
//
// `ledger.rs:354-361` says the link-NON-following stat variant must not be
// substituted for `std::fs::metadata`, and adds that **the API name is
// deliberately not spelled out** because a verify step greps the file for it.
//
// **AUDIT 12 FOUND THE GATE IS A PLAN-TIME GREP THAT NOTHING RE-RUNS**, and that
// **no standing pin existed** for the property it stands in for — a symlinked-to-
// REGULAR ledger being PERMITTED and COUNTED, which is the control that keeps
// the KIND check a KIND bound rather than a LINK BAN. It had to measure it by
// hand.
//
// **IT ALSO FOUND THE STATED REASON INVERTED.** Writing the forbidden literal in
// `ledger.rs` to forbid it would turn a ZERO-OCCURRENCE gate **RED**, not make it
// *"pass vacuously"*. **A grep gate whose forbidden literal cannot be written is
// a one-shot, not a standing control.**
//
// **THE REPAIR IS STRUCTURAL AND IT DISSOLVES THE INVERSION.** The pattern
// already exists one file over:
// `the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so`
// asserts eight forbidden APIs are absent from a SLICE of `policy.rs`'s
// PRODUCTION half, and it CAN name all eight because the names live in the TEST
// module, BELOW the `#[cfg(test)]` sentinel and therefore OUTSIDE the slice.
//
// **BOTH PINS BELOW ARE GREEN TODAY AND GREEN AFTER — they are STANDING
// CONTROLS, not this round's RED**, and that is stated here so the SUMMARY does
// not count them as evidence of a fix. The inverted comment is `19-33`'s to
// correct.
// ===========================================================================

#[test]
fn a_symlinked_to_regular_ledger_is_permitted_and_counted_before_and_after() {
    // **THE BEHAVIOURAL STANDING PIN.** `std::fs::metadata` FOLLOWS symlinks,
    // which is what keeps a ledger that is a symlink to a regular file COUNTABLE.
    // The link-non-following variant would answer about the LINK — never a
    // regular file — and this check would stop counting it, silently turning the
    // KIND bound into a LINK BAN.
    let envelope = TempDir::new().unwrap();
    let dir = envelope.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    let real = envelope.path().join("real-ledger");
    std::fs::write(&real, "").unwrap();
    std::os::unix::fs::symlink(&real, dir.join("pr-ledger.ndjson")).unwrap();
    assert!(
        std::fs::symlink_metadata(dir.join("pr-ledger.ndjson"))
            .unwrap()
            .file_type()
            .is_symlink(),
        "**THE MECHANISM, PINNED BEFORE THE VERDICT.** The ledger path must really be a \
         SYMLINK, or this row measures an ordinary regular file and says nothing."
    );

    let answer = ask(envelope.path(), "gh pr create --title x");
    let counted = std::fs::read_to_string(&real).unwrap().lines().count();
    println!("  symlinked-to-regular ledger: exit={} counted={counted}", answer.code);
    assert_eq!(
        answer.code, 0,
        "\n\n**A LEDGER THAT IS A SYMLINK TO A REGULAR FILE MUST BE PERMITTED.**\n\n\
         `std::fs::metadata` FOLLOWS symlinks and `metadata.file_type().is_file()` is TRUE \
         through the link. **A red here means the link-NON-following variant was substituted \
         at `ledger.rs:372`** — which turns `T-19-120`'s KIND bound into a LINK BAN and stops \
         the cap counting a ledger it can perfectly well read. stdout: {}",
        answer.stdout
    );
    assert_eq!(
        counted, 1,
        "\n\n**AND THE ATTEMPT MUST HAVE BEEN COUNTED THROUGH THE LINK.** A permit that did \
         not append is a cap that stopped counting, which is `T-19-112`'s harm by a quieter \
         route than any this phase has registered. Walked listing:\n{}",
        listing(envelope.path())
    );
}

#[test]
fn the_ledger_kind_bound_is_non_vacuous_at_four_kinds_and_the_fresh_root_still_permits() {
    // **THE NON-VACUITY HALF.** Without these, the row above passes against a
    // check that was deleted entirely.
    //
    // **GREEN today and after** — this is round 12's own landed rule, re-asserted
    // here over the same public entry point rather than moved.
    let fifo = TempDir::new().unwrap();
    let dir = fifo.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(dir.join("pr-ledger.ndjson"))
        .status()
        .expect("mkfifo is on PATH");
    assert!(made.success(), "the fixture creates a FIFO at the ledger path");
    let (code, millis) = ask_binary_under_timeout(fifo.path(), "gh pr create --title x", 12);
    println!("  FIFO ledger: exit={code} after {millis} ms (hard timeout 12 s)");
    assert_eq!(
        code, 2,
        "**THE LEDGER BOUND MUST BOUND KIND AND NOT ONLY SIZE.** A FIFO stats at length 0 and \
         passes a bound written over SIZE; the read below it never returns. Observed exit \
         {code} after {millis} ms."
    );

    let directory = TempDir::new().unwrap();
    std::fs::create_dir_all(directory.path().join(ALIAS).join("pr-ledger.ndjson")).unwrap();
    let answer = ask(directory.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 2,
        "a DIRECTORY at the ledger path is not a regular file either"
    );
    assert!(
        answer.reason().contains(policy::REASON_ENVELOPE_ASSERTION_FAILED),
        "and it must be refused UNDER the GENERAL unresolvable identifier and never under \
         `pr_cap_exceeded`, which names a cap that FIRED (D-24). Got: {}",
        answer.reason()
    );

    let chardev = TempDir::new().unwrap();
    let dir = chardev.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    std::os::unix::fs::symlink("/dev/null", dir.join("pr-ledger.ndjson")).unwrap();
    let answer = ask(chardev.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 2,
        "**A SYMLINK TO A CHARACTER DEVICE IS REFUSED, AND THAT IS THE ROW THAT PROVES THE \
         CHECK IS A KIND BOUND RATHER THAN A LINK BAN FROM THE OTHER SIDE.** The link is \
         FOLLOWED — and what it points at is not a regular file."
    );

    // -- **THE CONTROL THE WHOLE SUITE DEPENDS ON.** A fresh envelope root has no
    //    ledger file at all, so `std::fs::metadata` returns `Err` and the check
    //    falls through. **A kind check placed OUTSIDE the `Ok` arm would refuse
    //    EVERY first forge call in EVERY fresh root.**
    let fresh = TempDir::new().unwrap();
    let answer = ask(fresh.path(), "gh pr create --title x");
    assert_eq!(
        answer.code, 0,
        "**A FRESH ROOT WITH NO LEDGER MUST PERMIT.** If this is red, the kind check left the \
         `Ok` arm of `std::fs::metadata` at `ledger.rs:372`. stdout: {}",
        answer.stdout
    );
    assert_eq!(
        ledger_lines_anywhere(fresh.path()).len(),
        1,
        "and must leave exactly one ledger line. Walked listing:\n{}",
        listing(fresh.path())
    );
}

#[test]
fn the_link_non_following_stat_variant_is_absent_from_ledger_rs_production_half() {
    // **THE SOURCE-SLICE STANDING PIN, AND WHAT DISSOLVES THE INVERSION
    // `T-19-126`(i) RECORDS.**
    //
    // A WHOLE-FILE grep cannot name what it forbids: writing the forbidden
    // literal into `ledger.rs` to document it would turn a zero-occurrence gate
    // RED. **A PRODUCTION-SLICE assertion can name it, because the name lives in
    // THIS file — the test half — and never in the slice.** That is the same
    // structure
    // `the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so`
    // already uses one file over, where eight forbidden APIs are named below a
    // `#[cfg(test)]` sentinel and asserted absent above it.
    //
    // **GREEN today and after.**
    let sentinel = LEDGER_SOURCE
        .find("#[cfg(test)]")
        .expect("`ledger.rs` must carry a `#[cfg(test)]` sentinel; without one there is no \
                 production half to slice");
    let production = &LEDGER_SOURCE[..sentinel];

    // -- **THE POSITIVE CONTROLS COME FIRST**, for the reason every absence
    //    assertion in this phase carries one: an absence assertion cannot tell
    //    "the string is not in this region" from "this is not the region I think
    //    it is".
    for anchor in [
        "fn record_and_check_in",
        "MAX_LEDGER_BYTES",
        "std::fs::metadata",
        "file_type().is_file()",
    ] {
        assert!(
            production.contains(anchor),
            "\n\n**THE PRODUCTION SLICE MUST CONTAIN `{anchor}`.** Without these positive \
             controls the absence below passes because the slice is empty or is the wrong \
             region."
        );
    }
    assert_eq!(
        LEDGER_SOURCE.matches("#[cfg(test)]").count(),
        1,
        "**EXACTLY ONE `#[cfg(test)]` SENTINEL**, so the slice boundary is unambiguous. A \
         second sentinel would make `find` pick the first and silently shrink the region."
    );

    // -- **THE FORBIDDEN API, NAMED IN THE ASSERTION.** This literal sits in the
    //    TEST file and therefore OUTSIDE the sliced region, which is exactly why
    //    it can be written at all.
    let forbidden = "symlink_metadata";
    assert!(
        !production.contains(forbidden),
        "\n\n**`{forbidden}` HAS APPEARED IN `ledger.rs`'s PRODUCTION HALF.**\n\n\
         `std::fs::metadata` FOLLOWS symlinks, and that is what keeps a ledger that is a \
         SYMLINK TO A REGULAR FILE countable — the behavioural pin above drives it. The \
         link-NON-following variant would answer about the LINK, never a regular file, and \
         `T-19-120`'s KIND bound would become a LINK BAN.\n\n\
         **THE CORRECT RESPONSE IS TO DELETE THE CALL FROM `src/`, NEVER TO DELETE THIS \
         ASSERTION.**"
    );
    println!(
        "  the `ledger.rs` production slice is {} bytes and contains `{forbidden}` zero times; \
         the literal is written HERE, below the sentinel and outside the slice. **That is what \
         a whole-file grep cannot do, and it is why `ledger.rs:354-361`'s stated reason is \
         INVERTED — `19-33`'s to correct.**",
        production.len()
    );
    println!(
        "RECORDED (not asserted) [`T-19-126`(i), the INVERTED reason]\n  \
         `ledger.rs:354-361` says the API name is left unspelled because *a verify step greps \
         this file for that literal and asserts it appears zero times, so writing it here to \
         forbid it would make the gate pass vacuously*. **Writing it would turn a \
         zero-occurrence gate RED, not make it pass.** The comment is {} in the production \
         source at this run. **`19-33`'s to correct.**",
        if production.contains("pass\n    // vacuously") || production.contains("vacuously") {
            "PRESENT"
        } else {
            "ABSENT (repaired)"
        }
    );
}

#[test]
fn the_control_carrier_interior_alphabets_seventh_entry_does_not_reach_and_that_is_recorded() {
    // **`T-19-126`(ii) — ONE ROW'S EVIDENTIARY VALUE, RE-MEASURED UNDER REAL GNU
    // TAR.** `CONTROL_CARRIER_INTERIOR_PATH` entry 7 was re-spelled to
    // `tar --create --file /tmp/t -C<ENV>/alpha`, which answers *"Cowardly
    // refusing to create an empty archive"* — the re-spelling moved `-C` last and
    // dropped the `.` member — while the alphabet's own doc says every entry was
    // drawn from `19-30`'s measured sweep.
    //
    // **THE CLASS IS NOT VACUOUS** (entries 8 and 9 reach, verified), so this is
    // one row's evidentiary value rather than a hole in the class.
    //
    // **THE REPAIR IS AN ADDED REACHING ENTRY BESIDE IT AND AN APPENDED WR-02
    // NOTE, NEVER A DELETION AND NEVER A RE-SPELLING** — a case that stops being
    // drawn is a case that stops being able to fail. The addition is made in
    // `tests/envelope_wrapper_class.rs` by this same plan.
    let fixture = TempDir::new().unwrap();
    let dir = fixture.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pr-ledger.ndjson"), "marker\n").unwrap();
    let archive = fixture.path().join("t");

    let as_respelled = bash_reaches(&format!(
        "tar --create --file {} -C{}",
        archive.display(),
        dir.display()
    ));
    println!("  entry 7 AS RE-SPELLED: tar exit={as_respelled}, archive exists={}", archive.exists());
    assert_ne!(
        as_respelled, 0,
        "**ENTRY 7 AS RE-SPELLED MUST NOT REACH THE FILE**, which is the finding. If it now \
         succeeds, GNU tar's behaviour changed and `T-19-126`(ii) must be RE-MEASURED rather \
         than carried forward at audit 12's number."
    );

    let with_member = bash_reaches(&format!(
        "tar --create --file {} -C{} .",
        archive.display(),
        dir.display()
    ));
    assert_eq!(
        with_member, 0,
        "**THE ADDED REACHING SPELLING** — the same attached short option WITH the `.` member \
         — must succeed. It is what `19-32` ADDS beside entry 7."
    );
    let listed = std::process::Command::new("tar")
        .args(["-tf", &archive.display().to_string()])
        .output()
        .expect("tar is on PATH");
    let listed = String::from_utf8_lossy(&listed.stdout).to_string();
    assert!(
        listed.contains("pr-ledger.ndjson"),
        "**AND THE ARCHIVE MUST REALLY HOLD THE CARRIER**, which is what makes it a READ of \
         the envelope directory rather than a shape. Got: {listed:?}"
    );
    println!(
        "  the ADDED entry `tar --create --file /tmp/t -C<ENV>/alpha .` REACHES: the archive \
         holds {listed:?}"
    );
    println!(
        "RECORDED (not asserted) [`T-19-126`(ii)] audit 12 also measured the re-spelling's \
         ORIGINAL MOTIVE MOOT: with a `c`-containing envelope root BOTH the re-spelled and \
         the original forms answer exit 2, because the carrier clause is raised at the top of \
         `classify_segments`' loop, ABOVE program resolution. **Entry 7 stays; nothing is \
         deleted or reworded.**"
    );
}

// ===========================================================================
// SECTION 9 — THE FENCED-FILE ENUMERATION, RE-DERIVED MECHANICALLY
//
// **The planning-time answer is not inherited. It was re-run and it holds:**
//
// ```text
// grep -rnE '<<<|<<-|<<EOF|>&[0-9]|<&[0-9]' tests/ src/     55 hits
//     every `<<<` / `<<` / `<<-` row carries a target (`x`, `EOF`, `here`, `2`,
//     `0`, `1`) that names NO protected path, so NONE of them moves.
// grep -rnE '<ENV>([ "]|$)' tests/                          1 hit
//     `tests/envelope_wrapper_class.rs:7834`, the PLACEHOLDER CONSTANT's own
//     definition. **NO corpus entry names the envelope root alone.**
// grep -rn 'PARENT of the envelope directory' tests/ src/   2 hits
//     `policy.rs:10024` and `tests/envelope_interior_path.rs:2341` — EXACTLY the
//     two ancestor pins, both `19-33`'s named cross-fence exception and neither
//     touched by this plan.
// grep -rn 'policy.rs:2297' tests/                          3 hits
//     `envelope_wrapper_class.rs:8357`, `envelope_carrier_reach.rs:2889`,
//     `envelope_interior_path.rs:2458` — the THREE stale citations, `19-33`'s to
//     correct. This file cites `policy.rs:2422`.
// ```
//
// The mechanical form of the same enumeration is below: every fenced unit pin
// and every fenced corpus row is checked ONE AT A TIME against the simulation of
// `19-33`'s widened PATH SET, and the two rows that DO move are named.
// ===========================================================================

#[test]
fn every_fenced_row_keeps_its_answer_under_the_widened_path_set_except_the_two_named_pins() {
    let root = "/tmp/envroot";
    let dir = "/tmp/envroot/alpha";
    let binary = "/tmp/gsd-binary-dir/gsd-meta-manager";

    // -- **THE TWO ROWS THAT DO MOVE, NAMED IN ADVANCE.** They are the SAME row
    //    written twice, and they are `19-33`'s NAMED, BOUNDED, CROSS-FENCE
    //    exception. **This plan performs neither move.**
    assert!(
        !protected_today("/tmp/envroot", Some(dir), Some(binary)),
        "**`policy.rs:10022-10026` and `tests/envelope_interior_path.rs:2341` PIN THIS ROW \
         PERMITTED BY NAME TODAY, and the code does exactly what the pins say.** If this \
         assertion fails, the ancestor permit has already been changed and `19-32`'s premise \
         is wrong."
    );
    assert!(
        protected_after_19_33("/tmp/envroot", Some(dir), Some(root), Some(binary)),
        "**AND `19-33`'s MANDATED DESIGN MOVES IT.** The envelope root is a PROPER ANCESTOR of \
         the envelope directory and is AT OR UNDER itself, so the new clause reaches it. \
         **Both pins move together, each with its reasoning REWRITTEN rather than deleted** — \
         the shape round 11 used for `direction_i_…_stays_permitted`, which needed a human \
         authorisation. **`19-32` names the exception and performs none of it.**"
    );

    // -- **EVERY OTHER FENCED UNIT PIN, ONE AT A TIME.** These are the rows
    //    `policy.rs:10001-10047` and `tests/envelope_interior_path.rs:2318-2424`
    //    assert `false` for, and the ancestor clause must move NONE of them.
    for (word, why) in [
        (
            "/tmp/envroot/alpha/../other/x",
            "`..` is collapsed PER CANDIDATE, so a walk OUT of the envelope directory stays \
             out — and it is NOT an ancestor of it either",
        ),
        (
            "./alpha/pr-ledger.ndjson",
            "a RELATIVE word whose longest candidate is `/alpha/pr-ledger.ndjson`, two \
             components against a three-component directory — and `[alpha]` is not a prefix \
             of `[tmp, envroot, alpha]`, so the ANCESTOR clause does not reach it either",
        ),
        ("/tmp/envroot/alphax", "a SIBLING whose name merely EXTENDS the alias"),
        (
            "/tmp/envroot/alpha2/x",
            "the raw-string-prefix trap, one component deeper",
        ),
        (
            "/tmp/envroot/unrelated-sibling",
            "**THE ANCESTOR-vs-PREFIX DISCRIMINATOR, at the predicate level.** It is UNDER the \
             root and is NOT an ancestor of the envelope directory, so the ancestor clause \
             leaves it alone — where a PREFIX widened to the root would refuse it",
        ),
        (
            "/tmp/envroot/beta/pr-ledger.ndjson",
            "a SECOND alias directory under the same root, for the same reason",
        ),
        ("/tmp", "**THE STOP.** The root's own PARENT is above `<root>` and no rule is written for it"),
        ("/", "the filesystem root, one further up. A boundary reaching here is an OUTAGE"),
        ("pr-ledger.ndjson", "a RELATIVE operand with no `/` at all"),
        ("/tmp/pr-ledger.ndjson", "the BASENAME near miss"),
        (
            "/tmp/gsd-binary-dir/some-other-file",
            "a SIBLING of the binary in the same shared directory — the EXACT-PATH control",
        ),
        (
            "/tmp/gsd-binary-dir",
            "**the binary's PARENT DIRECTORY. NO ANCESTOR CLAUSE IS WRITTEN FOR THE BINARY**, \
             and `word_is_exactly` stays an EQUALITY",
        ),
        ("/tmp/envrooz/alpha/pr-ledger.ndjson", "the one-character-changed root"),
    ] {
        assert!(
            !protected_today(word, Some(dir), Some(binary)),
            "\n\n**A FENCED PIN ALREADY ANSWERS `true` TODAY: `{word}`.**\n  {why}\n\n\
             `19-32`'s premise is that these are all `false` at its base."
        );
        assert!(
            !protected_after_19_33(word, Some(dir), Some(root), Some(binary)),
            "\n\n**A FENCED PIN MOVED UNDER `19-33`'s DESIGN: `{word}` must still answer \
             `false`.**\n  {why}\n\n\
             This is the enumeration re-derived mechanically rather than inherited. **A row \
             that moves here is a finding about the DESIGN, and the correct response is to \
             report it — never to edit the pin in a file `19-33` may not touch.**"
        );
    }

    // -- **THE POSITIVE CONTROLS**, so the absences above are not vacuous.
    for (word, why) in [
        ("/tmp/envroot/alpha", "the envelope directory itself"),
        ("/tmp/envroot/alpha/pr-ledger.ndjson", "a file under it"),
        ("/tmp/gsd-binary-dir/gsd-meta-manager", "the binary itself"),
        (
            "of=/tmp/envroot/alpha/pr-ledger.ndjson",
            "**ROUND 12's OWN CLASS**: an interior path behind an `=`",
        ),
        ("-C/tmp/envroot/alpha", "an interior path behind an attached short option"),
    ] {
        assert!(
            protected_today(word, Some(dir), Some(binary)),
            "the positive control `{word}` must answer `true` TODAY: {why}. Without these, \
             every absence above passes because the simulation answers `false` for everything."
        );
        assert!(
            protected_after_19_33(word, Some(dir), Some(root), Some(binary)),
            "**AND CONTAINMENT: `{word}` must still answer `true` after `19-33`.** The new \
             clause is written BESIDE the existing prefix, never instead of it, so no refusal \
             that exists today can be lost."
        );
    }
    // -- **THE ANCESTOR CLAUSE's OWN POSITIVE CONTROLS**, so it is not vacuous.
    for (word, why) in [
        ("/tmp/envroot", "the root, named exactly"),
        ("/tmp/envroot/", "the trailing-slash spelling, which normalises identically"),
        ("-C/tmp/envroot", "**the ancestor composed with the interior scan**"),
        ("/tmp/envroot/./", "a `/.` spelling the normaliser collapses to the root"),
    ] {
        assert!(
            protected_after_19_33(word, Some(dir), Some(root), Some(binary)),
            "the ANCESTOR clause's positive control `{word}` must answer `true` after \
             `19-33`: {why}. Without these the whole clause could be a no-op and every \
             absence above would pass."
        );
    }

    // -- **EVERY `=`-ATTACHED ABSOLUTE PATH THE WHOLE CORPUS PINS**, checked one
    //    at a time against BOTH the today and the after simulations.
    for pinned in [
        "-c include.path=/tmp/evil.cfg",
        "-c core.hooksPath=/dev/null",
        "-c aliasx.q=-c include.path=/tmp/evil.cfg status",
        "--git-dir=/tmp/g",
        "SSH_AUTH_SOCK=/tmp/evil",
        "GSD_MM_ENVELOPE_ROOT=/tmp/fresh",
        "core.hooksPath=/tmp/nohooks",
        "includeIf.gitdir:/tmp/some-repo/.path=/tmp/evil.cfg",
        "HEAD:refs/heads/gsd-auto/alpha/w",
        "https://github.com/o/r",
        "s/x/y/",
        "src/",
        "--format=%H",
        "<<<x",
        "<<-EOF",
        "<<EOF",
        ">&2",
        "<&0",
    ] {
        assert!(
            !protected_after_19_33(pinned, Some(dir), Some(root), Some(binary)),
            "\n\n**A FENCED CORPUS ROW WOULD MOVE: `{pinned}`.**\n\n\
             This is the mechanical form of the fenced-file enumeration. \
             `--git-dir=/tmp/g` alone is pinned PERMITTED in FOUR files and named in FIVE \
             `policy.rs` doc sites, so a rule that turned it red would turn five files red at \
             once — and the five non-pathname OPERATOR spellings under `tests/` all carry \
             targets naming no protected path, which is why none of them moves."
        );
    }
}

// ===========================================================================
// SECTION 10 — `SEPARATORS`, RE-DERIVED AT ITS CURRENT LINE
//
// **The constant is byte-identical and has MOVED**, from `policy.rs:2297` to
// **`policy.rs:2422`**, because round 12 added prose above it.
//
// ```text
// git log -L 2422,2422:src/envelope/policy.rs   ->  84a9b05  (plan 19-05), EXACTLY ONE
//                                                   commit for the whole phase
// git log -L 2297,2297:src/envelope/policy.rs   ->  af72137  (plan 19-15, "Rule B — a
//                                                   severed prefix is not a command
//                                                   position"), which says NOTHING about
//                                                   `SEPARATORS`
// ```
//
// **So the CITATION is stale rather than the MECHANISM moved.** This file cites
// `policy.rs:2422` everywhere; the three stale citations under `tests/` are named
// in section 9 and are `19-33`'s to correct.
// ===========================================================================

#[test]
fn separators_is_byte_identical_at_its_current_line_and_a_redirection_is_not_one() {
    // `SEPARATORS` (`policy.rs:2422`) has ONE commit in the whole phase
    // (`84a9b05`, plan 19-05) and `is_separator` is a bare `SEPARATORS.contains`,
    // so these are true by construction.
    assert!(
        POLICY_SOURCE
            .contains(r#"const SEPARATORS: &[&str] = &[";", "&&", "||", "|", "&", "\n", "(", ")", "{", "}"];"#),
        "\n\n**`SEPARATORS` IS NOT BYTE-IDENTICAL.**\n\n\
         It is cited at `policy.rs:2422` in this file — the line it MOVED to when round 12 \
         added prose above it — and `git log -L 2422,2422` returns exactly one commit for the \
         whole phase, `84a9b05`. **`19-33` adds no token to the stream at all**, so this \
         constant may not move by one character."
    );
    assert!(
        !policy::is_separator(">") && !policy::is_separator("<") && !policy::is_separator("="),
        "`>`, `<` and `=` must NOT be separators. **This round's boundary is a SECOND \
         `Segment` FIELD carrying a word the walk already computed — never a change to how \
         the line is split into words.**"
    );
    assert!(
        policy::is_separator("&&")
            && policy::is_separator(";")
            && policy::is_separator("|")
            && policy::is_separator("\n"),
        "the positive control: the real command separators must still BE separators, or the \
         assertions above pass because `is_separator` answers `false` for everything. \
         **`\\n` in particular is what makes the HEREDOC BODY form a segment of its own, \
         which is this round's correction to audit 12.**"
    );
}

// ===========================================================================
// SECTION 11 — THE `cred.rs` ALIAS-BODY ROUTE, MEASURED AND RECORDED AS
// `T-19-86`'s
//
// `cred.rs`'s WHAT IT DOES NOT COVER list has **TWO** items — an agent that
// unsets `GIT_CONFIG_COUNT`, and a later `-c credential.helper=<something>` on
// the same command line — and **omits the alias-body route**.
//
// `git -c alias.q='!git -c credential.helper=store credential fill' q` is
// **exit 0** at the guard and returns the ambient `~/.git-credentials` secret
// under the envelope's full posture. **THAT IS `T-19-86`'s ROUTE.** Audit 12
// measured it, recorded it under `T-19-121`, and deliberately did NOT fold it
// into `T-19-86` — *"widening it to carry a credential-reach harm would be the
// attribution move both `19-27` and audit 11 established must not be made."*
// **Neither does this plan.** It is MEASURED here, the missing list item is
// `19-33`'s to add, and the attribution stays where audit 12 put it.
//
// **The secret is recorded PRESENT/ABSENT and never transcribed** (SAFE-04's own
// reasoning applies to this corpus exactly as it applies to the ledger).
// ===========================================================================

/// Run `git` under the envelope's full posture, with `19-29`'s injected
/// empty-`credential.helper` pair present.
fn git_under_envelope_posture(
    home: &Path,
    gitconfig: &Path,
    args: &[&str],
    stdin: &str,
) -> (i32, String) {
    use std::io::Write as _;
    let mut command = std::process::Command::new("git");
    command
        .args(args)
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", gitconfig)
        .env("GIT_CONFIG_SYSTEM", gitconfig)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "credential.helper")
        .env("GIT_CONFIG_VALUE_0", "")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = command.spawn().expect("git is on PATH");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(stdin.as_bytes())
        .expect("the request is written");
    let out = child.wait_with_output().expect("git exits");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

/// Whether a `git credential fill` answer carried the ambient secret.
fn fill_returned_the_ambient_secret(text: &str, username: &str) -> bool {
    text.lines().any(|line| line == format!("username={username}"))
        && text.lines().any(|line| line.starts_with("password="))
}

#[test]
fn the_alias_body_credential_route_is_measured_and_recorded_as_t_19_86s_and_is_not_folded() {
    let fixture = TempDir::new().unwrap();
    let home = fixture.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(
        home.join(".git-credentials"),
        "https://probeuser:probepass@github.com\n",
    )
    .unwrap();
    let envelope = fixture.path().join("env");
    let gitconfig = cred::write_gitconfig_in(&envelope, ALIAS, "fixture", "f@example.invalid")
        .expect("the generated git config is written");
    // **THE ARGV IS PASSED WHOLE RATHER THAN HAVING `credential fill` APPENDED,
    // because the alias-body route's WHOLE POINT is that the outer command line
    // invokes the ALIAS — `git -c alias.q='!…' q` — and git re-parses the body.**
    // A helper that appended `credential fill` would drive
    // `git -c alias.q=… credential fill`, which never runs the alias at all and
    // measures the CONTROL twice. That is a mis-quoted argv silently changing the
    // class, which is exactly what this plan's own discipline says to verify by
    // eye.
    let fill = |args: &[&str]| -> (i32, bool) {
        let (code, text) = git_under_envelope_posture(
            &home,
            &gitconfig,
            args,
            "protocol=https\nhost=github.com\n\n",
        );
        (code, fill_returned_the_ambient_secret(&text, "probeuser"))
    };

    // -- **THE CONTROL, DRIVEN FIRST.** Without it, every PRESENT below could be
    //    explained by the posture never having failed closed at all.
    let (control_code, control_secret) = fill(&["credential", "fill"]);
    println!(
        "  CONTROL (no `-c`): exit {control_code}, ambient secret {}",
        if control_secret { "PRESENT" } else { "ABSENT" }
    );
    assert!(
        !control_secret,
        "\n\n**THE CONTROL POSTURE MUST FAIL CLOSED.** With `19-29`'s empty-`credential.helper` \
         pair injected and no `-c` on the line, `git credential fill` must NOT return the \
         ambient username and password. Got exit {control_code}. If this leg reaches the \
         secret, the control is not working and the row below measures nothing."
    );

    // -- **THE SECOND CONTROL — `T-19-121`'s own row**, so the posture is proved
    //    to be overridable at all before the alias body is blamed for it.
    let (direct_code, direct_secret) = fill(&[
        "-c",
        "credential.helper=store",
        "credential",
        "fill",
    ]);
    println!(
        "  `-c credential.helper=store` (T-19-121): exit {direct_code}, ambient secret {}",
        if direct_secret { "PRESENT" } else { "ABSENT" }
    );
    assert!(
        direct_secret,
        "**`T-19-121`'s OWN ROW MUST STILL REACH THE SECRET** at real git. It is the second \
         control: it separates *the alias body is re-parsed* from *the posture cannot be \
         overridden at all*. Got exit {direct_code}."
    );

    // -- **THE ROUTE, against REAL GIT. The alias is INVOKED.**
    let (alias_code, alias_secret) = fill(&[
        "-c",
        "alias.q=!git -c credential.helper=store credential fill",
        "q",
    ]);
    println!(
        "  `-c alias.q='!git -c credential.helper=store credential fill' q`: exit \
         {alias_code}, ambient secret {}",
        if alias_secret { "PRESENT" } else { "ABSENT" }
    );
    assert!(
        alias_secret,
        "\n\n**THE ALIAS-BODY ROUTE DID NOT REPRODUCE.** Audit 12 measured it returning the \
         ambient secret under the envelope's full posture; if it does not reproduce, report \
         the measured behaviour as a finding rather than asserting audit 12's number. Got \
         exit {alias_code}. Secret recorded PRESENT/ABSENT, never transcribed."
    );

    // -- **AT THE GUARD: RECORDED, NEVER ASSERTED.**
    record_only(
        "the `!`-bodied ALIAS credential route. **`T-19-86`'s ROUTE — RECORDED, NOT FOLDED.** \
         Audit 12 declined to fold it into `T-19-86`'s declared harm or to use it to re-rate \
         anything, because widening `T-19-86` to carry a credential-reach harm is the \
         attribution move both `19-27` and audit 11 established must not be made. **This plan \
         declines too, and `19-33` adds the missing `cred.rs` list item without moving the \
         attribution**",
        |_| "git -c alias.q='!git -c credential.helper=store credential fill' q".to_string(),
    );

    // -- **`cred.rs`'s TWO-ITEM LIST, READ OUT OF THE PRODUCTION SOURCE.**
    assert!(
        CRED_SOURCE.contains("**WHAT IT DOES NOT COVER, at the same weight:**"),
        "the `cred.rs` include must reach the real WHAT IT DOES NOT COVER list"
    );
    assert!(
        CRED_SOURCE.contains("an agent that unsets `GIT_CONFIG_COUNT`"),
        "list item ONE must still be there"
    );
    assert!(
        CRED_SOURCE.contains("a later `-c credential.helper=<something>` on the same command line"),
        "list item TWO must still be there"
    );
    println!(
        "RECORDED (not asserted) [`cred.rs`'s WHAT IT DOES NOT COVER list]\n  \
         TWO items, quoted from the source: (1) *an agent that unsets `GIT_CONFIG_COUNT`, \
         which is D-09's stated ceiling*; (2) *a later `-c credential.helper=<something>` on \
         the same command line, which OVERRIDES the reset and brings the secret back* \
         (`T-19-121`). **The ALIAS-BODY route is ABSENT from the list, and the list item is \
         `19-33`'s to add.** The attribution does not move: it is `T-19-86`'s route."
    );
}

// ===========================================================================
// SECTION 12 — THE TWO ORDERING PINS, AT DELIBERATELY DIFFERENT IDENTIFIERS
//
// **These are the mechanical proof `19-33`'s widened WORD SET and widened PATH
// SET are still raised in the ONE per-segment walk rather than in a second
// pass.** Round 3's principle is DISCHARGED rather than weakened: the third word
// class is a third `.chain()` on the SAME iterator, and the ancestor clause is a
// second comparison inside the SAME per-candidate walk.
//
// **THE THREE-WAY SHAPE IS WHAT MAKES EACH PIN DISCRIMINATING.** A two-way pin
// would pass if the classifier simply stopped firing.
//
// ```text
// carrier segment FIRST, force push SECOND   measured force_push_blocked  -> ASSERTED envelope_assertion_failed  (RED)
// force push FIRST, carrier segment SECOND   measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
// an OUTSIDE-the-root word, either order     measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
// ```
//
// **ALL ORDERINGS WERE MEASURED BEFORE ANY WAS WRITTEN**, and the two rows whose
// post-fix identifier cannot be derived are not written at all.
// ===========================================================================

#[test]
fn after_19_33_ordering_pin_a_the_here_string_carrier_wins_its_own_segment() {
    // **THE FIRST REFUSAL WINS, AND THAT IS A FACT ABOUT WHERE THE CLAUSE IS
    // RAISED.** `classify_segments` walks the segments in order and answers on
    // the first that refuses; `protected_carrier_named` is raised at the TOP of
    // that loop, ABOVE program resolution. So a here-string carrier in segment
    // ONE must answer `envelope_assertion_failed` and never `force_push_blocked`
    // — and if it answers `force_push_blocked` after `19-33`, the third word
    // class was chained in a SECOND PASS after the classifier rather than at the
    // one existing reading site. **That is a finding about the DESIGN, reported
    // rather than relaxed.**
    refuses_carrier(
        |root| {
            format!(
                "xargs rm -rf <<< {}/{ALIAS} && git push --force origin main",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**ORDERING PIN A — the carrier segment is FIRST.** Measured `force_push_blocked` \
             at this file's base, because nothing sees the here-string word at all.\n\n\
             **THE TWO CONTROLS, both asserted below and both GREEN before AND after:** the \
             same two segments in the OPPOSITE order stay `force_push_blocked`, and an \
             OUTSIDE-the-root here-string in EITHER order stays `force_push_blocked`. **The \
             three differ in exactly one thing each: which segment comes first, and whether \
             the here-string names a protected path.**\n\n{WORD_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| {
            format!(
                "git push --force origin main && xargs rm -rf <<< {}/{ALIAS}",
                root.display()
            )
        },
        "force_push_blocked",
        "**CONTROL 1 — the SAME two segments, opposite order.** The force push is segment ONE \
         and the FIRST refusal wins, so this stays `force_push_blocked` before AND after. **A \
         red here after `19-33` means the carrier clause was raised OUT of the per-segment \
         walk and now pre-empts an earlier segment's verdict**, which would move a reason \
         identifier five rounds have settled (D-24).",
    );
    refuses(
        "xargs rm -rf <<< /tmp/outside && git push --force origin main",
        "force_push_blocked",
        "**CONTROL 2 — an OUTSIDE-the-root here-string in the FIRST segment.** It stays \
         `force_push_blocked` before AND after, which is what proves pin A discriminates on \
         the PATH SET rather than on the presence of a here-string.",
    );
}

#[test]
fn after_19_33_ordering_pin_b_the_ancestor_wins_its_own_segment() {
    // **PIN B IS DELIBERATELY AT A DIFFERENT AXIS FROM PIN A**, so the two
    // together prove BOTH widenings are raised at the same one site.
    refuses_carrier(
        |root| format!("rm -rf {} && git push --force origin main", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**ORDERING PIN B — the ANCESTOR segment is FIRST.** Measured `force_push_blocked` \
             at this file's base, because the comparison is a PREFIX and every ancestor is \
             shorter.\n\n\
             **THE TWO CONTROLS, both asserted below and both GREEN before AND after.**\n\n\
             {PATH_SET_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("git push --force origin main && rm -rf {}", root.display()),
        "force_push_blocked",
        "**CONTROL 1 — the SAME two segments, opposite order.** The FIRST refusal wins.",
    );
    refuses(
        "rm -rf /tmp/outside && git push --force origin main",
        "force_push_blocked",
        "**CONTROL 2 — an OUTSIDE-the-root operand in the FIRST segment**, which is what \
         proves pin B discriminates on the PATH SET rather than on the shape of the composite.",
    );
}

// ===========================================================================
// SECTION 13 — THE CARRIED-FORWARD MECHANISM PINS, ROUNDS 4 THROUGH 12
//
// RE-ASSERTED here over the same public functions rather than moved or edited in
// place. **All green today and all green after `19-33`.** `Token.literal` is
// READ IN THREE PLACES and **this round adds no fourth reader of it** — it adds
// a THIRD WORD CLASS to the words the third reader already receives, and a
// SECOND COMPARISON to the paths it already checks them against.
// ===========================================================================

#[test]
fn round_5s_literalness_bit_is_non_vacuous_and_this_round_must_not_remove_it() {
    refuses(
        "git pus? --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 5: a decision word carrying a pathname-expansion metacharacter is NOT \
         `Token.literal`, so the guard cannot know which subcommand runs and fails CLOSED. \
         **The literal filter stays and is applied BEFORE any candidate is produced** — and \
         `skip_redirection_target` computes the SAME bit for a redirection target by the SAME \
         `REWRITING_CHARACTERS` classification, which is why the third word class inherits it \
         by construction rather than by a second rule.",
    );
    let segments = policy::split_segments_with_heads("git pus? --force origin main")
        .expect("the command splits into segments");
    assert!(
        segments[0].tokens.iter().any(|token| !token.literal),
        "**AND THE BIT ITSELF IS NON-VACUOUS**, read from the REAL tokenizer rather than from \
         a fixture. A `Token.literal` that were `true` for everything would make three rules \
         vacuous at once."
    );
}

#[test]
fn the_segment_count_pins_stay_green_in_both_the_split_and_the_displaced_variant() {
    for (command, expected) in [
        (
            "git >/dev/null push --force origin main",
            vec!["git", "push", "--force", "origin", "main"],
        ),
        (
            "git x2>/tmp/o push --force origin main",
            vec!["git", "x2", "push", "--force", "origin", "main"],
        ),
        (
            "git <<<x push --force origin main",
            vec!["git", "push", "--force", "origin", "main"],
        ),
    ] {
        let segments = policy::split_segments_with_heads(command)
            .unwrap_or_else(|| panic!("`{command}` splits into segments"));
        let observed: Vec<Vec<String>> = segments
            .iter()
            .map(|segment| {
                segment
                    .tokens
                    .iter()
                    .map(|token| token.text.clone())
                    .collect()
            })
            .collect();

        assert_eq!(
            segments.len(),
            1,
            "\n\n**`{command}` MUST BE EXACTLY ONE SEGMENT. OBSERVED: {observed:?}**\n\n\
             **(a) THE SPLIT VARIANT.** Something entered the token stream as an OPERATOR \
             token, so `split_segments_with_heads` flushed the segment at it. The count is now \
             TWO and the leading `git` resolves `Governed` with an EMPTY argv — which \
             `classify_git` answers `Allow` for. **Round 6's headline refusal has been \
             silently converted into a permit.**\n\n\
             **(b) THE DISPLACED VARIANT.** Something entered as an ORDINARY word. The count \
             is still ONE and the EXACT-TOKEN assertion below fires instead: the argv the \
             classifier reads is no longer the argv the program receives, and every decision \
             index is shifted by one. **That is `T-19-98`'s registered shape.**\n\n\
             **THIS ROUND MUST TOUCH NEITHER.** `19-33` carries a LITERAL NON-PATHNAME \
             redirection target on a SECOND `Segment` FIELD — **never as a `Token` in the \
             stream** — and changes nothing about `tokenize`, `Token`, `Segment::tokens`, \
             `SEPARATORS` or `is_separator`."
        );

        let tokens: Vec<&str> = segments[0]
            .tokens
            .iter()
            .map(|token| token.text.as_str())
            .collect();
        assert_eq!(
            tokens, expected,
            "\n\n**`{command}` MUST CARRY EXACTLY THE SURVIVING ARGV.**\n\
             \n  expected : {expected:?}\
             \n  observed : {tokens:?}\n\n\
             This is variant (b), THE DISPLACED VARIANT. `T-19-98` is the registered shape, \
             and the third row is THIS ROUND's own operator: a here-string's target must keep \
             producing NO token at all."
        );
    }
}

#[test]
fn round_6s_deletion_model_and_its_over_deletion_control_are_unchanged() {
    refuses(
        "git >/dev/null push --force origin main",
        "force_push_blocked",
        "round 6's headline: the redirection is DELETED and the surviving argv is still a \
         force push.",
    );
    permits(
        "git x2>/tmp/o push --force origin main",
        "round 6's OVER-DELETION control: `x2>` is NOT an IO_NUMBER redirection, so `x2` \
         SURVIVES as a word, displaces the subcommand and the command is no longer a \
         recognisable force push. **A rule that added `>` to `SEPARATORS` turns this red.**",
    );
    refuses(
        "git 2>/dev/null push --force origin main",
        "force_push_blocked",
        "the IO_NUMBER row: `2>` IS a redirection, so both it and its target are deleted.",
    );
    refuses(
        "git >",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "a redirection with NO target fails CLOSED rather than guessing. **That is axis 1's \
         own residue: a word the tokenizer produced for no class at all is a production that \
         did not COMPLETE, and it fails closed at `Token::redirection_unresolvable` rather \
         than being read.**",
    );
    refuses(
        "git <<EOF push --force origin main",
        "force_push_blocked",
        "the HEREDOC row: `<<` takes a DELIMITER rather than a pathname. **`19-33` records \
         the delimiter on a SECOND field and this row must not move by one character** — the \
         surviving argv is still a force push and the identifier is still the classifier's.",
    );
}

#[test]
fn round_7s_fail_closed_callee_grammar_is_non_dead_and_this_round_must_not_remove_it() {
    refuses(
        "git --attr-source HEAD push --force origin main",
        "force_push_blocked",
        "round 7: a KNOWN value-taking global option consumes its value, so the subcommand is \
         still found.",
    );
    refuses(
        "git --bogus-opt status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 7: an UNKNOWN global option fails CLOSED rather than being guessed at.",
    );
    permits(
        "git - push --force origin main",
        "round 7's over-refusal control: a bare `-` is not a git global option.",
    );
    refuses(
        "git -- push --force origin main",
        "force_push_blocked",
        "round 7: `--` ends option parsing and the subcommand is still found.",
    );
}

#[test]
fn round_8s_confinement_clause_is_non_dead_and_this_round_must_not_remove_it() {
    refuses(
        "git -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 8: an `include.path` in a `-c` value pulls in configuration the guard cannot \
         read, so it fails CLOSED.",
    );
    permits(
        "git -c includepath=/tmp/evil.cfg status",
        "round 8's `--signed no`: `includepath` is not `include.path`.",
    );
    permits(
        "git -c notinclude.path=/tmp/evil.cfg status",
        "round 8's `--signed no` from the other side: the clause is CONFINED to the key.",
    );
    permits("git -c a=b status", "round 8: an ordinary `-c` value is untouched.");
}

#[test]
fn round_9s_reparse_clause_is_non_dead_and_this_round_must_not_remove_it() {
    refuses(
        "git -c alias.q=\"-c include.path=/tmp/evil.cfg status\" q",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 9: git RE-PARSES an alias body as a command line, so a confined key inside one \
         is reached.",
    );
    permits(
        "git -c aliasx.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "round 9's `--signed no`: `aliasx.` is not `alias.`.",
    );
    permits(
        "git -c notalias.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "round 9's `--signed no` from the other side.",
    );
    permits(
        "git -c alias.p='!git push --force origin main' p",
        "**`T-19-86` — OPEN at `high` by explicit user scoping decision, and this row MAY NOT \
         MOVE.**",
    );
    permits(
        "git -c alias.q='!git -c credential.helper=store credential fill' q",
        "**THE SECOND `!`-BODIED ROW, also `T-19-86`'s and also at exit 0.** It is measured \
         against real git in section 11 and RECORDED there; it is pinned PERMITTED here \
         because `19-33` writes no rule for it and a rule that quietly reached it would be an \
         attribution move `19-27` and audit 11 both established must not be made.",
    );
}

#[test]
fn round_10s_carrier_clause_is_non_dead_and_this_round_must_not_narrow_it() {
    for template in [
        "rm -f {}/alpha/pr-ledger.ndjson",
        "truncate -s 0 {}/alpha/pr-ledger.ndjson",
        "cp /bin/true {}/alpha/hooks/pre-push",
        "cp /dev/null {}/alpha/askpass",
        "rm -rf {}/alpha",
        "shred -u {}/alpha/pr-ledger.ndjson",
        "cat {}/alpha/pr-ledger.ndjson",
        "ls {}/alpha/hooks",
        "cd {}/alpha",
    ] {
        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "round 10's clause over the envelope-root OPERAND rows. **`19-33` widens WHICH \
             WORDS reach the reader and WHICH PATHS the comparison protects; it must not \
             narrow what the clause already reaches** — and the ancestor clause is written \
             BESIDE the existing prefix, never instead of it, so nothing here can be lost.",
        );
    }
    refuses(
        "env -u GIT_CONFIG_COUNT git push --force origin main",
        "hook_bypass_blocked",
        "D-09's stated ceiling, REFUSED.",
    );
    permits(
        "env -u GSD_MM_ENVELOPE_ROOT ls",
        "`GSD_MM_ENVELOPE_ROOT` is measured INERT as a guard lever.",
    );
}

#[test]
fn round_11s_own_clauses_are_non_dead_and_both_near_misses_stay_permitted() {
    refuses_carrier(
        |root| format!(": > {}/{ALIAS}/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 11's REDIRECTION-TARGET clause: an absolute literal PATHNAME target under the \
         envelope root is refused. **This is the CONTROL for section 1's `>&` rows** — the \
         same truncation through the PATHNAME side of bash's twelve-operator split.",
    );
    refuses(
        &format!("cp /bin/true {}", this_binary().display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 11's EXACT-PATH binary clause. **This is the twin of section 1's binary row**, \
         and the two differ in exactly one thing: the word's position.",
    );
    permits(
        &format!(
            "cp /bin/true {}/some-other-file",
            this_binary_parent().display()
        ),
        "round 11's first near-miss control: a SIBLING of the binary.",
    );
    permits(
        &format!("ls {}", this_binary_parent().display()),
        "round 11's second near-miss control: the binary's PARENT directory.",
    );
}

#[test]
fn round_12s_interior_scan_and_credential_clause_are_non_dead_and_this_round_must_not_narrow_them() {
    // -- the INTERIOR SCAN at a representative spread of ATTACHMENT CHARACTERS.
    for template in [
        "dd if=/dev/null of={}/alpha/pr-ledger.ndjson",
        "tar --directory={}/alpha --create --file /tmp/t .",
        "cp --target-directory={}/alpha /bin/true",
        "chmod --reference={}/alpha/pr-ledger.ndjson /tmp/x",
        "tar --create --file /tmp/t -C{}/alpha .",
        "cp /bin/true -t{}/alpha",
        "PATH=/usr/bin:{}/alpha mytool",
        "R={}/alpha",
        "mytool --opt=a={}/alpha/pr-ledger.ndjson",
    ] {
        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "round 12's `/`-anchored candidate scan, across FOUR different attachments — an \
             `=`, an attached SHORT option with none, a `:` inside an assignment prefix and a \
             SECOND `=`. **`19-33` adds a word class and a comparison; it must not narrow the \
             READING.**",
        );
    }
    // -- and the interior scan over the BINARY, the other half of the path set.
    refuses(
        &format!("dd if=/bin/true of={}", this_binary().display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 12's scan over the EXACT-PATH half.",
    );
    // -- the `credential.helper` clause with its five NEAR-MISS controls.
    for command in [
        "git -c credential.helper=store status",
        "git -c CREDENTIAL.HELPER=store status",
        "git -c credential.https://github.com.helper=store status",
        "git --config-env=credential.helper=EVILVAR status",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "round 12's by-name `credential.helper` clause over SECTION and FINAL COMPONENT.",
        );
    }
    for command in [
        "git -c credentialx.helper=store status",
        "git -c notcredential.helper=store status",
        "git -c credential.helperx=store status",
        "git -c credential.username=x status",
        "git -c helper=store status",
    ] {
        permits(
            command,
            "one of round 12's FIVE near-miss controls, which is what makes the clause \
             BY-NAME rather than a substring test.",
        );
    }
}

// ===========================================================================
// SECTION 14 — THE ROWS THAT ARE RECORDED AND NEVER ASSERTED
//
// `19-33` writes **no rule** for `C-11` … `C-15`, for `C-08`'s behavioural half,
// for `T-19-124`'s behavioural half, for `T-19-116`'s residues, for the ancestor
// ABOVE the root, or for the spellings whose reach was not demonstrated. A row
// asserted REFUSED lands permanently red in a file `19-33` may only ADD to; a row
// asserted PERMITTED pins a live bypass as correct. **`19-22` asserted such a row
// against its own comment, its own SUMMARY and its own plan-check, and it halted
// `19-23` mid-plan.**
// ===========================================================================

#[test]
fn the_unruled_rows_are_recorded_and_never_asserted() {
    // -- **`T-19-116`'s FOUR ORIGINAL RESIDUES, plus the FIFTH audit 12 added.**
    //    **Closing `T-19-122` closes only ONE of the two routes this audit added**:
    //    the here-string spelling over the binary is closed by the third word
    //    class; `rm -rf <binary-parent>` is NOT, because no ancestor clause is
    //    written for the binary and `word_is_exactly` stays an EQUALITY.
    record_only(
        "T-19-116 residue 1 — EXPANSION-BORNE. `T-19-116` stays OPEN at `high`",
        |_| "cp /bin/true $(command -v gsd-meta-manager)".to_string(),
    );
    record_only(
        "T-19-116 residue 2 — the TILDE spelling. `T-19-116` stays OPEN at `high`",
        |_| "cp /bin/true ~/.cargo/bin/gsd-meta-manager".to_string(),
    );
    record_only(
        "T-19-116 residue 3 — the cd-RELATIVE spelling. `T-19-116` stays OPEN at `high`",
        |_| {
            format!(
                "cd {} && cp /bin/true gsd-meta-manager",
                this_binary_parent().display()
            )
        },
    );
    record_only(
        "T-19-116 residue 4 — the GLOB spelling. `T-19-116` stays OPEN at `high`",
        |_| format!("cp /bin/true {}/gsd-meta-manage?", this_binary_parent().display()),
    );
    record_only(
        "**T-19-116's FIFTH route, ADDED BY AUDIT 12: `rm -rf <binary-parent>`.** No ancestor \
         clause is written for the BINARY and `word_is_exactly` stays an EQUALITY, because \
         the binary's directory is shared with everything the user installed. \
         `word_is_exactly`'s permit IS explicitly reasoned — a prefix would refuse \
         `ls <parent>` and every `cargo install` — **but the reasoning argues only that a \
         PREFIX would be wrong and never that the parent's own DELETION removes the binary. \
         That is an HONESTY gap rather than a rule gap, and `19-33` states it in \
         `word_is_exactly`'s own doc.** ASSERTED IN NEITHER DIRECTION",
        |_| format!("rm -rf {}", this_binary_parent().display()),
    );

    // -- **`T-19-123`'s RESIDUE ABOVE THE ROOT.** An ancestor above `<root>`
    //    reaches the same nine carriers and NO RULE IS WRITTEN FOR IT.
    record_only(
        "**`T-19-123`'s RESIDUE: an ancestor ABOVE the root.** With a user-set \
         `GSD_MM_ENVELOPE_ROOT=/tmp/xyz`, `rm -rf /tmp` reaches the same nine carriers. **The \
         clause STOPS at `<root>` on ownership and AR-19-11 grounds** — `~/.local/share`, \
         `$HOME`, `/tmp` and `/` are shared with everything the user has, and a boundary \
         reaching them is an OUTAGE rather than a boundary. Registered, disclosed, \
         UNACCEPTED, and named on AXIS 3 of the restated condition. ASSERTED IN NEITHER \
         DIRECTION",
        |root| {
            format!(
                "rm -rf {}",
                root.parent().expect("a temporary root has a parent").display()
            )
        },
    );

    // -- **THE SPELLINGS THE GUARD PERMITS BUT REAL `bash` DOES NOT REACH.** A
    //    spelling the shell does not resolve to the file is not a bypass, and
    //    this is the leg that tells them apart.
    record_only(
        "**`<<` WITH THE PATH AS THE DELIMITER** — `xargs rm -rf <<<ENV>/alpha`. The guard \
         permits it, and real `bash` warns *here-document at line 1 delimited by \
         end-of-file* and hands `xargs` an EMPTY body, so `rm -rf` runs with no operands and \
         **REACHES NOTHING**. RECORDED rather than asserted, exactly as `19-30` recorded \
         `python3 --out=…`",
        |root| format!("xargs rm -rf <<{}/{ALIAS}", root.display()),
    );
    record_only(
        "**`<<-` WITH THE PATH AS THE DELIMITER** — the same, tabs stripped. Also REACHES \
         NOTHING",
        |root| format!("xargs rm -rf <<-{}/{ALIAS}", root.display()),
    );
    record_only(
        "**`<&` WITH A PATH** — `cat <&<ledger>`. The guard permits it and real `bash` answers \
         `ambiguous redirect` at exit 1, so it REACHES NOTHING. **`<&` is the one of the five \
         non-pathname operators for which no reaching form was found**, and that is RECORDED \
         as a deliberate absence with its reason rather than asserted",
        |root| format!("cat <&{}/{ALIAS}/pr-ledger.ndjson", root.display()),
    );

    // -- **`C-11` … `C-15`, all at control (e), no rule, no `pr_cap_*` clamp.**
    record_only(
        "C-11 `.git/config` — T-19-111, no rule, attribution stays OUT of T-19-86",
        |_| {
            "printf '[alias]\\n\\tfp = push --force origin HEAD:refs/heads/main\\n' >> .git/config"
                .to_string()
        },
    );
    record_only("C-12 `.claude/settings.json` — layer 1's own registration", |_| {
        "printf x >> .claude/settings.json".to_string()
    });
    record_only("C-13 `.git/info/exclude` — D-23, the sweep's ignore block", |_| {
        "rm -f /tmp/proj/.git/info/exclude".to_string()
    });
    record_only("C-14 the run journal — D-24/D-25 park EVIDENCE", |_| {
        "rm -rf /tmp/proj/.planning/meta-manager/runs/run-1".to_string()
    });
    record_only(
        "C-15 `~/.config/gsd-meta-manager/config.json` — **NO `pr_cap_*` CLAMP is proposed or \
         written; clamping a configured cap is a PRODUCT decision about what a user may \
         configure, not a guard rule**",
        |_| "printf '{\"pr_cap_per_24h\":9999}' > ~/.config/gsd-meta-manager/config.json".to_string(),
    );
    record_only(
        "C-08 `settings.json` — the BEHAVIOURAL half. Whether the agent CLI re-reads its \
         settings mid-run is UNMEASURED and CLAIMED IN NEITHER DIRECTION",
        |root| format!("printf '{{}}' > {}/{ALIAS}/settings.json", root.display()),
    );
    record_only(
        "T-19-104's carrier `GIT_CONFIG_PARAMETERS`, a DIFFERENT mechanism — an environment \
         variable rather than argv, already refused by the envelope's own env-key deny at \
         `hook_bypass_blocked`. **RECORDED with its overlap stated: its control fails the \
         same way at the same identifier, so it discriminates nothing on THIS axis**",
        |_| "GIT_CONFIG_PARAMETERS=\"'credential.helper=store'\" git fetch origin".to_string(),
    );
    record_only(
        "`glab` is CONFIRMED NOT INSTALLED on this machine, so any `glab` row is UNMEASURABLE \
         AGAINST ITS CALLEE. **`--hostname` stays in `FORGE_VALUE_OPTS`** — audit 9 overturned \
         audit 8's removal suggestion because removal moves a counted creation form to \
         UNCOUNTED (`T-19-35`). A pin that would SKIP is a fail-open pin and is not written",
        |_| "glab mr create --title x".to_string(),
    );

    println!(
        "\nRECORDED (not asserted) [`T-19-124`'s BEHAVIOURAL half]\n  \
         What the agent CLI does with a `PreToolUse` hook that OVERRUNS its registered \
         timeout is a property of a CLOSED-SOURCE BINARY. It is UNMEASURED and is claimed in \
         NEITHER direction — exactly as `C-08`'s and `T-19-120`'s behavioural halves are.\n"
    );
    println!(
        "RECORDED (not asserted) [the 1 MiB slash-dense row]\n  \
         A command of 1 MiB of `/a` is refused in 20 ms by `MAX_GUARD_REQUEST_BYTES`, so the \
         WORST CASE is a request JUST UNDER it: 800 000 bytes DID NOT ANSWER in 300 s under a \
         hard external timeout (exit 124). **A test that is RED for hours is not a test**, so \
         it is measured once, recorded with its wall time, and handed to `19-33` to assert \
         once it is fast.\n"
    );
}

#[test]
fn no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically() {
    // **THE MECHANICAL SELF-ASSERTION.** A comment saying "these are recorded" is
    // a comment; this reads this file's own text and proves it.
    //
    // The positive control comes first, for the reason every absence assertion in
    // this phase carries one: an absence assertion cannot tell "the string is not
    // in this file" from "this is not the file I think it is".
    assert!(
        THIS_FILE.contains(
            "fn no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically"
        ),
        "the self-read must reach THIS file; if it does not, every absence below is vacuous"
    );
    assert!(
        THIS_FILE.contains("fn record_only("),
        "the self-read must see `record_only`'s definition"
    );

    for fragment in [
        ".git/config",
        ".claude/settings.json",
        ".git/info/exclude",
        ".planning/meta-manager/runs",
        "config.json",
        "settings.json",
        "command -v gsd-meta-manager",
        "~/.cargo/bin/gsd-meta-manager",
        "GIT_CONFIG_PARAMETERS",
        "glab ",
        "gsd-meta-manage?",
        "<<-{",
        "cat <&",
    ] {
        for (index, line) in THIS_FILE.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            if !line.contains(fragment) {
                continue;
            }
            assert!(
                !line.contains("refuses(")
                    && !line.contains("refuses_carrier(")
                    && !line.contains("permits(")
                    && !line.contains("permits_carrier("),
                "\n\n**AN UNRULED ROW HAS BEEN WRITTEN AS AN ASSERTION.**\n\
                 \n  line {}: {line}\n  fragment: {fragment}\n\n\
                 `19-33` writes NO rule for `C-11` … `C-15`, for `C-08`'s behavioural half, \
                 for `T-19-116`'s residues, for the ancestor ABOVE the root, or for the \
                 spellings whose reach was not demonstrated. A row asserted REFUSED lands \
                 permanently red in a file `19-33` may only ADD to; a row asserted PERMITTED \
                 pins a live bypass as correct. **Use `record_only`.**",
                index + 1
            );
        }
    }

    // -- **AND THE BINARY'S PARENT MUST APPEAR ONLY UNDER `record_only` WHEN IT
    //    IS THE OPERAND OF A DELETION.** `ls <BINPAR>` and
    //    `cp /bin/true <BINPAR>/some-other-file` ARE asserted — they are round
    //    11's own near-miss controls — but `rm -rf <BINPAR>` may not be, in
    //    either direction.
    for (index, line) in THIS_FILE.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("///") {
            continue;
        }
        if !line.contains("rm -rf {}") || !line.contains("this_binary_parent") {
            continue;
        }
        assert!(
            !line.contains("refuses(") && !line.contains("permits("),
            "\n\n**`rm -rf <binary-parent>` HAS BEEN ASSERTED.**\n  line {}: {line}\n\n\
             Asserting it REFUSED lands permanently red in a file `19-33` may not satisfy — \
             no ancestor clause is written for the binary — and asserting it PERMITTED pins a \
             live `T-19-116` bypass as correct. **Use `record_only`.**",
            index + 1
        );
    }
}

// ===========================================================================
// SECTION 15 — `SECTION_ENVELOPE` RE-MEASURED AND NOT EDITED
//
// **Headroom that is spent cannot be got back**, so this round measures and
// changes nothing. `19-33` is prohibited from opening `advisory.rs` too.
// ===========================================================================

#[test]
fn section_envelope_is_re_measured_and_this_round_spends_none_of_its_headroom() {
    let tokens = advisory::SECTION_ENVELOPE.split_whitespace().count();
    let widest = advisory::SECTION_ENVELOPE
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    println!(
        "  SECTION_ENVELOPE re-measured: {tokens} whitespace tokens (cap 215), widest line \
         {widest} chars (cap 80)"
    );
    assert_eq!(
        tokens, 211,
        "**`SECTION_ENVELOPE` MOVED.** `19-30` and `19-31` re-measured it at 211 tokens of an \
         UNRAISED 215 cap, exactly as audit 11 did. This round edits nothing in \
         `advisory.rs`, and neither does `19-33`."
    );
    assert_eq!(
        widest, 74,
        "**`SECTION_ENVELOPE`'s WIDEST LINE MOVED.** Measured 74 of an 80 cap."
    );

    // -- **THE FIRST `Guaranteed` CLAUSE IS STILL TRUE, and NEITHER `T-19-122`
    //    NOR `T-19-123` FALSIFIES IT.** *"As started"* states what the envelope
    //    ESTABLISHES and stops; the clause makes no claim about what a later
    //    command line can do.
    assert!(
        advisory::SECTION_ENVELOPE
            .contains("As started, this run cannot reach your ambient git credentials"),
        "the first `Guaranteed` clause must still open with `As started`, which is the whole \
         of `19-29`'s repair and the reason neither of this round's classes falsifies it"
    );
    assert!(
        advisory::SECTION_ENVELOPE.contains("runs no credential helper"),
        "`runs no credential helper` is a MEASURED phrase rather than a stylistic one."
    );
    assert!(
        advisory::SECTION_ENVELOPE.contains("the files and the binary this envelope runs on"),
        "**the `Not guaranteed` half's generalisation ALREADY COVERS BOTH of this round's \
         routes** — a command that rewrites the files or the binary this envelope runs on is \
         past the last layer, whether it names them as an operand, after a non-pathname \
         redirection operator, or one component up. **No new disclosure is owed here and none \
         is written.**"
    );
}
