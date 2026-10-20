#![allow(dead_code)]
// ============================================================================
// Shared support for the Phase 28 launcher tests (plan 28-01; later plans reuse it).
//
// The launcher's real collaborators are `systemd-run`, `bwrap`, `systemctl` and
// `claude`. CI runners have none of them, so these tests put FAKE executables
// first on `PATH`, written at runtime as `/bin/sh` scripts. Each fake logs its
// argv to `$FAKE_SHIM_LOG` and then behaves like the real tool in the one way
// the launcher depends on:
//
//   * `systemd-run` drops its options through the first `--`, records its own
//     pid under the unit name (a scope execs the command, so the pid IS the
//     command's), and execs the rest;
//   * `bwrap` logs and execs after the first `--`;
//   * `systemctl` answers `is-system-running`, `is-active` (by probing the pid
//     file) and `stop` (SIGTERM to the process group, then wait);
//   * `claude` answers `--version`, records its environment (token redacted),
//     its argv prompt and where its stdin points, and then runs the scenario a
//     test wrote.
//
// Everything runs under a scratch directory with short paths (the unix socket
// limit is 107 bytes), a private `HOME`, `XDG_*` directories and a 0600 stored
// token, so no test touches the real user's files.
// ============================================================================

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::lock::{read_holder_at, run_lock_path};
use gsd_meta_manager::launcher::config_dir::run_state_dir_in;
use rustix::process::{kill_process, kill_process_group, Pid, Signal};
use tempfile::TempDir;

/// The run id every launcher test uses.
pub const RUN_ID: &str = "r28";

/// The registry alias of the fixture project.
pub const ALIAS: &str = "p28";

/// The stored token every fixture writes.
pub const TOKEN: &str = "tok-test-0123456789abcdef";

/// The human-owned DIRECTION body: two items.
pub const BODY: &str = "# Driver direction: launcher test

## Direction

- control: run
- goal: Exercise the launcher.

## Items

### I-01 First slice

- intent: Phase 1 is planned, executed and verified.
- budget: dispatches=3

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
";

/// The master prompt template the fixture's `--prompt-file` holds.
pub const PROMPT_TEMPLATE: &str =
    "You are the driver.\nRUN_DIR={RUN_DIR}\nPROJECT_ROOT={PROJECT_ROOT}\nMODE={MODE}\n";

const SYSTEMD_RUN_SHIM: &str = r#"#!/bin/sh
# Fake systemd-run: log, record the unit's pid, drop the options, exec the command.
SHIMS=$(dirname "$0")
{
  printf 'systemd-run'
  for a in "$@"; do printf ' %s' "$(printf '%s' "$a" | tr '\n' ' ')"; done
  printf '\n'
} >> "$FAKE_SHIM_LOG"
unit=""
for a in "$@"; do
  case "$a" in --unit=*) unit=${a#--unit=};; esac
done
[ -n "$unit" ] && echo $$ > "$SHIMS/$unit.pid"
while [ $# -gt 0 ]; do
  if [ "$1" = "--" ]; then shift; break; fi
  shift
done
exec "$@"
"#;

const BWRAP_SHIM: &str = r#"#!/bin/sh
# Fake bwrap: log, drop the options, exec the command.
{
  printf 'bwrap'
  for a in "$@"; do printf ' %s' "$(printf '%s' "$a" | tr '\n' ' ')"; done
  printf '\n'
} >> "$FAKE_SHIM_LOG"
while [ $# -gt 0 ]; do
  if [ "$1" = "--" ]; then shift; break; fi
  shift
done
exec "$@"
"#;

const SYSTEMCTL_SHIM: &str = r#"#!/bin/sh
# Fake systemctl: is-system-running, is-active and stop, keyed on the pid files.
SHIMS=$(dirname "$0")
{
  printf 'systemctl'
  for a in "$@"; do printf ' %s' "$a"; done
  printf '\n'
} >> "$FAKE_SHIM_LOG"
[ "$1" = "--user" ] && shift
verb=$1
unit=${2%.scope}
pid=""
[ -n "$unit" ] && pid=$(cat "$SHIMS/$unit.pid" 2>/dev/null)
case "$verb" in
  is-system-running) echo running; exit 0;;
  is-active)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then echo active; exit 0; fi
    echo inactive; exit 3;;
  stop)
    if [ -n "$pid" ]; then
      kill -s TERM -- "-$pid" 2>/dev/null
      i=0
      while kill -0 "$pid" 2>/dev/null && [ "$i" -lt 150 ]; do sleep 0.1; i=$((i + 1)); done
    fi
    exit 0;;
