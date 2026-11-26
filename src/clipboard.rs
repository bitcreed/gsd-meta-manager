//! Copying the inbox checklist to the clipboard (37-07, D-09).
//!
//! A native tool when one is present, else an OSC 52 sequence the main loop
//! writes to the terminal. The choice is a pure function over [`ClipboardProbes`]
//! ([`pick_tool`]); [`probe`] is the only place the environment is read.
//!
//! **The target OS is data, not a `cfg`.** [`ClipboardProbes::os`] carries
//! `std::env::consts::OS`, so every branch compiles on every target and the
//! cross-target clippy gate (`-D warnings`) sees no Windows-only dead code.
//!
//! **Why stdout and stderr are null (RESEARCH Pitfall 4).** `wl-copy` and
//! `xclip` fork a background process that keeps serving the selection and
//! inherits the pipes. A reader waiting for end-of-file on a pipe would then wait
//! for as long as the selection lives, so the child gets no pipe the caller
//! waits on: stdin is written and closed, the exit status is polled with
//! `try_wait`, and a budget kills a tool that never exits.
//!
//! **Nothing here takes a program name or argument from project data (T-37-28):**
//! the five argument vectors are constants, and the text travels on stdin.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The most bytes `y` copies (D-09): a checklist is a phone note, not a log.
pub const MAX_COPY_BYTES: usize = 64 * 1024;

/// The first `try_wait` poll comes this soon; each later gap doubles.
const FIRST_POLL: Duration = Duration::from_millis(1);

/// The widest gap between two polls.
const MAX_POLL: Duration = Duration::from_millis(25);

/// The facts the tool choice is made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardProbes {
    /// `std::env::consts::OS` (`linux`, `macos`, `windows`, ...), as data.
    pub os: &'static str,
    /// `WAYLAND_DISPLAY` is set and non-empty.
    pub wayland_display: bool,
    /// `DISPLAY` is set and non-empty.
    pub display: bool,
    /// Which of the five tool names exist on `PATH`.
    pub on_path: Vec<&'static str>,
}

/// A native clipboard tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    WlCopy,
    Xclip,
    Xsel,
    Pbcopy,
    ClipExe,
}

/// Every tool name [`probe`] looks for on `PATH`.
const TOOL_NAMES: [&str; 5] = ["wl-copy", "xclip", "xsel", "pbcopy", "clip.exe"];

impl Tool {
    /// The name shown in the status line.
    pub fn name(self) -> &'static str {
        match self {
            Tool::WlCopy => "wl-copy",
            Tool::Xclip => "xclip",
            Tool::Xsel => "xsel",
            Tool::Pbcopy => "pbcopy",
            Tool::ClipExe => "clip.exe",
        }
    }

    /// The program and its fixed arguments.
    pub fn argv(self) -> Vec<&'static str> {
        match self {
            Tool::WlCopy => vec!["wl-copy"],
            Tool::Xclip => vec!["xclip", "-selection", "clipboard"],
            Tool::Xsel => vec!["xsel", "--clipboard", "--input"],
            Tool::Pbcopy => vec!["pbcopy"],
            Tool::ClipExe => vec!["clip.exe"],
        }
    }
}

/// D-09's order: `pbcopy` on macOS, `clip.exe` on Windows, else `wl-copy` under
/// `WAYLAND_DISPLAY`, then `xclip`, then `xsel` under `DISPLAY`. `None` means
/// OSC 52. **Pure.**
pub fn pick_tool(probes: &ClipboardProbes) -> Option<Tool> {
    let has = |name: &str| probes.on_path.contains(&name);
    match probes.os {
        "macos" => has("pbcopy").then_some(Tool::Pbcopy),
        "windows" => has("clip.exe").then_some(Tool::ClipExe),
        _ => {
            if probes.wayland_display && has("wl-copy") {
                Some(Tool::WlCopy)
            } else if probes.display && has("xclip") {
                Some(Tool::Xclip)
            } else if probes.display && has("xsel") {
                Some(Tool::Xsel)
            } else {
                None
            }
        }
    }
}

