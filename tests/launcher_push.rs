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

// ---------------------------------------------------------------- refusals (28-08 task 2)

/// A `gh` that logs its argv, one line per call, to `$FAKE_GH_LOG` and prints a
/// pull-request URL, as the real one does on success.
const GH_SHIM: &str = r#"#!/bin/sh
{
  for a in "$@"; do printf '%s\037' "$a"; done
  printf '\n'
} >> "$FAKE_GH_LOG"
echo "https://github.com/o/r/pull/1"
"#;

/// The shim's log: one line per `gh` call, arguments separated by `\x1f`.
fn gh_log(fx: &Fixture) -> PathBuf {
    fx.scratch.path().join("gh.log")
}

fn gh_calls(fx: &Fixture) -> Vec<Vec<String>> {
    fs::read_to_string(gh_log(fx))
        .unwrap_or_default()
        .lines()
        .map(|line| {
            line.split('\u{1f}')
                .filter(|a| !a.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

/// Put the `gh` shim on the fixture's `PATH`.
fn install_gh(fx: &Fixture) {
    launcher_support::write_executable(&fx.shims(), "gh", GH_SHIM);
}

/// Give a command the environment the push tests share: where `gh` logs, and the
/// global git configuration (when the test wrote one).
fn tuned(mut cmd: Command, fx: &Fixture) -> Command {
    cmd.env("FAKE_GH_LOG", gh_log(fx));
    let config = fx.scratch.path().join("gitconfig");
    if config.exists() {
        cmd.env("GIT_CONFIG_GLOBAL", config);
    }
    cmd
}

/// `preferences.driver.open_pr = true` in the fixture's registry.
fn enable_open_pr(fx: &Fixture) {
    let mut registry: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.registry()).expect("registry"))
            .expect("registry parses");
    registry["preferences"] = serde_json::json!({"driver": {"open_pr": true}});
    fs::write(fx.registry(), registry.to_string()).expect("registry");
}

/// Make `https://github.com/o/r.git` resolve to `remote` for every git the launcher
/// starts, so the pinned URL is GitHub-shaped (for `github_repo`) while the push
/// itself lands in the local bare repository.
fn alias_github_to(fx: &Fixture, remote: &Remote) {
    fs::write(
        fx.scratch.path().join("gitconfig"),
        format!(
            "[url \"{}\"]\n\tinsteadOf = https://github.com/o/r.git\n",
            remote.url()
        ),
    )
    .expect("gitconfig");
}

fn launch_tuned(fx: &Fixture, extra: &[&str]) -> std::process::Output {
    tuned(fx.launch_command(RUN_ID, extra), fx)
        .output()
        .expect("spawn driver launch")
}

fn push_tuned(fx: &Fixture) -> std::process::Output {
    tuned(push_command(fx), fx)
        .output()
        .expect("spawn driver push")
}

/// The first line of a pushed key, which the scan must never print.
const KEY_BODY: &str = "MIIBOgIBAAJBAKj34GkxFhD9";

/// A driver that commits a file holding a private key, then ends the run.
fn commit_a_secret_scenario() -> String {
    // The header is assembled here so this file does not itself hold a key shape.
    let begin = format!("-----BEGIN RSA {}-----", "PRIVATE KEY");
    let end = format!("-----END RSA {}-----", "PRIVATE KEY");
    format!(
        r#"
driverctl log start --mode headless --loop-mode loop
printf '%s\n' '{begin}' '{KEY_BODY}' 'abcdefgh' '{end}' > id_rsa
git add id_rsa
git commit --quiet -m "add a key"
driverctl log end --reason all-done
printf '%s\n' '{{"type":"result","subtype":"success"}}'
"#
    )
}

#[test]
fn push_refuses_a_live_run() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    // The driver commits, announces itself and waits: the launcher is live.
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
echo "driver work" > feature.txt
git add feature.txt
git commit --quiet -m "driver work"
touch "$FAKE_CLAUDE_DIR/started"
while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
driverctl log end --reason all-done
printf '%s\n' '{"type":"result","subtype":"success"}'
"#,
    );
    let launcher = fx.spawn_launch(&[]);
    assert!(
        launcher_support::wait_until(std::time::Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );

    let refused = run_push(&fx);
    assert_eq!(refused.status.code(), Some(1), "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("live"),
        "the refusal names the live run: {}",
        stderr(&refused)
    );
    assert_eq!(
        origin.refs(),
        "",
        "nothing was pushed while the run was live"
    );

    fx.release();
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn push_refuses_a_crashed_run() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    // The driver commits and dies without ending the run.
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
echo "driver work" > feature.txt
git add feature.txt
git commit --quiet -m "driver work"
exit 3
"#,
    );
    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    assert!(
        stdout(&launched).contains("ended status=crashed"),
        "{}",
        stdout(&launched)
    );
    assert_eq!(origin.refs(), "", "a crashed launch pushes nothing");

    let refused = run_push(&fx);
    assert_eq!(refused.status.code(), Some(1), "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("run_not_finished"),
        "{}",
        stderr(&refused)
    );
    assert_eq!(origin.refs(), "", "a crashed run is never pushed");
}

