#![cfg(unix)]
// ============================================================================
// CI-safe proofs about the Phase 29 live acceptance harness (plan 29-07, D-19,
// D-23, D-24, D-25): `scripts/prompt-acceptance.sh`. None of these starts
// claude, systemd or the network: the script's offline `selftest`, its
// pre-spend refusals (consent, pre-registration, ceiling), the committed
// evidence tree's privacy and ledger invariants, and one zero-spend REHEARSAL
// per case through the real `driver new`, `driver check` and `driver launch`
// against fake tools.
//
// The paid cases themselves (`run h1` ... `run o1`) are operator-run by plan
// 29-08 and never part of `cargo test`. The two `#[ignore]` entries below only
// CALL the harness, so even `cargo test -- --ignored` stays behind the consent,
// pre-registration and ledger gates (RESEARCH anti-pattern: no Rust test
// spends directly).
//
// The refusal proofs put FAKE `claude`, `systemctl`, `systemd-run`, `bwrap`,
// `curl` that record a hit and exit 99 first on PATH, so a harness path that
// reached any of them would fail the test instead of spending. The rehearsal
// uses a DIFFERENT set of fakes (pass-through shims and a claude that prints a
// zero-cost result line) and an evidence root away from the committed tree.
// ============================================================================

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const PHASE_DIR: &str = ".planning/phases/29-master-prompt-run-templates";

/// The cases whose refusals are proven below.
const CASES: [&str; 1] = ["h1"];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .is_ok()
}

/// Why the planning tree the harness reads is absent, if it is. `Cargo.toml` excludes
/// `.planning/` from the published crate and `gsd-pr-branch` filters it out, so a
/// packaged or PR-branch checkout has neither the spikes nor the phase directory;
/// those checkouts cannot run these proofs, and say so instead of failing.
fn planning_skip_reason() -> Option<String> {
    for rel in [
        PHASE_DIR,
        ".planning/phases/28-sandboxed-driver-launcher",
        ".planning/spikes/002-launcher-caps-and-tool-frontmatter",
        ".planning/spikes/003-divergent-gap-loop-validation",
    ] {
        if !repo().join(rel).is_dir() {
            return Some(format!(
                "{rel} is not in this checkout (.planning/ excluded)"
            ));
        }
    }
    None
}

/// Why this host cannot run the harness' bash, if it cannot.
fn skip_reason() -> Option<String> {
    if let Some(reason) = planning_skip_reason() {
        return Some(reason);
    }
    for tool in ["bash", "python3", "git"] {
        if !have(tool) {
            // On CI a missing tool must not turn this whole file into a vacuous green.
            assert!(
                std::env::var_os("CI").is_none(),
                "{tool} is not runnable on a CI host; these proofs cannot run"
            );
            return Some(format!("{tool} is not runnable here"));
        }
    }
    None
}

struct Trap {
    dir: TempDir,
}

impl Trap {
    fn new() -> Trap {
        let dir = TempDir::new().expect("tempdir");
        for tool in [
            "claude",
            "systemctl",
            "systemd-run",
            "bwrap",
            "curl",
            "wget",
            "ssh",
        ] {
            let path = dir.path().join(tool);
            fs::write(
                &path,
                format!(
                    "#!/bin/sh\necho \"$0 $*\" >> '{}/HIT'\nexit 99\n",
                    dir.path().display()
                ),
            )
            .expect("write a trap tool");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        }
        Trap { dir }
    }

    fn hits(&self) -> String {
        fs::read_to_string(self.dir.path().join("HIT")).unwrap_or_default()
    }

    fn script(&self, args: &[&str], envs: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut cmd = Command::new("bash");
        cmd.arg(repo().join("scripts/prompt-acceptance.sh"))
            .args(args)
            .env("PATH", path)
            .current_dir(repo())
            .stdin(Stdio::null());
        for (k, v) in envs {
            cmd.env(k, v);
        }
        cmd.output().expect("run the harness")
    }
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A consent file that approves everything up to the phase ceiling.
fn write_consent(dir: &Path, ceiling: &str) {
    fs::write(
        dir.join("consent.txt"),
        format!("approved_ceiling_usd={ceiling}\napproved_cases=h1 h2 h3 h4 o1\n"),
    )
    .expect("write a consent file");
}

#[test]
fn the_harness_selftest_passes_offline() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let out = trap.script(&["selftest"], &[]);
    assert!(
        out.status.success(),
        "selftest failed ({}):\n{}",
        out.status,
        text(&out)
    );
    assert!(text(&out).contains("prompt-acceptance selftest OK"));
    assert_eq!(
        trap.hits(),
        "",
        "the selftest reached claude, systemd or the network"
    );
}