/// Gather the facts [`pick_tool`] decides from. **The only environment reader:**
/// the OS constant, the two display variables and a `split_paths` scan of `PATH`
/// (no `which` child).
pub fn probe() -> ClipboardProbes {
    let set = |name: &str| std::env::var_os(name).is_some_and(|v| !v.is_empty());
    let path = std::env::var_os("PATH");
    ClipboardProbes {
        os: std::env::consts::OS,
        wayland_display: set("WAYLAND_DISPLAY"),
        display: set("DISPLAY"),
        on_path: tools_on_path(path.as_deref()),
    }
}

/// The tool names found as files in a directory of `path`.
fn tools_on_path(path: Option<&std::ffi::OsStr>) -> Vec<&'static str> {
    let Some(path) = path else {
        return Vec::new();
    };
    let dirs: Vec<std::path::PathBuf> = std::env::split_paths(path).collect();
    TOOL_NAMES
        .into_iter()
        .filter(|name| dirs.iter().any(|dir| dir.join(name).is_file()))
        .collect()
}

/// Run `tool` with `text` on stdin, within `budget`.
pub fn copy_with_tool(tool: Tool, text: &str, budget: Duration) -> Result<(), String> {
    copy_with_argv(&tool.argv(), text, budget)
}

/// [`copy_with_tool`] over an explicit argument vector, so a test can point it
/// at a fake tool.
///
/// The text is written from a thread so a tool that never reads cannot block the
/// caller past `budget`; the child is killed and reaped on expiry, which also
/// ends the writer (its pipe breaks). `Err("exited {code}")` for a failing exit
/// status and `Err("did not finish")` for the budget.
pub(crate) fn copy_with_argv(argv: &[&str], text: &str, budget: Duration) -> Result<(), String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("no tool".to_owned());
    };
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        let bytes = text.as_bytes().to_vec();
        std::thread::spawn(move || {
            // A tool that exits without reading breaks the pipe; its exit status
            // is the answer, not this write.
            let _ = stdin.write_all(&bytes);
            drop(stdin);
        });
    }
    let deadline = Instant::now() + budget;
    let mut poll = FIRST_POLL;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(match status.code() {
                        Some(code) => format!("exited {code}"),
                        None => "exited by a signal".to_owned(),
                    })
                };
            }
            Ok(None) => {}
            Err(e) => return Err(format!("could not wait: {e}")),
        }
        let now = Instant::now();
        if now >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("did not finish".to_owned());
        }
        std::thread::sleep(poll.min(deadline - now));
        poll = (poll * 2).min(MAX_POLL);
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// RFC 4648 base64 with padding.
pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        let sextet = |shift: u32| B64[((n >> shift) & 0x3f) as usize] as char;
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if chunk.len() > 1 { sextet(6) } else { '=' });
        out.push(if chunk.len() > 2 { sextet(0) } else { '=' });
    }
    out
}

