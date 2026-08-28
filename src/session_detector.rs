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
/// # CORRECTED 2026-08-28 (21-35): the section below is right about the SECURITY property, and being right about it is what hid the FUNCTIONAL one
///
/// **The reasoning this corrects, verbatim:** *"The property that matters is a
/// property of the SINK — that the id cannot become an option of the resumed
/// program — and it is asserted at the sink by
/// `tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`.
/// A second assertion here would certify a claim this function does not make,
/// and would let the class be counted as closed twice."*
///
/// **What it got right, and what is NOT being retracted.** The security
/// property genuinely IS a property of the sink. A hostile id cannot become an
/// option of the resumed program, and that is asserted where it is true — at
/// the argv, by fusion, in [`crate::ui::screens::detail`]. Adding a second
/// *security* assertion here would still certify a claim this function does not
/// make, and would still let the class be counted as closed twice. That half of
/// the reasoning survives this round intact.
///
/// **What it missed.** The **functional** property is a property of the
/// **pair**, not of either side: *what this build emits, this build must be
/// able to read back.* Round 11 fused the id to its option name at the producer
/// (`--resume=<id>`) and thereby changed the wire format this function parses.
/// Every control lived on one side or the other — the producer's controls
/// asserted the argv's shape, and this function had **none at all** — so **no
/// control spanned both**. The result: this function returned `None` for every
/// session the TUI itself had resumed. The row went dead in the Sessions tab
/// with no error and no log, and a build that had silently lost its resume
/// detection passed the whole gate green for a full verification pass.
///
/// The mechanism is worth naming, because "no control here" is what allowed it:
/// the two modules spell the same option name **independently**, across a
/// module boundary, and nothing coupled them. What couples them now is the
/// round trip,
/// `crate::ui::screens::detail::tests::the_argv_this_build_emits_is_an_argv_this_build_can_read_back`,
/// which drives the real producer's output through the real parser rather than
/// re-spelling either.
///
/// **The line this correction must not blur.** What has been added is
/// **wire-format parsing and its functional control**, never value validation.
/// The validator declined above stays declined, for the reasons already
/// recorded: the CLI resumes by session *title*, so a rule tight enough to
/// refuse a leading-hyphen id would silently delete legitimate sessions from
/// the Sessions tab. The new tests assert which SHAPES carry an id; they assert
/// nothing whatever about which VALUES are acceptable.
///
/// **Where the wrap now lives** (D-21-67). This function remains the one place
/// a session id **enters this build from another process**;
/// [`session_id_in_cmdline`], the pure half split out of it, is the one place
/// the id is **wrapped** — which is why that half returns `Option<Untrusted>`
/// and never `Option<String>`. That is a restatement of where the boundary
/// sits, not a loosening of it.
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
/// # CORRECTED 2026-08-28 (21-36): the byte-identity claim, and the two boundaries that make it total
///
/// **The sentence this corrects, verbatim:** *"The only condition on the VALUE
/// remains non-emptiness after `trim()`, unchanged from before this function
/// existed."* It stands unreworded in its own paragraph below; this block is
/// beside it, not instead of it.
///
/// **What it got right, and what is NOT retracted.** The emptiness condition is
/// real, it is load-bearing, and it is preserved exactly: a value that is empty
/// after `trim` carries no id, and it does not stop the scan. Nothing about
/// that changed.
///
/// **What it missed.** It described `trim()` as if the condition were also the
/// RETURN — and the return WAS the trimmed value. So the sentence was true of
/// the test and false of the result, and the gap between them was a rewrite of
/// the operator's data. `--resume=" abc "` read back as `"abc"`: a DIFFERENT
/// id, which resumes a different conversation or none at all. A whitespace-only
/// id read back as nothing whatever. That is verbatim the silent row-death that
/// round 12 existed to close, one shape over — and round 12's own 28-fixture
/// corpus could not see it, because not one of those fixtures carried leading
/// or trailing whitespace.
///
/// The condition now lives on a trimmed COPY. `text.trim()` is the TEST; `text`
/// is what is wrapped. The bytes handed to `Untrusted::from_untrusted_source`
/// are the candidate's own wire bytes, so what the type wraps is what the wire
/// carried: no value is rewritten upstream of the boundary that is supposed to
/// be the first thing to touch it.
///
/// ## The post-change contract: two named refusal classes, and no third outcome
///
/// For every byte string `s` on the wire, in either form above, this function
/// either returns `Some(t)` whose bytes are **byte-identical** to `s` — no
/// normalisation, no case folding, no trimming, no truncation, no length cap —
/// **or** returns `None` and `s` falls in exactly one of:
///
/// - **R1 — `s` is not valid UTF-8.** Refused, with the scan CONTINUING. This
///   replaces a `String::from_utf8_lossy` decode that substituted U+FFFD and
///   returned the result as an id — a FABRICATION, a non-empty id for a value
///   no process carries, which put a row in the Sessions tab offering to resume
///   a conversation that does not exist. **The residual, stated with its
///   direction:** an EXTERNALLY-launched `claude` whose argv genuinely carries
///   non-UTF-8 bytes now reads back `None` instead of a corrupted id —
///   **under-detection, and SILENT.** It is accepted because it replaces
///   MIS-detection, which is under-detection wearing a detection's clothes, and
///   because it costs no capability here: both of this build's producers take
///   `String` and can never emit a non-UTF-8 id, and a lossily-substituted id
///   could never have resumed the session it named.
/// - **R2 — `s` is valid UTF-8 and empty after `trim`.** Refused, with the scan
///   continuing. Unchanged from before.
///
/// This is still an ENCODING boundary and still not value validation. Nothing
/// added here inspects the id's first byte, its character class, its length or
/// its resemblance to a UUID. The validator declined in [`read_session_id`]'s
/// doc stays declined, for the reason recorded there: `claude` resumes by
/// session TITLE as well as by id, so a value rule silently deletes legitimate
/// sessions from the Sessions tab. UTF-8 well-formedness narrows nothing that
/// ever worked, because the pre-change code already imposed it — lossily.
///
/// ## Why the certificate is GENERATED rather than enumerated
///
/// The claim above is certified by
/// `crate::ui::screens::detail::tests::the_round_trip_property_holds_for_every_generated_byte_string`
/// over 4096 deterministically generated byte strings in every registered wire
/// form, with its reach committed as a floor by
/// `…::the_generator_reaches_every_named_class_and_both_branches_of_the_property`.
/// An ENUMERATION can only ever fail on a class somebody thought to enumerate,
/// and this phase watched that mechanism run three times: no leading-hyphen
/// fixture in round 10, no whitespace fixture in round 12, and an encoding class
/// that was not merely unenumerated but UNREPRESENTABLE while the test harness
/// was `&str`-typed.
///
/// **`proptest` was weighed and DECLINED, with the cost measured rather than
/// asserted.** In a scratch copy of this project's own manifest,
/// `cargo add --dev proptest` locks fourteen new packages and takes `Cargo.lock`
/// from 354 to 368 entries, none previously present. The `icu_properties`
/// precedent in `Cargo.toml` — where a dependency correctly replaced six rounds
/// of hand enumeration — does NOT extend: that crate supplies Unicode's own
/// REFERENCE DATA, an external ground truth this project cannot derive, whereas
/// a property engine supplies a SEARCH STRATEGY and no external source defines
/// the set of session-id classes, because that set does not exist anywhere to be
/// queried. Its default strategies would not have reached the whitespace-padded
/// class either, so the mixture grammar has to be hand-written in both worlds.
/// What is given up is shrinking, mitigated by a 12-byte length bound and a
/// failure message that prints the counterexample as an explicit byte vector.
///
/// ## The precedence rule, stated precisely (IN-07)
///
/// The leftmost-wins sentence above is TRUE and stays. It was read as implying
/// that a fused element would outrank a split form's successor, and that reading
/// is wrong. Precisely: the scan runs left to right by argv index, and the FIRST
/// element carrying a value under EITHER wire form wins. The split form's
/// successor is a **value by position** — taken literally even when it is itself
/// option-shaped, and never re-examined as an option in its own right. Measured
/// against this parser, `["claude", "--resume", "--resume=abc"]` returns
/// `Some("--resume=abc")`, not `Some("abc")`. The code was right; the doc is what
/// changed.
///
/// This is the same fact fusion establishes one parser later at the producer: a
/// value that LOOKS like an option is still that option's value. Asserted by
/// `tests::the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped`.
/// The option spellings were re-measured at `claude` **2.1.250** — `--help`
/// still documents `-r, --resume [value]`, an optional-value option — and that
/// is a DEPENDENCY BEHAVIOUR, not a property of this code. **The measurement
/// expires**: see the standing record in this phase's `deferred-items.md`, which
/// requires the probes to be re-run on any CLI upgrade rather than inferred from
/// a version number.
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
            // R1 — the ENCODING boundary. Ill-formed UTF-8 is REFUSED and the
            // scan CONTINUES; it is never substituted through. The decode here
            // used to be `String::from_utf8_lossy`, which replaced each
            // ill-formed sequence with U+FFFD and then returned the result as
            // an id: a FABRICATION, a non-empty id for a value no process
            // carries, which put a Sessions-tab row on screen offering to
            // resume a conversation that does not exist. Refusing costs no
            // capability — both of this build's producers take `String` and can
            // never emit a non-UTF-8 id, and a lossily-substituted id could
            // never have resumed the session it named.
            if let Ok(text) = std::str::from_utf8(candidate) {
                // R2 — the emptiness condition, unchanged in effect and moved
                // onto a TRIMMED COPY. `text.trim()` is the TEST; `text` is
                // what is returned. That distinction is the whole of the value
                // fix: the return used to be the trimmed value, so
                // `--resume=" abc "` read back as `"abc"` — a different id,
                // which resumes a different conversation or none.
                if !text.trim().is_empty() {
                    // The ONE place a session id is WRAPPED, which is the
                    // innermost point of the pair rather than the outermost.
                    // What is wrapped is the candidate's own wire bytes: no
                    // value is rewritten upstream of the boundary that is
                    // supposed to be the first thing to touch it.
                    return Some(Untrusted::from_untrusted_source(text.to_string()));
                }
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

        // --- The VALUE arrives UNTRIMMED (21-36, G1) ------------------------
        // The emptiness condition is a test on a trimmed COPY. The returned
        // value is the wire bytes. Until round 13 the trimmed value was what
        // was returned, so the two arms below read back a DIFFERENT id than
        // the one on the wire.
        assert_eq!(
            parsed(&["claude", "--resume= abc "]).as_deref(),
            Some(" abc "),
            "a fused value with leading and trailing whitespace must arrive \
             BYTE-IDENTICALLY. Returning `\"abc\"` here is not a cosmetic \
             difference: it is a different id, and it resumes a different \
             conversation or none at all. The trim is the emptiness TEST, never \
             the returned value."
        );
        assert_eq!(
            parsed(&["claude", "--resume", " abc "]).as_deref(),
            Some(" abc "),
            "the SPLIT form carries the value untrimmed for the same reason. \
             Both wire forms must agree: which form an id arrived in must not \
             change what the id IS."
        );

        // --- Both sides of the emptiness threshold, one step either side ----
        assert_eq!(
            parsed(&["claude", "--resume=\t"]).as_deref(),
            None,
            "ONE whitespace byte is still empty after trim, so it carries no \
             id — refusal class R2, unchanged from before this round."
        );
        assert_eq!(
            parsed(&["claude", "--resume=x"]).as_deref(),
            Some("x"),
            "ONE non-whitespace byte is the smallest value that is NOT empty \
             after trim, and it must arrive whole. This is the far side of the \
             same threshold the arm above tests the near side of."
        );

        // --- A candidate at the FIRST argv index ---------------------------
        assert_eq!(
            parsed(&["--resume=abc"]).as_deref(),
            Some("abc"),
            "the scan starts at index 0, not at index 1: nothing about the \
             parse depends on a program name preceding the option. A loop that \
             began at 1 would miss a cmdline whose very first element carries \
             the id."
        );

        // --- Ill-formed UTF-8 is REFUSED, not fabricated (21-36, G3) -------
        // This arm needs a direct `&[u8]` call: the `&str` helpers above
        // CANNOT construct it, which is the whole of why the encoding class
        // went three rounds unnoticed. It was never a missing fixture; it was
        // unrepresentable in the harness's type.
        assert_eq!(
            session_id_in_cmdline(b"claude\0--resume=\xff\xfe\0")
                .map(|id| id.as_raw_for_logic_only().to_string()),
            None,
            "a fused suffix of the two bytes 0xFF 0xFE is not valid UTF-8 and \
             must carry NO id — refusal class R1, with the scan continuing. \
             Before this round the decode was lossy and this returned \
             `Some(\"\\u{{fffd}}\\u{{fffd}}\")`: a FABRICATION, a non-empty id \
             for \
             a value no process carries, which puts a row in the Sessions tab \
             offering to resume a conversation that does not exist. Refusal \
             costs no capability here — both of this build's producers take \
             `String` and can never emit a non-UTF-8 id."
        );
        assert_eq!(
            session_id_in_cmdline(b"claude\0--resume=\xff\xfe\0--resume=later\0")
                .map(|id| id.as_raw_for_logic_only().to_string())
                .as_deref(),
            Some("later"),
            "an ill-formed candidate must not STOP the scan either — refusing \
             it and returning early would delete every later well-formed \
             element, which is a second under-detection bought with the fix \
             for the first."
        );
    }

    /// **IN-07: the split form's successor is a VALUE BY POSITION** (21-36).
    ///
    /// The doc above [`session_id_in_cmdline`] said only that *"the leftmost
    /// id-bearing element wins, deterministically, by argv index"*, which reads
    /// as if a fused element could outrank a split form's successor. Measured
    /// against the shipped parser, `["claude", "--resume", "--resume=abc"]`
    /// returns `Some("--resume=abc")`. The CODE is right and the DOC is what
    /// changed — so the reading is ASSERTED here rather than reasoned about,
    /// which is the difference between a pinned fact and an implication a later
    /// reader may draw either way.
    ///
    /// This is the same fact fusion establishes one parser later at the
    /// producer: a value that LOOKS like an option is still that option's
    /// value. The option spellings were re-measured at `claude` 2.1.250, and
    /// that is a DEPENDENCY BEHAVIOUR whose measurement EXPIRES — see the
    /// standing record in this phase's `deferred-items.md`.
    #[test]
    fn the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped() {
        assert_eq!(
            parsed(&["claude", "--resume", "--resume=abc"]).as_deref(),
            Some("--resume=abc"),
            "the element after a bare `--resume` is that option's VALUE, taken \
             literally, even when it is itself fused-option-shaped. A parser \
             that re-examined the successor as an option in its own right would \
             report `abc` — a DIFFERENT id than the one `claude` bound. The id \
             this build reports must be the id the receiving program received."
        );
        assert_eq!(
            parsed(&["claude", "--resume", "--resume"]).as_deref(),
            Some("--resume"),
            "the successor may even be the option's OWN name, so the id can \
             impersonate the option it follows. It is still a value by \
             position; re-examining it would make this build report an id the \
             receiving program never resumed."
        );
        assert_eq!(
            parsed(&["claude", "--resume", "-h"]).as_deref(),
            Some("-h"),
            "a short-option-shaped successor is a value too. This is also why \
             the PRODUCER fuses: once bound by the argv element itself, an id \
             whose first byte is `-` is data rather than syntax — measured at \
             `claude` 2.1.250, and that measurement expires with the dependency."
        );
    }

    // ======================================================================
    // THE `claude` ARGV CENSUS — the PRODUCER SET, measured instead of
    // remembered (21-37, G2, D-21-74, T-21-37-01)
    //
    // G1 was the VALUE axis and G3 the ENCODING axis. This is the PRODUCER
    // axis, and it is the same failure on a third one. Round 12 wrote the
    // round-trip claim as a property of THIS BUILD — *what this build emits,
    // this build can read back* — and asserted it over ONE of this build's
    // TWO `claude` argv producers, with nothing anywhere saying which. The
    // second producer is `crate::executor::claude::build_argv`, which emits
    // `--session-id <uuid>`; measured before this round, `claude -p
    // --session-id <uuid>` read back as `None`, so EVERY driver-launched
    // session was invisible to the detector that finds it again.
    //
    // A prose sentence naming the two producers would be the same artefact
    // that failed: a set somebody remembers. So the set is DERIVED FROM THE
    // TREE and adjudicated against a table, and a producer added by a later
    // phase becomes an unadjudicated site and a RED rather than a producer
    // somebody forgot to remember.
    //
    // # What this census does NOT see, with its direction
    //
    // The needle is a QUOTED OPTION LITERAL, not "an argv destined for
    // `claude`". A producer that assembled its option name from fragments,
    // read it out of a config value, or spelled it inside a macro is
    // INVISIBLE here and always will be. **Under-detection, and SILENT.**
    //
    // The residual is accepted rather than closed, for the reason
    // `crate::text`'s interpreter census gives for its own five interpolation
    // markers: every widening of the needle adds false positives to a control
    // whose entire value is that its zero — and here, its per-file counts —
    // can be trusted, and a census that reports correct code is a census the
    // next author disables. What bounds the residual is not this walk but the
    // round-trip requirement below: a `Producer` row that no test drives is a
    // RED, so a producer that IS adjudicated cannot stay uncoupled.
    // ======================================================================

    /// Every `.rs` file under `dir`, recursively, as `(relative path, lines)`.
    ///
    /// The recursive `read_dir` shape follows `crate::text`'s `collect_rs` and
    /// `ui::screens::render_escape_guard`'s `collect`: an unreadable entry is
    /// SKIPPED rather than panicked on, and paths are relative to
    /// `CARGO_MANIFEST_DIR` so the reported sites are the paths a reader can
    /// open.
    fn collect_rs(
        dir: &std::path::Path,
        base: &std::path::Path,
        out: &mut Vec<(String, Vec<(usize, String)>)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                collect_rs(&path, base, out);
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

    /// The option tokens' HEADS, each **missing its last character** so that no
    /// line of this module spells a token whole.
    ///
    /// The same anti-self-match idiom as `crate::text`'s `INTERPRETER_STEMS`
    /// and `ui::screens::render_escape_guard`'s `IMPL_HEAD`/`IMPL_TAIL`, and
    /// for the same reason: this census walks `src/`, and `src/session_detector.rs`
    /// is under `src/`. Spelled whole, these lines would be hits and the census
    /// would be reporting itself.
    ///
    /// **Split on the LAST character, deliberately.** A middle split would
    /// leave a fragment sitting in the array that is itself one of the tokens
    /// being looked for. So would the obvious-looking five-entry spelling
    /// `["--resum", "--resume", "-", "--session-i", "--session-id"]`, which
    /// pairs each token with a tail: two of those heads (`--resume` and
    /// `--session-id`) ARE tokens, so the array would match itself. The two
    /// fused spellings are therefore derived by appending
    /// [`OPTION_NEEDLE_FUSION`] to the assembled bare token instead of being
    /// stored.
    const OPTION_NEEDLE_HEADS: [&str; 3] = ["--resum", "-", "--session-i"];

    /// The last character each entry of [`OPTION_NEEDLE_HEADS`] is missing,
    /// paired with it positionally. Meaningless apart, which is the point.
    const OPTION_NEEDLE_TAILS: [&str; 3] = ["e", "r", "d"];

    /// The fusion character, appended to each assembled LONG token to give its
    /// fused spelling. The short option has no fused spelling of its own: its
    /// attached form is matched by PREFIX rather than by name, so `-r=abc` is
    /// the attached form and not a sixth token.
    const OPTION_NEEDLE_FUSION: &str = "=";

    /// The five tokens this parser recognises, each in both quoted forms:
    /// `"tok"` and `b"tok"`.
    ///
    /// The byte-string form is listed even though it is strictly subsumed —
    /// `b"--resume"` CONTAINS `"--resume"`, so the `str` needle already
    /// matches it. It is here so that a reader asking "does this census see a
    /// byte-string literal?" gets the answer from the list rather than from an
    /// argument about substrings.
    ///
    /// The needle carries BOTH quotes, so it matches a literal that IS the
    /// token and not one that merely begins with it: `"--resume=abc"` is a
    /// fixture VALUE, not an option spelling, and counting it would make the
    /// per-file numbers move every time somebody added a test arm.
    fn assembled_option_needles() -> Vec<String> {
        let mut needles = Vec::new();
        for (head, tail) in OPTION_NEEDLE_HEADS.iter().zip(OPTION_NEEDLE_TAILS.iter()) {
            let bare = format!("{head}{tail}");
            let mut tokens = vec![bare.clone()];
            if bare.starts_with("--") {
                tokens.push(format!("{bare}{OPTION_NEEDLE_FUSION}"));
            }
            for token in tokens {
                needles.push(format!("\"{token}\""));
                needles.push(format!("b\"{token}\""));
            }
        }
        needles
    }

    /// Is `line` a SITE — a NON-COMMENT line spelling one of the assembled
    /// tokens as a quoted literal?
    ///
    /// Comments are filtered because that is what makes a per-file COUNT stable
    /// enough to be an assertion rather than a tripwire: this phase's files
    /// carry more prose about these option names than code, and a census whose
    /// number moved every time a paragraph was edited would be disabled within
    /// one round.
    fn is_option_literal_site(line: &str, needles: &[String]) -> bool {
        if line.trim_start().starts_with("//") {
            return false;
        }
        needles.iter().any(|needle| line.contains(needle.as_str()))
    }

    /// What a site's file IS, with respect to the round-trip invariant.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SiteDisposition {
        /// It BUILDS a `claude` argv. The invariant is about this file, so it
        /// must be named by a round-trip test in [`CLAUDE_ARGV_ROUND_TRIPS`].
        Producer,
        /// It READS a `claude` argv — this parser and its own arms.
        Consumer,
        /// A same-spelled token that is not a `claude` option at all.
        NotClaude,
        /// In scope for the needle, out of scope for the invariant, with a
        /// reason and a DIRECTION.
        ///
        /// **Deliberately unconstructed today, and kept anyway.** No site in
        /// this tree needs it: the four measured files are two producers, one
        /// consumer and one same-spelled non-`claude` flag. It stays because
        /// the vocabulary is the census's contract — the next adjudicator who
        /// meets a genuine exclusion must be able to say so, and a vocabulary
        /// missing the honest answer is a vocabulary that gets a site filed
        /// under a wrong one. Its reason is additionally required to name a
        /// direction, below.
        #[allow(dead_code)]
        Excluded,
    }

    /// Every file under `src/` carrying an option-literal site, its MEASURED
    /// non-comment count, its disposition, and the reason.
    ///
    /// The counts are measured on the tree, never inherited from a plan. The
    /// pre-wave-1 baseline was **18 sites across these same four files** —
    /// `src/executor/claude.rs` 5, `src/session_detector.rs` 8,
    /// `src/ui/screens/detail.rs` 4, `src/state_reader/git_ops.rs` 1 — and
    /// 21-36 added parser arms to two of them.
    const CLAUDE_ARGV_SITES: [(&str, usize, SiteDisposition, &str); 4] = [
        (
            "src/executor/claude.rs",
            5,
            SiteDisposition::Producer,
            "build_argv emits `--session-id <uuid>` for every driver-launched \
             run, and `--resume <id>` when resuming. This is the producer the \
             round-trip invariant was written about and never asserted over.",
        ),
        (
            "src/session_detector.rs",
            13,
            SiteDisposition::Consumer,
            "this file is the CONSUMER: the option constants themselves plus \
             the parser arms that pin which shapes carry an id. It builds no \
             argv for anything to execute.",
        ),
        (
            "src/state_reader/git_ops.rs",
            1,
            SiteDisposition::NotClaude,
            "its `-r` is `git diff-tree`'s RECURSIVE flag, on a `git` argv. It \
             names no `claude` option, and it is kept in this table rather than \
             filtered out of the needle because a false positive that has to be \
             adjudicated is proof the needle matches more than what it was \
             aimed at.",
        ),
        (
            "src/ui/screens/detail.rs",
            7,
            SiteDisposition::Producer,
            "resume_terminal_argv fuses the id to its option name and emits \
             `--resume=<id>` into a terminal emulator's argv. Also carries the \
             round-trip harness and the generator's option-shaped prefixes.",
        ),
    ];

    /// The round-trip test that drives each `Producer` file's real argv through
    /// the real parser.
    ///
    /// A `Producer` row with no entry here is a file whose argv the invariant
    /// CLAIMS to cover and no test actually drives — which is precisely the
    /// state round 12 shipped in, and precisely what this census exists to make
    /// impossible to ship again.
    const CLAUDE_ARGV_ROUND_TRIPS: [(&str, &str); 1] = [(
        "src/ui/screens/detail.rs",
        "a_session_id_survives_the_round_trip_in_both_wire_forms",
    )];

    /// **Every `claude` argv option site under `src/` is adjudicated** (21-37,
    /// D-21-74, T-21-37-01, T-21-37-04).
    ///
    /// Walks `src/`, reports every non-comment line spelling one of this
    /// parser's option tokens as a quoted literal, and requires each such file
    /// to appear in [`CLAUDE_ARGV_SITES`] with a matching count and a
    /// disposition — and each `Producer` to be driven through the parser by a
    /// named test.
    ///
    /// # Why the rubber-stamp guards are here
    ///
    /// A disposition table in which every row is an exclusion reports nothing
    /// while LOOKING like a control, and a reason field left empty converts a
    /// decision into an omission. So three properties of the TABLE itself are
    /// asserted, not reviewed: at least one row is a `Producer`; both known
    /// producers are `Producer` rather than `Excluded`; and every non-`Producer`
    /// row carries a non-empty reason (T-21-37-04).
    #[test]
    fn every_claude_argv_option_site_under_src_is_adjudicated() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(&base.join("src"), &base, &mut files);
        assert!(
            !files.is_empty(),
            "the census walked src/ and found no Rust source at all, so a clean \
             report here would be a walk that never looked"
        );
        files.sort_by(|a, b| a.0.cmp(&b.0));

        let needles = assembled_option_needles();
        assert_eq!(
            needles.len(),
            10,
            "the needle assembly produced {needles:?}. Five tokens in two quoted \
             forms is ten; a different number means the heads and tails have \
             drifted apart and this census is looking for the wrong thing."
        );

        // --- The measured set ------------------------------------------------
        let mut measured: Vec<(String, Vec<usize>)> = Vec::new();
        for (path, lines) in &files {
            let hits: Vec<usize> = lines
                .iter()
                .filter(|(_, line)| is_option_literal_site(line, &needles))
                .map(|(number, _)| *number)
                .collect();
            if !hits.is_empty() {
                measured.push((path.clone(), hits));
            }
        }

        // --- Every measured file must be adjudicated -------------------------
        let unadjudicated: Vec<&str> = measured
            .iter()
            .map(|(path, _)| path.as_str())
            .filter(|path| !CLAUDE_ARGV_SITES.iter().any(|(known, ..)| known == path))
            .collect();
        assert!(
            unadjudicated.is_empty(),
            "these files spell a `claude` argv option token and appear in no row \
             of CLAUDE_ARGV_SITES: {unadjudicated:?}. A new file here is a new \
             `claude` argv site nobody has decided about — adjudicate it as \
             Producer, Consumer, NotClaude or Excluded WITH a reason, and if it \
             is a Producer, give it a round trip. Do not delete this assertion."
        );

        // --- Every adjudicated file must still exist and still match ---------
        for (path, expected, _, _) in CLAUDE_ARGV_SITES {
            let hits = measured
                .iter()
                .find(|(measured_path, _)| measured_path == path)
                .map(|(_, hits)| hits.clone())
                .unwrap_or_default();
            assert_eq!(
                hits.len(),
                expected,
                "{path} carries {} non-comment option-literal site(s), and \
                 CLAUDE_ARGV_SITES says {expected}: a delta of {}. Sites found \
                 at lines {hits:?}. Re-MEASURE the count and update the row in \
                 the same commit that moved it — a count that is quietly \
                 widened to whatever the tree says is a tripwire, not a census.",
                hits.len(),
                hits.len() as i64 - expected as i64
            );
        }

        // --- The table is not a rubber stamp (T-21-37-04) --------------------
        assert!(
            CLAUDE_ARGV_SITES
                .iter()
                .any(|(.., disposition, _)| *disposition == SiteDisposition::Producer),
            "no row of CLAUDE_ARGV_SITES is a Producer. A census whose every row \
             is an exclusion reports nothing while looking like a control."
        );
        for known_producer in ["src/ui/screens/detail.rs", "src/executor/claude.rs"] {
            let row = CLAUDE_ARGV_SITES
                .iter()
                .find(|(path, ..)| *path == known_producer);
            assert_eq!(
                row.map(|(.., disposition, _)| *disposition),
                Some(SiteDisposition::Producer),
                "{known_producer} builds a `claude` argv and its row says \
                 {:?}. Both of this build's producers must be adjudicated \
                 Producer: excluding one is exactly how the invariant came to \
                 be wider than its evidence.",
                row.map(|(.., disposition, _)| *disposition)
            );
        }
        for (path, _, disposition, reason) in CLAUDE_ARGV_SITES {
            if disposition != SiteDisposition::Producer {
                assert!(
                    !reason.trim().is_empty(),
                    "{path} is adjudicated {disposition:?} with an EMPTY reason. \
                     An unreasoned exclusion is an omission wearing a decision's \
                     clothes."
                );
            }
            if disposition == SiteDisposition::Excluded {
                assert!(
                    reason.contains("detection"),
                    "{path} is Excluded and its reason names no DIRECTION. An \
                     exclusion is a decision to not see something, and the only \
                     honest form of it says which way the blindness runs — \
                     under-detection (silent) or mis-detection. This phase's \
                     whole record is written in those two words; a reason \
                     carrying neither is a reason that has not been thought \
                     through."
                );
            }
        }

        // --- Every Producer is driven through this parser by a named test ----
        for (path, _, disposition, _) in CLAUDE_ARGV_SITES {
            if disposition != SiteDisposition::Producer {
                continue;
            }
            assert!(
                CLAUDE_ARGV_ROUND_TRIPS
                    .iter()
                    .any(|(producer, _)| *producer == path),
                "{path} is adjudicated a `claude` argv Producer and NO round-trip \
                 test names it in CLAUDE_ARGV_ROUND_TRIPS. The invariant `what \
                 this build emits, this build can read back` is stated over THIS \
                 BUILD's producers, so a producer with no round trip makes the \
                 claim wider than its evidence — which is the defect this census \
                 exists to report. Measured: `claude -p --session-id <uuid>` \
                 reads back None, so every driver-launched session is invisible \
                 to the detector that finds it again."
            );
        }

        // --- A named round trip must actually exist --------------------------
        // Named last, deliberately: a doc or a table pointing at a test that
        // does not exist is the WR-05 hazard, and it must be caught by a walk
        // rather than by a reader.
        for (producer, test_name) in CLAUDE_ARGV_ROUND_TRIPS {
            let definition = format!("fn {test_name}(");
            assert!(
                files
                    .iter()
                    .any(|(_, lines)| lines.iter().any(|(_, line)| line.contains(&definition))),
                "CLAUDE_ARGV_ROUND_TRIPS names `{test_name}` as {producer}'s \
                 round trip and no file under src/ defines it. A table naming a \
                 test that does not exist certifies nothing at all."
            );
        }
    }
}
