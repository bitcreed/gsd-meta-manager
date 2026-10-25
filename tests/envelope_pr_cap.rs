// ============================================================================
// SAFE-06 end to end: the pull-request cap the push step enforces through the
// ledger (`envelope::ledger::record_and_check_in`, the call
// `launcher/push.rs::open_pull_request` makes before it starts `gh`), with the
// window, boundary, per-run and record-before-permit behaviour pinned (D-06,
// D-19, D-20).
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
//
// The cap is reached only through `ledger::record_and_check_in` with an
// `EnvelopePolicy` literal. Nothing here goes through the retired `PreToolUse`
// guard or its settings file.
// ============================================================================

use std::path::Path;

use gsd_meta_manager::envelope::{ledger, policy};
use tempfile::TempDir;

/// The alias every fixture drives.
const ALIAS: &str = "alpha";

/// The ledger's own source, for the production-half slice pin at the foot of the
/// file.
const LEDGER_SOURCE: &str = include_str!("../src/envelope/ledger.rs");

/// The policy a fixture wants: only the two caps vary.
///
/// Built as a literal rather than resolved from a registry file, because the caps
/// are what the ledger is handed. The namespace is the real default so the
/// fixture does not invent a shape.
fn policy_with_caps(cap_24h: u32, cap_run: u32) -> policy::EnvelopePolicy {
    policy::EnvelopePolicy {
        namespace: policy::default_namespace(ALIAS),
        pr_cap_per_24h: cap_24h,
        pr_cap_per_run: cap_run,
        credential: None,
    }
}

/// Record one `gh pr create` attempt of `run_id` through `record_and_check_in` and
/// return the cap's verdict.
fn attempt(
    envelope_root: &Path,
    run_id: &str,
    caps: &policy::EnvelopePolicy,
) -> ledger::CapVerdict {
    let entry = ledger::LedgerEntry {
        at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        run_id: run_id.into(),
        command: "gh pr create".into(),
        platform: "github".into(),
    };
    ledger::record_and_check_in(envelope_root, ALIAS, &entry, caps)
        .expect("a plain alias in a usable root has a usable ledger")
}

/// Whether the cap let the attempt through.
fn permitted(verdict: &ledger::CapVerdict) -> bool {
    matches!(verdict, ledger::CapVerdict::Permit { .. })
}