/// The OSC 52 sequence that asks the terminal to set its clipboard to `text`:
/// `ESC ] 52 ; c ;` + base64 + `BEL`. The payload is base64 only, so no byte of
/// the text can end or alter the sequence (T-37-25).
pub fn osc52(text: &str) -> String {
    format!("\u{1b}]52;c;{}\u{7}", base64(text.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probes(
        os: &'static str,
        wayland: bool,
        display: bool,
        tools: &[&'static str],
    ) -> ClipboardProbes {
        ClipboardProbes {
            os,
            wayland_display: wayland,
            display,
            on_path: tools.to_vec(),
        }
    }

    #[test]
    fn pick_tool_by_environment() {
        let cases = [
            (
                probes("macos", false, false, &["pbcopy"]),
                Some(Tool::Pbcopy),
            ),
            (probes("macos", false, false, &[]), None),
            (
                probes("windows", false, false, &["clip.exe"]),
                Some(Tool::ClipExe),
            ),
            (
                probes("linux", true, true, &["wl-copy", "xclip"]),
                Some(Tool::WlCopy),
            ),
            (probes("linux", true, false, &["xclip"]), None),
            (
                probes("linux", false, true, &["xclip", "xsel"]),
                Some(Tool::Xclip),
            ),
            (probes("linux", false, true, &["xsel"]), Some(Tool::Xsel)),
            (
                probes("linux", false, false, &["wl-copy", "xclip", "xsel"]),
                None,
            ),
            (probes("linux", true, true, &[]), None),
        ];
        for (p, want) in cases {
            assert_eq!(pick_tool(&p), want, "{p:?}");
        }
        assert_eq!(Tool::Xclip.argv(), ["xclip", "-selection", "clipboard"]);
        assert_eq!(Tool::Xsel.argv(), ["xsel", "--clipboard", "--input"]);
        assert_eq!(Tool::WlCopy.name(), "wl-copy");
    }

    #[test]
    fn base64_vectors() {
        for (input, want) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), want, "{input:?}");
        }
    }

    #[test]
    fn osc52_payload_is_base64_only() {
        let hostile = "a\u{1b}]52;c;x\u{7}\u{9b}b\u{202e}c\n";
        let seq = osc52(hostile);
        let payload = seq
            .strip_prefix("\u{1b}]52;c;")
            .and_then(|rest| rest.strip_suffix('\u{7}'))
            .expect("framed");
        assert!(
            payload
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')),
            "{payload:?}"
        );
        assert_eq!(seq.matches('\u{1b}').count(), 1);
        assert_eq!(seq.matches('\u{7}').count(), 1);
    }

    #[test]
    fn tools_on_path_scans_the_path_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("xsel"), "").expect("write");
        std::fs::write(dir.path().join("other"), "").expect("write");
        let path = std::env::join_paths([dir.path()]).expect("join");
        assert_eq!(tools_on_path(Some(&path)), ["xsel"]);
        assert!(tools_on_path(None).is_empty());
    }

    #[test]
    fn an_empty_argv_is_refused() {
        assert_eq!(
            copy_with_argv(&[], "x", Duration::from_secs(1)),
            Err("no tool".to_owned())
        );
    }

    #[cfg(unix)]
    mod unix {
        use super::super::*;

        /// Run `script` under `sh` (the script file is never exec'd, so no ETXTBSY
        /// race with another test thread's fork).
        fn run(script: &str, text: &str, budget: Duration) -> Result<(), String> {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("tool.sh");
            std::fs::write(&path, script).expect("script");
            let shown = path.to_string_lossy().into_owned();
            copy_with_argv(&["sh", &shown], text, budget)
        }

        #[test]
        fn copy_with_tool_feeds_stdin() {
            let dir = tempfile::tempdir().expect("tempdir");
            let out = dir.path().join("got.txt");
            let script = format!("cat > '{}'\n", out.display());
            run(&script, "one\ntwo\n", Duration::from_secs(5)).expect("ok");
            assert_eq!(std::fs::read_to_string(out).expect("read"), "one\ntwo\n");
        }

        #[test]
        fn copy_with_tool_survives_a_forking_tool() {
            let started = Instant::now();
            run(
                "cat > /dev/null\nsleep 30 &\nexit 0\n",
                "text",
                Duration::from_secs(5),
            )
            .expect("a forking tool is fine");
            assert!(started.elapsed() < Duration::from_secs(4));
        }

        #[test]
        fn copy_with_tool_reports_the_exit_code() {
            assert_eq!(
                run("cat > /dev/null\nexit 3\n", "x", Duration::from_secs(5)),
                Err("exited 3".to_owned())
            );
        }

        #[test]
        fn copy_with_tool_kills_a_tool_that_never_exits() {
            let started = Instant::now();
            assert_eq!(
                run("exec sleep 30\n", "x", Duration::from_millis(150)),
                Err("did not finish".to_owned())
            );
            assert!(started.elapsed() < Duration::from_secs(5));
        }

        #[test]
        fn a_tool_that_never_reads_cannot_block_the_writer() {
            let big = "y".repeat(MAX_COPY_BYTES);
            assert_eq!(
                run("exec sleep 30\n", &big, Duration::from_millis(150)),
                Err("did not finish".to_owned())
            );
        }
    }
}
