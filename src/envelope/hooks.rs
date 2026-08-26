//! Hook stub generation, and the hook body the stub re-enters this binary for.
//!
//! **The generated file carries no policy logic** (D-04). It is a shebang, a
//! comment and one `exec` of the absolute binary path captured from
//! [`std::env::current_exe`] at generation time. Three consequences, and each is
//! why the obvious "generate a shell script that checks the refs" is declined:
//!
//! 1. Policy lives in Rust, where [`super::policy`] is unit-testable against
//!    hostile inputs with no repository and no child process.
//! 2. One binary and one version means the TUI, the driver and the hook can
//!    never disagree about what the policy is.
//! 3. A shell script full of `case` statements is precisely the artifact an
//!    agent rewrites — and a rewritten three-line stub that no longer execs this
//!    binary is a stub that does nothing, which the pushing side notices,
//!    whereas a rewritten `case` arm is invisible.
//!
//! Nothing here writes into the driven repository. The stub is delivered to git
//! through the environment instead; see [`super::cred::hooks_path_env`].

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use tempfile::NamedTempFile;

use super::policy::{self, ParkReason, PushVerdict};

/// What every park this module records says would unpark the run.
///
/// One word, and the same word for all of them, because Phase 19 owns
/// *producing* the reason and Phase 20 owns what happens next (D-24). A field
/// that tried to prescribe the recovery would have to be edited every time the
/// recovery changed, and then it is no longer a stable identifier.
const NEEDS_HUMAN: &str = "human";

/// Record a refusal in the active run's journal, **after** the verdict is
/// decided and **before** the process exits non-zero.
///
/// **The order is the decision.** Decide the verdict, write the refusal to
/// stderr, park, then exit non-zero — and the exit code does not depend on what
/// this returned. A refusal that could not be recorded is still a refusal;
/// making the exit conditional on the recording would let an agent that unset
/// [`super::cred::PROJECT_ROOT_ENV`] turn a park failure into a permitted
/// operation, which is exactly the inversion D-25 forbids.
///
/// `envelope::park` ([`super::park`]) writes its own second stderr line for any
/// outcome other than `Appended`, so silence is impossible here without deleting
/// a line inside `park` itself. It is the **one** appender every refusal in this
/// module reaches: `pre_push`, `pre_commit` and `guard` all come through here.
fn park_refusal(reason: ParkReason, detail: &str) {
    let _ = super::park(reason, NEEDS_HUMAN, detail);
}

/// The hook filenames git looks for, and the subdirectory it looks in.
pub const PRE_PUSH_HOOK: &str = "pre-push";
pub const PRE_COMMIT_HOOK: &str = "pre-commit";
const HOOKS_SUBDIR: &str = "hooks";

/// Every hook this envelope generates, and therefore every filename
/// [`assert_provenance_in`] will certify.
///
/// A closed list rather than "whatever file git happened to run": a stub named
/// something this envelope never generated has no sanctioned counterpart to be
/// compared against, and must be refused before the comparison is attempted.
const SANCTIONED_HOOKS: &[&str] = &[PRE_PUSH_HOOK, PRE_COMMIT_HOOK];

/// The marker delimiters of the envelope's `.git/info/exclude` block (D-23).
pub const EXCLUDE_BLOCK_START: &str = "# >>> gsd-meta-manager envelope >>>";
pub const EXCLUDE_BLOCK_END: &str = "# <<< gsd-meta-manager envelope <<<";

/// An all-zero object id: git's way of saying "there is nothing on that side".
fn is_zero_sha(sha: &str) -> bool {
    !sha.is_empty() && sha.chars().all(|c| c == '0')
}

/// Where this alias's hooks live, or `None` for a hostile alias.
///
/// The **one** definition of that path. [`install_in`] writes into it,
/// [`assert_provenance_in`] certifies against it, and
/// [`super::cred::build_env_in`] hands it to git as `core.hooksPath` — three
/// consumers whose disagreement would be a hook that is installed somewhere git
/// never looks, or certified against a directory it was not installed in.
pub fn hooks_dir_in(root: &Path, alias: &str) -> Option<PathBuf> {
    super::envelope_dir_in(root, alias).map(|dir| dir.join(HOOKS_SUBDIR))
}

/// Install the `pre-push` stub for `alias`, returning the hooks **directory**.
///
/// The directory rather than the file, because the directory is what
/// [`super::cred::hooks_path_env`] hands to git as `core.hooksPath`.
pub fn install(alias: &str) -> anyhow::Result<PathBuf> {
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so no hook stub can name it")?;
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!(
            "no application data directory is resolvable, and the envelope refuses \
             to fall back to a directory that could sit inside a repository"
        )
    })?;
    install_in(&root, alias, &binary)
}

/// [`install`] against an explicit envelope root and binary path.
///
/// Split out so the end-to-end fixture can install into a temporary directory
/// and name the built binary explicitly — under `cargo test`,
/// [`std::env::current_exe`] is the *test* binary, which would produce a stub
/// that execs something with no `envelope` subcommand.
pub fn install_in(root: &Path, alias: &str, binary: &Path) -> anyhow::Result<PathBuf> {
    let hooks_dir = hooks_dir_in(root, alias).ok_or_else(|| {
        anyhow!("refusing to install a hook for alias {alias:?}: not a plain path component")
    })?;
    std::fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("failed to create {}", hooks_dir.display()))?;

    // Both hooks, from one loop and one stub shape. `pre-commit` is not an
    // optional extra: it is the layer that sees a sweep BEFORE it becomes a
    // commit, and `pre-push` is its backstop for the commit that skipped it.
    for hook in SANCTIONED_HOOKS {
        write_stub(&hooks_dir, binary, alias, hook)?;
    }

    Ok(hooks_dir)
}

/// One stub, written atomically and made executable.
fn write_stub(hooks_dir: &Path, binary: &Path, alias: &str, hook: &str) -> anyhow::Result<()> {
    // The `config.rs:234-249` atomic-write idiom: a temp file in the *target*
    // directory, then `persist`. A half-written hook is a hook git would exec.
    let mut tmp = NamedTempFile::new_in(hooks_dir)
        .with_context(|| format!("failed to create a temp file in {}", hooks_dir.display()))?;
    tmp.write_all(stub_body(binary, alias, hook).as_bytes())
        .with_context(|| format!("failed to write the {hook} stub in {}", hooks_dir.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))
            .with_context(|| {
                format!("failed to make the {hook} stub executable in {}", hooks_dir.display())
            })?;
    }

    let path = hooks_dir.join(hook);
    tmp.persist(&path)
        .with_context(|| format!("failed to persist the hook stub to {}", path.display()))?;
    Ok(())
}

/// The three lines git will exec.
///
/// Both interpolated values are POSIX-quoted by [`sh_quote`] rather than wrapped
/// in double quotes.
///
/// **The reason this doc used to give became FALSE at D-19-2, and it is the kind
/// of falsehood a maintainer ACTS on** (WR-02, round 8). It said an alias is a
/// *plain path component*, "which is a weaker constraint than shell-safe —
/// `is_plain_path_component` accepts a quote character". It does not. Since the
/// alphabet clause at `src/journal/mod.rs:339` that predicate accepts only
/// `[A-Za-z0-9._-]` ([`crate::text::is_identity_char`]), so the quote, the
/// backtick, the dollar, the semicolon, the space, the backslash, the pipe and
/// every other shell metacharacter are refused. That is MEASURED, not asserted:
/// `tests::the_path_component_predicate_refuses_every_shell_metacharacter` in
/// this module answers `false` for sixteen of them and is the control the
/// sentence rests on. A reader who checked the old claim would have found it
/// false and concluded the quoting was redundant. Deleting a real defence
/// because its stated reason was stale is what this correction exists to prevent.
///
/// **Why the quoting stays anyway, given as a reason a maintainer can accept
/// rather than as an assertion.** Two things, and neither is caution:
///
/// 1. **It is the only defence here that survives a later WIDENING of the
///    alphabet.** The alphabet is a recorded PRODUCT decision with a stated trade
///    (`is_identity_char`: no non-ASCII alias), and its own doc calls reverting
///    the clause "costly, NOT one-way". A future round that relaxes it makes
///    every unquoted generated script wrong at once — in files that already sit
///    on disk and run on every push, which no rebuild touches.
/// 2. **A generated script is not a place to depend on a predicate defined three
///    modules away.** These three lines are read and executed by git, in a
///    separate process, outside this binary. The coupling would be invisible from
///    the file that breaks, and nothing in the stub could state it.
///
/// The quoting itself does not rest on this paragraph either:
/// `tests::a_quote_in_an_alias_cannot_escape_the_generated_stub` pins it directly.
///
/// The hook filename **is** the subcommand name, which is why there is one stub
/// shape rather than one per hook: a second template is a second place for the
/// generated file to stop being a three-line stub.
///
/// `--hook-path "$0"` is the one value that is deliberately **not** baked in.
/// A path recorded at generation time travels with a copy of the file, so a
/// relocated stub would hand [`assert_provenance`] the original's path and
/// certify itself. `$0` is the path the shell was actually invoked as, which is
/// the only value a copy cannot forge by being copied.
fn stub_body(binary: &Path, alias: &str, hook: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # gsd-meta-manager envelope hook. Policy lives in the binary below (D-04).\n\
         exec {} envelope {hook} {} --hook-path \"$0\"\n",
        sh_quote(&binary.to_string_lossy()),
        sh_quote(alias),
    )
}

