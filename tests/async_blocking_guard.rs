// ============================================================================
// The mechanical blocking-call-inside-`async fn` audit (D-29, D-28, WR-10)
//
// One rule governs every assertion below: **a comment is not a guard; the test
// is.** Until this file existed, the boundary between "a blocking syscall" and
// "an `async fn` body" was held by prose — `src/driver/mod.rs` says *"No
// blocking syscall inside an `async fn`"*, `src/driver/run.rs` says it three
// more times, and `src/executor/claude.rs`'s module doc says why its one spawn
// is exempt. Every one of those sentences is true today and none of them can
// stop the next edit. This file can.
//
// **It is a lint, not a proof, and that is not a hedge — it is the accurate
// description of what a lexical scanner can do.** It reads source text. It
// cannot see through a helper function: a blocking call reached indirectly,
// through a chain this file does not name, is invisible to it. It cannot see
// through a trait object, a macro expansion, or a closure stored and called
// later. A green run here means "no NAMED blocking call appears lexically
// inside an `async fn` body without an intervening hand-off". It does not mean
// "this program never blocks its runtime". Anybody reading a pass as the second
// statement has been misled by this file, which is why the first statement is
// written here rather than inferred.
//
// **Why it is worth having anyway, stated from an observation rather than a
// theory.** The deadlock this discipline prevents was REPRODUCED, not
// speculated about: `tests/driver_lock.rs:201-215` records the scenario in
// which a blocking advisory `flock` inside an `async fn` defeated
// `tokio::time::timeout` outright on a current-thread runtime, because
// `Timeout::poll` polls its inner future inline and a parked thread polls
// nothing at all. The mitigation shipped in Phase 18 (`66c34ae`) put every
// blocking call in the driver behind `spawn_blocking`; nothing but this file
// keeps it there. And Phase 19 raised the stakes: the envelope added several
// synchronous `git` shell-outs — hook installation, the credential
// environment, the settings round-trip, the worktree secret scan, the
// protection probe — on paths Phase 18 already gave a TUI-side caller. There
// the same stall is not a slow CLI, it is a frozen frame the user cannot
// escape.
//
// It is an integration test rather than an in-source one because it reads the
// source tree, and a test that walks `src/` has no business living inside it.
// **The walk covers `src/` only, never `tests/`.** That is what lets this
// file's own prose name the tokens it forbids without invalidating its own
// gate, exactly as `tests/spawn_seam_guard.rs` — the sibling this file is
// modelled on — does.
//
// Two further limits, stated rather than left to be discovered:
//
//   - **`async` blocks are not tracked, only `async fn` bodies.** D-29 scopes
//     the boundary to `async fn`, and widening the tracker to `async move {`
//     would mean guessing at closure boundaries a brace counter cannot settle.
//     A blocking call inside a bare `async` block is out of this audit's scope
//     by design.
//   - **In-source `#[cfg(test)]` modules are skipped.** See
//     `TEST_MODULE_EXCLUSION` below for why, and for what the alternative would
//     have cost.
// ============================================================================

use std::path::{Path, PathBuf};

/// The tree under audit. Resolved at compile time, so the test is
/// cwd-independent — the idiom `tests/spawn_seam_guard.rs:23` already uses. The
/// joined subpath is `src`, never `tests`.
const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// The blocking calls D-29 names, by the shape they take in source.
///
/// The synchronous process-execution methods on the standard-library command
/// type (`output`, `status`, `spawn`), the advisory file-lock call, and the
/// blocking standard-library filesystem calls the driver's paths are built out
/// of.
///
/// A marker beginning with `.` is matched without a left word boundary, because
/// a `.` cannot continue an identifier — see [`calls_marker`]. The awaited
/// forms of `output`/`status` belong to `tokio::process::Command` and are not
/// blocking; [`is_awaited`] is what tells the two apart, and it is the reason
/// `src/state_reader/git_ops.rs`'s async git reads are not reported here.
const BLOCKING_MARKERS: &[&str] = &[
    ".output()",
    ".status()",
    ".spawn()",
    "flock(",
    "fs::read_to_string(",
    "fs::write(",
    "fs::read(",
    "fs::create_dir_all(",
    "fs::remove_file(",
    "fs::remove_dir_all(",
    "fs::metadata(",
    "fs::read_dir(",
    "fs::rename(",
    "fs::copy(",
    "File::open(",
    "File::create(",
    "OpenOptions::new(",
];

