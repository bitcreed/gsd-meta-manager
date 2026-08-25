// ============================================================================
// The mechanical spawn-seam audit (D-17, PITFALLS:521)
//
// One rule governs every assertion below: **a comment is not a guard; the test
// is.** `src/executor/mod.rs` says the opt-in escape hatch has no production
// call site, and `src/journal/` has claimed since Phase 16 that the strict
// unknown-field attribute is kept out by a grep. Prose cannot enforce either.
// This file does.
//
// It is an integration test rather than an in-source one because it reads the
// source tree, and a test that walks `src/` has no business living inside it.
// It is deliberately **portable** — no `#![cfg(unix)]` — because none of the
// properties it checks are platform-dependent.
//
// **The walk covers `src/` only, never `tests/`.** That is what lets this file's
// own prose name the tokens it forbids without invalidating its own gate.
// ============================================================================

use std::path::{Path, PathBuf};

/// The tree under audit. Resolved at compile time, so the test is
/// cwd-independent — the idiom `tests/executor_lifecycle.rs:26-29` already uses.
const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// The integration-test root, walked by the tree-wide scans only.
///
/// Added in round 6: the `DEGENERATE` uniqueness guard claimed a TREE-WIDE
/// property while scanning `src/` alone, and pass 6 found three hand-copied
/// subsets sitting in `tests/` the whole time (WR-04). A guard whose message
/// overclaims its scan is the inheritance vector this round closes.
const TESTS_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");

/// The identifier that bypasses the opt-in gate, and the one file allowed to
/// mention it.
const ESCAPE_HATCH: &str = "for_testing_bypassing_opt_in";
const ESCAPE_HATCH_HOME: &str = "src/executor/mod.rs";

/// Every file under `src/` permitted to contain a process-spawn site.
///
/// **This is a declared allowlist, not a habit.** A later plan in this phase
/// adds `src/driver/spawn.rs`; whoever adds it must add the entry here in the
/// same commit, and that deliberate edit is the entire point — a spawn site that
/// nobody had to think about is how an agent comes to be launched against a
/// directory the user never opted in.
const SPAWN_ALLOWLIST: &[&str] = &[
    // The agent spawn. The one that takes the capability type.
    "src/executor/claude.rs",
    // Git shell-out inside this module's own in-source test helper. No agent.
    "src/executor/outcome.rs",
    // A `sleep` child inside this module's own in-source tests, so the "already
    // gone" path can be pointed at a REAL pid that has been reaped rather than
    // at a number chosen to be implausible. The module's production surface
    // signals two existing process groups and spawns nothing.
    "src/driver/kill.rs",
    // A `sleep` child inside this module's own in-source tests, so the pid-reuse
    // probe can be pointed at a REAL live pid whose cmdline lacks the run id.
    // No agent, and nothing in the module's production surface spawns anything.
    "src/driver/liveness.rs",
    // The detached driver spawn; it re-invokes this same binary, so the
    // capability gate runs in the child.
    "src/driver/spawn.rs",
    // The read-only remote-protection probe: the external GitHub client, asked
    // three questions and never told anything. Bounded by its own budget and run
    // once at run start, never on the guard's per-tool-call path. No agent.
    "src/envelope/advisory.rs",
    // The configured credential command (`gh auth token`, `pass show …`), run
    // as argv with no shell so a value carrying a `;` cannot become a second
    // command. It reads a secret to stdout and never to a file. No agent.
    "src/envelope/cred.rs",
    // `gitleaks` when a binary happens to be on PATH, as an ADDITIVE second
    // opinion on the built-in credential rules (D-11). Its absence is never a
    // reason to allow, and its output is discarded rather than reproduced. No
    // agent.
    "src/envelope/scan.rs",
    // `git check-ignore` for the run-record ignore diagnostic. No agent.
    "src/journal/writer.rs",
    // The TUI's blocking `$EDITOR` shell-out. No agent.
    "src/main.rs",
    // `git init` and the project-creation hook shell-out. No agent.
    "src/project_creator.rs",
    // `pgrep -x claude` for session detection. Reads only, spawns no agent.
    "src/session_detector.rs",
    // Git reads that back project state. No agent.
    "src/state_reader/git_ops.rs",
    // The `gsd-tools` launcher. No agent.
    "src/state_reader/queue_md.rs",
    // tmux, for switching the user to an existing session. No agent.
    "src/terminal_switch.rs",
    // `which` plus the terminal launch for "open in terminal". No agent.
    "src/ui/screens/detail.rs",
];

/// The three shapes a process spawn takes in this tree.
const SPAWN_MARKERS: &[&str] = &["Command::new(", "CommandWrap::with_new(", "process_group("];

/// Whether `line` calls `marker`, as opposed to merely containing its letters.
///
/// **A plain `contains` is wrong here and plan 17-06 is where it first bit.**
/// `rustix::process::kill_process_group(` contains `process_group(`, so a module
/// that only *signals* an existing group was reported as a process-spawn site —
/// and the only way to make the suite green would have been to put a file that
/// spawns nothing onto a **spawn** allowlist, which quietly turns an audit into
/// a list of files somebody once had to add. `test_kill_process_group` and any
/// future `…_process_group` helper are the same case.
///
/// The fix is a left word boundary: the character immediately before the match
/// must not be one that could continue a Rust identifier or a path. `.` and `::`
/// still match, so `cmd.as_std_mut().process_group(0)` and
/// `CommandExt::process_group(` are found exactly as before. The right side
/// needs no boundary because every marker already ends in `(`.
fn calls_marker(line: &str, marker: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = line[from..].find(marker) {
        let at = from + offset;
        let preceded_by_identifier = line[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !preceded_by_identifier {
            return true;
        }
        from = at + marker.len();
    }
    false
}

/// The agent-override fields WR-16 named, by the shape of their declaration.
///
/// Matched as `name:` rather than as a bare identifier on purpose. A bare
/// identifier also matches every *use* — `args.claude_program`, the shorthand
/// `claude_program,` in a pattern — and each of those sits an arbitrary distance
/// below the `#[cfg]` that governs it, so covering them would mean widening the
/// lookback window until it stopped proving anything. The declarations and the
/// struct-literal initialisers are the sites where the attribute is either
/// present or the field exists in release; those are what this audits.
const AGENT_OVERRIDE_FIELDS: &[&str] = &["claude_program:", "claude_args:"];

/// The file whose declaration is the released binary's actual attack surface.
///
/// Named separately so the audit can refuse to pass when it finds no declaration
/// there at all — a rename that emptied the check would otherwise look exactly
/// like a clean run.
const OVERRIDE_PARSER_HOME: &str = "src/cli.rs";

/// How many lines above a declaration the gate may sit.
///
/// Three, because `src/cli.rs` spells the field as doc / `#[cfg]` / `#[arg]` /
/// declaration and a bare struct field spells it as `#[cfg]` / declaration. The
/// window is deliberately small: a gate far enough above to need a bigger one is
/// a gate whose scope a reader cannot see, which is the failure this whole file
/// exists to make impossible.
const GATE_LOOKBACK: usize = 3;

/// The cfg predicate that makes a declaration debug-only.
const DEBUG_ONLY_CFG: &str = "debug_assertions";

/// Whether `line` is a declaration or initialiser of an agent-override field.
fn is_override_declaration(line: &str) -> bool {
    let trimmed = line.trim_start();
    let trimmed = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    AGENT_OVERRIDE_FIELDS
        .iter()
        .any(|field| trimmed.starts_with(field))
}

/// Whether `line` is an attribute that restricts what follows it to debug builds.
///
/// Whitespace is stripped before matching, so `rustfmt` cannot break the audit by
/// reflowing an attribute. `all(debug_assertions, …)` passes because a *narrower*
/// gate is still debug-only; `any(debug_assertions, …)` and
/// `not(debug_assertions)` are rejected because both are ways of widening the
/// gate back to the release build, which is precisely the refactor this test is
/// here to catch.
fn is_debug_only_gate(line: &str) -> bool {
    let dense: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    dense.starts_with("#[cfg(")
        && dense.contains(DEBUG_ONLY_CFG)
        && !dense.contains("any(")
        && !dense.contains("not(")
}

/// The 1-based line numbers of every override-field declaration in `lines`, and
/// of the subset of them that no debug-only gate governs.
///
/// One pass returning both, so the audit can distinguish "no violations" from
/// "nothing was examined" — the two outcomes an assertion on emptiness alone
/// cannot tell apart.
fn override_declarations(lines: &[String]) -> (Vec<usize>, Vec<usize>) {
    let mut found = Vec::new();
    let mut ungated = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("//") || !is_override_declaration(line) {
            continue;
        }
        found.push(index + 1);

        let window_start = index.saturating_sub(GATE_LOOKBACK);
        let gated = lines[window_start..index]
            .iter()
            .any(|above| is_debug_only_gate(above));
        if !gated {
            ungated.push(index + 1);
        }
    }

    (found, ungated)
}

/// Serde's strict unknown-field rejection attribute, assembled at **runtime**
/// from two halves.
///
/// Written this way on purpose: spelled out as one literal, this file's own
/// source would match a naive `grep -r` of the repository for the attribute and
/// the guard would start reporting itself. The halves are meaningless apart.
const REJECT_HEAD: &str = "deny_unknown";
const REJECT_TAIL: &str = "_fields";

/// One source file: its path relative to the crate root, and its numbered lines.
type SourceFile = (String, Vec<(usize, String)>);

/// Every `*.rs` file under `src/`, recursively, sorted by path.
///
/// The recursive `read_dir` shape follows `src/journal/writer.rs:571` and
/// `src/archive.rs:116`. An unreadable entry is skipped rather than panicked on,
/// exactly as those do.
fn source_files() -> Vec<SourceFile> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    collect(Path::new(SRC_ROOT), &base, &mut out);
    assert!(
        !out.is_empty(),
        "the audit walked {SRC_ROOT} and found no Rust source at all, which means \
         it is auditing nothing"
    );
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Every `*.rs` file under `src/` AND `tests/`, sorted by path.
///
/// For the scans whose property is genuinely tree-wide. Kept separate from
/// [`source_files`] rather than replacing it: most guards in this file are
/// deliberately about PRODUCTION code, and widening them wholesale would change
/// what they assert.
fn source_and_test_files() -> Vec<SourceFile> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    collect(Path::new(SRC_ROOT), &base, &mut out);
    collect(Path::new(TESTS_ROOT), &base, &mut out);
    assert!(
        out.iter().any(|(path, _)| path.starts_with("src/")),
        "the tree-wide walk found no production source, so it is auditing less \
         than it claims"
    );
    assert!(
        out.iter().any(|(path, _)| path.starts_with("tests/")),
        "the tree-wide walk found no integration test source at all. That is \
         exactly the shape of the overclaim this walk exists to fix — a scan \
         that reports clean because it never looked."
    );
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect(dir: &Path, base: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect(&path, base, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect();
        out.push((relative, lines));
    }
}

/// The lines of `file` that are not comments.
///
/// A line whose trimmed form starts with `//` is dropped, so a doc comment
/// naming a forbidden token cannot invalidate its own gate. That filter is what
/// lets the code below be documented in the very terms it forbids.
fn executable_lines(file: &SourceFile) -> impl Iterator<Item = &(usize, String)> {
    file.1
        .iter()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
}

/// Every `(path, line number, line)` under `src/` whose executable text contains
/// `needle`.
fn executable_hits(files: &[SourceFile], needle: &str) -> Vec<(String, usize, String)> {
    let mut hits = Vec::new();
    for file in files {
        for (number, line) in executable_lines(file) {
            if line.contains(needle) {
                hits.push((file.0.clone(), *number, line.trim().to_string()));
            }
        }
    }
    hits
}

/// Render hits for a failure message, one per line.
fn render(hits: &[(String, usize, String)]) -> String {
    hits.iter()
        .map(|(path, number, line)| format!("\n  {path}:{number}: {line}"))
        .collect::<String>()
}

#[test]
fn the_escape_hatch_has_no_call_site_in_src() {
    let files = source_files();
    let hits = executable_hits(&files, ESCAPE_HATCH);

    let offenders: Vec<_> = hits
        .iter()
        .filter(|(path, _, _)| path != ESCAPE_HATCH_HOME)
        .cloned()
        .collect();
    assert!(
        offenders.is_empty(),
        "`{ESCAPE_HATCH}` bypasses the user's opt-in. A production call site means the \
         compiler is no longer what enforces the gate, and CTRL-03's \"never\" stops being \
         literally true (D-17, PITFALLS:521). Offending lines:{}",
        render(&offenders)
    );

    assert_eq!(
        hits.len(),
        1,
        "the escape hatch must appear on exactly one executable line — its own \
         definition in {ESCAPE_HATCH_HOME}. Found:{}",
        render(&hits)
    );
    assert!(
        hits[0].2.contains("pub fn"),
        "the single occurrence must be the `pub fn` definition, not a use of it. \
         Found:{}",
        render(&hits)
    );
}

#[test]
fn drivable_project_has_exactly_two_constructors_and_private_fields() {
    let files = source_files();
    let executor = files
        .iter()
        .find(|(path, _)| path == ESCAPE_HATCH_HOME)
        .unwrap_or_else(|| panic!("{ESCAPE_HATCH_HOME} must exist"));

    let production: Vec<_> = executable_lines(executor)
        .filter(|(_, line)| line.contains("pub fn from_registry"))
        .collect();
    assert_eq!(
        production.len(),
        1,
        "there is exactly one production constructor for the capability token (D-16); \
         found {} in {ESCAPE_HATCH_HOME}",
        production.len()
    );

    let hatch: Vec<_> = executable_lines(executor)
        .filter(|(_, line)| line.contains(&format!("pub fn {ESCAPE_HATCH}")))
        .collect();
    assert_eq!(
        hatch.len(),
        1,
        "there is exactly one escape hatch, and a second would be a second way to \
         bypass the gate; found {}",
        hatch.len()
    );

    // The struct body, from its opening line to the first line that is a bare
    // closing brace at column zero.
    let mut body = Vec::new();
    let mut inside = false;
    for (number, line) in &executor.1 {
        if line.starts_with("pub struct DrivableProject {") {
            inside = true;
            continue;
        }
        if inside {
            if line == "}" {
                break;
            }
            body.push((*number, line.clone()));
        }
    }
    assert!(
        !body.is_empty(),
        "the audit could not locate the `DrivableProject` struct body, so it is \
         checking nothing"
    );

    let public: Vec<_> = body
        .iter()
        .filter(|(_, line)| line.trim_start().starts_with("pub "))
        .map(|(number, line)| {
            (
                ESCAPE_HATCH_HOME.to_string(),
                *number,
                line.trim().to_string(),
            )
        })
        .collect();
    assert!(
        public.is_empty(),
        "every field of `DrivableProject` must be private: private fields are what stop \
         a caller assembling the token without passing a constructor, which is the whole \
         reason it is a type rather than a bool. Public fields:{}",
        render(&public)
    );
}

#[test]
fn every_process_spawn_site_in_src_is_on_the_allowlist() {
    let files = source_files();

    let mut observed: Vec<String> = Vec::new();
    for file in &files {
        let spawns = executable_lines(file)
            .any(|(_, line)| SPAWN_MARKERS.iter().any(|marker| calls_marker(line, marker)));
        if spawns {
            observed.push(file.0.clone());
        }
    }
    observed.sort();

    let mut allowed: Vec<String> = SPAWN_ALLOWLIST.iter().map(|path| path.to_string()).collect();
    allowed.sort();

    let unexpected: Vec<&String> = observed.iter().filter(|p| !allowed.contains(p)).collect();
    assert!(
        unexpected.is_empty(),
        "a process-spawn site appeared in a file that is not on the allowlist. Confirm it \
         takes a capability type and then add it to `SPAWN_ALLOWLIST` in this file, in the \
         same commit (PITFALLS:521). Unexpected: {unexpected:?}"
    );

    let vanished: Vec<&String> = allowed.iter().filter(|p| !observed.contains(p)).collect();
    assert!(
        vanished.is_empty(),
        "an allowlisted file no longer spawns anything, so the allowlist is now wider than \
         the truth it describes. Remove the stale entry: {vanished:?}"
    );

    let claude = files
        .iter()
        .find(|(path, _)| path == "src/executor/claude.rs")
        .expect("the agent spawn seam must exist");
    assert!(
        executable_lines(claude).any(|(_, line)| line.contains("project: &DrivableProject")),
        "the agent spawn seam must still take the capability type and never a bare path — \
         that signature is what makes the opt-in gate a compile-time property (D-16)"
    );
}

#[test]
fn a_signal_to_a_process_group_is_not_mistaken_for_a_spawn() {
    // The guard-of-the-guard. Without the left word boundary in `calls_marker`,
    // the marker `process_group(` matches inside `kill_process_group(`, and a
    // module that only signals an existing group is reported as a process-spawn
    // site. The only way to make the suite green then is to put a file that
    // spawns nothing onto a SPAWN allowlist — which converts an audit of "every
    // spawn takes a capability type" into a list of files somebody once had to
    // add, silently and without anybody noticing the meaning changed.
    for line in [
        "    rustix::process::kill_process_group(group, sig).map_err(Error::from)",
        "    let _ = rustix::process::test_kill_process_group(group);",
    ] {
        assert!(
            !calls_marker(line, "process_group("),
            "signalling an existing process group is not spawning one: {line}"
        );
    }

    // The control arm, and it is not optional: a boundary check that rejected
    // everything would also pass the loop above while disabling the audit
    // outright. These are the two real spawn shapes in the tree.
    for line in [
        "    cmd.as_std_mut().process_group(0);",
        "    .process_group(0)",
        "    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);",
    ] {
        assert!(
            calls_marker(line, "process_group("),
            "a genuine detached spawn must still be found: {line}"
        );
    }
    assert!(calls_marker(
        "    let mut cmd = std::process::Command::new(program);",
        "Command::new("
    ));
}

#[test]
fn the_agent_program_override_fields_are_debug_only() {
    // **Why a `#[cfg]` needs a guard at all**, which is the same argument the
    // header of this file makes about comments. An attribute is enforced by the
    // compiler only for as long as it is there. A refactor that lifts it to a
    // wider predicate, or drops it while moving a field, compiles cleanly, ships
    // cleanly, and is indistinguishable from never having added it — because the
    // property it protects is invisible in every build a developer runs. Debug
    // is where these fields are supposed to work.
    //
    // What is at stake if this test fails: a release build of this binary
    // accepts a flag that makes the "driver" exec an arbitrary program with an
    // opted-in project as its working directory, and journals the result as an
    // ordinary GSD run (D-30, WR-16).
    let files = source_files();

    let mut found_total = 0usize;
    let mut found_in_parser = 0usize;
    let mut offenders: Vec<(String, usize, String)> = Vec::new();

    for file in &files {
        let lines: Vec<String> = file.1.iter().map(|(_, line)| line.clone()).collect();
        let (found, ungated) = override_declarations(&lines);

        found_total += found.len();
        if file.0 == OVERRIDE_PARSER_HOME {
            found_in_parser += found.len();
        }

        for number in ungated {
            offenders.push((file.0.clone(), number, lines[number - 1].trim().to_string()));
        }
    }

    assert!(
        offenders.is_empty(),
        "an agent-override field is declared without a debug-only gate. A release build \
         of this binary would then accept a flag letting any caller exec an arbitrary \
         program inside an opted-in project root, with the journal recording it as a \
         normal GSD run (D-30, WR-16). The attribute belongs within {GATE_LOOKBACK} lines \
         above the declaration. Ungated:{}",
        render(&offenders)
    );

    // Non-vacuity, in the register `source_files` already uses: an audit that
    // examined nothing passes for the wrong reason. A rename of these fields must
    // fail here loudly and be re-pointed deliberately, not pass silently.
    assert!(
        found_in_parser >= AGENT_OVERRIDE_FIELDS.len(),
        "the audit found {found_in_parser} agent-override declarations in \
         {OVERRIDE_PARSER_HOME} and expected at least {}, so it is checking less than it \
         thinks. If the fields were renamed, re-point `AGENT_OVERRIDE_FIELDS`; if they \
         were removed outright, delete this test in the same commit",
        AGENT_OVERRIDE_FIELDS.len()
    );
    assert!(
        found_total >= found_in_parser,
        "counting is broken, which would make every assertion above meaningless"
    );
}