/// POSIX single-quote `value` so no character in it can reach the shell.
///
/// `pub(super)` because [`super::cred::write_askpass_stub_in`] generates a stub
/// of the same shape and must quote it the same way. A second copy of this
/// three-line function would be a second place for the `'\''` escaping to be got
/// subtly wrong, in generated files that run on every push.
pub(super) fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Refuse unless this hook is the sanctioned one, running the binary it names.
///
/// Two facts, and the hook refuses if either fails (D-10):
///
/// 1. **The binary the stub was generated with still exists on disk.** On Linux
///    a running-but-deleted executable reports its path as `… (deleted)`, so a
///    binary replaced or removed under a live envelope is detectable here rather
///    than at the next exec.
/// 2. **The file this process was invoked from is the one inside *this alias's*
///    envelope hooks directory.** A copy of the hook relocated to some other
///    `hooksPath` must not silently become the sanctioned one.
///
/// Both sides are canonicalised before comparison, so a symlinked data directory
/// — `~/.local/share` pointed elsewhere is ordinary — does not read as a
/// relocation. Canonicalisation is safe *here* in a way it is not in
/// [`crate::journal::is_plain_path_component`]: this compares two paths that must both
/// already exist, rather than deciding whether a path that does not exist yet is
/// allowed to be created.
pub fn assert_provenance(alias: &str, invoked_from: &Path) -> anyhow::Result<()> {
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!("no application data directory is resolvable, so no hook can be sanctioned")
    })?;
    assert_provenance_in(&root, alias, invoked_from)
}

/// [`assert_provenance`] against an explicit envelope root.
pub fn assert_provenance_in(root: &Path, alias: &str, invoked_from: &Path) -> anyhow::Result<()> {
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so its provenance is unknown")?;
    if !binary.exists() {
        return Err(anyhow!(
            "the envelope binary at {} no longer exists on disk; refusing to act \
             as a hook for a build that has been removed or replaced",
            binary.display()
        ));
    }

    let hooks_dir = hooks_dir_in(root, alias).ok_or_else(|| {
        anyhow!("alias {alias:?} is not a plain path component, so it sanctions no hook")
    })?;

    // The hook's own filename selects which sanctioned file it is compared
    // against, so one provenance check covers every hook this envelope
    // generates. A filename that is not on the closed list has no sanctioned
    // counterpart at all, which is a refusal rather than a comparison.
    let name = invoked_from
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if !SANCTIONED_HOOKS.contains(&name) {
        return Err(anyhow!(
            "hook provenance failed: {name:?} is not a hook this envelope generates, \
             so there is no sanctioned hook to certify it against"
        ));
    }
    let sanctioned = hooks_dir.join(name);

    let sanctioned_real = sanctioned.canonicalize().with_context(|| {
        format!(
            "no sanctioned hook exists at {}, so nothing can be certified against it",
            sanctioned.display()
        )
    })?;
    let invoked_real = invoked_from.canonicalize().with_context(|| {
        format!("cannot resolve the hook this process ran from ({})", invoked_from.display())
    })?;

    if invoked_real != sanctioned_real {
        return Err(anyhow!(
            "hook provenance failed: this process ran from {} but the sanctioned \
             hook for alias {alias:?} is {}; a relocated copy does not inherit the \
             envelope's authority",
            invoked_real.display(),
            sanctioned_real.display()
        ));
    }

    Ok(())
}

/// The `pre-push` hook body: assert provenance, then run all three checks.
///
/// The order is the mechanism. Nothing below runs for a hook that has not proved
/// it is the sanctioned one, so a relocated copy cannot decide anything — not
/// even to allow.
///
/// Three independent refusals, and **all three run** rather than short-circuiting
/// on the first: a human whose push was blocked should learn everything that is
/// wrong with it in one round trip, not one problem per attempt.
///
/// 1. **The namespace verdict** (D-05), from the ref lines git supplies.
/// 2. **The full-worktree credential scan** (SAFE-03), whose report is printed
///    on both outcomes so the list of files it declined to read is visible even
///    when it found nothing.
/// 3. **The swept-path backstop** (D-22), over the paths the commits being
///    pushed actually touch. This is the layer that catches a commit made with
///    the verification step suppressed, which the `pre-commit` hook by
///    construction never saw.
pub fn pre_push(
    alias: &crate::registry::Alias,
    stdin: impl BufRead,
    invoked_from: &Path,
    repo_root: &Path,
) -> anyhow::Result<i32> {
    // Re-entry judges the alias it is handed (D-17-2): the generated stub's
    // argv is unchanged, but the binary it re-enters now insists the value can
    // still name an identity. The conversion happens in `main.rs`'s arm, which
    // fails closed with this hook's own exit code; by the time control is here
    // the value has been judged.
    let alias = alias.as_str();
    // A provenance mismatch is an envelope assertion that failed, and it is a
    // refusal like any other — so it parks like any other. Without this, the one
    // refusal that fires when the hook itself has been relocated would be the
    // one refusal leaving no trace, which is T-19-56 exactly.
    assert_provenance(alias, invoked_from).inspect_err(|_| {
        park_refusal(
            ParkReason::EnvelopeAssertionFailed,
            "a pre-push hook was invoked from a path the envelope does not sanction",
        );
    })?;

    let lines = read_ref_lines(stdin)?;
    let mut refused = classify_refs(alias, &lines);

    // The scan report goes to stderr on BOTH outcomes (D-14): a "clean" that
    // silently omitted forty unread files is the lie this phase argues against.
    let report = super::scan::scan_with_external(repo_root, super::scan::ScanLimits::default());
    eprint!("{}", report.render());
    if !report.is_clean() {
        eprintln!(
            "gsd-meta-manager envelope: REFUSED push — the worktree carries \
             credential-shaped content (reason: {})",
            policy::REASON_SECRET_DETECTED,
        );
        // The detail names neither the file nor the match: it reaches a journal,
        // and a detail that quoted the finding back would carry the credential
        // into the record (SAFE-04). The scan report above already names the
        // file, the line and the rule, on stderr, where a human is reading.
        park_refusal(
            ParkReason::SecretDetected,
            "the worktree carries credential-shaped content",
        );
        refused += 1;
    }

    refused += refuse_swept_paths(repo_root, &pushed_ranges(&lines));

    Ok(if refused > 0 { 1 } else { 0 })
}

/// The `pre-commit` hook body: assert provenance, then judge the staged paths.
///
/// Same order and same reason as [`pre_push`]. The staged list is read with a
/// read-only `git --no-optional-locks` invocation — the shape
/// [`crate::state_reader::git_ops`] already uses everywhere — because a hook
/// that mutated the repository to decide whether the repository may be mutated
/// would be its own counterexample.
///
/// **A staged list that cannot be read is a refusal, never an allow.** git
/// exits zero with empty output for a genuinely empty index, and this cannot
/// tell that apart from a failed read; the safe reading of the ambiguity is the
/// one that blocks, and an empty commit is not a thing a driven run needs.
pub fn pre_commit(
    alias: &crate::registry::Alias,
    invoked_from: &Path,
    repo_root: &Path,
) -> anyhow::Result<i32> {
    // See `pre_push`: judged in `main.rs`'s arm, unwrapped here.
    let alias = alias.as_str();
    // Same reason as [`pre_push`]'s: a refusal that leaves no trace is the one
    // failure a later reader cannot audit (T-19-56).
    assert_provenance(alias, invoked_from).inspect_err(|_| {
        park_refusal(
            ParkReason::EnvelopeAssertionFailed,
            "a pre-commit hook was invoked from a path the envelope does not sanction",
        );
    })?;

    let staged = crate::state_reader::git_ops::git_read_raw(
        repo_root,
        &["diff", "--cached", "--name-only", "-z"],
    )
    .ok_or_else(|| {
        park_refusal(
            ParkReason::EnvelopeAssertionFailed,
            "the staged path list could not be read, so the commit was refused",
        );
        anyhow!(
            "could not read the staged path list, so this commit cannot be judged; \
             refusing rather than allowing (reason: {})",
            policy::REASON_ENVELOPE_ASSERTION_FAILED
        )
    })?;

    let paths: Vec<PathBuf> = staged
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(PathBuf::from)
        .collect();

    Ok(if refuse_paths(repo_root, &paths, "commit") > 0 { 1 } else { 0 })
}

/// Refuse any of `paths` that [`policy::forbidden_repo_path`] names, returning
/// how many were refused.
///
/// One function for both hooks, so the two enforcement points cannot differ in
/// what they refuse **or** in what they say about it.
fn refuse_paths(repo_root: &Path, paths: &[PathBuf], operation: &str) -> usize {
    let mut refused = 0usize;
    for path in paths {
        let nested = has_nested_git(repo_root, path);
        let Some(reason) = policy::forbidden_repo_path(path, nested) else {
            continue;
        };
        let what = if nested {
            "sits inside a nested repository or linked worktree"
        } else {
            "is inside a directory the envelope reserves"
        };
        eprintln!(
            "gsd-meta-manager envelope: REFUSED {operation} — {} {what} \
             (reason: {})",
            path.display(),
            reason.as_str(),
        );
        // `reason` is the verdict `policy::forbidden_repo_path` already reached,
        // carried through rather than re-derived from the path a second time.
        park_refusal(reason, &format!("a refused {operation} path"));
        refused += 1;
    }
    refused
}