/// This repository's own synchronous seams, named because the scanner cannot
/// find them any other way.
///
/// **This list is the direct consequence of the "lint, not a proof" limit, not
/// a refutation of it.** Every blocking call this phase and Phase 17 actually
/// added on a driver path sits behind a helper: `dry_run::build_report` makes
/// two synchronous `git` calls, `establish_envelope` writes four envelope
/// layers, `terminal_label` reads a whole journal, `lock::acquire` calls
/// `flock`. A scanner that looked only for the syscalls in
/// [`BLOCKING_MARKERS`] would walk every `async fn` in the driver and find
/// nothing at all, then report green — which is precisely the vacuous gate this
/// phase argues is worse than no gate.
///
/// Naming them does not make the scanner able to see through a helper; it makes
/// it able to see through *these* helpers. A new synchronous seam that nobody
/// adds here is still invisible, and that residue is the honest ceiling.
///
/// Seven of the thirteen are Phase 19's own: `hooks::install`, `write_settings`,
/// `cred::build_env`, `record_and_check`, `scan_worktree`, `probe_protection`
/// and `git_read_raw` all live under `src/envelope/` (`git_read_raw` in
/// `src/state_reader/git_ops.rs`, reached from three envelope modules), and
/// every one of them shells out to `git` or walks a worktree synchronously.
/// They are named here so `grep -r envelope tests/async_blocking_guard.rs`
/// finds this end of the link and not just the definitions.
///
/// `every_named_blocking_helper_still_exists_in_the_tree` keeps this list from
/// going stale: a rename that emptied a marker would otherwise look exactly
/// like a clean run.
const BLOCKING_HELPERS: &[&str] = &[
    "build_report(",
    // The driver's own preview entry point, added in Phase 20 when the dry-run
    // grew a second mode. It wraps `build_report` / `build_routed_report`, so
    // the two synchronous `git` calls are still there — only the name at the
    // call site changed, and a marker that named only the old one would have
    // silently stopped seeing the join-failure fallback it was written for.
    "preview_text(",
    "build_routed_report(",
    // Phase 21's third report builder, and it makes the same two synchronous
    // `git` calls its two siblings above do — `working_tree_stat` and
    // `push_refspecs`, on the same foreground preview path. Named here in the
    // commit that introduced it, because this list's own doc is explicit that a
    // new synchronous seam nobody adds here stays invisible to the scanner.
    "build_goal_report(",
    "establish_envelope(",
    "terminal_label(",
    "lock::acquire(",
    "JournalRun::start(",
    "inbox::tail(",
    "probe_protection(",
    "scan_worktree(",
    "cred::build_env(",
    "hooks::install(",
    "write_settings(",
    "record_and_check(",
    "git_read_raw(",
    // Phase 20's, and it was a **pre-existing hole** rather than one that phase
    // opened: `RunSnapshot::capture` does full-tree file I/O and shells out to
    // git twice — its own doc says so in as many words — and it has been
    // reachable from an `async fn` since Phase 15 without appearing here at all.
    // Phase 20 is what made naming it urgent: the iteration loop captures a
    // snapshot before **every** command rather than twice per run, and the
    // thread it must not park is the one polling the terminate arm. A driver
    // that stops answering the kill switch during a long observe is a driver the
    // user experiences as ignoring the stop.
    "RunSnapshot::capture(",
];

/// The hand-off shapes that move blocking work off the runtime thread.
///
/// Once one of these appears, the scanner stops reporting until the block it
/// opened closes. That is deliberately generous: the canonical wrapper in
/// `src/driver/mod.rs`'s dry-run arm — `spawn_blocking(move || { … })` — must
/// read as compliant rather than as a violation, and so must every one of the
/// nine in `src/driver/run.rs`.
const HANDOFF_MARKERS: &[&str] = &["spawn_blocking", "block_in_place"];

/// Why in-source `#[cfg(test)]` modules are outside the walk.
///
/// The boundary D-29 protects is the one between blocking work and a *running*
/// runtime that has other futures to poll — a TUI frame to draw, a stream to
/// read, a timeout to fire. A `#[tokio::test]` body owns its own runtime, has
/// nothing else scheduled on it, and blocks only itself; `src/driver/kill.rs`
/// spawns a real `sleep` child inside one on purpose, so an "already gone" pid
/// is a pid that really was reaped rather than a number chosen to be
/// implausible.
///
/// The alternative — following `tests/spawn_seam_guard.rs` and allowlisting at
/// whole-file granularity — was rejected after being measured: it would have
/// put `src/app.rs`, `src/driver/kill.rs` and `src/driver/mod.rs` on the
/// allowlist for their test bodies alone, and `src/driver/mod.rs` is where
/// `drive` lives. An allowlist that blinds the guard to the function D-29 was
/// written about is not an allowlist, it is an off switch.
const TEST_MODULE_EXCLUSION: &str = "in-source #[cfg(test)] modules are not walked";