#[test]
fn an_override_field_declared_without_the_debug_gate_is_reported() {
    // The control arm, and it is not optional: `the_agent_program_override_fields_are_debug_only`
    // asserts an emptiness, and a matcher that recognised no declaration at all
    // would satisfy it forever while auditing nothing. These snippets are
    // synthetic rather than read from the tree, so the arm keeps proving the
    // matcher works even once — especially once — the tree is correct.
    let ungated: Vec<String> = [
        "    /// Test and development only: the program to spawn instead of `claude`",
        "    #[arg(long, hide = true)]",
        "    claude_program: Option<PathBuf>,",
    ]
    .iter()
    .map(|line| line.to_string())
    .collect();
    let (found, violations) = override_declarations(&ungated);
    assert_eq!(found.len(), 1, "the declaration itself must be recognised");
    assert_eq!(
        violations,
        vec![3],
        "a declaration with no debug-only gate above it must be reported, or this \
         audit is a no-op that passes"
    );

    // And the same snippet with the gate restored must be clean, so the matcher
    // is not simply reporting everything.
    let gated: Vec<String> = [
        "    /// Test and development only: the program to spawn instead of `claude`",
        "    #[cfg(debug_assertions)]",
        "    #[arg(long, hide = true)]",
        "    claude_program: Option<PathBuf>,",
    ]
    .iter()
    .map(|line| line.to_string())
    .collect();
    let (found, violations) = override_declarations(&gated);
    assert_eq!(found.len(), 1);
    assert!(
        violations.is_empty(),
        "a properly gated declaration must not be reported: {violations:?}"
    );

    // The gate predicate itself. `all(...)` narrows and is still debug-only;
    // `any(...)` and `not(...)` are the two shapes that quietly hand the field
    // back to the release build, which is the refactor this test exists to catch.
    for accepted in [
        "#[cfg(debug_assertions)]",
        "  #[ cfg ( debug_assertions ) ]",
        "#[cfg(all(debug_assertions, unix))]",
    ] {
        assert!(
            is_debug_only_gate(accepted),
            "this restricts the declaration to debug builds: {accepted}"
        );
    }
    for rejected in [
        "#[cfg(any(debug_assertions, feature = \"dev-tools\"))]",
        "#[cfg(not(debug_assertions))]",
        "#[cfg(test)]",
        "#[arg(long, hide = true)]",
        "    claude_program: Option<PathBuf>,",
    ] {
        assert!(
            !is_debug_only_gate(rejected),
            "this does not restrict the declaration to debug builds: {rejected}"
        );
    }

    // A use is not a declaration. Widening the matcher to catch these would mean
    // widening the lookback window past the point where it proves anything.
    for use_site in [
        "    let executor = match &args.claude_program {",
        "            claude_program,",
    ] {
        assert!(
            !is_override_declaration(use_site),
            "a use site is out of this audit's scope by design: {use_site}"
        );
    }
}

#[test]
fn no_executable_line_in_src_opts_into_strict_unknown_field_rejection() {
    let files = source_files();
    let attribute = format!("{REJECT_HEAD}{REJECT_TAIL}");
    let hits = executable_hits(&files, &attribute);

    assert!(
        hits.is_empty(),
        "no executable line under src/ may opt into strict unknown-field rejection. \
         Parsing is tolerant by construction: every line reaching the journal reader came \
         from a file an untrusted agent's output shaped, and Phase 20 is a known future \
         emitter of new record kinds — an attribute that rejects them turns forward \
         compatibility into a hard parse failure (D-30). Offending lines:{}",
        render(&hits)
    );
}

// ============================================================================
// The model-seam profile's controls, coupled by tests rather than by convention
//
// Four guards, each in this file's house style: scan `executable_lines` so a doc
// comment naming a declined alternative does not trip the scan, assert
// non-vacuity so a rename that empties a scan fails rather than passes, and diff
// observed against declared in BOTH directions wherever a set is involved.
// ============================================================================

/// The hook-disabling flag, assembled at **runtime** from two halves.
///
/// Written this way for the reason [`REJECT_HEAD`] is: spelled out as one
/// literal, this file's own source would match its own scan and the guard would
/// start reporting itself. The halves are meaningless apart.
const HOOK_DISABLING_HEAD: &str = "--safe";
const HOOK_DISABLING_TAIL: &str = "-mode";

/// The flag that would resume a prior session on a seam spawn.
const RESUME_FLAG: &str = "--resume";

/// The struct bodies whose free-string fields must be enumerated, and the file
/// that declares each.
const UNTRUSTED_STRUCTS: &[(&str, &str)] = &[
    ("ProjectState", "src/state_reader/mod.rs"),
    ("RoadmapPhase", "src/state_reader/roadmap_md.rs"),
];

/// The module whose enumeration those fields must appear in.
const UNTRUSTED_ENUMERATION_HOME: &str = "src/driver/untrusted.rs";

/// Every free-string field `struct_name` declares, as `Struct::field`.
///
/// A field counts when its declared type is `String`, `Option<String>` or
/// `Vec<String>` — the three shapes that carry third-party *text*. A
/// `HashMap<String, T>` is excluded because there the `String` is a **key** the
/// reader generates (a phase number), not a payload the repository wrote; the
/// control arm below proves that distinction rather than asserting it.
///
/// The scan stops at the first line that is exactly `}`, so a nested type inside
/// the struct cannot leak fields from beyond it.
fn free_string_fields(file: &SourceFile, struct_name: &str) -> Vec<String> {
    let opening = format!("pub struct {struct_name} {{");
    let mut out = Vec::new();
    let mut inside = false;

    for (_, line) in &file.1 {
        if !inside {
            if line.trim_start().starts_with(&opening) {
                inside = true;
            }
            continue;
        }
        if line.trim_end() == "}" {
            break;
        }
        if let Some(field) = free_string_field_name(line) {
            out.push(format!("{struct_name}::{field}"));
        }
    }

    out.sort();
    out
}

/// The field name on `line`, when `line` declares a free-string field.
fn free_string_field_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with("#[") {
        return None;
    }
    let declaration = trimmed.strip_prefix("pub ")?;
    let (name, rest) = declaration.split_once(':')?;
    if name.trim().is_empty() || name.contains(' ') {
        return None;
    }
    let declared_type: String = rest
        .trim()
        .trim_end_matches(',')
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    matches!(
        declared_type.as_str(),
        "String" | "Option<String>" | "Vec<String>"
    )
    .then(|| name.trim().to_string())
}

