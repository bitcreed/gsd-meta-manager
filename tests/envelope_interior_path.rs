// ============================================================================
// Round 12 — the path that is not the WORD but is INSIDE it, and the two false
// claims the code makes about it.
//
// **What this file is.** The ELEVENTH evidence file. Audit 11 said the plane is
// finished — it swept the cells adjacent to all six named axes and found no
// twelfth plane, and it verified that round 11's three new mechanisms create no
// new reachable state. What it also found is that **every path rule this phase
// has written asks whether a WORD is a path, and a word can carry a path
// without being one.** `lexical_absolute_components` (`policy.rs:5457-5476`)
// opens with `if !word.starts_with('/') { return None; }`, and BOTH halves of
// `protected_carrier_named`'s path set are built on it — `word_is_within`
// (`:5491`) and `word_is_exactly` (`:5533`) each call it first and answer
// `false` on `None`. So `dd if=/bin/true of=<BINARY>` carries an absolute path
// inside a token that does not START with `/` and is invisible to the whole
// rule, while `Token.literal` is TRUE for it.
//
// **This file is an ELEVENTH evidence file rather than an appendix to
// `tests/envelope_carrier_reach.rs`**, for the reason `19-14` created a third,
// `19-16` a fourth, `19-18` a fifth, `19-20` a sixth, `19-22` a seventh,
// `19-24` an eighth, `19-26` a ninth and `19-28` a tenth: round 11's evidence
// and round 12's evidence stay attributable to the round that produced them.
// The mechanism pins in section 11 are RE-ASSERTED here over the same public
// functions, not moved and not edited in place.
//
// **This file is RED at this plan's end BY DESIGN.** `19-30` writes the corpus
// and the reproducers and STOPS; `19-31` writes the rules and the honesty
// repairs. If any derived-post-fix row here had been GREEN against the pre-fix
// tree it would have been a FINDING — a property green before the fix is a
// property that could not have failed on it — and it would have been reported
// rather than asserted. Every commit producing this file shows ZERO `src/`
// hunks.
//
// ============================================================================
// THE PROCESS FINDING, WHICH IS WORTH MORE THAN THE MECHANISM
// ============================================================================
//
// **`19-28` SAW THIS AND CLASSIFIED IT WRONG.** `19-28-SUMMARY.md:381-383`
// records that `git --git-dir=<ENV>/alpha push --force origin main` answers
// `force_push_blocked` rather than `envelope_assertion_failed` *"because the
// token does not begin with `/` and `lexical_absolute_components` rejects
// it"* — the mechanism, named exactly right — and `19-29-SUMMARY.md:641-644`
// carried it forward as *"Not a defect; recorded because its space-separated
// twin answers differently."*
//
// **The OBSERVATION was correct and the CLASSIFICATION was wrong.** An operand
// whose verdict changes when you delete one space is not a curiosity about
// identifiers; it is a word class the rule cannot see. **A
// recorded-and-misclassified observation is how this one survived a round that
// had already found it.** Neither `19-28`'s nor `19-29`'s record is edited to
// say so — this is written BESIDE, which is the discipline audit 11 used for
// its own two corrections to those same subsections.
//
// ============================================================================
// THE THREE CELLS AUDIT 11 LEFT ON THE ELEVENTH PLANE
// ============================================================================
//
// ```text
// T-19-119  high    `lexical_absolute_components`, the normaliser BOTH halves
//                   of the path set are built on. It requires the WORD to start
//                   with `/`, so a path after an `=` — or after any other
//                   character, or after none — is invisible while
//                   `Token.literal` is TRUE. NONE of the seven declared
//                   directions.
// T-19-121  high    `git -c credential.helper=<value>` on ONE permitted line.
//                   `cred.rs:420-425` bounds the residue on the claim that the
//                   spelling is "ARGV-VISIBLE and is already GOVERNED by
//                   `scan_leading`'s leading-option region". ARGV-VISIBLE it
//                   is; GOVERNED it is not.
// T-19-120  medium  `MAX_LEDGER_BYTES` (`ledger.rs:114`, checked `:291-311`)
//                   bounds the ledger's SIZE where the failure is its KIND. A
//                   FIFO stats at 0, passes the bound, and the read below it
//                   never returns.
// ```
//
// ============================================================================
// THE EXIT CODES MEASURED AT THIS FILE'S BASE COMMIT (`77b3b90`), AGAINST THE
// BUILT BINARY, ONE FRESH `GSD_MM_ENVELOPE_ROOT` PER ROW, EVERY WALK EMPTY
// ============================================================================
//
// `git diff --numstat 1e56389..b6eaafa -- src/ tests/` was confirmed EMPTY
// before the first measurement, and `git diff b6eaafa..77b3b90` touches only
// `.planning/`, so this is the tree audit 11 measured against.
//
// ```text
// exit=0  dd if=/bin/true of=<BINARY>                            <- T-19-119, layers 2 AND 3
// exit=2  cp /bin/true <BINARY>                                  <- the TWIN, one character over
// exit=0  dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson        <- T-19-119, resets a FIRED cap
// exit=2  cp /dev/null <ENV>/alpha/pr-ledger.ndjson              <- the TWIN
// exit=0  tar --directory=<ENV>/alpha -cf /tmp/t .               <- T-19-119, C-01…C-09
// exit=2  tar --directory <ENV>/alpha -cf /tmp/t .               <- the TWIN
// exit=0  cp --target-directory=<ENV>/alpha /bin/true            <- T-19-119
// exit=0  chmod --reference=<ENV>/alpha/pr-ledger.ndjson /tmp/x  <- T-19-119, a READ
// exit=0  rsync --temp-dir=<ENV>/alpha /bin/true /tmp/x          <- T-19-119
// exit=0  tar -C<ENV>/alpha -cf /tmp/t .                         <- ATTACHED SHORT OPTION, NO `=`
// exit=0  cp -t<ENV>/alpha /bin/true                             <- the same, second program
// exit=0  PATH=/usr/bin:<ENV>/alpha mytool                       <- a `:` attachment, and bash EXECS from it
// exit=0  GSD_MM_ENVELOPE_ROOT=<ENV>/alpha gh pr create --title x <- an ASSIGNMENT PREFIX
// exit=0  tar --directory=<BINARY> -cf /tmp/t .                  <- the BINARY, `=`-attached
// exit=2  git --git-dir=<ENV>/alpha push --force origin main     <- `force_push_blocked`, 19-28's own observation
// exit=0  dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen      <- `policy.rs:5743`'s own example, PERMITTED
// exit=124 gh pr create --title x   (ledger is a FIFO, 20.02 s)  <- T-19-120
// ```
//
// **Every asserted row names its CONTROL and states what makes the pair
// discriminating** — on this axis that is exactly one character: whether the
// carrier path is JOINED to the preceding text or SEPARATED from it, or whether
// the interior path is the protected one.
//
// ## The rows that must NOT move (exit 0 before AND after)
//
// ```text
// exit=0  git --git-dir=/tmp/g status              <- `=`-attached, OUTSIDE the path set
// exit=0  git -c aliasx.q="-c include.path=/tmp/evil.cfg status" q  <- TWO `=` in one quoted word
// exit=0  GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x
// exit=0  git push origin HEAD:refs/heads/gsd-auto/alpha/w  <- a `:`-attached ref carrying `alpha`
// exit=0  git log --format=%H / git commit --author='A <a@b.c>' / sed s/x/y/ /tmp/f
// exit=0  curl https://github.com/o/r              <- a URL operand
// exit=0  rm -f /tmp/pr-ledger.ndjson              <- the BASENAME near miss
// exit=0  cat <ENVX>/alpha/pr-ledger.ndjson        <- the one-character-changed root
// exit=0  cp /bin/true <BINPAR>/some-other-file / ls <BINPAR>  <- EXACT-PATH-not-PREFIX
// exit=0  rg pr-ledger.ndjson src/                 <- a word whose only `/` is TRAILING
// exit=0  rm -rf ~/<ENV-REL>/alpha / <ENV>/alph? / {…,x}  <- (v),(vi),(vii)
// exit=0  D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson  <- (ii)
// exit=0  echo x > /tmp/outside && ls              <- round 11's outside-the-root control
// exit=0  git x2>/tmp/o push --force origin main   <- round 6's over-deletion control
// exit=2  git >/dev/null push --force origin main  <- round 6's headline
// exit=2  : > <ENV>/alpha/pr-ledger.ndjson         <- round 11's redirection rule
// exit=2  cp /bin/true <BINARY>                    <- round 11's exact-path clause
// exit=0  git -c alias.p='!git push --force origin main' p  <- T-19-86, may not move
// ```
//
// ============================================================================
// THE ROWS THAT ARE RECORDED AND NEVER ASSERTED
// ============================================================================
//
// Because `19-31` writes **no rule** for them, a row asserted REFUSED lands
// permanently red in a file `19-31` may not edit, and a row asserted PERMITTED
// pins a live bypass as correct. **`19-22` asserted such a row against its own
// comment, its own SUMMARY and its own plan-check, and it halted `19-23`
// mid-plan.** Section 12 uses `record_only` and ONLY `record_only`, and
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

/// This file's own text, for the mechanical self-assertion in section 12.
const THIS_FILE: &str = include_str!("envelope_interior_path.rs");

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
// The harness — carried forward UNCHANGED IN SHAPE from
// `tests/envelope_carrier_reach.rs:145-450`
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

/// The derivation every INTERIOR-PATH row shares, written once and quoted into
/// each row's own failure message.
///
/// **CONTAINMENT IS STATED PRECISELY RATHER THAN APPROXIMATELY, because this
/// phase has been bitten by a claim that was true in spirit.**
const INTERIOR_PATH_DERIVATION: &str = "\
**THE DERIVATION.** `19-31`'s mandated clause widens what rule (a) READS from \
THE WORD to EVERY `/`-ANCHORED SUBSTRING of a LITERAL word: for a word `w` that \
passes the EXISTING `!token.operator && token.literal` filter, every byte index \
`i` with `w.as_bytes()[i] == b'/'` yields a candidate `&w[i..]`, and each \
candidate goes through the EXISTING `lexical_absolute_components` normaliser \
and the EXISTING two comparisons — `word_is_within` (a component-wise PREFIX \
over the envelope directory) and `word_is_exactly` (an EQUALITY against the \
guard's own binary). **The rule never asks what precedes the `/`**, so it is \
not keyed to `=`, to `:`, to `,`, to an attached short option or to any other \
character, and a character list would be a program-grammar enumeration — D-08's \
defect one level over. It is raised at the SAME ONE SITE (`hooks.rs:1090-1094`), \
on the segment the walk already holds, BEFORE the resolution match, so the \
reason identifier is the GENERAL unresolvable one, `envelope_assertion_failed`, \
and NOT one a classifier would have earned (D-24). \
**CONTAINMENT, EXACTLY.** It is NOT `\"i == 0 is today's rule\"`: \
`lexical_absolute_components` opens with `word.trim_start_matches(\"./\")` \
BEFORE it tests `starts_with('/')`. The precise statement is that for every \
word today's rule answers `Some(v)` for, the string it actually NORMALISES — \
`w` with its leading `./`s stripped — begins with `/`, is therefore itself one \
of the candidates, and re-normalises to exactly `v`. The candidate set \
contains today's answer BYTE FOR BYTE; the `./` strip can only ADD answers and \
can never remove one. **No refusal that exists today can be lost.**";

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
/// **Section 12 uses this and ONLY this**, for the reason stated in this file's
/// header: `19-31` writes no rule for `C-11` … `C-15`, for `C-08`'s behavioural
/// half, for `T-19-120`'s behavioural half, for `T-19-116`'s three residues, for
/// `T-19-121`'s unreached key shapes or for the derived over-refusal.
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
/// EXACT-PATH boundary rather than a prefix one.
fn this_binary_parent() -> PathBuf {
    this_binary()
        .parent()
        .expect("the running binary has a parent directory")
        .to_path_buf()
}

/// The PRODUCT binary, which is what a hook stub must exec.
const PRODUCT_BIN: &str = env!("CARGO_BIN_EXE_gsd-meta-manager");

/// The envelope root spelled RELATIVE TO `$HOME`, for the tilde rows.
const ENVELOPE_ROOT_RELATIVE_TO_HOME: &str = ".local/share/gsd-meta-manager/envelope";

/// The REAL envelope root this machine resolves, for the tilde rows.
///
/// **Nothing here writes to it.** The guard DECIDES about a command; it never
/// executes one.
fn real_envelope_root() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(ENVELOPE_ROOT_RELATIVE_TO_HOME))
}

// ---------------------------------------------------------------------------
// THE SIMULATION — `19-31`'s mandated design, computed HERE so this file can
// check its own derived expectations against a tree that does not have the rule
// ---------------------------------------------------------------------------
//
// **Why a simulation is legitimate here and is not a second implementation.**
// Every derived-post-fix verdict in this file is a claim about what `19-31`'s
// MANDATED design answers. A claim nobody can compute is a claim nobody can
// check, and `19-27` measured a plan mandating a row that certified nothing.
// These three functions are the design's own definition, written as code, and
// section 10 uses them to prove (a) CONTAINMENT — the candidate set holds
// today's answer byte for byte — and (b) that every fenced row in every file
// `19-31` may not edit keeps its verdict. **They are test-local and no
// production line depends on them.**

/// EVERY `/`-anchored substring of `word` — the candidate set.
///
/// The rule never asks what precedes the `/`.
fn slash_anchored_candidates(word: &str) -> Vec<&str> {
    word.char_indices()
        .filter(|(_, character)| *character == '/')
        .map(|(index, _)| &word[index..])
        .collect()
}

/// `lexical_absolute_components` (`policy.rs:5457-5476`), mirrored byte for
/// byte so the simulation runs the SAME normalisation the rule already uses.
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

