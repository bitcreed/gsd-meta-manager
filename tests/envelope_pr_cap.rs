// ============================================================================
// SAFE-06 end to end: the pull-request cap is enforced from the `PreToolUse`
// guard, and the settings file that delivers that guard cannot fail silently
// (D-06, D-07, D-19, D-20).
//
// **Every fixture here is offline, agent-free and clock-free** (D-35). No real
// pull request is opened, no network call is made, no agent runs, and no test
// sleeps: the rolling window is exercised by *injecting* timestamps into the
// ledger rather than by waiting for one to elapse. A test that slept for
// twenty-four hours would not be a test.
//
// The pairing D-32 asks for is deliberate throughout: every refusal below has an
// allow beside it. An envelope that refused every pull request would satisfy
// "the fourth is refused" and be useless, so "the third still succeeds" is
// asserted in the same fixture rather than in a different file.
// ============================================================================

use std::path::{Path, PathBuf};

use gsd_meta_manager::envelope::{hooks, ledger, policy};
use tempfile::TempDir;

/// The alias every fixture drives.
const ALIAS: &str = "alpha";

/// The binary path baked into a generated settings file.
///
/// A fixed path rather than `current_exe()`: under `cargo test` that is the test
/// binary, which has no `envelope` subcommand — the same trap plan 19-01
/// recorded and 19-04 hit again.
const BIN: &str = "/opt/gsd-meta-manager";

/// A registry file whose opt-in carries the caps this fixture wants.
///
/// Written as JSON text rather than through `serde_json::to_string` of a
/// `Config`, for the reason `config.rs`'s own migration literal records: a
/// round-trip proves the type agrees with itself, while a literal proves the
/// type agrees with what is actually on disk.
fn registry_with_caps(dir: &Path, cap_24h: u32, cap_run: u32) -> PathBuf {
    let path = dir.join("config.json");
    let body = format!(
        r#"{{
  "version": 2,
  "projects": {{
    "{ALIAS}": {{
      "path": "/nonexistent/project",
      "added": "2026-08-18T00:00:00Z",
      "driver_opt_in": {{
        "opted_in_at": "2026-08-18T00:00:00Z",
        "claude_md_digest": null,
        "pr_cap_per_24h": {cap_24h},
        "pr_cap_per_run": {cap_run}
      }}
    }}
  }},
  "preferences": {{}}
}}"#
    );
    std::fs::write(&path, body).expect("the fixture registry is writable");

    // **Without this the whole file could pass vacuously.** `resolve_policy`
    // degrades an unreadable registry to the tighter DEFAULTS rather than
    // failing — the right behaviour for a guard, and a trap for a fixture: a
    // typo here would silently test the defaults while claiming to test the
    // configured caps.
    let loaded = gsd_meta_manager::config::load_config(&path)
        .expect("the fixture registry parses as a real registry");
    let opt_in = loaded.projects[ALIAS]
        .driver_opt_in
        .as_ref()
        .expect("the fixture registry carries an opt-in");
    assert_eq!(opt_in.pr_cap_per_24h, Some(cap_24h));
    assert_eq!(opt_in.pr_cap_per_run, Some(cap_run));

    path
}

/// One guard answer.
struct Answer {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Answer {
    fn permitted(&self) -> bool {
        self.code == 0
    }