#[test]
fn the_phase_28_harness_still_runs_its_own_selftest_when_executed() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let out = Command::new("bash")
        .arg(repo().join("scripts/launcher-acceptance.sh"))
        .arg("selftest")
        .current_dir(repo())
        .stdin(Stdio::null())
        .output()
        .expect("run the Phase 28 harness");
    assert!(out.status.success(), "{}", text(&out));
    assert!(text(&out).contains("launcher-acceptance selftest OK"));
}

#[test]
fn run_is_refused_with_exit_10_without_consent() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    for case in CASES {
        let trap = Trap::new();
        let acc = TempDir::new().expect("tempdir");
        let out = trap.script(
            &["run", case],
            &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
        );
        assert_eq!(out.status.code(), Some(10), "{case}: {}", text(&out));
        assert!(text(&out).contains("no cost consent"), "{case}");
        assert_eq!(trap.hits(), "", "{case}");
    }
}

#[test]
fn run_is_refused_when_the_consent_does_not_cover_the_case() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    fs::write(
        acc.path().join("consent.txt"),
        "approved_ceiling_usd=25\napproved_cases=h2 h3\n",
    )
    .expect("write consent");
    let out = trap.script(
        &["run", "h1"],
        &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
    );
    assert_eq!(out.status.code(), Some(10), "{}", text(&out));
    assert!(text(&out).contains("does not approve case h1"));
    assert_eq!(trap.hits(), "");
}

#[test]
fn run_is_refused_with_exit_9_without_a_preregistered_rubric() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    for case in CASES {
        let trap = Trap::new();
        let acc = TempDir::new().expect("tempdir");
        write_consent(acc.path(), "25");
        let out = trap.script(
            &["run", case],
            &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
        );
        assert_eq!(out.status.code(), Some(9), "{case}: {}", text(&out));
        assert!(text(&out).contains("not pre-registered"), "{case}");
        assert_eq!(trap.hits(), "", "{case}");
    }
}

#[test]
fn a_campaign_ceiling_above_twenty_five_is_refused_before_anything_else() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    let out = trap.script(
        &["run", "h1"],
        &[
            ("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8")),
            ("CAMPAIGN_CEILING_USD", "26"),
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert_eq!(trap.hits(), "");
}

#[test]
fn a_launch_that_could_pass_the_ceiling_is_refused_with_exit_8() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    for case in CASES {
        // ledger 24.8 + h1's 0.40 budget > the approved 25
        let trap = Trap::new();
        let acc = TempDir::new().expect("tempdir");
        write_consent(acc.path(), "25");
        fs::write(
            acc.path().join("campaign.txt"),
            "2026-10-20T00:00:00Z o1 1 24.8000 killed-full-budget\n",
        )
        .expect("write a ledger");
        let out = trap.script(
            &["run", case],
            &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
        );
        assert_eq!(out.status.code(), Some(8), "{case}: {}", text(&out));
        assert_eq!(trap.hits(), "", "{case}");
    }
    // a lower approved ceiling binds even when the ledger is far under 25
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    write_consent(acc.path(), "1");
    fs::write(
        acc.path().join("campaign.txt"),
        "2026-10-20T00:00:00Z h1 1 0.8000 killed-full-budget\n",
    )
    .expect("write a ledger");
    let out = trap.script(
        &["run", "h1"],
        &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
    );
    assert_eq!(out.status.code(), Some(8), "{}", text(&out));
    assert_eq!(trap.hits(), "");
}

