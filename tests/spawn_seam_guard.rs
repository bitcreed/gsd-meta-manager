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