/// Every deliberate blocking call inside an `async fn`, as `(file, marker)`.
///
/// **This is a declared allowlist, not a habit.** Whoever adds an entry adds it
/// in the same commit as the code it covers, and that deliberate edit is the
/// entire point — a blocking call that nobody had to think about is how a
/// multi-hour run comes to freeze a frame the user cannot escape.
///
/// Entries are `(path, marker)` pairs rather than bare paths, and the extra
/// column is doing real work: `src/driver/run.rs` is the run body, the most
/// safety-relevant `async` code in this tree, and allowlisting the whole file
/// for its two join-failure fallbacks would leave every future blocking call in
/// it unreported. Scoping the exemption to the marker that is actually
/// deliberate keeps the rest of the file guarded.
///
/// The constant lives in this file rather than in a separate data file, matching
/// the `SPAWN_ALLOWLIST` convention exactly — one guard shape in this tree, not
/// two — so a reader who knows the sibling knows this one, and the justification
/// sits where the entry does instead of one file away.
///
/// `no_allowlist_entry_is_stale` refuses an entry that no longer suppresses
/// anything, so the list cannot grow wider than the truth it describes.
const ASYNC_BLOCKING_ALLOWLIST: &[(&str, &str)] = &[
    // The TUI's suspend-and-edit. `ratatui::restore()` has already handed the
    // terminal to `$EDITOR` on the line above; there is no frame to draw and no
    // input to read until the editor exits, so blocking the runtime here is the
    // intended behaviour rather than a cost. Handing it to `spawn_blocking`
    // would let the event loop keep drawing over the editor's screen.
    ("src/main.rs", ".status()"),
    // The agent spawn. The child is long-lived and duplex, so it is `spawn`ed
    // and then read asynchronously — `src/executor/claude.rs`'s module doc
    // states this outright ("not `spawn_blocking`. The child is long-lived and
    // duplex"). `Command::spawn` forks and execs; it does not wait.
    ("src/executor/claude.rs", ".spawn()"),
    // The dry-run preview's join-failure fallback. The report is built inside
    // `spawn_blocking` on every healthy path; this inline re-run is reachable
    // only if that task panicked or the runtime is shutting down, and it exists
    // so a preview stays honest on a path no healthy run reaches.
    //
    // **The marker moved from `build_report(` to `preview_text(` in Phase 20**,
    // in the same commit as the code that moved it. The dry-run gained a second
    // mode, so the two builders sit behind one entry point and the async fn no
    // longer names `build_report` at all. `no_allowlist_entry_is_stale` is what
    // caught the drift: the old entry stopped suppressing anything, which is
    // precisely the "wider than the truth it describes" state it refuses — and
    // the blocking work had not gone anywhere, only its name had.
    ("src/driver/mod.rs", "preview_text("),
    // The terminal record's two join-failure fallbacks, the same shape and the
    // same reason: the label is read inside `spawn_blocking`, and the inline
    // re-run keeps a park reason on `run.json` rather than losing it merely
    // because a task failed to join. A run that ends with no record at all is
    // the one failure OBS-01 cannot tolerate.
    ("src/driver/run.rs", "terminal_label("),
    // The per-iteration snapshot capture's join-failure fallback, the same shape
    // and the same reason as the two above it: every healthy iteration captures
    // inside `spawn_blocking`, and this inline re-run is reachable only if that
    // task panicked or the runtime is shutting down. It re-runs rather than
    // yielding a default snapshot on purpose — a default compares unequal to
    // everything and would silently reset the no-progress evidence, which is
    // CTRL-06's stall detector switched off on the one path no healthy run
    // reaches.
    ("src/driver/run.rs", "RunSnapshot::capture("),
    // The executor's own capture fallback, which is the hole this marker was
    // added to close. It has been unreported since Phase 15 — not because it was
    // judged acceptable, but because nothing named the helper — and the entry is
    // added in the same commit as the marker so the exemption is a decision
    // rather than an inheritance. Same shape again: `capture_snapshot` hands the
    // work to `spawn_blocking` and re-runs inline only on a join failure.
    ("src/executor/claude.rs", "RunSnapshot::capture("),
];

/// The floor below which this audit is examining too little to mean anything.
///
/// Non-vacuity in the register `tests/spawn_seam_guard.rs` already uses: an
/// audit that walked nothing passes for the wrong reason, and a brace-tracking
/// bug that quietly stopped entering `async fn` bodies would look exactly like
/// a clean tree. Roughly 1700 production `async` body lines exist at the time of
/// writing; the floor is set well below that so ordinary churn does not trip it,
/// and well above zero so a broken walk does.
const MIN_ASYNC_BODY_LINES: usize = 800;

/// The two files D-29 is actually about. If neither has an `async fn` body the
/// walk found, the audit is not auditing the thing it was written for.
const REQUIRED_ASYNC_FILES: &[&str] = &["src/driver/mod.rs", "src/driver/run.rs"];

/// One reported violation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Hit {
    path: String,
    line: usize,
    marker: &'static str,
    text: String,
}

/// One source file: its path relative to the crate root, and its lines.
type SourceFile = (String, Vec<String>);