#[test]
fn existing_evidence_is_never_spent_over_without_rerun() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    for case in CASES {
        let trap = Trap::new();
        let acc = TempDir::new().expect("tempdir");
        fs::create_dir_all(acc.path().join(case)).expect("mkdir");
        fs::write(acc.path().join(case).join("verdict.txt"), "RESULT PASS\n").expect("write");
        let out = trap.script(
            &["run", case],
            &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
        );
        assert_eq!(out.status.code(), Some(2), "{case}: {}", text(&out));
        assert!(text(&out).contains("RERUN=1"), "{case}");
        assert_eq!(trap.hits(), "", "{case}");
    }
}

#[test]
fn a_rehearsal_cannot_point_at_the_committed_evidence() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let fakes = TempDir::new().expect("tempdir");
    let out = trap.script(
        &["run", "h1"],
        &[("ACC_PATH_PREPEND", fakes.path().to_str().expect("utf8"))],
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("needs ACCEPTANCE_DIR away from the committed evidence"));
    assert_eq!(trap.hits(), "");
}

#[test]
fn an_unknown_case_is_a_usage_error() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let out = trap.script(&["run", "zz"], &[]);
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert_eq!(trap.hits(), "");
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.push(path.clone());
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// T-29-24: the committed evidence is derived files only. An absent tree is fine
/// (nothing has been measured yet).
#[test]
fn the_committed_acceptance_tree_holds_no_stream_config_or_token() {
    let root = repo().join(PHASE_DIR).join("acceptance");
    let mut all = Vec::new();
    walk(&root, &mut all);
    for path in &all {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert!(
            !name.ends_with(".jsonl") && name != "claude-config",
            "{} must not be committed evidence",
            path.display()
        );
        if path.is_file() {
            let body = fs::read_to_string(path).unwrap_or_default();
            for needle in ["sk-ant", "oat01", "accessToken", "refreshToken"] {
                assert!(
                    !body.contains(needle),
                    "{} holds the secret-shaped text {needle:?}",
                    path.display()
                );
            }
        }
    }
}

/// T-29-25: the committed ledger never sums past the $25 phase ceiling. An absent
/// ledger is zero.
#[test]
fn the_committed_campaign_ledger_stays_within_the_phase_ceiling() {
    let ledger = repo().join(PHASE_DIR).join("acceptance/campaign.txt");
    let Ok(body) = fs::read_to_string(&ledger) else {
        return;
    };
    let mut sum = 0.0_f64;
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        assert!(fields.len() >= 5, "malformed ledger line: {line}");
        sum += fields[3]
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("USD column is not a number: {line}"));
    }
    assert!(
        sum <= 25.0,
        "the campaign ledger sums to {sum}, past the $25 ceiling"
    );
}

// ----------------------------------------------------------------------------
// The zero-spend rehearsal: every case's `run` path through the REAL binary's
// `driver new`, `driver check` and `driver launch`, against fake tools.
// ----------------------------------------------------------------------------