/// Whether ANY candidate of `word` resolves under `dir` or equals `binary` —
/// the widened predicate, over the two EXISTING comparisons.
fn simulated_protected(word: &str, dir: Option<&str>, binary: Option<&str>) -> bool {
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
// SECTION 1 — THE ATTACHMENT SWEEP: the measurement that DECIDES where the
// boundary is
//
// **Every row here was driven at the guard AND probed under real `bash` before
// it was written.** A spelling the guard permits but the shell does not reach
// is not a bypass, and the reach leg is what tells them apart — so the rows
// whose reach was demonstrated are ASSERTED at their derived post-fix verdict
// and the rows whose reach was NOT demonstrated are RECORDED in section 12.
//
// **THE REACH TABLE, MEASURED under real `bash` at this file's base:**
//
// ```text
// dd if=/dev/null of=<D>/pr-ledger.ndjson   REACHES  (the file was truncated)
// tar --directory=<D> -cf /tmp/t .          REACHES  (`tar -tf` lists the dir's own entries)
// cp --target-directory=<D> /bin/true       REACHES  (`true` appeared in the dir)
// rsync --temp-dir=<D> /bin/true /tmp/x     REACHES  (accepted and used; the temp file is removed)
// chmod --reference=<D>/pr-ledger.ndjson F  REACHES  (F took the referenced file's 0644)
// tar -C<D> -cf /tmp/t .                    REACHES  — an attached SHORT option with NO `=`
// cp -t<D> /bin/true                        REACHES  — the same, second program
// PATH=/usr/bin:<D> mytool                  REACHES  — bash EXECS `mytool` out of <D>
// git --git-dir=<D> rev-parse --git-dir     REACHES  (git names <D> back in its own error)
// python3 --out=<D>/x                       DOES NOT — `unknown option --out=…`
// mytool --opt=a=<D>/pr-ledger.ndjson       DOES NOT — no program with that grammar was found
// ssh -oControlPath=<D>/s host              DOES NOT — the connection never opened
// rsync host:<D> /tmp/x                     DOES NOT — no remote to reach
// ```
//
// **The two genuinely-reaching TWO-`=` spellings that WERE found are both
// already refused today by OTHER clauses**, so neither discriminates on this
// axis and both are RECORDED with their overlap stated in section 12:
// `GIT_CONFIG_PARAMETERS='core.excludesFile=<D>/excl' git status` is exit 2
// `hook_bypass_blocked`, and `git -c alias.q=--git-dir=<D> q` is exit 2
// `envelope_assertion_failed` from round 9's re-parse clause.
// ===========================================================================

#[test]
fn after_19_31_an_equals_attached_carrier_path_is_refused_over_both_protected_paths() {
    // **THE WHOLE FINDING IN TWO LINES.** `cp /bin/true <BINARY>` is exit 2
    // today; `dd if=/bin/true of=<BINARY>` — the same program-free replacement,
    // differing ONLY in the `=` join — is exit 0, and audit 11 drove it to a
    // moved bare remote. Section 2 re-drives that end to end.
    let binary = this_binary().display().to_string();
    refuses(
        &format!("dd if=/bin/true of={binary}"),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-119` OVER THE BINARY — layers 2 AND 3 in one permitted call.**\n\n\
             **THE CONTROL AND WHAT MAKES THE PAIR DISCRIMINATING:** its twin \
             `cp /bin/true <BINARY>` is ALREADY exit 2 `envelope_assertion_failed` today \
             (re-asserted in section 11 as round 11's exact-path clause), and \
             `dd if=/bin/true of=<BINPAR>/some-other-file` is exit 0 before AND after. The \
             three rows differ in exactly one thing each: the JOIN, and whether the interior \
             path is the binary. A control refused the same way at the same identifier would \
             prove nothing — that is `19-27`'s measured failure mode.\n\n\
             {INTERIOR_PATH_DERIVATION}"
        ),
    );
    permits(
        &format!(
            "dd if=/bin/true of={}/some-other-file",
            this_binary_parent().display()
        ),
        "**THE EXACT-PATH-NOT-PREFIX CONTROL, in the `=`-attached spelling.** The binary half \
         is an EQUALITY because its directory is shared with everything else the user \
         installed; a candidate naming a SIBLING can never equal the full binary path. A rule \
         that widened this half to a directory prefix turns this row red rather than turning a \
         driven run unusable (AR-19-11).",
    );

    refuses_carrier(
        |root| format!("dd if=/dev/null of={}/alpha/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**`T-19-119` OVER THE ENVELOPE DIRECTORY — this spelling RESETS A FIRED PR CAP**, \
             re-driven end to end in section 3.\n\n\
             **THE CONTROL:** its twin `cp /dev/null <ENV>/alpha/pr-ledger.ndjson` is ALREADY \
             exit 2 today, and `dd if=/dev/null of=/tmp/pr-ledger.ndjson` — the BASENAME near \
             miss — is exit 0 before AND after. The pair differs in exactly one character.\n\n\
             {INTERIOR_PATH_DERIVATION}"
        ),
    );
    permits(
        "dd if=/dev/null of=/tmp/pr-ledger.ndjson",
        "**THE BASENAME NEAR MISS, in the `=`-attached spelling.** The boundary is the \
         DIRECTORY this run owns, never a filename: a candidate `[tmp, pr-ledger.ndjson]` is \
         SHORTER than any three-component envelope directory, and `word_is_within` returns \
         early on `word.len() < dir.len()`. **That early return is the mechanical reason the \
         interior scan costs nothing.**",
    );

    // -- The remaining `=`-attached spellings audit 11 measured, each with its
    //    outside-the-path-set twin pinned PERMITTED in section 4.
    for template in [
        "tar --directory={}/alpha -cf /tmp/t .",
        "cp --target-directory={}/alpha /bin/true",
        "chmod --reference={}/alpha/pr-ledger.ndjson /tmp/x",
        "rsync --temp-dir={}/alpha /bin/true /tmp/x",
    ] {
        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            &format!(
                "**`T-19-119` reaches `C-01` … `C-09` through an option-attached value, and \
                 real `bash` was confirmed to reach the file for every one of these four.** \
                 `chmod --reference=` is a READ and is refused with the writes for the reason \
                 `protected_carrier_named`'s own cost section already discloses — the guard \
                 cannot tell a read from a write without knowing every program's grammar.\n\n\
                 **THE CONTROL:** each has an outside-the-path-set twin pinned PERMITTED in \
                 section 4 (`tar --directory=/tmp/g`, `cp --target-directory=/tmp/g`, \
                 `chmod --reference=/tmp/g/x`), and each has a SPACE-SEPARATED twin that is \
                 already exit 2 today. The pairs differ in one character each.\n\n\
                 {INTERIOR_PATH_DERIVATION}"
            ),
        );
    }

    // -- The BINARY in an `=`-attached position that is NOT `of=`, so the
    //    exact-path half is reached by more than one spelling.
    refuses(
        &format!("tar --directory={binary} -cf /tmp/t ."),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "the EXACT-PATH half must be reached by the interior scan too, not only the \
             prefix half. **THE CONTROL:** `tar --directory=/tmp/g -cf /tmp/t .` is exit 0 \
             before and after (section 4), and `ls <BINPAR>` stays permitted — so this row \
             measures the EQUALITY rather than the option.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_31_an_attachment_that_is_not_an_equals_sign_is_reached_the_same_way() {
    // **THIS IS THE ROW THAT PROVES AN `=`-KEYED RULE WOULD BE ONE CHARACTER
    // SHORT IN EXACTLY THE WAY THE CURRENT ONE IS.** `tar -C<path>` and
    // `cp -t<path>` carry an absolute path with NO `=` anywhere in the word, and
    // real `bash` reaches the directory for both. A rule that split at the first
    // `=`, at every `=`, or on a LIST of attachment characters would miss them —
    // and a character list is a PROGRAM-GRAMMAR ENUMERATION, which is D-08's
    // defect one level over and is what
    // `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
    // forbids one level over.
    for template in ["tar -C{}/alpha -cf /tmp/t .", "cp -t{}/alpha /bin/true"] {
        let template = template.to_string();
        refuses_carrier(
            |root| template.replace("{}", &root.display().to_string()),
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            &format!(
                "**AN ATTACHED SHORT OPTION WITH NO `=` AT ALL, REACH CONFIRMED UNDER REAL \
                 `bash`.**\n\n\
                 **THE CONTROL AND WHAT MAKES THE PAIR DISCRIMINATING:** `tar -C/tmp/g` and \
                 `cp -t/tmp/g` are exit 0 before AND after (section 4). The pair differs only \
                 in whether the interior path is the protected one — never in the attachment, \
                 which is identical on both sides. **That is what makes this a measurement of \
                 the PATH SET rather than of the `=`.**\n\n{INTERIOR_PATH_DERIVATION}"
            ),
        );
    }
}

#[test]
fn after_19_31_a_colon_attachment_inside_an_assignment_prefix_is_reached_too() {
    // **A `:` ATTACHMENT, AND THE PATH IS AT A NON-ZERO INDEX BEHIND TWO
    // DIFFERENT CHARACTERS AT ONCE** — an `=` (the assignment) and a `:` (the
    // `PATH` separator). Real `bash` REACHES it: with an executable dropped in
    // the envelope directory, `PATH=/usr/bin:<ENV>/alpha mytool` runs it.
    //
    // **This row is also why the design is NOT "split at the first `=` and test
    // the tail":** the tail here is `/usr/bin:<ENV>/alpha`, which normalises to
    // `[usr, bin:<root>, alpha]` — a component list that is not the envelope
    // directory at all. Only a `/`-anchored scan reaches it.
    refuses_carrier(
        |root| format!("PATH=/usr/bin:{}/alpha mytool", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**THE CONTROL:** `PATH=/usr/bin:/tmp/g mytool` is exit 0 before AND after \
             (section 4), and so is `PATH=/usr/bin:<BINPAR> mytool` — the binary's PARENT, \
             which the EXACT-PATH half deliberately does not cover. The three rows differ \
             only in what the interior path IS.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
}

#[test]
fn after_19_31_an_assignment_prefix_naming_a_protected_value_is_refused_and_the_unprotected_twin_is_not(
) {
    // **THE ASSIGNMENT PREFIX, DRAWN AT BOTH VERDICTS SO THE ROW DISCRIMINATES
    // RATHER THAN MERELY PERMITTING.**
    //
    // `FOO=/tmp/x cmd` stays PERMITTED, **and the reason is the PATH SET rather
    // than an assignment-prefix exception — no exception for them is written.**
    // The consequence in the other direction is a GAIN and not a cost: it closes
    // the obvious next spelling `R=<ENV>/alpha; rm -rf $R`, whose `rm` operand is
    // expansion-borne and permitted, leaving the assignment as the only place the
    // path is literal.
    refuses_carrier(
        |root| {
            format!(
                "GSD_MM_ENVELOPE_ROOT={}/alpha gh pr create --title x",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**THE CONTROL:** `GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x` is exit \
             0 before AND after — it is drawn in section 4 and is RECORDED (not asserted) at \
             `tests/envelope_control_carrier.rs:1105-1107` as `E-01`'s inert-prefix row. The \
             pair differs ONLY in whether the assigned value is under this run's own envelope \
             directory.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| format!("R={}/alpha", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "the BARE assignment, with no command after it. **THE CONTROL:** `R=/tmp/g` is \
             exit 0 before and after (section 4).\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
}

// ===========================================================================
// SECTION 2 — `T-19-119` DRIVEN END TO END AGAINST REAL GIT
//
// **The bare-remote fixture is REBUILT here rather than cited from audit 11.**
// Both remote SHAs are recorded on every leg, a CONTROL sits on both sides of
// the replacement, and LAYER 2 is measured SEPARATELY from layer 3.
//
// **Measured at this file's base:**
//
// ```text
// CONTROL  real binary                    push -> REFUSED,   main 0bd5d44 UNMOVED
// GUARD    cp /bin/true <BINARY>               -> exit 2
// GUARD    dd if=/bin/true of=<BINARY>         -> exit 0     (the whole finding, two lines)
// LEG B    binary replaced through `of=`  push -> COMPLETED, main 0bd5d44 -> 6efde46 MOVED
// LAYER 2  <replaced> envelope guard alpha     -> exit 0     (against exit 2 for the real one)
// CONTROL  binary restored, remote rewound push-> REFUSED,   main 0bd5d44 UNMOVED
// ```
//
// **These two tests assert the PRE-FIX behaviour, so they are GREEN today and
// GREEN after `19-31`.** `19-31`'s clause is a GUARD rule: it refuses the
// *command that replaces the binary*, before the replacement happens. It cannot
// and must not change what git does once the binary is already `/bin/true`, and
// pretending otherwise is exactly the kind of claim this phase exists to refuse.
// The rows that go green after `19-31` are section 1's.
// ===========================================================================

/// Replace `path`'s contents with `/bin/true` **through the `of=` spelling**,
/// reporting HOW it succeeded.
///
/// **The direct write hits `ETXTBSY` when the kernel still holds the file open
/// for execution from the hook that just ran, and that is `C-10`'s OWN SEAM** —
/// the same race the documented `tests/envelope_tracer.rs` flake is about. It is
/// RECORDED rather than smoothed.
fn replace_through_of(path: &Path) -> String {
    for _ in 0..20 {
        let status = std::process::Command::new("dd")
            .arg("if=/bin/true")
            .arg(format!("of={}", path.display()))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("dd is on PATH");
        if status.success() {
            return "dd if=/bin/true of=<BINARY> (direct)".to_string();
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let staged = path.with_extension("staged");
    std::fs::copy("/bin/true", &staged).expect("staging a replacement succeeds");
    std::fs::rename(&staged, path).expect("renaming over the binary succeeds");
    "mv <staged> <BINARY> (after ETXTBSY on the direct dd — C-10's own seam)".to_string()
}

#[test]
fn the_t_19_119_option_attached_replacement_moves_a_bare_remote_with_a_control_on_both_sides() {
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

    // -- CONTROL, before: the real binary in place.
    let before = rev(&remote, "main");
    assert!(
        !force_push("CONTROL real binary"),
        "the CONTROL leg must be REFUSED by the pre-push hook. If it is not, the fixture is not \
         delivering the hook and every leg below measures nothing."
    );
    assert_eq!(
        rev(&remote, "main"),
        before,
        "the CONTROL leg must leave the remote's `main` UNMOVED. before={before}"
    );

    // -- **THE TWO-LINE FINDING, DRIVEN OVER THE GUARD'S OWN BINARY.**
    //
    // **AND THE PATH THIS PAIR IS ABOUT IS `current_exe()`, NOT THE FIXTURE'S
    // COPY — measured, and the distinction is the reason the end-to-end legs
    // below can run at all.** `word_is_exactly` compares against the binary
    // `guard` was handed, which under `cargo test` is THIS TEST BINARY. The
    // fixture's private copy is a different absolute path, so the guard is
    // silent about it in BOTH spellings — recorded below, asserted in neither
    // direction, and it is what lets the layer-3 legs proceed without the
    // corpus's own protection getting in the way.
    let guarded = this_binary();
    let joined = ask(
        &root.join("guard-env-joined"),
        &format!("dd if=/bin/true of={}", guarded.display()),
    );
    let separated = ask(
        &root.join("guard-env-separated"),
        &format!("cp /bin/true {}", guarded.display()),
    );
    let fixture_joined = ask(
        &root.join("guard-env-fixture-joined"),
        &format!("dd if=/bin/true of={}", binary.display()),
    );
    let fixture_separated = ask(
        &root.join("guard-env-fixture-separated"),
        &format!("cp /bin/true {}", binary.display()),
    );
    println!(
        "  [TWIN PAIR over current_exe()] `dd if=… of=<BINARY>` exit={}   \
         `cp /bin/true <BINARY>` exit={}",
        joined.code, separated.code
    );
    println!(
        "  RECORDED (not asserted) [the `=`-joined verdict itself]: exit={} at this file's \
         base. **It is ASSERTED at its DERIVED post-fix verdict in section 1**, not here — a \
         row asserted at the PRE-fix verdict in a file `19-31` may only ADD to would land \
         permanently red the day the rule lands.",
        joined.code
    );
    println!(
        "  RECORDED (not asserted) [the FIXTURE's private copy, a path the guard was not \
         handed]: `dd if=… of=<copy>` exit={}   `cp /bin/true <copy>` exit={} — the EXACT-PATH \
         half is silent about a binary that is not this run's own, which is fail-open and is \
         stated on `protected_carrier_named`'s own signature",
        fixture_joined.code, fixture_separated.code
    );
    assert_eq!(
        separated.code, 2,
        "\n\n**THE SPACE-SEPARATED TWIN MUST BE REFUSED.** It is round 11's exact-path clause \
         and it is the control this whole finding is measured against. stdout: {}",
        separated.stdout
    );

    // -- LEG B: the binary replaced THROUGH THE PERMITTED SPELLING.
    let how = replace_through_of(&binary);
    println!("  [LEG B] the binary was replaced by: {how}");
    assert!(
        force_push("LEG B replaced binary"),
        "\n\n**`T-19-119` DID NOT REPRODUCE END TO END.**\n\n\
         With the binary the stubs exec replaced through the permitted `of=` spelling, the \
         force push must COMPLETE. A refusal here means the fixture is not exercising the \
         carrier and the finding must be RE-MEASURED rather than asserted at audit 11's number."
    );
    let after = rev(&remote, "main");
    assert_ne!(
        after, before,
        "\n\n**THE REMOTE MUST HAVE MOVED.**\n  before : {before}\n  after  : {after}\n\n\
         The `of=` replacement removes LAYER 3 without touching `GIT_CONFIG_COUNT` at all, so \
         the hook the envelope installed still runs — it just execs a program that exits 0."
    );
    println!("  [LEG B] remote main {before} -> {after}  MOVED");

    // -- CONTROL, after: restore the binary, rewind the remote, same push.
    std::fs::copy(PRODUCT_BIN, &binary).expect("the product binary restores");
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
        !force_push("CONTROL restored binary"),
        "restoring the binary must restore the refusal. Without this leg, LEG B could be \
         explained by anything that happened between the two pushes."
    );
    assert_eq!(
        rev(&remote, "main"),
        before,
        "the restored-binary control must leave the remote's `main` UNMOVED again"
    );
}

#[test]
fn the_t_19_119_replacement_takes_layer_2_as_well_and_that_is_measured_separately() {
    // **LAYER 2, MEASURED ON ITS OWN.** `guard_command` bakes the SAME binary
    // path into `settings.json` as the `PreToolUse` hook, so the replacement
    // that removed layer 3 above also answers every guard call with exit 0.
    let fixture = TempDir::new().unwrap();
    let envelope = fixture.path().join("env");
    let real = fixture.path().join("real");
    let replaced = fixture.path().join("replaced");
    std::fs::copy(PRODUCT_BIN, &real).expect("the product binary copies");
    std::fs::copy("/bin/true", &replaced).expect("/bin/true copies");

    let request = serde_json::json!({
        "session_id": "fixture",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": "git push --force origin main" },
    })
    .to_string();

    let drive = |binary: &Path| -> i32 {
        use std::io::Write as _;
        let mut child = std::process::Command::new(binary)
            .args(["envelope", "guard", ALIAS])
            .env("GSD_MM_ENVELOPE_ROOT", &envelope)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the binary spawns");
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(request.as_bytes())
            .expect("the request is written");
        child.wait().expect("the guard exits").code().unwrap_or(-1)
    };

    let real_code = drive(&real);
    let replaced_code = drive(&replaced);
    println!("  layer 2: real binary exit={real_code}, replaced binary exit={replaced_code}");

    assert_eq!(
        real_code, 2,
        "the REAL binary must REFUSE `git push --force origin main` at layer 2. This is the \
         control: without it, the replaced binary's exit 0 could be explained by the request \
         being malformed."
    );
    assert_eq!(
        replaced_code, 0,
        "\n\n**LAYER 2 DID NOT REPRODUCE.**\n\n\
         The replaced binary must answer exit 0 — a PERMIT — to a force push the real binary \
         refuses. Layer 2 and layer 3 fall to the SAME single permitted call, which is what \
         `T-19-119` reaches through an option-attached spelling `T-19-116`'s own rows do not."
    );
}

// ===========================================================================
// SECTION 3 — `T-19-119` RE-DRIVEN ON THE LEDGER, FROM A FIRED CAP
//
// **ONE PERSISTENT ROOT across all five calls**, because a fresh root per row
// would destroy the measurement: the cap only fires against a ledger that
// already holds this run's own attempts.
//
// **Measured at this file's base:**
//
// ```text
// call 1  gh pr create --title x  -> exit 0 (permit),      ledger 1 line
// call 2  gh pr create --title x  -> exit 2 pr_cap_exceeded, ledger 2 lines
// call 3  gh pr create --title x  -> exit 2 pr_cap_exceeded, ledger 3 lines
// call 4  gh pr create --title x  -> exit 2 pr_cap_exceeded, ledger 4 lines (412 bytes)
// GUARD   cp /dev/null <ledger>   -> exit 2   (the TWIN)
// GUARD   dd if=/dev/null of=<ledger> -> exit 0
// (the permitted spelling performed)      ledger truncated to 0 bytes
// call 5  gh pr create --title x  -> exit 0 (PERMITTED AGAIN), ledger 1 line
// ```
//
// **GREEN today and after**, for the same reason as section 2: this is the
// pre-fix harm. The guard row `19-31` turns green is section 1's.
// ===========================================================================

#[test]
fn the_t_19_119_ledger_truncation_resets_a_fired_pr_cap_in_one_persistent_root() {
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

    // -- THE TWIN PAIR, driven against this same root before either is performed.
    let separated = ask(root, &format!("cp /dev/null {}", ledger.display()));
    let joined = ask(root, &format!("dd if=/dev/null of={}", ledger.display()));
    println!(
        "  [TWIN PAIR] `cp /dev/null <ledger>` exit={}   `dd if=/dev/null of=<ledger>` exit={}",
        separated.code, joined.code
    );
    assert_eq!(
        separated.code, 2,
        "the SPACE-SEPARATED twin must be REFUSED — round 10's clause over the envelope-root \
         operand rows. It is the control this finding is measured against."
    );
    println!(
        "  RECORDED (not asserted) [the `of=`-joined verdict itself]: exit={} at this file's \
         base. **It is ASSERTED at its DERIVED post-fix verdict in section 1**, not here — a \
         row asserted at the PRE-fix verdict in a file `19-31` may only ADD to would land \
         permanently red the day the rule lands. The truncation below is performed directly, \
         so THIS test's harm keeps reproducing after the rule closes the spelling.",
        joined.code
    );

    // -- Perform the permitted spelling, exactly as the guard let it through.
    let status = std::process::Command::new("dd")
        .arg("if=/dev/null")
        .arg(format!("of={}", ledger.display()))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("dd is on PATH");
    assert!(status.success(), "the permitted `dd` truncation succeeds");
    assert_eq!(
        std::fs::metadata(&ledger).unwrap().len(),
        0,
        "the ledger must be truncated to zero bytes"
    );

    let fifth = ask(root, "gh pr create --title x");
    println!(
        "  call 5 after the truncation: exit={} lines={}",
        fifth.code,
        lines()
    );
    assert_eq!(
        fifth.code, 0,
        "\n\n**THE CAP MUST BE RESET.**\n\n\
         After the truncation a fifth `gh pr create --title x` must be PERMITTED again — the \
         run has reset its own limit through a command the guard allowed. That is `T-19-112`'s \
         harm reached by a FOURTH route, and closing `T-19-119` narrows it without closing it. \
         Trail: {trail:?}"
    );
}

// ===========================================================================
// SECTION 4 — THE COST, PINNED FROM BOTH SIDES
//
// **This half is what stops the rule being written wrong.** Every row here is
// exit 0 today and asserted exit 0 AFTER, so a rule that widened past the PATH
// SET turns THIS file red rather than turning a driven run unusable (AR-19-11).
//
// **THE COST IS BOUNDED BY THE PATH SET AND NOT BY THE SPLIT, AND THAT IS WHY
// THE SPLIT CAN BE TOTAL.** Splitting produces more CANDIDATES; a candidate is
// refused only if it PREFIXES a directory of at least three components rooted at
// `<root>/<alias>`, or EQUALS a full binary path. `word_is_within` returns early
// on `dir.is_empty() || word.len() < dir.len()` over COMPONENT VECTORS, so a
// short suffix can never match a deep directory at all.
//
// ```text
// --author=A <a@b.c>                 no `/` in the word              -> no candidate
// --format=%H                        no `/`                          -> no candidate
// sed s/x/y/                         [x, y], [y], []                 -> shorter than any envelope dir
// https://github.com/o/r             [github.com, o, r], [o, r], [r] -> wrong first component
// HEAD:refs/heads/gsd-auto/alpha/w   [heads, gsd-auto, alpha, w], …  -> wrong first component
// git --git-dir=/tmp/g status        [tmp, g]                        -> not under <root>/<alias>
// FOO=/tmp/x cmd                     [tmp, x]                        -> not protected: PERMITTED
// rg pr-ledger.ndjson src/           `src/` has NO leading `/`       -> no candidate at all
// ```
// ===========================================================================

#[test]
fn the_cost_rows_carry_an_equals_and_a_slash_and_stay_permitted_before_and_after() {
    permits(
        "git --git-dir=/tmp/g status",
        "**AN `=`-ATTACHED ABSOLUTE PATH OUTSIDE THE PROTECTED SET, AND THE MOST LOAD-BEARING \
         COST ROW IN THE PHASE.** It is pinned PERMITTED in FOUR other places — \
         `tests/envelope_callee_grammar.rs:1025`, `tests/envelope_reparsed_value.rs:1075`, \
         `tests/envelope_config_resolution.rs:1169` and `tests/envelope_wrapper_class.rs:5133`'s \
         `GitGlobalOption` entry — and it is named in `policy.rs`'s own docs at `:413`, `:763`, \
         `:874`, `:4304` and `:4308`. **A rule that turned it red would turn five files red at \
         once.** Its candidate `[tmp, g]` is two components against a three-component envelope \
         directory, so `word_is_within`'s early return refuses to compare them at all.",
    );
    permits(
        "git -c aliasx.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "**TWO `=` INSIDE ONE QUOTED WORD.** Its candidates are `[tmp, evil.cfg]` and \
         `[evil.cfg]`, neither of which can prefix a three-component envelope directory. This \
         row also carries round 9's `--signed no` control (`aliasx.` is not `alias.`), so a \
         rule that widened past the path set would break TWO rounds' claims at once.",
    );
    permits(
        "git log --format=%H",
        "a `%`-formatted value with NO `/` anywhere in the word — there is no candidate to \
         produce.",
    );
    permits(
        "git commit --author='A <a@b.c>'",
        "an `=`-attached value carrying an `@` and an angle-bracketed address, and still no \
         `/`. **The `<` here is INSIDE a quoted word and is not a redirection operator**, which \
         is round 6's grammar and is re-asserted in section 11.",
    );
    permits(
        "sed s/x/y/ /tmp/f",
        "a substitution whose candidates are `[x, y]`, `[y]` and the EMPTY component list, and \
         an operand whose candidate is `[tmp, f]`. **The empty list is the degenerate case and \
         it is handled by the same early return**: zero components is shorter than three.",
    );
    permits(
        "curl https://github.com/o/r",
        "a URL operand. Its candidates are `[github.com, o, r]`, `[o, r]` and `[r]` — every one \
         rooted at a component no envelope directory carries.",
    );
    permits(
        "git push origin HEAD:refs/heads/gsd-auto/alpha/w",
        "**A `:`-ATTACHED PATH WITH THREE `/` IN IT, WHOSE CANDIDATES DELIBERATELY INCLUDE THE \
         WORD `alpha`.** `[heads, gsd-auto, alpha, w]` is the longest, and it fails on its \
         FIRST component rather than on its length — which is the half of the cost argument \
         the length early return does not cover.",
    );
    permits(
        "git push origin refs/heads/gsd-auto/alpha/w",
        "the same ref with no `:` at all, which is what a driven run actually pushes.",
    );
    permits(
        "rg pr-ledger.ndjson src/",
        "**A WORD WHOSE ONLY `/` IS TRAILING.** `src/`'s single candidate is `/`, which \
         normalises to the EMPTY component list. A rule that treated an empty list as a match \
         would refuse every command carrying a trailing slash.",
    );
    permits(
        "rm -f /tmp/pr-ledger.ndjson",
        "round 10's `--signed no`, carried forward: the BASENAME is a carrier filename and the \
         DIRECTORY is not the envelope root.",
    );
    permits(
        "GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x",
        "**AN ASSIGNMENT PREFIX WITH AN UNPROTECTED VALUE, AND THE OTHER HALF OF SECTION 1's \
         DISCRIMINATING PAIR.** It stays permitted because `/tmp/fresh` is not a protected \
         path — **not because assignment prefixes are excluded, and no exception for them is \
         written.** `tests/envelope_control_carrier.rs:1105-1107` RECORDS this same row as \
         `E-01`'s inert-prefix measurement; this file ASSERTS it, because after `19-31` its \
         verdict is load-bearing.",
    );
    permits(
        "R=/tmp/g",
        "the bare assignment, unprotected value — the control for section 1's `R=<ENV>/alpha`.",
    );
    permits(
        "mytool --opt=a=/tmp/g/x",
        "a TWO-`=` word under a program name absent from both production halves, with an \
         unprotected interior path. **The program name is the point**: the rule must never \
         have to know it (D-08).",
    );
    for command in [
        "tar --directory=/tmp/g -cf /tmp/t .",
        "tar -C/tmp/g -cf /tmp/t .",
        "cp --target-directory=/tmp/g /bin/true",
        "cp -t/tmp/g /bin/true",
        "chmod --reference=/tmp/g/x /tmp/y",
        "rsync --temp-dir=/tmp/g /bin/true /tmp/x",
        "PATH=/usr/bin:/tmp/g mytool",
        "dd if=/tmp/g/x of=/tmp/stolen",
    ] {
        permits(
            command,
            "**THE OUTSIDE-THE-PATH-SET TWIN OF ONE OF SECTION 1's ASSERTED ROWS, IN THE SAME \
             ATTACHMENT.** The pair differs ONLY in whether the interior path is the protected \
             one, which is what makes section 1 a measurement of the PATH SET rather than of \
             the attachment character. A rule keyed to `=`, to `:` or to an attached short \
             option would turn these red and would still be one character short.",
        );
    }
}

#[test]
fn the_near_miss_and_exact_path_controls_stay_permitted_before_and_after() {
    permits_carrier(
        |root| {
            format!(
                "cat {}x/alpha/pr-ledger.ndjson",
                root.display()
            )
        },
        "**THE ONE-CHARACTER-CHANGED ROOT.** Its longest candidate differs from the envelope \
         directory in a single component, and the comparison is COMPONENT-WISE rather than a \
         raw `starts_with` — so a sibling root is not this root. **The interior scan does not \
         change that**: every candidate is normalised the same way before it is compared.",
    );
    permits(
        &format!(
            "cp /bin/true {}/some-other-file",
            this_binary_parent().display()
        ),
        "the EXACT-PATH-not-PREFIX control: a sibling of the binary in the same shared \
         directory. A clause written as a directory prefix refuses `cargo install` and \
         `ls ~/.cargo/bin`, which is how a safety control gets switched off (AR-19-11).",
    );
    permits(
        &format!("ls {}", this_binary_parent().display()),
        "the same, from the other side — the DIRECTORY the binary lives in is not protected.",
    );
    permits(
        &format!("PATH=/usr/bin:{} mytool", this_binary_parent().display()),
        "**AND THE SAME CONTROL IN THIS ROUND'S OWN NEW ATTACHMENT.** The binary's PARENT \
         carried at a non-zero index behind a `:` is still not the binary, because the \
         exact-path half is an EQUALITY over the whole candidate.",
    );
    permits(
        &format!("cp -t{} /bin/true", this_binary_parent().display()),
        "the same again behind an attached short option with no `=`.",
    );
}

#[test]
fn the_verdict_preserving_spellings_from_rounds_10_and_11_stay_permitted() {
    // **THE WHOLE PERMITTED HALF OF ROUNDS 10 AND 11, CARRIED FORWARD
    // UNCHANGED.** `19-31` widens what a LITERAL word is read for; it must not
    // touch the LITERALNESS FILTER itself, which is applied BEFORE any candidate
    // is produced. If it did, every row here would turn red — a rule that
    // quietly widened.
    if let Some(real_root) = real_envelope_root() {
        println!("  the real envelope root resolves to {}", real_root.display());
    }
    permits(
        "rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha",
        "direction (v), the TILDE: resolving it needs the ENVIRONMENT, which the guard may not \
         read at guard time. **`Token.literal` is FALSE for this word, so the interior scan \
         never runs on it** — which is exactly why the filter must stay and must be applied \
         first.",
    );
    permits_carrier(
        |root| format!("rm -rf {}/alph?", root.display()),
        "direction (vi), the GLOB: resolving it needs the FILESYSTEM, which the latency and \
         TOCTOU rules forbid. Not `Token.literal`.",
    );
    permits_carrier(
        |root| {
            format!(
                "rm -f {}/alpha/{{pr-ledger.ndjson,x}}",
                root.display()
            )
        },
        "direction (vii), the BRACE LIST. Not `Token.literal`.",
    );
    permits(
        "D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson",
        "direction (ii), EXPANSION-BORNE — audit 9's own composite. The carrier's location is \
         fetched by a PERMITTED GOVERNED READ and then acted on by an ungoverned command.",
    );
    permits(
        "rm -f pr-ledger.ndjson",
        "direction (iv), a RELATIVE word with NO `/` at all. **The restated residue's SECOND \
         clause covers it**: a word whose text carries no absolute path anywhere has nothing in \
         it to normalise.",
    );
    permits(
        "echo x > /tmp/outside && ls",
        "round 11's outside-the-root redirection control: the target is absolute and literal \
         and is NOT under the envelope root.",
    );
    permits(
        "git config --get core.hooksPath",
        "round 10's permitted READ, pinned UNCHANGED: a human debugging the run can still ask \
         where the hooks are (AR-19-11).",
    );
    permits(
        "git -c alias.p='!git push --force origin main' p",
        "**`T-19-86` — OPEN at `high` by explicit user scoping decision, and this row MAY NOT \
         MOVE.** `19-30` does not fix, narrow, re-scope or re-classify it, and `T-19-111` is \
         kept OUT of it.",
    );
}

// ===========================================================================
// SECTION 5 — THE NEW OVER-REFUSAL SURFACE, STATED IN ITS GENERAL FORM AND
// RECORDED AT BOTH VERDICTS
//
// **THE SURFACE, GENERALLY:** any word whose text contains **this run's own**
// envelope directory or **this run's own** binary path as a `/`-anchored
// substring is refused after `19-31` **even where the program would not have
// used that substring as a path** — as a value, a pattern, a message, a URL
// fragment or a relative path that happens to contain it.
//
// **It fails CLOSED, it is bounded to words naming this run's own directory or
// binary, and it is the same family as the already-disclosed refusal of
// `cat <ledger>`** (`protected_carrier_named`'s own cost section). It is stated
// here as a cost rather than discovered later.
//
// **THREE INSTANCES ARE DRIVEN so the record is about the SURFACE rather than
// one spelling**, and **not one of them is asserted**: a derived over-refusal is
// a cost to disclose, not a control to certify. Each is measured at exit 0 today
// with its DERIVED post-fix verdict written beside it.
//
// **The relative instance's reach was measured under real `bash`**: from a
// working directory that is not `/`, `rm -f ./tmp/<root>/alpha/pr-ledger.ndjson`
// deleted a DECOY file under `$PWD` and left the real ledger untouched — which
// is precisely why the refusal is an over-refusal rather than a mitigation.
// ===========================================================================

/// The envelope root spelled as a RELATIVE word that still contains it — the
/// most reachable instance of the disclosed over-refusal.
fn relative_spelling_of(root: &Path) -> String {
    format!("./{}", root.display().to_string().trim_start_matches('/'))
}

#[test]
fn the_derived_over_refusal_surface_is_recorded_in_three_instances_and_asserted_in_neither_direction(
) {
    println!(
        "\n=== THE DERIVED OVER-REFUSAL SURFACE — RECORDED, NEVER ASSERTED ===\n\
         Every row below is exit 0 TODAY. Its DERIVED post-fix verdict is exit 2 \
         `envelope_assertion_failed`, because the word's text carries this run's own envelope \
         directory as a `/`-anchored substring even though the program would not have used \
         that substring as a path.\n"
    );

    record_only(
        "INSTANCE 1 — a RELATIVE word containing the carrier path. DERIVED post-fix: exit 2. \
         Under real bash this reaches `$PWD/tmp/<root>/alpha/pr-ledger.ndjson`, a DIFFERENT \
         file — measured with a decoy, which survived the real ledger untouched",
        |root| format!("rm -f {}/alpha/pr-ledger.ndjson", relative_spelling_of(root)),
    );
    record_only(
        "INSTANCE 2 — a NON-PATH VALUE carrying the carrier path. DERIVED post-fix: exit 2. \
         `--format=` is a git output template and names no file at all",
        |root| format!("git log --format={}/alpha", relative_spelling_of(root)),
    );
    record_only(
        "INSTANCE 3 — a URL-SHAPED OPERAND carrying it after a `#`. DERIVED post-fix: exit 2. \
         The fragment is never a path; the candidate at the interior `/tmp` is",
        |root| {
            format!(
                "curl https://example.com/q#{}/alpha",
                relative_spelling_of(root)
            )
        },
    );
    record_only(
        "INSTANCE 3b — THE SAME URL WITH A `?` INSTEAD OF A `#`, AND IT IS A DIFFERENT ANSWER. \
         DERIVED post-fix: exit 0, UNCHANGED — `?` is a pathname-expansion metacharacter, so \
         `Token.literal` is FALSE and the interior scan never runs on the word. **The two \
         spellings look identical and are not**, which is why the surface is stated over what \
         the predicate READS rather than over what a URL looks like",
        |root| {
            format!(
                "curl https://example.com/q?p={}/alpha",
                relative_spelling_of(root)
            )
        },
    );

    // -- The MECHANICAL half: the simulation agrees with the derivation above,
    //    so the recorded post-fix verdicts are computed rather than asserted by
    //    eye. This is the only claim this test makes.
    let root = "/tmp/envroot/alpha";
    for (word, expected, why) in [
        (
            "./tmp/envroot/alpha/pr-ledger.ndjson",
            true,
            "instance 1 — a relative word whose candidate IS the protected path",
        ),
        (
            "--format=./tmp/envroot/alpha",
            true,
            "instance 2 — a non-path value carrying it",
        ),
        (
            "https://example.com/q#./tmp/envroot/alpha",
            true,
            "instance 3 — a URL fragment carrying it",
        ),
        (
            "--format=./tmp/envrootx/alpha",
            false,
            "and the one-character-changed control, which the surface does NOT reach",
        ),
    ] {
        assert_eq!(
            simulated_protected(word, Some(root), None),
            expected,
            "`{word}`: {why}. **The derived verdicts recorded above are COMPUTED from the \
             mandated design rather than asserted by eye** — a claim nobody can compute is a \
             claim nobody can check."
        );
    }
}

// ===========================================================================
// SECTION 6 — `T-19-121`: THE CREDENTIAL REACH ON ONE PERMITTED LINE, AND THE
// WHOLE KEY-SHAPE SPACE
//
// **`cred.rs:420-425`'s claim, QUOTED VERBATIM:**
//
// > * **a later `-c credential.helper=<something>` on the same command line,
// >   which OVERRIDES the reset and brings the secret back.** That is a BOUNDED
// >   residue rather than a reason to decline the control, and the bound is
// >   stated rather than assumed: **that spelling is ARGV-VISIBLE and is already
// >   governed** by [`super::policy::scan_leading`]'s leading-option region and
// >   layer 2's whole grammar — unlike every write spelling, which is not.
//
// **ARGV-VISIBLE it is. GOVERNED it is not.** `scan_leading` READS the word and
// **no rule ACTS on it**: the by-name deny covers `core.hooksPath`, round 8's
// confinement clause covers `include.path`, and nothing covers
// `credential.helper` — measured below at exit 0 on four separate surfaces.
//
// **This is `T-19-84`/`T-19-107`/`T-19-109`/`T-19-115`'s shape a SIXTH time**,
// and this phase has already shipped a COUNTED completeness claim ("FIVE forms")
// that was wrong the day it was written. **The correction is required whether or
// not any rule lands**, which is why the measurement is this task's product and
// no rule is asserted anywhere in this section.
//
// **THE KEY-SHAPE SPACE, MEASURED AGAINST REAL GIT under the envelope's full
// posture (`19-29`'s injected empty-helper pair present):**
//
// ```text
//                                                    real git   guard
// CONTROL  no `-c` at all                            exit 128, ABSENT   —
// -c credential.helper=store                         secret PRESENT     exit 0
// -c CREDENTIAL.HELPER=store                         secret PRESENT     exit 0
// -c Credential.Helper=store                         secret PRESENT     exit 0
// -c credential.https://github.com.helper=store      secret PRESENT     exit 0
// -c credentialx.helper=store                        secret ABSENT      exit 0
// -c notcredential.helper=store                      secret ABSENT      exit 0
// -c credential.helperx=store                        secret ABSENT      exit 0
// --config-env=credential.helper=EVILVAR             secret PRESENT     exit 0
// GIT_CONFIG_PARAMETERS='credential.helper=store'    secret PRESENT     exit 2 hook_bypass_blocked
// ```
//
// **The URL-SCOPED shape is what a by-name equality on `credential.helper`
// MISSES**, and the three near misses are what it must not catch. Both are
// recorded so `19-31` can state a by-name clause's partiality at the same weight
// rather than discover it.
// ===========================================================================

/// A `git` invocation under the envelope's own credential posture, including
/// `19-29`'s injected EMPTY-`credential.helper` pair.
fn git_under_envelope_posture(
    home: &Path,
    gitconfig: &Path,
    extra_pairs: &[(&str, &str)],
    args: &[&str],
    stdin: Option<&str>,
) -> (i32, String) {
    use std::io::Write as _;
    let mut command = std::process::Command::new("git");
    command
        .args(args)
        .env("HOME", home)
        .env("GIT_CONFIG_GLOBAL", gitconfig)
        .env("GIT_CONFIG_SYSTEM", gitconfig)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .stdin(if stdin.is_some() {
            std::process::Stdio::piped()
        } else {
            std::process::Stdio::null()
        })
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    // The COUNT is derived from the pairs and never written by hand — a count
    // that disagrees with the keys makes git ignore the injection ENTIRELY and
    // silently (`cred.rs:197-205`).
    command.env("GIT_CONFIG_COUNT", extra_pairs.len().to_string());
    for (index, (key, value)) in extra_pairs.iter().enumerate() {
        command.env(format!("GIT_CONFIG_KEY_{index}"), key);
        command.env(format!("GIT_CONFIG_VALUE_{index}"), value);
    }

    let mut child = command.spawn().expect("git is on PATH");
    if let Some(text) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(text.as_bytes())
            .expect("the request is written");
    }
    let out = child.wait_with_output().expect("git exits");
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

/// Whether a `git credential fill` answer carried the ambient secret.
///
/// **The secret is recorded as PRESENT/ABSENT and never transcribed.** SAFE-04's
/// own reasoning applies to this corpus exactly as it applies to the ledger.
fn fill_returned_the_ambient_secret(text: &str, username: &str) -> bool {
    text.lines().any(|line| line == format!("username={username}"))
        && text.lines().any(|line| line.starts_with("password="))
}

#[test]
fn t_19_121_the_credential_helper_key_shape_space_is_measured_with_the_control_driven_first() {
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

    // `19-29`'s injected EMPTY-`credential.helper` pair — the control this
    // finding overrides. Without it the CONTROL leg would pass for the wrong
    // reason.
    let injected: &[(&str, &str)] = &[("credential.helper", "")];
    let fill = |args: &[&str]| -> (i32, bool) {
        let mut full: Vec<&str> = args.to_vec();
        full.push("credential");
        full.push("fill");
        let (code, text) = git_under_envelope_posture(
            &home,
            &gitconfig,
            injected,
            &full,
            Some("protocol=https\nhost=github.com\n\n"),
        );
        (code, fill_returned_the_ambient_secret(&text, "probeuser"))
    };

    // -- **THE CONTROL, DRIVEN FIRST.** Without it, every PRESENT below could be
    //    explained by the posture never having failed closed at all.
    let (control_code, control_secret) = fill(&[]);
    println!("  CONTROL (no `-c`): exit {control_code}, ambient secret {}", if control_secret { "PRESENT" } else { "ABSENT" });
    assert!(
        !control_secret,
        "\n\n**THE CONTROL POSTURE MUST FAIL CLOSED.**\n\
         With `19-29`'s empty-`credential.helper` pair injected and no `-c` on the line, \
         `git credential fill` must NOT return the ambient username and password. Got exit \
         {control_code}. If this leg reaches the secret, the control is not working and every \
         row below measures nothing."
    );

    // -- **THE HARM.**
    let (harm_code, harm_secret) = fill(&["-c", "credential.helper=store"]);
    println!("  `-c credential.helper=store`: exit {harm_code}, ambient secret {}", if harm_secret { "PRESENT" } else { "ABSENT" });
    assert!(
        harm_secret,
        "\n\n**`T-19-121` DID NOT REPRODUCE.**\n\n\
         `git -c credential.helper=store credential fill` must OVERRIDE `19-29`'s reset and \
         return the AMBIENT username and password. Got exit {harm_code}. If it does not \
         reproduce, report the measured behaviour as a finding rather than asserting audit \
         11's number. Secret recorded PRESENT/ABSENT, never transcribed."
    );

    // -- **THE WHOLE KEY-SHAPE SPACE.**
    for (label, args, expected) in [
        ("CREDENTIAL.HELPER (case)", vec!["-c", "CREDENTIAL.HELPER=store"], true),
        ("Credential.Helper (case)", vec!["-c", "Credential.Helper=store"], true),
        (
            "credential.https://github.com.helper (URL-SCOPED)",
            vec!["-c", "credential.https://github.com.helper=store"],
            true,
        ),
        ("credentialx.helper (near miss)", vec!["-c", "credentialx.helper=store"], false),
        ("notcredential.helper (near miss)", vec!["-c", "notcredential.helper=store"], false),
        ("credential.helperx (near miss)", vec!["-c", "credential.helperx=store"], false),
    ] {
        let (code, secret) = fill(&args);
        println!(
            "  {label}: exit {code}, ambient secret {}",
            if secret { "PRESENT" } else { "ABSENT" }
        );
        assert_eq!(
            secret, expected,
            "\n\n**THE KEY-SHAPE SPACE IS THE PRODUCT OF THIS TASK.**\n\
             `{label}` must reach the secret: {expected}. Got {secret} at exit {code}.\n\n\
             **git folds the config SECTION and the VARIABLE to lower case and leaves the \
             SUBSECTION case-sensitive**, which is why the case rows resolve; the URL-SCOPED \
             shape is what a by-name equality on `credential.helper` would MISS; and the three \
             near misses are what it must NOT catch. **`19-31` decides from these measurements \
             whether a by-name clause can be written; this file asserts nothing about a rule.**"
        );
    }

    // -- `--config-env`, whose value names an ENVIRONMENT VARIABLE rather than
    //    the helper itself. Driven separately because it needs that variable set.
    let mut command = std::process::Command::new("git");
    use std::io::Write as _;
    let mut child = command
        .args([
            "--config-env=credential.helper=GSD_MM_PROBE_HELPER",
            "credential",
            "fill",
        ])
        .env("HOME", &home)
        .env("GIT_CONFIG_GLOBAL", &gitconfig)
        .env("GIT_CONFIG_SYSTEM", &gitconfig)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "credential.helper")
        .env("GIT_CONFIG_VALUE_0", "")
        .env("GSD_MM_PROBE_HELPER", "store")
        .env_remove("GIT_CONFIG_PARAMETERS")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("git is on PATH");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"protocol=https\nhost=github.com\n\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let config_env_secret = fill_returned_the_ambient_secret(&text, "probeuser");
    println!(
        "  --config-env=credential.helper=<VAR>: exit {:?}, ambient secret {}",
        out.status.code(),
        if config_env_secret { "PRESENT" } else { "ABSENT" }
    );
    assert!(
        config_env_secret,
        "**`--config-env` IS A SECOND SPELLING OF THE SAME KEY** and it reaches the secret \
         too. A by-name clause written only over `-c` values would not see it, which is what \
         `19-31` must state at the same weight rather than discover."
    );
}

#[test]
fn t_19_121s_cred_rs_bound_is_quoted_from_the_source_and_the_layers_it_names_are_measured() {
    // **THE FALSE CLAIM, READ OUT OF THE PRODUCTION SOURCE RATHER THAN RETYPED.**
    // A quotation a test retypes is a quotation that can drift from the file it
    // is about.
    //
    // **IT IS PRINTED AND NOT ASSERTED, DELIBERATELY.** `19-31` REPAIRS this
    // sentence, and an assertion that the old words are still there would land
    // permanently red in a file `19-31` may only ADD to. The claim is recorded
    // here; the correction is registered as `19-31`'s in `deferred-items.md`,
    // required whether or not any rule lands.
    assert!(
        CRED_SOURCE.contains("fn hooks_path_env"),
        "the `cred.rs` include must reach the real file, or the recording below is vacuous"
    );
    let still_present = CRED_SOURCE.contains("that spelling is ARGV-VISIBLE and is already");
    println!(
        "RECORDED (not asserted) [cred.rs:420-425's bound]\n  \
         the claim `that spelling is ARGV-VISIBLE and is already governed by \
         scan_leading's leading-option region and layer 2's whole grammar` is \
         {} in the production source at this run.\n  \
         ARGV-VISIBLE it is. GOVERNED it is not — measured below.",
        if still_present { "PRESENT" } else { "ABSENT (repaired)" }
    );

    // -- The two layers the claim names, MEASURED. Both do act — on OTHER keys.
    //    These two rows are STABLE in both directions and are asserted.
    refuses(
        "git -c core.hooksPath=/dev/null push --force origin main",
        "hook_bypass_blocked",
        "the BY-NAME deny covers `core.hooksPath`. **It is a real rule and it fires**, which is \
         what makes the claim's shape plausible and its content false.",
    );
    refuses(
        "git -c include.path=/tmp/evil.cfg status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 8's CONFINEMENT clause covers `include.path`. **Also a real rule that fires.**",
    );

    // -- And the key nothing covers, on four separate surfaces. **RECORDED and
    //    never asserted**: `19-31`'s credential clause is SEVERABLE, so a row
    //    asserted PERMITTED lands red if it lands and a row asserted REFUSED
    //    lands red if it is severed — and `19-31` may only ADD to this file.
    for (label, command) in [
        ("credential fill", "git -c credential.helper=store credential fill"),
        ("fetch origin", "git -c credential.helper=store fetch origin"),
        ("ls-remote origin", "git -c credential.helper=store ls-remote origin"),
        (
            "an in-namespace push",
            "git -c credential.helper=store push origin HEAD:refs/heads/gsd-auto/alpha/w",
        ),
    ] {
        let owned = command.to_string();
        record_only(
            &format!(
                "T-19-121 `-c credential.helper=store` on {label} — GOVERNED BY NEITHER LAYER. \
                 `19-31`'s clause is SEVERABLE, so this is asserted in NEITHER direction"
            ),
            move |_| owned.clone(),
        );
    }
}

// ===========================================================================
// SECTION 7 — `T-19-120`: THE LEDGER BOUND BOUNDS SIZE WHERE THE FAILURE IS
// KIND
//
// `MAX_LEDGER_BYTES` (`ledger.rs:114`) is checked at `:291-311` as
// `if let Ok(size) = std::fs::metadata(&path).map(|meta| meta.len())`. **A FIFO
// stats at length 0**, passes the bound, and the `std::fs::read` below it blocks
// forever.
//
// **MEASURED at this file's base:**
//
// ```text
// FIFO ledger        gh pr create --title x  -> exit 124 after 20.02 s
// 8 366 000-byte regular ledger (just under) -> exit 0 in 0.42 s, 89 001 lines (STILL COUNTING)
// FRESH root, no ledger at all               -> exit 0 in 0.01 s, exactly 1 ledger line
// ```
//
// **THE COMMENT BESIDE THE CHECK STATES THE FAIL-OPEN AND ITS REASON** — *"A
// file that cannot be stat'd is not a file this check can refuse on"* — and that
// reasoning is SOUND for a stat that FAILS. **The FIFO case is a stat that
// SUCCEEDS and answers 0.** So the correction belongs inside the SAME `Ok` arm
// and must not touch the `Err` arm: a fresh envelope root has no ledger at all,
// and every first forge call in the suite depends on that `Err` continuing to
// fall through.
//
// **THE BEHAVIOURAL HALF — what the agent CLI does with a `PreToolUse` hook that
// never returns — is UNMEASURED, is a property of a closed-source binary, and is
// claimed in NEITHER direction.** It is recorded in section 12.
// ===========================================================================

/// One ledger line, dated OUTSIDE the 24 h window so the CAP itself is
/// unaffected and the measurement is about SIZE and KIND alone.
const OLD_LEDGER_LINE: &str = "{\"at\":\"2020-01-01T00:00:00Z\",\"run_id\":\"old-run\",\
\"command\":\"gh pr create\",\"platform\":\"github\"}\n";

/// Drive the PRODUCT binary under a HARD timeout, because a blocking read
/// cannot be driven in-process — `ask` would hang the whole test binary.
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

#[test]
fn after_19_31_a_ledger_that_is_not_a_regular_file_is_refused_rather_than_read() {
    let envelope = TempDir::new().unwrap();
    let dir = envelope.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    let ledger = dir.join("pr-ledger.ndjson");

    let made = std::process::Command::new("mkfifo")
        .arg(&ledger)
        .status()
        .expect("mkfifo is on PATH");
    assert!(made.success(), "the fixture creates a FIFO at the ledger path");
    let size = std::fs::metadata(&ledger).unwrap().len();
    assert_eq!(
        size, 0,
        "**THE MECHANISM, PINNED BEFORE THE VERDICT.** A FIFO stats at length 0, which is why \
         it passes a bound written over SIZE. If it ever stats at something else, this finding \
         is not the finding this file says it is."
    );

    let (code, millis) = ask_binary_under_timeout(envelope.path(), "gh pr create --title x", 12);
    println!("  FIFO ledger: exit={code} after {millis} ms (hard timeout 12 s)");
    assert_eq!(
        code, 2,
        "\n\n**THE LEDGER BOUND MUST BOUND KIND AND NOT ONLY SIZE.**\n\n\
         Measured at this file's base: exit **124** after **20.02 s** under a 20-second hard \
         timeout, against the 0.42 s the same call answers in for an 8 366 000-byte REGULAR \
         ledger and against `GUARD_TIMEOUT_SECS = 5`. Observed here: exit {code} after \
         {millis} ms.\n\n\
         **THE DERIVATION.** `19-31` adds `metadata.file_type().is_file()` INSIDE THE SAME \
         `Ok` arm at `ledger.rs:291-311`, refusing anything that is not a regular file with \
         `ParkReason::EnvelopeAssertionFailed` — never `PrCapExceeded`, which names a cap that \
         FIRED (D-24). `std::fs::metadata` FOLLOWS symlinks, which is what keeps a \
         symlinked-to-regular ledger permitted and counted, and `symlink_metadata` must not be \
         substituted.\n\n\
         **THE CONTROLS AND WHAT MAKES THEM DISCRIMINATING** are in the two tests below: a \
         regular ledger just under the bound that still permits AND still counts, and a FRESH \
         root with no ledger at all. The pairs differ only in the ledger's KIND."
    );
}

#[test]
fn a_regular_ledger_just_under_the_bound_still_permits_and_still_counts_before_and_after() {
    // **CONTROL 1 — the half that keeps this a KIND bound rather than a disarmed
    // cap.** `MAX_LEDGER_BYTES` is 8 MiB; this ledger is a little over 8.36 MB
    // of lines dated outside the 24 h window, so the SIZE bound does not fire
    // and the tally still runs.
    let envelope = TempDir::new().unwrap();
    let dir = envelope.path().join(ALIAS);
    std::fs::create_dir_all(&dir).unwrap();
    let ledger = dir.join("pr-ledger.ndjson");
    {
        use std::io::Write as _;
        let mut out = std::io::BufWriter::new(std::fs::File::create(&ledger).unwrap());
        let chunk = OLD_LEDGER_LINE.repeat(1000);
        let mut written = 0usize;
        while written + chunk.len() < 8 * 1024 * 1024 - 8192 {
            out.write_all(chunk.as_bytes()).unwrap();
            written += chunk.len();
        }
        out.flush().unwrap();
    }
    let bytes = std::fs::metadata(&ledger).unwrap().len();
    let before = std::fs::read_to_string(&ledger).unwrap().lines().count();
    assert!(
        bytes < 8 * 1024 * 1024,
        "the control ledger must be UNDER the 8 MiB bound, got {bytes} bytes — otherwise this \
         row measures the SIZE refusal rather than the kind one"
    );

    let answer = ask(envelope.path(), "gh pr create --title x");
    let after = std::fs::read_to_string(&ledger).unwrap().lines().count();
    println!("  just-under control: {bytes} bytes, exit={}, {before} -> {after} lines", answer.code);
    assert_eq!(
        answer.code, 0,
        "**A LEDGER JUST UNDER THE BOUND MUST STILL PERMIT.** stdout: {}",
        answer.stdout
    );
    assert_eq!(
        after,
        before + 1,
        "**AND MUST STILL COUNT.** A kind check that refused a regular file, or a bound that \
         stopped appending, would turn this row red — which is what keeps `T-19-120`'s fix a \
         KIND bound rather than a disarmed cap."
    );
}

#[test]
fn a_fresh_envelope_root_with_no_ledger_at_all_still_permits_before_and_after() {
    // **CONTROL 2 — THE ONE THE WHOLE SUITE DEPENDS ON.** A fresh envelope root
    // has no ledger file, so `std::fs::metadata` returns `Err` and the check
    // falls through to `create_dir_all` and `append_entry`. **A kind check
    // placed OUTSIDE the `Ok` arm would refuse EVERY first forge call in EVERY
    // fresh root**, turning a large fraction of the suite red for a reason
    // unrelated to the finding.
    //
    // **Asserted exit 0 BEFORE AND AFTER. A red here after `19-31` means the
    // kind check left the `Ok` arm** — and the correct response is to move it
    // back, never to relax this row.
    let envelope = TempDir::new().unwrap();
    let ledger = envelope.path().join(ALIAS).join("pr-ledger.ndjson");
    assert!(
        std::fs::metadata(&ledger).is_err(),
        "**THE MECHANISM, PINNED BEFORE THE VERDICT.** There must be NO ledger file at all in a \
         fresh root, so the `Err` arm is the one this row exercises."
    );

    let answer = ask(envelope.path(), "gh pr create --title x");
    let written = ledger_lines_anywhere(envelope.path());
    println!(
        "  fresh-root control: exit={} ledger lines={}",
        answer.code,
        written.len()
    );
    assert_eq!(
        answer.code, 0,
        "**A FRESH ROOT WITH NO LEDGER MUST PERMIT.** If this is red after `19-31`, the kind \
         check was placed outside the `Ok` arm of `std::fs::metadata` at `ledger.rs:291-311`. \
         stdout: {}",
        answer.stdout
    );
    assert_eq!(
        written.len(),
        1,
        "and must leave exactly one ledger line. Walked listing:\n{}",
        listing(envelope.path())
    );
}

#[test]
fn the_ledger_kind_reachability_table_is_measured_with_its_four_silences_beside_it() {
    // **THE ABSOLUTE-LITERAL TWIN IS REFUSED AND FOUR OF THE DECLARED SILENCES
    // ARE NOT** — so `T-19-120` is reachable through exactly the spellings rule
    // (a) cannot see, which is the same residue this whole round is about.
    refuses_carrier(
        |root| format!("mkfifo {}/alpha/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "the ABSOLUTE LITERAL spelling of the FIFO creation IS reached by round 10's clause \
         today — measured, and GREEN before and after. **It is the control that makes the four \
         rows below meaningful.**",
    );
    permits(
        "mkfifo ~/.local/share/gsd-meta-manager/envelope/alpha/pr-ledger.ndjson",
        "direction (v), the TILDE.",
    );
    permits_carrier(
        |root| format!("mkfifo {}/alpha/pr-ledger.ndjso?", root.display()),
        "direction (vi), the GLOB.",
    );
    permits_carrier(
        |root| format!("mkfifo {}/alpha/{{pr-ledger.ndjson,x}}", root.display()),
        "direction (vii), the BRACE LIST.",
    );
    permits(
        "D=$(git config --get core.hooksPath); mkfifo $D/../pr-ledger.ndjson",
        "direction (ii), EXPANSION-BORNE.",
    );
    // -- And `T-19-119`'s own spelling, which is this round's finding one path
    //    over: an option-attached FIFO path. `mkfifo` takes no such option, so
    //    the row is RECORDED in section 12 rather than asserted.
}

// ===========================================================================
// SECTION 8 — `policy.rs:5743`'s OWN EXAMPLE, MEASURED FALSE
//
// The disclosed-cost paragraph argues that the read over-refusal is unavoidable
// because the guard cannot tell a read from a write without knowing every
// program's grammar — *"is `dd if=X of=Y` a read of `X` or a write of `Y`?"*.
//
// **`dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` is exit 0.** The
// argument is made with a spelling the guard refuses NEITHER way, so the example
// illustrated nothing at the moment it was written. **That is a false claim the
// code makes about this exact spelling**, and it is registered as `19-31`'s to
// correct **whether or not any rule lands**:
//
// * if the fix lands, the sentence becomes TRUE and needs the WR-02 note saying
//   it was false when written;
// * if the fix is severed, it must be replaced by an example the guard actually
//   refuses — `tee F` is the one already beside it.
// ===========================================================================

#[test]
fn after_19_31_policy_rs_5743s_own_dd_example_is_refused_in_the_direction_it_names() {
    assert!(
        POLICY_SOURCE.contains("fn protected_carrier_named"),
        "the `policy.rs` include must reach the real file"
    );
    println!(
        "RECORDED (not asserted) [policy.rs:5743's example]\n  \
         the sentence `is `dd if=X of=Y` a read of `X` or a write of `Y`?` is {} in the \
         production source at this run. It was measured FALSE at 19-30: \
         `dd if=<ledger> of=/tmp/stolen` was exit 0.",
        if POLICY_SOURCE.contains("a read of `X` or a write of `Y`") {
            "PRESENT"
        } else {
            "ABSENT (repaired)"
        }
    );

    refuses_carrier(
        |root| {
            format!(
                "dd if={}/alpha/pr-ledger.ndjson of=/tmp/stolen",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**THE DOC'S OWN EXAMPLE, IN THE DIRECTION IT NAMES.** `policy.rs:5743` cites \
             `dd if=X of=Y` as the reason a read cannot be told from a write — and at `19-30` \
             this exact spelling was **exit 0**, so the guard refused it neither way.\n\n\
             **THE CONTROL:** `dd if=/tmp/g/x of=/tmp/stolen` is exit 0 before AND after \
             (section 4). The pair differs only in whether the interior path is the protected \
             one.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
}

// ===========================================================================
// SECTION 9 — THE TWO ORDERING PINS, AT DELIBERATELY DIFFERENT IDENTIFIERS
//
// **These are the mechanical proof `19-31`'s widened candidate set is still
// raised in the ONE per-segment walk rather than in a second pass.** Round 3's
// principle is DISCHARGED rather than weakened: the candidate scan is a scan of
// ONE WORD inside the predicate that already reads that word.
// ===========================================================================

#[test]
fn after_19_31_ordering_pin_a_is_the_three_way_git_dir_pin_with_the_one_character_restored() {
    // **THIS IS `19-28`'s PIN A WITH THE ONE CHARACTER RESTORED, AND THE
    // THREE-WAY SHAPE IS WHAT MAKES IT DISCRIMINATING.** It separates the rule
    // firing from the classifier firing AND from the join being irrelevant:
    //
    // ```text
    // git --git-dir=<ENV>/alpha push --force origin main  measured force_push_blocked      -> ASSERTED envelope_assertion_failed  (RED)
    // git --git-dir <ENV>/alpha push --force origin main  ALREADY envelope_assertion_failed -> ASSERTED unchanged                  (GREEN)
    // git --git-dir=/tmp/other  push --force origin main  measured force_push_blocked      -> ASSERTED unchanged                  (GREEN)
    // ```
    //
    // A two-way pin would pass if the classifier stopped firing; a two-way pin
    // the other way would pass if the join were ignored entirely. **All three
    // orderings were measured before any of them was written.**
    refuses_carrier(
        |root| {
            format!(
                "git --git-dir={}/alpha push --force origin main",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "**THE `=`-JOINED SPELLING — `19-28`'s OWN OBSERVATION, CLASSIFIED CORRECTLY THIS \
             TIME.** It is `force_push_blocked` today because the token does not begin with \
             `/`; after `19-31` the carrier clause is raised BEFORE the resolution match and \
             answers first.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| {
            format!(
                "git --git-dir {}/alpha push --force origin main",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**LEG 2 — THE SPACE-SEPARATED TWIN, ALREADY GREEN.** It is what makes the pair a \
         measurement of ONE CHARACTER rather than of the push, and it must not move.",
    );
    refuses(
        "git --git-dir=/tmp/other push --force origin main",
        "force_push_blocked",
        "**LEG 3 — THE DISCRIMINATING CONTROL.** The same `=`-joined shape with a value that is \
         NOT under the envelope root stays at the CLASSIFIER's identifier. A control refused at \
         the SAME identifier for the SAME reason would prove nothing — that is `19-27`'s \
         measured failure mode.",
    );
}

#[test]
fn after_19_31_ordering_pin_b_across_segments_the_first_refusal_still_wins() {
    // **PIN B — ACROSS segments, ORDER DOES matter**, and after `19-31` the two
    // orders land at TWO DIFFERENT identifiers. That difference IS the
    // assertion.
    //
    // ```text
    // dd if=/bin/true of=<ENV>/alpha/x && git push --force origin main
    //     measured force_push_blocked  -> ASSERTED envelope_assertion_failed  (RED)
    // git push --force origin main && dd if=/bin/true of=<ENV>/alpha/x
    //     measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
    // dd if=/bin/true of=/tmp/other/x && git push --force origin main
    //     measured force_push_blocked  -> ASSERTED unchanged                  (GREEN)
    // ```
    //
    // **The pair does NOT discriminate before the fix and DOES after it**, which
    // is exactly what a derived-post-fix row is; the discrimination is stated
    // rather than assumed.
    refuses_carrier(
        |root| {
            format!(
                "dd if=/bin/true of={}/alpha/x && git push --force origin main",
                root.display()
            )
        },
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        &format!(
            "the carrier is in SEGMENT ONE, so after `19-31` `classify_segments` returns ITS \
             refusal and never reaches the push.\n\n{INTERIOR_PATH_DERIVATION}"
        ),
    );
    refuses_carrier(
        |root| {
            format!(
                "git push --force origin main && dd if=/bin/true of={}/alpha/x",
                root.display()
            )
        },
        "force_push_blocked",
        "**THE ORDER CONTROL.** The same two commands the other way round: segment ONE is the \
         push, so the push's identifier wins and stays. **If this row ever answered \
         `envelope_assertion_failed`, the clause would be running in a SECOND PASS over the \
         segments rather than in the one per-segment walk** — which is the finding to report, \
         not a row to relax.",
    );
    refuses(
        "dd if=/bin/true of=/tmp/other/x && git push --force origin main",
        "force_push_blocked",
        "**THE PATH-SET CONTROL.** Segment one carries an interior path that is NOT protected, \
         so the push's identifier wins in both trees.",
    );
}

// ===========================================================================
// SECTION 10 — CONTAINMENT AND THE FENCED-FILE ENUMERATION, RE-DERIVED
// MECHANICALLY
//
// **The planning-time answer was ZERO assertions move. It is RE-DERIVED here
// rather than inherited**, because seven times a rule has turned a row red in a
// file the plan may not edit.
//
// **THE THREE GREPS, RUN AT THIS FILE'S BASE, WITH EVERY HIT DISPOSITIONED:**
//
// ```text
// grep -rnE '[^ "]<ENV>|[^ "]<BIN>|[^ "]<ENVX>|[^ "]<BINPAR>' tests/
//   5 hits, ALL of them `//` or `///` DOC lines
//   (envelope_wrapper_class.rs:7976, :8556, :8558; envelope_carrier_reach.rs:1154;
//    envelope_control_carrier.rs:753)
//
// grep -rnE '[^ "(]\{\}/|[^ "(]\{root|[^ "(]\{binary' tests/
//   8 hits: five `refs/heads/gsd-auto/{}/…` refspecs (candidates
//   [heads, gsd-auto, <alias>, …] — WRONG FIRST COMPONENT), one `/proc/{}/stat`,
//   and envelope_config_resolution.rs:1825/:1832's
//   `includeIf.gitdir:{repo}/.path={evil}` where `{repo}` is a TEMP GIT REPO and
//   the row is a REAL-GIT probe, not a guard row.
//
// grep -rnoE '=/[^ ")]*' tests/ src/envelope/
//   Every `=`-attached absolute path in the whole corpus is one of
//   /tmp/evil.cfg, /dev/null, /tmp/nohooks, /tmp/g, /tmp/e, /tmp/x, /tmp/s,
//   /tmp/fresh, /tmp/nowhere, /tmp/evil, /x, /ENV_WINS, /CLI_WINS, /PARAM_WINS,
//   /ALIAS_WINS, /INCLUDE_WINS, /PARAM_INCLUDE_WINS, /SHOULD_NOT_WIN,
//   /nonexistent-hooks-dir.
//   **NOT ONE names an envelope root and NOT ONE equals a binary.**
// ```
//
// **ONE ARTEFACT MOVES AND IT IS A COMMENT, NOT AN ASSERTION.**
// `policy.rs:9719`'s reason for the `./alpha/pr-ledger.ndjson` row — *"relative
// again, with the leading `./` the normaliser strips: stripping it must not turn
// a relative word into an absolute one"* — goes STALE, because under the new
// rule a relative word DOES yield absolute candidates while that row still
// answers `false` for a different reason (two components against three).
// **That is `19-31`'s WR-02 correction and it is named here in advance; it is
// not edited by `19-30`.**
// ===========================================================================

#[test]
fn containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte() {
    // **THE CLAIM, STATED PRECISELY RATHER THAN APPROXIMATELY.** It is NOT
    // *"`i == 0` is today's rule"*: `lexical_absolute_components` strips leading
    // `./`s BEFORE it tests `starts_with('/')`, so `.//abs/p` is accepted today
    // at index 0 and `./abs` is not. The exact statement is that the string
    // today's rule NORMALISES is itself one of the candidates and re-normalises
    // to the same component list.
    for word in [
        "/tmp/envroot/alpha/pr-ledger.ndjson",
        "/tmp/envroot/alpha/hooks/../pr-ledger.ndjson",
        ".//tmp/envroot/alpha/pr-ledger.ndjson",
        "././/tmp/envroot/alpha",
        "//tmp//envroot//alpha//x",
        "/",
        "/tmp/envroot",
    ] {
        let Some(today) = components(word) else {
            continue;
        };
        let reached = slash_anchored_candidates(word)
            .into_iter()
            .filter_map(components)
            .any(|candidate| candidate == today);
        assert!(
            reached,
            "\n\n**CONTAINMENT FAILED FOR `{word}`.**\n\
             Today's rule normalises it to {today:?}, and no `/`-anchored candidate \
             re-normalises to that. **If this ever fails, a refusal that exists today can be \
             LOST by the widened rule** — which is the one thing `19-31`'s design may not do, \
             and the correct response is to report it as a finding about the DESIGN rather than \
             to relax this row."
        );
    }

    // -- The converse half, so the claim is not vacuous: the strip can only ADD.
    assert!(
        components("./abs").is_none(),
        "the positive control for the ADD direction: `./abs` is `None` today"
    );
    assert!(
        slash_anchored_candidates("./abs")
            .into_iter()
            .filter_map(components)
            .any(|parts| parts == vec!["abs"]),
        "and it yields `[abs]` as a candidate under the widened rule. **The `./` strip can only \
         ADD answers, never remove one.**"
    );
}

#[test]
fn every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule() {
    // **`policy.rs`'s OWN UNIT PINS, RE-DERIVED ONE AT A TIME.** The two that
    // look most exposed are checked first and by name.
    let dir = "/tmp/envroot/alpha";
    let binary = "/tmp/gsd-binary-dir/gsd-meta-manager";
    for (word, why) in [
        (
            "/tmp/envroot/alpha/../other/x",
            "`..` is collapsed PER CANDIDATE, so a walk OUT of the envelope directory stays out \
             — the row `policy.rs:9744` pins and the one that looks most exposed",
        ),
        (
            "./alpha/pr-ledger.ndjson",
            "its only candidate is `/alpha/pr-ledger.ndjson`, TWO components against a \
             three-component directory — **and `policy.rs:9719`'s stated REASON for this row \
             goes stale even though the row does not, which is `19-31`'s WR-02 correction**",
        ),
        ("/tmp/envroot/alphax", "a SIBLING whose name merely EXTENDS the alias"),
        (
            "/tmp/envroot/alpha2/x",
            "the raw-string-prefix trap, one component deeper",
        ),
        ("/tmp/envroot", "the PARENT of the envelope directory is not under it"),
        ("pr-ledger.ndjson", "a RELATIVE operand with no `/` at all"),
        ("/tmp/pr-ledger.ndjson", "the BASENAME near miss"),
        (
            "/tmp/gsd-binary-dir/some-other-file",
            "a SIBLING of the binary in the same shared directory — the EXACT-PATH control",
        ),
        ("/tmp/gsd-binary-dir", "the binary's PARENT directory"),
    ] {
        assert!(
            !simulated_protected(word, Some(dir), Some(binary)),
            "\n\n**A FENCED PIN MOVED: `{word}` must still answer `false`.**\n  {why}\n\n\
             This is the enumeration `19-30` re-derived mechanically rather than inheriting. A \
             row that moves here is a finding about the DESIGN, and the correct response is to \
             report it — never to edit the pin in a file `19-31` may not touch."
        );
    }

    // -- The POSITIVE controls, so the absences above are not vacuous.
    for (word, why) in [
        ("/tmp/envroot/alpha", "the directory itself"),
        ("/tmp/envroot/alpha/pr-ledger.ndjson", "a file under it"),
        (
            "/tmp/envroot/alpha/hooks/../pr-ledger.ndjson",
            "a `..` walk that lands back INSIDE",
        ),
        ("/tmp/gsd-binary-dir/gsd-meta-manager", "the binary itself"),
        (
            "of=/tmp/envroot/alpha/pr-ledger.ndjson",
            "**AND THE ROUND'S OWN CLASS**: an interior path behind an `=`",
        ),
        (
            "-C/tmp/envroot/alpha",
            "an interior path behind an attached short option with NO `=`",
        ),
        (
            "PATH=/usr/bin:/tmp/envroot/alpha",
            "an interior path behind BOTH an `=` and a `:`",
        ),
        (
            "--opt=a=/tmp/envroot/alpha/x",
            "an interior path behind TWO `=`",
        ),
    ] {
        assert!(
            simulated_protected(word, Some(dir), Some(binary)),
            "the positive control `{word}` must answer `true`: {why}. Without these, every \
             absence above passes because the simulation answers `false` for everything."
        );
    }

    // -- **EVERY `=`-ATTACHED ABSOLUTE PATH THE WHOLE CORPUS PINS**, taken from
    //    the third grep and checked one at a time against BOTH protected paths.
    for pinned in [
        "-c include.path=/tmp/evil.cfg",
        "-c core.hooksPath=/dev/null",
        "-c aliasx.q=-c include.path=/tmp/evil.cfg status",
        "--git-dir=/tmp/g",
        "SSH_AUTH_SOCK=/tmp/evil",
        "GSD_MM_ENVELOPE_ROOT=/tmp/fresh",
        "core.hooksPath=/tmp/nohooks",
        "core.hooksPath=/ENV_WINS",
        "core.hooksPath=/CLI_WINS",
        "core.hooksPath=/PARAM_WINS",
        "core.hooksPath=/ALIAS_WINS",
        "core.hooksPath=/INCLUDE_WINS",
        "core.hooksPath=/SHOULD_NOT_WIN",
        "core.hooksPath=/nonexistent-hooks-dir",
        "includeIf.gitdir:/tmp/some-repo/.path=/tmp/evil.cfg",
        "HEAD:refs/heads/gsd-auto/alpha/w",
        "https://github.com/o/r",
        "s/x/y/",
        "src/",
        "--format=%H",
    ] {
        assert!(
            !simulated_protected(pinned, Some(dir), Some(binary)),
            "\n\n**A FENCED CORPUS ROW WOULD MOVE: `{pinned}`.**\n\n\
             This is the mechanical form of the fenced-file enumeration. `--git-dir=/tmp/g` \
             alone is pinned PERMITTED in FOUR files and named in FIVE `policy.rs` doc sites, \
             so a rule that turned it red would turn five files red at once."
        );
    }
}

// ===========================================================================
// SECTION 11 — THE CARRIED-FORWARD MECHANISM PINS, ROUNDS 4 THROUGH 11
//
// RE-ASSERTED here over the same public functions rather than moved or edited
// in place. **All green today and all green after `19-31`.** `Token.literal` is
// READ IN THREE PLACES and **this round adds no fourth reader of it** — it adds
// CANDIDATES to the words the third reader already receives.
// ===========================================================================

#[test]
fn round_5s_literalness_bit_is_non_vacuous_and_this_round_must_not_remove_it() {
    refuses(
        "git pus? --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 5: a decision word carrying a pathname-expansion metacharacter is NOT \
         `Token.literal`, so the guard cannot know which subcommand runs and fails CLOSED. \
         **The literal filter stays and is applied BEFORE any candidate is produced** — \
         otherwise the tilde, glob and brace rows this phase pinned PERMITTED in both word \
         positions would turn red, which is a rule that quietly widened.",
    );
    let segments = policy::split_segments_with_heads("git pus? --force origin main")
        .expect("the command splits into segments");
    assert!(
        segments[0].tokens.iter().any(|token| !token.literal),
        "**AND THE BIT ITSELF IS NON-VACUOUS**, read from the REAL tokenizer rather than from a \
         fixture. A `Token.literal` that were `true` for everything would make three rules \
         vacuous at once."
    );
}

#[test]
fn a_redirection_operator_is_not_a_separator_and_this_round_must_not_make_it_one() {
    // `SEPARATORS` (`policy.rs:2297`) has ONE commit in the whole phase
    // (`84a9b05`, plan 19-05) and `is_separator` is a bare `SEPARATORS.contains`,
    // so this is true by construction.
    assert!(
        !policy::is_separator(">"),
        "`>` must NOT be a separator. This round adds no token to the stream at all."
    );
    assert!(
        !policy::is_separator("<"),
        "`<` must NOT be a separator, for the same reason."
    );
    assert!(
        !policy::is_separator("="),
        "**AND `=` MUST NOT BE ONE EITHER.** This round's boundary is `/`-anchored substrings \
         of a word, computed INSIDE the carrier predicate — never a change to how the line is \
         split into words."
    );
    assert!(
        policy::is_separator("&&") && policy::is_separator(";") && policy::is_separator("|"),
        "the positive control: the real command separators must still BE separators, or the \
         assertions above pass because `is_separator` answers `false` for everything."
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
             `classify_git` answers `Allow` for. **Round 6's headline refusal has been silently \
             converted into a permit.**\n\n\
             **(b) THE DISPLACED VARIANT.** Something entered as an ORDINARY word. The count is \
             still ONE and the EXACT-TOKEN assertion below fires instead: the argv the \
             classifier reads is no longer the argv the program receives, and every decision \
             index is shifted by one. **That is `T-19-98`'s registered shape**, which is why \
             this pin asserts the exact token list as well as the count.\n\n\
             **THIS ROUND MUST TOUCH NEITHER.** `19-31` changes what a LITERAL word is READ FOR, \
             inside `protected_carrier_named`. It changes nothing about `tokenize`, `Token`, \
             `Segment`, `segment.tokens`, `SEPARATORS` or `is_separator`."
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
             This is variant (b), THE DISPLACED VARIANT. `T-19-98` is the registered shape."
        );
    }
}

#[test]
fn round_6s_deletion_model_and_its_over_deletion_control_are_unchanged() {
    refuses(
        "git >/dev/null push --force origin main",
        "force_push_blocked",
        "round 6's headline: the redirection is DELETED and the surviving argv is still a force \
         push.",
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
        "a redirection with NO target fails CLOSED rather than guessing.",
    );
    refuses(
        "git <<EOF push --force origin main",
        "force_push_blocked",
        "the HEREDOC row: `<<` takes a DELIMITER rather than a pathname, and it must never be \
         recorded as a carrier candidate.",
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
    permits(
        "git -c a=b status",
        "round 8: an ordinary `-c` value is untouched.",
    );
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
            "round 10's clause over the envelope-root OPERAND rows. **`19-31` widens what a \
             LITERAL word is READ FOR; it must not narrow what the clause already reaches** — \
             and CONTAINMENT is what guarantees it cannot.",
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
        |root| format!(": > {}/alpha/pr-ledger.ndjson", root.display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 11's REDIRECTION-TARGET clause: an absolute literal pathname target under the \
         envelope root is refused.",
    );
    refuses(
        &format!("cp /bin/true {}", this_binary().display()),
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "round 11's EXACT-PATH binary clause. **This is the twin of section 1's headline row**, \
         and the two differ in exactly one character.",
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
fn the_carrier_rule_still_reads_a_path_and_never_a_program_name_and_never_an_attachment_character()
{
    // **THE NO-PROGRAM-NAMES FENCE, CARRIED FORWARD AND EXTENDED TO THIS ROUND'S
    // OWN CLASS.** D-08's whole argument is that `resolve_program` never asks
    // what the wrapper is CALLED.
    assert!(
        POLICY_SOURCE.contains("fn resolve_program"),
        "the `policy.rs` include must reach the real file"
    );
    assert!(
        HOOKS_SOURCE.contains("fn classify_segments("),
        "the `hooks.rs` include must reach the real file"
    );
    assert!(
        LEDGER_SOURCE.contains("MAX_LEDGER_BYTES"),
        "the `ledger.rs` include must reach the real file"
    );
    for program in [
        "dd", "shred", "mv", "unlink", "perl", "python3", "rsync", "mkfifo", "tar", "cp", "chmod",
    ] {
        assert!(
            !POLICY_SOURCE.contains(&format!("\"{program}\""))
                && !HOOKS_SOURCE.contains(&format!("\"{program}\"")),
            "\n\n**`{program}` HAS APPEARED AS A STRING LITERAL IN THE GUARD'S PRODUCTION \
             LOGIC.**\n\n\
             This round draws rows under `dd`, `tar`, `cp`, `chmod`, `rsync`, `mkfifo` and a \
             `mytool` that does not exist — all program names the guard must NEVER have to \
             know. **The correct response is to DELETE the program name from `src/`, never to \
             delete this assertion.**"
        );
    }
    // **`"-C"` AND `"-t"` ARE DELIBERATELY ABSENT FROM THIS LIST AND THAT IS
    // MEASURED, NOT AN OVERSIGHT.** `policy.rs` already carries `"-C"` four
    // times and `"-t"` twice — as GIT'S OWN leading options, which round 7's
    // callee-grammar rule must know by name in order to consume their values.
    // Knowing git's grammar in the GIT classifier is not the same thing as
    // knowing `tar`'s grammar in the CARRIER predicate, and a fence that
    // conflated them would fire on a rule that is correct.
    for spelling in [
        "\"of=\"",
        "\"if=\"",
        "\"--git-dir=\"",
        "\"--target-directory=\"",
        "\"--directory=\"",
        "\"--reference=\"",
        "\"--temp-dir=\"",
    ] {
        assert!(
            !POLICY_SOURCE.contains(spelling),
            "\n\n**{spelling} HAS APPEARED AS A STRING LITERAL IN `policy.rs`.**\n\n\
             **THE MANDATED BOUNDARY IS `/`-ANCHORED SUBSTRINGS OF A LITERAL WORD, WHICH ASKS \
             NOTHING ABOUT WHAT PRECEDES THE `/`.** A rule keyed to a list of option spellings \
             — or to a list of attachment characters — is a PROGRAM-GRAMMAR ENUMERATION and is \
             D-08's defect one level over: it would be one character short in exactly the way \
             the current rule is. **Delete the enumeration, never this assertion.**"
        );
    }
}

// ===========================================================================
// SECTION 12 — THE ROWS THAT ARE RECORDED AND NEVER ASSERTED
//
// `19-31` writes **no rule** for `C-11` … `C-15`, for `C-08`'s behavioural
// half, for `T-19-120`'s behavioural half, for `T-19-116`'s residues or for the
// derived over-refusal — and its `T-19-121` clause is SEVERABLE. A row asserted
// REFUSED lands permanently red in a file `19-31` may only ADD to; a row
// asserted PERMITTED pins a live bypass as correct. **`19-22` asserted such a
// row against its own comment, its own SUMMARY and its own plan-check, and it
// halted `19-23` mid-plan.**
// ===========================================================================

#[test]
fn the_unruled_rows_are_recorded_and_never_asserted() {
    // -- `T-19-116`'s THREE RESIDUES, re-measured. **Closing `T-19-119` does
    //    NOT close `T-19-116`**: none of these is an option attachment, and all
    //    are the OTHER silences.
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

    // -- `C-11` … `C-15`, all at control (e), no rule, no `pr_cap_*` clamp.
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
        |root| format!("printf '{{}}' > {}/alpha/settings.json", root.display()),
    );

    // -- `T-19-121`'s key shapes at the GUARD, and the two spellings that are
    //    already refused for OTHER reasons — recorded with their overlap stated.
    record_only(
        "T-19-121 the URL-SCOPED key shape — real git RESOLVES it (secret PRESENT), so a \
         by-name equality on `credential.helper` would MISS it",
        |_| "git -c credential.https://github.com.helper=store credential fill".to_string(),
    );
    record_only(
        "T-19-121 the `--config-env` spelling — real git RESOLVES it (secret PRESENT)",
        |_| "git --config-env=credential.helper=EVILVAR credential fill".to_string(),
    );
    record_only(
        "T-19-104's carrier `GIT_CONFIG_PARAMETERS`. **MEASURED DIFFERENTLY FROM THE PLAN'S \
         EXPECTATION: it is exit 2 `hook_bypass_blocked` at the guard, not exit 0** — the key \
         is in `ENVELOPE_ENV_KEYS`. Real git DOES resolve the helper from it, so the reach is \
         real and the guard already refuses the carrier. **RECORDED with its overlap stated: \
         its control fails the same way, so it discriminates nothing on THIS axis**",
        |_| "GIT_CONFIG_PARAMETERS=\"'credential.helper=store'\" git fetch origin".to_string(),
    );

    // -- The two genuinely-REACHING two-`=` spellings, both already refused
    //    today by OTHER clauses. **Their controls fail the same way at the same
    //    identifier, so neither can be asserted on this axis.**
    record_only(
        "a TWO-`=` word that real git RESOLVES: `GIT_CONFIG_PARAMETERS='core.excludesFile=<ENV>/alpha/excl'`. \
         Already exit 2 `hook_bypass_blocked` — and so is its UNPROTECTED control \
         `…=/tmp/g/excl`, at the SAME identifier. **OVERLAP STATED; asserted in neither \
         direction**",
        |root| {
            format!(
                "GIT_CONFIG_PARAMETERS='core.excludesFile={}/alpha/excl' git status",
                root.display()
            )
        },
    );
    record_only(
        "a TWO-`=` word inside an alias body: `git -c alias.q=--git-dir=<ENV>/alpha q`. Already \
         exit 2 `envelope_assertion_failed` from ROUND 9's re-parse clause — the SAME \
         identifier this round's rule would use, so the pair does not discriminate. **And real \
         git refuses to run it at all (`alias 'q' changes environment variables`), so no reach \
         is claimed either**",
        |root| format!("git -c alias.q=--git-dir={}/alpha q", root.display()),
    );

    // -- The attachment spellings whose REACH was NOT demonstrated under real
    //    `bash`. **A spelling the guard permits but the shell does not reach is
    //    not a bypass**, and this is the leg that tells them apart.
    record_only(
        "`python3 --out=<ENV>/alpha/x` — audit 11 listed it; MEASURED, python3 answers `unknown \
         option --out=…` and reaches NOTHING. RECORDED rather than asserted",
        |root| format!("python3 --out={}/alpha/x", root.display()),
    );
    record_only(
        "`mytool --opt=a=<ENV>/alpha/pr-ledger.ndjson` — a TWO-`=` word under a program name \
         absent from both production halves. **No program grammar was found that consumes a \
         two-`=` word as a path**, so the reach is not claimed and the row is RECORDED. It is \
         drawn in the AXIS's interior-path alphabet, where it fences the rule SHAPE: a \
         first-`=`-split rule misses it",
        |root| {
            format!(
                "mytool --opt=a={}/alpha/pr-ledger.ndjson",
                root.display()
            )
        },
    );
    record_only(
        "`ssh -oControlPath=<ENV>/alpha/s host` — an attached short option with an `=` inside. \
         ssh accepts the option but the connection never opened, so no reach is claimed",
        |root| format!("ssh -oControlPath={}/alpha/s host", root.display()),
    );
    record_only(
        "`rsync host:<ENV>/alpha /tmp/x` — the `:` attachment in its rsync spelling. No remote \
         to reach, so RECORDED. **The `:` attachment IS asserted in section 1 in its \
         `PATH=/usr/bin:<ENV>/alpha` spelling, where bash genuinely execs out of the \
         directory**",
        |root| format!("rsync host:{}/alpha /tmp/x", root.display()),
    );
    record_only(
        "`glab` is CONFIRMED NOT INSTALLED on this machine, so any `glab` row is UNMEASURABLE \
         AGAINST ITS CALLEE. **`--hostname` stays in `FORGE_VALUE_OPTS`** — audit 9 overturned \
         audit 8's removal suggestion because removal moves a counted creation form to \
         UNCOUNTED (`T-19-35`). A pin that would SKIP is a fail-open pin and is not written",
        |_| "glab mr create --title x".to_string(),
    );

    println!(
        "\nRECORDED (not asserted) [T-19-120's BEHAVIOURAL half]\n  \
         What the agent CLI does with a `PreToolUse` hook that NEVER RETURNS is a property of a \
         CLOSED-SOURCE BINARY. It is UNMEASURED and is claimed in NEITHER direction — exactly \
         as `C-08`'s behavioural half is.\n"
    );
}

#[test]
fn no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically() {
    // **THE MECHANICAL SELF-ASSERTION.** A comment saying "these are recorded"
    // is a comment; this reads this file's own text and proves it.
    //
    // The positive control comes first, for the reason every absence assertion
    // in this phase carries one: an absence assertion cannot tell "the string is
    // not in this file" from "this is not the file I think it is".
    assert!(
        THIS_FILE.contains("fn no_repo_side_or_unruled_row_is_asserted_and_this_file_says_so_mechanically"),
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
        "credential.https://github.com.helper",
        "--config-env=credential.helper",
        "GIT_CONFIG_PARAMETERS",
        "python3 --out=",
        "ssh -oControlPath=",
        "rsync host:",
        "glab ",
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
                 `19-31` writes NO rule for `C-11` … `C-15`, for `C-08`'s behavioural half, for \
                 `T-19-116`'s residues, for the spellings whose reach was not demonstrated, or \
                 for the two-`=` spellings whose controls fail the same way — and its \
                 `T-19-121` clause is SEVERABLE. A row asserted REFUSED lands permanently red \
                 in a file `19-31` may only ADD to; a row asserted PERMITTED pins a live bypass \
                 as correct. **Use `record_only`.**",
                index + 1
            );
        }
    }

    // -- And the DERIVED OVER-REFUSAL must appear only under `record_only`,
    //    because a cost to DISCLOSE is not a control to CERTIFY.
    for (index, line) in THIS_FILE.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("///") {
            continue;
        }
        if !line.contains("relative_spelling_of(root)") {
            continue;
        }
        assert!(
            !line.contains("refuses_carrier(") && !line.contains("permits_carrier("),
            "\n\n**A DERIVED OVER-REFUSAL INSTANCE HAS BEEN ASSERTED.**\n\
             \n  line {}: {line}\n\n\
             The over-refusal is a COST to disclose at both verdicts, not a control to certify \
             in either direction. Use `record_only`.",
            index + 1
        );
    }
}

// ===========================================================================
// SECTION 13 — `SECTION_ENVELOPE` RE-MEASURED AND NOT EDITED
//
// **Headroom that is spent cannot be got back**, so this round measures and
// changes nothing. `19-31` is prohibited from opening `advisory.rs` too.
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
        "**`SECTION_ENVELOPE` MOVED.** `19-30` re-measured it at 211 tokens of an UNRAISED 215 \
         cap, exactly as audit 11 did. This round edits nothing in `advisory.rs`, and neither \
         does `19-31`."
    );
    assert_eq!(
        widest, 74,
        "**`SECTION_ENVELOPE`'s WIDEST LINE MOVED.** Measured 74 of an 80 cap."
    );

    // -- **THE FIRST `Guaranteed` CLAUSE IS STILL TRUE, and `T-19-121` does NOT
    //    falsify it.** *"As started"* states what the envelope ESTABLISHES and
    //    stops; the clause makes no claim about what a later command line can do,
    //    which is layer 2's business and is disclosed in the `Not guaranteed`
    //    half.
    assert!(
        advisory::SECTION_ENVELOPE
            .contains("As started, this run cannot reach your ambient git credentials"),
        "the first `Guaranteed` clause must still open with `As started`, which is the whole of \
         `19-29`'s repair and the reason `T-19-121` does not falsify it"
    );
    assert!(
        advisory::SECTION_ENVELOPE.contains("runs no credential helper"),
        "`runs no credential helper` is a MEASURED phrase rather than a stylistic one: with \
         `19-29`'s empty pair injected, `git config --get-all credential.helper` still LISTS \
         the user's helper while `git credential fill` fails closed."
    );
    assert!(
        advisory::SECTION_ENVELOPE
            .contains("the files and the binary this envelope runs on"),
        "the `Not guaranteed` half's generalisation already covers this round's route — a \
         command that rewrites the files or the binary this envelope runs on is past the last \
         layer. **No new disclosure is owed here and none is written.**"
    );
}


// ===========================================================================
// SECTION 13 — `T-19-121`: THE `credential.helper` CLAUSE, MEASURED BEFORE IT
// WAS WRITTEN
//
// **These rows were driven as `record_only` prints against the built clause and
// converted to assertions only after every verdict was read.** `19-30` measured
// the key-shape space against REAL GIT with the no-`-c` control driven FIRST
// under the envelope's full posture, including `19-29`'s injected empty-helper
// pair:
//
// ```text
//                                                    real git             guard (19-30)
// CONTROL  no `-c` at all                            exit 128, ABSENT     —
// -c credential.helper=store                         secret PRESENT       exit 0
// -c CREDENTIAL.HELPER=store                         secret PRESENT       exit 0
// -c Credential.Helper=store                         secret PRESENT       exit 0
// -c credential.https://github.com.helper=store      secret PRESENT       exit 0   <- URL-SCOPED
// --config-env=credential.helper=<VAR>               secret PRESENT       exit 0
// -c credentialx.helper=store                        secret ABSENT        exit 0
// -c notcredential.helper=store                      secret ABSENT        exit 0
// -c credential.helperx=store                        secret ABSENT        exit 0
// GIT_CONFIG_PARAMETERS="'credential.helper=store'"  secret PRESENT       exit 2 hook_bypass_blocked
// ```
//
// **THE CLAUSE'S SHAPE IS DERIVED FROM THAT TABLE RATHER THAN CHOSEN.** Git
// folds a config key's SECTION and its FINAL NAME case-insensitively and leaves
// any SUBSECTION between them case-SENSITIVE, so a predicate over section
// `credential` and final component `helper` reaches every row real git resolved
// the helper for — including the URL-SCOPED spelling, whose subsection is an
// OPEN family no enumeration could close — and reaches none of the three near
// misses. **It is NOT a substring or `contains` test**: `contains` refuses
// `credential.helperx`, a key real git IGNORES, and is not a property of the key
// grammar at all.
// ===========================================================================

#[test]
fn t_19_121_the_credential_helper_key_is_refused_by_name_on_a_leading_option() {
    for command in [
        "git -c credential.helper=store status",
        "git -c CREDENTIAL.HELPER=store status",
        "git -c Credential.Helper=store status",
        "git -c credential.helper=!gh auth git-credential status",
        "git -c credential.helper= status",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "**`T-19-121` — the ARGV half of the credential defence.** `cred.rs`'s injected \
             EMPTY `credential.helper` pair resets git's helper list and reads no command line, \
             so it defends every WRITE spelling; a later `-c credential.helper=<something>` on \
             argv APPENDS to the list git resolves and brings the secret back, measured PRESENT \
             on one permitted line. **THE CONTROLS are the three near misses below, each \
             PERMITTED**, which is what makes this a BY-NAME clause rather than a substring \
             test. The identifier is `envelope_assertion_failed` — the general unresolvable one \
             the sibling refusals carry — and never `HookBypassBlocked`, which names the \
             hooks-path mechanism this refusal does not use (D-24).\n\n\
             **The `-c credential.helper=` EMPTY-VALUE row is an OVER-REFUSAL, disclosed rather \
             than discovered.** The clause reads the KEY half only, so it cannot tell a RESET \
             from a set — and it costs nothing reachable, because the envelope already injects \
             exactly that empty pair.",
        );
    }
}

#[test]
fn t_19_121_the_url_scoped_spelling_is_reached_because_the_subsection_is_never_read() {
    refuses(
        "git -c credential.https://github.com.helper=store status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
        "**THE URL-SCOPED SPELLING, which audit 11 probed and real git resolves the helper \
         for.** Reading SECTION and FINAL COMPONENT and never the subsection is what reaches it: \
         the subsection is any URL, an OPEN family no enumeration could close, and it is also \
         the one half of a git config key git does NOT fold — so reading it would import a case \
         rule as well. **THE CONTROL:** `git -c credential.helper.x=store status` is PERMITTED \
         below, and it is the discriminating one — same section, a subsection, and a FINAL \
         component that is not `helper`, which real git resolves no helper from either.",
    );
}

#[test]
fn t_19_121_the_config_env_carrier_is_reached_too_and_that_was_not_predicted() {
    // **A GAIN BEYOND WHAT THE PLAN PREDICTED, MEASURED RATHER THAN CLAIMED.**
    // `19-30` measured `--config-env=credential.helper=<VAR>` at exit 0 and
    // recorded it as a shape a by-name clause would NOT reach. It IS reached:
    // `leading_git_option` yields an assignment for `--config-env` in both its
    // grammars, and this clause reads the KEY half, which `--config-env` spells
    // on argv exactly as `-c` does. **Recorded here as a correction to a
    // planning-time expectation, not as a row that was aimed at.**
    for command in [
        "git --config-env=credential.helper=EVILVAR status",
        "git --config-env credential.helper=EVILVAR status",
    ] {
        refuses(
            command,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "the `--config-env` carrier delivers an environment variable NAME where `-c` \
             delivers a body — its VALUE half is not read, for the reason the sibling clauses \
             give — but its KEY half is spelled identically and is read identically. Real git \
             was measured resolving the helper from it (`19-30`), so this is a REACH the clause \
             closes rather than an over-refusal. **THE CONTROL:** \
             `git --config-env=credentialx.helper=EVILVAR status` is PERMITTED below.",
        );
    }
}

#[test]
fn t_19_121_the_near_miss_keys_are_permitted_and_that_is_what_makes_the_clause_by_name() {
    // **THE DISCRIMINATING CONTROLS.** Every one of these is a key real git
    // resolves NO helper from, measured secret ABSENT at `19-30`. A `contains`
    // or substring rule turns the third red; a `starts_with("credential.")` rule
    // turns the fifth red; a rule that read only the section turns the third and
    // fifth red. **A control refused the same way at the same identifier would
    // prove nothing — that is `19-27`'s measured failure mode.**
    for command in [
        "git -c credentialx.helper=store status",
        "git -c notcredential.helper=store status",
        "git -c credential.helperx=store status",
        "git -c credential=store status",
        "git -c credential.helper.x=store status",
        "git --config-env=credentialx.helper=EVILVAR status",
        "git -c user.name=x commit -m y",
    ] {
        permits(
            command,
            "a key real git resolves NO credential helper from must stay PERMITTED. \
             `credential.helperx` is the row that proves the clause is not a substring test; \
             `credential.helper.x` is the row that proves the FINAL COMPONENT is read rather \
             than the section alone — git reads `helper` there as a SUBSECTION and `x` as the \
             variable, and resolves no helper; `credential` alone has no section at all and is \
             CONFINED for the reason `config_key_names_an_indirection_section` gives, which is \
             also what keeps round 7's `-c a=b` callee-grammar property green.",
        );
    }
}

#[test]
fn t_19_121_what_the_clause_does_not_reach_is_recorded_in_neither_direction() {
    // **RECORDED, NEVER ASSERTED, AND THE REASON IS THAT AN ASSERTION EITHER WAY
    // WOULD BE A CLAIM ABOUT THE WRONG MECHANISM.**
    //
    // `GIT_CONFIG_PARAMETERS` carries the same key and real git DOES resolve the
    // helper from it, so the reach is real — but it is an ENVIRONMENT variable
    // and not argv, so it is not in the region `scan_leading` reads and this
    // clause is silent about it. The guard refuses the carrier today at
    // `hook_bypass_blocked`, because the variable NAME is in the envelope's own
    // env-key deny. **A row asserted `refused` here would credit this clause
    // with a refusal another mechanism produced (D-24); a row asserted
    // `permitted` would be false. `T-19-104` stays registered.**
    //
    // **`19-30` recorded `T-19-104`'s carrier as `open at exit 0`; it is exit 2
    // `hook_bypass_blocked`, and `19-30`'s own summary corrected the plan on
    // exactly this point.** Re-confirmed here.
    record_only("T-19-104 / GIT_CONFIG_PARAMETERS — a DIFFERENT mechanism", |_| {
        "GIT_CONFIG_PARAMETERS='credential.helper=store' git status".to_string()
    });
    // The `git config` WRITING form is an OPERAND of the `config` verb, not a
    // leading option. It writes a FILE — the family the injected empty pair does
    // not cover either — and no rule is written for it here.
    record_only("T-19-121 / the `git config` WRITING form — unreached", |_| {
        "git config credential.helper store".to_string()
    });
}