    fn reason(&self) -> String {
        let value: serde_json::Value =
            serde_json::from_str(self.stdout.trim()).unwrap_or(serde_json::Value::Null);
        value["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
}

/// Ask the guard about one shell command.
fn ask(envelope_root: &Path, config: &Path, command: &str) -> Answer {
    let request = serde_json::json!({
        "session_id": "fixture",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = hooks::guard_in(
        envelope_root,
        config,
        ALIAS,
        None,
        request.as_bytes(),
        &mut out,
        &mut err,
    )
    .expect("the guard answers rather than erroring");

    Answer {
        code,
        stdout: String::from_utf8(out).unwrap(),
        stderr: String::from_utf8(err).unwrap(),
    }
}

/// Seed the ledger with `count` attempts stamped `secs_ago` seconds in the past.
///
/// **This is the injected clock.** The window is decided against the timestamp
/// carried by the attempt being recorded, so an aged ledger is indistinguishable
/// from a ledger that aged — without a single second of real elapsed time.
fn seed(envelope_root: &Path, entries: &[(i64, &str)]) {
    let path = ledger::ledger_path_in(envelope_root, ALIAS).expect("a plain alias has a ledger");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    let mut body = String::new();
    for (secs_ago, run_id) in entries {
        let at = (chrono::Utc::now() - chrono::Duration::seconds(*secs_ago))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        body.push_str(&serde_json::to_string(&ledger::LedgerEntry {
            at,
            run_id: run_id.to_string(),
            command: "gh pr create".to_string(),
            platform: "github".to_string(),
        })
        .unwrap());
        body.push('\n');
    }
    std::fs::write(&path, body).unwrap();
}

/// How many lines the ledger holds.
fn ledger_lines(envelope_root: &Path) -> Vec<String> {
    let path = ledger::ledger_path_in(envelope_root, ALIAS).unwrap();
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------------
// 1. The cap boundary, end to end through the guard
// ---------------------------------------------------------------------------

#[test]
fn the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused() {
    let envelope = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // Per-run cap raised out of the way: this fixture is about the rolling
    // window, and a per-run bound firing first would make it prove nothing.
    let config = registry_with_caps(home.path(), 3, 9);

    // Two attempts already inside the window, from earlier runs.
    seed(
        envelope.path(),
        &[(60 * 60, "run-earlier-1"), (23 * 60 * 60, "run-earlier-2")],
    );

    let third = ask(envelope.path(), &config, "gh pr create --title x --body y");
    assert!(
        third.permitted(),
        "the THIRD attempt inside the window must still succeed — an envelope that \
         refused every pull request would pass the refusal assertion below and be \
         useless. stderr: {}",
        third.stderr
    );

    let fourth = ask(envelope.path(), &config, "gh pr create --title x --body y");
    assert_eq!(fourth.code, 2, "stdout: {}", fourth.stdout);
    assert!(
        fourth.reason().contains(policy::REASON_PR_CAP_EXCEEDED),
        "the refusal must carry D-24's taxonomy identifier: {}",
        fourth.reason()
    );
}

#[test]
fn an_attempt_that_has_fallen_out_of_the_window_releases_its_slot() {
    let envelope = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let config = registry_with_caps(home.path(), 1, 9);

    // One second past the boundary. Injected, not waited for.
    seed(envelope.path(), &[(24 * 60 * 60 + 1, "run-yesterday")]);

    let answer = ask(envelope.path(), &config, "gh pr create --title x");
    assert!(
        answer.permitted(),
        "a cap that never releases a slot is not a rolling window: {}",
        answer.stderr
    );
}

#[test]
fn an_attempt_exactly_at_the_boundary_still_counts_against_the_window() {
    let envelope = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let config = registry_with_caps(home.path(), 1, 9);

    seed(envelope.path(), &[(24 * 60 * 60, "run-yesterday")]);

    let answer = ask(envelope.path(), &config, "gh pr create --title x");
    assert_eq!(
        answer.code, 2,
        "the boundary is inclusive — the over-counting direction, chosen to match \
         D-20's recorded bias. stdout: {}",
        answer.stdout
    );
}

// ---------------------------------------------------------------------------
// 2. The per-run cap is a separate bound
// ---------------------------------------------------------------------------

#[test]
fn a_second_attempt_in_one_run_is_refused_while_the_window_still_has_capacity() {
    let envelope = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let config = registry_with_caps(home.path(), 9, 1);

    let first = ask(envelope.path(), &config, "gh pr create --title x");
    assert!(first.permitted(), "{}", first.stderr);

    let second = ask(envelope.path(), &config, "gh pr create --title x");
    assert_eq!(second.code, 2, "stdout: {}", second.stdout);
    assert!(
        second.reason().contains("per-run cap"),
        "waiting does not clear a per-run bound, so the refusal must distinguish the \
         two: {}",
        second.reason()
    );
    assert!(
        second.reason().contains(policy::REASON_PR_CAP_EXCEEDED),
        "{}",
        second.reason()
    );
}

// ---------------------------------------------------------------------------
// 3. Write before permit (D-20)
// ---------------------------------------------------------------------------

#[test]
fn the_refused_attempt_is_on_disk_because_the_ledger_records_before_it_permits() {
    let envelope = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let config = registry_with_caps(home.path(), 1, 9);

    let permitted = ask(envelope.path(), &config, "gh pr create --title first");
    assert!(permitted.permitted(), "{}", permitted.stderr);

    let refused = ask(envelope.path(), &config, "gh pr create --title second");
    assert_eq!(refused.code, 2, "stdout: {}", refused.stdout);

    let lines = ledger_lines(envelope.path());
    assert_eq!(
        lines.len(),
        2,
        "the REFUSED attempt is recorded too: the guard is the only point that \
         observes an attempt at all, so an attempt it saw and did not record is one \
         nothing can ever count. {lines:?}"
    );
    assert!(
        refused.reason().contains("over-count"),
        "and the cost of that ordering is stated in the refusal rather than left to \
         be discovered: {}",
        refused.reason()
    );

    // The paired negative: nothing that is not a creation reaches the ledger.
    let listed = ask(envelope.path(), &config, "gh pr list");
    assert!(listed.permitted());
    assert_eq!(
        ledger_lines(envelope.path()).len(),
        2,
        "a cap that counted reads would refuse a run for looking"
    );
}

// ---------------------------------------------------------------------------
// 4. The settings round trip refuses rather than warns (D-07)
// ---------------------------------------------------------------------------

#[test]
fn a_one_byte_corruption_of_the_settings_file_makes_the_comparison_return_an_error() {
    let envelope = TempDir::new().unwrap();
    let path = hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN))
        .expect("a freshly generated settings file verifies");

    let expected = hooks::settings_value(Path::new(BIN), ALIAS);
    hooks::verify_settings(&path, &expected)
        .expect("the file as written matches the value that was written");

    // One byte. The agent CLI would ignore the result SILENTLY in print mode,
    // which is why this must be an error at generation time and not a warning
    // somebody reads after the run that needed it.
    let mut bytes = std::fs::read(&path).unwrap();
    let brace = bytes
        .iter()
        .position(|byte| *byte == b'{')
        .expect("the rendered JSON opens with a brace");
    bytes[brace] = b'[';
    std::fs::write(&path, &bytes).unwrap();

    let error = hooks::verify_settings(&path, &expected)
        .expect_err("a corrupted settings file must REFUSE the run, not warn about it");
    let text = error.to_string();
    assert!(
        text.contains("SILENTLY") && text.contains(policy::REASON_ENVELOPE_ASSERTION_FAILED),
        "the refusal must say why silence is the danger: {text}"
    );

    // The paired allow: regenerating repairs it, so the check is not a wall.
    hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN))
        .expect("regeneration repairs a corrupted file");
}

#[test]
fn a_settings_value_that_differs_from_the_file_is_a_mismatch_not_a_pass() {
    let envelope = TempDir::new().unwrap();
    let path = hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN)).unwrap();