/// The fake tools of a rehearsal. The shims are pass-through (modelled on
/// `SYSTEMD_RUN_SHIM` / `BWRAP_SHIM` in tests/launcher_support/mod.rs); the fake
/// `claude` answers `--version`, otherwise appends its (truncated) argv to the hit
/// log and prints a success `result` line that costs nothing. The hit log path is
/// baked into each script, so the launcher's environment carries nothing extra.
fn write_rehearsal_fakes(dir: &Path) -> PathBuf {
    let hits = dir.join("hits.log");
    let h = hits.display();
    let log_argv = format!(
        "{{ printf '%s' \"$TOOL\"; for a in \"$@\"; do printf ' %s' \"$(printf '%s' \"$a\" | head -c 120 | tr '\\n' ' ')\"; done; printf '\\n'; }} >> '{h}'\n"
    );
    let systemd_run = format!(
        "#!/bin/sh\nSHIMS=$(dirname \"$0\")\nTOOL=systemd-run\n{log_argv}unit=\"\"\nfor a in \"$@\"; do case \"$a\" in --unit=*) unit=${{a#--unit=}};; esac; done\n[ -n \"$unit\" ] && echo $$ > \"$SHIMS/$unit.pid\"\nwhile [ $# -gt 0 ]; do if [ \"$1\" = \"--\" ]; then shift; break; fi; shift; done\nexec \"$@\"\n"
    );
    let bwrap = format!(
        "#!/bin/sh\nTOOL=bwrap\n{log_argv}while [ $# -gt 0 ]; do if [ \"$1\" = \"--\" ]; then shift; break; fi; shift; done\nexec \"$@\"\n"
    );
    let systemctl = format!(
        "#!/bin/sh\nSHIMS=$(dirname \"$0\")\nTOOL=systemctl\n{log_argv}[ \"$1\" = \"--user\" ] && shift\nverb=$1\nunit=${{2%.scope}}\npid=\"\"\n[ -n \"$unit\" ] && pid=$(cat \"$SHIMS/$unit.pid\" 2>/dev/null)\ncase \"$verb\" in\n  is-system-running) echo running; exit 0;;\n  is-active) if [ -n \"$pid\" ] && kill -0 \"$pid\" 2>/dev/null; then echo active; exit 0; fi; echo inactive; exit 3;;\n  stop) if [ -n \"$pid\" ]; then kill -s TERM -- \"-$pid\" 2>/dev/null; i=0; while kill -0 \"$pid\" 2>/dev/null && [ \"$i\" -lt 150 ]; do sleep 0.1; i=$((i + 1)); done; fi; exit 0;;\nesac\nexit 0\n"
    );
    let claude = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"2.1.304 (Claude Code)\"; exit 0; fi\nTOOL=claude\n{log_argv}printf '%s\\n' '{{\"type\":\"result\",\"subtype\":\"success\",\"total_cost_usd\":0}}'\nexit 0\n"
    );
    for (name, body) in [
        ("systemd-run", systemd_run),
        ("bwrap", bwrap),
        ("systemctl", systemctl),
        ("claude", claude),
    ] {
        let path = dir.join(name);
        fs::write(&path, body).expect("write a rehearsal fake");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    hits
}

/// A directory outside `/tmp` (the harness refuses a scratch root there) and outside
/// any git work tree, or why there is none.
fn rehearsal_base() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME is not set")?;
    let base = home.join(".local/state/gsdmm-rh");
    fs::create_dir_all(&base).map_err(|e| format!("cannot create {}: {e}", base.display()))?;
    let inside_git = Command::new("git")
        .arg("-C")
        .arg(&base)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if inside_git {
        return Err(format!("{} is inside a git work tree", base.display()));
    }
    Ok(base)
}

/// Run one rehearsal and hand (harness output, hit log, scratch evidence root) to `check`
/// while the scratch tree still exists; it is removed afterwards.
fn rehearse(case: &str, check: impl FnOnce(&Output, &str, &Path)) {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    for tool in ["flock", "timeout"] {
        if !have(tool) {
            println!("SKIP: {tool} is not runnable here");
            return;
        }
    }
    let base = match rehearsal_base() {
        Ok(base) => base,
        Err(reason) => {
            println!("SKIP: no usable rehearsal scratch root: {reason}");
            return;
        }
    };
    let work = TempDir::new_in(&base).expect("rehearsal scratch");
    let runtime = TempDir::new().expect("runtime dir");
    fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).expect("chmod");
    let fakes = work.path().join("fakes");
    fs::create_dir_all(&fakes).expect("fakes dir");
    let hits = write_rehearsal_fakes(&fakes);
    // a dummy stored token under a scratch XDG_CONFIG_HOME (never the real config dir)
    let config_home = work.path().join("cfg");
    let token = config_home.join("gsd-meta-manager/driver-oauth-token");
    fs::create_dir_all(token.parent().expect("token dir")).expect("token dir");
    fs::write(&token, "tok-rehearsal-0123456789abcdef").expect("token");
    fs::set_permissions(&token, fs::Permissions::from_mode(0o600)).expect("chmod token");
    let acc = work.path().join("acc");
    let committed = repo().join(PHASE_DIR).join("acceptance");
    let before = listing(&committed);
    let out = Command::new("bash")
        .arg(repo().join("scripts/prompt-acceptance.sh"))
        .args(["run", case])
        .env("ACC_PATH_PREPEND", &fakes)
        .env("ACCEPTANCE_DIR", &acc)
        .env("SCRATCH_ROOT", work.path().join("scratch"))
        .env("GSDMM_BIN", env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .env("XDG_CONFIG_HOME", &config_home)
        .env("XDG_DATA_HOME", work.path().join("data"))
        .env("XDG_STATE_HOME", work.path().join("state"))
        .env("XDG_RUNTIME_DIR", runtime.path())
        .current_dir(repo())
        .stdin(Stdio::null())
        .output()
        .expect("run the rehearsal");
    assert_eq!(
        before,
        listing(&committed),
        "a rehearsal wrote under the committed acceptance tree"
    );
    let hit_log = fs::read_to_string(&hits).unwrap_or_default();
    check(&out, &hit_log, &acc);
}

