//! Test-only fixtures for the focus path: a process table laid out in a
//! directory ([`FakeProc`]) and backends pointed at fixture scripts
//! ([`World`]). Nothing here starts a real multiplexer, and nothing reads the
//! real `/proc`, so the tests run where neither tmux nor herdr exists.
//!
//! A fake `/proc` is just the files [`super::ProcFs`] reads: `<pid>/stat`,
//! `<pid>/cmdline`, `<pid>/environ` and the `<pid>/fd/0` link. The fixture
//! scripts are `#!/bin/sh` programs that append their argv (and the herdr
//! socket they were handed) to ONE shared log, so a test can assert the exact
//! order of the herdr and tmux calls.

use super::herdr::HerdrBackend;
use super::tmux::TmuxBackend;
use super::{MultiplexerBackend, ProcFs};
use crate::session_detector::{ClaudeSession, SessionKind};
use crate::text::Untrusted;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// Serialises writing a fixture script and running it, across EVERY test that
/// does. A fork on another test thread inherits a just-written script's write
/// descriptor until it execs, which fails OUR exec with ETXTBSY ("Text file
/// busy"); the script is therefore written and every process that runs it exits
/// inside this lock, and a residual ETXTBSY is retried (the established pattern
/// in `tests/launcher_config_dir.rs`).
pub(crate) static EXEC_LOCK: Mutex<()> = Mutex::new(());

/// Take [`EXEC_LOCK`], recovering from a poisoned one (a failed test must not
/// fail the others).
pub(crate) fn exec_lock() -> MutexGuard<'static, ()> {
    EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A process table in a temporary directory.
pub(crate) struct FakeProc {
    dir: tempfile::TempDir,
}

impl FakeProc {
    pub(crate) fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    /// The [`ProcFs`] that reads this table.
    pub(crate) fn procfs(&self) -> ProcFs {
        ProcFs::at(self.dir.path())
    }

    /// Add one process: its comm and parent, its argv, its `environ` entries
    /// (`KEY=VALUE`) and the target of its stdin link (`None`: no `fd/0`).
    #[cfg(unix)]
    pub(crate) fn process(
        &self,
        pid: u32,
        comm: &str,
        ppid: u32,
        argv: &[&str],
        environ: &[&str],
        stdin: Option<&str>,
    ) -> &Self {
        let dir = self.dir.path().join(pid.to_string());
        std::fs::create_dir_all(dir.join("fd")).expect("proc dir");
        std::fs::write(
            dir.join("stat"),
            format!(
                "{pid} ({comm}) S {ppid} {pid} {pid} 0 -1 4194560 0 0 0 0 0 0 0 0 20 0 1 0 100 0 0"
            ),
        )
        .expect("stat");
        let nul_joined = |parts: &[&str]| -> Vec<u8> {
            parts
                .iter()
                .flat_map(|p| p.bytes().chain(std::iter::once(0)))
                .collect()
        };
        std::fs::write(dir.join("cmdline"), nul_joined(argv)).expect("cmdline");
        std::fs::write(dir.join("environ"), nul_joined(environ)).expect("environ");
        if let Some(target) = stdin {
            std::os::unix::fs::symlink(target, dir.join("fd").join("0")).expect("fd/0");
        }
        self
    }
}

/// The herdr socket the measured sessions' environ carried.
pub(crate) const HERDR_SOCKET: &str = "/home/blk/.config/herdr/herdr.sock";

/// The tmux server socket of the measured machine (`$TMUX` before the first
/// comma).
pub(crate) const TMUX_SOCKET: &str = "/tmp/tmux-1000/default";

/// A Claude session fixture. `tty` is the `pts/N` form the detector stores.
pub(crate) fn claude(pid: u32, session_id: Option<&str>, tty: Option<&str>) -> ClaudeSession {
    ClaudeSession {
        pid,
        kind: SessionKind::Claude,
        session_id: session_id.map(|s| Untrusted::from_untrusted_source(s.to_string())),
        working_dir: PathBuf::from("/work/proj"),
        start_time: None,
        tty: tty.map(str::to_string),
    }
}

/// `herdr pane list` JSON for `(pane id, tab id, session id)` rows.
pub(crate) fn pane_list_json(panes: &[(&str, &str, &str)]) -> String {
    let rows: Vec<String> = panes
        .iter()
        .map(|(id, tab, sid)| {
            format!(
                r#"{{"pane_id":"{id}","tab_id":"{tab}","cwd":"/work/proj","agent":"claude","agent_session":{{"kind":"id","value":"{sid}"}}}}"#
            )
        })
        .collect();
    format!(
        r#"{{"id":"cli:pane:list","result":{{"panes":[{}]}}}}"#,
        rows.join(",")
    )
}