/// Whether any ancestor directory of `rel` itself contains a `.git` entry.
///
/// The repository's **own** root is excluded by construction — the walk stops
/// before the empty path — so this answers "is there a *second* repository
/// between here and the root", which is the question D-22 asks.
fn has_nested_git(repo_root: &Path, rel: &Path) -> bool {
    let mut current = rel.parent();
    while let Some(dir) = current {
        if dir.as_os_str().is_empty() {
            break;
        }
        if repo_root.join(dir).join(".git").exists() {
            return true;
        }
        current = dir.parent();
    }
    false
}

/// The paths the commits this push would introduce actually touch, refused via
/// the same predicate the commit hook uses.
fn refuse_swept_paths(repo_root: &Path, ranges: &[(String, String)]) -> usize {
    let mut paths: Vec<PathBuf> = Vec::new();

    for (local_sha, remote_sha) in ranges {
        // A deletion introduces no commits, so there is nothing to inspect.
        if is_zero_sha(local_sha) {
            continue;
        }
        // An all-zero remote sha means the remote has no such ref yet, so there
        // is no range to diff against; the commit itself is what is new.
        let output = if is_zero_sha(remote_sha) {
            crate::state_reader::git_ops::git_read_raw(
                repo_root,
                &["show", "--name-only", "--format=", local_sha],
            )
        } else {
            crate::state_reader::git_ops::git_read_raw(
                repo_root,
                &["diff", "--name-only", &format!("{remote_sha}..{local_sha}")],
            )
        };

        let Some(output) = output else { continue };
        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let path = PathBuf::from(line);
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }

    refuse_paths(repo_root, &paths, "push")
}

/// Read git's ref lines into memory, because two checks need them.
fn read_ref_lines(stdin: impl BufRead) -> anyhow::Result<Vec<String>> {
    let mut lines = Vec::new();
    for (index, line) in stdin.lines().enumerate() {
        lines.push(
            line.with_context(|| format!("failed to read pre-push ref line {}", index + 1))?,
        );
    }
    Ok(lines)
}

/// The `(local sha, remote sha)` pairs of every well-formed ref line.
///
/// Malformed lines are **not** silently dropped here — [`classify_refs`] has
/// already refused them. This function's only job is to answer "which commits
/// would this push introduce", and a line it cannot parse contributes no
/// commits to that answer while the push is already being refused.
fn pushed_ranges(lines: &[String]) -> Vec<(String, String)> {
    lines
        .iter()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            (fields.len() == 4).then(|| (fields[1].to_string(), fields[3].to_string()))
        })
        .collect()
}

/// Write the envelope's ignore block into `.git/info/exclude` (D-23).
///
/// **This is the single persistent mutation the envelope makes to a driven
/// repository**, and it is an explicit exception to the touch-nothing posture
/// rather than one quietly taken. Everything else the envelope does is
/// process-scoped and vanishes with the process; this survives.
///
/// It is justified because an ignore rule is **inert for the human**: it changes
/// no tracked file, no configuration git reads for behaviour, and nothing the
/// user would notice except that `git status` stops offering to commit the
/// envelope's own leavings. It is made safe by being idempotent, delimited by an
/// unmistakable marker block, and **rewritten in place rather than appended**,
/// so a thousand runs leave exactly one block.
///
/// **Why not the repository's tracked ignore file.** `.git/info/exclude` is
/// untracked and per-repository, so the agent cannot commit a change to it,
/// cannot push it, and cannot leak these paths to the remote by editing a
/// tracked file. Writing the same patterns into a tracked ignore file would
/// have put the envelope's own posture into the repository's history, where the
/// agent can edit it, and where the edit is a commit the push hook would then
/// have to police. This location prevents the failure rather than punishing it.
pub fn write_exclude_block(project_root: &Path) -> anyhow::Result<()> {
    let info_dir = git_dir(project_root)?.join("info");
    std::fs::create_dir_all(&info_dir)
        .with_context(|| format!("failed to create {}", info_dir.display()))?;
    let exclude = info_dir.join("exclude");

    let existing = std::fs::read_to_string(&exclude).unwrap_or_default();
    let mut kept: Vec<&str> = Vec::new();
    let mut inside_block = false;
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed == EXCLUDE_BLOCK_START {
            inside_block = true;
            continue;
        }
        if trimmed == EXCLUDE_BLOCK_END {
            inside_block = false;
            continue;
        }
        if !inside_block {
            kept.push(line);
        }
    }
    // Trailing blank lines are dropped so the second run produces byte-identical
    // output to the first. Without this the block would drift down the file by
    // one line per invocation, which is exactly the accumulation D-23 forbids.
    while kept.last().map(|line| line.trim().is_empty()).unwrap_or(false) {
        kept.pop();
    }

    let mut out = String::new();
    for line in kept {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(EXCLUDE_BLOCK_START);
    out.push('\n');
    out.push_str(
        "# Managed by gsd-meta-manager. Rewritten in place on every run; edits\n\
         # inside this block are discarded. Delete the whole block to opt out.\n",
    );
    for prefix in policy::forbidden_repo_prefixes() {
        out.push_str(prefix);
        out.push_str("/\n");
    }
    out.push_str(EXCLUDE_BLOCK_END);
    out.push('\n');

    let mut tmp = NamedTempFile::new_in(&info_dir)
        .with_context(|| format!("failed to create a temp file in {}", info_dir.display()))?;
    tmp.write_all(out.as_bytes())
        .with_context(|| format!("failed to write {}", exclude.display()))?;
    tmp.persist(&exclude)
        .with_context(|| format!("failed to persist {}", exclude.display()))?;
    Ok(())
}

/// The repository's git directory, whether `.git` is a directory or the pointer
/// file a linked worktree carries.
///
/// The pointer case is not hypothetical: a driven run inside a linked worktree
/// has a `.git` **file**, and a `write_exclude_block` that assumed a directory
/// would silently write into a path that is not an exclude file at all.
fn git_dir(project_root: &Path) -> anyhow::Result<PathBuf> {
    let dot_git = project_root.join(".git");
    let meta = std::fs::metadata(&dot_git)
        .with_context(|| format!("{} is not a git repository", project_root.display()))?;
    if meta.is_dir() {
        return Ok(dot_git);
    }

    let pointer = std::fs::read_to_string(&dot_git)
        .with_context(|| format!("failed to read {}", dot_git.display()))?;
    let target = pointer
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:"))
        .map(str::trim)
        .ok_or_else(|| anyhow!("{} carries no `gitdir:` pointer", dot_git.display()))?;

    let target = PathBuf::from(target);
    Ok(if target.is_absolute() {
        target
    } else {
        project_root.join(target)
    })
}

/// Read git's ref lines and return the exit code, with no provenance check.
///
/// Separate from [`pre_push`] so the classification can be unit-tested against
/// hostile stdin without installing a hook on disk first. Nothing outside this
/// module may call it: a caller that skipped [`assert_provenance`] would be
/// re-opening exactly the seam D-10 closes.
///
/// git supplies `<local-ref> <local-sha> <remote-ref> <remote-sha>` on stdin,
/// **regardless of how git was invoked** — from a nested shell, from a Makefile,
/// from a script the agent wrote. That is why this is the layer that observes
/// ground truth rather than an argv the tool boundary happened to see.
///
/// Two behaviours that look like details and are not:
///
/// - A **deletion** line (an all-zero local sha) is still classified by its
///   destination ref. Deleting `main` is a write to `main`.
/// - An **unparseable** line is refused, never skipped. A hook that cannot read
///   its input must not allow; skipping would make a malformed line the cheapest
///   possible bypass.
///
/// The return value is how many refs were refused, and a non-zero count becomes
/// a non-zero exit — and the exit code **is** the control (D-25), not the
/// message, which is only there so a human reading a failed push knows what
/// happened.
fn classify_refs(alias: &str, lines: &[String]) -> usize {
    let namespace = policy::default_namespace(alias);
    let mut refused = 0usize;

    for (index, line) in lines.iter().enumerate() {
        // A line with no tokens carries no ref; it is nothing, not something
        // unreadable. Anything else with the wrong field count is refused below.
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 4 {
            eprintln!(
                "gsd-meta-manager envelope: REFUSED — pre-push line {} is unreadable \
                 ({} fields, expected 4) (reason: {})",
                index + 1,
                fields.len(),
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
            );
            park_refusal(
                ParkReason::EnvelopeAssertionFailed,
                "a pre-push ref line could not be read",
            );
            refused += 1;
            continue;
        }

        match policy::classify_push_ref(fields[2], &namespace) {
            PushVerdict::Allow => {}
            PushVerdict::Refuse { reason, ref_name } => {
                eprintln!(
                    "gsd-meta-manager envelope: REFUSED push to {ref_name} \
                     (reason: {reason}); a driven run may push only inside {namespace}",
                );
                // `PushVerdict::Refuse` carries only `REASON_PUSH_OUTSIDE_NAMESPACE`
                // — this branch has exactly one reason and the debug assertion
                // below is what keeps that true if the verdict ever grows a
                // second. Deriving the park reason a second way is how a journal
                // starts disagreeing with the refusal that produced it.
                debug_assert_eq!(reason, policy::REASON_PUSH_OUTSIDE_NAMESPACE);
                park_refusal(
                    ParkReason::PushOutsideNamespace,
                    "a push destination outside the reserved namespace",
                );
                refused += 1;
            }
        }
    }

    refused
}