/// Every `Struct::field` pair the untrusted enumeration names.
///
/// Read out of the enumeration's own source rather than by calling into the
/// crate, so the guard checks what is **written** rather than what a constructor
/// happened to return — the same posture every other scan in this file takes.
fn enumerated_untrusted_fields(files: &[SourceFile]) -> Vec<String> {
    let home = files
        .iter()
        .find(|(path, _)| path == UNTRUSTED_ENUMERATION_HOME)
        .expect("the untrusted enumeration must exist");

    let mut out = Vec::new();
    let mut pending_struct: Option<String> = None;

    for (_, line) in executable_lines(home) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("struct_name: \"") {
            if let Some(name) = rest.split('"').next() {
                pending_struct = Some(name.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("field: \"") {
            if let (Some(struct_name), Some(field)) = (&pending_struct, rest.split('"').next()) {
                out.push(format!("{struct_name}::{field}"));
                pending_struct = None;
            }
        }
    }

    out.sort();
    out
}

#[test]
fn the_seam_profile_couples_the_empty_tool_set_to_the_schema() {
    // Guard one. It calls `build_argv` and inspects the vector it RETURNS. It
    // does not assert about a constant that `build_argv` also reads: a test that
    // compares a constant with itself cannot detect its return, which is exactly
    // the Critical Phase 20's code review found (`src/driver/dry_run.rs:582-584`
    // is the counter-pattern this codebase wrote for itself).
    use gsd_meta_manager::executor::claude::build_argv;
    use gsd_meta_manager::executor::{ExecutionOptions, SpawnProfile};

    let seam = ExecutionOptions {
        profile: SpawnProfile::ModelSeam {
            json_schema: gsd_meta_manager::driver::goal::escalation_schema().to_string(),
        },
        ..ExecutionOptions::default()
    };
    let words: Vec<String> = build_argv(&seam)
        .iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect();

    assert!(!words.is_empty(), "build_argv returned nothing to examine");

    let tools_at = words
        .iter()
        .position(|word| word == "--tools")
        .unwrap_or_else(|| panic!("the seam profile must carry the tool flag; argv was {words:?}"));
    assert_eq!(
        words.get(tools_at + 1).map(String::as_str),
        Some(""),
        "the tool flag must carry an EMPTY value. With it present and empty the \
         CLI advertises exactly the structured-output tool; with it absent it \
         advertises everything, and the OQ1 measurement — and every later \
         injection assertion — is confounded by a seam that quietly had file \
         access. argv was {words:?}"
    );
    assert!(
        words.iter().any(|word| word == "--json-schema"),
        "the seam profile must carry the schema flag together with the empty \
         tool set: they are one control, not two. argv was {words:?}"
    );

    // And the executor profile is byte-identical to what it produces today, so
    // the new discriminant cannot silently change an existing run.
    let executor: Vec<String> = build_argv(&ExecutionOptions::default())
        .iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect();
    let expected: Vec<&str> = vec![
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--replay-user-messages",
        "--session-id",
        "<uuid>",
        "--setting-sources",
        "project",
        "--permission-mode",
        "dontAsk",
        "--strict-mcp-config",
    ];
    assert_eq!(executor.len(), expected.len(), "argv was {executor:?}");
    for (index, want) in expected.iter().enumerate() {
        if *want == "<uuid>" {
            assert_eq!(executor[index].len(), 36, "argv was {executor:?}");
            continue;
        }
        assert_eq!(&executor[index], want, "argv was {executor:?}");
    }
}

#[test]
fn no_executable_line_in_src_passes_the_hook_disabling_flag() {
    // Guard two. The reason rather than the rule, because a future author reads
    // the failure message at the moment of the change.
    let files = source_files();
    let flag = format!("{HOOK_DISABLING_HEAD}{HOOK_DISABLING_TAIL}");
    let hits = executable_hits(&files, &flag);

    assert!(
        hits.is_empty(),
        "no executable line under src/ may pass the CLI's hook-disabling flag on \
         any profile. It suppresses CLAUDE.md, which the model seam wants — and \
         it ALSO disables hooks, and Phase 19's git envelope is enforced by a \
         PreToolUse hook (src/envelope/hooks.rs). A seam spawned with it runs \
         with the git boundary off and NOTHING on the wire says so. Set \
         CLAUDE_CODE_DISABLE_CLAUDE_MDS in the spawn closure instead. \
         Offending lines:{}",
        render(&hits)
    );

    // Non-vacuity: the scan must have had a non-empty tree to examine.
    assert!(
        files.len() > 1,
        "the scan examined {} files, so the emptiness above proves nothing",
        files.len()
    );
}

#[test]
fn the_hook_disabling_scan_fires_on_code_and_not_on_a_comment() {
    // Guard two's control arm, both directions in one test so the guard cannot
    // be satisfied by a scanner that reports nothing. Synthetic lines rather than
    // lines read from the tree, so this keeps proving the scanner works once —
    // especially once — the tree is correct.
    let flag = format!("{HOOK_DISABLING_HEAD}{HOOK_DISABLING_TAIL}");

    let executable = format!("    push(&mut argv, \"{flag}\");");
    let synthetic: SourceFile = (
        "src/synthetic.rs".to_string(),
        vec![(1, executable.clone())],
    );
    assert_eq!(
        executable_hits(std::slice::from_ref(&synthetic), &flag).len(),
        1,
        "the scanner must report an executable line carrying the flag, or the \
         guard above is a no-op that passes forever: {executable}"
    );

    // The declined alternative must remain DOCUMENTABLE in a comment — that is
    // what the rationale-in-code convention requires, and `src/executor/mod.rs`
    // really does name it in a doc comment today.
    let commented = format!("/// The declined alternative: the CLI's `{flag}` flag disables hooks.");
    let synthetic_comment: SourceFile =
        ("src/synthetic.rs".to_string(), vec![(1, commented.clone())]);
    assert!(
        executable_hits(std::slice::from_ref(&synthetic_comment), &flag).is_empty(),
        "the scanner must NOT report a doc comment naming the flag, or a guard \
         whose whole point is a documented rationale would forbid documenting \
         it: {commented}"
    );
}

#[test]
fn the_permitted_mcp_set_stays_empty_on_both_profiles() {
    // Guard three, both directions: losing the strict flag fails, and gaining a
    // config path fails.
    use gsd_meta_manager::executor::claude::build_argv;
    use gsd_meta_manager::executor::{ExecutionOptions, SpawnProfile};

    for (name, options) in [
        ("executor", ExecutionOptions::default()),
        (
            "seam",
            ExecutionOptions {
                profile: SpawnProfile::ModelSeam {
                    json_schema: gsd_meta_manager::driver::goal::escalation_schema().to_string(),
                },
                ..ExecutionOptions::default()
            },
        ),
    ] {
        let words: Vec<String> = build_argv(&options)
            .iter()
            .map(|word| word.to_string_lossy().into_owned())
            .collect();
        assert!(!words.is_empty(), "{name}: build_argv returned nothing");

        assert!(
            words.iter().any(|word| word == "--strict-mcp-config"),
            "the {name} profile lost --strict-mcp-config. With it present and no \
             config path supplied the permitted MCP server set is EMPTY; without \
             it, a project-local .mcp.json in a cloned third-party repository \
             introduces tools into a driven run. argv was {words:?}"
        );
        assert!(
            !words.iter().any(|word| word == "--mcp-config"),
            "the {name} profile supplied an MCP config path. With none supplied \
             the permitted set is empty; supplying one moves it from empty to \
             whatever that file names, which is a widening wearing a control's \
             clothing. argv was {words:?}"
        );

        // Each consultation is a fresh single-turn spawn. The empty-tool and
        // schema flags were never exercised in composition with a resumed
        // session, and the seams are single-turn BY CONSTRUCTION — this
        // assertion is what keeps that true rather than incidental.
        assert!(
            !words.iter().any(|word| word == RESUME_FLAG),
            "the {name} profile carried a session-resume flag. A resumed seam \
             would carry prior turns into a consultation whose bounds were \
             measured on a fresh one. argv was {words:?}"
        );
    }

    // And no executable line under src/ supplies a config path either, so the
    // absence above cannot be reintroduced through a different construction.
    let files = source_files();
    let hits = executable_hits(&files, "--mcp-config");
    assert!(
        hits.is_empty(),
        "an executable line under src/ supplies an MCP config path:{}",
        render(&hits)
    );
    assert!(
        !executable_hits(&files, "--strict-mcp-config").is_empty(),
        "no executable line under src/ emits --strict-mcp-config at all, so the \
         absence assertions above are about a flag nothing produces"
    );
}

#[test]
fn every_free_string_field_that_could_reach_a_prompt_is_enumerated() {
    // Guard four, both directions. A `String`-typed field on either struct that
    // the enumeration does not name is a field that could reach a prompt
    // UNLABELLED; an enumerated entry with no corresponding field means the
    // enumeration is wider than the truth it describes.
    let files = source_files();

    let mut observed: Vec<String> = Vec::new();
    for (struct_name, path) in UNTRUSTED_STRUCTS {
        let file = files
            .iter()
            .find(|(candidate, _)| candidate == path)
            .unwrap_or_else(|| panic!("{path} must exist to be audited"));
        let fields = free_string_fields(file, struct_name);
        assert!(
            !fields.is_empty(),
            "the parser found no free-string field on {struct_name} in {path}, so \
             the diff below would pass vacuously. If the struct was renamed, \
             re-point UNTRUSTED_STRUCTS in the same commit"
        );
        observed.extend(fields);
    }
    observed.sort();

    let declared = enumerated_untrusted_fields(&files);
    assert!(
        !declared.is_empty(),
        "the enumeration in {UNTRUSTED_ENUMERATION_HOME} parsed as empty, so this \
         audit is checking nothing"
    );

    let unenumerated: Vec<&String> = observed.iter().filter(|f| !declared.contains(f)).collect();
    assert!(
        unenumerated.is_empty(),
        "a free-string field is declared on a struct parsed from a third-party \
         repository and is not named in {UNTRUSTED_ENUMERATION_HOME}. Every such \
         field is text somebody else wrote, and one nobody classified is one that \
         can reach a model prompt unlabelled. Add it with a Disposition in the \
         same commit. Unenumerated: {unenumerated:?}"
    );

    let stale: Vec<&String> = declared.iter().filter(|f| !observed.contains(f)).collect();
    assert!(
        stale.is_empty(),
        "the enumeration names a field neither struct declares any more, so it is \
         wider than the truth it describes — an allowlist wider than the truth is \
         the failure that shape exists to catch (T-20-17). Stale: {stale:?}"
    );
}

#[test]
fn the_spawn_closure_comment_no_longer_claims_one_variable_is_set() {
    // The pinned-honesty guard, in the shape `src/driver/dry_run.rs:580-595`
    // establishes. The stale claim is spelled out here VERBATIM rather than
    // referenced, because a test that compared the comment with itself could not
    // detect its return.
    //
    // What was falsified: the closure used to set one variable and its comment
    // said so. It now sets three, two of them under the seam profile only, and a
    // user-facing — here, maintainer-facing — statement the code has falsified is
    // worse than having said nothing.
    let files = source_files();
    let claude = files
        .iter()
        .find(|(path, _)| path == "src/executor/claude.rs")
        .expect("the agent spawn seam must exist");
    let text: String = claude
        .1
        .iter()
        .map(|(_, line)| format!("{line}\n"))
        .collect();

    assert!(
        !text.contains("then set the one we mean to set"),
        "the spawn closure sets three variables now, so the comment claiming it \
         sets one is false. Rewrite it in the same commit as the code that \
         falsified it (the `dry_run.rs:78-83` precedent)"
    );
    assert!(
        text.contains("set the ones we mean to set"),
        "and the replacement must state the plural rather than merely being \
         vaguer than what it replaced"
    );

    // The honesty the phase actually owes: there is NO field on this transport
    // reporting whether CLAUDE.md suppression took effect. Claiming otherwise —
    // or saying nothing and letting a reader assume the init envelope covers it
    // the way it covers the tool set and the MCP list — is the unearned
    // assurance this codebase's conventions exist to prevent.
    assert!(
        text.contains("There is NO on-the-wire signal that either took effect."),
        "the spawn closure must state that no init-envelope field reports \
         whether CLAUDE.md suppression took effect, and name what guards it \
         instead (the argv/env source scan plus the 21-05 corpus fixture)"
    );
    assert!(
        text.contains("CLAUDE_CODE_DISABLE_CLAUDE_MDS")
            && text.contains("MAX_STRUCTURED_OUTPUT_RETRIES"),
        "and it must still name both variables, or the statement above is about \
         code that is no longer there"
    );
}

// ============================================================================
// The goal-decomposition capability, and the property its SHAPE holds
//
// The property: **the driver sets its goal once, from a human, and may never
// enqueue itself another goal from an artifact created during its own run.** A
// run that can write its own next goal has no bound that means anything.
//
// The shape that holds it is the one `DrivableProject` already uses — private
// fields, exactly two constructors, and a consuming method that takes the value
// by MOVE — so a second decomposition inside the iteration loop does not
// compile. These guards check the shape has not quietly stopped being that
// shape, which is a thing a comment cannot do.
// ============================================================================

/// The capability type, and the one file allowed to construct it.
const DECOMPOSITION_TYPE: &str = "GoalDecomposition";
const DECOMPOSITION_HOME: &str = "src/driver/run.rs";

/// Its production constructor, and the function that must enclose the only call.
const DECOMPOSITION_CONSTRUCTOR: &str = "from_argv_goal";
const DECOMPOSITION_CALL_SITE_HOME: &str = "src/driver/mod.rs";
const DECOMPOSITION_CALL_SITE_FN: &str = "drive";

/// Its self-incriminating test escape hatch.
const DECOMPOSITION_ESCAPE_HATCH: &str = "for_testing_bypassing_the_human_goal";

/// Its consuming method. Takes `self` by value, never `&self`.
const DECOMPOSITION_CONSUMER: &str = "pub async fn decompose(";

/// The label of the iteration loop the capability may never be constructed
/// inside.
const ITERATION_LOOP_LABEL: &str = "'iterations:";

/// The body of `struct <name>`, from its opening line to the first bare `}` at
/// column zero.
///
/// The same parse `drivable_project_has_exactly_two_constructors_and_private_fields`
/// performs inline, lifted so two guards can share it rather than each growing
/// its own copy that can disagree.
fn struct_body(file: &SourceFile, name: &str) -> Vec<(usize, String)> {
    let opening = format!("pub struct {name} {{");
    let mut body = Vec::new();
    let mut inside = false;
    for (number, line) in &file.1 {
        if line.starts_with(&opening) {
            inside = true;
            continue;
        }
        if inside {
            if line == "}" {
                break;
            }
            body.push((*number, line.clone()));
        }
    }
    body
}

/// The name of the function enclosing 1-based `line_number` in `lines`.
///
/// Found by scanning **upwards** for the nearest `fn <name>(` declaration,
/// which is what makes the answer about where the call actually sits rather
/// than about which function happens to be nearest in the file.
fn enclosing_fn(lines: &[(usize, String)], line_number: usize) -> Option<String> {
    lines
        .iter()
        .filter(|(number, _)| *number <= line_number)
        .rev()
        .find_map(|(_, line)| fn_name_on(line))
}

/// The function name a line declares, if it declares one.
fn fn_name_on(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    for prefix in ["pub(crate) ", "pub ", "async ", "const ", "unsafe "] {
        while let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped;
        }
    }
    // `async` can follow `pub`, so strip once more after the visibility pass.
    for prefix in ["async ", "const ", "unsafe "] {
        while let Some(stripped) = rest.strip_prefix(prefix) {
            rest = stripped;
        }
    }
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Whether 1-based `line_number` sits inside the brace scope opened by a line
/// carrying `label`.
///
/// Brace counting from the label line, which is what distinguishes "inside the
/// loop" from "below the loop in the same function" — a line-number comparison
/// alone would call every later line a violation and every earlier one clean,
/// which is not the property.
///
/// Braces inside string literals and comments are not tracked. That is a stated
/// limit rather than an oversight: the scan runs over Rust source in this tree's
/// own house style, and the control arm below is what proves it answers both
/// directions on the shapes that actually occur.
fn inside_label_scope(lines: &[(usize, String)], label: &str, line_number: usize) -> bool {
    let mut depth = 0i32;
    let mut open = false;
    for (number, line) in lines {
        if line.trim_start().starts_with("//") {
            continue;
        }
        if !open {
            if line.contains(label) {
                open = true;
                depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
            }
            continue;
        }
        if *number == line_number {
            return depth > 0;
        }
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if depth <= 0 {
            open = false;
        }
    }
    false
}

#[test]
fn the_goal_decomposition_has_exactly_two_constructors_and_private_fields() {
    let files = source_files();
    let home = files
        .iter()
        .find(|(path, _)| path == DECOMPOSITION_HOME)
        .unwrap_or_else(|| panic!("{DECOMPOSITION_HOME} must exist"));

    let production: Vec<_> = executable_lines(home)
        .filter(|(_, line)| line.contains(&format!("pub fn {DECOMPOSITION_CONSTRUCTOR}")))
        .collect();
    assert_eq!(
        production.len(),
        1,
        "there is exactly one production constructor for the decomposition \
         capability, and it takes `DriveArgs` — which can only be built from \
         this process's own argv. A second constructor is a second way for a \
         goal to enter the driver, and the one that matters is the one that \
         could take a goal out of a file the run itself wrote. Found {}",
        production.len()
    );

    let hatch: Vec<_> = executable_lines(home)
        .filter(|(_, line)| line.contains(&format!("pub fn {DECOMPOSITION_ESCAPE_HATCH}")))
        .collect();
    assert_eq!(
        hatch.len(),
        1,
        "there is exactly one escape hatch, and a second would be a second way \
         to bypass the human-goal property; found {}",
        hatch.len()
    );

    let body = struct_body(home, DECOMPOSITION_TYPE);
    assert!(
        !body.is_empty(),
        "the audit could not locate the `{DECOMPOSITION_TYPE}` struct body, so \
         it is checking nothing"
    );
    let public: Vec<_> = body
        .iter()
        .filter(|(_, line)| line.trim_start().starts_with("pub "))
        .map(|(number, line)| {
            (
                DECOMPOSITION_HOME.to_string(),
                *number,
                line.trim().to_string(),
            )
        })
        .collect();
    assert!(
        public.is_empty(),
        "every field of `{DECOMPOSITION_TYPE}` must be private: a public field \
         is a caller assembling the capability without passing a constructor, \
         which is the whole reason it is a type rather than a `String`. Public \
         fields:{}",
        render(&public)
    );
}

#[test]
fn the_decomposition_is_consumed_by_move_and_carries_no_clone_or_copy() {
    let files = source_files();
    let home = files
        .iter()
        .find(|(path, _)| path == DECOMPOSITION_HOME)
        .unwrap_or_else(|| panic!("{DECOMPOSITION_HOME} must exist"));

    // The consuming method's signature, read as source text rather than
    // inferred. **Against the unfixed behaviour — a method taking `&self` —
    // this FAILS by finding a reference signature**, because the whole
    // mechanism is that there is no capability left afterwards.
    let consumer: Vec<_> = executable_lines(home)
        .filter(|(_, line)| line.contains(DECOMPOSITION_CONSUMER))
        .collect();
    assert_eq!(
        consumer.len(),
        1,
        "the consuming method must be declared exactly once; found {}",
        consumer.len()
    );

    // The receiver is on the line after the signature in this file's style, so
    // read the few lines that follow and require a bare `self,`.
    let signature_line = consumer[0].0;
    let receiver: Vec<String> = home
        .1
        .iter()
        .filter(|(number, _)| *number > signature_line && *number <= signature_line + 3)
        .map(|(_, line)| line.trim().to_string())
        .collect();
    assert!(
        receiver.iter().any(|line| line == "self,"),
        "the consuming method must take the capability BY VALUE. A `&self` or \
         `&mut self` receiver leaves the value alive, so a second decomposition \
         inside the iteration loop would compile and the never-self-goal \
         property would be a comment again. Found: {receiver:?}"
    );
    assert!(
        !receiver.iter().any(|line| line.starts_with("&self")
            || line.starts_with("&mut self")
            || line == "&self,"),
        "a reference receiver was found: {receiver:?}"
    );

    // And no derive that would make the move a formality. A `Clone` lets the
    // caller keep a copy; a `Copy` means the move never happened at all.
    let opening = format!("pub struct {DECOMPOSITION_TYPE} {{");
    let declaration = home
        .1
        .iter()
        .position(|(_, line)| line.starts_with(&opening))
        .expect("the capability type must be declared");
    let derives: Vec<String> = home.1[declaration.saturating_sub(4)..declaration]
        .iter()
        .map(|(_, line)| line.trim().to_string())
        .filter(|line| line.starts_with("#[derive"))
        .collect();
    for derive in &derives {
        assert!(
            !derive.contains("Clone") && !derive.contains("Copy"),
            "`{DECOMPOSITION_TYPE}` must derive neither Clone nor Copy — either \
             would let a caller keep a second capability past the move, and the \
             move is the mechanism. Found: {derive}"
        );
    }
}

#[test]
fn the_decomposition_constructor_has_one_call_site_and_it_is_above_the_loop() {
    let files = source_files();
    let marker = format!("{DECOMPOSITION_TYPE}::{DECOMPOSITION_CONSTRUCTOR}(");
    let hits = executable_hits(&files, &marker);

    assert_eq!(
        hits.len(),
        1,
        "the decomposition capability must be constructed at exactly ONE site \
         under src/. A second site is a second place a run can acquire a goal, \
         and the one that matters is a site inside the iteration loop — which is \
         a run enqueueing itself a goal from an artifact it just wrote. \
         Found:{}",
        render(&hits)
    );

    let (path, number, _) = &hits[0];
    assert_eq!(
        path, DECOMPOSITION_CALL_SITE_HOME,
        "the one call site must live where the above-the-run refusals do"
    );

    let home = files
        .iter()
        .find(|(candidate, _)| candidate == path)
        .expect("the call site's file was just read");
    assert_eq!(
        enclosing_fn(&home.1, *number).as_deref(),
        Some(DECOMPOSITION_CALL_SITE_FN),
        "the construction must sit in `{DECOMPOSITION_CALL_SITE_FN}`, above the \
         run and beside the other refusals — not inside a helper whose position \
         a later reader would have to go and check"
    );

    // And nowhere under src/ constructs it inside the iteration loop's scope.
    for file in &files {
        for (line_number, line) in executable_lines(file) {
            if line.contains(&marker) {
                assert!(
                    !inside_label_scope(&file.1, ITERATION_LOOP_LABEL, *line_number),
                    "{}:{line_number} constructs the decomposition capability \
                     INSIDE the iteration loop. That is a run taking a new goal \
                     from an artifact created during its own run, which removes \
                     every bound the run has (T-21-23)",
                    file.0
                );
            }
        }
    }
}

#[test]
fn the_loop_scope_scanner_reports_a_construction_inside_a_label_and_not_one_above_it() {
    // The guard-of-the-guard, **both directions in one test**, so the assertion
    // above cannot be satisfied by a scanner that reports nothing. Synthetic
    // source rather than lines read from the tree, so it keeps proving the
    // scanner works once — especially once — the tree is correct.
    let lines: Vec<(usize, String)> = [
        "fn drive() {",
        "    let capability = GoalDecomposition::from_argv_goal(&args);",
        "    'iterations: loop {",
        "        let sneaky = GoalDecomposition::from_argv_goal(&args);",
        "        if done {",
        "            break 'iterations;",
        "        }",
        "    }",
        "    let after = GoalDecomposition::from_argv_goal(&args);",
        "}",
    ]
    .iter()
    .enumerate()
    .map(|(index, line)| (index + 1, line.to_string()))
    .collect();

    assert!(
        inside_label_scope(&lines, ITERATION_LOOP_LABEL, 4),
        "a construction INSIDE the loop label's scope must be reported, or the \
         guard above is a no-op that passes forever"
    );
    assert!(
        !inside_label_scope(&lines, ITERATION_LOOP_LABEL, 2),
        "a construction ABOVE the loop must NOT be reported — the sanctioned \
         call site is exactly there, and a scanner that reported it would make \
         the property unsatisfiable"
    );
    assert!(
        !inside_label_scope(&lines, ITERATION_LOOP_LABEL, 9),
        "a construction BELOW the closed loop must not be reported either; a \
         line-number comparison rather than brace counting would get this wrong"
    );

    // And the enclosing-function finder, on the same synthetic text.
    assert_eq!(enclosing_fn(&lines, 2).as_deref(), Some("drive"));
    assert_eq!(fn_name_on("pub async fn decompose(").as_deref(), Some("decompose"));
    assert_eq!(fn_name_on("    let x = 1;"), None);
}

// ============================================================================
// GUARD FIVE: the seam count is two, and a third is a test failure with a name
//
// **Exactly two seams, and the count is a property of the design rather than a
// coincidence**: goal decomposition, once, above the loop; and ambiguity
// escalation, at the router's no-rule branch. There is deliberately no third for
// error recovery — an error the deterministic rules cannot classify is a park,
// not a prompt.
//
// The diff fails in BOTH directions, and the second direction is the one that
// rots quietly: a third site is a violation, and an allowlisted site that no
// longer spawns a seam is equally one, because the allowlist is then wider than
// the truth it describes (T-20-17).
//
// The entries are `path::enclosing_fn` rather than bare paths, because both
// sanctioned seams live in the same file — a per-file allowlist would report one
// entry where two seams exist and would go on passing if a third appeared beside
// them.
// ============================================================================

/// The call that spawns a model seam. Every site is diffed against the list
/// below; the function's own definition is excluded by name.
const SEAM_CALL: &str = "consult_model_seam(";
const SEAM_DEFINITION: &str = "async fn consult_model_seam(";

/// The two sanctioned seam sites, as `path::enclosing_fn`.
const SEAM_SITES: &[&str] = &[
    // The goal decomposition: once, above the loop, consuming a capability that
    // makes a second one a compile error.
    "src/driver/run.rs::decompose",
    // The ambiguity escalation: at `router::Decision::NoRule`, the one state the
    // deterministic rule table does not cover.
    "src/driver/run.rs::execute_run",
];

#[test]
fn every_model_seam_spawn_site_in_src_is_one_of_exactly_two() {
    let files = source_files();

    let mut observed: Vec<String> = Vec::new();
    for file in &files {
        for (number, line) in executable_lines(file) {
            if !line.contains(SEAM_CALL) || line.contains(SEAM_DEFINITION) {
                continue;
            }
            let enclosing = enclosing_fn(&file.1, *number).unwrap_or_else(|| {
                panic!(
                    "{}:{number} spawns a model seam outside any function, which \
                     this audit cannot attribute",
                    file.0
                )
            });
            observed.push(format!("{}::{enclosing}", file.0));
        }
    }
    observed.sort();
    observed.dedup();
    assert!(
        !observed.is_empty(),
        "the scan found no model-seam spawn site at all under src/, so the diff \
         below would pass vacuously. If {SEAM_CALL:?} was renamed, re-point \
         SEAM_CALL in the same commit"
    );

    let mut allowed: Vec<String> = SEAM_SITES.iter().map(|site| site.to_string()).collect();
    allowed.sort();

    let extra: Vec<&String> = observed.iter().filter(|site| !allowed.contains(site)).collect();
    assert!(
        extra.is_empty(),
        "a THIRD model seam appeared. The count is two by design: decomposition \
         once above the loop, and ambiguity only where the router returns \
         `router_no_rule`. An error the deterministic rules cannot classify is a \
         PARK, not a prompt — a recovery consultation is the third seam this \
         guard exists to refuse. Unexpected: {extra:?}"
    );

    let stale: Vec<&String> = allowed.iter().filter(|site| !observed.contains(site)).collect();
    assert!(
        stale.is_empty(),
        "an allowlisted seam site no longer spawns a seam, so the allowlist is \
         now wider than the truth it describes — an allowlist wider than the \
         truth is the failure that shape exists to catch (T-20-17). If a seam \
         was removed on purpose, remove its entry in the same commit. Stale: \
         {stale:?}"
    );
}

// ============================================================================
// The ambiguity seam's order of operations
//
// The arm's order IS the design: budget, then spawn, then re-parse, then verb.
// Each step is load-bearing in a different direction, and swapping any adjacent
// pair produces a build that still compiles and still passes every behavioural
// test written against the pieces:
//
// * budget AFTER spawn is a cap that reports a consultation which already
//   happened — the tokens are spent and the control is a log line;
// * re-parse AFTER verb is a command string assembled from an unvalidated
//   action, which is SAFE-08 inverted.
//
// A behavioural test cannot see the order — it sees only the outcome — so the
// order is pinned here, over the source, where it is visible.
// ============================================================================

/// The four markers whose relative order in the no-rule arm is the design.
const SEAM_ARM_ORDER: &[&str] = &[
    "budget.permit_consultation()",
    "consult_model_seam(",
    "escalated_action(",
    ".command_for(",
];

/// The marker opening the arm whose order is pinned.
const NO_RULE_ARM: &str = "router::Decision::NoRule { observed } =>";

#[test]
fn the_ambiguity_seam_asks_the_budget_then_spawns_then_reparses_then_builds_a_command() {
    let files = source_files();
    let run = files
        .iter()
        .find(|(path, _)| path == DECOMPOSITION_HOME)
        .unwrap_or_else(|| panic!("{DECOMPOSITION_HOME} must exist"));

    let arm_start = executable_lines(run)
        .find(|(_, line)| line.contains(NO_RULE_ARM))
        .map(|(number, _)| *number)
        .unwrap_or_else(|| {
            panic!(
                "the no-rule arm could not be located by its opening marker \
                 {NO_RULE_ARM:?}, so this audit is checking nothing. If the arm \
                 was reshaped, re-point NO_RULE_ARM in the same commit"
            )
        });

    let mut previous = arm_start;
    for marker in SEAM_ARM_ORDER {
        let at = executable_lines(run)
            .find(|(number, line)| *number > arm_start && line.contains(marker))
            .map(|(number, _)| *number)
            .unwrap_or_else(|| {
                panic!(
                    "the no-rule arm no longer contains {marker:?} after line \
                     {arm_start}. Every step of budget → spawn → re-parse → verb \
                     is load-bearing; a missing one is a step somebody removed"
                )
            });
        assert!(
            at >= previous,
            "the ambiguity seam's steps are out of order: {marker:?} appears at \
             line {at}, before a step that must precede it at line {previous}. \
             Budget BEFORE spawn, or the cap describes a consultation that has \
             already happened; re-parse BEFORE the command is built, or a command \
             string is assembled from an unvalidated action (SAFE-08)"
        );
        previous = at;
    }

    // And the arm contains no retry: nothing loops over the consultation, and
    // nothing calls the seam twice.
    let arm_text: String = executable_lines(run)
        .filter(|(number, _)| *number >= arm_start && *number <= previous)
        .map(|(_, line)| format!("{line}\n"))
        .collect();
    assert_eq!(
        arm_text.matches("consult_model_seam(").count(),
        1,
        "the ambiguity seam consults the model EXACTLY once per no-rule state. A \
         second call inside the arm is a retry, and retrying a model that has \
         just produced an invalid action is how a bounded seam becomes an \
         unbounded one. Arm:\n{arm_text}"
    );
}

#[test]
fn the_goal_escape_hatch_has_no_call_site_in_src() {
    let files = source_files();
    let hits = executable_hits(&files, DECOMPOSITION_ESCAPE_HATCH);

    let offenders: Vec<_> = hits
        .iter()
        .filter(|(path, _, _)| path != DECOMPOSITION_HOME)
        .cloned()
        .collect();
    assert!(
        offenders.is_empty(),
        "`{DECOMPOSITION_ESCAPE_HATCH}` builds the decomposition capability from \
         a string that did NOT come from a human's argv. A production call site \
         means the compiler is no longer what enforces the never-self-goal \
         property. Offending lines:{}",
        render(&offenders)
    );
    assert_eq!(
        hits.len(),
        1,
        "the escape hatch must appear on exactly one executable line — its own \
         definition in {DECOMPOSITION_HOME}. Found:{}",
        render(&hits)
    );
    assert!(
        hits[0].2.contains("pub fn"),
        "the single occurrence must be the `pub fn` definition, not a use of it. \
         Found:{}",
        render(&hits)
    );
}

#[test]
fn the_free_string_field_parser_distinguishes_payloads_from_map_keys() {
    // Guard four's control arm, over synthetic struct text, so the parser keeps
    // being proved correct once the tree is correct.
    let synthetic: SourceFile = (
        "src/synthetic.rs".to_string(),
        [
            "pub struct Sample {",
            "    pub status: String,",
            "    pub pause_context: Option<String>,",
            "    pub deferred: Vec<String>,",
            "    /// pub commented_out: String,",
            "    #[serde(default)]",
            "    pub keyed: HashMap<String, DiskInference>,",
            "    pub count: u32,",
            "    pub nested: Vec<RoadmapPhase>,",
            "}",
            "pub struct Beyond {",
            "    pub leaked: String,",
            "}",
        ]
        .iter()
        .enumerate()
        .map(|(index, line)| (index + 1, line.to_string()))
        .collect(),
    );

    let found = free_string_fields(&synthetic, "Sample");
    assert_eq!(
        found,
        vec![
            "Sample::deferred".to_string(),
            "Sample::pause_context".to_string(),
            "Sample::status".to_string(),
        ],
        "the parser must find String, Option<String> and Vec<String> — the three \
         shapes that carry third-party TEXT — and must not find a HashMap whose \
         String is a key the reader generates, a non-string field, a commented \
         declaration, or a field declared in a struct beyond the closing brace"
    );
}

// ============================================================================
// GUARD SIX: one stamped terminal write, and nothing else writes a run's ending
//
// The property: **every production path that closes a run out records how many
// model consultations it spent.** `escalations_used`'s own doc
// (`src/journal/mod.rs`) says the value is written unconditionally, so that a
// reader can tell "this run spent none" apart from "this record came from a
// build that predates the counter" — and three paths used to reintroduce that
// ambiguity on exactly the runs an operator investigates: the terminate-signal
// shutdown, the startup kill, and the spawn-failure arm.
//
// A test can assert the value on three known paths; it cannot assert that a
// FOURTH path added next year will carry it. What can be checked is the shape:
// one function writes a terminal label, so a new terminal path inherits the
// stamp instead of having to remember it. That is the same single-call-site
// technique `DrivableProject::from_registry` and the decomposition capability
// already use, and the reason the plan chose it over a comment.
//
// **The scan covers every file under `src/`, and the widening is review-WR-01.**
// It used to be pinned to `src/driver/run.rs` alone by a single file constant,
// which read as exact and was not: `JournalRun::finish` is a `pub fn` on a `pub
// struct`, so a terminal write added from `src/driver/kill.rs`, `src/app.rs` or
// any UI screen was outside this guard's view entirely — and it would have
// written `escalations_used: null` on precisely the killed and failed-to-spawn
// runs an operator opens an investigation with, with the guard green. That was a
// SILENT UNDER-DETECTION, which is the opposite of the direction the paragraph
// that used to sit here claimed for the guard as a whole. The sanctioned site is
// now a declared `(file, fn)` pair on `TERMINAL_WRITE_ALLOWLIST`, in the register
// `SPAWN_ALLOWLIST` and the seam-site guard already use.
//
// **What is still approximate, each named with the direction it fails in.** They
// are listed one at a time rather than summarised, because a summary is exactly
// how the old blanket claim came to cover a limit that was not true of it. No
// claim is made about this guard's failure direction *as a whole*: it has two.
//
// 1. The production/test boundary is found by a LINE MARKER (`mod tests {` at
//    column zero), not by parsing. A file that spelled its test module
//    differently would be scanned in full, and its tests' own journal closes
//    would be reported as production offenders. **Over-detection — loud**: the
//    failure arrives as a named line a reader can look at.
// 2. `executable_lines`'s comment filter handles LINE comments only (IN-03): a
//    terminal write inside a `/* … */` block is counted as executable.
//    **Over-detection — loud**, for the same reason.
// 3. The scan matches TWO call spellings — the method call `.finish(` and the
//    fully-qualified `JournalRun::finish(`. A third spelling nobody anticipated
//    is not matched, and NO assertion fires when that happens. **Under-detection
//    — silent.** This is the one limit here that fails the quiet way, and it is
//    written down rather than left to be rediscovered. What bounds it is the
//    non-vacuity block below: a rename that emptied the scan trips
//    `helper_writes`, the declaration count and the distinct-file count, so the
//    guard can go blind only to a spelling ADDED beside a surviving one.
// 4. The boundary of limit 1 excludes everything from the `mod tests {` marker
//    to **END OF FILE** (`test_region_start(file).unwrap_or(usize::MAX)`, then
//    `*number >= boundary -> continue`), not merely the test module. A
//    production terminal write placed after a file's marker is not scanned.
//    **Under-detection — silent**, and this limit was LIVE rather than
//    theoretical: `src/state_reader/mod.rs` carried a production
//    `pub fn count_backlog_items` 219 lines past its marker, invisible here
//    while clippy's `items_after_test_module` reported it independently
//    (review-WR-01). It is now BOUNDED by
//    `no_production_item_follows_a_test_module_marker`, which fails loudly on
//    any column-zero item after any file's marker — so the skipped region is
//    provably empty rather than merely assumed to be.
// 5. `enclosing_fn` attributes a line to the nearest PRECEDING `fn`, with no
//    brace tracking. A terminal write sitting between the end of
//    `fn finish_run(`'s body and the next declaration is attributed to
//    `finish_run` and therefore allowlisted. **Under-detection — silent**, and
//    it is named rather than fixed: a brace-tracking parser is out of
//    proportion to the risk here. It is partially bounded by the tree-wide
//    declaration count below, which fails if a second helper appears, and by
//    limit 4's bound now that the post-marker region is known empty.
//
// This repository has already paid once for a guard that read as exact while
// being quietly approximate. It is not paying again for one that claims a
// failure direction it does not have.
// ============================================================================

/// The two spellings a journal terminal write takes, and the one helper allowed
/// to make one.
///
/// **Two needles rather than one, and `21-REVIEW.md` is why.** The method-call
/// form was the only one matched, so `JournalRun::finish(&mut journal, label)` —
/// legal Rust, identical effect — walked past the guard. A third spelling is
/// still unmatched; limit 3 in the header above says so in those words rather
/// than leaving it to be discovered.
const TERMINAL_WRITE_CALL: &str = ".finish(";
const TERMINAL_WRITE_UFCS_CALL: &str = "JournalRun::finish(";
const TERMINAL_WRITE_HELPER: &str = "finish_run";

/// Every `(file, enclosing fn)` pair under `src/` permitted to write a run's
/// ending.
///
/// **This is a declared allowlist, not a habit** — `SPAWN_ALLOWLIST`'s rule
/// (`:32-37`), applied to the other property this file audits by single call
/// site. A `(file, fn)` pair rather than a bare path, for the reason the
/// seam-site guard gives at `:1438-1441`: a per-file entry would forgive every
/// function in that file, and the whole property is that ONE function closes a
/// run out.
///
/// Adding an entry here is the deliberate edit that is the point. A second
/// stamped helper is a second way to close a run out, and a run closed out
/// somewhere nobody had to think about is a run whose consultation count is
/// `null` on the record an operator is auditing (WR-02, DRIVE-04).
const TERMINAL_WRITE_ALLOWLIST: &[(&str, &str)] = &[
    // The one stamped helper: it sets `escalations_used` and then finishes the
    // journal, in that order, so every terminal path in the driver inherits the
    // stamp instead of having to remember it.
    ("src/driver/run.rs", TERMINAL_WRITE_HELPER),
];

/// Whether `line` writes a run's ending, in either spelling the scan matches.
fn writes_terminal(line: &str) -> bool {
    line.contains(TERMINAL_WRITE_CALL) || line.contains(TERMINAL_WRITE_UFCS_CALL)
}

/// The line that opens an in-module test region, matched at column zero.
const TEST_REGION_MARKER: &str = "mod tests {";

/// The 1-based number of the first line that opens `file`'s in-module test
/// region, or `None` when it has none.
///
/// Column-zero only, so a nested `mod tests {` inside another module — indented
/// in this tree's style — does not end the production region early.
fn test_region_start(file: &SourceFile) -> Option<usize> {
    file.1
        .iter()
        .find(|(_, line)| line.starts_with(TEST_REGION_MARKER))
        .map(|(number, _)| *number)
}

/// Every production terminal write under `src/`, as `(path, line, enclosing fn)`.
///
/// **The whole tree, one file at a time, each with its OWN production/test
/// boundary.** A file with no column-zero test-region marker is scanned in full,
/// which is the right answer for a file that has no in-module tests — the marker
/// is how a file declares where its tests begin, and a file that declares none
/// has none to exclude.
///
/// Shared with the control arm below, so what the control arm proves is what this
/// guard actually runs rather than a second copy that can drift from it.
fn terminal_write_hits(files: &[SourceFile]) -> Vec<(String, usize, Option<String>, String)> {
    let mut hits = Vec::new();
    for file in files {
        let boundary = test_region_start(file).unwrap_or(usize::MAX);
        for (number, line) in executable_lines(file) {
            if *number >= boundary || !writes_terminal(line) {
                continue;
            }
            hits.push((
                file.0.clone(),
                *number,
                enclosing_fn(&file.1, *number),
                line.trim().to_string(),
            ));
        }
    }
    hits
}

#[test]
fn every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper() {
    let files = source_files();

    // The boundary must survive in the allowlisted file SPECIFICALLY. Everywhere
    // else a missing marker means "no in-module tests" and scanning in full is
    // correct; here it would mean silently scanning `run.rs`'s own tests, which
    // close journals of their own and would flood this guard with offenders.
    for (path, _) in TERMINAL_WRITE_ALLOWLIST {
        let home = files
            .iter()
            .find(|(candidate, _)| candidate == path)
            .unwrap_or_else(|| panic!("{path} is on TERMINAL_WRITE_ALLOWLIST and must exist"));
        test_region_start(home).unwrap_or_else(|| {
            panic!(
                "{path} has no line beginning {TEST_REGION_MARKER:?} at column zero, \
                 so this guard cannot tell its production region from its tests and \
                 would scan the whole file. If the test module was renamed or moved, \
                 re-point TEST_REGION_MARKER in the same commit"
            )
        });
    }

    let hits = terminal_write_hits(&files);

    let allowed = |path: &str, enclosing: Option<&str>| -> bool {
        enclosing.is_some_and(|name| {
            TERMINAL_WRITE_ALLOWLIST
                .iter()
                .any(|(file, function)| *file == path && *function == name)
        })
    };

    let offenders: Vec<(String, usize, String)> = hits
        .iter()
        .filter(|(path, _, enclosing, _)| !allowed(path, enclosing.as_deref()))
        .map(|(path, number, _, line)| (path.clone(), *number, line.clone()))
        .collect();
    assert!(
        offenders.is_empty(),
        "a terminal write in a production region under src/ does not go through \
         `{TERMINAL_WRITE_HELPER}`. That helper stamps the run's model consultation \
         count before it finishes the journal, and it is the ONLY place a run's \
         ending is written so that a new terminal path inherits the stamp rather \
         than having to remember it. A bare `{TERMINAL_WRITE_CALL}` — or the \
         fully-qualified `{TERMINAL_WRITE_UFCS_CALL}` — writes `escalations_used: \
         null` on a run that spent consultations, which reads identically to a \
         record from a build that predates the counter, on exactly the killed and \
         failed-to-spawn runs a reader is auditing (WR-02, DRIVE-04). \
         `JournalRun::finish` is `pub` on a `pub struct`, so this is reachable from \
         any module in the tree and not only from the driver. Offending lines:{}",
        render(&offenders)
    );

    // --- non-vacuity, re-tuned to the widened scope -------------------------
    //
    // A scan widened without re-tuning its control assertions passes for the old,
    // narrow reason and has been widened in name only. Each assertion below is
    // now a statement about the whole tree, except the one that is deliberately
    // not — and that one says so.

    let helper_writes = hits.len();
    assert_eq!(
        helper_writes, 1,
        "the tree must contain exactly ONE production terminal write, and the scan \
         must have found it. {helper_writes} is either a second way to close a run \
         out or a scan that is checking nothing — if the call was renamed, re-point \
         `{TERMINAL_WRITE_CALL}` / `{TERMINAL_WRITE_UFCS_CALL}` in the same commit"
    );

    // The property WR-01 says was unchecked, stated directly: the whole tree
    // contributes terminal writes from exactly one file. The old file-scoped scan
    // could not make this assertion, because it had already assumed it.
    let mut contributing: Vec<&String> = hits.iter().map(|(path, _, _, _)| path).collect();
    contributing.sort();
    contributing.dedup();
    assert_eq!(
        contributing.len(),
        1,
        "exactly one file under src/ may contribute a production terminal write. \
         Found {contributing:?} — a second file closing runs out is the hole the \
         old, file-scoped scan could not see"
    );

    // The allowlist must not be wider than the truth it describes — the seam-site
    // guard's second direction (`:1501-1509`, T-20-17), applied here.
    let stale: Vec<&(&str, &str)> = TERMINAL_WRITE_ALLOWLIST
        .iter()
        .filter(|(file, function)| {
            !hits.iter().any(|(path, _, enclosing, _)| {
                path == file && enclosing.as_deref() == Some(*function)
            })
        })
        .collect();
    assert!(
        stale.is_empty(),
        "an allowlisted terminal-write site no longer writes a run's ending, so the \
         allowlist is now wider than the truth it describes. If the helper was \
         removed or renamed on purpose, remove its entry in the same commit. \
         Stale: {stale:?}"
    );

    // The declaration count is now TREE-WIDE, so a second stamped helper defined
    // in another file is caught rather than being invisible for the same reason
    // the writes themselves used to be.
    let declaration = format!("fn {TERMINAL_WRITE_HELPER}(");
    let mut declarations: Vec<(String, usize, String)> = Vec::new();
    for file in &files {
        let boundary = test_region_start(file).unwrap_or(usize::MAX);
        for (number, line) in executable_lines(file) {
            if *number < boundary && line.contains(&declaration) {
                declarations.push((file.0.clone(), *number, line.trim().to_string()));
            }
        }
    }
    assert_eq!(
        declarations.len(),
        1,
        "there must be exactly one stamped terminal-write helper anywhere under \
         src/; a second is a second way to close a run out, and the whole property \
         is that there is one. Found:{}",
        render(&declarations)
    );

    // And the call count stays scoped to the allowlisted file ON PURPOSE: the four
    // production terminal paths — the normal end, the terminate-signal shutdown,
    // the startup kill and the spawn failure — all live in the driver's run
    // module, so a tree-wide count here would weaken a number that is exact.
    for (path, function) in TERMINAL_WRITE_ALLOWLIST {
        if *function != TERMINAL_WRITE_HELPER {
            continue;
        }
        let home = files
            .iter()
            .find(|(candidate, _)| candidate == path)
            .unwrap_or_else(|| panic!("{path} must exist"));
        let boundary = test_region_start(home).unwrap_or(usize::MAX);
        let uses = executable_lines(home)
            .filter(|(number, line)| {
                *number < boundary
                    && line.contains(&format!("{TERMINAL_WRITE_HELPER}("))
                    && !line.contains(&format!("fn {TERMINAL_WRITE_HELPER}("))
            })
            .count();
        assert!(
            uses >= 4,
            "the driver has four production terminal paths — the normal end, the \
             terminate-signal shutdown, the startup kill and the spawn failure — \
             and only {uses} in {path} call the stamped helper. A path that stopped \
             calling it is a path that stopped recording the count"
        );
    }
}

/// The token list that makes a column-zero line an ITEM declaration.
///
/// Explicit rather than a regex, so what the assertion below can and cannot see
/// is readable in one place. `#[` is included because an attribute at column
/// zero introduces the item on the following line.
const ITEM_OPENERS: &[&str] = &[
    "pub ",
    // `line.starts_with("pub ")` cannot match `pub(crate) fn` or `pub(super) fn`
    // — the tree's DOMINANT restricted-visibility spelling, with 51 such items at
    // column zero. The old header claimed they were covered by a bound about
    // where the tree declares its items, which was backwards: all 51 sit at
    // column zero already and the scan simply could not see them (pass-5 WR-01).
    "pub(",
    "fn ",
    "async ",
    "const ",
    "static ",
    "struct ",
    "enum ",
    "trait ",
    "impl ",
    "mod ",
    "type ",
    "use ",
    "macro_rules!",
    "#[",
];

/// **The bound guard six's limit 4 names, and guard eight's limit 4.**
///
/// Both guards find a file's production/test boundary with a column-zero
/// `mod tests {` marker and then skip **from the marker to END OF FILE**
/// (`test_region_start(file).unwrap_or(usize::MAX)`, then `*number >= boundary
/// -> continue`). That skip is an approximation, and it fails the quiet way: a
/// production item placed after the marker is invisible to every scan in this
/// file, and a green result would then be a statement about nothing.
///
/// The region was not hypothetical. `src/state_reader/mod.rs` carried its marker
/// at :311 and a production `pub fn count_backlog_items` at :530 — 219 lines
/// into the blind region, with a real caller — and clippy's
/// `items_after_test_module` corroborated it independently while this guard said
/// nothing. Roughly 37% of `src/`'s lines sit past a marker.
///
/// **The only way a skipped region can be trusted is if it is empty**, so this
/// asserts exactly that. It converts a SILENT UNDER-DETECTION into a LOUD
/// OVER-DETECTION: the failure arrives as a `path:line: text` a reader can look
/// at, and it is deliberately over-eager — a legitimate future post-marker item
/// means deleting this assertion **consciously**, in a commit that says why,
/// rather than discovering years later that a guard had a blind spot.
///
/// **What it does NOT see, in the register this file uses.** Two shapes, both
/// under-detection and both silent:
///
/// 1. An item that is INDENTED — nested inside a post-marker `mod`, say — rather
///    than introduced at column zero.
/// 2. An item whose first token is outside [`ITEM_OPENERS`].
///
/// The previous version of this paragraph bounded both by asserting where the
/// tree declares its items, which is not a bound on either: gap 2 was live and at
/// column zero when that sentence was written — 51 `pub(crate) ` and `pub(super) `
/// items that `starts_with("pub ")` could not match — while the header called the
/// skipped region "provably empty" (pass-5 WR-01, the third consecutive round to
/// ship an overclaiming header). What bounds the two remaining gaps is the
/// scanned-files non-vacuity floor below plus the
/// synthetic control beside this test, which plants a `pub(crate) fn` and a
/// `pub(super) const` after a marker and demands that **the same collection
/// function this assertion calls** reports them. A sentence is not a bound; a
/// control that plants an offender and fails when it is missed is.
/// Every column-zero item declaration that follows a file's `mod tests {`
/// marker, **and the number of files the scan actually looked at**.
///
/// **One function, two consumers, and the pairing in the return type is
/// load-bearing.** The live assertion below and the planted-offender control
/// beside it both call this, so the control is a witness for the scan that runs
/// rather than for a re-implementation that could only ever agree with itself —
/// which is the "green about a region it never read" shape this whole test
/// exists to close.
///
/// `scanned_files` is returned alongside the offenders because it is the live
/// assertion's ONLY non-vacuity floor: if [`TEST_REGION_MARKER`] were ever
/// re-spelled, every file would fall out of the scan, the offender list would be
/// empty forever, and the emptiness claim would hold for exactly the wrong
/// reason. An extraction that returned only the offenders would delete that
/// protection while leaving the live assertion passing — it would pass
/// *precisely when* the floor was gone.
fn post_marker_offenders(files: &[SourceFile]) -> (Vec<(String, usize, String)>, usize) {
    let mut offenders: Vec<(String, usize, String)> = Vec::new();
    let mut scanned_files = 0usize;

    for file in files {
        let Some(marker) = test_region_start(file) else {
            continue;
        };
        scanned_files += 1;
        for (number, line) in &file.1 {
            // The marker line itself opens the test region; it is not an
            // offender, and neither is anything before it.
            if *number <= marker {
                continue;
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            if ITEM_OPENERS.iter().any(|token| line.starts_with(token)) {
                offenders.push((file.0.clone(), *number, line.trim().to_string()));
            }
        }
    }

    (offenders, scanned_files)
}

/// **The control arm: a planted post-marker item must be REPORTED.**
///
/// It calls [`post_marker_offenders`] — the same function
/// [`no_production_item_follows_a_test_module_marker`] consumes — so what it
/// witnesses is the live scan. Both planted shapes are the ones the scan was
/// blind to before the restricted-visibility opener joined [`ITEM_OPENERS`]:
/// `pub(crate)` and `pub(super)`, the tree's dominant spellings, 51 of them
/// at column zero while the header called the skipped region provably empty.
#[test]
fn the_boundary_self_check_sees_restricted_visibility_items() {
    let planted = synthetic_file(
        "src/planted.rs",
        &[
            "pub fn above_the_marker() {}",
            "mod tests {",
            "    fn a_test() {}",
            "}",
            "pub(crate) fn smuggled() {}",
            "pub(super) const X: u8 = 0;",
            "    pub(crate) fn indented_and_therefore_invisible() {}",
        ],
    );

    let (offenders, scanned_files) = post_marker_offenders(&[planted]);
    let reported: Vec<(usize, String)> = offenders
        .iter()
        .map(|(_, number, line)| (*number, line.clone()))
        .collect();
    assert_eq!(
        reported,
        vec![
            (5, "pub(crate) fn smuggled() {}".to_string()),
            (6, "pub(super) const X: u8 = 0;".to_string()),
        ],
        "the scan must report BOTH restricted-visibility items planted after \
         the marker. `pub fn above_the_marker` is before it and is not an \
         offender; the INDENTED item on the last line is limit 1 — silent \
         under-detection, named in the header rather than fixed, and asserted \
         here so the limit is a measured fact rather than a guess. Got: \
         {offenders:?}"
    );
    assert_eq!(
        scanned_files, 1,
        "the planted file declares a marker, so the scan must count it — the \
         count is the live assertion's only non-vacuity floor and this proves \
         it is computed rather than defaulted"
    );
}

#[test]
fn no_production_item_follows_a_test_module_marker() {
    let files = source_files();

    let (offenders, scanned_files) = post_marker_offenders(&files);

    assert!(
        offenders.is_empty(),
        "a column-zero item declaration follows a file's `mod tests {{` marker. \
         Every scan in this file skips from that marker to END OF FILE, so an \
         item down there is invisible to guard six's terminal-write audit and to \
         guard eight's construction-site audit — both would report green about a \
         region they never read. Move the item ABOVE the marker (which also \
         clears clippy's `items_after_test_module`), or, if a post-marker item is \
         genuinely wanted, delete this assertion in the same commit and say what \
         the guards are giving up. Offending lines:{}",
        render(&offenders)
    );

    // Non-vacuity: an assertion over an empty set of files is satisfied by
    // finding nothing, which is exactly the failure mode this whole test exists
    // to close. If `TEST_REGION_MARKER` were ever re-spelled, every file would
    // fall out of the scan and the emptiness above would hold forever. The count
    // is carried OUT of `post_marker_offenders` rather than recomputed here, so
    // the extraction that gave the control arm a shared code path could not
    // quietly drop this floor along the way.
    assert!(
        scanned_files >= 10,
        "this tree has many files with in-module tests; only {scanned_files} \
         carried a `{TEST_REGION_MARKER}` marker, so the scan is looking at \
         almost nothing and its emptiness proves almost nothing"
    );
}

/// Build a `SourceFile` from a path and its literal lines.
fn synthetic_file(path: &str, lines: &[&str]) -> SourceFile {
    (
        path.to_string(),
        lines
            .iter()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect(),
    )
}

#[test]
fn the_terminal_write_scanner_reports_a_bare_call_and_not_the_helpers_own() {
    // Guard six's control arm, every direction in one test, over synthetic source
    // rather than lines read from the tree — so it keeps proving the scanner works
    // once, especially once, the tree is correct. An assertion on emptiness that
    // no scanner could ever populate is the Phase-20 defect this repository has
    // already paid for.
    //
    // **Two synthetic files, and both halves of the widening.** The scan now
    // covers every file under `src/` and matches two call spellings, so a control
    // arm over one file and one spelling would go on passing for the reason the
    // scan used to be narrow. The second file carries the UFCS spelling, which is
    // the exact pair — another file, another spelling — the old guard was blind
    // to on both counts.
    let home = synthetic_file(
        "src/driver/run.rs",
        &[
            "fn finish_run(journal: &mut JournalRun, label: &str, used: u32) -> Result<()> {",
            "    journal.set_escalations_used(used);",
            "    journal.finish(label)",
            "}",
            "async fn shutdown_on_terminate(journal: &mut JournalRun) {",
            "    if let Err(err) = journal.finish(&label) {",
            "        warn!(\"could not close\");",
            "    }",
            "}",
            "mod tests {",
            "    fn a_test() { journal.finish(\"succeeded\").expect(\"finish\"); }",
            "}",
        ],
    );
    // A second file, with NO test-region marker at all — scanned in full, which is
    // correct for a file that declares no in-module tests, and which is how a
    // terminal write in `src/app.rs` or a UI screen becomes visible.
    let elsewhere = synthetic_file(
        "src/elsewhere.rs",
        &[
            "fn tear_down(journal: &mut JournalRun) {",
            "    // JournalRun::finish(journal, \"aborted\") — a comment, never a write",
            "    let _ = JournalRun::finish(journal, \"aborted\");",
            "}",
        ],
    );

    let boundary = test_region_start(&home)
        .expect("the synthetic source declares a column-zero test module");
    assert_eq!(boundary, 10, "the production region ends at the test module");
    assert!(
        test_region_start(&elsewhere).is_none(),
        "the second fixture must declare NO test region, or it is not exercising \
         the scan-in-full path a file without in-module tests takes"
    );

    let attributed: Vec<(String, usize, Option<String>)> =
        terminal_write_hits(&[home.clone(), elsewhere.clone()])
            .into_iter()
            .map(|(path, number, enclosing, _)| (path, number, enclosing))
            .collect();

    assert_eq!(
        attributed,
        vec![
            (
                "src/driver/run.rs".to_string(),
                3,
                Some(TERMINAL_WRITE_HELPER.to_string())
            ),
            (
                "src/driver/run.rs".to_string(),
                6,
                Some("shutdown_on_terminate".to_string())
            ),
            (
                "src/elsewhere.rs".to_string(),
                3,
                Some("tear_down".to_string())
            ),
        ],
        "the scanner must attribute the helper's OWN write to the helper, a bare \
         write elsewhere in the same file to the function that made it, and a \
         UFCS write in a DIFFERENT file to that file's function. A scanner that \
         reported none of them would satisfy the emptiness assertion above forever \
         while auditing nothing; one that reported the helper's own write as an \
         offender would make the property unsatisfiable; and one that missed \
         either of the last two is the guard as it stood before WR-01"
    );

    // A write below the boundary is out of scope by design: the in-module tests
    // close journals of their own, and forgiving them by name would be an
    // allowlist where a region boundary is the honest answer.
    assert!(
        executable_lines(&home)
            .any(|(number, line)| *number > boundary && writes_terminal(line)),
        "the synthetic fixture must contain a test-region write, or the boundary \
         is doing nothing here"
    );
    assert!(
        !attributed
            .iter()
            .any(|(path, number, _)| path == "src/driver/run.rs" && *number > boundary),
        "and the scanner must not report it"
    );

    // The comment line in the second fixture spells the UFCS needle exactly and
    // must NOT be reported: limit 2 in the header above is about BLOCK comments,
    // and a line comment is filtered. That is what lets this file's own prose name
    // the spellings it forbids.
    assert!(
        !attributed
            .iter()
            .any(|(path, number, _)| path == "src/elsewhere.rs" && *number == 2),
        "a line comment naming the UFCS spelling is not a write, or no guard in \
         this file could document the token it matches on"
    );
}

// ---------------------------------------------------------------------------
// Guard seven: the arrival-evidence field stays evidence
// ---------------------------------------------------------------------------

/// The wire field a corpus test uses to prove third-party content arrived.
const EVIDENCE_FIELD: &str = "FIELD_OBSERVED_MARKERS";

/// The one module that may name it: where the schema declares it.
const EVIDENCE_FIELD_HOME: &str = "src/driver/goal.rs";

#[test]
fn the_arrival_evidence_field_is_named_only_where_the_schema_declares_it() {
    let files = source_files();
    let hits = executable_hits(&files, EVIDENCE_FIELD);

    // The guard-of-the-guard: a scanner that found nothing at all would report
    // "no offenders" forever, which is the shape `tests/driver_dry_run.rs:428`
    // calls a tripwire that has never been seen to fire.
    assert!(
        hits.iter().any(|(path, _, _)| path == EVIDENCE_FIELD_HOME),
        "the scan found no reference to `{EVIDENCE_FIELD}` in {EVIDENCE_FIELD_HOME} \
         at all, so the emptiness below is a fact about the scanner rather than \
         about the tree"
    );

    let offenders: Vec<_> = hits
        .iter()
        .filter(|(path, _, _)| path != EVIDENCE_FIELD_HOME)
        .cloned()
        .collect();
    assert!(
        offenders.is_empty(),
        "`{EVIDENCE_FIELD}` carries text a hostile repository file can choose — a \
         corpus fixture names its own marker, so a model can be induced to report \
         any marker at all. It exists so a TEST can prove arrival before claiming \
         the injection lost, and a production path that read it would be a control \
         the attacker writes (T-21-36). Move the read into \
         `tests/driver_injection_corpus.rs` or delete it. Offending lines:{}",
        render(&offenders)
    );
}

// ---------------------------------------------------------------------------
// Guard eight: `CommandSource` is built in exactly one place
// ---------------------------------------------------------------------------
//
// **This guard is the missing half of a design decision, not a new rule.**
// `driver::command_source` validates a blank `--command` and refuses it, and the
// renderer deliberately does NOT defend again — because two places answering one
// question are two places that can disagree about the answer. That reduction is
// sound only while `command_source` really is the single production constructor
// of [`driver::CommandSource`]. Until now that was a grep result somebody ran
// once; here it is a property a test enforces.
//
// **The name collision that shaped these needles is GONE, and that history is
// worth keeping.** `src/driver/run.rs:614` used to declare a SECOND, unrelated
// `enum CommandSource` (variants `Fixed(String)` and `Routed { target_phase }`).
// A scan for the bare type name would have reported that enum's own sites as
// offenders, and the only way to green the suite would have been to allowlist
// `run.rs` — exempting the very file whose collision made this delicate. So the
// needles were the fully-qualified variant spellings WITH an opening
// parenthesis, chosen precisely because `Command`/`Goal` did not exist on the
// other enum and its `Routed` was a struct variant spelled with a brace.
//
// 21-11 recorded the rename as accepted debt; 21-14 called it due, because a
// guard whose needles are shaped by a name collision is one refactor away from
// silent blindness. `run.rs`'s enum is now `IterationSource`, the tree declares
// exactly one `CommandSource`, and this guard asserts that single declaration
// (below) instead of carrying a watchdog for the day the dodge stopped working.
// The parenthesis suffix is now kept for construction-vs-brace CLARITY rather
// than for collision avoidance — it is no longer load-bearing.
//
// **What this scan cannot see, each named with the direction it fails in**,
// in the register guard six above establishes. No claim is made about this
// guard's failure direction as a whole; it has both.
//
// 1. A variant spelling nobody anticipated matches no needle, and no assertion
//    fires when that happens. **Under-detection — silent.** Two things bound
//    it: the PER-NEEDLE non-vacuity below — each of the three type-qualified
//    needles must match at least one allowlisted line tree-wide, and each
//    allowlisted site must be matched by all three — so a needle that goes
//    blind is a named failure; and the import assertion below, which makes the
//    cheapest evasion (importing the variants so they can be written bare)
//    loud. `Self::`-qualified construction WAS a live instance of this limit
//    and is now matched rather than merely named.
//
//    **This replaced a LINE COUNT, and the replacement is the point** (pass-5
//    warning 3). The bound used to be `contributed >= 3` per allowlisted
//    function — three attributed lines, on the reasoning that each site names
//    three variants one per line. That stops biting the moment a site grows a
//    fourth line naming any variant, which 21-15 did: a needle could then go
//    blind and the count would still clear 3. Counting DISTINCT needles cannot
//    be satisfied by a site that grew.
// 2. The three `Self::`-qualified needles may legitimately match ZERO sites,
//    and are therefore excluded from the per-needle floor (D-16-3). They exist
//    to CATCH an evasion spelling, not to be used: demanding that each match
//    something would demand writing the pattern this guard forbids.
//    **Under-detection — silent** for that spelling specifically, bounded by
//    the type-qualified triple, which any real construction site must also
//    name.
// 3. `enclosing_fn` finds the nearest preceding `fn` with no brace tracking, so
//    a line sitting between the end of `command_source`'s body and the next
//    declaration is attributed to `command_source` and allowlisted.
//    **Under-detection — silent**, named rather than fixed: a brace-tracking
//    parser is out of proportion here, exactly as in guard six's limit 5.
// 4. The production/test boundary is the shared marker approximation, and it
//    skips from the marker to END OF FILE. **Under-detection — silent**, and
//    now bounded tree-wide by `no_production_item_follows_a_test_module_marker`,
//    which proves the skipped region contains no items at all.
// 5. `executable_lines` filters LINE comments only, so a variant named inside a
//    `/* … */` block counts as executable. **Over-detection — loud**: the
//    failure arrives as a named line a reader can look at.
// 6. The three `Self::`-qualified needles name no TYPE, so they would also match
//    an unrelated enum that happens to have a `Command`, `Routed` or `Goal`
//    tuple variant and constructs it as `Self::` inside its own `impl`.
//    **Over-detection — loud**, and deliberately accepted in that direction: a
//    false offender is a line a reader dismisses in seconds, whereas the
//    under-detection the needles close (a `Self::Command(` construction that no
//    needle matched) is the silent kind. No such enum exists in the tree today —
//    the scan attributes its hits to `command_source` and `preview_text` in
//    `src/driver/mod.rs` and to `iteration_source` in `src/driver/run.rs`, none
//    via a `Self::` needle.

/// The variant spellings, as they are written when built or matched.
///
/// **Six needles, not three**: the `Self::`-qualified forms join the
/// type-qualified ones (review-WR-02, gap 1). `Self::Command(payload)` inside an
/// `impl CommandSource` block is legal Rust with identical effect and matched no
/// needle — the same hole the UFCS spelling opened in guard six, in a different
/// guard. Adding them is safe only AFTER the rename: while `run.rs` declared its
/// own `CommandSource`, a bare `Self::Routed(` could have matched that enum's
/// sites instead.
///
/// The trailing parenthesis is now a readability convention (it distinguishes a
/// tuple-variant construction from a struct-variant brace), not the collision
/// dodge it originally was. See the header above.
const COMMAND_SOURCE_VARIANTS: &[&str] = &[
    "CommandSource::Command(",
    "CommandSource::Routed(",
    "CommandSource::Goal(",
    "Self::Command(",
    "Self::Routed(",
    "Self::Goal(",
];

/// The three needles that any real construction or match site MUST name.
///
/// A subset of [`COMMAND_SOURCE_VARIANTS`], and the subset the non-vacuity
/// floors below are taken over. The `Self::`-qualified siblings are deliberately
/// out: they exist to CATCH an evasion spelling rather than to be used, so
/// requiring each to match would require writing the pattern this guard forbids
/// (D-16-3).
const COMMAND_SOURCE_TYPE_QUALIFIED: &[&str] = &[
    "CommandSource::Command(",
    "CommandSource::Routed(",
    "CommandSource::Goal(",
];

/// The two production functions permitted to name a `CommandSource` variant.
///
/// **A declared `(file, fn)` allowlist**, the same shape `TERMINAL_WRITE_ALLOWLIST`
/// and the seam-site guard use, and the shape `21-VERIFICATION.md` names as the
/// model for this class of check.
///
/// Which is which, and why both are sanctioned:
///
/// * `command_source` **constructs** them. It is the single production
///   constructor, and that is the whole property this guard exists to enforce.
/// * `preview_text` **matches** them. Its arms name the variants as patterns
///   rather than building them; the match is exhaustive with no wildcard, which
///   is what makes a fourth source a compile error at every consumer.
/// * `iteration_source` **matches** them too, and never builds one. It narrows
///   the argv-resolved value to the two shapes the run loop can execute
///   (`IterationSource`), refusing `Goal` — which reaches the run layer only if
///   the decomposition and approval layers were bypassed. It joined this list in
///   21-15, in the commit that moved `execute_run`'s source resolution above
///   every disk write.
///
/// No attempt is made to tell a construction from a pattern match textually. The
/// distinction is not needed — both functions are allowlisted BY NAME, and an
/// occurrence outside them is a finding whichever it is — and a heuristic that
/// tried would be one more approximation this file would then have to document.
const COMMAND_SOURCE_ALLOWLIST: &[(&str, &str)] = &[
    ("src/driver/mod.rs", "command_source"),
    ("src/driver/mod.rs", "preview_text"),
    ("src/driver/run.rs", "iteration_source"),
];

/// How many hits each allowlisted function must contribute: one per variant.
const COMMAND_SOURCE_VARIANT_COUNT: usize = 3;

#[test]
fn every_command_source_variant_is_named_only_where_it_is_built_or_matched() {
    let files = source_files();

    // The same per-file production/test boundary guard six uses. In-module tests
    // construct these variants freely and legitimately — `src/driver/mod.rs`'s own
    // tests build all three — and they are excluded BY CONSTRUCTION rather than by
    // an allowlist somebody has to maintain.
    let mut hits: Vec<(String, usize, Option<String>, String)> = Vec::new();
    for file in &files {
        let boundary = test_region_start(file).unwrap_or(usize::MAX);
        for (number, line) in executable_lines(file) {
            if *number >= boundary
                || !COMMAND_SOURCE_VARIANTS
                    .iter()
                    .any(|needle| line.contains(needle))
            {
                continue;
            }
            hits.push((
                file.0.clone(),
                *number,
                enclosing_fn(&file.1, *number),
                line.trim().to_string(),
            ));
        }
    }

    let offenders: Vec<(String, usize, String)> = hits
        .iter()
        .filter(|(path, _, enclosing, _)| {
            !enclosing.as_deref().is_some_and(|name| {
                COMMAND_SOURCE_ALLOWLIST
                    .iter()
                    .any(|(file, function)| *file == path && *function == name)
            })
        })
        .map(|(path, number, _, line)| (path.clone(), *number, line.clone()))
        .collect();
    assert!(
        offenders.is_empty(),
        "a production line names a `CommandSource` variant outside the two \
         sanctioned functions. What a third site costs: `command_source` validates \
         a blank `--command` and refuses it, and the renderer deliberately does NOT \
         defend again, because two places answering one question are two places \
         that can disagree. That reduction is sound ONLY while `command_source` is \
         the single production constructor — a second construction site is \
         review-CR-02 returning through a new spelling, with every behavioural test \
         still green. If the new site is a legitimate consumer that matches rather \
         than builds, add it to COMMAND_SOURCE_ALLOWLIST in the same commit and say \
         which it is. Offending lines:{}",
        render(&offenders)
    );

    // Both directions, in the register the seam-site guard establishes
    // (`:1501-1509`, T-20-17): an allowlisted site that no longer names a variant
    // means the allowlist is wider than the truth it describes.
    //
    // **DISTINCT NEEDLES, not attributed lines** (pass-5 warning 3). This used to
    // count lines — `contributed >= 3` per site, on the reasoning that each site
    // names three variants one per line — and that bound stops biting the moment
    // a site grows a fourth line naming any variant, which 21-15 did. A needle
    // could then go blind and the count would still clear three. Each site must
    // be matched by all THREE type-qualified needles, and each needle must match
    // somewhere: neither can be satisfied by a site that merely got longer.
    for (file, function) in COMMAND_SOURCE_ALLOWLIST {
        let matched: Vec<&&str> = COMMAND_SOURCE_TYPE_QUALIFIED
            .iter()
            .filter(|needle| {
                hits.iter().any(|(path, _, enclosing, line)| {
                    path == file
                        && enclosing.as_deref() == Some(*function)
                        && line.contains(**needle)
                })
            })
            .collect();
        assert_eq!(
            matched.len(),
            COMMAND_SOURCE_TYPE_QUALIFIED.len(),
            "{file}::{function} is allowlisted as a site that names all \
             {COMMAND_SOURCE_VARIANT_COUNT} `CommandSource` variants, but only \
             {matched:?} of {COMMAND_SOURCE_TYPE_QUALIFIED:?} match a line \
             attributed to it. Either the allowlist is now wider than the truth \
             it describes — the site stopped naming them, and the entry should go \
             in the same commit — or a needle stopped matching and this guard is \
             auditing less than it claims."
        );
    }

    // And the same property from the needle's side: a needle matching nothing
    // ANYWHERE is a needle that has gone blind, whatever the per-site counts say.
    // The `Self::`-qualified needles are excluded by D-16-3 — they exist to catch
    // an evasion spelling, and demanding a match would demand writing the pattern
    // this guard forbids.
    for needle in COMMAND_SOURCE_TYPE_QUALIFIED {
        let matches = hits.iter().filter(|(_, _, _, line)| line.contains(needle)).count();
        assert!(
            matches >= 1,
            "the needle {needle:?} matches no production line at all. A needle \
             that matches nothing audits nothing, and every emptiness assertion \
             it participates in above holds for the wrong reason. Needles: \
             {COMMAND_SOURCE_VARIANTS:?}"
        );
    }

    // **One name, one type — the collision watchdog's replacement.** The old
    // assertion here checked that the needles had not begun matching
    // `src/driver/run.rs`'s second, unrelated `enum CommandSource`; it was a
    // watchdog for a dodge. 21-14 removed the thing being dodged (that enum is
    // now `IterationSource`), so the property can be asserted directly and
    // positively: the tree declares `enum CommandSource` exactly once.
    //
    // A second declaration anywhere is a loud failure naming its file, because
    // two types with one name is what made these needles delicate in the first
    // place — and the fix is the rename, never an allowlist entry for the
    // colliding file.
    let declaration = "enum CommandSource";
    let mut declarations: Vec<(String, usize, String)> = Vec::new();
    for file in &files {
        let boundary = test_region_start(file).unwrap_or(usize::MAX);
        for (number, line) in executable_lines(file) {
            if *number < boundary && line.contains(declaration) {
                declarations.push((file.0.clone(), *number, line.trim().to_string()));
            }
        }
    }
    assert_eq!(
        declarations.len(),
        1,
        "exactly one `enum CommandSource` may be declared under src/ — the \
         argv-resolution enum in src/driver/mod.rs. A second type with the same \
         name is the debt 21-11 accepted and 21-14 paid off: it forces every \
         guard that scans for the name to dodge by needle shape, and a needle \
         shaped by a collision goes blind one refactor later. Rename the new \
         type; do not allowlist its file. Found:{}",
        render(&declarations)
    );

    // And specifically that `run.rs`'s enum stayed renamed. The count above
    // would also pass if mod.rs's declaration vanished and run.rs's returned,
    // which is the one way to satisfy it while reintroducing the collision.
    let run_rs_declares: Vec<&(String, usize, String)> = declarations
        .iter()
        .filter(|(path, _, _)| path == "src/driver/run.rs")
        .collect();
    assert!(
        run_rs_declares.is_empty(),
        "src/driver/run.rs declares an `enum CommandSource` again. Its \
         per-iteration enum is `IterationSource`; the argv-resolution enum lives \
         in src/driver/mod.rs. Found: {run_rs_declares:?}"
    );

    // **The bare-spelling escape hatch, made loud (review-WR-02, gap 2).** Every
    // needle above is qualified, so `use crate::driver::CommandSource::Command;`
    // followed by a bare `Command(x)` would construct a variant that no needle
    // matches and no assertion notices — silent under-detection, and the
    // cheapest possible evasion of this entire guard. Importing the TYPE
    // (`use ...::CommandSource;`) is fine and common; importing a VARIANT PATH
    // is what this refuses.
    let mut variant_imports: Vec<(String, usize, String)> = Vec::new();
    for file in &files {
        let boundary = test_region_start(file).unwrap_or(usize::MAX);
        for (number, line) in executable_lines(file) {
            let trimmed = line.trim_start();
            if *number < boundary && trimmed.starts_with("use ") && line.contains("CommandSource::")
            {
                variant_imports.push((file.0.clone(), *number, line.trim().to_string()));
            }
        }
    }
    assert!(
        variant_imports.is_empty(),
        "a production line imports a `CommandSource` VARIANT path. Every needle \
         this guard scans for is qualified, so an imported variant can be written \
         bare — `Command(x)` — and construct a `CommandSource` that this guard \
         never sees, which is review-CR-02 returning through a spelling nobody \
         audits. Import the type, not its variants. Offending lines:{}",
        render(&variant_imports)
    );
}

// ---------------------------------------------------------------------------
// Guard nine: `DriveArgs` declares no raw argv `String` field
// ---------------------------------------------------------------------------
//
// **The guard four gap-closure cycles never had: one that reads the TYPE rather
// than a list.** Every cycle in this phase protected a hand-enumerated subset of
// the argv-derived values and every enumeration was exactly one item short —
// three `CommandSource` arms one at a time, then three of `DriveArgs`'s six
// string fields, with all three of the next round's Criticals landing in the
// other three. 21-15 moved the enumeration to the compiler: all six argv-derived
// string fields are `payload::NonBlank`, and `DriveArgs::from_argv`'s exhaustive
// destructure makes a seventh field a compile error until somebody classifies
// it.
//
// This is that compile-side bound's **textual witness**. The destructure forces
// a new field to be *handled*; it does not force it to be handled by giving it a
// payload type. A seventh field declared `pub goal_file: Option<String>` and
// dutifully copied across the destructure compiles fine and reintroduces the
// whole defect class. That is what this scan refuses.
//
// **What this scan SEES, what it does NOT, and what the floors actually bound.**
// Rewritten in round 6 because pass 6 measured the previous version of this
// block certifying bounds that no committed control measured — the inheritance
// vector this phase has now paid for four times. Every claim below names the
// control that goes red without it.
//
// **SEEN** — each with a planted-defect arm in
// `the_raw_argv_field_scanner_sees_every_measured_silent_spelling`, all of which
// exercise the SAME extracted fns the live assertion consumes:
//
// * `pub(crate)` / `pub(super)` field openers. That is the exact spelling
//   `ITEM_OPENERS` was widened for four commits later in this same file, and
//   pass 6 measured guard nine silent on both.
// * **A field declared with NO visibility modifier at all** — `goal_hint:
//   String,`. Round 7's addition, and the sharpest of the set: pass 7 measured
//   it silent to the scan AND to the floor, which share `is_field_opener`, so
//   the reading was `field_lines=12 protected=6 offenders=[]` — pass 6's exact
//   signature, in a spelling that appeared in neither of these two lists. Its
//   arm is `the_scanner_reports_a_bare_private_field`, committed red against the
//   unfixed opener before the opener was widened.
// * **A payload type riding beside an allowlisted `OsString`** — `pub
//   claude_args: (Vec<OsString>, String),`. Round 7's second addition. The
//   allowlist suppressed the whole judgment rather than the OsString-presence
//   report alone, and the integrity pin that exists to catch a repurposed entry
//   was a `contains`, which this type satisfies. Its arm is
//   `an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring`, also
//   committed red first; the pin is now an EQUALITY on the parsed declared type.
// * The str-family payload spellings — `Box<str>`, `Cow<'_, str>`,
//   `&'static str`, `&str` — none of which contains the `String` token.
// * `OsString`, **deny-by-default over the whole scanned body**: any field whose
//   type names it is reported unless its NAME is on the two-entry suppress-only
//   allowlist `OSSTRING_ALLOWED`. A seventh argv field carries a new name by
//   definition, so it is bound without anyone pre-listing it. The
//   false-positive direction has its own arm (`Vec<OsString>` and
//   `Option<PathBuf>` on the allowlisted names, asserted NOT reported), and the
//   allowlist's integrity is pinned in the live assertion against the real
//   declarations, so an entry cannot be silently repurposed.
// * Trailing `//` comments in BOTH positions — last field (the declaration never
//   terminated in `,`, so it was buffered and dropped) and mid-struct (the
//   buffer swallowed the FOLLOWING declaration, so the offender was
//   misattributed and the next field was never judged at all). Attribution is
//   asserted, not merely the count.
// * A declaration still buffered when the body ends, which is now flushed and
//   judged rather than dropped.
//
// **SILENT, with direction** — all under-detection, each bounded by its own
// assertion rather than by this sentence:
//
// 1. A field whose type hides behind a local `type` alias for `String` — say
//    `type Alias = String; pub alias: Alias,` — matches no token.
//    **Under-detection, silent.** Bounded by
//    `no_type_alias_hides_a_string_from_guard_nine`, which asserts on its own
//    line that `type .* = String` under `src/driver/` is zero.
// 2. A **macro-expanded** field declaration never appears in the source text at
//    all. **Under-detection, silent, and bounded by nothing in this file** —
//    stated rather than mitigated, because no textual scan can see it. What
//    bounds it is `from_argv`'s no-`..` destructure, which a macro-declared
//    field must still be named in.
// 3. The struct-body extraction is column-zero brace based, like every other
//    scan in this file. **Over- and under-detection**, per the shared marker
//    approximation; cross-referenced to
//    `no_production_item_follows_a_test_module_marker`, which bounds the related
//    region gap. The extraction's own control asserts by name that the region
//    never reached `RawDriveArgs`.
//
// **A RETIRED reliance, named because relying on it silently is how round 6
// shipped a false bound.** Until round 7, what actually stopped a bare
// (no-`pub`) argv field was not this guard at all: **thirteen integration crates
// build `DriveArgs { … }` as struct literals, and a private field breaks all
// thirteen at compile time.** That is a **coincidence of the fixture tree, not a
// property of the code** — delete or restructure those thirteen crates and the
// spelling compiles unprotected. `21-18-SUMMARY.md`'s named-shape row 15
// reported that coincidental bound RETIRED; pass 7 measured it retired for the
// two payload-type spellings and still load-bearing for this one, which is what
// let the previous version of this block certify a bound nothing asserted. It is
// no longer relied on in either direction: `is_field_opener` now sees the
// spelling, and `the_scanner_reports_a_bare_private_field` is the committed
// control that goes red if it stops seeing it. Nothing in this header claims a
// bound beyond the arms named in it.
//
// **What the FLOORS bound, stated as measured rather than as hoped.** The
// `>=10 field-declaration` and `>=6 NonBlank` assertions catch **extraction
// breakage and wholesale declaration-style drift** — a scanner that stopped
// matching this tree's style fails loudly instead of reporting a clean empty
// set. They provably do **NOT** catch a single added field: pass 6 measured
// `field_lines=12 protected=6 -> PASSES` with a planted `pub(crate)` offender
// sitting in the body, and that exact input is now the floor probe inside the
// control test. **The OFFENDER SCAN is what catches an added field**, which is
// why its blind spellings were the whole of the gap. Since round 6 the scan and
// the floor share one `is_field_opener`, so they cannot disagree about what a
// field declaration is the way pass 6 measured them disagreeing.
//
// The scan reads `DriveArgs` **only**. `RawDriveArgs` sits a few lines away and
// legitimately holds `String`s — it IS the raw side of the parse boundary, the
// shape argv supplies before anything has judged it — so the extraction control
// below asserts by name that the extracted region never reached it.

/// The `DriveArgs` field declarations whose type text names a bare `String`.
///
/// A pure function over source lines, so the live assertion and its
/// planted-defect control arms exercise **the same code path**. A control that
/// re-implemented the scan would witness only its agreement with itself.
///
/// Each returned entry is `(line number of the declaration's first line, the
/// joined declaration text)`.
fn raw_string_argv_fields(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    let body = drive_args_body(lines);
    let mut out = Vec::new();
    // A declaration may WRAP across lines — a long type, or rustfmt's doing —
    // and pass 5 recorded that names wrapping across two source lines defeat a
    // full-name grep. Lines are joined up to the trailing comma before the type
    // is judged, so a wrapped `Option<\n    String,\n>` cannot evade the needle.
    let mut pending: Option<(usize, String)> = None;
    for (number, line) in body {
        let trimmed = line.trim();
        if trimmed.starts_with("///") || trimmed.starts_with("//") || trimmed.starts_with("#[") {
            continue;
        }
        // The trailing-comment strip happens BEFORE the `,` test, which is what
        // makes a commented declaration terminate. Without it the comment kept
        // the text from ending in `,`, so a last field was buffered and dropped
        // and a mid-struct one swallowed the next declaration whole.
        let trimmed = without_trailing_comment(trimmed);
        let (start, joined) = match pending.take() {
            Some((start, acc)) => (start, format!("{acc} {trimmed}")),
            None => {
                if !is_field_opener(trimmed) || !trimmed.contains(':') {
                    continue;
                }
                (number, trimmed.to_string())
            }
        };
        if !joined.ends_with(',') {
            pending = Some((start, joined));
            continue;
        }
        judge_declaration(start, joined, &mut out);
    }
    // **Flush.** A declaration still buffered when the body ends is a real
    // declaration — the last-field case — and dropping it is silent
    // under-detection of exactly the shape a seventh field would take.
    if let Some((start, joined)) = pending.take() {
        judge_declaration(start, joined, &mut out);
    }
    out
}

/// Judge one joined declaration and push it if it carries a raw argv payload.
///
/// **An allowlisted name suppresses the `OsString`-PRESENCE report only, never a
/// payload-type report.** Until round 7 this fn `return`ed out of the `OsString`
/// branch, so `names_string_payload` never ran for any declaration naming
/// `OsString` — and `pub claude_args: (Vec<OsString>, String),` was therefore
/// silent, an allowlist entry repurposed to carry raw argv text while inheriting
/// its own suppression. Pass 7 measured it; the plant is
/// [`an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring`],
/// committed red against this fn before the branch was opened.
///
/// A declaration is reported at most once even when both rules would fire.
fn judge_declaration(start: usize, joined: String, out: &mut Vec<(usize, String)>) {
    let joined = joined.trim().to_string();
    let Some((_, type_text)) = joined.split_once(':') else {
        return;
    };
    // Deny-by-default on `OsString`, suppressed only by NAME (D-18-2).
    if names_token(type_text, "OsString") {
        let name = declared_field_name(&joined);
        if !OSSTRING_ALLOWED.contains(&name) {
            out.push((start, joined));
            return;
        }
        // Allowlisted: the OsString-presence report is suppressed, and the
        // payload judgment below still runs.
    }
    if names_string_payload(type_text) {
        out.push((start, joined));
    }
}

/// The TYPE a declaration declares, as text — trimmed, trailing comment and
/// trailing comma stripped.
///
/// Extracted so [`drive_args_declares_no_raw_argv_string_field`]'s allowlist
/// integrity pin can compare for EQUALITY rather than containment. Pass 7
/// measured the `contains` form passing over `pub claude_args: (Vec<OsString>,
/// String),` — the exact repurposing the pin exists to refuse contains its own
/// expected type as a substring, so containment can never see it.
fn declared_type_text(joined: &str) -> &str {
    let code = without_trailing_comment(joined.trim());
    let type_text = match code.split_once(':') {
        Some((_, right)) => right,
        None => return "",
    };
    type_text.trim().trim_end_matches(',').trim()
}

/// Every `(line number, line)` between `pub struct DriveArgs {` and the
/// column-zero `}` that closes it.
fn drive_args_body(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    let mut body = Vec::new();
    let mut inside = false;
    for (number, line) in lines {
        if !inside {
            if line.starts_with("pub struct DriveArgs {") {
                inside = true;
            }
            continue;
        }
        if line == "}" {
            break;
        }
        body.push((*number, line.clone()));
    }
    body
}

/// Whether `text` names `token` as a whole word.
///
/// Word-boundary semantics, so `OsString` does not match the `String` token
/// (the byte before is `s`, an identifier byte) and `PathBuf` matches nothing.
/// A substring test would report `claude_args: Vec<OsString>` as a `String`
/// offender and make the property unsatisfiable.
fn names_token(text: &str, token: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0usize;
    while let Some(found) = text[from..].find(token) {
        let start = from + found;
        let end = start + token.len();
        let before_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// Whether `text` names `String` as a whole word. Kept under its own name
/// because [`no_type_alias_hides_a_string_from_guard_nine`] asks exactly this
/// narrower question about `type` aliases.
fn names_bare_string(text: &str) -> bool {
    names_token(text, "String")
}

/// Whether `text` names a string PAYLOAD type in any of its spellings.
///
/// **Widened from `names_bare_string` because a `String` wearing a coat is the
/// same defect.** `Box<str>`, `Cow<'_, str>`, `&'static str` and `&str` all
/// carry unvalidated argv text and none of them contains the `String` token;
/// pass 6 measured all four silent. The `str` token covers every one of them —
/// it appears as a whole word inside `Box<str>` and not inside `OsString`,
/// where the neighbouring bytes are identifier bytes.
///
/// `OsString` is deliberately NOT folded in here: it is judged by name in
/// [`judge_declaration`], deny-by-default against [`OSSTRING_ALLOWED`], because
/// two fields legitimately carry it.
fn names_string_payload(text: &str) -> bool {
    names_token(text, "String") || names_token(text, "str")
}

/// The only field NAMES permitted to carry an `OsString` in `DriveArgs`.
///
/// **A suppress-only allowlist, and the direction is the decision (D-18-2).**
/// `OsString` is the type argv actually arrives in, so a payload field respelled
/// `OsString` is the same defect wearing the platform type — but `claude_args`
/// legitimately holds argv as `Vec<OsString>`. The rule is therefore
/// deny-by-default over the whole scanned body, suppressed BY NAME for these
/// two, rather than a protected-name rule: the threat this guard exists for is a
/// SEVENTH argv field, which by definition carries a name no pre-written list
/// contains, so a by-name protection rule would cover no case the threat model
/// names.
///
/// This list can only SUPPRESS, never widen. A new legitimate OsString-carrying
/// field goes RED until a human adds its name — over-reporting, loud, the safe
/// direction. And it cannot be silently repurposed: the live assertion pins each
/// entry to its legitimate type in the real body.
const OSSTRING_ALLOWED: [&str; 2] = ["claude_args", "claude_program"];

/// The field name a declaration declares — the last identifier before the `:`.
fn declared_field_name(joined: &str) -> &str {
    joined
        .split_once(':')
        .map(|(left, _)| left)
        .unwrap_or(joined)
        .split_whitespace()
        .next_back()
        .unwrap_or("")
}

/// Strip a trailing `// …` comment from a declaration line.
///
/// **Pass 6 measured both positions broken.** As the LAST field the comment kept
/// the joined text from ever ending in `,`, so the declaration was buffered and
/// dropped; MID-STRUCT the buffer swallowed the FOLLOWING declaration into its
/// text, so the offender was misattributed and the next field was never judged
/// at all. A simple split on `//` suffices here: these are declaration lines,
/// and the loop has already skipped comment-only lines.
fn without_trailing_comment(text: &str) -> &str {
    match text.split_once("//") {
        Some((code, _)) => code.trim_end(),
        None => text,
    }
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Whether `trimmed` opens a field declaration.
///
/// **Extracted so the offender scan and the non-vacuity floor cannot disagree
/// about what a field declaration is.** Pass 6 measured them disagreeing: the
/// floor counted twelve while the scan reported nothing, with a planted
/// `pub(crate)` offender in the body — two filters, one property, and the gap
/// between them was the whole of WR-02's floor half.
///
/// **Widened in round 7 to bare (private) declarations.** Pass 7 measured the
/// `pub `/`pub(` requirement letting `goal_hint: String,` — a field with no
/// visibility modifier at all — through both the scan and the floor, producing
/// pass 6's exact silent signature in a spelling neither of guard nine's lists
/// named. A private field is still a field, `from_argv` still has to destructure
/// it, and a `String` on it still carries unvalidated argv text.
///
/// **Over-detection direction, and why it is loud rather than silent.** A bare
/// identifier followed by `:` is also the shape of a match arm, a struct-literal
/// initialiser and a labelled loop. None of them is in scope here **by
/// construction**: [`raw_string_argv_fields`] runs only over
/// [`drive_args_body`]'s lines, which are the lines between `pub struct
/// DriveArgs {` and its column-zero `}`. Should that region ever widen, a
/// non-field line matching this opener would be **reported** — it would appear
/// in the offender list and fail the live assertion visibly — not pass silently.
/// The under-detection direction is the one this repository keeps paying for,
/// and this widening moves in the opposite direction.
fn is_field_opener(trimmed: &str) -> bool {
    if trimmed.starts_with("pub ") || trimmed.starts_with("pub(") {
        return true;
    }
    // A bare declaration: the first token is an identifier and the line reaches
    // a `:` before any `//`. Implemented with `chars()` rather than a regex —
    // this crate has no regex dependency and is not gaining one for a scanner.
    let code = without_trailing_comment(trimmed);
    let Some((left, _)) = code.split_once(':') else {
        return false;
    };
    let name = left.trim();
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// **The control arm, and it runs FIRST in this file's reading order for a
/// reason.** An assertion on emptiness that no scanner could ever populate is
/// the defect this repository has already paid for twice. Two planted raw argv
/// fields — one the shape a future flag would take, one the shape pass 5 found
/// unprotected — must both be reported by the SAME function the live assertion
/// consumes.
#[test]
fn the_raw_argv_field_scanner_reports_a_planted_string_field() {
    let planted = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    /// The registry alias to drive.",
            "    pub alias: payload::NonBlank,",
            "    #[cfg(debug_assertions)]",
            "    pub claude_args: Vec<OsString>,",
            "    /// A seventh argv field, added raw — the five-time losing bet.",
            "    pub goal_file: Option<String>,",
            "    pub run_id: Option<String>,",
            "    pub dry_run: bool,",
            "}",
            "pub struct RawDriveArgs {",
            "    pub alias: String,",
            "}",
        ],
    );

    let found = raw_string_argv_fields(&planted.1);
    let reported: Vec<String> = found.iter().map(|(_, text)| text.clone()).collect();
    assert_eq!(
        reported,
        vec![
            "pub goal_file: Option<String>,".to_string(),
            "pub run_id: Option<String>,".to_string(),
        ],
        "the scanner must report BOTH planted raw argv fields and nothing else — \
         `alias: payload::NonBlank` is the protected shape, `claude_args: \
         Vec<OsString>` must not match on a substring, `dry_run: bool` is not a \
         string at all, and `RawDriveArgs`'s own `String` sits past the closing \
         brace and is out of the extracted region entirely. Got: {found:?}"
    );

    // A wrapped declaration must not evade the scan: pass 5 recorded that a name
    // split across two source lines defeats a full-name grep, and a guard that
    // could be silenced by running rustfmt is not a guard.
    let wrapped = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub some_extremely_long_argv_field_name:",
            "        Option<String>,",
            "    pub alias: payload::NonBlank,",
            "}",
        ],
    );
    assert_eq!(
        raw_string_argv_fields(&wrapped.1).len(),
        1,
        "a field declaration wrapped across two lines is still a field \
         declaration; got {:?}",
        raw_string_argv_fields(&wrapped.1)
    );

    // And the other direction: a fully protected declaration reports nothing, so
    // the assertions above are about the planted fields rather than about a
    // scanner that flags everything.
    let clean = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub alias: payload::NonBlank,",
            "    pub command: Option<payload::NonBlank>,",
            "    pub max_steps: Option<u32>,",
            "    pub claude_program: Option<PathBuf>,",
            "}",
        ],
    );
    assert!(
        raw_string_argv_fields(&clean.1).is_empty(),
        "a fully protected declaration must report nothing; got {:?}",
        raw_string_argv_fields(&clean.1)
    );
}

/// Every spelling pass 6 measured guard nine silent (or wrong) on, planted and
/// asserted through the SAME extracted fns the live assertion consumes.
///
/// **The WR-02 reproduction, brought in-tree.** Pass 6 fed ten declarations to
/// this scanner: six were reported by nothing, one was reported with the
/// FOLLOWING line swallowed into its text (so the next field was never judged at
/// all), and the stated `>=10 field / >=6 NonBlank` floor was measured
/// non-binding — `field_lines=12 protected=6` PASSES with a planted `pub(crate)`
/// offender sitting right there. `pub(crate)` and `pub(super)` are the sharp
/// end: `ITEM_OPENERS`, four commits later in this very file, was widened for
/// exactly that spelling.
///
/// **Red arm, observed verbatim against the unfixed scanner** (this commit; the
/// `#[ignore]` comes off in the fix commit, so the committed tree stays green
/// while the red evidence is in history). Every plant fed to
/// `raw_string_argv_fields` as it stands:
///
/// ```text
/// === WR-02 reproduction against the UNFIXED scanner ===
///   pub(crate) opener                  -> reported=0 []
///   pub(super) opener                  -> reported=0 []
///   Option<Box<str>>                   -> reported=0 []
///   Option<Cow<'static, str>>          -> reported=0 []
///   Option<&'static str>               -> reported=0 []
///   Option<OsString>                   -> reported=0 []
///   trailing // last field             -> reported=0 []
///   trailing // mid-struct             -> reported=1 ["pub goal_file: Option<String>, // seventh pub dry_run: bool,"]
///   Vec<OsString> (must NOT report)    -> reported=0 []
///   Option<PathBuf> (must NOT report)  -> reported=0 []
///   FLOOR PROBE: field_lines=12 protected=6 offenders=[]
/// ```
///
/// Seven silent, one MISATTRIBUTED with the following declaration swallowed
/// into its text, and the floor reading exactly the `field_lines=12
/// protected=6` pass 6 recorded while the offender list was empty. The live
/// assertion of this test fails at the first plant:
///
/// ```text
/// thread 'the_raw_argv_field_scanner_sees_every_measured_silent_spelling' (274278) panicked at tests/spawn_seam_guard.rs:2931:9:
/// assertion `left == right` failed: a `pub(crate)`-opened raw argv field must be REPORTED. It compiles, it is reachable from `from_argv`'s destructure, and it is the exact spelling `ITEM_OPENERS` in this same file was widened for. Got: []
///   left: 0
///  right: 1
/// ```
#[test]
fn the_raw_argv_field_scanner_sees_every_measured_silent_spelling() {
    /// One planted declaration inside an otherwise clean `DriveArgs`.
    fn planted_with(lines: &[&str]) -> Vec<(usize, String)> {
        let mut body = vec!["pub struct DriveArgs {", "    pub alias: payload::NonBlank,"];
        body.extend_from_slice(lines);
        body.push("}");
        synthetic_file("src/driver/mod.rs", &body).1
    }

    // --- Openers: the spelling ITEM_OPENERS was widened for, four commits later
    for opener in ["pub(crate)", "pub(super)"] {
        let declaration = format!("    {opener} goal_file: Option<String>,");
        let planted = planted_with(&[declaration.as_str()]);
        let found = raw_string_argv_fields(&planted);
        assert_eq!(
            found.len(),
            1,
            "a `{opener}`-opened raw argv field must be REPORTED. It compiles, it \
             is reachable from `from_argv`'s destructure, and it is the exact \
             spelling `ITEM_OPENERS` in this same file was widened for. Got: \
             {found:?}"
        );
    }

    // --- Payload type spellings: every one of these is a String wearing a coat
    for type_text in [
        "Option<Box<str>>",
        "Option<Cow<'static, str>>",
        "Option<&'static str>",
        "&'static str",
        "Box<str>",
    ] {
        let declaration = format!("    pub goal_file: {type_text},");
        let planted = planted_with(&[declaration.as_str()]);
        let found = raw_string_argv_fields(&planted);
        assert_eq!(
            found.len(),
            1,
            "`{type_text}` is a string payload spelled around the `String` token. \
             A seventh argv field wearing it carries the identical defect, so it \
             must be reported. Got: {found:?}"
        );
    }

    // --- OsString, DENY-BY-DEFAULT (D-18-2). `goal_file` is on no allowlist,
    //     which is the point: it stands in for the seventh field's NEW name.
    let planted = planted_with(&["    pub goal_file: Option<OsString>,"]);
    let found = raw_string_argv_fields(&planted);
    assert_eq!(
        found.len(),
        1,
        "`OsString` is the type argv actually arrives in, so a payload field \
         respelled `OsString` is the same defect wearing the platform type. It is \
         reported unless its NAME is on the two-entry suppress-allowlist — a \
         by-name PROTECTION rule would cover no case the threat model names, \
         because a seventh field carries a new name by definition. Got: {found:?}"
    );

    // --- The false-positive direction, with its own control: the two
    //     legitimate OsString/PathBuf carriers must NOT be reported.
    let legitimate = planted_with(&[
        "    pub claude_args: Vec<OsString>,",
        "    pub claude_program: Option<PathBuf>,",
    ]);
    assert!(
        raw_string_argv_fields(&legitimate).is_empty(),
        "`claude_args: Vec<OsString>` and `claude_program: Option<PathBuf>` are \
         the allowlisted legitimate carriers; reporting them would make the \
         property unsatisfiable. Got: {:?}",
        raw_string_argv_fields(&legitimate)
    );

    // --- Trailing `//` comment, LAST field: the declaration never terminates in
    //     `,` as far as the scan is concerned, so it was dropped entirely.
    let last_field = planted_with(&["    pub goal_file: Option<String>, // seventh"]);
    let found = raw_string_argv_fields(&last_field);
    assert_eq!(
        found.len(),
        1,
        "a raw argv field carrying a trailing `//` comment is still a raw argv \
         field. As the LAST declaration it also exercises the pending flush — a \
         scan that buffers and never judges the buffer drops it silently. Got: \
         {found:?}"
    );

    // --- Trailing `//` comment, MID-STRUCT: pass 6 measured this one
    //     MISATTRIBUTED — the following declaration was swallowed into its text,
    //     so `dry_run` was never judged as a declaration at all.
    let mid_struct = planted_with(&[
        "    pub goal_file: Option<String>, // seventh",
        "    pub dry_run: bool,",
    ]);
    let found = raw_string_argv_fields(&mid_struct);
    assert_eq!(
        found.len(),
        1,
        "exactly one offender: the commented `goal_file`. Got: {found:?}"
    );
    assert!(
        !found[0].1.contains("dry_run"),
        "the FOLLOWING declaration must not be swallowed into the offender's \
         text. Pass 6 measured exactly that: the two lines were joined, so the \
         report named the wrong span AND the next field was never judged on its \
         own. Got: {:?}",
        found[0].1
    );

    // --- The floor probe. Pass 6 measured `field_lines=12 protected=6 ->
    //     PASSES (silent)` on precisely this input: a full twelve-field body
    //     PLUS one `pub(crate)` offender. The floor is not what catches a single
    //     added field; the OFFENDER SCAN is.
    let full_plus_one = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub alias: payload::NonBlank,",
            "    pub command: Option<payload::NonBlank>,",
            "    pub target_phase: Option<payload::NonBlank>,",
            "    pub max_steps: Option<u32>,",
            "    pub wall_clock_cap_secs: Option<u64>,",
            "    pub max_escalations: Option<u32>,",
            "    pub approved_plan: Option<payload::NonBlank>,",
            "    pub run_id: Option<payload::NonBlank>,",
            "    pub dry_run: bool,",
            "    pub goal: Option<payload::NonBlank>,",
            "    pub claude_program: Option<PathBuf>,",
            "    pub claude_args: Vec<OsString>,",
            "    pub(crate) goal_file: Option<String>,",
            "}",
        ],
    );
    let found = raw_string_argv_fields(&full_plus_one.1);
    assert_eq!(
        found.len(),
        1,
        "a twelve-field body plus one `pub(crate)` offender must report the \
         offender. Pass 6 measured `offenders=[]` on exactly this input while \
         both floors passed, which is what proved the floors bound extraction \
         breakage and wholesale style drift — never a single added field. Got: \
         {found:?}"
    );

    // And the floor's OWN filter must agree with the scan about what a field
    // declaration is: a scan and a floor that disagree is how the offender went
    // unseen while the count read twelve.
    let floor_visible = full_plus_one
        .1
        .iter()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            is_field_opener(trimmed) && trimmed.contains(':')
        })
        .count();
    assert_eq!(
        floor_visible, 13,
        "the floor's field filter must see the `pub(crate)` declaration too — \
         one shared `is_field_opener` is what stops the scan and the floor from \
         disagreeing again. Got: {floor_visible}"
    );
}