/// Fixture `herdr` and `tmux` programs, a shared call log and the directory
/// holding what they print. Holds [`EXEC_LOCK`] for its whole life.
pub(crate) struct World {
    _guard: MutexGuard<'static, ()>,
    dir: tempfile::TempDir,
    pub(crate) tmux: &'static TmuxBackend,
    pub(crate) herdr: &'static HerdrBackend,
}

impl World {
    /// `pane_list`: what `herdr pane list` prints. `list_panes`: what
    /// `tmux list-panes -a -F ...` prints.
    #[cfg(unix)]
    pub(crate) fn new(pane_list: &str, list_panes: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;

        let guard = exec_lock();
        let dir = tempfile::tempdir().expect("tempdir");
        let log = dir.path().join("log");
        std::fs::write(dir.path().join("pane_list.json"), pane_list).expect("pane list");
        std::fs::write(dir.path().join("list_panes.txt"), list_panes).expect("list-panes");

        let herdr_script = format!(
            "#!/bin/sh\n\
             echo \"herdr $* sock=$HERDR_SOCKET_PATH\" >> \"{log}\"\n\
             case \"$1 $2\" in\n\
             \"pane list\") cat \"{dir}/pane_list.json\" ;;\n\
             \"agent focus\") echo '{{\"id\":\"cli:agent:focus\",\"result\":{{\"type\":\"ok\"}}}}' ;;\n\
             *) echo \"unexpected: $*\" >&2; exit 2 ;;\n\
             esac\n",
            log = log.display(),
            dir = dir.path().display(),
        );
        let tmux_script = format!(
            "#!/bin/sh\n\
             echo \"tmux $*\" >> \"{log}\"\n\
             if [ \"$1\" = \"-S\" ]; then shift 2; fi\n\
             case \"$1\" in\n\
             list-panes) cat \"{dir}/list_panes.txt\" ;;\n\
             select-window|select-pane) ;;\n\
             *) echo \"unexpected: $*\" >&2; exit 2 ;;\n\
             esac\n",
            log = log.display(),
            dir = dir.path().display(),
        );
        let write = |name: &str, body: &str| -> &'static str {
            let path = dir.path().join(name);
            std::fs::write(&path, body).expect("script");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
            Box::leak(path.to_string_lossy().into_owned().into_boxed_str())
        };
        let herdr: &'static HerdrBackend = Box::leak(Box::new(HerdrBackend::with_program(write(
            "herdr",
            &herdr_script,
        ))));
        let tmux: &'static TmuxBackend = Box::leak(Box::new(TmuxBackend::with_program(write(
            "tmux",
            &tmux_script,
        ))));
        Self {
            _guard: guard,
            dir,
            tmux,
            herdr,
        }
    }

    /// The registry the focus path sees: tmux, then herdr, as the real one.
    pub(crate) fn registry(&self) -> [&'static dyn MultiplexerBackend; 2] {
        [self.tmux, self.herdr]
    }

    fn log_path(&self) -> PathBuf {
        self.dir.path().join("log")
    }

    /// Every call the fixture programs received, in order, one line each.
    pub(crate) fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.log_path())
            .map(|text| text.lines().map(str::to_string).collect())
            .unwrap_or_default()
    }

    /// Run `f` and return its message with the call log, retrying a residual
    /// ETXTBSY. Each attempt starts from an empty log.
    pub(crate) fn run(&self, f: impl Fn() -> String) -> (String, Vec<String>) {
        let mut message = String::new();
        for _ in 0..5 {
            let _ = std::fs::remove_file(self.log_path());
            message = f();
            if !message.contains("Text file busy") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        (message, self.calls())
    }
}

/// The fixture programs' tmux call, exactly as the log shows it, for `rest`
/// addressed to `socket`.
pub(crate) fn tmux_call(socket: Option<&str>, rest: &str) -> String {
    match socket {
        Some(socket) => format!("tmux -S {socket} {rest}"),
        None => format!("tmux {rest}"),
    }
}

/// The `list-panes` call the tmux backend makes (the format string is the
/// backend's, pinned here so a drift shows up).
pub(crate) const LIST_PANES_ARGS: &str =
    "list-panes -a -F #{pane_tty} #{session_name}:#{window_index}.#{pane_index}";