// ---------------------------------------------------------------------------
// D-06 layer 2: the `PreToolUse` guard
// ---------------------------------------------------------------------------

/// The largest guard request this process will read, in bytes.
///
/// A bound rather than an unbounded `read_to_string`, for the same reason
/// `crate::journal::reader` bounds its line reads: the writer is on the other
/// side of a pipe, and a guard that can be made to read forever is a guard that
/// can be made to hang — which is the failure mode this whole module is written
/// against. A request past the bound is refused, never truncated-and-judged:
/// judging a prefix of a command is worse than refusing it.
const MAX_GUARD_REQUEST_BYTES: u64 = 1 << 20;

/// How deep the guard follows `sh -c` payloads.
///
/// One level covers `bash -c "git push --force"`, which is an ordinary thing for
/// an agent to write and not an evasion at all. Deeper nesting is bounded rather
/// than followed: the cost of recursion here is latency on the critical path,
/// and an agent nesting three shells to hide a push is doing something layer 3
/// exists for.
const MAX_SHELL_RECURSION: usize = 2;

/// The shells whose `-c` payload the guard re-splits and re-classifies.
const NESTED_SHELLS: &[&str] = &["sh", "bash", "zsh", "dash", "ksh"];

/// One `PreToolUse` request, with unknown fields preserved rather than rejected.
///
/// The tolerant posture `crate::executor::stream_json` established: the agent
/// CLI adds fields between releases, and a guard that failed to deserialise a
/// request carrying a field it had not heard of would **deny every tool call**
/// the day the CLI grew one. Preserving is what keeps the tolerance from
/// becoming an allow — nothing is dropped, it is simply not required.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GuardRequest {
    /// The tool about to run — `Bash`, `Write`, `Edit`, …
    #[serde(default)]
    pub tool_name: String,
    /// The tool's own input object. For `Bash` it carries `command`.
    #[serde(default)]
    pub tool_input: serde_json::Value,
    /// Everything else the CLI sent, kept verbatim.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The `PreToolUse` guard: split the command, apply the shared classifiers, and
/// answer before the tool runs (D-06 layer 2).
///
/// **Latency is a first-class requirement here, not a nicety, and the reason is
/// recorded in this repository rather than imagined.** `src/executor/mod.rs:225-239`
/// records a *reproduced* 180-240 second hang caused by `PreToolUse` hooks that
/// ran synchronously on the agent's critical path with no timeout, and
/// `--setting-sources project` is the shipped mitigation. The envelope's own
/// hook must not reintroduce the bug the envelope was born from. Three rules
/// follow from that, and each is enforced rather than intended:
///
/// 1. **No network in the guard path at all.** The remote-protection probe
///    happens once at run start, never per tool call. Nothing below opens a
///    socket, and a test asserts the function body names no HTTP client.
/// 2. **The repository is consulted for exactly one command shape** — a `push`
///    with no refspec, which cannot be judged without its resolved destination
///    ([`policy::push_needs_resolved_dests`]). Every other command is decided
///    from argv alone, and the context is resolved at most once per invocation.
/// 3. **The ledger read is a single pass over one append-only file.**
///
/// The registered timeout that backs all three lives in the settings file
/// [`write_settings`] generates, as [`GUARD_TIMEOUT_SECS`].
///
/// **Failure is denial, never permission.** A request that will not parse, a
/// command whose words cannot be recovered, a verb assembled from an expansion,
/// an `eval` — each is refused. A guard that permitted what it could not
/// understand would be a guard that an unparseable command walks straight
/// through.
///
/// **A permit writes nothing at all**, and that is deliberate rather than
/// lazy: answering `"permissionDecision": "allow"` would make this deny-only
/// control into an approval authority, auto-approving commands the user's own
/// permission rules would otherwise have prompted for. Staying silent leaves
/// the ordinary permission flow exactly where it was.
pub fn guard(alias: &crate::registry::Alias, stdin: impl std::io::Read) -> anyhow::Result<i32> {
    // See `pre_push`: judged in `main.rs`'s arm, unwrapped here. `guard_in`
    // keeps `&str` — it is the explicit-roots variant the suite drives directly,
    // and it creates nothing.
    let alias = alias.as_str();
    let root = super::envelope_root().ok_or_else(|| {
        // The guard's own refusal-before-a-verdict. It parks for the same reason
        // every other refusal does: an unrecorded refusal is one a later reader
        // cannot tell from a tool call that was never made (T-19-56).
        park_refusal(
            ParkReason::EnvelopeAssertionFailed,
            "the guard could not resolve an envelope root, so it refused the tool call",
        );
        anyhow!(
            "no application data directory is resolvable, so no pull-request ledger can \
             be consulted and the guard refuses rather than permitting unbounded"
        )
    })?;
    let project_root = std::env::var_os(super::cred::PROJECT_ROOT_ENV).map(PathBuf::from);

    guard_in(
        &root,
        &crate::config::Config::default_path(),
        alias,
        project_root.as_deref(),
        stdin,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )
}

/// [`guard`] against explicit roots and streams.
///
/// Split out for the same reason [`install_in`] is: a test may not write into
/// the developer's real `~/.local/share`, and a decision that can only be
/// observed by spawning a process is a decision that gets tested once.
#[allow(clippy::too_many_arguments)]
pub fn guard_in(
    root: &Path,
    config_path: &Path,
    alias: &str,
    project_root: Option<&Path>,
    stdin: impl std::io::Read,
    out: &mut impl Write,
    err: &mut impl Write,
) -> anyhow::Result<i32> {
    let request = match read_guard_request(stdin) {
        Ok(request) => request,
        Err(reason) => return deny(out, err, ParkReason::EnvelopeAssertionFailed, &reason),
    };

    // Only a shell tool carries a command to classify. Anything else is layer
    // 1's business (`--disallowedTools` and the settings `deny` list), and
    // denying it here would be this layer answering a question it was not asked.
    if request.tool_name != "Bash" {
        return Ok(0);
    }

    let Some(command) = request
        .tool_input
        .get("command")
        .and_then(|value| value.as_str())
    else {
        return deny(
            out,
            err,
            ParkReason::EnvelopeAssertionFailed,
            "this Bash request carries no readable `command`, so nothing about it can be \
             classified; an unjudgeable command is refused rather than permitted",
        );
    };

    let Some(segments) = policy::split_segments(command) else {
        return deny(
            out,
            err,
            ParkReason::EnvelopeAssertionFailed,
            "the command's words cannot be recovered — an unterminated quote or a trailing \
             line continuation — so it cannot be classified and is refused",
        );
    };

    let policy = resolve_policy(config_path, alias);
    // Resolved at most once per invocation, and only if some segment turns out
    // to need it. `None` means "not asked for yet", not "unresolvable".
    let mut push_ctx: Option<policy::GitContext> = None;

    match classify_segments(
        &segments,
        0,
        root,
        alias,
        project_root,
        &policy,
        &mut push_ctx,
    ) {
        // The park reason travels WITH the refusal the classifier reached, so
        // the journal cannot disagree with the message about which boundary was
        // crossed.
        Ok(Some((park, reason))) => deny(out, err, park, &reason),
        Ok(None) => Ok(0),
        // An error reaching a verdict is not a verdict. The ledger could not be
        // written, or the window could not be computed — either way the attempt
        // is unbounded, and unbounded is exactly what the cap exists to prevent.
        Err(error) => deny(
            out,
            err,
            ParkReason::EnvelopeAssertionFailed,
            &format!(
                "the envelope could not reach a verdict for this command, so it is refused \
                 (reason: {}): {}",
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                crate::journal::redact::redact(&error.to_string())
            ),
        ),
    }
}