#[test]
fn secret_in_the_tip_parks_and_pushes_nothing() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    fx.write_scenario(&commit_a_secret_scenario());

    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    assert_eq!(origin.refs(), "", "a tip holding a key is never pushed");

    let state = fx.state_json();
    assert_eq!(state["status"], "parked");
    assert_eq!(state["status_reason"], "push-refused:secret_detected");
    let log = fx.log();
    assert_eq!(
        lines_of(&log, "push-refused").len(),
        1,
        "one refusal line: {log}"
    );
    assert!(
        lines_of(&log, "push-refused")[0].contains("reason=secret_detected"),
        "{log}"
    );
    assert!(lines_of(&log, "push").is_empty(), "no push is recorded");

    // A manual push of the parked run is refused the same way.
    let again = run_push(&fx);
    assert_eq!(again.status.code(), Some(1), "{}", stdout(&again));
    assert_eq!(origin.refs(), "");

    // The key text is in no output the launcher produced.
    for (name, text) in [
        ("launch stdout", stdout(&launched)),
        ("launch stderr", stderr(&launched)),
        ("push stdout", stdout(&again)),
        ("push stderr", stderr(&again)),
        ("LOG.md", fx.log()),
        ("state.json", fx.run_file("state.json")),
    ] {
        assert!(!text.contains(KEY_BODY), "{name} holds the key text");
    }
}

#[test]
fn push_uses_the_pinned_remote_not_the_rewritten_checkout_config() {
    let fx = Fixture::new();
    let pinned = Remote::new(&fx, "origin.git");
    let hostile = Remote::new(&fx, "hostile.git");
    init_project(&fx, &pinned.url());
    // The driver redirects the checkout's origin to a second repository, then commits.
    fx.write_scenario(&format!(
        r#"
driverctl log start --mode headless --loop-mode loop
git config remote.origin.url '{}'
echo "driver work" > feature.txt
git add feature.txt
git commit --quiet -m "driver work"
driverctl log end --reason all-done
printf '%s\n' '{{"type":"result","subtype":"success"}}'
"#,
        hostile.url()
    ));

    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    let tip = git(fx.project(), &["rev-parse", "work"]);
    assert_eq!(
        pinned.branch("gsd-auto/p28/r28").as_deref(),
        Some(tip.as_str()),
        "the push went to the URL pinned before the driver existed"
    );
    assert_eq!(hostile.refs(), "", "the rewritten remote received nothing");
}

#[test]
fn launcher_pushes_once_at_run_end() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    fx.write_scenario(COMMIT_AND_END);

    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    let tip = git(fx.project(), &["rev-parse", "work"]);
    assert_eq!(
        origin.branch("gsd-auto/p28/r28").as_deref(),
        Some(tip.as_str()),
        "the launcher pushed without being asked"
    );

    let log = fx.log();
    let lines: Vec<&str> = log.lines().collect();
    let position = |event: &str| {
        lines
            .iter()
            .position(|l| l.contains(&format!("RUN | {event} |")))
    };
    let (end, push) = (position("launch-end"), position("push"));
    assert!(end.is_some() && push.is_some(), "{log}");
    assert!(
        end < push,
        "the push comes after the launch-end line, once the final status is on record: {log}"
    );
    assert_eq!(lines_of(&log, "push").len(), 1, "{log}");
    assert_eq!(fx.state_json()["push"]["outcome"], "pushed");
}