/// **WR-01's plant: a field declared with NO visibility modifier.**
///
/// Pass 7 traced the control flow and measured `is_field_opener` accepting only
/// `pub `/`pub(`. A seventh argv field spelled `goal_hint: String,` — no `pub` —
/// is therefore skipped by the offender scan AND by the non-vacuity floor, which
/// share that fn. The floor reads `field_lines=12 protected=6` and the scan
/// reports `offenders=[]`: pass 6's exact silent signature, reproduced by a
/// spelling that appeared in neither guard nine's SEEN list nor its SILENT list.
///
/// What USED to stop this spelling was thirteen struct-literal fixture crates —
/// a coincidence, not a bound, and the SUMMARY reported those retired.
///
/// **Red arm, observed verbatim against the unfixed scanner** (this commit; the
/// `#[ignore]` comes off in the fix commit, so the committed tree stays green
/// while the red evidence lands in history):
///
/// ```text
/// running 1 test
/// test the_scanner_reports_a_bare_private_field ... FAILED
///
/// ---- the_scanner_reports_a_bare_private_field stdout ----
///
/// thread 'the_scanner_reports_a_bare_private_field' (664715) panicked at tests/spawn_seam_guard.rs:3315:5:
/// assertion `left == right` failed: a field declared with NO visibility modifier is still a field, and a `String` on it is still raw argv text. `is_field_opener` accepted only `pub `/`pub(`, so this spelling was skipped by the scan AND by the floor that shares the fn — pass 6's exact silent signature (`field_lines=12 protected=6 offenders=[]`) reproduced by a spelling in neither the SEEN nor the SILENT list. Got: []
///   left: []
///  right: ["goal_hint: String,"]
///
/// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s
/// ```
#[test]
fn the_scanner_reports_a_bare_private_field() {
    // The real twelve declarations, plus a THIRTEENTH carrying no visibility
    // modifier at all. `goal_hint: String,` compiles, is reachable from
    // `from_argv`'s destructure, and carries unvalidated argv text — the whole
    // defect class, spelled in a way neither the SEEN nor the SILENT list named.
    let planted = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub alias: payload::NonBlank,",
            "    pub command: Option<payload::NonBlank>,",
            "    pub target_phase: Option<payload::NonBlank>,",
            "    pub max_steps: Option<u32>,",
            "    pub wall_clock_cap_secs: Option<u64>,",
            "    pub max_escalations: Option<u32>,",
            "    pub approved_plan: Option<payload::NonBlank>,",
            "    pub run_id: Option<payload::NonBlank>,",
            "    pub dry_run: bool,",
            "    pub goal: Option<payload::NonBlank>,",
            "    pub claude_program: Option<PathBuf>,",
            "    pub claude_args: Vec<OsString>,",
            "    goal_hint: String,",
            "}",
        ],
    );

    let found = raw_string_argv_fields(&planted.1);
    let reported: Vec<String> = found.iter().map(|(_, text)| text.clone()).collect();
    assert_eq!(
        reported,
        vec!["goal_hint: String,".to_string()],
        "a field declared with NO visibility modifier is still a field, and a \
         `String` on it is still raw argv text. `is_field_opener` accepted only \
         `pub `/`pub(`, so this spelling was skipped by the scan AND by the floor \
         that shares the fn — pass 6's exact silent signature \
         (`field_lines=12 protected=6 offenders=[]`) reproduced by a spelling in \
         neither the SEEN nor the SILENT list. Got: {found:?}"
    );

    // The floor's own filter must see it too. One shared `is_field_opener` is
    // what stops the scan and the floor from disagreeing; if the widening had
    // touched only the scan, this would read 12 and the disagreement pass 6
    // measured would be back in a new spelling.
    let floor_visible = planted
        .1
        .iter()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            is_field_opener(trimmed) && trimmed.contains(':')
        })
        .count();
    assert_eq!(
        floor_visible, 13,
        "the floor's field filter must count the bare declaration as a field \
         declaration too. Got: {floor_visible}"
    );

    // The over-detection direction, bounded rather than asserted in prose. The
    // scan runs only between the struct's braces, so the shapes that LOOK like
    // `word:` elsewhere in a Rust file — match arms, struct-literal
    // initialisers, labelled loops — are out of the region by construction.
    // Inside the region, a protected bare field must still report nothing.
    let clean_bare = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    alias: payload::NonBlank,",
            "    max_steps: Option<u32>,",
            "}",
        ],
    );
    assert!(
        raw_string_argv_fields(&clean_bare.1).is_empty(),
        "widening the opener must not make the scan report protected \
         declarations; got {:?}",
        raw_string_argv_fields(&clean_bare.1)
    );
}