/// Classify every simple command in `segments`, returning the first refusal as
/// its **park reason and its message together**.
///
/// The pair rather than the message alone (D-24): the caller has to park under
/// the reason this function decided, and a caller that inferred one from the
/// message would be deriving the same fact a second way — which is how a journal
/// comes to disagree with the refusal it records.
///
/// `depth` bounds the `sh -c` recursion at [`MAX_SHELL_RECURSION`].
#[allow(clippy::too_many_arguments)]
fn classify_segments(
    segments: &[Vec<policy::Token>],
    depth: usize,
    root: &Path,
    alias: &str,
    project_root: Option<&Path>,
    envelope: &policy::EnvelopePolicy,
    push_ctx: &mut Option<policy::GitContext>,
) -> anyhow::Result<Option<(ParkReason, String)>> {
    for segment in segments {
        let Some(program_token) = segment.first() else {
            continue;
        };
        let words: Vec<&str> = segment.iter().map(|token| token.text.as_str()).collect();
        let program = policy::program_name(words[0]);

        // A verb the shell will assemble at run time is a verb this function
        // cannot see. `layer 2 raises the cost of an accident`; it does not
        // pretend to evaluate a shell.
        if program_token.expansion {
            return Ok(Some((
                ParkReason::EnvelopeAssertionFailed,
                format!(
                    "this command's program is assembled by shell expansion, so what it will \
                     run is not knowable before it runs; refused rather than guessed at \
                     (reason: {})",
                    policy::REASON_ENVELOPE_ASSERTION_FAILED
                ),
            )));
        }
        if program == "eval" {
            return Ok(Some((
                ParkReason::EnvelopeAssertionFailed,
                format!(
                    "`eval` builds a command at run time, so no classifier can see what it \
                     will run; refused (reason: {})",
                    policy::REASON_ENVELOPE_ASSERTION_FAILED
                ),
            )));
        }

        // `bash -c "git push --force"` is an ordinary thing to write, not an
        // evasion, so the payload is classified rather than the wrapper.
        if NESTED_SHELLS.contains(&program) {
            if let Some(payload) = shell_c_payload(&words) {
                if depth >= MAX_SHELL_RECURSION {
                    return Ok(Some((
                        ParkReason::EnvelopeAssertionFailed,
                        format!(
                            "this command nests shells more deeply than the guard follows, so \
                             its innermost command cannot be classified; refused (reason: {})",
                            policy::REASON_ENVELOPE_ASSERTION_FAILED
                        ),
                    )));
                }
                let Some(inner) = policy::split_segments(payload) else {
                    return Ok(Some((
                        ParkReason::EnvelopeAssertionFailed,
                        "the nested shell payload's words cannot be recovered, so it cannot \
                         be classified and is refused"
                            .to_string(),
                    )));
                };
                if let Some(refusal) = classify_segments(
                    &inner,
                    depth + 1,
                    root,
                    alias,
                    project_root,
                    envelope,
                    push_ctx,
                )? {
                    return Ok(Some(refusal));
                }
                continue;
            }
        }

        if program == "git" {
            let rest: Vec<&str> = words[1..].to_vec();
            if policy::push_needs_resolved_dests(&rest) && push_ctx.is_none() {
                let repo = project_root
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."));
                *push_ctx = Some(policy::resolve_push_context(&repo, &envelope.namespace));
            }
            let ctx = push_ctx.clone().unwrap_or_else(|| policy::GitContext {
                namespace: envelope.namespace.clone(),
                resolved_push_dests: Vec::new(),
            });
            if let policy::GitVerdict::Refuse { reason, detail } = policy::classify_git(&rest, &ctx)
            {
                // The classifier's own reason, carried out whole.
                return Ok(Some((
                    reason,
                    format!(
                        "gsd-meta-manager envelope: REFUSED (reason: {}) — {detail}",
                        reason.as_str()
                    ),
                )));
            }
            continue;
        }

        if let Some((platform, label)) = policy::pr_command_label(&words) {
            let entry = super::ledger::LedgerEntry {
                at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                run_id: current_run_id(),
                command: label,
                platform: platform.to_string(),
            };
            let verdict = super::ledger::record_and_check_in(root, alias, &entry, envelope)?;
            if let Some(detail) = verdict.refusal_detail() {
                return Ok(Some((
                    ParkReason::PrCapExceeded,
                    format!(
                        "gsd-meta-manager envelope: REFUSED (reason: {}) — {detail}",
                        policy::REASON_PR_CAP_EXCEEDED
                    ),
                )));
            }
        }
    }

    Ok(None)
}

/// The `-c` payload of a nested shell invocation, if this is one.
fn shell_c_payload<'a>(words: &[&'a str]) -> Option<&'a str> {
    let index = words.iter().position(|word| *word == "-c")?;
    words.get(index + 1).copied()
}

/// The run this guard invocation belongs to, for the per-run cap.
///
/// Read from the environment the driver builds for the child, because the guard
/// is a fresh process per tool call and has no other way to know. An absent
/// value yields a stable placeholder rather than a fresh id: a **fresh** id per
/// invocation would silently reset the per-run cap on every tool call, which is
/// the one failure mode a per-run cap cannot survive.
fn current_run_id() -> String {
    std::env::var(super::cred::RUN_ID_ENV).unwrap_or_else(|_| "unattributed-run".to_string())
}

/// This alias's envelope settings, defaults applied.
///
/// **An unreadable registry or an unregistered alias resolves to the defaults
/// rather than to an error**, and the direction is what makes that safe: every
/// default is the *tighter* value — the reserved namespace and the 3/1 caps —
/// so a guard that cannot read configuration confines the run more, never less.
/// The alternative, refusing every tool call because a config file moved, is a
/// control that fails into unusability and therefore gets switched off.
fn resolve_policy(config_path: &Path, alias: &str) -> policy::EnvelopePolicy {
    let opt_in = crate::config::load_config(config_path)
        .ok()
        .and_then(|config| {
            config
                .projects
                .get(alias)
                .and_then(|project| project.driver_opt_in.clone())
        });

    match opt_in {
        Some(opt_in) => policy::EnvelopePolicy::resolve(alias, &opt_in),
        None => policy::EnvelopePolicy {
            namespace: policy::default_namespace(alias),
            pr_cap_per_24h: policy::DEFAULT_PR_CAP_PER_24H,
            pr_cap_per_run: policy::DEFAULT_PR_CAP_PER_RUN,
            credential: None,
        },
    }
}

/// Read one request from the guard's stdin, bounded.
fn read_guard_request(stdin: impl std::io::Read) -> Result<GuardRequest, String> {
    use std::io::Read as _;

    let mut text = String::new();
    stdin
        .take(MAX_GUARD_REQUEST_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| format!("the guard could not read its request: {error}"))?;

    if text.len() as u64 > MAX_GUARD_REQUEST_BYTES {
        return Err(format!(
            "the guard request is larger than {MAX_GUARD_REQUEST_BYTES} bytes; judging a \
             prefix of a command is worse than refusing it, so it is refused"
        ));
    }

    serde_json::from_str(&text).map_err(|error| {
        format!("the guard request is not JSON it can read ({error}), so it is refused")
    })
}

/// Write the deny decision and return the exit code.
///
/// **Two carriers, deliberately.** The JSON on stdout is the documented hook
/// protocol; the non-zero exit with the reason on stderr is what still blocks if
/// the protocol's shape ever drifts under us. A control with one carrier is a
/// control that a schema change silently disarms — the same argument D-07 makes
/// about the settings file, applied to the guard's own answer.
fn deny(
    out: &mut impl Write,
    err: &mut impl Write,
    park: ParkReason,
    reason: &str,
) -> anyhow::Result<i32> {
    let decision = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    });
    writeln!(out, "{decision}").context("failed to write the guard's decision")?;
    writeln!(err, "{reason}").context("failed to mirror the guard's refusal")?;

    // **Third, and after the verdict is already settled.** The two carriers
    // above are the control; this is the evidence (D-24, D-25). The exit code
    // below does not depend on what it returns, which is the property that
    // stops an agent from turning a suppressed park into a permitted command.
    //
    // The detail names the reason and nothing else. `reason` above is the
    // guard's message and can quote a token out of the command line it refused;
    // that belongs on this process's stderr, where the user is, and not in a
    // journal (SAFE-04).
    park_refusal(park, "a tool call the envelope refused");

    // Exit code 2 is the hook protocol's blocking error, and it is the carrier
    // that does not depend on the JSON being understood.
    Ok(2)
}

// ---------------------------------------------------------------------------
// The settings file that delivers the guard (D-07)
// ---------------------------------------------------------------------------

/// The generated settings file's name inside the alias's envelope directory.
const SETTINGS_FILE: &str = "settings.json";

/// The `PreToolUse` guard's registered timeout, in seconds.
///
/// **Five seconds, and the number has a reason rather than a shrug.** The guard
/// does no network I/O, reads one append-only file in one pass, and consults the
/// repository for exactly one command shape; on any machine that can run the
/// agent at all its work is milliseconds. Five seconds is therefore not a budget
/// it is expected to use — it is the ceiling past which something has gone wrong
/// (a filesystem hang, a `git` that will not return) and the run is better off
/// losing the guard than losing the agent.
///
/// The alternative — no `timeout` key at all — is exactly the configuration that
/// produced the **reproduced** 180-240 second hang recorded at
/// `src/executor/mod.rs:225-239`, where the user's own global `PreToolUse` hooks
/// ran unbounded on the agent's critical path. `<specifics>` names re-creating
/// that the single biggest self-inflicted-wound risk of this phase, so the value
/// is asserted by a test rather than trusted to stay in the literal.
pub const GUARD_TIMEOUT_SECS: u64 = 5;

/// The tool the guard is registered against.
const GUARD_MATCHER: &str = "Bash";

/// The whole generated settings file, as a value.
///
/// **Typed, never templated** (D-07). Every byte of this file is produced by
/// `serde_json` from this struct tree. There is no `format!`, no string
/// concatenation and no template file anywhere in its generation, because a
/// templated settings file is a settings file with a hand-made syntax error
/// waiting in it — and the agent CLI, in print mode, **silently ignores a
/// settings file that fails validation, with no error dialog**. A syntax error
/// here does not announce itself; it disarms the layer without a word.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EnvelopeSettings {
    /// The tool denylist, layer 1's list carried a second time.
    pub permissions: SettingsPermissions,
    /// The hook registrations, layer 2's delivery.
    pub hooks: SettingsHooks,
}

