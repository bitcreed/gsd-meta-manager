//! Golden and lint tests over the embedded driver master prompt (Phase 29,
//! PRMT-01 / PRMT-03 / PRMT-05, D-01..D-07, D-12, D-13, D-22, D-24).
//!
//! The subject is `launcher::prompt::MASTER_PROMPT`, the constant the binary
//! carries, never a file path. Every enum the prompt shows is compared with the
//! protocol constants, never with a hand-copied list, so the prompt cannot drift
//! from the `driverctl` CLI it tells a model to call.

use std::collections::BTreeSet;
use std::path::Path;

use gsd_meta_manager::driverctl::protocol::{
    DONE_WHEN, END_REASONS, MAX_REPORT_LINES, MODES, RECOVER_ACTIONS, REPORT_STATUSES, RULE_ENUM,
};
use gsd_meta_manager::launcher::prompt::{
    header_version, stop_rules_block, DRIVER_PROMPT_VERSION, MASTER_PROMPT, STOP_RULES_BEGIN,
};
use gsd_meta_manager::launcher::sandbox::{render_prompt, MAX_PROMPT_BYTES};
use gsd_meta_manager::state_reader::driver_frontmatter::ItemState;
use regex::Regex;

#[test]
fn the_asset_header_version_equals_the_shipped_constant() {
    assert_eq!(
        header_version(MASTER_PROMPT),
        Some(DRIVER_PROMPT_VERSION),
        "line 1 of src/launcher/assets/MASTER-PROMPT.md and DRIVER_PROMPT_VERSION disagree"
    );
    assert_eq!(DRIVER_PROMPT_VERSION, "driver-prompt/1.0.0");
}

#[test]
fn the_embedded_prompt_is_not_empty() {
    assert!(
        !MASTER_PROMPT.trim().is_empty(),
        "the embedded master prompt is empty"
    );
}

#[test]
fn the_render_fits_the_argv_limit_and_leaves_no_placeholder() {
    let run_dir = format!("/{}", "r".repeat(199));
    assert_eq!(run_dir.len(), 200);
    let rendered = render_prompt(
        MASTER_PROMPT,
        Path::new(&run_dir),
        Path::new("/home/user/project"),
        "loop",
    );
    assert!(
        rendered.len() < MAX_PROMPT_BYTES,
        "rendered prompt is {} bytes, limit {MAX_PROMPT_BYTES}",
        rendered.len()
    );
    for placeholder in ["{RUN_DIR}", "{PROJECT_ROOT}", "{MODE}"] {
        assert!(
            !rendered.contains(placeholder),
            "{placeholder} survived the render"
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// The text under `heading` (an exact line) up to, not including, the next
/// line that starts with `## ` (or the end of the prompt).
fn section<'a>(prompt: &'a str, heading: &str) -> &'a str {
    let start = prompt
        .lines()
        .scan(0usize, |offset, line| {
            let at = *offset;
            *offset += line.len() + 1;
            Some((at, line))
        })
        .find(|(_, line)| *line == heading)
        .map(|(at, _)| at)
        .unwrap_or_else(|| panic!("the prompt has no {heading:?} heading"));
    let body = &prompt[start..];
    let after_heading = body.find('\n').map_or(body.len(), |i| i + 1);
    let end = body[after_heading..]
        .lines()
        .scan(after_heading, |offset, line| {
            let at = *offset;
            *offset += line.len() + 1;
            Some((at, line))
        })
        .find(|(_, line)| line.starts_with("## "))
        .map_or(body.len(), |(at, _)| at);
    &body[..end]
}

/// The numbered top-level rules (`1. ...`, continuation lines indented) of a
/// section, each as its full text.
fn numbered_rules(section: &str) -> Vec<String> {
    let mut rules: Vec<String> = Vec::new();
    for line in section.lines() {
        let starts_rule = line
            .split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        if starts_rule {
            rules.push(line.to_string());
        } else if let Some(last) = rules.last_mut() {
            if line.starts_with(' ') {
                last.push('\n');
                last.push_str(line);
            }
        }
    }
    rules
}

/// The fenced block under the worker-brief heading.
fn worker_brief() -> Vec<&'static str> {
    let body = section(MASTER_PROMPT, "## Worker brief (send this, filled in)");
    let mut in_fence = false;
    let mut brief = Vec::new();
    for line in body.lines() {
        if line.starts_with("```") {
            if in_fence {
                break;
            }
            in_fence = true;
        } else if in_fence {
            brief.push(line);
        }
    }
    assert!(!brief.is_empty(), "no fenced worker brief");
    // SAFETY of lifetimes: MASTER_PROMPT is 'static, so are its lines.
    brief
}

