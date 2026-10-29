//! The one launcher spawn file (Phase 28; RESEARCH §6, LNCH-08).
//!
//! Every process the launcher starts is built here: `systemd-run` (which runs
//! `bwrap`, which runs the agent), `systemctl`, and, in later plans, `git` for
//! the push mirror, `claude setup-token` and `gh`. That is why the spawn
//! allowlist (`tests/spawn_seam_guard.rs`) carries ONE entry for the launcher: a
//! spawn site nobody had to think about is how an agent comes to be launched
//! outside its sandbox. The agent is reached only through `bwrap` (or, in 28-04,
//! the explicit degraded spawn).
//!
//! Six shapes:
//!
//! * [`spawn_launch_child`], the long-lived driver child: a process-group leader
//!   with stdin `/dev/null` (no TTY, no `tmux`), stdout and stderr into ONE pipe
//!   whose reader redacts each whole line on its way to a file (SAFE-04), and
//!   EXACTLY the environment it is given.
//! * [`run_bounded`], a short host command whose stdout is read within a budget
//!   ([`crate::bounded_output::stdout_within`]).
//! * [`respawn_detached`], this binary again as the detached launcher (D-04).
//! * [`run_interactive`], a host command that talks to the user's terminal
//!   (`claude setup-token`, 28-06).
//! * [`run_bounded_with_stderr`], [`run_bounded`] that also keeps the last line of
//!   the command's stderr (the push step's `git` and `gh`, 28-08).
//! * [`spawn_bridged_child`], the driver as the in-sandbox bridge's child
//!   (28.1-05): environment and stdio inherited, so it is bwrap's own.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::bounded_output::{stdout_within, stdout_within_with_stderr, BoundedOutput};
use crate::journal::redact::{redact, redact_value};

/// A stream-json line longer than this is skipped when looking for the result, and is
/// never copied into the capture file ([`pump_redacted`]). One constant for both
/// sides, so the pump never writes a line the reader would have to skip.
pub(super) const MAX_STREAM_LINE: u64 = 4 * 1024 * 1024;

/// What replaces a master output line longer than [`MAX_STREAM_LINE`]: a fixed note,
/// never a fragment of the line (the 31-08 / WR-05 rule, see [`last_line_of_tail`]).
const OVERLONG_STREAM_NOTE: &str =
    "(a line of the master's output was too long to keep and is not shown)";

/// How long the launcher waits, after the scope or group is stopped, for the capture
/// pump to see EOF. `[inferred — 2026-10-29]`: a leftover process that still holds the
/// pipe must not hang the launcher; on timeout it proceeds with what is on disk.
pub(super) const CAPTURE_FINISH_BUDGET: Duration = Duration::from_secs(5);

/// The launch child's capture pump (SAFE-04): lets the launcher wait until every byte
/// the child wrote has been redacted and written.
pub struct CaptureHandle {
    done: mpsc::Receiver<()>,
}

impl CaptureHandle {
    /// Wait up to `budget` for the pump to reach EOF and flush. `true` when it did.
    /// On `false` the pump is left to finish by itself and the caller reads what is on
    /// disk.
    pub fn finish(self, budget: Duration) -> bool {
        // A disconnect (the pump thread ended or panicked) is as final as a message.
        !matches!(
            self.done.recv_timeout(budget),
            Err(mpsc::RecvTimeoutError::Timeout)
        )
    }
}

/// Start the driver command `argv` in `cwd`, headless.
///
/// * The environment is cleared and then set to `env`: the full child
///   environment is decided by the pure [`super::sandbox::child_env`], so the
///   scrub is golden-testable rather than a loop around a command builder.
///   `[inferred — audit]`: this deviates from the `env_remove` approach of
///   the retired in-process executor, which removed keys from an inherited
///   environment.
/// * `stdin` is `/dev/null` and `stdout`/`stderr` are the write end of one pipe
///   (LNCH-08). A pump thread reads it a whole line at a time, redacts each line
///   with the SAFE-04 table ([`pump_redacted`]) and appends it to `out`, so a
///   credential-shaped token the master prints never reaches the file, even when it
///   arrives split across two writes. Wait for the pump with
///   [`CaptureHandle::finish`] before reading `out`.
/// * `process_group(0)` makes the child a group leader, so the launcher can
///   signal the whole group (and `die-with-parent` is not the only defence).
///
/// With `PATH` in `env`, the program lookup uses the child's `PATH`.
///
/// `[inferred — 2026-10-29]`: if the launcher dies, the pipe's reader goes with it and
/// the (degraded) child's next write fails. That is acceptable: with no launcher no
/// one enforces the caps, and the child is stopped by its group or scope.
pub fn spawn_launch_child(
    argv: &[OsString],
    env: &BTreeMap<OsString, OsString>,
    cwd: &Path,
    out: File,
) -> std::io::Result<(Child, CaptureHandle)> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "an empty command line")
    })?;
    let (reader, writer) = std::io::pipe()?;
    let err = writer.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(err)
        .process_group(0);
    let spawned = cmd.spawn();
    // The command owns the parent's copies of the write end. Drop them now, or the
    // pump never sees EOF.
    drop(cmd);
    let child = spawned?;

    let (tx, done) = mpsc::channel();
    std::thread::Builder::new()
        .name("launch-capture".to_owned())
        .spawn(move || {
            pump_redacted(BufReader::new(reader), out);
            let _ = tx.send(());
        })?;
    Ok((child, CaptureHandle { done }))
}