/// The reason a refusal carries, or `None` for a permit.
fn refusal_reason(verdict: &ledger::CapVerdict) -> Option<&'static str> {
    match verdict {
        ledger::CapVerdict::Refuse { reason, .. } => Some(reason.as_str()),
        ledger::CapVerdict::Permit { .. } => None,
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
        body.push_str(
            &serde_json::to_string(&ledger::LedgerEntry {
                at,
                run_id: run_id.to_string(),
                command: "gh pr create".to_string(),
                platform: "github".to_string(),
            })
            .unwrap(),
        );
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
// 1. The cap boundary, end to end through the ledger
// ---------------------------------------------------------------------------

#[test]
fn the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused() {
    let envelope = TempDir::new().unwrap();
    // Per-run cap raised out of the way: this fixture is about the rolling
    // window, and a per-run bound firing first would make it prove nothing.
    let caps = policy_with_caps(3, 9);

    // Two attempts already inside the window, from earlier runs.
    seed(
        envelope.path(),
        &[(60 * 60, "run-earlier-1"), (23 * 60 * 60, "run-earlier-2")],
    );

    let third = attempt(envelope.path(), "run-now", &caps);
    assert!(
        permitted(&third),
        "the THIRD attempt inside the window must still succeed — an envelope that \
         refused every pull request would pass the refusal assertion below and be \
         useless. {third:?}"
    );

    let fourth = attempt(envelope.path(), "run-now", &caps);
    assert_eq!(
        refusal_reason(&fourth),
        Some(policy::REASON_PR_CAP_EXCEEDED),
        "the refusal must carry D-24's taxonomy identifier: {fourth:?}"
    );
}

#[test]
fn an_attempt_that_has_fallen_out_of_the_window_releases_its_slot() {
    let envelope = TempDir::new().unwrap();
    let caps = policy_with_caps(1, 9);

    // One second past the boundary. Injected, not waited for.
    seed(envelope.path(), &[(24 * 60 * 60 + 1, "run-yesterday")]);

    let verdict = attempt(envelope.path(), "run-now", &caps);
    assert!(
        permitted(&verdict),
        "a cap that never releases a slot is not a rolling window: {verdict:?}"
    );
}

#[test]
fn an_attempt_exactly_at_the_boundary_still_counts_against_the_window() {
    let envelope = TempDir::new().unwrap();
    let caps = policy_with_caps(1, 9);

    seed(envelope.path(), &[(24 * 60 * 60, "run-yesterday")]);

    let verdict = attempt(envelope.path(), "run-now", &caps);
    assert!(
        matches!(verdict, ledger::CapVerdict::Refuse { .. }),
        "the boundary is inclusive — the over-counting direction, chosen to match \
         D-20's recorded bias. {verdict:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. The per-run cap is a separate bound
// ---------------------------------------------------------------------------

#[test]
fn a_second_attempt_in_one_run_is_refused_while_the_window_still_has_capacity() {
    let envelope = TempDir::new().unwrap();
    let caps = policy_with_caps(9, 1);

    let first = attempt(envelope.path(), "run-now", &caps);
    assert!(permitted(&first), "{first:?}");

    let second = attempt(envelope.path(), "run-now", &caps);
    assert_eq!(
        refusal_reason(&second),
        Some(policy::REASON_PR_CAP_EXCEEDED),
        "{second:?}"
    );
    let detail = second.refusal_detail().expect("a refusal has a detail");
    assert!(
        detail.contains("per-run cap"),
        "waiting does not clear a per-run bound, so the refusal must distinguish the \
         two: {detail}"
    );
}

// ---------------------------------------------------------------------------
// 3. Write before permit (D-20)
// ---------------------------------------------------------------------------

#[test]
fn the_refused_attempt_is_on_disk_because_the_ledger_records_before_it_permits() {
    let envelope = TempDir::new().unwrap();
    let caps = policy_with_caps(1, 9);

    let permitted_first = attempt(envelope.path(), "run-first", &caps);
    assert!(permitted(&permitted_first), "{permitted_first:?}");

    let refused = attempt(envelope.path(), "run-second", &caps);
    assert!(
        matches!(refused, ledger::CapVerdict::Refuse { .. }),
        "{refused:?}"
    );

    let lines = ledger_lines(envelope.path());
    assert_eq!(
        lines.len(),
        2,
        "the REFUSED attempt is recorded too: the push step is the only point that \
         observes an attempt at all, so an attempt it saw and did not record is one \
         nothing can ever count. {lines:?}"
    );
    let detail = refused.refusal_detail().expect("a refusal has a detail");
    assert!(
        detail.contains("over-count"),
        "and the cost of that ordering is stated in the refusal rather than left to \
         be discovered: {detail}"
    );

    // The paired negative, restated: nothing that is not a creation reaches the
    // ledger. The push step records only `gh pr create`
    // (`launcher/push.rs::open_pull_request` hard-codes the entry's command), and
    // it never starts a non-creating `gh`, so there is no `gh pr list` path to
    // drive here. The ledger holds exactly the two creations above.
}

// ---------------------------------------------------------------------------
// A hostile alias has no ledger at all
// ---------------------------------------------------------------------------

#[test]
fn a_hostile_alias_gets_no_ledger() {
    let envelope = TempDir::new().unwrap();
    let caps = policy_with_caps(3, 1);
    for hostile in ["../escaped", "a/b", ""] {
        assert!(
            ledger::ledger_path_in(envelope.path(), hostile).is_none(),
            "{hostile:?}"
        );
        let entry = ledger::LedgerEntry {
            at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            run_id: "run-now".into(),
            command: "gh pr create".into(),
            platform: "github".into(),
        };
        assert!(
            ledger::record_and_check_in(envelope.path(), hostile, &entry, &caps).is_err(),
            "{hostile:?} must be refused rather than given a ledger"
        );
    }
}

// ---------------------------------------------------------------------------
// The link-non-following stat variant stays out of the ledger's production half
// (moved here unchanged from `tests/envelope_word_set.rs`).
// ---------------------------------------------------------------------------

#[test]
fn the_link_non_following_stat_variant_is_absent_from_ledger_rs_production_half() {
    // **THE SOURCE-SLICE STANDING PIN, AND WHAT DISSOLVES THE INVERSION
    // `T-19-126`(i) RECORDS.**
    //
    // A WHOLE-FILE grep cannot name what it forbids: writing the forbidden
    // literal into `ledger.rs` to document it would turn a zero-occurrence gate
    // RED. **A PRODUCTION-SLICE assertion can name it, because the name lives in
    // THIS file — the test half — and never in the slice.** That is the same
    // structure
    // `the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so`
    // already uses one file over, where eight forbidden APIs are named below a
    // `#[cfg(test)]` sentinel and asserted absent above it.
    //
    // **GREEN today and after.**
    let sentinel = LEDGER_SOURCE.find("#[cfg(test)]").expect(
        "`ledger.rs` must carry a `#[cfg(test)]` sentinel; without one there is no \
                 production half to slice",
    );
    let production = &LEDGER_SOURCE[..sentinel];

    // -- **THE POSITIVE CONTROLS COME FIRST**, for the reason every absence
    //    assertion in this phase carries one: an absence assertion cannot tell
    //    "the string is not in this region" from "this is not the region I think
    //    it is".
    for anchor in [
        "fn record_and_check_in",
        "MAX_LEDGER_BYTES",
        "std::fs::metadata",
        "file_type().is_file()",
    ] {
        assert!(
            production.contains(anchor),
            "\n\n**THE PRODUCTION SLICE MUST CONTAIN `{anchor}`.** Without these positive \
             controls the absence below passes because the slice is empty or is the wrong \
             region."
        );
    }
    assert_eq!(
        LEDGER_SOURCE.matches("#[cfg(test)]").count(),
        1,
        "**EXACTLY ONE `#[cfg(test)]` SENTINEL**, so the slice boundary is unambiguous. A \
         second sentinel would make `find` pick the first and silently shrink the region."
    );

    // -- **THE FORBIDDEN API, NAMED IN THE ASSERTION.** This literal sits in the
    //    TEST file and therefore OUTSIDE the sliced region, which is exactly why
    //    it can be written at all.
    let forbidden = "symlink_metadata";
    assert!(
        !production.contains(forbidden),
        "\n\n**`{forbidden}` HAS APPEARED IN `ledger.rs`'s PRODUCTION HALF.**\n\n\
         `std::fs::metadata` FOLLOWS symlinks, and that is what keeps a ledger that is a \
         SYMLINK TO A REGULAR FILE countable — the behavioural pin above drives it. The \
         link-NON-following variant would answer about the LINK, never a regular file, and \
         `T-19-120`'s KIND bound would become a LINK BAN.\n\n\
         **THE CORRECT RESPONSE IS TO DELETE THE CALL FROM `src/`, NEVER TO DELETE THIS \
         ASSERTION.**"
    );
    println!(
        "  the `ledger.rs` production slice is {} bytes and contains `{forbidden}` zero times; \
         the literal is written HERE, below the sentinel and outside the slice. **That is what \
         a whole-file grep cannot do, and it is why `ledger.rs:354-361`'s stated reason is \
         INVERTED — `19-33`'s to correct.**",
        production.len()
    );
    println!(
        "RECORDED (not asserted) [`T-19-126`(i), the INVERTED reason]\n  \
         `ledger.rs:354-361` says the API name is left unspelled because *a verify step greps \
         this file for that literal and asserts it appears zero times, so writing it here to \
         forbid it would make the gate pass vacuously*. **Writing it would turn a \
         zero-occurrence gate RED, not make it pass.** The comment is {} in the production \
         source at this run. **`19-33`'s to correct.**",
        if production.contains("pass\n    // vacuously") || production.contains("vacuously") {
            "PRESENT"
        } else {
            "ABSENT (repaired)"
        }
    );
}