/// Whether `line` calls `marker`, as opposed to merely containing its letters.
///
/// **A plain `contains` is wrong here and `tests/spawn_seam_guard.rs:88-102`
/// records the case that first proved it**: `rustix::process::kill_process_group(`
/// contains `process_group(`, so a module that only *signals* an existing group
/// was reported as a spawn site — and the only way to green the suite would have
/// been to allowlist a file that spawns nothing, which quietly turns an audit
/// into a list of files somebody once had to add. A guard that fires on a
/// coincidence is a guard that gets disabled.
///
/// The rule is the sibling's, with one generalisation this file needs. A left
/// word boundary is required only when the marker *begins* with a character
/// that could continue an identifier: `read_dir(` must not match
/// `fs::read_dir(`'s letters inside `spawn_read_dir(`, but `.spawn()` needs no
/// left boundary because a `.` already is one. The sibling applies the boundary
/// unconditionally, which — applied to `.spawn()` — rejects every real hit,
/// since the character before the `.` is always the end of a receiver name.
/// The right side needs no boundary because every marker already ends in `(`.
fn calls_marker(line: &str, marker: &str) -> bool {
    let needs_boundary = marker
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');

    let mut from = 0;
    while let Some(offset) = line[from..].find(marker) {
        let at = from + offset;
        if !needs_boundary {
            return true;
        }
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

/// `line` with its string and character literals blanked out.
///
/// Two things depend on this. Brace counting is the scanner's whole backbone,
/// and a `'{'` character literal or a `r#"{"type":"user"…"#` fixture would skew
/// it silently — the tracker would then leave an `async fn` body early and
/// report nothing for the rest of the file, which is a false *negative* and the
/// worst failure a guard can have. And a marker's letters appearing inside an
/// assertion message must not be reported as a call.
///
/// A raw string that spans lines is not handled; there is none in production
/// code today, and the two in `src/driver/run.rs` sit inside the test module
/// this walk skips.
fn strip_literals(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // Raw string: `r"…"`, `r#"…"#`, `r##"…"##`.
        if c == 'r' {
            let mut hashes = 0;
            let mut j = i + 1;
            while j < chars.len() && chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                i = skip_raw(&chars, j + 1, hashes);
                continue;
            }
        }

        if c == '"' {
            i = skip_quoted(&chars, i + 1);
            continue;
        }

        // A `'` opens a character literal or a lifetime, and only the first
        // carries braces. `&'a str` must survive; `'{'` must not.
        if c == '\'' {
            if let Some(next) = skip_char_literal(&chars, i) {
                i = next;
                continue;
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

fn skip_raw(chars: &[char], from: usize, hashes: usize) -> usize {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '"' {
            let closed = (1..=hashes).all(|offset| chars.get(i + offset) == Some(&'#'));
            if closed {
                return i + hashes + 1;
            }
        }
        i += 1;
    }
    chars.len()
}

fn skip_quoted(chars: &[char], from: usize) -> usize {
    let mut i = from;
    while i < chars.len() {
        match chars[i] {
            '\\' => i += 2,
            '"' => return i + 1,
            _ => i += 1,
        }
    }
    chars.len()
}

fn skip_char_literal(chars: &[char], at: usize) -> Option<usize> {
    if chars.get(at + 1) == Some(&'\\') {
        // `'\n'`, `'\''`, `'\u{1F600}'` — scan to the closing quote.
        let mut i = at + 2;
        while i < chars.len() {
            if chars[i] == '\'' {
                return Some(i + 1);
            }
            i += 1;
        }
        return None;
    }
    if chars.get(at + 2) == Some(&'\'') {
        return Some(at + 3);
    }
    None
}

/// Whether the call on `lines[index]` is awaited.
///
/// `tokio::process::Command`'s `output` and `status` are futures and block
/// nothing; the standard library's are blocking calls. Lexically the two differ
/// only by the `.await`, which `rustfmt` puts either on the same line or at the
/// start of the next one. Both forms are recognised, and a two-line lookahead
/// skips blanks and comments so an interposed comment cannot turn an async read
/// into a reported violation.
fn is_awaited(lines: &[String], index: usize) -> bool {
    if lines[index].contains(".await") {
        return true;
    }
    for line in lines.iter().skip(index + 1).take(2) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        return trimmed.starts_with(".await");
    }
    false
}

/// Whether `code` opens an `async fn` declaration.
fn opens_async_fn(code: &str) -> bool {
    calls_marker(code, "async fn ")
}

/// Whether `code` opens a module body.
fn opens_module(code: &str) -> bool {
    let trimmed = code.trim();
    let trimmed = trimmed.strip_prefix("pub ").unwrap_or(trimmed);
    trimmed.starts_with("mod ") && trimmed.ends_with('{')
}

/// Where the scanner is, relative to the nearest `async fn`.
enum Position {
    /// Not inside one.
    Outside,
    /// Past the `async fn` token, before the body's opening brace. The payload
    /// is the brace depth the declaration started at.
    ///
    /// This state is why the scanner works at all on a multi-line signature.
    /// Treating the declaration line as the body's start makes `depth <= base`
    /// true immediately and the body is left on the line it was entered — a bug
    /// that silently exempts every function whose parameters do not fit on one
    /// line, which is most of the ones in `src/driver/run.rs`.
    Signature(i32),
    /// Inside the body, which started at this brace depth.
    Body(i32),
}

/// Scan one file. Returns its violations and how many `async fn` body lines it
/// examined, so the caller can tell "no violations" from "nothing was walked".
fn scan(path: &str, lines: &[String]) -> (Vec<Hit>, usize) {
    let mut hits = Vec::new();
    let mut examined = 0usize;

    let mut depth: i32 = 0;
    let mut position = Position::Outside;
    let mut handoff: Option<i32> = None;
    let mut test_module: Option<i32> = None;
    let mut pending_test = false;

    for (index, raw) in lines.iter().enumerate() {
        if raw.trim_start().starts_with("//") {
            continue;
        }
        let code = strip_literals(raw);
        let trimmed = code.trim();

        if test_module.is_none() {
            if trimmed.starts_with("#[cfg(test)]") {
                pending_test = true;
            } else if pending_test && opens_module(trimmed) {
                test_module = Some(depth);
                pending_test = false;
            } else if pending_test && !trimmed.is_empty() {
                pending_test = false;
            }
        }

        if test_module.is_none() {
            if matches!(position, Position::Outside) && opens_async_fn(&code) {
                position = Position::Signature(depth);
            }

            if matches!(position, Position::Body(_)) {
                examined += 1;

                if handoff.is_none() && HANDOFF_MARKERS.iter().any(|m| code.contains(m)) {
                    handoff = Some(depth);
                }

                if handoff.is_none() {
                    let found = BLOCKING_MARKERS
                        .iter()
                        .chain(BLOCKING_HELPERS.iter())
                        .find(|marker| calls_marker(&code, marker));
                    if let Some(marker) = found {
                        if !is_awaited(lines, index) {
                            hits.push(Hit {
                                path: path.to_string(),
                                line: index + 1,
                                marker,
                                text: raw.trim().to_string(),
                            });
                        }
                    }
                }
            }
        }

        depth += code.chars().filter(|c| *c == '{').count() as i32;
        depth -= code.chars().filter(|c| *c == '}').count() as i32;

        if let Some(base) = test_module {
            if depth <= base {
                test_module = None;
            }
        }
        position = match position {
            Position::Signature(base) if depth > base => Position::Body(base),
            Position::Body(base) if depth <= base => {
                handoff = None;
                Position::Outside
            }
            other => other,
        };
        if let Some(base) = handoff {
            if depth <= base {
                handoff = None;
            }
        }
    }

    (hits, examined)
}

/// Every `*.rs` file under `src/`, recursively, sorted by path.
///
/// The recursive `read_dir` shape follows `tests/spawn_seam_guard.rs:219-260`.
/// An unreadable entry is skipped rather than panicked on, exactly as it does.
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
        out.push((relative, text.lines().map(|line| line.to_string()).collect()));
    }
}

