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

// Unix-only: exercises the Unix-gated driver / std::os::unix surface.
#![cfg(unix)]

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
/// **This is a declared allowlist, not a habit.** Whoever adds a spawn site must
/// add the entry here in the same commit, and that deliberate edit is the entire point — a spawn site that
/// nobody had to think about is how an agent comes to be launched against a
/// directory the user never opted in.
const SPAWN_ALLOWLIST: &[&str] = &[
    // The agent spawn. The one that takes the capability type.
    "src/executor/claude.rs",
    // The Codex agent spawn (260922-hdj). Takes the capability type exactly as
    // the Claude spawn does.
    "src/executor/codex.rs",
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
    // The real-git DRIFT PIN inside this module's own in-source tests: the
    // two-sided probe that classifies every entry of the leading-option grammar
    // constants against the installed `git`. No agent.
    //
    // **The guard's own path shells out to NOTHING and must not**, because it
    // runs synchronously on the agent's `PreToolUse` critical path — a reproduced
    // 180-240 second hang is why `push_needs_resolved_dests` exists — and because
    // a guard that asks the program it guards to describe its own grammar can be
    // lied to by a `git` earlier on `PATH`. This file-level entry gives that
    // property up, so it is replaced by a STRICTER one local to the module:
    // `the_guards_own_path_shells_out_to_nothing` in `policy.rs`'s own test
    // module asserts no spawn marker appears in the file's text ABOVE its
    // `#[cfg(test)]`, which is the production half this entry stops covering.
    "src/envelope/policy.rs",
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
    // `pgrep -x claude` / `pgrep -x codex` for session detection. Reads only,
    // spawns no agent.
    "src/session_detector.rs",
    // Git reads that back project state. No agent.
    "src/state_reader/git_ops.rs",
    // The `gsd-tools` launcher. No agent.
    "src/state_reader/queue_md.rs",
    // tmux, for switching the user to an existing session. No agent.
    "src/terminal_switch.rs",
    // `which` plus the terminal launch for "open in terminal". No agent.
    "src/ui/screens/detail.rs",
    // The Phase 28 launcher: `systemd-run` (which runs `bwrap`, which runs the
    // agent), `systemctl`, `git` for the push mirror, `claude setup-token` and
    // `gh`. Every launcher spawn lives in this one file, so this is the one
    // entry. The agent is reached only through `bwrap`, or through the explicit
    // degraded spawn of 28-04; nothing else in `src/launcher/` may spawn.
    "src/launcher/process.rs",
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
        let spawns = executable_lines(file).any(|(_, line)| {
            SPAWN_MARKERS
                .iter()
                .any(|marker| calls_marker(line, marker))
        });
        if spawns {
            observed.push(file.0.clone());
        }
    }
    observed.sort();

    let mut allowed: Vec<String> = SPAWN_ALLOWLIST
        .iter()
        .map(|path| path.to_string())
        .collect();
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