/// Copy `input` to `out` one whole line at a time, redacting each line (SAFE-04).
///
/// * Only COMPLETE lines (and the final unterminated one at EOF) reach the redactor,
///   which is what reassembles a token the child split across two writes (the
///   31-08 / WR-05 rule from [`last_line_of_tail`]).
/// * A line that parses as JSON is redacted by tree walk ([`redact_value`], which
///   cannot produce invalid JSON, so the outcome reader sees the same `result`,
///   `rate_limit_event` and OAuth-revoked marker). `RedactedLine::new` is not used:
///   it truncates string leaves.
/// * Any other line is redacted as text.
/// * A line over [`MAX_STREAM_LINE`] is discarded through its next `\n` and
///   [`OVERLONG_STREAM_NOTE`] is written in its place, never a fragment.
/// * After a write error the pump keeps reading and discarding until EOF, so the
///   child never blocks on a full pipe or gets `EPIPE` `[inferred — 2026-10-29]`.
fn pump_redacted<R: BufRead, W: Write>(mut input: R, mut out: W) {
    let mut line = Vec::new();
    let mut writable = true;
    loop {
        line.clear();
        let Ok(read) = input
            .by_ref()
            .take(MAX_STREAM_LINE)
            .read_until(b'\n', &mut line)
        else {
            break;
        };
        if read == 0 {
            break;
        }
        let rendered = if !line.ends_with(b"\n") && read as u64 >= MAX_STREAM_LINE {
            discard_through_newline(&mut input);
            OVERLONG_STREAM_NOTE.to_owned()
        } else {
            let body = line.strip_suffix(b"\n").unwrap_or(&line);
            redact_line(body)
        };
        if writable && write_line(&mut out, &rendered).is_err() {
            writable = false;
        }
    }
    if writable {
        let _ = out.flush();
    }
}

/// One whole line, without its terminator, redacted: as a JSON tree if it parses as
/// JSON, else as text.
fn redact_line(body: &[u8]) -> String {
    match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(mut value) => {
            redact_value(&mut value);
            serde_json::to_string(&value).unwrap_or_else(|_| redact(&String::from_utf8_lossy(body)))
        }
        Err(_) => redact(&String::from_utf8_lossy(body)),
    }
}

fn write_line<W: Write>(out: &mut W, line: &str) -> std::io::Result<()> {
    out.write_all(line.as_bytes())?;
    out.write_all(b"\n")
}

/// Read and drop bytes up to and including the next `\n` (or EOF), in bounded steps.
fn discard_through_newline<R: BufRead>(input: &mut R) {
    let mut scratch = Vec::new();
    loop {
        scratch.clear();
        match input
            .by_ref()
            .take(MAX_STREAM_LINE)
            .read_until(b'\n', &mut scratch)
        {
            Ok(0) | Err(_) => return,
            Ok(_) if scratch.ends_with(b"\n") => return,
            Ok(_) => {}
        }
    }
}

/// Start `argv` as the in-sandbox bridge's child (28.1-05, D-08).
///
/// * The environment and all three stdio streams are inherited: the bridge is
///   bwrap's command, and the driver it runs must see exactly the environment the
///   launcher built for the sandbox, with `claude`'s stream-json on the stdout
///   the launcher gave bwrap.
/// * **No `env_clear`, no `process_group`, deliberately:** the bridge holds no
///   policy and no secret, so there is nothing to scrub, and the child stays in the
///   sandbox's own group so one signal to the group reaches both.
pub fn spawn_bridged_child(argv: &[OsString]) -> std::io::Result<Child> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "an empty command line")
    })?;
    Command::new(program).args(args).spawn()
}