/// Whether `hit` is covered by a declared allowlist entry.
fn is_allowlisted(hit: &Hit) -> bool {
    ASYNC_BLOCKING_ALLOWLIST
        .iter()
        .any(|(path, marker)| *path == hit.path && *marker == hit.marker)
}

fn render(hits: &[Hit]) -> String {
    hits.iter()
        .map(|hit| {
            format!(
                "\n  {}:{} [{}]: {}",
                hit.path, hit.line, hit.marker, hit.text
            )
        })
        .collect::<String>()
}

/// Every violation in the tree, plus the walk's own coverage numbers.
fn audit() -> (Vec<Hit>, usize, Vec<String>) {
    let files = source_files();
    let mut hits = Vec::new();
    let mut examined = 0usize;
    let mut with_async = Vec::new();

    for (path, lines) in &files {
        let (file_hits, file_examined) = scan(path, lines);
        if file_examined > 0 {
            with_async.push(path.clone());
        }
        examined += file_examined;
        hits.extend(file_hits);
    }

    (hits, examined, with_async)
}

#[test]
fn every_blocking_call_inside_an_async_fn_is_handed_off_or_allowlisted() {
    let (hits, examined, with_async) = audit();

    let offenders: Vec<Hit> = hits.iter().filter(|hit| !is_allowlisted(hit)).cloned().collect();
    assert!(
        offenders.is_empty(),
        "a known-blocking call appears lexically inside an `async fn` body with no \
         intervening hand-off. There are exactly two correct responses and neither is \
         weakening the marker set: wrap the call in `tokio::task::spawn_blocking` — \
         `src/driver/mod.rs`'s dry-run arm is the canonical shape, clone-not-move and all \
         — or, if the block is deliberate, add a `(file, marker)` entry to \
         `ASYNC_BLOCKING_ALLOWLIST` in this file with a one-line reason, in the SAME \
         commit as the code it covers. The deadlock this prevents was observed rather \
         than theorised (`tests/driver_lock.rs:201-215`, D-29). Offending lines:{}",
        render(&offenders)
    );

    // Non-vacuity. A brace-tracking bug that stopped entering `async fn` bodies
    // would satisfy the assertion above forever while auditing nothing, and it
    // would look identical to a clean tree.
    assert!(
        examined >= MIN_ASYNC_BODY_LINES,
        "the audit examined only {examined} lines of `async fn` bodies and expected at \
         least {MIN_ASYNC_BODY_LINES}, so it is checking far less than it thinks. Either \
         the tree lost most of its async surface — in which case lower the floor \
         deliberately — or the walk is broken ({TEST_MODULE_EXCLUSION})"
    );

    for required in REQUIRED_ASYNC_FILES {
        assert!(
            with_async.iter().any(|path| path == required),
            "the audit found no `async fn` body in {required}, which is one of the two \
             files D-29 was written about. A guard that does not reach `drive` and the run \
             body is not guarding the boundary it claims to. Files with async bodies: \
             {with_async:?}"
        );
    }
}

#[test]
fn no_allowlist_entry_is_stale() {
    let (hits, _, _) = audit();

    let unused: Vec<&(&str, &str)> = ASYNC_BLOCKING_ALLOWLIST
        .iter()
        .filter(|(path, marker)| {
            !hits
                .iter()
                .any(|hit| hit.path == *path && hit.marker == *marker)
        })
        .collect();

    assert!(
        unused.is_empty(),
        "an allowlist entry no longer suppresses anything, so the allowlist is now wider \
         than the truth it describes and the next blocking call written there would pass \
         unremarked. Remove the stale entry in the same commit as the code that made it \
         stale: {unused:?}"
    );
}

