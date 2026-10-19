#![cfg(target_os = "linux")]
// ============================================================================
// The push step through the REAL binary (Phase 28, plan 28-08; LNCH-06, D-24,
// D-25, D-26; RESEARCH §4).
//
// A driven run's commits leave the machine only through the host-side push step:
// the launcher fetches the recorded branch's tip from the checkout into a bare
// mirror it owns and pushes it, with one fixed refspec, to
// `refs/heads/gsd-auto/<alias>/<run-id>` of the URL pinned when the mirror was
// created. The remote here is a local bare repository (`file://`), the `claude`
// is the fake of `tests/launcher_support` (it runs in the checkout and really
// commits), and `gh` is a shim that logs its argv. Nothing touches a network, a
// real remote or the user's own git configuration.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use launcher_support::{Fixture, ALIAS, RUN_ID};

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A `git` the test runs itself, in `dir`, with no user or system configuration.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A bare repository the pushes land in.
struct Remote {
    path: PathBuf,
}

impl Remote {
    fn new(fx: &Fixture, name: &str) -> Remote {
        let path = fx.scratch.path().join(name);
        fs::create_dir_all(&path).expect("remote dir");
        git(&path, &["init", "--quiet", "--bare"]);
        Remote { path }
    }

    fn url(&self) -> String {
        format!("file://{}", self.path.display())
    }

    /// The commit `refs/heads/<branch>` points at, if it exists.
    fn branch(&self, branch: &str) -> Option<String> {
        let out = Command::new("git")
            .arg("--git-dir")
            .arg(&self.path)
            .args(["rev-parse", "--verify", "--quiet"])
            .arg(format!("refs/heads/{branch}"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("git runs");
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }

    /// Every ref the remote holds.
    fn refs(&self) -> String {
        git(&self.path, &["for-each-ref", "--format=%(refname)"])
    }
}

/// Make the fixture's project a git repository on branch `work` with one commit and
/// `origin` pointing at `origin_url`.
fn init_project(fx: &Fixture, origin_url: &str) {
    let dir = fx.project();
    git(dir, &["init", "--quiet", "-b", "work"]);
    git(dir, &["config", "user.name", "Driver Test"]);
    git(dir, &["config", "user.email", "driver@example.invalid"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
    fs::write(dir.join("README"), "base\n").expect("README");
    git(dir, &["add", "README"]);
    git(dir, &["commit", "--quiet", "-m", "base"]);
    git(dir, &["remote", "add", "origin", origin_url]);
}

/// A driver that commits one file on the checkout's branch and ends the run.
const COMMIT_AND_END: &str = r#"
driverctl log start --mode headless --loop-mode loop
echo "driver work" > feature.txt
git add feature.txt
git commit --quiet -m "driver work"
driverctl log end --reason all-done
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

/// `driver push p28 r28`, not yet run.
fn push_command(fx: &Fixture) -> Command {
    let mut cmd = fx.hermetic_command();
    cmd.args(["driver", "push", ALIAS, RUN_ID]);
    cmd
}

fn run_push(fx: &Fixture) -> std::process::Output {
    push_command(fx).output().expect("spawn driver push")
}

/// The LOG lines of one event: `RUN | <event> |` (so `push` does not match
/// `push-refused`).
fn lines_of<'a>(log: &'a str, event: &str) -> Vec<&'a str> {
    let needle = format!("RUN | {event} |");
    log.lines().filter(|l| l.contains(&needle)).collect()
}

#[test]
fn push_sends_the_recorded_branch_tip_into_the_namespace() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    let start = git(fx.project(), &["rev-parse", "HEAD"]);
    fx.write_scenario(COMMIT_AND_END);

    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    assert!(
        stdout(&launched).contains("ended status=done"),
        "{}",
        stdout(&launched)
    );
    let tip = git(fx.project(), &["rev-parse", "work"]);
    assert_ne!(tip, start, "the fake driver committed");

    // The launch recorded what the push will send, before the driver existed.
    let state = fx.state_json();
    assert_eq!(state["launch_branch"], "work");
    assert_eq!(state["launch_start_sha"], start.as_str());

    let pushed = run_push(&fx);
    assert!(pushed.status.success(), "{}", stderr(&pushed));
    assert!(stdout(&pushed).contains("pushed"), "{}", stdout(&pushed));
    assert_eq!(
        origin.branch("gsd-auto/p28/r28").as_deref(),
        Some(tip.as_str()),
        "the reserved branch holds the project's tip"
    );
    assert_eq!(
        origin.refs(),
        "refs/heads/gsd-auto/p28/r28",
        "nothing else reached the remote"
    );

    let log = fx.log();
    let push_lines = lines_of(&log, "push");
    assert_eq!(push_lines.len(), 1, "{log}");
    assert!(
        push_lines[0].contains("dest=refs/heads/gsd-auto/p28/r28")
            && push_lines[0].contains("outcome=pushed"),
        "{}",
        push_lines[0]
    );
    let state = fx.state_json();
    assert_eq!(state["push"]["outcome"], "pushed");
    assert_eq!(state["push"]["dest"], "refs/heads/gsd-auto/p28/r28");
    assert_eq!(state["push"]["sha"], tip.as_str());

    // A second push of an unchanged tip is a no-op: no new LOG line.
    let again = run_push(&fx);
    assert!(again.status.success(), "{}", stderr(&again));
    assert!(
        stdout(&again).contains("already pushed"),
        "{}",
        stdout(&again)
    );
    assert_eq!(fx.log(), log, "an unchanged tip writes nothing");
}
