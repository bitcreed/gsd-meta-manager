// ============================================================================
// The mechanical spawn-seam audit (D-17, PITFALLS:521)
//
// One rule governs every assertion below: **a comment is not a guard; the test
// is.** `src/journal/` has claimed since Phase 16 that the strict
// unknown-field attribute is kept out by a grep, and the spawn allowlist below
// claims to list every process-spawn site. Prose cannot enforce either. This
// file does.
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
/// cwd-independent.
const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// Every file under `src/` permitted to contain a process-spawn site.
///
/// **This is a declared allowlist, not a habit.** Whoever adds a spawn site must
/// add the entry here in the same commit, and that deliberate edit is the entire point — a spawn site that
/// nobody had to think about is how an agent comes to be launched against a
/// directory the user never opted in.
const SPAWN_ALLOWLIST: &[&str] = &[
    // A `sleep` child inside this module's own in-source tests, so the "already
    // gone" path can be pointed at a REAL pid that has been reaped rather than
    // at a number chosen to be implausible. The module's production surface
    // signals two existing process groups and spawns nothing.
    "src/driver/kill.rs",
    // A `sleep` child inside this module's own in-source tests, so the pid-reuse
    // probe can be pointed at a REAL live pid whose cmdline lacks the run id.
    // No agent, and nothing in the module's production surface spawns anything.
    "src/driver/liveness.rs",
    // `gitleaks` when a binary happens to be on PATH, as an ADDITIVE second
    // opinion on the built-in credential rules (D-11). Its absence is never a
    // reason to allow, and its output is discarded rather than reproduced. No
    // agent.
    "src/envelope/scan.rs",
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
    // `which`, `xdg-terminal-exec --print-id`, and the GUI terminal launch. No
    // agent.
    "src/multiplexer/gui.rs",
    // `tmux new-window`, and its live direct-exec test. No agent.
    "src/multiplexer/tmux.rs",
    // tmux, for switching the user to an existing session. No agent.
    "src/terminal_switch.rs",
    // The Phase 28 launcher: `systemd-run` (which runs `bwrap`, which runs the
    // agent), `systemctl`, `git` for the push mirror, `claude setup-token` and
    // `gh`. Every launcher spawn lives in this one file, so this is the one
    // entry. The agent is reached only through `bwrap`, or through the explicit
    // degraded spawn of 28-04; nothing else in `src/launcher/` may spawn.
    "src/launcher/process.rs",
];

/// The two shapes a process spawn takes in this tree.
const SPAWN_MARKERS: &[&str] = &["Command::new(", "process_group("];

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
/// The recursive `read_dir` shape follows `src/archive.rs:116`. An unreadable
/// entry is skipped rather than panicked on, exactly as that does.
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
        "a process-spawn site appeared in a file that is not on the allowlist. Confirm the \
         file is a deliberate spawn site and add it to `SPAWN_ALLOWLIST` with a one-line \
         reason, in the same commit (PITFALLS:521). Unexpected: {unexpected:?}"
    );

    let vanished: Vec<&String> = allowed.iter().filter(|p| !observed.contains(p)).collect();
    assert!(
        vanished.is_empty(),
        "an allowlisted file no longer spawns anything, so the allowlist is now wider than \
         the truth it describes. Remove the stale entry: {vanished:?}"
    );
}

#[test]
fn a_signal_to_a_process_group_is_not_mistaken_for_a_spawn() {
    // The guard-of-the-guard. Without the left word boundary in `calls_marker`,
    // the marker `process_group(` matches inside `kill_process_group(`, and a
    // module that only signals an existing group is reported as a process-spawn
    // site. The only way to make the suite green then is to put a file that
    // spawns nothing onto a SPAWN allowlist — which converts an audit of
    // deliberate spawn sites into a list of files somebody once had to add,
    // silently and without anybody noticing the meaning changed.
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
// The hook-disabling flag stays out of src/, coupled by a test not a convention
//
// (The argv-profile guards that used to sit beside this one, for the in-process
// executor's seam and MCP flags, went with that executor in Phase 31.)
//
// Two guards, each in this file's house style: scan `executable_lines` so a doc
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
         PreToolUse hook (the Phase 19 guard, since deleted in Phase 31). A seam spawned with it runs \
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
    // what the rationale-in-code convention requires.
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
// deleted `Commands::Drive` (the hidden Phase 17 subcommand) and then the four
// envelope re-entry arms (`PrePush`, `PreCommit`, `Askpass`, `Guard`) with the
// guard they served, so the census now counts THREE (`Add`, `Remove` and the
// `envelope scan` evidence gate); the count was re-measured, not relaxed.
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
//    exact-count assertion **loudly**, so the three rows below cannot rot
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
//    at three while every row goes on naming a judge for an entry point nobody
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
const ARGV_ALIAS_ENTRY_POINTS: [(&str, &str); 3] = [
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
    // renamed or swapped for a different one leaves the count at three while the
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
/// A synthetic body carrying the three real spellings plus one planted fourth
/// must report four, through the same extracted fn the live assertion uses.
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
         different one, and the count stays at three while the table describes a \
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