/// **WR-02's plant: an allowlist entry silently repurposed.**
///
/// Pass 7 traced two failures that compound. `judge_declaration`'s `OsString`
/// branch `return`s after the allowlist check, so `names_string_payload` never
/// runs for any declaration naming `OsString` — an allowlisted name suppresses
/// the whole judgment rather than just the OsString-presence report. And the
/// `OSSTRING_ALLOWED` integrity pin, which exists to catch exactly a repurposed
/// entry, was a `contains`: `"pub claude_args: (Vec<OsString>, String),"
/// .contains("Vec<OsString>")` is **true**, so the pin passes over the very
/// declaration it was written to refuse.
///
/// **Red arm, observed verbatim against the unfixed scanner** (this commit; the
/// `#[ignore]` comes off in the fix commit):
///
/// ```text
/// running 1 test
/// test an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring ... FAILED
///
/// ---- an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring stdout ----
///
/// thread 'an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring' (665429) panicked at tests/spawn_seam_guard.rs:3386:5:
/// assertion `left == right` failed: an allowlisted NAME suppresses the `OsString`-presence report only. It must never suppress a raw payload TYPE riding beside it: `judge_declaration` returned from the OsString branch before `names_string_payload` ever ran, so this declaration was silent. Got: []
///   left: []
///  right: ["pub claude_args: (Vec<OsString>, String),"]
///
/// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s
/// ```
#[test]
fn an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring() {
    // `claude_args` is on OSSTRING_ALLOWED because it legitimately holds argv as
    // `Vec<OsString>`. The suppression is by NAME, so a declaration that keeps
    // the name and grows a raw `String` beside the `OsString` inherits the
    // suppression for free — and the integrity pin that exists to catch exactly
    // that was a `contains`, which this type satisfies.
    let planted = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub alias: payload::NonBlank,",
            "    pub claude_args: (Vec<OsString>, String),",
            "}",
        ],
    );

    let found = raw_string_argv_fields(&planted.1);
    let reported: Vec<String> = found.iter().map(|(_, text)| text.clone()).collect();
    assert_eq!(
        reported,
        vec!["pub claude_args: (Vec<OsString>, String),".to_string()],
        "an allowlisted NAME suppresses the `OsString`-presence report only. It \
         must never suppress a raw payload TYPE riding beside it: \
         `judge_declaration` returned from the OsString branch before \
         `names_string_payload` ever ran, so this declaration was silent. Got: \
         {found:?}"
    );

    // The false-positive direction keeps its controls: the two legitimate
    // carriers must still report nothing, or the widening has made the property
    // unsatisfiable rather than stricter.
    let legitimate = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub claude_args: Vec<OsString>,",
            "    pub claude_program: Option<PathBuf>,",
            "}",
        ],
    );
    assert!(
        raw_string_argv_fields(&legitimate.1).is_empty(),
        "the legitimate allowlisted carriers must stay unreported; got {:?}",
        raw_string_argv_fields(&legitimate.1)
    );

    // And a NON-allowlisted name carrying the same compound type is reported by
    // the OsString rule as well as the payload rule — once, not twice.
    let unlisted = synthetic_file(
        "src/driver/mod.rs",
        &[
            "pub struct DriveArgs {",
            "    pub goal_hint: (Vec<OsString>, String),",
            "}",
        ],
    );
    assert_eq!(
        raw_string_argv_fields(&unlisted.1).len(),
        1,
        "a seventh field carrying the compound type is reported exactly once; \
         got {:?}",
        raw_string_argv_fields(&unlisted.1)
    );
}

