#![cfg(unix)]
// ============================================================================
// CI-safe proofs about the Phase 28 live acceptance harness (plan 28-09):
// `scripts/launcher-acceptance.sh`. None of these starts claude, systemd or the
// network: the script's offline `selftest`, its pre-spend refusals, and the
// committed evidence tree's privacy and ledger invariants. The paid cases
// themselves (`run l1` ... `run d33`) are operator-run and never part of
// `cargo test`.
//
// Every test puts FAKE `claude`, `systemctl`, `systemd-run`, `bwrap`, `curl`
// that record a hit and exit 99 first on PATH, so a harness path that reached any
// of them would fail the test instead of spending.
// ============================================================================

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

const PHASE_DIR: &str = ".planning/phases/28-sandboxed-driver-launcher";

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

/// Why this host cannot run the harness' bash, if it cannot.
fn skip_reason() -> Option<String> {
    for tool in ["bash", "python3", "git"] {
        if !have(tool) {
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
        cmd.arg(repo().join("scripts/launcher-acceptance.sh"))
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
    assert!(text(&out).contains("launcher-acceptance selftest OK"));
    assert_eq!(
        trap.hits(),
        "",
        "the selftest reached claude, systemd or the network"
    );
}

#[test]
fn run_d33_is_refused_with_exit_9_until_the_rubric_is_preregistered() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    let out = trap.script(
        &["run", "d33"],
        &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
    );
    assert_eq!(out.status.code(), Some(9), "{}", text(&out));
    assert!(text(&out).contains("not pre-registered"));
    assert_eq!(trap.hits(), "");
}

#[test]
fn a_campaign_ceiling_above_twenty_is_refused_before_anything_else() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    let out = trap.script(
        &["run", "l1"],
        &[
            ("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8")),
            ("CAMPAIGN_CEILING_USD", "25"),
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
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    fs::write(
        acc.path().join("campaign.txt"),
        "2026-10-19T00:00:00Z d33 1 19.9000 budget-charged\n",
    )
    .expect("write a ledger");
    let out = trap.script(
        &["run", "l1"],
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
    let trap = Trap::new();
    let acc = TempDir::new().expect("tempdir");
    fs::create_dir_all(acc.path().join("l2")).expect("mkdir");
    fs::write(acc.path().join("l2/verdict.txt"), "RESULT PASS\n").expect("write");
    let out = trap.script(
        &["run", "l2"],
        &[("ACCEPTANCE_DIR", acc.path().to_str().expect("utf8"))],
    );
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("RERUN=1"));
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

/// T-28-27: the committed evidence is derived files only.
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

/// T-28-25: the committed ledger never sums past the $20 phase ceiling.
#[test]
fn the_committed_campaign_ledger_stays_within_the_phase_ceiling() {
    let ledger = repo().join(PHASE_DIR).join("acceptance/campaign.txt");
    let body = fs::read_to_string(&ledger).expect("acceptance/campaign.txt exists");
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
        sum <= 20.0,
        "the campaign ledger sums to {sum}, past the $20 ceiling"
    );
}
