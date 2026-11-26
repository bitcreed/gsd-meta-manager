//! Heuristic labels on owner items (Phase 38, D-11, OWN-28).
//!
//! A label is a hint printed next to an item: `decision?` for a check that reads
//! like a question the owner has to rule on, `looks resolved` for one whose text says
//! it was fixed. A label changes text only: it never moves, hides, folds, re-ranks or
//! re-counts a row, and never changes a lane, `closes_phase` or a total. The matcher
//! reads the raw project text and emits two constants, so nothing project-authored
//! reaches the output through it.

use std::sync::LazyLock;

use regex::Regex;

use super::{Lane, OwnerItem};

/// One heuristic label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Label {
    /// The check reads like a decision for the owner.
    Decision,
    /// The text says the check was fixed or superseded.
    LooksResolved,
}

impl Label {
    /// The label as printed.
    pub fn as_str(self) -> &'static str {
        match self {
            Label::Decision => "decision?",
            Label::LooksResolved => "looks resolved",
        }
    }
}

/// Words that make a check read like a decision, matched on Unicode word
/// boundaries, case-insensitively.
fn decision_words() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:owner\s+decision|decide|decision|ratify|ratification|approve|approval|accept|sign(?:-|\s+)off|rule\s+on|review)\b",
        )
        .expect("static regex")
    });
    &RE
}

/// `OK` as a whole word, case-sensitive (`okay` and `ok` are ordinary words).
fn ok_word() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bOK\b").expect("static regex"));
    &RE
}

/// Phrases that say the check was dealt with, case-insensitive on word boundaries.
fn resolved_words() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?i)\b(?:resolved|fixed\s+by|fixed\s+in|closed\s+by|addressed\s+by|addressed\s+in|superseded\s+by)\b",
        )
        .expect("static regex")
    });
    &RE
}

fn any_match(re: &Regex, texts: &[Option<&str>]) -> bool {
    texts.iter().flatten().any(|text| re.is_match(text))
}

/// The labels of one item in `lane`. `decision?` reads the name and the expected
/// text and is never given in the Decide lane (everything there is a decision);
/// `looks resolved` also reads the reason.
pub fn labels_of(item: &OwnerItem, lane: Lane) -> Vec<Label> {
    let name = Some(item.name.as_str());
    let expected = item.expected.as_deref();
    let reason = item.reason.as_deref();
    let mut out = Vec::new();
    if lane != Lane::Decide
        && (any_match(decision_words(), &[name, expected])
            || any_match(ok_word(), &[name, expected]))
    {
        out.push(Label::Decision);
    }
    if any_match(resolved_words(), &[name, expected, reason]) {
        out.push(Label::LooksResolved);
    }
    out
}

/// How many items of a group carry each label.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LabelCounts {
    pub decision: usize,
    pub looks_resolved: usize,
}

impl LabelCounts {
    /// `2 decision? · 1 looks resolved`; a zero count is left out, and the text is
    /// empty when both are zero.
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if self.decision > 0 {
            parts.push(format!("{} {}", self.decision, Label::Decision.as_str()));
        }
        if self.looks_resolved > 0 {
            parts.push(format!(
                "{} {}",
                self.looks_resolved,
                Label::LooksResolved.as_str()
            ));
        }
        parts.join(" \u{b7} ")
    }
}