#[test]
fn drive_args_declares_no_raw_argv_string_field() {
    let files = source_files();
    let home = files
        .iter()
        .find(|(path, _)| path == "src/driver/mod.rs")
        .expect("src/driver/mod.rs must exist");

    let body = drive_args_body(&home.1);

    // **The extraction control.** `RawDriveArgs` is declared a few lines below
    // `DriveArgs` and legitimately holds `String`s — it is the raw side of the
    // parse boundary. An extraction that ran past `DriveArgs`'s closing brace
    // would report every one of them and make this property unsatisfiable, so
    // the region is asserted by name rather than assumed.
    assert!(
        !body.iter().any(|(_, line)| line.contains("RawDriveArgs")),
        "the extracted `DriveArgs` body reached `RawDriveArgs`, so the scan is \
         reading the wrong struct"
    );

    // **Non-vacuity, and it is the whole reason this test can be trusted.** A
    // scanner that matched nothing would satisfy the emptiness assertion below
    // forever — which is the shape three of this phase's guards shipped in.
    let field_lines: Vec<&(usize, String)> = body
        .iter()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            is_field_opener(trimmed) && trimmed.contains(':')
        })
        .collect();
    assert!(
        field_lines.len() >= 10,
        "the scan saw only {} field declarations in `DriveArgs`, so its emptiness \
         proves almost nothing — the struct carries twelve. Either the extraction \
         broke or the declaration style moved — which is what these floors DO \
         bound; a single added field is caught by the offender scan, not here.",
        field_lines.len()
    );
    let protected = field_lines
        .iter()
        .filter(|(_, line)| line.contains("NonBlank"))
        .count();
    assert!(
        protected >= 6,
        "only {protected} of `DriveArgs`'s field declarations name `NonBlank`. All \
         SIX argv-derived string fields — alias, command, target_phase, \
         approved_plan, run_id, goal — must carry the payload type; a field that \
         lost it is the five-time losing bet reopening."
    );

    // **The widening's own control, against the REAL body.** Round 7 widened
    // `is_field_opener` to bare (private) declarations. Every field `DriveArgs`
    // actually declares carries `pub`, so the widened opener must see EXACTLY
    // the set the narrow one saw. The narrow rule is respelled here on purpose,
    // as an independent expected value: if the widened count ever exceeds it,
    // the extra lines are not field declarations, the over-detection direction
    // has become real in this region, and whoever widened the extraction has to
    // say what guard nine should do about it. Under-detection is caught by the
    // floor above and by the offender scan; this catches the other side.
    let narrow_visible = body
        .iter()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            (trimmed.starts_with("pub ") || trimmed.starts_with("pub(")) && trimmed.contains(':')
        })
        .count();
    assert_eq!(
        field_lines.len(),
        narrow_visible,
        "the widened `is_field_opener` sees {} field declarations in the real \
         `DriveArgs` body where the pre-round-7 `pub`-only rule sees \
         {narrow_visible}. Every real field carries `pub`, so the two must agree; \
         a difference means the widening is matching something that is not a \
         field declaration.",
        field_lines.len()
    );

    // **The allowlist's own integrity, pinned against the REAL body — as an
    // EQUALITY, because containment could never see the harm it was written
    // for.** `OSSTRING_ALLOWED` suppresses the `OsString` deny-by-default rule
    // for two NAMES. Nothing in that mechanism alone stops someone respelling
    // one of those fields into a payload carrier and inheriting the suppression
    // for free — so each allowlisted name is pinned here to the type it
    // legitimately declares.
    //
    // Until round 7 this was `declaration.contains(expected_type)`, and pass 7
    // measured what that means: `pub claude_args: (Vec<OsString>, String),`
    // CONTAINS `Vec<OsString>`, so the pin passed over the exact repurposing it
    // exists to refuse. A compound type that inherits the suppression is the
    // planted control
    // (`an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring`),
    // committed red before this line changed. The comparison is now equality on
    // the parsed declared type text, so anything other than the legitimate type
    // — wider, narrower or merely different — breaks loudly.
    for (name, expected_type) in [
        ("claude_args", "Vec<OsString>"),
        ("claude_program", "Option<PathBuf>"),
    ] {
        let declaration = field_lines
            .iter()
            .find(|(_, line)| declared_field_name(line.trim()) == name)
            .unwrap_or_else(|| {
                panic!(
                    "`{name}` is on OSSTRING_ALLOWED but is not declared in \
                     `DriveArgs` at all. An allowlist entry for a field that does \
                     not exist is a suppression waiting for a future field to \
                     inherit — drop the entry or restore the field."
                )
            });
        let actual_type = declared_type_text(&declaration.1);
        assert_eq!(
            actual_type, expected_type,
            "`{name}` is on OSSTRING_ALLOWED because it legitimately carries \
             exactly `{expected_type}`. Its declaration now reads {:?}. The \
             allowlist can only SUPPRESS, and it must not be silently repurposed \
             for a payload field: a type that merely CONTAINS the expected one — \
             `(Vec<OsString>, String)` was pass 7's measured example — smuggles \
             raw argv text past the deny-by-default rule under a name that was \
             cleared for something else. Either restore the type or take the name \
             off the allowlist and let the deny-by-default rule judge it.",
            declaration.1.trim()
        );
    }

    let offenders = raw_string_argv_fields(&home.1);
    let rendered: Vec<(String, usize, String)> = offenders
        .iter()
        .map(|(number, text)| ("src/driver/mod.rs".to_string(), *number, text.clone()))
        .collect();
    assert!(
        offenders.is_empty(),
        "a `DriveArgs` field is declared with a bare `String` type. Every \
         argv-derived string field must be `payload::NonBlank`, whose private \
         field is the only thing that makes a blank payload unrepresentable — \
         four gap-closure cycles each protected a hand-picked subset and each \
         subset was exactly one item short. `DriveArgs::from_argv`'s exhaustive \
         destructure forces a new field to be HANDLED; it does not force it to be \
         handled by giving it a payload type, and this is the check that does. If \
         the new field genuinely is not an argv payload, it does not belong on \
         this struct; if it is, type it `payload::NonBlank` and give it a row in \
         the degenerate matrix. Offending declarations:{}",
        render(&rendered)
    );
}