esac
exit 0
"#;

const CLAUDE_SHIM: &str = r#"#!/bin/sh
# Fake claude: answer --version, record what the launcher gave it, run the scenario.
if [ "$1" = "--version" ]; then echo "2.1.303 (Claude Code)"; exit 0; fi
REC=$FAKE_CLAUDE_DIR
mkdir -p "$REC"
env | sed 's/^\(CLAUDE_CODE_OAUTH_TOKEN\)=.*/\1=REDACTED/' > "$REC/env.txt"
printf '%s' "$CLAUDE_CODE_OAUTH_TOKEN" > "$REC/token.txt"
# The prompt is the argument after `--` (the launcher puts it last, WR-03).
PROMPT=; PREV=
for A in "$@"; do if [ "$PREV" = "--" ]; then PROMPT=$A; fi; PREV=$A; done
printf '%s' "$PROMPT" > "$REC/prompt.txt"
printf '%s\n' "$@" > "$REC/argv.txt"
readlink /proc/self/fd/0 > "$REC/stdin.txt"
if [ -n "$FAKE_CLAUDE_SCENARIO" ]; then exec /bin/sh "$FAKE_CLAUDE_SCENARIO"; fi
exit 0
"#;

/// Write `body` to `dir/name` as an executable.
pub fn write_executable(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, body).expect("write the executable");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod the executable");
    path
}