    // Valid JSON, valid schema, wrong value — the case a mere "does it parse?"
    // check would wave through.
    let mut tampered = hooks::settings_value(Path::new(BIN), ALIAS);
    tampered.hooks.pre_tool_use[0].hooks[0].timeout = 900;
    std::fs::write(&path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();

    let expected = hooks::settings_value(Path::new(BIN), ALIAS);
    let error = hooks::verify_settings(&path, &expected)
        .expect_err("a well-formed file carrying the wrong value must still refuse");
    assert!(error.to_string().contains("do not match"), "{error}");
}

// ---------------------------------------------------------------------------
// 5. The registered timeout (the reproduced hang this phase must not re-create)
// ---------------------------------------------------------------------------

#[test]
fn the_generated_settings_file_registers_the_guard_with_the_named_timeout_constant() {
    let envelope = TempDir::new().unwrap();
    let path = hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN)).unwrap();

    // Parsed as raw JSON rather than back into the struct: the struct would
    // agree with itself by construction, and what has to be true is that the
    // KEYS the agent CLI reads are the keys on disk.
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    let entry = &value["hooks"]["PreToolUse"][0];
    assert_eq!(entry["matcher"], "Bash", "{value}");
    let command = &entry["hooks"][0];
    assert_eq!(command["type"], "command", "{value}");
    assert_eq!(
        command["timeout"], hooks::GUARD_TIMEOUT_SECS,
        "the timeout must be the named constant, not a literal that drifted from it. \
         A PreToolUse hook with NO timeout is the exact configuration that produced \
         the reproduced 180-240 second hang at src/executor/mod.rs:225-239. {value}"
    );
    assert!(
        command["command"]
            .as_str()
            .unwrap()
            .contains("envelope guard"),
        "{value}"
    );
    assert!(
        command["command"].as_str().unwrap().contains(ALIAS),
        "the alias is quoted into the command, so one settings file drives one \
         alias's policy: {value}"
    );
}