/// Run a short host command and return its status and stdout, within `budget`.
///
/// The host environment is inherited, plus `extra_env`. `None` when the command
/// could not be started or did not finish in time (it is then killed).
pub fn run_bounded(
    program: &str,
    args: &[OsString],
    cwd: Option<&Path>,
    extra_env: &[(&str, &str)],
    budget: Duration,
) -> Option<BoundedOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    stdout_within(&mut cmd, budget)
}

/// Start this binary again with `args` as the detached launcher (D-04).
///
/// * `stdin` is `/dev/null`, `stdout` and `stderr` are `log`, and the working
///   directory is `/`, so nothing of the caller's terminal or directory is kept
///   alive by the daemon.
/// * **No `process_group(0)`, deliberately:** a process-group leader cannot call
///   `setsid(2)` (`EPERM`), and the child's first act is `setsid` (the hidden
///   `--daemon-child` flag of `driver launch`). A fresh child that stays in the
///   caller's group is not a leader, so it can.
/// * The environment is inherited: the daemon is the same launcher the caller
///   would have run in the foreground.
pub fn respawn_detached(args: &[OsString], log: File) -> std::io::Result<Child> {
    let program = std::env::current_exe()?;
    let err = log.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(err);
    cmd.spawn()
}

/// Run `program args` with the caller's own stdin, stdout and stderr, and wait
/// for it (28-06: `claude setup-token` asks the user to open a browser and
/// prints the token for them to copy).
///
/// Nothing is captured, so the launcher never holds what the program prints; the
/// user pastes the token back themselves (D-21). The environment is inherited:
/// this is the user's own login on the host, not a sandboxed run.
pub fn run_interactive(program: &str, args: &[&str]) -> std::io::Result<ExitStatus> {
    Command::new(program).args(args).status()
}

/// How much of the end of a command's stderr is read back.
const STDERR_TAIL_BYTES: u64 = 4096;

/// What [`run_bounded_with_stderr`] returns when the last stderr line is not wholly
/// inside the read-back window (WR-05): a fixed note, never a fragment of the line.
const OVERLONG_STDERR_NOTE: &str = "(the last stderr line was too long to keep and is not shown)";

/// The `GIT_*` variables that would point a host `git` at a repository other than
/// the one the argv names. Removed from every `git` this module starts: the push
/// step names its repositories explicitly (`--git-dir`, `-C`), and an inherited
/// `GIT_DIR` would silently override both.
const GIT_REPO_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

/// The last non-empty trimmed line of `tail`, the final bytes of a command's stderr.
///
/// `window_cut` says the read-back window began inside the stream. Its first line
/// is then a fragment, so everything up to and including the first `\n` is
/// discarded before the selection (WR-05): a token cut there has lost the prefix
/// the SAFE-04 patterns anchor on, and a fragment handed to the redactor would be
/// persisted half-redacted. If the cut window holds no complete line after that,
/// the last line was the cut one, and [`OVERLONG_STDERR_NOTE`] is returned instead
/// of any of its bytes.
fn last_line_of_tail(tail: &[u8], window_cut: bool) -> String {
    let whole = if window_cut {
        match tail.iter().position(|b| *b == b'\n') {
            Some(at) => &tail[at + 1..],
            None => return OVERLONG_STDERR_NOTE.to_owned(),
        }
    } else {
        tail
    };
    let line = String::from_utf8_lossy(whole)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned();
    if window_cut && line.is_empty() {
        return OVERLONG_STDERR_NOTE.to_owned();
    }
    line
}