#[test]
fn every_named_blocking_helper_still_exists_in_the_tree() {
    // The stale-marker guard, and it is not optional. `BLOCKING_HELPERS` is the
    // half of the marker set that carries this repository's own blocking seams,
    // and a rename would empty a marker silently — the audit would keep passing
    // while quietly checking one thing less. This is the same non-vacuity
    // argument `tests/spawn_seam_guard.rs` makes about its override fields.
    let files = source_files();

    let missing: Vec<&&str> = BLOCKING_HELPERS
        .iter()
        .filter(|marker| {
            !files
                .iter()
                .any(|(_, lines)| lines.iter().any(|line| line.contains(**marker)))
        })
        .collect();

    assert!(
        missing.is_empty(),
        "a named blocking helper no longer appears anywhere under src/. If it was \
         renamed, re-point the marker; if the seam was removed outright, delete the \
         marker in the same commit. Leaving it is a marker that matches nothing and an \
         audit that silently checks less: {missing:?}"
    );
}

#[test]
fn the_non_vacuity_floor_counts_real_async_bodies_rather_than_reporting_a_constant() {
    // **The guard of the guard's guard.** `every_blocking_call_inside_an_async_fn_
    // is_handed_off_or_allowlisted` asserts an emptiness and then defends that
    // against vacuity with `examined >= MIN_ASYNC_BODY_LINES`. But nothing
    // proved the FLOOR itself was live: if `scan` returned a large count for
    // reasons unrelated to async bodies, the floor would pass while the walk was
    // broken, and the emptiness above it would be meaningless.
    //
    // Asserting `examined + 1 > examined` would prove arithmetic, not the
    // scanner. What has to be shown is that the counter RESPONDS to its input —
    // zero when there is no async body, and the body's own size when there is.

    // Arm one: a file with no `async fn` contributes nothing to the count, even
    // though it contains a call the marker set knows. A walk that counted every
    // line regardless would clear the floor while auditing nothing.
    let sync_only = lines(&[
        "pub fn observe(path: &Path) -> RunSnapshot {",
        "    let snapshot = RunSnapshot::capture(path);",
        "    snapshot",
        "}",
    ]);
    let (hits, examined) = scan("src/synthetic_sync.rs", &sync_only);
    assert_eq!(
        examined, 0,
        "a file with no `async fn` must contribute ZERO examined lines; a counter \
         that ticked here would let a broken walk clear the floor on synchronous \
         code alone"
    );
    assert!(
        hits.is_empty(),
        "and a blocking call OUTSIDE an async fn is not a violation — D-29 is \
         about the reactor thread, not about blocking calls in general: {hits:?}"
    );

    // Arm two: an async body of known size is counted, and counted as its own
    // lines rather than the file's.
    let async_body = lines(&[
        "// a leading comment that is not inside any body",
        "pub async fn observe(path: &Path) -> u64 {",
        "    let a = 1;",
        "    let b = 2;",
        "    a + b",
        "}",
        "// a trailing comment that is not inside any body",
    ]);
    let (_, examined_async) = scan("src/synthetic_async.rs", &async_body);
    assert!(
        examined_async >= 3,
        "the three statement lines of the async body must be counted, got \
         {examined_async}"
    );
    assert!(
        examined_async < async_body.len(),
        "but the comments outside the body must NOT be, or the count is a file \
         length wearing a floor's name: counted {examined_async} of {} lines",
        async_body.len()
    );

    // Arm three: the floor discriminates between the two arms above. A tree
    // whose walk broke completely would measure what arm one measured — and
    // that value must fall BELOW the floor, or the floor could never fail and
    // the non-vacuity guard would be decoration.
    assert!(
        examined < MIN_ASYNC_BODY_LINES,
        "a broken walk measures {examined}, which must be under the floor of \
         {MIN_ASYNC_BODY_LINES} for the floor to be capable of failing at all"
    );
    let (_, real, _) = audit();
    assert!(
        real >= MIN_ASYNC_BODY_LINES,
        "and the real tree measures {real}, which clears it — so the floor sits \
         strictly between a broken walk and a working one, which is the only \
         position from which it distinguishes them"
    );
}