/// The `permissions` object.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SettingsPermissions {
    /// Patterns the child may not run. Always [`policy::disallowed_tools`], so
    /// the file and the argv flag cannot drift apart.
    pub deny: Vec<String>,
}

/// The `hooks` object.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SettingsHooks {
    /// The event this envelope registers for. Renamed rather than spelled in
    /// snake_case, because the consumer's key is `PreToolUse` and a serde
    /// attribute is checkable where a hand-typed key is not.
    #[serde(rename = "PreToolUse")]
    pub pre_tool_use: Vec<HookMatcher>,
}

/// One matcher and the hooks it fires.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HookMatcher {
    /// The tool name pattern this entry applies to.
    pub matcher: String,
    /// The commands to run, in order.
    pub hooks: Vec<HookCommand>,
}

/// One registered command, with its timeout.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HookCommand {
    /// Always `command`; `type` is a Rust keyword, hence the rename.
    #[serde(rename = "type")]
    pub kind: String,
    /// The shell command line the CLI runs.
    pub command: String,
    /// Seconds. **Never absent** — see [`GUARD_TIMEOUT_SECS`].
    pub timeout: u64,
}

/// The settings value for one alias.
///
/// ## Every control here has a second carrier (D-07)
///
/// The rule this file is generated under is that it must **never be the sole
/// carrier of any control**, because the consumer ignores an invalid settings
/// file silently. Per control, its second carrier:
///
/// | Control in this file | Second carrier |
/// |---|---|
/// | `permissions.deny` on `Bash(git push:*)` and the rest of D-08's verbs | the same list on argv as `--disallowedTools` — [`policy::disallowed_tools`] is the single source both read |
/// | `permissions.deny` on `Write`/`Edit` of `.claude/**` | the same list on argv, again from [`policy::disallowed_tools`] |
/// | the push boundary the guard enforces | the `pre-push` git hook ([`pre_push`]), which sees the refs git actually pushes regardless of how git was invoked |
/// | the swept-worktree boundary | the `pre-commit` git hook ([`pre_commit`]), plus `pre-push` as its backstop |
/// | the file's own delivery | [`settings_json`] renders the identical value for `--settings` to take **inline on argv**, where a file cannot go missing or be corrupted on disk |
///
/// **One control genuinely has no git-hook counterpart, and saying so is the
/// point of this table rather than a hole in it: the pull-request cap.** No git
/// hook observes `gh pr create`, because it is not a git operation. If this file
/// were ignored, the cap would degrade to unenforced while every push boundary
/// stayed standing — which is the "degrade, never disarm" outcome D-07 requires,
/// but it is a degradation and it is recorded here rather than papered over.
/// The mitigations that remain for it are the argv delivery above and the
/// round-trip check in [`write_settings_in`].
pub fn settings_value(binary: &Path, alias: &str) -> EnvelopeSettings {
    EnvelopeSettings {
        permissions: SettingsPermissions {
            deny: policy::disallowed_tools(),
        },
        hooks: SettingsHooks {
            pre_tool_use: vec![HookMatcher {
                matcher: GUARD_MATCHER.to_string(),
                hooks: vec![HookCommand {
                    kind: "command".to_string(),
                    command: guard_command(binary, alias),
                    timeout: GUARD_TIMEOUT_SECS,
                }],
            }],
        },
    }
}

/// The one string in the settings file that is composed rather than serialised.
///
/// A command line is a string by nature; the distinction D-07 draws is that the
/// **JSON** is never assembled from text, and it is not. Both interpolated
/// values are POSIX-quoted by [`sh_quote`] — the same helper the hook stubs use,
/// for the same reason: an alias is a plain path component, which is a weaker
/// constraint than shell-safe.
fn guard_command(binary: &Path, alias: &str) -> String {
    format!(
        "{} envelope guard {}",
        sh_quote(&binary.to_string_lossy()),
        sh_quote(alias)
    )
}

/// The settings value as the JSON text `--settings` accepts inline.
///
/// The argv-carried half of the delivery. A file can be missing, relocated or
/// truncated on disk; an argv element cannot, which is the same property D-06
/// gives layer 1 over layer 2.
pub fn settings_json(binary: &Path, alias: &str) -> anyhow::Result<String> {
    serde_json::to_string(&settings_value(binary, alias))
        .context("failed to render the envelope settings as JSON")
}

/// Generate `<envelope>/<alias>/settings.json`, verified by reading it back.
///
/// See [`write_settings_in`] for the three mitigations this rests on.
pub fn write_settings(alias: &str) -> anyhow::Result<PathBuf> {
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so no settings file can name it")?;
    let root = super::envelope_root().ok_or_else(no_data_directory)?;
    write_settings_in(&root, alias, &binary)
}

/// [`write_settings`] against an explicit envelope root and binary path.
///
/// **The write is followed immediately by a read-back and a comparison, and a
/// mismatch refuses the run** rather than warning. That ordering is the whole
/// mitigation: the agent CLI silently ignores a settings file that fails
/// validation in print mode, so a file that is wrong on disk produces a run with
/// no `PreToolUse` layer and no indication that anything happened. The only
/// moment this process can still tell the difference is right here, before the
/// child is spawned — a warning logged now would be read after the run that
/// needed it.
pub fn write_settings_in(root: &Path, alias: &str, binary: &Path) -> anyhow::Result<PathBuf> {
    let dir = super::envelope_dir_in(root, alias).ok_or_else(|| hostile_alias(alias))?;
    std::fs::create_dir_all(&dir).context("failed to create the alias's envelope directory")?;

    let settings = settings_value(binary, alias);
    let path = dir.join(SETTINGS_FILE);
    persist_settings(&path, &settings)?;
    verify_settings(&path, &settings)?;

    Ok(path)
}

/// The error for an envelope root that cannot be resolved.
fn no_data_directory() -> anyhow::Error {
    anyhow!(
        "no application data directory is resolvable, and the envelope refuses to write \
         its settings into a directory that could sit inside a repository"
    )
}

/// The error for an alias that is not a plain path component.
fn hostile_alias(alias: &str) -> anyhow::Error {
    anyhow!("alias {alias:?} is not a plain path component, so it sanctions no settings file")
}

/// Serialise and persist atomically, the `config.rs:234-249` idiom.
fn persist_settings(path: &Path, settings: &EnvelopeSettings) -> anyhow::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow!("the settings path has no parent directory"))?;
    let rendered = serde_json::to_vec_pretty(settings)
        .context("failed to render the envelope settings as JSON")?;

    let mut tmp = NamedTempFile::new_in(dir)
        .with_context(|| format!("failed to create a temp file in {}", dir.display()))?;
    tmp.write_all(&rendered)
        .with_context(|| format!("failed to write {}", path.display()))?;
    tmp.persist(path)
        .with_context(|| format!("failed to persist {}", path.display()))?;
    Ok(())
}

