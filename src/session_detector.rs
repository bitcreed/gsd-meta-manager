use std::path::PathBuf;
use std::process::Command;

use crate::text::Untrusted;

/// One live `claude` process, as seen through `/proc`.
///
/// **`session_id` is [`Untrusted`]** (D-21-19). It is scraped verbatim out of
/// another process's `--resume` argument in [`read_session_id`], so nothing
/// about it was authored by this build and nothing constrains it to ASCII —
/// which is exactly why the Sessions tab's `sid[..8]` byte slice could panic
/// the whole TUI (T-21-25-05).
///
/// `tty` is deliberately NOT retyped: it is compared against tmux's
/// `#{pane_tty}` and is a `/dev/` path component the kernel produced, not
/// something read out of a repository.
#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub pid: u32,
    pub session_id: Option<Untrusted>,
    pub working_dir: PathBuf,
    pub start_time: Option<u64>,
    /// Controlling TTY in tmux-friendly form (e.g. "pts/3"). The leading
    /// "/dev/" is stripped so a `contains()` match against tmux's
    /// `#{pane_tty}` (which prints "/dev/pts/3") still hits.
    pub tty: Option<String>,
}

/// Detect active Claude Code sessions by inspecting the Linux /proc filesystem.
///
/// Uses `pgrep -x claude` to find PIDs, then reads /proc entries for each.
/// Silently skips any PID where reads fail (stale/exited processes).
/// Uses std::process::Command (not tokio) — called from spawn_blocking.
pub fn detect_sessions() -> Vec<ClaudeSession> {
    let pids = match get_claude_pids() {
        Some(pids) => pids,
        None => return Vec::new(),
    };

    pids.into_iter().filter_map(build_session).collect()
}