/// Any inline JSON object serves as the seam schema: the properties under test
/// are that the seam profile carries a schema beside the empty tool set and that
/// both profiles keep the MCP set empty, not what the schema says. (The goal
/// layer that used to build the real one was retired in Phase 31.)
fn seam_test_schema() -> String {
    serde_json::json!({"type": "object"}).to_string()
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
            json_schema: seam_test_schema(),
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
    let commented =
        format!("/// The declined alternative: the CLI's `{flag}` flag disables hooks.");
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
                    json_schema: seam_test_schema(),
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
// No production item may follow a test-module marker
//
// Several source scans in this tree (and clippy's `items_after_test_module`)
// treat everything from a file's column-zero `mod tests {` to END OF FILE as
// test code. A production item placed after that marker would be invisible to
// them, a silent under-detection that was LIVE once: `src/state_reader/mod.rs`
// carried a production `pub fn count_backlog_items` 219 lines past its marker
// (review-WR-01). The guard below fails loudly on any column-zero item after any
// file's marker, so the skipped region is provably empty rather than assumed so.
// ============================================================================

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

// ---------------------------------------------------------------------------
// Guard ten: every argv alias entry point is classified
//
// **What this guard measures, and what it cannot.** Eight subcommand variants
// of `Commands`/`EnvelopeAction` once carried an `alias` field, each one an
// argv-to-identity conversion, and a NINTH could be added unnoticed. Phase 31
// deleted `Commands::Drive` (the hidden Phase 17 subcommand) and with it one
// row, so the census now counts SEVEN; the count was re-measured, not relaxed.
// This census makes growth break a count, so a human has to classify the new
// arm before the suite goes green again.
//
// **Limits, in guard six's register, with directions.**
//
// 1. **The scan matches two exact declaration spellings** — a trimmed line of
//    exactly `alias: String,` or `alias: Option<String>,` (enum-variant fields
//    carry no `pub`). A field RENAMED (`project: String,`) or given an aliased
//    type is invisible to it: **under-detection, silent**. It is bounded only
//    in the other direction — a removed or respelled EXISTING field breaks the
//    exact-count assertion **loudly**, so the seven rows below cannot rot
//    unnoticed even though an extra one under a new name could hide.
// 2. **The JUDGE column is hand-maintained prose.** The census forces a human
//    to write a row; it cannot verify that the named judge does what the row
//    says. That verification lives in each judge's own pinned tests, and they
//    are named so this claim is checkable rather than asserted:
//    `registry::tests::registering_a_look_alike_beside_its_visible_twin_is_refused`
//    and `every_constructible_alias_can_name_its_own_envelope_root` for
//    `Alias::new`;
//    `journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin`
//    for the seam predicate.
// 3. **The row-existence check matches variant NAMES** (round 7). Pass 7 warned
//    that this census bounded a COUNT and nothing else: rename `Scan` to
//    `Sweep`, or delete one variant and add a different one, and the count stays
//    at seven while every row goes on naming a judge for an entry point nobody
//    can invoke. `census_row_offence` now asserts each row's variant token is
//    still declared in `src/cli.rs`, with
//    `every_census_row_names_a_variant_that_still_exists` as the live assertion
//    and a permanent planted `Commands::Adopt` row beside it so the clean zero is
//    never indistinguishable from a checker that stopped matching. What it does
//    NOT catch: a variant REMOVED and RE-ADDED under the same name with a
//    different meaning — the row still resolves, the judge named in it may no
//    longer be the judge in the arm. **Under-detection, silent**, bounded only by
//    the judge strings' own tests named in limit 2.
//
// No claim beyond these three.
// ---------------------------------------------------------------------------

/// The file whose alias-carrying variants this census bounds.
const ARGV_ALIAS_HOME: &str = "src/cli.rs";

/// Every argv entry point that carries an alias, with the judge that converts
/// it into an identity.
///
/// A row per variant. `BY DECISION` marks the one deliberately-raw consumer;
/// every other row names a constructor.
const ARGV_ALIAS_ENTRY_POINTS: [(&str, &str); 7] = [
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
fn census_row_offence(row: (&str, &str), lines: &[(usize, String)]) -> Option<String> {
    let (variant, judge) = row;
    if judge.trim().is_empty() {
        return Some(format!(
            "{variant} carries no judge — an unclassified row defeats the census"
        ));
    }
    if !(judge.contains("Alias::new") || judge.contains("BY DECISION")) {
        return Some(format!(
            "{variant}'s judge must name a constructor (`Alias::new`) or be an \
             explicitly recorded raw-by-design decision (`BY DECISION`). \
             Got: {judge:?}"
        ));
    }
    // **Round 7: the row must name a variant that still exists.** Pass 7's
    // warning was that the census bounds a COUNT and nothing else, so a variant
    // renamed or swapped for a different one leaves the count at seven while the
    // table describes a tree that no longer exists.
    let token = variant.rsplit("::").next().unwrap_or(variant).trim();
    if !variant_is_declared(token, lines) {
        return Some(format!(
            "{variant} names a variant `{token}` that {ARGV_ALIAS_HOME} no longer \
             declares. Either it was renamed — update the row and re-pin its \
             refusal — or it was removed, in which case drop the row rather than \
             leaving a judge recorded for an entry point nobody can invoke."
        ));
    }
    None
}

/// Whether `token` appears as an enum-variant declaration in `lines`.
///
/// A variant declaration is a non-comment line whose trimmed form is `token`
/// followed by end-of-line, `,`, `{`, `(` or whitespace — the three shapes
/// `src/cli.rs` actually uses (`List,`, `Add {`, and a bare unit variant).
fn variant_is_declared(token: &str, lines: &[(usize, String)]) -> bool {
    lines.iter().any(|(_, line)| {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            return false;
        }
        let Some(rest) = trimmed.strip_prefix(token) else {
            return false;
        };
        match rest.chars().next() {
            None => true,
            Some(next) => matches!(next, ',' | '{' | '(' | ' '),
        }
    })
}

/// Every line in `lines` that declares an argv alias field.
///
/// Extracted so the live assertion and the planted-extra control consume the
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
/// A synthetic body carrying the seven real spellings plus one planted eighth
/// must report eight, through the same extracted fn the live assertion uses.
#[test]
fn the_alias_census_reports_a_planted_extra_field() {
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
            "    /// The planted extra: a new subcommand carrying an alias.",
            "    Adopt {",
            "        alias: String,",
            "    },",
            "}",
        ],
    );

    assert_eq!(
        argv_alias_fields(&planted.1).len(),
        ARGV_ALIAS_ENTRY_POINTS.len() + 1,
        "an extra subcommand carrying an alias must be REPORTED by the same \
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
    // floor would let an extra field slide in under it.
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
         different one, and the count stays at seven while the table describes a \
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

/// The Codex agent spawn must take the capability type, exactly as the Claude
/// one does (260922-hdj). A sibling of the assertion at the end of
/// `every_process_spawn_site_in_src_is_on_the_allowlist`, kept as its own test
/// so that one stays byte-identical.
#[test]
fn the_codex_spawn_seam_takes_the_capability_type() {
    let files = source_files();
    let codex = files
        .iter()
        .find(|(path, _)| path == "src/executor/codex.rs")
        .expect("the codex spawn seam must exist");
    assert!(
        executable_lines(codex).any(|(_, line)| line.contains("project: &DrivableProject")),
        "the codex spawn seam must take the capability type and never a bare path — \
         that signature is what makes the opt-in gate a compile-time property (D-16)"
    );
}

/// Codex's permission-bypass flag family and its full-access sandbox value,
/// assembled at RUNTIME from halves so no line of any file — this one included
/// — carries them (the idiom `REJECT_HEAD`/`REJECT_TAIL` above uses).
fn codex_bypass_needles() -> Vec<String> {
    vec![
        format!("{}{}", "--dangerously", "-bypass-approvals-and-sandbox"),
        format!("{}{}", "--dangerously", "-bypass-hook-trust"),
        format!("{}{}", "danger-full", "-access"),
        format!("{}{}", "--yo", "lo"),
    ]
}

/// Every executable line in `files` carrying one of [`codex_bypass_needles`].
fn codex_bypass_hits(files: &[SourceFile]) -> Vec<(String, usize, String)> {
    codex_bypass_needles()
        .iter()
        .flat_map(|needle| executable_hits(files, needle))
        .collect()
}

#[test]
fn no_executable_line_in_src_carries_a_codex_bypass_flag_or_the_full_access_sandbox() {
    let hits = codex_bypass_hits(&source_files());
    assert!(
        hits.is_empty(),
        "a Codex permission-bypass flag or the full-access sandbox value appeared on an \
         executable line under src/. The driven agent must never run with approvals or \
         the sandbox switched off (D-15 house rule, T-hdj-02). Offending lines:{}",
        render(&hits)
    );
}

#[test]
fn the_codex_bypass_scan_fires_on_code_and_not_on_a_comment() {
    // The planted positive control: without it, a scan whose needles had
    // drifted into matching nothing would pass the test above for the wrong
    // reason.
    for needle in codex_bypass_needles() {
        let planted: SourceFile = (
            "src/planted.rs".to_string(),
            vec![
                (1, format!("    push(&mut argv, \"{needle}\");")),
                (2, format!("    // never emit {needle}")),
            ],
        );
        let hits = codex_bypass_hits(std::slice::from_ref(&planted));
        assert_eq!(
            hits.iter().map(|hit| hit.1).collect::<Vec<_>>(),
            vec![1],
            "the scan must report the code line and only the code line for {needle:?}"
        );
    }
}