/// A sorted listing of every path under `dir` (empty when absent).
fn listing(dir: &Path) -> Vec<String> {
    let mut all = Vec::new();
    walk(dir, &mut all);
    let mut names: Vec<String> = all.iter().map(|p| p.display().to_string()).collect();
    names.sort();
    names
}

/// What every rehearsal must show, whatever the case.
fn assert_rehearsal_reached_the_fake_claude(case: &str, out: &Output, hit_log: &str, acc: &Path) {
    let all = text(out);
    println!("---- rehearsal {case}: console ----\n{all}");
    let tool_hits = |tool: &str| hit_log.lines().filter(|l| l.starts_with(tool)).count();
    println!(
        "---- rehearsal {case}: hit log ({} claude, {} systemd-run, {} bwrap, {} systemctl) ----",
        tool_hits("claude "),
        tool_hits("systemd-run "),
        tool_hits("bwrap "),
        tool_hits("systemctl ")
    );
    for line in hit_log.lines().filter(|l| l.starts_with("claude ")) {
        println!("{}", line.chars().take(200).collect::<String>());
    }
    assert_eq!(out.status.code(), Some(0), "{case}: {all}");
    assert!(
        all.contains("REHEARSAL: consent would refuse (exit 10)"),
        "{case}: the consent gate was not reported\n{all}"
    );
    assert!(
        all.contains("REHEARSAL: prereg would refuse (exit 9)"),
        "{case}: the prereg gate was not reported\n{all}"
    );
    assert!(
        hit_log.lines().any(|l| l.starts_with("claude ")),
        "{case}: the fake claude was never reached\n{hit_log}"
    );
    assert!(
        acc.join(case).join("check.txt").is_file(),
        "{case}: no driver check evidence"
    );
    let check = fs::read_to_string(acc.join(case).join("check.txt")).unwrap_or_default();
    assert!(
        check.starts_with("driver check rc=0"),
        "{case}: driver check did not exit 0 on the filled template:\n{check}"
    );
    let env = fs::read_to_string(acc.join(case).join("run-env.txt")).unwrap_or_default();
    assert!(env.contains("REHEARSAL=1"), "{case}: {env}");
}

#[test]
fn rehearsal_h1_reaches_the_fake_claude_through_driver_new_check_and_launch() {
    rehearse("h1", |out, hit_log, acc| {
        assert_rehearsal_reached_the_fake_claude("h1", out, hit_log, acc);
        let observed = fs::read_to_string(acc.join("h1/observed.txt")).unwrap_or_default();
        assert!(observed.contains("run_id=2"), "{observed}");
        assert!(
            !observed.contains("--prompt-file"),
            "h1 launches by run id alone:\n{observed}"
        );
    });
}

// ----------------------------------------------------------------------------
// D-19 opt-in live layer. These only call the gated harness.
// ----------------------------------------------------------------------------

/// Run `scripts/prompt-acceptance.sh run <case>` for real. Reached only with
/// `cargo test -- --ignored`, and then still behind the consent, pre-registration
/// and ledger gates of the harness (exit 10, 9, 8) - this test never spends by itself.
fn run_live(case: &str) {
    let out = Command::new("bash")
        .arg(repo().join("scripts/prompt-acceptance.sh"))
        .args(["run", case])
        .current_dir(repo())
        .stdin(Stdio::null())
        .output()
        .expect("run the harness");
    assert_eq!(out.status.code(), Some(0), "{case}: {}", text(&out));
}

#[test]
#[ignore = "paid live run: needs plan 29-08's consent.txt; spends real model budget"]
fn live_h1_crash_reconcile() {
    run_live("h1");
}