#[test]
fn a_run_that_added_no_commits_pushes_nothing() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    fx.write_scenario(launcher_support::WELL_MANNERED_SCENARIO);

    let launched = fx.launch(&[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    assert_eq!(origin.refs(), "");
    assert!(lines_of(&fx.log(), "push").is_empty());
}

#[test]
fn no_pr_without_open_pr() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, "https://github.com/o/r.git");
    alias_github_to(&fx, &origin);
    install_gh(&fx);
    fx.write_scenario(COMMIT_AND_END);

    let launched = launch_tuned(&fx, &[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    let tip = git(fx.project(), &["rev-parse", "work"]);
    assert_eq!(
        origin.branch("gsd-auto/p28/r28").as_deref(),
        Some(tip.as_str()),
        "the work was pushed"
    );
    assert!(
        !gh_log(&fx).exists(),
        "gh never started: {:?}",
        gh_calls(&fx)
    );
    assert!(lines_of(&fx.log(), "pr").is_empty());
}

#[test]
fn open_pr_opens_one_pr_then_the_cap_refuses() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, "https://github.com/o/r.git");
    alias_github_to(&fx, &origin);
    install_gh(&fx);
    enable_open_pr(&fx);
    fx.write_scenario(COMMIT_AND_END);

    let launched = launch_tuned(&fx, &[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    let calls = gh_calls(&fx);
    assert_eq!(calls.len(), 1, "one pull request: {calls:?}");
    let call = &calls[0];
    assert_eq!(&call[..2], ["pr", "create"], "{call:?}");
    let flag = |name: &str| {
        call.iter()
            .position(|a| a == name)
            .and_then(|i| call.get(i + 1))
            .cloned()
    };
    assert_eq!(flag("--repo").as_deref(), Some("o/r"));
    assert_eq!(flag("--head").as_deref(), Some("gsd-auto/p28/r28"));
    assert_eq!(flag("--base").as_deref(), Some("work"));
    assert!(
        flag("--title").is_some_and(|t| t.contains("r28")),
        "{call:?}"
    );
    let state = fx.state_json();
    assert_eq!(state["push"]["pr"], "https://github.com/o/r/pull/1");
    assert_eq!(lines_of(&fx.log(), "pr").len(), 1, "{}", fx.log());

    // A second pushed tip of the same run: the per-run cap (1) refuses the second PR.
    fs::write(fx.project().join("more.txt"), "more\n").expect("more.txt");
    git(fx.project(), &["add", "more.txt"]);
    git(fx.project(), &["commit", "--quiet", "-m", "more work"]);
    let second = push_tuned(&fx);
    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(gh_calls(&fx).len(), 1, "no second gh call");
    let log = fx.log();
    let refusals = lines_of(&log, "pr-refused");
    assert_eq!(refusals.len(), 1, "{log}");
    assert!(refusals[0].contains("reason=pr_cap_exceeded"), "{log}");
    let new_tip = git(fx.project(), &["rev-parse", "work"]);
    assert_eq!(
        origin.branch("gsd-auto/p28/r28").as_deref(),
        Some(new_tip.as_str()),
        "the second tip itself was pushed (a fast-forward of the same branch)"
    );
}

#[test]
fn non_github_remote_skips_the_pr() {
    let fx = Fixture::new();
    let origin = Remote::new(&fx, "origin.git");
    init_project(&fx, &origin.url());
    install_gh(&fx);
    enable_open_pr(&fx);
    fx.write_scenario(COMMIT_AND_END);

    let launched = launch_tuned(&fx, &[]);
    assert!(launched.status.success(), "{}", stderr(&launched));
    assert!(origin.branch("gsd-auto/p28/r28").is_some(), "pushed");
    let log = fx.log();
    assert_eq!(lines_of(&log, "pr-skipped").len(), 1, "{log}");
    assert!(!gh_log(&fx).exists(), "gh never started");
}