#[test]
fn the_snapshot_capture_marker_covers_both_of_the_trees_inline_captures() {
    // Phase 20's plan expected to ADD the snapshot-capture coverage here. It was
    // already added — by plan 20-01, in the same commit as the loop that made it
    // urgent — so this test pins the coverage rather than re-adding it, and
    // proves the claim is true of the tree rather than of the plan text.
    assert!(
        BLOCKING_HELPERS.contains(&"RunSnapshot::capture("),
        "the synchronous full-tree capture must be visible to the scanner; it \
         does full-tree file I/O and shells out to git twice, and was invisible \
         from Phase 15 until Phase 20 named it"
    );

    // **`capture_snapshot(` is deliberately NOT a marker**, and that is the
    // whole finding. Both spellings of it — `src/executor/claude.rs:747` and
    // `src/driver/run.rs:1401` — are `async fn`s that hand the work to
    // `spawn_blocking` internally. Naming them would report three call sites
    // that are already correct, and the only way to make the suite green again
    // would be three allowlist entries suppressing non-problems. An allowlist
    // that grows for calls that were never violations is exactly the silent
    // widening CTRL-06 forbids.
    assert!(
        !BLOCKING_HELPERS.contains(&"capture_snapshot("),
        "`capture_snapshot` is an async wrapper that already hands off; naming it \
         would force allowlist entries for compliant code"
    );

    // Both inline re-runs are the join-failure fallbacks, one per capture site,
    // and each is allowlisted at marker granularity rather than by file.
    for file in ["src/driver/run.rs", "src/executor/claude.rs"] {
        assert!(
            ASYNC_BLOCKING_ALLOWLIST
                .iter()
                .any(|(path, marker)| *path == file && *marker == "RunSnapshot::capture("),
            "{file}'s inline join-failure capture must carry a declared entry"
        );
    }
}

#[test]
fn the_scanner_reports_a_planted_blocking_call_and_spares_a_handed_off_one() {
    // The control arm, and it is not optional: the gate above asserts an
    // emptiness, and a scanner that recognised nothing at all would satisfy it
    // forever while auditing nothing. These snippets are synthetic rather than
    // read from the tree, so the arm keeps proving the scanner works even
    // once — especially once — the tree is correct.
    let violating = lines(&[
        "pub async fn run(",
        "    path: &Path,",
        ") -> anyhow::Result<()> {",
        "    let text = std::fs::read_to_string(path)?;",
        "    Ok(())",
        "}",
    ]);
    let (hits, examined) = scan("src/synthetic.rs", &violating);
    assert_eq!(
        hits.len(),
        1,
        "a blocking read inside a multi-line-signature `async fn` must be reported: {hits:?}"
    );
    assert_eq!(hits[0].line, 4);
    assert_eq!(hits[0].marker, "fs::read_to_string(");
    assert!(examined > 0, "the body must have been examined at all");

    // The same call, handed off. This is the shape `src/driver/run.rs` uses
    // nine times over; reporting it would make the guard unusable and it would
    // be switched off, which is the failure mode this file exists inside a
    // phase that argues about.
    let handed_off = lines(&[
        "pub async fn run(path: PathBuf) -> anyhow::Result<()> {",
        "    let text = tokio::task::spawn_blocking(move || {",
        "        std::fs::read_to_string(&path)",
        "    })",
        "    .await??;",
        "    Ok(())",
        "}",
    ]);
    let (hits, _) = scan("src/synthetic.rs", &handed_off);
    assert!(
        hits.is_empty(),
        "a call inside `spawn_blocking` is the compliant shape, not a violation: {hits:?}"
    );

    // And the hand-off must not extend past the block it opened: the
    // join-failure fallback in `src/driver/mod.rs` sits in a `match` arm on the
    // task's RESULT, outside the closure, and it is reported (and then
    // allowlisted) rather than swallowed.
    let fallback = lines(&[
        "pub async fn run(path: PathBuf) -> anyhow::Result<()> {",
        "    let text = match tokio::task::spawn_blocking(move || {",
        "        std::fs::read_to_string(&path)",
        "    })",
        "    .await",
        "    {",
        "        Ok(text) => text,",
        "        Err(_) => std::fs::read_to_string(&path)?,",
        "    };",
        "    Ok(())",
        "}",
    ]);
    let (hits, _) = scan("src/synthetic.rs", &fallback);
    assert_eq!(
        hits.len(),
        1,
        "the inline fallback outside the closure must still be reported, or the hand-off \
         would silently exempt the rest of the function: {hits:?}"
    );
    assert_eq!(hits[0].line, 8);

    // An awaited call is `tokio::process::Command`'s, not the standard
    // library's, and blocks nothing. Both spellings `rustfmt` produces.
    let awaited = lines(&[
        "pub async fn read(path: &Path) -> anyhow::Result<()> {",
        "    let one = tokio::process::Command::new(\"git\").output().await?;",
        "    let two = tokio::process::Command::new(\"git\")",
        "        .arg(\"status\")",
        "        .output()",
        "        .await?;",
        "    Ok(())",
        "}",
    ]);
    let (hits, _) = scan("src/synthetic.rs", &awaited);
    assert!(
        hits.is_empty(),
        "an awaited process call is a future, not a blocking call: {hits:?}"
    );

    // A synchronous body is out of scope entirely — the boundary is about a
    // runtime with other work to poll.
    let synchronous = lines(&[
        "pub fn run(path: &Path) -> anyhow::Result<()> {",
        "    let text = std::fs::read_to_string(path)?;",
        "    Ok(())",
        "}",
    ]);
    let (hits, examined) = scan("src/synthetic.rs", &synchronous);
    assert!(hits.is_empty(), "a plain `fn` is not this audit's subject");
    assert_eq!(examined, 0, "and none of it counts as examined");
}