/// [`run_bounded`], plus the last non-empty line of the command's stderr.
///
/// stderr goes to a private temporary file (never a pipe, so it cannot fill and
/// block the child), and only its final [`STDERR_TAIL_BYTES`] are read back. The
/// partial first line of a cut window is never returned, because a token cut there
/// has lost the prefix the SAFE-04 patterns anchor on; a final line longer than the
/// window yields `OVERLONG_STDERR_NOTE` (see `last_line_of_tail`, WR-05). The
/// caller decides what to do with the line: it can carry a URL with credentials in
/// it, so it must be redacted before it is printed or logged. For `git`, the
/// repository-selecting `GIT_*` variables are removed from the inherited
/// environment first.
pub fn run_bounded_with_stderr(
    program: &str,
    args: &[OsString],
    cwd: Option<&Path>,
    extra_env: &[(&str, &str)],
    budget: Duration,
) -> Option<(BoundedOutput, String)> {
    use std::io::{Read, Seek, SeekFrom};

    let mut sink = tempfile::tempfile().ok()?;
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    if program == "git" {
        for key in GIT_REPO_ENV {
            cmd.env_remove(key);
        }
    }
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let output = stdout_within_with_stderr(&mut cmd, budget, Stdio::from(sink.try_clone().ok()?))?;

    let len = sink.seek(SeekFrom::End(0)).ok()?;
    sink.seek(SeekFrom::Start(len.saturating_sub(STDERR_TAIL_BYTES)))
        .ok()?;
    let mut tail = Vec::new();
    sink.take(STDERR_TAIL_BYTES).read_to_end(&mut tail).ok()?;
    let line = last_line_of_tail(&tail, len > STDERR_TAIL_BYTES);
    Some((output, line))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A token-shaped secret, assembled at runtime so this file holds no literal.
    const SECRET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ012345";

    #[test]
    fn a_cut_window_with_no_line_break_yields_the_note_and_no_fragment() {
        let tail = format!("{SECRET}{SECRET}");
        let line = last_line_of_tail(tail.as_bytes(), true);
        assert_eq!(line, OVERLONG_STDERR_NOTE);
        assert!(!line.contains(&SECRET[..8]), "{line}");
        // A cut window whose only line break ends the fragment has no whole line either.
        assert_eq!(
            last_line_of_tail(format!("{SECRET}\n").as_bytes(), true),
            OVERLONG_STDERR_NOTE
        );
    }

    #[test]
    fn the_partial_first_line_of_a_cut_window_is_dropped() {
        let tail = format!("{SECRET} fragment\nfatal: the real error\n\n");
        assert_eq!(
            last_line_of_tail(tail.as_bytes(), true),
            "fatal: the real error"
        );
    }

    #[test]
    fn an_uncut_tail_keeps_its_last_line() {
        assert_eq!(last_line_of_tail(b"one\n  two  \n\n", false), "two");
        // No line break at all is a whole line when the window was not cut.
        assert_eq!(last_line_of_tail(b"only", false), "only");
        assert_eq!(last_line_of_tail(b"", false), "");
    }

    #[test]
    fn run_bounded_with_stderr_never_returns_a_token_cut_by_the_window() {
        // WR-05: 100 'x', a space, a 36-byte token, then 4070 'y', no newline. The sink
        // is 4207 bytes, so the 4096-byte window starts 10 bytes into the token: its
        // `ghp_` prefix is gone and the SAFE-04 patterns could no longer anchor on it.
        let payload = format!("{} ghp_{SECRET}{}", "x".repeat(100), "y".repeat(4070));
        assert_eq!(payload.len(), 100 + 1 + 36 + 4070);
        let args = [
            OsString::from("-c"),
            OsString::from("printf '%s' \"$1\" >&2"),
            OsString::from("sh"),
            OsString::from(payload),
        ];
        let (_, line) = run_bounded_with_stderr("sh", &args, None, &[], Duration::from_secs(20))
            .expect("sh runs");
        for run in SECRET.as_bytes().windows(8) {
            let run = std::str::from_utf8(run).expect("ascii");
            assert!(
                !line.contains(run),
                "a token fragment {run:?} leaked: {line}"
            );
        }
    }

    // ---- SAFE-04: the capture pump between the master's stdio and launch-N.jsonl ----

    /// A reader that returns its chunks one `read` at a time, as a pipe does when the
    /// child splits a line across two writes.
    struct Chunks {
        chunks: std::collections::VecDeque<Vec<u8>>,
        at: usize,
    }

    impl Read for Chunks {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let Some(chunk) = self.chunks.front() else {
                return Ok(0);
            };
            let n = (chunk.len() - self.at).min(buf.len());
            buf[..n].copy_from_slice(&chunk[self.at..self.at + n]);
            self.at += n;
            if self.at == chunk.len() {
                self.chunks.pop_front();
                self.at = 0;
            }
            Ok(n)
        }
    }

    fn pump_chunks(chunks: &[&[u8]]) -> String {
        let reader = BufReader::with_capacity(
            8,
            Chunks {
                chunks: chunks.iter().map(|c| c.to_vec()).collect(),
                at: 0,
            },
        );
        let mut out = Vec::new();
        pump_redacted(reader, &mut out);
        String::from_utf8(out).expect("the pump writes UTF-8")
    }

    /// No 8-byte window of `SECRET` appears in `text`.
    fn assert_no_token_window(text: &str) {
        for run in SECRET.as_bytes().windows(8) {
            let run = std::str::from_utf8(run).expect("ascii");
            assert!(
                !text.contains(run),
                "a token fragment {run:?} leaked: {text}"
            );
        }
    }

    #[test]
    fn a_json_line_split_inside_a_token_is_redacted_whole_and_stays_json() {
        let line = format!(r#"{{"type":"assistant","text":"key sk-ant-{SECRET} end"}}"#);
        let cut = line.find(SECRET).expect("token present") + 10;
        let out = pump_chunks(&[
            &line.as_bytes()[..cut],
            format!("{}\n", &line[cut..]).as_bytes(),
        ]);
        assert_no_token_window(&out);
        let value: serde_json::Value =
            serde_json::from_str(out.trim_end()).expect("still one JSON line");
        assert_eq!(value["type"], "assistant");
        assert!(out.contains("[REDACTED:anthropic-key]"), "{out}");
    }

    #[test]
    fn a_text_line_with_a_token_is_text_redacted() {
        let out = pump_chunks(&[
            format!("plain line\ntoken ghp_{}", &SECRET[..9]).as_bytes(),
            format!("{} end\nafter\n", &SECRET[9..]).as_bytes(),
        ]);
        assert_no_token_window(&out);
        assert!(out.contains("plain line\n"), "{out}");
        assert!(out.contains("[REDACTED:github-token]"), "{out}");
        assert!(out.ends_with("after\n"), "{out}");
    }

    #[test]
    fn the_oauth_revoked_marker_and_type_survive_redaction_so_the_outcome_is_unchanged() {
        let line = serde_json::json!({
            "type": "result",
            "is_error": true,
            "result": format!("Failed to authenticate: OAuth token revoked ghp_{SECRET}"),
        })
        .to_string();
        let out = pump_chunks(&[line.as_bytes(), b"\n"]);
        assert_no_token_window(&out);
        let mut acc = super::super::outcome::StreamAccumulator::default();
        acc.push(&out);
        let summary = acc.into_summary(chrono::Utc::now());
        assert!(!summary.result_ok(), "an error result stays an error");
        assert!(out.contains("OAuth token revoked"), "{out}");
    }

    #[test]
    fn a_line_over_the_cap_becomes_the_note_and_none_of_its_bytes_are_kept() {
        let mut over = format!("ghp_{SECRET}").into_bytes();
        over.resize(MAX_STREAM_LINE as usize + 100, b'x');
        over.extend_from_slice(b"\nnext\n");
        let out = pump_chunks(&[&over]);
        assert_eq!(out, format!("{OVERLONG_STREAM_NOTE}\nnext\n"));
    }

    #[test]
    fn a_final_line_with_no_newline_is_redacted_whole_at_eof() {
        let out = pump_chunks(&[b"first\n", format!("last ghp_{SECRET}").as_bytes()]);
        assert_no_token_window(&out);
        assert_eq!(out, "first\nlast [REDACTED:github-token]\n");
    }

    #[test]
    fn a_write_error_does_not_stop_the_pump_from_draining_its_input() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut reader = BufReader::with_capacity(8, &b"one\ntwo\nthree\n"[..]);
        pump_redacted(&mut reader, Broken);
        assert!(
            reader.fill_buf().expect("readable").is_empty(),
            "input drained"
        );
    }

    #[test]
    fn the_child_stdout_and_stderr_reach_the_file_redacted_even_when_a_token_is_split() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("launch-1.jsonl");
        let file = std::fs::File::create(&path).expect("create");
        // Each token is printed in two writes with a pause between, once per stream.
        let script = "printf 'plain out\\n'; printf 'tok sk-ant-%s' \"$1\"; sleep 0.2; \
                      printf '%s end\\n' \"$2\"; \
                      printf 'tok ghp_%s' \"$1\" >&2; sleep 0.2; printf '%s end\\n' \"$2\" >&2; \
                      printf 'plain err\\n' >&2";
        let argv = [
            OsString::from("sh"),
            OsString::from("-c"),
            OsString::from(script),
            OsString::from("sh"),
            OsString::from(&SECRET[..9]),
            OsString::from(&SECRET[9..]),
        ];
        let env = BTreeMap::from([(
            OsString::from("PATH"),
            std::env::var_os("PATH").unwrap_or_default(),
        )]);
        let (mut child, capture) =
            spawn_launch_child(&argv, &env, dir.path(), file).expect("spawn");
        child.wait().expect("wait");
        assert!(
            capture.finish(Duration::from_secs(10)),
            "the pump reached EOF"
        );
        let text = std::fs::read_to_string(&path).expect("read");
        assert_no_token_window(&text);
        assert!(text.contains("plain out\n"), "{text}");
        assert!(text.contains("plain err\n"), "{text}");
        assert!(text.contains("[REDACTED:anthropic-key]"), "{text}");
        assert!(text.contains("[REDACTED:github-token]"), "{text}");
    }
}