/// Limit 1's bound, asserted rather than assumed.
///
/// The scan matches the token `String` in a field's type text. A local
/// `type Whatever = String;` would let a raw field wear a name the scan does not
/// know — silent under-detection. The tree declares no such alias today, and
/// this is what keeps that true: if one appears, this fails and whoever added it
/// has to decide what guard nine should do about it.
#[test]
fn no_type_alias_hides_a_string_from_guard_nine() {
    let files = source_files();
    let aliases: Vec<(String, usize, String)> = files
        .iter()
        .filter(|(path, _)| path.starts_with("src/driver/"))
        .flat_map(|file| {
            executable_lines(file)
                .filter(|(_, line)| {
                    let trimmed = line.trim_start();
                    trimmed.starts_with("type ") && names_bare_string(trimmed)
                })
                .map(|(number, line)| (file.0.clone(), *number, line.trim().to_string()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        aliases.is_empty(),
        "a `type` alias under src/driver/ resolves to `String`. Guard nine reads \
         field TYPE TEXT, so an aliased raw field would be invisible to it — the \
         silent direction named as limit 1 in that guard's header. Either drop \
         the alias or teach `raw_string_argv_fields` to resolve it, in the same \
         commit. Found:{}",
        render(&aliases)
    );
}

// ---------------------------------------------------------------------------
// Declared-prohibition needles: a prohibition that is enforced, not trusted
// ---------------------------------------------------------------------------
//
// **Round 4 declared both prohibitions below and shipped violations of both, in
// the very files its own plans edited.** That is the process finding pass 5
// recorded, and the answer is not a third sentence: a prohibition nobody can
// mechanically check is a prohibition nobody checked. Both are cheap to assert,
// so both are asserted.

/// The manufactured-blank spelling: an unreachable match arm answered with a
/// fabricated value rather than a typed refusal.
const MANUFACTURED_BLANK: &str = "=> String::new()";

/// THREE members of the blank-shape payload set, as source text, split in half.
///
/// **Assembled at RUNTIME from halves, following `REJECT_HEAD`/`REJECT_TAIL` in
/// this same file.** Once the uniqueness scan was widened to `tests/` in round 6
/// it began walking this file too, and a witness spelled out as one literal made
/// the guard report ITSELF. The halves are meaningless apart.
///
/// **Three rather than one, and from three DIFFERENT members (round 7).** Pass 7
/// measured the single-witness version detecting a hand copy only if the copy
/// happened to carry the one `"\n  \n"` member — while the failure message told
/// the reader that every blank-shape pin consumes the const. Two of the three are
/// drawn from the members 21-19 added from OUTSIDE the pre-round-7 ranges, so a
/// copy made from the current const is more likely to carry one.
///
/// Index 0 is the whitespace member, index 1 the zero-width-space member, index 2
/// the bidi-override member 21-19 added.
const DEGENERATE_WITNESS_HEADS: [&str; 3] = [
    r#""\n "#,
    r#""\u{2"#,
    r#""\u{20"#,
];

/// The tails of [`DEGENERATE_WITNESS_HEADS`], by the same index.
const DEGENERATE_WITNESS_TAILS: [&str; 3] = [
    r#" \n""#,
    r#"00b}""#,
    r#"2e}""#,
];

/// [`DEGENERATE_WITNESS_HEADS`] and [`DEGENERATE_WITNESS_TAILS`], joined pairwise.
fn degenerate_witnesses() -> [String; 3] {
    [0usize, 1, 2].map(|index| {
        format!(
            "{}{}",
            DEGENERATE_WITNESS_HEADS[index], DEGENERATE_WITNESS_TAILS[index]
        )
    })
}

/// The one file that may spell a witness as part of the shared const itself.
const DEGENERATE_HOME: &str = "src/test_support.rs";

/// The executable sites BESIDES [`DEGENERATE_HOME`] that may spell a witness,
/// as `(witness index, path, exact expected hit count, why it is not a copy)`.
///
/// **Why this table exists, stated plainly.** The `"\n  \n"` witness (index 0)
/// is distinctive — no ordinary string literal contains it — which is what let
/// the single-witness version assert plain uniqueness. The two members round 7
/// added are NOT distinctive: `"\u{200b}"` and `"\u{202e}"` are ordinary hostile
/// fixtures that legitimately appear in the class's own membership pins and in
/// the look-alike suffix list 21-19 added. Widening the witness set therefore
/// buys detection at the cost of over-detection, and the honest way to pay it is
/// to name each legitimate site rather than to quietly narrow the scan.
///
/// The COUNT is exact on purpose: an allowed site cannot grow a second member of
/// the set — the first step of becoming the hand copy this guard exists to
/// catch — without breaking this loudly. The reason column is the adjudication.
const WITNESS_ALLOWED_ELSEWHERE: [(usize, &str, usize, &str); 4] = [
    (
        1,
        "src/journal/writer.rs",
        1,
        "the look-alike SUFFIX list: values appended to a visible stem, which is \
         LOOK_ALIKE_PAIRS' question rather than DEGENERATE's, and it carries a \
         member that is in neither const",
    ),
    (
        1,
        "src/text.rs",
        2,
        "the invisible class's OWN membership pins, in the class's own module; \
         both lists carry code points outside DEGENERATE, so neither is a subset \
         of it",
    ),
    (
        2,
        "src/journal/writer.rs",
        1,
        "the same look-alike SUFFIX list as the row above",
    ),
    (
        2,
        "src/text.rs",
        1,
        "the derived class's solely-invisible pin, whose eight code points are \
         mostly outside DEGENERATE entirely",
    ),
];

#[test]
fn no_match_arm_in_the_driver_manufactures_a_blank_value() {
    let files = source_files();
    let driver: Vec<SourceFile> = files
        .iter()
        .filter(|(path, _)| path == "src/driver/run.rs" || path == "src/driver/mod.rs")
        .cloned()
        .collect();
    assert_eq!(
        driver.len(),
        2,
        "both driver files must be in the scan, or its emptiness is a fact about \
         the file list rather than about the tree"
    );

    let offenders = executable_hits(&driver, MANUFACTURED_BLANK);
    assert!(
        offenders.is_empty(),
        "a match arm answers an unreachable state by manufacturing an empty \
         string. `\"\"` already means FIELD ABSENT on the tolerant read path \
         (D-30), so a fabricated blank written into a record is corrupt evidence \
         rather than a safe default — and unreachable arms outlive the beliefs \
         that make them unreachable, which pass 5 reproduced: a committed \
         `run.json` carrying `\"gsd_command\": \"\"` from a direct call to the \
         `pub` `execute_run`. An unreachable state is answered with a typed \
         refusal. Round 4 wrote this prohibition and shipped two violations of it \
         in the file that declared it, which is why it is now a test. Offending \
         lines:{}",
        render(&offenders)
    );

    // **The positive control, so the zero above is not vacuous.** `String::new()`
    // itself is legitimate outside a match arm — `run.json`'s
    // `claude_code_version` is constructed empty at write one, because the value
    // is genuinely not known yet — so a scanner that had stopped matching
    // anything at all would report the same clean zero as a clean tree.
    let bare = executable_hits(&driver, "String::new()");
    assert!(
        !bare.is_empty(),
        "the scan found no `String::new()` at all in the two driver files, so the \
         emptiness above is a statement about the scanner rather than about the \
         tree"
    );
}

/// No hand copy of the blank-shape set CARRYING ONE OF THREE NAMED WITNESSES is
/// spelled outside its home and the sites named in [`WITNESS_ALLOWED_ELSEWHERE`].
///
/// **This scan used to walk `src/` alone while its message said "tree-wide", and
/// three hand-copied subsets sat in `tests/` the whole time** (pass-6 WR-04):
/// `driver_dry_run.rs` carried two of six and four of six, `driver_goal_seam.rs`
/// four of six. The scan now walks `tests/` as well, which is what makes the
/// walk tree-wide; the three subsets consume the const, which is what makes the
/// scan pass.
///
/// **What this scan performs, and the direction it fails in — round 7's
/// correction, and it is a NARROWING of the claim rather than a widening of the
/// scan.** Pass 7 measured the previous version detecting a hand copy through
/// exactly ONE witness literal while its failure message told the reader that
/// *every* blank-shape pin consumes the const. It does not check that, and no
/// textual scan can: it checks that three specific literals do not appear where
/// they should not.
///
/// * **Under-detection, silent, and this is the residual to know about.** A hand
///   copy that carries only members OTHER than the three witnesses — say
///   `["", "   ", "\t"]`, three real members of the set and none of them a
///   witness — **is invisible to this scan and always will be.** Nothing in this
///   file bounds it. Three witnesses make such a copy less likely than one did;
///   they do not make it impossible, and the failure message no longer says
///   otherwise.
/// * **Over-detection, loud, and adjudicated site by site.** Two of the three
///   witnesses are ordinary hostile fixtures with legitimate homes elsewhere.
///   Those homes are enumerated in [`WITNESS_ALLOWED_ELSEWHERE`] with exact hit
///   counts, so an allowed site that GROWS a second member — the first step of
///   becoming the copy this guard exists to catch — breaks here rather than
///   sliding under a blanket exemption.
///
/// Reachable only because 21-17 dropped `test_support`'s `#[cfg(test)]` gate
/// (D-17-5): before that an integration crate could not name the const at all,
/// which is the limitation the retired in-place disclosures in those two files
/// recorded honestly at the time.
#[test]
fn the_degenerate_payload_set_is_spelled_in_exactly_one_place() {
    use std::collections::BTreeMap;

    let files = source_and_test_files();

    for (index, witness) in degenerate_witnesses().iter().enumerate() {
        let hits = executable_hits(&files, witness);

        let home_hits = hits
            .iter()
            .filter(|(path, _, _)| path == DEGENERATE_HOME)
            .count();
        assert_eq!(
            home_hits, 1,
            "witness {index} must be spelled exactly once in {DEGENERATE_HOME}, \
             as part of the shared `DEGENERATE` const; found {home_hits}. Zero \
             means this scan is looking at nothing and its claim is vacuous — \
             either the member was removed from the const or the halves this \
             witness is assembled from no longer join to a member's source text."
        );

        // Elsewhere: an exact per-path census, compared BOTH ways against the
        // adjudicated table. An extra path is an unadjudicated copy; a missing
        // path is a stale row that would otherwise exempt a file forever; a
        // changed count is an allowed site that grew.
        let mut actual: BTreeMap<&str, usize> = BTreeMap::new();
        for (path, _, _) in hits.iter().filter(|(path, _, _)| path != DEGENERATE_HOME) {
            *actual.entry(path.as_str()).or_default() += 1;
        }
        let expected: BTreeMap<&str, usize> = WITNESS_ALLOWED_ELSEWHERE
            .iter()
            .filter(|(witness_index, _, _, _)| *witness_index == index)
            .map(|(_, path, count, _)| (*path, *count))
            .collect();

        let offenders: Vec<(String, usize, String)> = hits
            .iter()
            .filter(|(path, _, _)| {
                path != DEGENERATE_HOME && !expected.contains_key(path.as_str())
            })
            .cloned()
            .collect();

        assert_eq!(
            actual,
            expected,
            "the per-file census for witness {index} does not match \
             WITNESS_ALLOWED_ELSEWHERE.\n\
             \n\
             A path present here but absent from the table is a blank-shape \
             payload list spelled outside {DEGENERATE_HOME}. Every blank-shape \
             pin consumes `test_support::DEGENERATE`, because a const each seam \
             copies from is a const each seam can copy from INCOMPLETELY — pass 5 \
             found the `--run-id` pin carrying three of the six shapes, added in \
             the very commit that defined six, so the two zero-width shapes were \
             never asserted at the one seam where they were reachable end to end. \
             Consume the const.\n\
             \n\
             A path in the table with a HIGHER count is an adjudicated site that \
             grew another member of the set; re-adjudicate it or make it consume \
             the const. A path in the table with a LOWER count, or missing, is a \
             stale exemption: drop the row, or it goes on exempting a file for a \
             reason that no longer holds.\n\
             \n\
             **What this scan does NOT check:** a hand copy carrying none of the \
             three witnesses is invisible to it — silent under-detection, stated \
             rather than mitigated. Unadjudicated lines:{}",
            render(&offenders)
        );
    }
}

// ---------------------------------------------------------------------------
// Guard ten: every argv alias entry point is classified
//
// **What this guard measures, and what it cannot.** Pass 6 closed the payload
// question for `DriveArgs` — `from_argv`'s no-`..` destructure forces a seventh
// argv field to be classified, and guard nine refuses a raw-`String` payload
// type. Nothing did the same for `Commands`/`EnvelopeAction`: eight subcommand
// variants carry an `alias` field, each one an argv-to-identity conversion, and
// a NINTH subcommand could add a ninth unnoticed. This census makes growth
// break a count, so a human has to classify the new arm before the suite goes
// green again.
//
// **Limits, in guard six's register, with directions.**
//
// 1. **The scan matches two exact declaration spellings** — a trimmed line of
//    exactly `alias: String,` or `alias: Option<String>,` (enum-variant fields
//    carry no `pub`). A field RENAMED (`project: String,`) or given an aliased
//    type is invisible to it: **under-detection, silent**. It is bounded only
//    in the other direction — a removed or respelled EXISTING field breaks the
//    exact-count assertion **loudly**, so the eight rows below cannot rot
//    unnoticed even though a ninth under a new name could hide.
// 2. **The JUDGE column is hand-maintained prose.** The census forces a human
//    to write a row; it cannot verify that the named judge does what the row
//    says. That verification lives in each judge's own pinned tests, and they
//    are named so this claim is checkable rather than asserted:
//    `registry::tests::registering_a_look_alike_beside_its_visible_twin_is_refused`
//    and `every_constructible_alias_can_name_its_own_envelope_root` for
//    `Alias::new`; `driver::tests::every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary`
//    for `NonBlank` at `from_argv`;
//    `journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin`
//    for the seam predicate.
//
// No claim beyond these two.
// ---------------------------------------------------------------------------

/// The file whose alias-carrying variants this census bounds.
const ARGV_ALIAS_HOME: &str = "src/cli.rs";

/// Every argv entry point that carries an alias, with the judge that converts
/// it into an identity.
///
/// A row per variant. `BY DECISION` marks the one deliberately-raw consumer;
/// every other row names a constructor.
const ARGV_ALIAS_ENTRY_POINTS: [(&str, &str); 8] = [
    (
        "Commands::Add",
        "registry::Alias::new in the Add arm (registration refusal)",
    ),
    (
        "Commands::Remove",
        "raw lookup key BY DECISION D-17-3 — recovery path for entries an older \
         build registered; creates nothing; membership-checked",
    ),
    (
        "Commands::Drive",
        "payload::NonBlank at DriveArgs::from_argv (visibility); identity at the \
         seams via is_plain_path_component",
    ),
    (
        "EnvelopeAction::PrePush",
        "registry::Alias::new in the arm; fail-closed exit 1",
    ),
    (
        "EnvelopeAction::PreCommit",
        "registry::Alias::new in the arm; fail-closed exit 1",
    ),
    (
        "EnvelopeAction::Askpass",
        "registry::Alias::new in the arm; fail-closed, no credential emitted",
    ),
    (
        "EnvelopeAction::Guard",
        "registry::Alias::new in the arm; fail-closed exit 2 denies",
    ),
    (
        "EnvelopeAction::Scan",
        "registry::Alias::new in the arm (replaced the manual predicate check)",
    ),
];

/// Judge ONE census row against the scanned [`ARGV_ALIAS_HOME`] lines.
///
/// `Some(reason)` when the row is defective, `None` when it is sound. Extracted
/// so the live assertion and the planted-stale-row control consume the SAME code
/// path — a control that re-implemented the check would witness only its own
/// agreement with itself, which is guard nine's `is_field_opener` rule applied to
/// guard ten.
fn census_row_offence(row: (&str, &str), _lines: &[(usize, String)]) -> Option<String> {
    let (variant, judge) = row;
    if judge.trim().is_empty() {
        return Some(format!(
            "{variant} carries no judge — an unclassified row defeats the census"
        ));
    }
    if !(judge.contains("Alias::new")
        || judge.contains("NonBlank")
        || judge.contains("BY DECISION"))
    {
        return Some(format!(
            "{variant}'s judge must name a constructor (`Alias::new`, `NonBlank`) \
             or be an explicitly recorded raw-by-design decision (`BY DECISION`). \
             Got: {judge:?}"
        ));
    }
    None
}

/// Every line in `lines` that declares an argv alias field.
///
/// Extracted so the live assertion and the planted-ninth control consume the
/// SAME code path — a control that re-implemented the scan would witness only
/// its own agreement with itself.
fn argv_alias_fields(lines: &[(usize, String)]) -> Vec<(usize, String)> {
    lines
        .iter()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            trimmed == "alias: String," || trimmed == "alias: Option<String>,"
        })
        .cloned()
        .collect()
}

/// **The control arm, first in reading order.** A census whose scanner could
/// never see growth is the tautology this repository has paid for repeatedly.
/// A synthetic body carrying the eight real spellings plus one planted ninth
/// must report nine, through the same extracted fn the live assertion uses.
#[test]
fn the_alias_census_reports_a_planted_ninth_field() {
    let planted = synthetic_file(
        ARGV_ALIAS_HOME,
        &[
            "pub enum Commands {",
            "    Add {",
            "        path: PathBuf,",
            "        alias: Option<String>,",
            "    },",
            "    Remove {",
            "        alias: String,",
            "    },",
            "    Drive {",
            "        alias: String,",
            "        dry_run: bool,",
            "    },",
            "}",
            "pub enum EnvelopeAction {",
            "    PrePush {",
            "        alias: String,",
            "    },",
            "    PreCommit {",
            "        alias: String,",
            "    },",
            "    Askpass {",
            "        alias: String,",
            "    },",
            "    Guard {",
            "        alias: String,",
            "    },",
            "    Scan {",
            "        alias: String,",
            "    },",
            "    /// The planted ninth: a new subcommand carrying an alias.",
            "    Adopt {",
            "        alias: String,",
            "    },",
            "}",
        ],
    );

    assert_eq!(
        argv_alias_fields(&planted.1).len(),
        ARGV_ALIAS_ENTRY_POINTS.len() + 1,
        "a ninth subcommand carrying an alias must be REPORTED by the same \
         function the live assertion consumes — otherwise the census is an \
         assertion about a number nothing could ever change. Got: {:?}",
        argv_alias_fields(&planted.1)
    );

    // The other direction, so the count above is about the planted field rather
    // than about a scanner that flags everything: a variant carrying no alias
    // reports nothing.
    let clean = synthetic_file(
        ARGV_ALIAS_HOME,
        &[
            "pub enum Commands {",
            "    List,",
            "    Status {",
            "        verbose: bool,",
            "        label: String,",
            "    },",
            "}",
        ],
    );
    assert!(
        argv_alias_fields(&clean.1).is_empty(),
        "a body declaring no argv alias must report nothing; got {:?}",
        argv_alias_fields(&clean.1)
    );
}

#[test]
fn every_argv_alias_field_is_classified() {
    let files = source_files();
    let home = files
        .iter()
        .find(|(path, _)| path == ARGV_ALIAS_HOME)
        .unwrap_or_else(|| panic!("{ARGV_ALIAS_HOME} must exist"));

    let declared = argv_alias_fields(&home.1);

    // EXACT, not `>=`: growth and shrinkage both have to be classified, and a
    // floor would let a ninth field slide in under it.
    assert_eq!(
        declared.len(),
        ARGV_ALIAS_ENTRY_POINTS.len(),
        "the number of argv alias fields in {ARGV_ALIAS_HOME} changed. A new \
         subcommand carrying an alias is a new argv-to-identity conversion: give \
         it a judge (`registry::Alias::new`, or a recorded raw-by-design decision \
         with its reason), add its row to ARGV_ALIAS_ENTRY_POINTS, and pin its \
         refusal. The count is the tripwire; the table is the record. Declared: \
         {declared:?}"
    );

    for row in ARGV_ALIAS_ENTRY_POINTS {
        if let Some(reason) = census_row_offence(row, &home.1) {
            panic!("{reason}");
        }
    }
}

/// **Guard ten's stale-row control, and the plant is permanent.**
///
/// The live assertion's clean zero must not be indistinguishable from a checker
/// that stopped matching. A synthetic row naming a variant `src/cli.rs` does not
/// declare — with a perfectly good judge, so no other clause can catch it — is
/// fed to the SAME `census_row_offence` the live assertion consumes.
///
/// **Red arm, observed verbatim against the row check as it stood before the
/// variant-existence clause existed** (this commit; the `#[ignore]` comes off in
/// the fix commit):
///
/// ```text
/// running 1 test
/// test every_census_row_names_a_variant_that_still_exists ... FAILED
///
/// ---- every_census_row_names_a_variant_that_still_exists stdout ----
///
/// thread 'every_census_row_names_a_variant_that_still_exists' (720027) panicked at tests/spawn_seam_guard.rs:4276:5:
/// a census row naming a variant `src/cli.rs` does not declare must be REPORTED. Pass 7's warning was that the census bounds a COUNT and nothing else: rename `Scan` to `Sweep`, or delete one variant and add a different one, and the count stays at eight while the table describes a tree that no longer exists. The rows would go on naming judges for variants nobody can invoke. Got: None
///
/// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.03s
/// ```
#[test]
#[ignore = "red: pass-7 guard-ten stale-row plant; un-ignored in the fix commit"]
fn every_census_row_names_a_variant_that_still_exists() {
    let files = source_files();
    let home = files
        .iter()
        .find(|(path, _)| path == ARGV_ALIAS_HOME)
        .unwrap_or_else(|| panic!("{ARGV_ALIAS_HOME} must exist"));

    // --- The plant. `Commands::Adopt` is not declared anywhere in src/cli.rs.
    let stale = (
        "Commands::Adopt",
        "registry::Alias::new in the Adopt arm (registration refusal)",
    );
    let offence = census_row_offence(stale, &home.1);
    assert!(
        offence.is_some(),
        "a census row naming a variant `{ARGV_ALIAS_HOME}` does not declare must \
         be REPORTED. Pass 7's warning was that the census bounds a COUNT and \
         nothing else: rename `Scan` to `Sweep`, or delete one variant and add a \
         different one, and the count stays at eight while the table describes a \
         tree that no longer exists. The rows would go on naming judges for \
         variants nobody can invoke. Got: {offence:?}"
    );

    // --- The live assertion: every real row still names a real variant.
    let stale_rows: Vec<&str> = ARGV_ALIAS_ENTRY_POINTS
        .iter()
        .filter(|row| census_row_offence(**row, &home.1).is_some())
        .map(|(variant, _)| *variant)
        .collect();
    assert!(
        stale_rows.is_empty(),
        "a census row names a variant that {ARGV_ALIAS_HOME} no longer declares. \
         Either the variant was renamed — update the row and re-pin its refusal — \
         or it was removed, in which case drop the row rather than leaving a \
         judge recorded for an entry point that does not exist. Stale: \
         {stale_rows:?}"
    );

    // --- The other direction, so the emptiness above is about the tree rather
    //     than about a checker that reports everything: a row naming a variant
    //     that IS declared must be sound.
    let sound = (
        "Commands::Add",
        "registry::Alias::new in the Add arm (registration refusal)",
    );
    assert_eq!(
        census_row_offence(sound, &home.1),
        None,
        "a row naming a declared variant with a named judge must not be reported"
    );
}