/// A project with one run directory, a registry, fake tools and a private home.
pub struct Fixture {
    pub project_dir: TempDir,
    pub scratch: TempDir,
    pub run_dir: PathBuf,
    /// The `XDG_RUNTIME_DIR` the launcher and `driver stop` run with: a private
    /// directory by default, the real one in the live tests.
    pub runtime_dir: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        Self::build(TempDir::new(), TempDir::new())
    }

    /// A fixture whose directories live under `base` instead of the system temp
    /// dir. The live tests need this: bwrap's `--tmpfs /tmp` hides everything
    /// under `/tmp`, so the fake `claude` and the project must not be there.
    pub fn new_in(base: &Path) -> Self {
        fs::create_dir_all(base).expect("fixture base dir");
        Self::build(TempDir::new_in(base), TempDir::new_in(base))
    }

    fn build(project_dir: std::io::Result<TempDir>, scratch: std::io::Result<TempDir>) -> Self {
        let project_dir = project_dir.expect("project dir");
        let scratch = scratch.expect("scratch dir");
        let run_dir = project_dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(RUN_ID);
        fs::create_dir_all(&run_dir).expect("run dir");
        fs::write(run_dir.join("DIRECTION.md"), BODY).expect("DIRECTION.md");

        let runtime_dir = scratch.path().join("run");
        let fixture = Fixture {
            project_dir,
            scratch,
            run_dir,
            runtime_dir,
        };
        for dir in ["run", "home", "config", "data", "shims", "rec"] {
            fs::create_dir_all(fixture.scratch.path().join(dir)).expect("scratch subdir");
        }
        fs::set_permissions(
            fixture.scratch.path().join("run"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("XDG_RUNTIME_DIR mode");

        let shims = fixture.shims();
        write_executable(&shims, "systemd-run", SYSTEMD_RUN_SHIM);
        write_executable(&shims, "bwrap", BWRAP_SHIM);
        write_executable(&shims, "systemctl", SYSTEMCTL_SHIM);
        write_executable(&shims, "claude", CLAUDE_SHIM);

        fixture.write_token(Some(TOKEN));
        fs::write(
            fixture.registry(),
            serde_json::json!({
                "version": 2,
                "projects": {ALIAS: {
                    "path": fixture.project(),
                    "added": "2026-01-01T00:00:00Z",
                }}
            })
            .to_string(),
        )
        .expect("registry");
        fs::write(fixture.prompt_file(), PROMPT_TEMPLATE).expect("prompt file");
        fixture
    }

    pub fn project(&self) -> &Path {
        self.project_dir.path()
    }

    pub fn shims(&self) -> PathBuf {
        self.scratch.path().join("shims")
    }

    /// Where the fake claude records what it was given.
    pub fn rec_dir(&self) -> PathBuf {
        self.scratch.path().join("rec")
    }

    pub fn registry(&self) -> PathBuf {
        self.scratch.path().join("registry.json")
    }

    pub fn prompt_file(&self) -> PathBuf {
        self.scratch.path().join("prompt.md")
    }

    pub fn home(&self) -> PathBuf {
        self.scratch.path().join("home")
    }

    pub fn shim_log_path(&self) -> PathBuf {
        self.scratch.path().join("shim.log")
    }

    pub fn scenario_path(&self) -> PathBuf {
        self.scratch.path().join("scenario.sh")
    }

    /// The stored token file, where `dirs::config_dir()` puts it for this fixture.
    pub fn token_path(&self) -> PathBuf {
        self.scratch
            .path()
            .join("config/gsd-meta-manager/driver-oauth-token")
    }

    /// Write (or, with `None`, remove) the 0600 stored token.
    pub fn write_token(&self, token: Option<&str>) {
        let path = self.token_path();
        match token {
            Some(token) => {
                fs::create_dir_all(path.parent().expect("token dir")).expect("token dir");
                fs::write(&path, token).expect("write the token");
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("chmod");
            }
            None => {
                let _ = fs::remove_file(&path);
            }
        }
    }

    /// The scenario the fake claude runs, as `/bin/sh` text.
    pub fn write_scenario(&self, body: &str) {
        fs::write(self.scenario_path(), body).expect("write the scenario");
    }

    /// The launcher's state directory for the fixture run (`<driver-root>/<hash>/<run>`).
    pub fn state_dir(&self) -> PathBuf {
        let root = self.scratch.path().join("data/gsd-meta-manager/driver");
        let project = fs::canonicalize(self.project()).expect("canonical project");
        run_state_dir_in(&root, &project, RUN_ID)
    }

    pub fn config_dir(&self) -> PathBuf {
        self.state_dir().join("claude-config")
    }

    pub fn run_file(&self, name: &str) -> String {
        fs::read_to_string(self.run_dir.join(name)).unwrap_or_default()
    }

    pub fn log(&self) -> String {
        self.run_file("LOG.md")
    }

    pub fn state_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.run_file("state.json")).expect("state.json parses")
    }

    pub fn shim_log(&self) -> String {
        fs::read_to_string(self.shim_log_path()).unwrap_or_default()
    }

    /// One of the files the fake claude recorded.
    pub fn recorded(&self, name: &str) -> String {
        fs::read_to_string(self.rec_dir().join(name)).unwrap_or_default()
    }

    /// The binary under a hermetic environment with the fake tools first on
    /// `PATH`, `--config` already given; the caller appends the verb.
    pub fn hermetic_command(&self) -> Command {
        let scratch = self.scratch.path();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
        cmd.arg("--config")
            .arg(self.registry())
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", self.shims().display()))
            .env("HOME", self.home())
            .env("XDG_RUNTIME_DIR", &self.runtime_dir)
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_DATA_HOME", scratch.join("data"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("FAKE_SHIM_LOG", self.shim_log_path())
            .env("FAKE_CLAUDE_DIR", self.rec_dir())
            .env("FAKE_CLAUDE_SCENARIO", self.scenario_path())
            // Host credentials the launcher must strip before the driver sees them.
            .env("SSH_AUTH_SOCK", "/tmp/fake-agent.sock")
            .env("GH_TOKEN", "ghp_fake_host_token")
            .env("ANTHROPIC_API_KEY", "sk-ant-fake-host-key")
            .stdin(Stdio::null());
        cmd
    }

    /// The `driver launch` command, not yet run: the real binary under a
    /// hermetic environment, with the fake tools first on `PATH`.
    pub fn launch_command(&self, run_id: &str, extra_args: &[&str]) -> Command {
        self.launch_command_for(ALIAS, run_id, extra_args)
    }

    /// [`launch_command`](Self::launch_command) with an explicit project target.
    pub fn launch_command_for(&self, target: &str, run_id: &str, extra_args: &[&str]) -> Command {
        let mut cmd = self.launch_command_raw(target, run_id, extra_args);
        cmd.arg("--foreground");
        cmd
    }

    /// `driver launch` exactly as given: no `--foreground`, so it detaches.
    pub fn launch_command_raw(&self, target: &str, run_id: &str, extra_args: &[&str]) -> Command {
        let mut cmd = self.hermetic_command();
        cmd.args(["driver", "launch", target, run_id, "--prompt-file"])
            .arg(self.prompt_file())
            .args(extra_args);
        cmd
    }

    /// Run `driver launch` for [`RUN_ID`] to completion.
    pub fn launch(&self, extra_args: &[&str]) -> Output {
        self.launch_command(RUN_ID, extra_args)
            .output()
            .expect("the binary is built and spawnable")
    }

    /// Start a foreground `driver launch` for [`RUN_ID`] and return at once.
    pub fn spawn_launch(&self, extra_args: &[&str]) -> Child {
        self.launch_command(RUN_ID, extra_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary is built and spawnable")
    }

    /// `driver stop <alias> <run-id>`, not yet run.
    pub fn stop_command(&self, run_id: &str, extra_args: &[&str]) -> Command {
        let mut cmd = self.hermetic_command();
        cmd.args(["driver", "stop", ALIAS, run_id]).args(extra_args);
        cmd
    }

    /// Whether `pid` is a process this fixture started: its environment carries
    /// this fixture's `FAKE_SHIM_LOG`. A pid file can outlive its process, and a
    /// recycled pid must never be signalled.
    fn is_ours(&self, pid: u32) -> bool {
        let marker = format!("FAKE_SHIM_LOG={}", self.shim_log_path().display());
        fs::read(format!("/proc/{pid}/environ")).is_ok_and(|raw| {
            raw.split(|byte| *byte == 0)
                .any(|entry| entry == marker.as_bytes())
        })
    }

    /// The fake driver process groups the shims recorded (`<unit>.pid`).
    pub fn driver_pids(&self) -> Vec<u32> {
        let Ok(entries) = fs::read_dir(self.shims()) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "pid"))
            .filter_map(|e| fs::read_to_string(e.path()).ok())
            .filter_map(|text| text.trim().parse::<u32>().ok())
            .collect()
    }

    /// Whether any fake driver process group member is still alive.
    pub fn any_driver_alive(&self) -> bool {
        self.driver_pids()
            .into_iter()
            .any(|pid| fs::metadata(format!("/proc/{pid}")).is_ok() && self.is_ours(pid))
    }

    /// SIGKILL every fake driver group and the launcher this fixture started.
    pub fn kill_fixture_processes(&self) {
        for pid in self.driver_pids() {
            if self.is_ours(pid) {
                if let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) {
                    let _ = kill_process_group(pid, Signal::KILL);
                }
            }
        }
        if let Some(holder) = read_holder_at(&run_lock_path(&self.run_dir)) {
            if self.is_ours(holder.pgid) {
                if let Some(pid) = i32::try_from(holder.pgid).ok().and_then(Pid::from_raw) {
                    let _ = kill_process(pid, Signal::KILL);
                }
            }
        }
    }

    /// Let a scenario that waits for [`RELEASE_FILE`] go on.
    pub fn release(&self) {
        fs::create_dir_all(self.rec_dir()).expect("rec dir");
        fs::write(self.rec_dir().join(RELEASE_FILE), "go").expect("release file");
    }

    /// Whether the scenario has started (it created [`STARTED_FILE`]).
    pub fn scenario_started(&self) -> bool {
        self.rec_dir().join(STARTED_FILE).exists()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.kill_fixture_processes();
    }
}

/// Poll `cond` every 50 ms until it holds or `timeout` passes.
pub fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The file a scenario creates once the driver has logged its start.
pub const STARTED_FILE: &str = "started";

/// The file a scenario waits for before it goes on.
pub const RELEASE_FILE: &str = "release";

/// A driver that starts, announces itself and then waits for [`Fixture::release`],
/// ending the way a well-mannered one does.
pub const WAITING_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
touch "$FAKE_CLAUDE_DIR/started"
while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

/// A scenario that behaves like a well-mannered driver: start, one allowed
/// dispatch gate, end with `stop`, and a successful result line.
pub const WELL_MANNERED_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{"prompt":"run phase 1"}}' | driverctl hook > "$FAKE_CLAUDE_DIR/hook_out.txt"
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;
