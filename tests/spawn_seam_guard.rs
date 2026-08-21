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

/// **The bound guard six's limit 4 names, and guard eight's limit 3.**
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
/// What it does NOT see, in the register this file uses: an item that is not
/// introduced at column zero (indented inside a post-marker `mod`), and an item
/// whose first token is outside [`ITEM_OPENERS`]. Both are under-detection and
/// silent; both are bounded by the fact that the tree's production style puts
/// items at column zero, and by the non-vacuity control below.
#[test]
fn no_production_item_follows_a_test_module_marker() {
    let files = source_files();

    let mut offenders: Vec<(String, usize, String)> = Vec::new();
    let mut scanned_files = 0usize;

    for file in &files {
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
    // fall out of the scan and the emptiness above would hold forever.
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
// **The needles are variant spellings rather than the bare type name, and the
// reason is a live name collision.** `src/driver/run.rs:614` declares a SECOND,
// unrelated `enum CommandSource` with variants `Fixed(String)` and
// `Routed { target_phase }`. A scan for the bare type name would report that
// enum's own construction and match sites as offenders, and the only way to
// green the suite would be to allowlist `run.rs` — which would exempt the very
// file whose collision made this delicate. So the needles are the three
// fully-qualified variant spellings WITH an opening parenthesis:
//
// * `Command` and `Goal` do not exist on `run.rs`'s enum at all;
// * `run.rs` writes its `Routed` as a STRUCT variant, so it is spelled with a
//   brace and never with a parenthesis.
//
// Renaming one of the two types is the better long-term answer and is out of
// scope here — `21-11-PLAN.md` records it as accepted debt. This guard is
// written so the rename would make it SIMPLER rather than so that it depends on
// the collision persisting: after a rename the needles still match exactly the
// sites they match today.

/// The three variant spellings, as they are written when built or matched.
const COMMAND_SOURCE_VARIANTS: &[&str] = &[
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
///
/// No attempt is made to tell a construction from a pattern match textually. The
/// distinction is not needed — both functions are allowlisted BY NAME, and an
/// occurrence outside them is a finding whichever it is — and a heuristic that
/// tried would be one more approximation this file would then have to document.
const COMMAND_SOURCE_ALLOWLIST: &[(&str, &str)] = &[
    ("src/driver/mod.rs", "command_source"),
    ("src/driver/mod.rs", "preview_text"),
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
    // The count is also the non-vacuity assertion. Each function names all three
    // variants — `command_source` builds one per arm, `preview_text` matches one
    // per arm — so a needle that stopped matching, because of a rename or a
    // reformat that split a line, is a FAILURE here rather than a silently empty
    // scan that would satisfy the emptiness assertion above forever.
    for (file, function) in COMMAND_SOURCE_ALLOWLIST {
        let contributed = hits
            .iter()
            .filter(|(path, _, enclosing, _)| {
                path == file && enclosing.as_deref() == Some(*function)
            })
            .count();
        assert!(
            contributed >= COMMAND_SOURCE_VARIANT_COUNT,
            "{file}::{function} is allowlisted as a site that names all \
             {COMMAND_SOURCE_VARIANT_COUNT} `CommandSource` variants, but the scan \
             attributes only {contributed} lines to it. Either the allowlist is now \
             wider than the truth it describes — the site stopped naming them, and \
             the entry should go in the same commit — or a needle stopped matching \
             and this guard is auditing less than it claims. Needles: \
             {COMMAND_SOURCE_VARIANTS:?}"
        );
    }

    // The name collision, asserted rather than assumed. `src/driver/run.rs`
    // declares its OWN unrelated `enum CommandSource`; if these needles ever begin
    // matching it, the argument above about `Fixed`/`Routed { .. }` has stopped
    // being true and the needles — not the allowlist — are what must change.
    let collided: Vec<(String, usize, String)> = hits
        .iter()
        .filter(|(path, _, _, _)| path == "src/driver/run.rs")
        .map(|(path, number, _, line)| (path.clone(), *number, line.clone()))
        .collect();
    assert!(
        collided.is_empty(),
        "these needles matched `src/driver/run.rs`, which declares a SECOND, \
         unrelated `enum CommandSource` whose variants are `Fixed(..)` and \
         `Routed {{ .. }}`. Allowlisting that file would exempt the very file whose \
         name collision made the needles delicate. Re-point \
         COMMAND_SOURCE_VARIANTS, or rename one of the two enums. Matched:{}",
        render(&collided)
    );
}