fn get_claude_pids() -> Option<Vec<u32>> {
    let output = Command::new("pgrep").args(["-x", "claude"]).output().ok()?;

    if !output.status.success() {
        return Some(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<u32> = stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();

    Some(pids)
}

fn build_session(pid: u32) -> Option<ClaudeSession> {
    let proc_path = PathBuf::from(format!("/proc/{}", pid));

    // Read working directory from /proc/PID/cwd symlink
    let working_dir = std::fs::read_link(proc_path.join("cwd")).ok()?;

    // Read session_id from /proc/PID/cmdline (null-byte separated)
    let session_id = read_session_id(pid);

    // Read start_time from /proc/PID/stat field 22
    let start_time = read_start_time(pid);

    // Read TTY from /proc/PID/fd/0 — the controlling terminal symlinks
    // here for any interactive Claude session.
    let tty = read_tty(pid);

    Some(ClaudeSession {
        pid,
        session_id,
        working_dir,
        start_time,
        tty,
    })
}

fn read_tty(pid: u32) -> Option<String> {
    let link = std::fs::read_link(format!("/proc/{}/fd/0", pid)).ok()?;
    let s = link.to_string_lossy();
    // /dev/pts/3 → pts/3 ; /dev/tty1 → tty1 ; anything else passes through.
    let stripped = s.strip_prefix("/dev/").unwrap_or(&s);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

/// The ONE place a session id enters this build: the token after another
/// process's `--resume` in its `/proc/<pid>/cmdline`.
///
/// # This value is passed through UNVALIDATED, and that is a decision (D-21-48)
///
/// Said plainly, because the next reader must not mistake the absence of a
/// validator for the absence of thought: **nothing here constrains the id
/// beyond non-emptiness after `trim()`.** Not its first byte, not its
/// character set, not its length. A `-h`, a `--dangerously-skip-permissions`
/// or a `'; rm -rf / #` read out of a hostile neighbour's command line is
/// returned from this function unchanged.
///
/// **The control that makes that safe is `resume_terminal_argv` in
/// `crate::ui::screens::detail`**, which fuses the id to its option name in a
/// single argv element (`--resume=<id>`). Fusion binds the value to the option
/// regardless of its first byte, so `claude`'s own option parser reads it as
/// data rather than as an option of its own (CWE-88). If you are here because
/// you deleted or loosened that fusion, this sentence is the one that says why
/// it existed.
///
/// # A rejecting validator was CONSIDERED and DECLINED, for two reasons
///
/// **One, it costs capability in the SILENT direction.** The installed CLI's
/// own error text is `--resume requires a valid session ID or session title`
/// — `claude` resumes by session **title** as well as by UUID. A rule tight
/// enough to refuse `-h` therefore also refuses legitimate title resumes, and
/// those sessions would simply stop appearing as resumable in the Sessions tab
/// with no message to the operator. That is a feature deletion wearing a
/// security fix's clothes, and it would be invisible.
///
/// **Two, it is the structurally weaker control.** A validator can only
/// enumerate shapes to refuse, and an enumeration can always be one shape
/// short — this phase's own history is eleven verification passes of exactly
/// that. Fusion is not an enumeration: it removes the receiving parser's
/// ability to reinterpret ANY byte of the value. It is the same argument round
/// 10 made correctly about deleting the command interpreter, carried one
/// parser further than round 10 stopped.
///
/// # What this pass-through does NOT bound, and in which direction
///
/// A hostile id still reaches every OTHER consumer of
/// [`ClaudeSession::session_id`]. That is already closed at the TYPE rather
/// than here: the field is `Option<crate::text::Untrusted>`, and every
/// Sessions-tab render goes through `shown()`, so an invisible or bidi
/// character cannot reach a row unescaped.
///
/// The residual is a FUTURE consumer that takes the raw accessor and hands it
/// to a parser without an argv fusion. Its direction is **under-detection, and
/// SILENT** — nothing here would report it. It is bounded by
/// `Untrusted::as_raw_for_logic_only`'s three-question rule and by the
/// interpreter census over `src/`, and NOT by this function.
///
/// # No control is added in this file, deliberately
///
/// The property that matters is a property of the SINK — that the id cannot
/// become an option of the resumed program — and it is asserted at the sink by
/// `tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`.
/// A second assertion here would certify a claim this function does not make,
/// and would let the class be counted as closed twice.
fn read_session_id(pid: u32) -> Option<Untrusted> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    session_id_in_cmdline(&cmdline)
}

/// The resume long option's **bare name**, as it appears on the wire when the
/// option and its value travel as two separate argv elements — the SPLIT form,
/// which is what a human typing `claude --resume <id>` by hand produces.
const RESUME_OPTION_NAME: &[u8] = b"--resume";

/// The same option name with the **fusion character appended** — the prefix of
/// the SINGLE argv element this build's own producer emits since round 11
/// (`crate::ui::screens::detail::RESUME_OPTION_FUSED_PREFIX`).
///
/// The two modules spell this option name independently, across a module
/// boundary. The only thing coupling them is the round-trip control
/// `crate::ui::screens::detail::tests::the_argv_this_build_emits_is_an_argv_this_build_can_read_back`;
/// when that coupling did not exist, they drifted and the drift was silent.
const RESUME_OPTION_FUSED_PREFIX: &[u8] = b"--resume=";

/// The PURE half of the pair: the `/proc/<pid>/cmdline` bytes → session id
/// parse, with no I/O in it (D-21-67).
///
/// It is split out of [`read_session_id`] because a round trip cannot be
/// asserted against a function that reads `/proc`: a test can hand this one the
/// exact bytes the kernel would present, which is what lets the argv this build
/// EMITS be driven back through the parser this build READS with.
///
/// # BOTH wire forms are read, because both are legitimate on the wire
///
/// - **Fused** — one element beginning with [`RESUME_OPTION_FUSED_PREFIX`], the
///   value being everything after that prefix. This is what this TUI emits.
/// - **Split** — an element equal to [`RESUME_OPTION_NAME`], the value being
///   the element after it. This is what a human typing the command by hand, or
///   any launcher that is not this TUI, still produces. Dropping it would
///   re-break detection for every session not started here.
///
/// The leftmost id-bearing element wins, deterministically, by argv index.
///
/// # What is added here is WIRE-FORMAT PARSING, never value validation
///
/// The only condition on the VALUE remains non-emptiness after `trim()`,
/// unchanged from before this function existed; an empty value does not stop
/// the scan, so a later well-formed element is still found. Nothing constrains
/// the id's first byte, character set or length — see [`read_session_id`]'s doc
/// for why a validator was considered and declined.
///
/// # Where the boundary sits, restated rather than loosened
///
/// [`read_session_id`] remains the one place a session id **enters this build
/// from another process**; this function is the one place it is **wrapped**.
/// That is why it returns `Option<Untrusted>` and never `Option<String>` — the
/// wrap sits at the innermost point of the pair, so there is no arrangement of
/// callers in which an unwrapped id escapes this module.
pub(crate) fn session_id_in_cmdline(cmdline: &[u8]) -> Option<Untrusted> {
    let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();

    let mut index = 0;
    while index < args.len() {
        // The ONLY new condition is on the ARGUMENT'S SHAPE — which of the two
        // wire forms this element is, if either. Nothing here inspects what the
        // value IS.
        let candidate: Option<&[u8]> =
            if let Some(suffix) = args[index].strip_prefix(RESUME_OPTION_FUSED_PREFIX) {
                Some(suffix)
            } else if args[index] == RESUME_OPTION_NAME {
                // The split form's value is the NEXT element; step over it so
                // it is not re-examined as an option in its own right. A
                // trailing option name with nothing after it yields nothing
                // and does not panic.
                let next = args.get(index + 1).copied();
                index += 1;
                next
            } else {
                None
            };

        if let Some(candidate) = candidate {
            let val = String::from_utf8_lossy(candidate);
            let val = val.trim();
            if !val.is_empty() {
                // The ONE place a session id is WRAPPED, which is the
                // innermost point of the pair rather than the outermost.
                return Some(Untrusted::from_untrusted_source(val.to_string()));
            }
        }

        index += 1;
    }

    None
}

fn read_start_time(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // /proc/PID/stat fields are space-separated, but field 2 (comm) can contain spaces
    // and is enclosed in parentheses. Find the closing paren, then count from there.
    let after_comm = stat.find(')')?.checked_add(2)?;
    let rest = stat.get(after_comm..)?;
    // Field 22 (starttime) is at index 19 after the comm field (0-indexed from field 3)
    let field = rest.split_whitespace().nth(19)?;
    field.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sessions_returns_vec() {
        // Should not panic, returns empty vec if no claude processes
        let sessions = detect_sessions();
        // We can't assert specific content in CI, but it should not panic
        let _ = sessions;
    }

    #[test]
    fn test_read_session_id_nonexistent_pid() {
        // Should return None for non-existent PID
        assert!(read_session_id(999_999_999).is_none());
    }

    #[test]
    fn test_read_start_time_nonexistent_pid() {
        assert!(read_start_time(999_999_999).is_none());
    }

    /// NUL-join argv elements into the `/proc/<pid>/cmdline` encoding, with the
    /// trailing NUL the kernel emits.
    fn cmdline(elements: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for element in elements {
            bytes.extend_from_slice(element.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    /// What [`session_id_in_cmdline`] recovers from a cmdline built out of
    /// `elements`, as a plain `String` for comparison.
    fn parsed(elements: &[&str]) -> Option<String> {
        session_id_in_cmdline(&cmdline(elements)).map(|id| id.as_raw_for_logic_only().to_string())
    }

    /// **Which SHAPES on the wire carry a session id — and which do not.**
    ///
    /// These arms exercise the parser ALONE and need no producer, which is why
    /// they live beside the function they test rather than in `detail.rs`: the
    /// next reader of `session_id_in_cmdline` looks here.
    ///
    /// This is a **functional** control over wire-format parsing. It is NOT a
    /// second assertion of the security property — that one is a property of
    /// the SINK, it is asserted at the sink in `detail.rs`, and asserting it
    /// again here would let the class be counted as closed twice. See the
    /// correction block above [`read_session_id`], which draws that line
    /// explicitly.
    ///
    /// Every arm below is about the argument's SHAPE. None is about its VALUE:
    /// the only condition on the value is non-emptiness after `trim()`.
    #[test]
    fn session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape() {
        // --- The two forms that DO carry an id -----------------------------
        assert_eq!(
            parsed(&["claude", "--resume=abc"]).as_deref(),
            Some("abc"),
            "the FUSED form is one argv element whose value is everything after \
             the fusion character. This is the shape this build's own producer \
             emits, so failing here means the TUI cannot read its own sessions."
        );
        assert_eq!(
            parsed(&["claude", "--resume", "abc"]).as_deref(),
            Some("abc"),
            "the SPLIT form is the two-element window a hand-typed \
             `claude --resume <id>` puts on the wire. This build no longer \
             emits it, but every session not started by this TUI arrives in it."
        );
        assert_eq!(
            parsed(&["claude", "--resume=a=b"]).as_deref(),
            Some("a=b"),
            "a fused value containing the fusion character must arrive WHOLE: \
             the prefix is stripped once, not split on the LAST `=`, which \
             would truncate the id `a=b` to `b` and resume nothing."
        );

        // --- An empty value yields nothing, and the scan CONTINUES ----------
        assert_eq!(
            parsed(&["claude", "--resume="]).as_deref(),
            None,
            "a fused element with an EMPTY suffix carries no id. This is the \
             pre-existing non-empty-after-trim() condition, unchanged."
        );
        assert_eq!(
            parsed(&["claude", "--resume=", "--resume=later"]).as_deref(),
            Some("later"),
            "an empty value must not STOP the scan — a later well-formed \
             element is still found. Returning None here would be a new \
             behaviour the pre-fix parser did not have."
        );
        assert_eq!(
            parsed(&["claude", "--resume=   "]).as_deref(),
            None,
            "a whitespace-only fused suffix carries no id, by the same trim() \
             as the empty one"
        );
        assert_eq!(
            parsed(&["claude", "--resume=  ", "--resume=later"]).as_deref(),
            Some("later"),
            "a whitespace-only value must not stop the scan either"
        );

        // --- The option name with nothing after it -------------------------
        assert_eq!(
            parsed(&["claude", "--resume"]).as_deref(),
            None,
            "a trailing bare option name has no value element after it, so it \
             carries no id"
        );
        // The same shape WITHOUT the kernel's trailing NUL, so the final
        // element genuinely has no successor at all. This is the arm that
        // reaches the `args.get(index + 1) == None` branch; it must yield None
        // rather than panicking on an out-of-bounds index.
        assert_eq!(
            session_id_in_cmdline(b"claude\0--resume")
                .map(|id| id.as_raw_for_logic_only().to_string()),
            None,
            "a bare option name as the FINAL element, with nothing after it at \
             all, must yield no id and must NOT panic: an index that walks off \
             the end of the argument list would take the whole TUI down on a \
             cmdline any local process can plant."
        );

        // --- Shapes that carry no id at all --------------------------------
        assert_eq!(
            parsed(&["claude", "--print", "hello"]).as_deref(),
            None,
            "a cmdline with no resume option at all carries no id"
        );
        assert_eq!(
            parsed(&["claude", "--resumes", "abc"]).as_deref(),
            None,
            "`--resumes` merely BEGINS with the resume spelling; it is neither \
             the bare option name nor the fused prefix, so it carries no id. A \
             starts_with() on the bare name would wrongly claim `abc` here."
        );
        assert_eq!(
            parsed(&["claude", "--resume-session", "abc"]).as_deref(),
            None,
            "`--resume-session` is a different option that shares a prefix; its \
             value is not this option's value"
        );

        // --- Precedence: the LEFTMOST id-bearing element wins ---------------
        assert_eq!(
            parsed(&["claude", "--resume", "first", "--resume=second"]).as_deref(),
            Some("first"),
            "split-then-fused: the leftmost id-bearing element wins, by argv \
             index. Which form it is must not affect precedence."
        );
        assert_eq!(
            parsed(&["claude", "--resume=first", "--resume", "second"]).as_deref(),
            Some("first"),
            "fused-then-split: same rule, opposite order. Precedence is \
             deterministic by position, not by wire form."
        );
    }
}