#[test]
fn a_marker_that_merely_shares_letters_with_an_identifier_does_not_trip_the_guard() {
    // The guard-of-the-guard, inherited from `tests/spawn_seam_guard.rs`. A
    // plain `contains` reports a coincidence, and a guard that fires on a
    // coincidence is a guard somebody deletes.
    for line in [
        "    let entries = self.fs_read_dir(root)?;",
        "    async_fs::read_to_string(path).await",
    ] {
        assert!(
            !calls_marker(line, "read_dir(") || line.contains("fs::read_dir("),
            "an identifier that merely contains a marker's letters is not a call: {line}"
        );
    }
    assert!(!calls_marker("    let x = self.fs_read_dir(root);", "read_dir("));
    assert!(!calls_marker("    unflock(&file);", "flock("));

    // The control arm, and it is not optional: a boundary check that rejected
    // everything would pass the loop above while disabling the audit outright.
    assert!(calls_marker("    std::fs::read_dir(&root)?;", "fs::read_dir("));
    assert!(calls_marker("    flock(&file, FlockOperation::NonBlockingLockExclusive)", "flock("));

    // The generalisation this file needs over the sibling's rule: a marker that
    // begins with `.` carries its own left boundary, and applying the
    // identifier check to it would reject every real hit, since the character
    // before the `.` is always the end of a receiver name.
    assert!(calls_marker("    let mut child = wrap.spawn().map_err(|source| {", ".spawn()"));
    assert!(calls_marker(
        "    let status = std::process::Command::new(&editor).arg(&path).status();",
        ".status()"
    ));
    assert!(!calls_marker("    tokio::spawn(read_stdout(stdout, tx));", ".spawn()"));
}

#[test]
fn a_literal_carrying_a_brace_does_not_derail_the_brace_tracker() {
    // The failure this prevents is a false NEGATIVE, which is the worst kind a
    // guard can have: an unbalanced brace inside a string literal makes the
    // tracker leave the `async fn` body early, and every blocking call below it
    // goes unreported while the suite stays green.
    assert_eq!(strip_literals("    let open = '{';").matches('{').count(), 0);
    assert_eq!(strip_literals(r#"    let s = "a { b";"#).matches('{').count(), 0);
    assert_eq!(
        strip_literals(r##"    let s = r#"{"type":"user"}"#;"##)
            .matches('{')
            .count(),
        0
    );
    assert_eq!(strip_literals(r#"    let s = "he said \" { ";"#).matches('{').count(), 0);

    // A lifetime is not a character literal, and eating one would swallow the
    // rest of the line — including its braces.
    assert!(strip_literals("fn f<'a>(x: &'a str) -> Wrapper<'a> { x }").contains('{'));

    // And the tracker actually survives it end to end.
    let planted = lines(&[
        "pub async fn run(path: &Path) -> anyhow::Result<()> {",
        "    let brace = '{';",
        "    let json = \"{ unbalanced\";",
        "    let text = std::fs::read_to_string(path)?;",
        "    Ok(())",
        "}",
    ]);
    let (hits, _) = scan("src/synthetic.rs", &planted);
    assert_eq!(
        hits.len(),
        1,
        "the blocking read below two brace-carrying literals must still be found: {hits:?}"
    );
}

#[test]
fn an_in_source_test_module_is_outside_the_walk() {
    // The exclusion is a decision with a cost, so it gets an assertion rather
    // than only a comment: `TEST_MODULE_EXCLUSION` says what it is, and this
    // says it is real and bounded — production code before the module is still
    // walked, so the skip cannot swallow the file.
    let file = lines(&[
        "pub async fn run(path: PathBuf) -> anyhow::Result<()> {",
        "    let text = tokio::task::spawn_blocking(move || {",
        "        std::fs::read_to_string(&path)",
        "    })",
        "    .await??;",
        "    Ok(())",
        "}",
        "",
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "",
        "    #[tokio::test]",
        "    async fn a_fixture() {",
        "        let mut child = std::process::Command::new(\"sleep\").spawn().unwrap();",
        "        let _ = std::fs::read_to_string(\"/tmp/x\");",
        "    }",
        "}",
    ]);
    let (hits, examined) = scan("src/synthetic.rs", &file);
    assert!(
        hits.is_empty(),
        "a `#[tokio::test]` body owns its own runtime and blocks only itself \
         ({TEST_MODULE_EXCLUSION}): {hits:?}"
    );
    assert!(
        examined > 0,
        "the production `async fn` above the module must still have been examined, or \
         the exclusion is swallowing the file rather than the test module"
    );

    // The control arm: the same test-module body, with the `#[cfg(test)]`
    // removed, IS reported — so the exclusion is keyed on the attribute rather
    // than on the module being named `tests`, and a production `mod` cannot
    // hide behind the name.
    let uncfgd = lines(&[
        "mod tests {",
        "    pub async fn a_fixture() {",
        "        let _ = std::fs::read_to_string(\"/tmp/x\");",
        "    }",
        "}",
    ]);
    let (hits, _) = scan("src/synthetic.rs", &uncfgd);
    assert_eq!(
        hits.len(),
        1,
        "a module without the attribute is ordinary code and is walked: {hits:?}"
    );
}

fn lines(source: &[&str]) -> Vec<String> {
    source.iter().map(|line| line.to_string()).collect()
}