// ---------------------------------------------------------------------------
// 6. Second-carrier coverage: the two carriers cannot drift apart
// ---------------------------------------------------------------------------

#[test]
fn every_pattern_the_settings_file_denies_is_also_carried_on_argv() {
    let envelope = TempDir::new().unwrap();
    let path = hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN)).unwrap();

    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let denied: Vec<String> = value["permissions"]["deny"]
        .as_array()
        .expect("the deny list is an array")
        .iter()
        .map(|entry| entry.as_str().unwrap().to_string())
        .collect();

    assert!(!denied.is_empty(), "an empty deny list carries nothing");

    let argv = policy::disallowed_tools();
    for pattern in &denied {
        assert!(
            argv.contains(pattern),
            "`{pattern}` is registered in the settings file but is NOT on the \
             `--disallowedTools` argv list. D-07's rule is that the settings file is \
             never the sole carrier of a control, and two lists that can differ are \
             two lists that will: {argv:?}"
        );
    }
    assert_eq!(
        denied.len(),
        argv.len(),
        "the two carriers are the same list from the same source, so a difference in \
         length is a drift that has already happened"
    );

    // The `.claude/**` entries are not decoration: `--setting-sources project`
    // means the driven repository's own settings file IS loaded by the child,
    // and it is agent-writable.
    assert!(
        denied.iter().any(|p| p.starts_with("Write(.claude/")),
        "{denied:?}"
    );
    assert!(
        denied.iter().any(|p| p.starts_with("Edit(.claude/")),
        "{denied:?}"
    );
}

#[test]
fn the_argv_delivery_renders_the_same_value_the_file_carries() {
    let envelope = TempDir::new().unwrap();
    let path = hooks::write_settings_in(envelope.path(), ALIAS, Path::new(BIN)).unwrap();

    let from_file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let from_argv: serde_json::Value =
        serde_json::from_str(&hooks::settings_json(Path::new(BIN), ALIAS).unwrap()).unwrap();

    assert_eq!(
        from_file, from_argv,
        "the file and the inline `--settings` JSON are the same value from the same \
         function; a second renderer would be a second thing to keep in step"
    );
}

// ---------------------------------------------------------------------------
// The guard is the layer that sees a command before it runs — including the
// ones the cap does not cover.
// ---------------------------------------------------------------------------

#[test]
fn a_hostile_alias_gets_no_settings_file_and_no_ledger() {
    let envelope = TempDir::new().unwrap();
    for hostile in ["../escaped", "a/b", ""] {
        assert!(
            hooks::write_settings_in(envelope.path(), hostile, Path::new(BIN)).is_err(),
            "{hostile:?}"
        );
        assert!(ledger::ledger_path_in(envelope.path(), hostile).is_none());
    }
}