// ---------------------------------------------------------------------------
// D-05 / SC1: the five hard rules
// ---------------------------------------------------------------------------

#[test]
fn the_five_sc1_rules_are_numbered_hard_rules_with_their_anchors() {
    let hard = section(MASTER_PROMPT, "## Hard rules (no exceptions)");
    let rules = numbered_rules(hard);
    assert!(
        rules.len() >= 5,
        "fewer than five numbered hard rules: {rules:?}"
    );
    let at_most = format!("at most {MAX_REPORT_LINES} lines");
    let anchors: [(usize, Vec<&str>); 5] = [
        (1, vec!["One fresh subagent per GSD step"]),
        (2, vec!["11-field report", at_most.as_str(), "verbatim"]),
        (3, vec!["isolation: \"worktree\""]),
        (4, vec!["Never re-spawn", "SendMessage"]),
        (5, vec!["Every record goes through `driverctl`"]),
    ];
    for (number, needles) in anchors {
        let rule = &rules[number - 1];
        assert!(
            rule.starts_with(&format!("{number}. ")),
            "hard rule {number} is out of order: {rule:?}"
        );
        for needle in needles {
            assert!(
                rule.contains(needle),
                "hard rule {number} lost its anchor {needle:?}:\n{rule}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// D-06 / PRMT-01: the report contract
// ---------------------------------------------------------------------------

#[test]
fn the_worker_brief_lists_the_eleven_report_fields_in_order() {
    let expected = [
        "STATUS",
        "ITEM",
        "STEP",
        "COMMITS",
        "WHERE",
        "CHANGED",
        "CHECKS",
        "DONE_WHEN",
        "FINDINGS",
        "NEEDS_HUMAN",
        "SUGGESTED_NEXT",
    ];
    let field = Regex::new(r"^([A-Z][A-Z_]*):").expect("regex");
    let brief = worker_brief();
    let found: Vec<&str> = brief
        .iter()
        .filter_map(|line| field.captures(line).map(|c| c.get(1).unwrap().as_str()))
        .collect();
    assert_eq!(found, expected, "the brief's report fields (at line start)");
    let limit = format!("at most {MAX_REPORT_LINES} lines");
    assert!(
        brief.iter().any(|line| line.contains(&limit)),
        "the worker brief must say {limit:?} (protocol::MAX_REPORT_LINES)"
    );
}

// ---------------------------------------------------------------------------
// D-12 / D-13: the stop-rule precedence block
// ---------------------------------------------------------------------------

#[test]
fn the_stop_rule_block_holds_eight_ordered_rules_and_the_lift_rule() {
    let block = stop_rules_block(MASTER_PROMPT).expect("a stop-rules block in the prompt");
    let items: Vec<&str> = block
        .lines()
        .filter(|line| {
            line.split_once(". ")
                .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
        .collect();
    assert_eq!(items.len(), 8, "exactly eight numbered rules: {items:?}");
    for (index, item) in items.iter().enumerate() {
        assert!(
            item.starts_with(&format!("{}. ", index + 1)),
            "rule {} is out of order: {item:?}",
            index + 1
        );
    }
    assert!(
        items[0].contains("control: stop")
            && items[0].contains("control: pause")
            && items[0].contains("not a park"),
        "rule 1 is human control, ends the run and is not a park: {:?}",
        items[0]
    );
    let parks = [
        "launcher-cap",
        "run-budget",
        "divergence",
        "gap-rounds",
        "budget",
        "no-progress",
        "needs-human",
    ];
    for (offset, word) in parks.iter().enumerate() {
        let item = items[offset + 1];
        assert!(
            item.contains(&format!("--rule {word}")),
            "rule {} must name `--rule {word}`: {item:?}",
            offset + 2
        );
    }
    assert!(block.contains("Divergence outranks gap rounds"));
    assert!(
        block.contains("lifts only a `needs-human` park"),
        "the OQ-R1 lift rule is missing"
    );
    assert!(block.contains("budget edit"));
    assert!(
        !block.contains(['{', '}']),
        "the block is copied into the DIRECTION template, whose lint flags braces"
    );
}

#[test]
fn divergence_is_listed_before_and_outranks_gap_rounds() {
    let block = stop_rules_block(MASTER_PROMPT).expect("a stop-rules block");
    let items: Vec<&str> = block
        .lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    let divergence = items
        .iter()
        .position(|l| l.contains("--rule divergence"))
        .expect("a divergence rule");
    let gap_rounds = items
        .iter()
        .position(|l| l.contains("--rule gap-rounds"))
        .expect("a gap-rounds rule");
    assert!(divergence < gap_rounds, "divergence must be listed first");
    assert!(
        items[divergence].contains("outranks"),
        "the divergence rule must say it outranks gap rounds: {:?}",
        items[divergence]
    );
}

// ---------------------------------------------------------------------------
// D-07: every alternative the prompt shows is in the protocol's vocabulary
// ---------------------------------------------------------------------------

fn words(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|w| (*w).to_string()).collect()
}

#[test]
fn every_flag_alternative_in_the_prompt_is_a_protocol_value() {
    let ends: Vec<&str> = END_REASONS.iter().map(|(word, _)| *word).collect();
    let states: Vec<&str> = ItemState::ALL.iter().map(|s| s.as_str()).collect();
    let allowed: [(&str, BTreeSet<String>); 7] = [
        ("rule", words(RULE_ENUM)),
        ("reason", words(&ends)),
        ("action", words(RECOVER_ACTIONS)),
        ("status", words(REPORT_STATUSES)),
        ("state", words(&states)),
        ("done-when", words(DONE_WHEN)),
        ("mode", words(MODES)),
    ];
    let shown = Regex::new(r"(?:^|\s)--(rule|reason|action|status|state|done-when|mode)\s+<?([a-z][a-z-]*(?:\|[a-z][a-z-]*)*)>?")
        .expect("regex");
    let mut seen = 0usize;
    for line in MASTER_PROMPT.lines() {
        for caps in shown.captures_iter(line) {
            let flag = &caps[1];
            let value = &caps[2];
            if flag == "rule" && value == "rule" {
                continue; // the `[--rule <rule>]` placeholder
            }
            let set = &allowed.iter().find(|(f, _)| *f == flag).expect("flag").1;
            for alternative in value.split('|') {
                assert!(
                    set.contains(alternative),
                    "--{flag} {alternative:?} is not a protocol value ({set:?}) in: {line}"
                );
                seen += 1;
            }
        }
    }
    assert!(
        seen >= 20,
        "only {seen} alternatives were checked; the regex is not matching"
    );
    // A `reset` recovery is expressible: `status --state` offers `pending`.
    let status_line = MASTER_PROMPT
        .lines()
        .find(|l| l.starts_with("driverctl log status"))
        .expect("the status line of the recording block");
    for state in &states {
        assert!(
            status_line.contains(state),
            "{state} missing from {status_line}"
        );
    }
}

// ---------------------------------------------------------------------------
// T-29-02 / PRMT-01: no human at run time
// ---------------------------------------------------------------------------

#[test]
fn every_human_channel_mention_carries_a_negation() {
    const SUBJECTS: [&str; 4] = ["AskUserQuestion", "tmux", "stdin", "wait for input"];
    const NEGATIONS: [&str; 7] = ["Never", "never", "no ", "No ", "not ", "Do not", "without"];
    const INSTRUCTIONS: [&str; 3] = ["call AskUserQuestion", "tmux attach", "read from stdin"];
    let mut mentions = 0usize;
    for line in MASTER_PROMPT.lines() {
        if SUBJECTS.iter().any(|s| line.contains(s)) {
            mentions += 1;
            assert!(
                NEGATIONS.iter().any(|n| line.contains(n)),
                "a line names a human channel without a negation: {line:?}"
            );
        }
        for shape in INSTRUCTIONS {
            assert!(
                !line.contains(shape),
                "the prompt instructs {shape:?}: {line:?}"
            );
        }
    }
    assert!(
        mentions >= 2,
        "expected the prohibitions to be present, found {mentions}"
    );
}

// ---------------------------------------------------------------------------
// D-22 / PRMT-05, D-16, D-17, D-18, D-14: single-line anchors
// ---------------------------------------------------------------------------

#[test]
fn the_router_hint_is_cited_once_and_is_advisory() {
    let cite = "next {PROJECT_ROOT} --json";
    assert_eq!(MASTER_PROMPT.matches(cite).count(), 1, "cited exactly once");
    let line = MASTER_PROMPT
        .lines()
        .find(|l| l.contains(cite))
        .expect("the citation line");
    assert!(
        line.contains("advisory"),
        "the citation line must say advisory: {line:?}"
    );
    let loop_section = section(MASTER_PROMPT, "## Loop");
    assert!(
        loop_section.contains(cite),
        "the citation belongs to the Loop section"
    );
    for phrase in [
        "never binds",
        "never replaces checking done-when",
        "is information, not a stop rule",
    ] {
        assert!(
            MASTER_PROMPT.contains(phrase),
            "the router-hint text lost {phrase:?}"
        );
    }
}

#[test]
fn the_reconcile_control_gap_and_verification_anchors_exist() {
    for anchor in [
        "LAUNCHER CAP: control=",
        "exactly one `recovered`",
        "caps.gap_rounds",
        "human-verification",
        "Before a gap-closure step, check stop rules 4 and 5 first.",
    ] {
        assert!(MASTER_PROMPT.contains(anchor), "the prompt lost {anchor:?}");
    }
}

// ---------------------------------------------------------------------------
// D-24: the test-only stripped variant is DERIVED from the embedded prompt
//
// Mirrors spike-003 `tools/make_variant.py`: every span the transform touches must
// occur exactly once, or the transform fails naming it. The committed fixture is
// the input plan 29-07's H4 case hands to the hidden `--prompt-file`; it is never
// edited by hand (rerun with `REGENERATE_PROMPT_VARIANT=1`).
// ---------------------------------------------------------------------------

const VARIANT_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md"
);

const VARIANT_HEADER_SUFFIX: &str = " TEST-ONLY VARIANT (D-24): gap-round and divergence stop rules removed; recorded as custom:<sha8> when launched";

const NO_ROUND_LIMIT_LINE: &str = "- Gap closure has no round limit in this run: keep closing `gaps_found` until verification passes or a dispatch is denied.";

const GAP_SEMANTICS_BEGIN: &str = "<!-- gap-semantics:begin -->";
const GAP_SEMANTICS_END: &str = "<!-- gap-semantics:end -->";
const PRE_CHECK_SENTENCE: &str = "Before a gap-closure step, check stop rules 4 and 5 first.";

/// The byte offset of the single occurrence of `needle`, or an error naming `span`.
fn exactly_once(text: &str, needle: &str, span: &str) -> Result<usize, String> {
    match text.matches(needle).count() {
        1 => Ok(text.find(needle).expect("counted once")),
        n => Err(format!(
            "span {span:?} ({needle:?}) must occur exactly once, found {n}"
        )),
    }
}

/// The prompt with its gap-round and divergence rules removed (D-24).
fn strip_gap_rules(prompt: &str) -> Result<String, String> {
    // (1) The header gains the test-only suffix.
    let header = format!("<!-- {DRIVER_PROMPT_VERSION}");
    let at = exactly_once(prompt, &header, "driver-prompt header")?;
    if at != 0 {
        return Err("span \"driver-prompt header\" is not on line 1".to_string());
    }
    let mut text = format!("{header}{VARIANT_HEADER_SUFFIX}{}", &prompt[header.len()..]);

    // (2) Stop rules 4 and 5 become one no-round-limit line.
    let begin = exactly_once(&text, STOP_RULES_BEGIN, "stop-rules begin marker")?;
    let block = stop_rules_block(&text)
        .ok_or_else(|| "span \"stop-rules block\" is missing or out of order".to_string())?
        .to_string();
    for number in [4, 5] {
        let prefix = format!("{number}. ");
        let count = block
            .lines()
            .filter(|line| line.starts_with(&prefix))
            .count();
        if count != 1 {
            return Err(format!(
                "span \"stop rule {number}\" ({prefix:?} line in the stop-rules block) must occur exactly once, found {count}"
            ));
        }
    }
    let mut stripped = String::new();
    for line in block.split_inclusive('\n') {
        if line.starts_with("4. ") {
            stripped.push_str(NO_ROUND_LIMIT_LINE);
            stripped.push('\n');
        } else if !line.starts_with("5. ") {
            stripped.push_str(line);
        }
    }
    text.replace_range(begin..begin + block.len(), &stripped);

    // (3) The gap-semantics section goes, with its trailing newline.
    let from = exactly_once(&text, GAP_SEMANTICS_BEGIN, "gap-semantics begin marker")?;
    let to = exactly_once(&text, GAP_SEMANTICS_END, "gap-semantics end marker")?;
    if to < from {
        return Err("span \"gap-semantics\" has its end marker before its begin".to_string());
    }
    let mut cut_end = to + GAP_SEMANTICS_END.len();
    if text[cut_end..].starts_with('\n') {
        cut_end += 1;
    }
    text.replace_range(from..cut_end, "");

    // (4) The gap-closure pre-check line goes.
    let matches: Vec<&str> = text
        .split_inclusive('\n')
        .filter(|line| line.trim() == PRE_CHECK_SENTENCE)
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "span \"gap-closure pre-check sentence\" ({PRE_CHECK_SENTENCE:?}) must occur exactly once, found {}",
            matches.len()
        ));
    }
    let line = matches[0].to_string();
    text = text.replacen(&line, "", 1);
    Ok(text)
}

#[test]
fn the_no_gap_rules_variant_is_derived_from_the_embedded_prompt() {
    let variant = strip_gap_rules(MASTER_PROMPT).expect("the transform applies to the prompt");
    if std::env::var("REGENERATE_PROMPT_VARIANT").as_deref() == Ok("1") {
        let dir = Path::new(VARIANT_FIXTURE).parent().expect("a fixture dir");
        std::fs::create_dir_all(dir).expect("create the fixture dir");
        std::fs::write(VARIANT_FIXTURE, &variant).expect("write the fixture");
        return;
    }
    let committed = std::fs::read_to_string(VARIANT_FIXTURE).unwrap_or_default();
    assert!(
        committed == variant,
        "tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md is missing or no longer equals the \
         derivation of the embedded prompt. After a deliberate prompt change, rerun with \
         REGENERATE_PROMPT_VARIANT=1 to rewrite it (never edit it by hand)."
    );
}

#[test]
fn the_variant_transform_names_a_span_that_is_not_unique() {
    // Rule 5 duplicated.
    let rule5 = stop_rules_block(MASTER_PROMPT)
        .expect("a stop-rules block")
        .lines()
        .find(|line| line.starts_with("5. "))
        .expect("stop rule 5");
    let doubled = MASTER_PROMPT.replacen(rule5, &format!("{rule5}\n{rule5}"), 1);
    let err = strip_gap_rules(&doubled).expect_err("a doubled rule 5 must fail");
    assert!(err.contains("stop rule 5"), "{err}");

    // Gap-semantics end marker removed.
    let no_end = MASTER_PROMPT.replacen(GAP_SEMANTICS_END, "", 1);
    let err = strip_gap_rules(&no_end).expect_err("a missing end marker must fail");
    assert!(err.contains("gap-semantics end marker"), "{err}");

    // Pre-check sentence removed.
    let no_check = MASTER_PROMPT.replacen(PRE_CHECK_SENTENCE, "", 1);
    let err = strip_gap_rules(&no_check).expect_err("a missing pre-check must fail");
    assert!(err.contains("pre-check"), "{err}");
}

#[test]
fn the_variant_drops_rules_4_and_5_and_keeps_the_rest() {
    let variant = strip_gap_rules(MASTER_PROMPT).expect("the transform applies");
    let block = stop_rules_block(&variant).expect("the variant keeps its stop-rules block");
    for gone in ["4. ", "5. "] {
        assert!(
            !block.lines().any(|line| line.starts_with(gone)),
            "the variant still has a rule starting {gone:?}"
        );
    }
    for kept in ["1. ", "2. ", "3. ", "6. ", "7. ", "8. "] {
        assert!(
            block.lines().any(|line| line.starts_with(kept)),
            "the variant lost the rule starting {kept:?}"
        );
    }
    assert!(variant.contains("no round limit"));
    assert!(block.contains("lifts only a `needs-human` park"));
    assert!(!variant.contains("gap-semantics"));
    assert!(!variant.contains(PRE_CHECK_SENTENCE));
}