/// The label counts over `items` in `lane`.
pub fn label_counts(items: &[OwnerItem], lane: Lane) -> LabelCounts {
    let mut counts = LabelCounts::default();
    for item in items {
        for label in labels_of(item, lane) {
            match label {
                Label::Decision => counts.decision += 1,
                Label::LooksResolved => counts.looks_resolved += 1,
            }
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::Category;

    fn item(name: &str, expected: Option<&str>, reason: Option<&str>) -> OwnerItem {
        OwnerItem {
            id: "x#1".to_owned(),
            test: None,
            name: name.to_owned(),
            expected: expected.map(str::to_owned),
            result: "pending".to_owned(),
            category: Category::Pending,
            reason: reason.map(str::to_owned),
            blocked_by: None,
            gaps: Vec::new(),
            why_human: None,
        }
    }

    fn of(name: &str) -> Vec<Label> {
        labels_of(&item(name, None, None), Lane::Test)
    }

    #[test]
    fn labels_read_the_d11_words_only() {
        for word in [
            "owner decision",
            "decide",
            "decision",
            "ratify",
            "ratification",
            "approve",
            "approval",
            "accept",
            "sign-off",
            "sign off",
            "rule on",
            "review",
            "Review",
            "pre-review",
            "REVIEW the screen",
            "Looks OK on a phone",
        ] {
            assert_eq!(of(word), [Label::Decision], "{word}");
            assert_eq!(
                labels_of(&item("Check", Some(word), None), Lane::Fix),
                [Label::Decision],
                "expected: {word}"
            );
        }
        for word in [
            "unresolved",
            "preview",
            "reviewer",
            "okay",
            "ok",
            "acceptance",
            "approved",
            "reviewed",
            "\u{c9}review",
            "review\u{e9}",
            "Looks fine",
        ] {
            assert!(of(word).is_empty(), "{word}");
        }
        for phrase in [
            "resolved",
            "fixed by 12",
            "fixed in 1.2",
            "closed by the patch",
            "addressed by 7",
            "addressed in 7",
            "superseded by 9",
            "not resolved yet",
        ] {
            assert_eq!(of(phrase), [Label::LooksResolved], "{phrase}");
            assert_eq!(
                labels_of(&item("Check", None, Some(phrase)), Lane::Test),
                [Label::LooksResolved],
                "reason: {phrase}"
            );
        }
        // `decision?` never reads the reason.
        assert!(labels_of(&item("Check", None, Some("approve")), Lane::Test).is_empty());
        // Both labels, decision first.
        assert_eq!(
            of("Approve: fixed by 12"),
            [Label::Decision, Label::LooksResolved]
        );
        assert!(labels_of(&item("Decide the pricing tiers", None, None), Lane::Decide).is_empty());
        assert!(labels_of(&item("", None, None), Lane::Test).is_empty());
    }

    #[test]
    fn counts_text_leaves_out_zero_counts() {
        let items = [
            item("Approve it", None, None),
            item("Review it", None, None),
            item("Fixed by 3", None, None),
            item("Plain", None, None),
        ];
        let counts = label_counts(&items, Lane::Test);
        assert_eq!(counts.text(), "2 decision? \u{b7} 1 looks resolved");
        assert_eq!(
            label_counts(&items[3..], Lane::Test).text(),
            "",
            "nothing, nothing printed"
        );
        let only = label_counts(&items[2..3], Lane::Test);
        assert_eq!(only.text(), "1 looks resolved");
    }

    #[test]
    fn labels_never_change_entries_or_ranking() {
        use crate::state_reader::owner_items::{
            next_actions, triage_entries, GroupCounts, Kind, OwnerDigest, OwnerGroup, Scope,
        };
        let group = |phase: &str, name: &str, closes: bool| OwnerGroup {
            lane: Lane::Test,
            kind: Kind::Uat,
            scope: Scope::Current,
            workstream: None,
            phase: phase.to_owned(),
            phase_name: "P".to_owned(),
            file: format!("{phase}-UAT.md"),
            file_status: "testing".to_owned(),
            closes_phase: closes,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: Some(5),
            stale: false,
            run: None,
            command: Some(format!("/gsd-verify-work {phase}")),
            supersedes: Vec::new(),
            counts: GroupCounts {
                checks: 2,
                ..Default::default()
            },
            items: vec![item(name, None, None), item(name, None, None)],
        };
        let plain = OwnerDigest {
            groups: vec![
                group("1", "Tap the button", false),
                group("2", "Open the menu", true),
            ],
            ..Default::default()
        };
        let mut loud = plain.clone();
        for g in &mut loud.groups {
            for it in &mut g.items {
                it.name = "Approve: fixed by 12".to_owned();
            }
        }
        let summarize = |d: &OwnerDigest| {
            let entries = triage_entries(d, 90);
            let shape: Vec<_> = entries
                .iter()
                .map(|e| {
                    (
                        e.group.lane,
                        e.group.counts,
                        e.folded,
                        e.indices.clone(),
                        e.group.closes_phase,
                    )
                })
                .collect();
            let order: Vec<String> = next_actions(&[("a", entries.as_slice())], 5)
                .iter()
                .map(|n| n.entry.group.phase.clone())
                .collect();
            (shape, order)
        };
        assert_eq!(summarize(&plain), summarize(&loud));
    }
}