/// Read the file back, deserialise it into the same struct tree, and compare.
///
/// **Any mismatch is an error, never a warning.** Public so the same check can
/// be re-run against a file that has been on disk since it was written — which
/// is what a test corrupting one byte exercises, and what a caller that wants to
/// re-assert the envelope before a second spawn would use.
pub fn verify_settings(path: &Path, expected: &EnvelopeSettings) -> anyhow::Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| {
        format!(
            "the envelope settings at {} could not be read back, so it cannot be shown to \
             be the file that was written; refusing the run",
            path.display()
        )
    })?;

    let actual: EnvelopeSettings = serde_json::from_str(&text).map_err(|error| {
        anyhow!(
            "the envelope settings at {} did not deserialise into the value that was \
             written ({error}). The agent CLI would ignore this file SILENTLY, leaving a \
             run with no PreToolUse layer and nothing to indicate it; refusing the run \
             instead (reason: {})",
            path.display(),
            policy::REASON_ENVELOPE_ASSERTION_FAILED
        )
    })?;

    if &actual != expected {
        return Err(anyhow!(
            "the envelope settings at {} do not match the value that was written. The \
             agent CLI would ignore a malformed file SILENTLY; refusing the run instead \
             (reason: {})",
            path.display(),
            policy::REASON_ENVELOPE_ASSERTION_FAILED
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: &str = "/opt/gsd-meta-manager";

    /// git's stdin, as the line vector both `pre-push` checks read.
    fn ref_lines(stdin: &str) -> Vec<String> {
        read_ref_lines(stdin.as_bytes()).expect("a &str always reads")
    }

    #[test]
    fn the_generated_stub_is_three_lines_and_execs_the_absolute_binary_path() {
        let body = stub_body(Path::new(BIN), "demo", PRE_PUSH_HOOK);
        assert_eq!(body.lines().count(), 3, "the stub must stay a stub:\n{body}");
        assert!(body.starts_with("#!/bin/sh\n"));
        assert!(body.ends_with(
            "exec '/opt/gsd-meta-manager' envelope pre-push 'demo' --hook-path \"$0\"\n"
        ));
    }

    #[test]
    fn the_pre_commit_stub_is_the_same_shape_with_its_own_subcommand() {
        let body = stub_body(Path::new(BIN), "demo", PRE_COMMIT_HOOK);
        assert_eq!(body.lines().count(), 3, "{body}");
        assert!(body.ends_with(
            "exec '/opt/gsd-meta-manager' envelope pre-commit 'demo' --hook-path \"$0\"\n"
        ));
    }

    /// **The premise [`stub_body`]'s doc rests on, MEASURED rather than
    /// asserted** (WR-02, round 8).
    ///
    /// That doc used to justify the POSIX quoting by saying a plain path
    /// component is "a weaker constraint than shell-safe —
    /// `is_plain_path_component` accepts a quote character". Since D-19-2's
    /// alphabet clause at `src/journal/mod.rs:339` it does not: the predicate
    /// accepts only `[A-Za-z0-9._-]`, so every shell metacharacter is refused.
    ///
    /// A false security rationale is the kind of falsehood a maintainer ACTS on
    /// — checks it, finds it false, concludes the quoting is redundant, and
    /// deletes a real defence. The doc now states the post-D-19-2 truth and
    /// gives a defence-in-depth reason instead, and this test is the control
    /// that truth needs: widen the alphabet to admit any of these and this goes
    /// red in the same commit, rather than the doc quietly becoming false again.
    #[test]
    fn the_path_component_predicate_refuses_every_shell_metacharacter() {
        for (name, c) in [
            ("single quote", '\''),
            ("double quote", '"'),
            ("backtick", '`'),
            ("dollar", '$'),
            ("semicolon", ';'),
            ("space", ' '),
            ("backslash", '\\'),
            ("pipe", '|'),
            ("ampersand", '&'),
            ("open paren", '('),
            ("close paren", ')'),
            ("asterisk", '*'),
            ("question mark", '?'),
            ("less than", '<'),
            ("greater than", '>'),
            ("newline", '\n'),
        ] {
            let value = format!("de{c}mo");
            let accepted = crate::journal::is_plain_path_component(&value);
            println!("{name:<14} {c:?}  is_plain_path_component -> {accepted}");
            assert!(
                !accepted,
                "`is_plain_path_component` accepted {value:?} ({name}). The \
                 alphabet clause is the whole reason `stub_body`'s doc no longer \
                 claims this predicate is weaker than shell-safe; if it is \
                 widened, that doc's premise must be corrected in the same commit \
                 and every already-generated hook stub on disk is affected."
            );
        }
    }

    #[test]
    fn a_quote_in_an_alias_cannot_escape_the_generated_stub() {
        let body = stub_body(Path::new(BIN), "de'mo", PRE_PUSH_HOOK);
        assert!(
            body.contains(r"'de'\''mo'"),
            "a quote must be POSIX-escaped, not interpolated raw:\n{body}"
        );
    }

    #[test]
    fn a_push_inside_the_namespace_exits_zero() {
        let stdin = "refs/heads/x abc refs/heads/gsd-auto/demo/x def\n";
        assert_eq!(classify_refs("demo", &ref_lines(stdin)), 0);
    }

    #[test]
    fn a_push_outside_the_namespace_exits_non_zero() {
        let stdin = "refs/heads/x abc refs/heads/main def\n";
        assert_ne!(classify_refs("demo", &ref_lines(stdin)), 0);
    }

    #[test]
    fn one_refused_ref_in_a_batch_refuses_the_whole_push() {
        let stdin = "refs/heads/a 1 refs/heads/gsd-auto/demo/a 2\n\
                     refs/heads/b 3 refs/heads/main 4\n";
        assert_ne!(classify_refs("demo", &ref_lines(stdin)), 0);
    }

    #[test]
    fn a_deletion_is_classified_by_its_destination_ref() {
        let deleting_main =
            "(delete) 0000000000000000000000000000000000000000 refs/heads/main 5\n";
        assert_ne!(classify_refs("demo", &ref_lines(deleting_main)), 0);
    }

    #[test]
    fn an_unreadable_ref_line_is_refused_rather_than_skipped() {
        assert_ne!(classify_refs("demo", &ref_lines("garbage\n")), 0);
        assert_ne!(
            classify_refs("demo", &ref_lines("one two three\n")),
            0,
            "a three-field line has no destination ref and must not be allowed"
        );
    }

    #[test]
    fn a_blank_line_is_nothing_rather_than_something_unreadable() {
        assert_eq!(classify_refs("demo", &ref_lines("\n\n")), 0);
    }

    #[test]
    fn the_pushed_ranges_are_the_sha_pair_of_every_well_formed_line() {
        let lines = ref_lines(
            "refs/heads/a 1111 refs/heads/gsd-auto/demo/a 2222\n\
             garbage\n\
             refs/heads/b 3333 refs/heads/gsd-auto/demo/b 0000000000000000000000000000000000000000\n",
        );
        assert_eq!(
            pushed_ranges(&lines),
            vec![
                ("1111".to_string(), "2222".to_string()),
                (
                    "3333".to_string(),
                    "0000000000000000000000000000000000000000".to_string()
                ),
            ],
            "an unparseable line contributes no commits, and is refused elsewhere"
        );
    }

    #[test]
    fn a_zero_sha_is_recognised_in_both_the_short_and_the_full_spelling() {
        assert!(is_zero_sha("0000000000000000000000000000000000000000"));
        assert!(is_zero_sha("0000"));
        assert!(!is_zero_sha(""), "an empty field is not a zero sha");
        assert!(!is_zero_sha("0000000000000000000000000000000000000001"));
    }

    #[test]
    fn a_nested_repository_is_found_at_any_depth_but_the_root_is_not_one() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("vendor/thing/.git")).unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();

        assert!(has_nested_git(root, Path::new("vendor/thing/src/lib.rs")));
        assert!(has_nested_git(root, Path::new("vendor/thing/README.md")));
        assert!(
            !has_nested_git(root, Path::new("src/main.rs")),
            "the repository's OWN .git must not make every path nested"
        );
        assert!(!has_nested_git(root, Path::new("README.md")));
    }

    #[test]
    fn the_exclude_block_is_idempotent_and_leaves_unrelated_lines_alone() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join(".git/info")).unwrap();
        let exclude = root.join(".git/info/exclude");
        std::fs::write(&exclude, "# a user's own rules\nscratch.txt\n").unwrap();

        write_exclude_block(root).unwrap();
        let once = std::fs::read(&exclude).unwrap();
        write_exclude_block(root).unwrap();
        let twice = std::fs::read(&exclude).unwrap();

        assert_eq!(
            once, twice,
            "a second run changed the file, so a thousand runs accumulate a \
             thousand blocks (D-23)"
        );
        let text = String::from_utf8(twice).unwrap();
        assert!(text.contains("scratch.txt"), "the user's own rules survived:\n{text}");
        assert_eq!(text.matches(EXCLUDE_BLOCK_START).count(), 1, "{text}");
        assert_eq!(text.matches(EXCLUDE_BLOCK_END).count(), 1, "{text}");
        assert!(text.contains(".claude/worktrees/"), "{text}");
        assert!(text.contains(crate::journal::RUNS_SUBDIR), "{text}");
    }

    #[test]
    fn the_exclude_block_writes_into_a_linked_worktrees_real_git_directory() {
        // A `.git` FILE rather than a directory. A writer that assumed a
        // directory would create `.git/info/` beside the pointer file, where git
        // never looks, and the ignore rule would be silently inert.
        let tmp = tempfile::TempDir::new().unwrap();
        let real_git = tmp.path().join("real.git");
        std::fs::create_dir_all(&real_git).unwrap();
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join(".git"), format!("gitdir: {}\n", real_git.display())).unwrap();

        write_exclude_block(&work).unwrap();

        let written = std::fs::read_to_string(real_git.join("info/exclude")).unwrap();
        assert!(written.contains(EXCLUDE_BLOCK_START), "{written}");
        assert!(
            !work.join(".git/info").exists(),
            "nothing may be created beside the pointer file"
        );
    }

    #[test]
    fn a_hostile_alias_is_refused_before_any_directory_is_created() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = install_in(tmp.path(), "../escaped", Path::new(BIN)).unwrap_err();
        assert!(
            err.to_string().contains("plain path component"),
            "the refusal must name why: {err}"
        );
        assert_eq!(
            std::fs::read_dir(tmp.path()).unwrap().count(),
            0,
            "a refused alias must leave the envelope root untouched"
        );
    }

    #[test]
    fn the_installed_hook_certifies_itself() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();
        assert_provenance_in(tmp.path(), "demo", &hooks_dir.join(PRE_PUSH_HOOK))
            .expect("the hook inside its own envelope directory is the sanctioned one");
    }

    #[test]
    fn a_copy_of_the_hook_outside_the_envelope_certifies_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();

        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let copy = elsewhere.join(PRE_PUSH_HOOK);
        std::fs::copy(hooks_dir.join(PRE_PUSH_HOOK), &copy).unwrap();

        let err = assert_provenance_in(tmp.path(), "demo", &copy).unwrap_err();
        assert!(
            err.to_string().contains("provenance"),
            "the refusal must name provenance, not look like a policy verdict: {err}"
        );
    }

    #[test]
    fn a_hook_for_an_alias_with_no_installed_envelope_certifies_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();
        // Same file, asked to certify itself as some OTHER alias's hook: the
        // shape an agent reaches for when it wants one envelope's authority to
        // cover a project the user never opted in.
        let err =
            assert_provenance_in(tmp.path(), "other", &hooks_dir.join(PRE_PUSH_HOOK)).unwrap_err();
        assert!(
            err.to_string().contains("sanctioned hook"),
            "the refusal must say no sanctioned hook exists for that alias: {err}"
        );
    }

    #[test]
    fn a_hostile_alias_sanctions_no_hook_at_all() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = assert_provenance_in(tmp.path(), "../escaped", Path::new("/bin/sh")).unwrap_err();
        assert!(err.to_string().contains("plain path component"), "{err}");
    }

    // ---- the `PreToolUse` guard (D-06 layer 2) ----

    /// The guard's answer: exit code, the JSON it wrote, and its stderr mirror.
    struct Answer {
        code: i32,
        stdout: String,
        stderr: String,
    }

    impl Answer {
        fn denied(&self) -> bool {
            self.code == 2
        }

        fn reason(&self) -> String {
            let value: serde_json::Value =
                serde_json::from_str(self.stdout.trim()).unwrap_or(serde_json::Value::Null);
            value["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        }
    }

    /// Run the guard over a Bash request carrying `command`.
    fn ask(root: &Path, command: &str) -> Answer {
        ask_request(
            root,
            &serde_json::json!({
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": { "command": command },
            })
            .to_string(),
        )
    }

    /// Run the guard over a verbatim request body.
    fn ask_request(root: &Path, body: &str) -> Answer {
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        let code = guard_in(
            root,
            // A path with no registry on it: `resolve_policy` must apply the
            // tighter defaults rather than failing the call.
            &root.join("no-such-config.json"),
            "alpha",
            None,
            body.as_bytes(),
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

    #[test]
    fn the_two_spellings_of_a_force_push_reach_the_same_verdict_as_each_other() {
        let tmp = tempfile::TempDir::new().unwrap();
        let spaced = ask(tmp.path(), "git  push   --force origin main");
        let short = ask(tmp.path(), "git push -f origin main");

        assert!(spaced.denied(), "{}", spaced.stdout);
        assert!(short.denied(), "{}", short.stdout);
        assert!(
            spaced.reason().contains(policy::REASON_FORCE_PUSH_BLOCKED)
                && short.reason().contains(policy::REASON_FORCE_PUSH_BLOCKED),
            "argv is parsed, not prefix-matched: {:?} vs {:?}",
            spaced.reason(),
            short.reason()
        );
    }

    #[test]
    fn a_push_inside_the_reserved_namespace_is_permitted_and_writes_nothing() {
        // The paired allow test. An envelope that refused everything would pass
        // every refusal assertion in this file.
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask(
            tmp.path(),
            "git push origin refs/heads/gsd-auto/alpha/work:refs/heads/gsd-auto/alpha/work",
        );
        assert_eq!(answer.code, 0, "{}", answer.stderr);
        assert!(
            answer.stdout.is_empty(),
            "a permit answers nothing at all: emitting `allow` would turn a deny-only \
             control into an approval authority. Got: {}",
            answer.stdout
        );
    }

    #[test]
    fn a_force_push_hidden_behind_a_separator_is_still_seen() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask(tmp.path(), "echo hi && git push --force origin main");
        assert!(
            answer.denied(),
            "a guard that classified only the first command would look at `echo`: {}",
            answer.stdout
        );
    }

    #[test]
    fn a_force_push_inside_a_nested_shell_is_still_seen() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask(tmp.path(), r#"bash -c "git push --force origin main""#);
        assert!(answer.denied(), "{}", answer.stdout);
    }

    #[test]
    fn a_command_whose_words_cannot_be_recovered_is_denied() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask(tmp.path(), "git commit -m 'unterminated");
        assert!(answer.denied(), "{}", answer.stdout);
        assert!(
            answer.reason().contains("cannot be recovered"),
            "{}",
            answer.reason()
        );
    }

    #[test]
    fn a_verb_assembled_by_expansion_and_an_eval_are_both_denied() {
        let tmp = tempfile::TempDir::new().unwrap();
        let expanded = ask(tmp.path(), "$TOOL push --force");
        assert!(expanded.denied(), "{}", expanded.stdout);
        assert!(expanded.reason().contains("shell expansion"), "{}", expanded.reason());

        let evaluated = ask(tmp.path(), "eval \"git push --force\"");
        assert!(evaluated.denied(), "{}", evaluated.stdout);
        assert!(evaluated.reason().contains("eval"), "{}", evaluated.reason());
    }

    #[test]
    fn a_request_whose_json_cannot_be_parsed_is_denied() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask_request(tmp.path(), "{not json at all");
        assert!(
            answer.denied(),
            "a guard that permitted what it could not parse would be walked straight \
             through: {}",
            answer.stdout
        );
        assert!(answer.reason().contains("not JSON"), "{}", answer.reason());
    }

    #[test]
    fn an_unknown_field_is_preserved_rather_than_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let body = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": { "command": "git status" },
            "a_field_from_a_later_cli_release": { "nested": [1, 2, 3] },
        })
        .to_string();

        let answer = ask_request(tmp.path(), &body);
        assert_eq!(
            answer.code, 0,
            "a guard that rejected an unrecognised field would deny EVERY tool call the \
             day the CLI grew one: {}",
            answer.stderr
        );

        let parsed: GuardRequest = serde_json::from_str(&body).unwrap();
        assert!(
            parsed.extra.contains_key("a_field_from_a_later_cli_release"),
            "preserved, not merely tolerated: {:?}",
            parsed.extra
        );
    }

    #[test]
    fn a_request_for_another_tool_is_not_this_layers_business() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask_request(
            tmp.path(),
            &serde_json::json!({
                "tool_name": "Read",
                "tool_input": { "file_path": "/etc/passwd" },
            })
            .to_string(),
        );
        assert_eq!(answer.code, 0, "layer 1's denylist owns non-shell tools");
    }

    #[test]
    fn a_bash_request_with_no_readable_command_is_denied() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask_request(
            tmp.path(),
            &serde_json::json!({ "tool_name": "Bash", "tool_input": {} }).to_string(),
        );
        assert!(answer.denied(), "{}", answer.stdout);
    }

    #[test]
    fn the_refusal_is_mirrored_to_stderr_as_well_as_to_the_protocol() {
        let tmp = tempfile::TempDir::new().unwrap();
        let answer = ask(tmp.path(), "git push --force origin main");
        assert!(
            answer.stderr.contains(policy::REASON_FORCE_PUSH_BLOCKED),
            "the refusal must be observable from the process boundary as well as from \
             the protocol, because a protocol that drifts must not disarm the guard: {}",
            answer.stderr
        );
        assert_eq!(answer.code, 2, "exit 2 is the second carrier");
    }

    #[test]
    fn the_pull_request_cap_is_enforced_from_the_guard_and_the_third_still_succeeds() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Defaults are 3 per 24h and 1 per run; the run id is absent here, so
        // every invocation shares the `unattributed-run` placeholder — which is
        // exactly what proves the per-run cap is not silently reset per process.
        let first = ask(tmp.path(), "gh pr create --title x --body y");
        assert_eq!(first.code, 0, "{}", first.stderr);

        let second = ask(tmp.path(), "gh pr create --title x --body y");
        assert!(
            second.denied(),
            "the per-run cap is 1, so a second attempt in the same run is refused even \
             though the window has capacity: {}",
            second.stdout
        );
        assert!(
            second.reason().contains(policy::REASON_PR_CAP_EXCEEDED)
                && second.reason().contains("over-count"),
            "{}",
            second.reason()
        );
    }

    #[test]
    fn a_read_only_forge_command_is_not_recorded_against_the_cap() {
        let tmp = tempfile::TempDir::new().unwrap();
        for _ in 0..5 {
            let answer = ask(tmp.path(), "gh pr list --limit 5");
            assert_eq!(answer.code, 0, "{}", answer.stderr);
        }
        assert!(
            !super::super::ledger::ledger_path_in(tmp.path(), "alpha")
                .unwrap()
                .exists(),
            "a listing is not a creation, and a cap that counted reads would refuse a \
             run for looking"
        );
    }

    #[test]
    fn the_guard_makes_no_network_call_on_any_path_it_takes() {
        // The mechanical half of the latency requirement. The behavioural half
        // is that every test in this block answers without a socket at all —
        // this one is what a future edit has to get past.
        let source = include_str!("hooks.rs");
        let guard_body: String = source
            .split("pub fn guard_in(")
            .nth(1)
            .expect("guard_in exists")
            .split("\n}\n")
            .next()
            .expect("its body ends")
            .to_string();

        // Without this the extraction could silently yield an empty string and
        // every assertion below would pass having read nothing — the vacuous
        // pass this phase exists to argue against.
        assert!(
            guard_body.contains("classify_segments"),
            "the extracted body is not the guard's: {guard_body}"
        );

        for client in ["reqwest", "ureq", "hyper", "curl", "TcpStream"] {
            assert!(
                !guard_body.contains(client),
                "`{client}` appeared on the guard's critical path; \
                 src/executor/mod.rs:225-239 records the 180-240 second hang this rule \
                 exists to prevent"
            );
        }
    }
}
