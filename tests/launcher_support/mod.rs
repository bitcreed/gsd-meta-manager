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
use std::process::{Command, Output, Stdio};

use gsd_meta_manager::launcher::config_dir::run_state_dir_in;
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
printf '%s' "$2" > "$REC/prompt.txt"
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
}

impl Fixture {
    pub fn new() -> Self {
        let project_dir = TempDir::new().expect("project dir");
        let scratch = TempDir::new().expect("scratch dir");
        let run_dir = project_dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(RUN_ID);
        fs::create_dir_all(&run_dir).expect("run dir");
        fs::write(run_dir.join("DIRECTION.md"), BODY).expect("DIRECTION.md");

        let fixture = Fixture {
            project_dir,
            scratch,
            run_dir,
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

    /// The `driver launch` command, not yet run: the real binary under a
    /// hermetic environment, with the fake tools first on `PATH`.
    pub fn launch_command(&self, run_id: &str, extra_args: &[&str]) -> Command {
        self.launch_command_for(ALIAS, run_id, extra_args)
    }

    /// [`launch_command`](Self::launch_command) with an explicit project target.
    pub fn launch_command_for(&self, target: &str, run_id: &str, extra_args: &[&str]) -> Command {
        let scratch = self.scratch.path();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
        cmd.arg("--config")
            .arg(self.registry())
            .args(["driver", "launch", target, run_id, "--prompt-file"])
            .arg(self.prompt_file())
            .args(extra_args)
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", self.shims().display()))
            .env("HOME", self.home())
            .env("XDG_RUNTIME_DIR", scratch.join("run"))
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

    /// Run `driver launch` for [`RUN_ID`] to completion.
    pub fn launch(&self, extra_args: &[&str]) -> Output {
        self.launch_command(RUN_ID, extra_args)
            .output()
            .expect("the binary is built and spawnable")
    }
}

/// A scenario that behaves like a well-mannered driver: start, one allowed
/// dispatch gate, end with `stop`, and a successful result line.
pub const WELL_MANNERED_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{"prompt":"run phase 1"}}' | driverctl hook > "$FAKE_CLAUDE_DIR/hook_out.txt"
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;
