//! The DIRECTION lint behind `driver check` (Phase 29; D-11, DRIVE-03).
//!
//! RED stub: the real lint lands in the next commit.

use std::fmt;

/// One lint finding: a file line (frontmatter lines counted) and a message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// 1-based line in DIRECTION.md.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DIRECTION.md:{}: {}", self.line, self.message)
    }
}

impl Finding {
    /// The warning form: `DIRECTION.md:<line>: warning: <message>`.
    pub fn as_warning(&self) -> String {
        format!("DIRECTION.md:{}: warning: {}", self.line, self.message)
    }
}

/// The result of [`lint_direction`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintReport {
    /// Findings that make DIRECTION.md an invalid plan.
    pub problems: Vec<Finding>,
    /// Findings worth a look that do not change the verdict.
    pub warnings: Vec<Finding>,
}

impl LintReport {
    /// No problems (warnings do not count).
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
    }
}

/// The brace spans of the shipped template body outside HTML comments.
pub fn template_placeholders() -> Vec<&'static str> {
    Vec::new()
}

/// Lint a whole DIRECTION.md (frontmatter optional).
pub fn lint_direction(_text: &str) -> LintReport {
    LintReport::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver_templates::{DIRECTION_TEMPLATE, RUN_ID_TOKEN};

    /// The template with the run id filled and every placeholder replaced.
    fn filled() -> String {
        let mut text = DIRECTION_TEMPLATE.replace(RUN_ID_TOKEN, "20261019-t");
        for span in template_placeholders() {
            text = text.replace(span, "filled");
        }
        text
    }

    /// The 1-based line of the first line containing `needle`.
    fn line_of(text: &str, needle: &str) -> usize {
        text.lines()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("no line contains {needle:?}"))
            + 1
    }

    fn item(id: &str, after: &str) -> String {
        format!(
            "\n### {id} more\n\n- intent: x\n- done-when: y\n- budget: dispatches=1\n- after: {after}\n- hold: false\n- answer:\n"
        )
    }

    /// Exactly one problem, returned.
    fn only_problem(text: &str) -> Finding {
        let report = lint_direction(text);
        assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
        report.problems.into_iter().next().expect("one problem")
    }

    #[test]
    fn the_placeholder_list_is_the_templates_brace_spans_outside_comments() {
        assert_eq!(
            template_placeholders(),
            [
                "{run-id}",
                "{one or two sentences: what this run is for}",
                "{project-specific rules, e.g. \"run cargo test --no-fail-fast, not cargo test\"}",
                "{short title}",
                "{what should be true afterwards and why, in plain words}",
                "{a check the driver can run itself, e.g. `cargo test --no-fail-fast` passes AND `.planning/phases/21-*/21-VERIFICATION.md` has status: passed}",
                "{optional GSD command hint, e.g. /gsd-plan-phase 26; the driver may choose otherwise}",
            ]
        );
    }

    #[test]
    fn the_unfilled_template_reports_one_problem_per_placeholder_and_nothing_else() {
        let text = DIRECTION_TEMPLATE.replace(RUN_ID_TOKEN, "20261019-t");
        let report = lint_direction(&text);
        let spans: Vec<&str> = template_placeholders()
            .into_iter()
            .filter(|s| *s != RUN_ID_TOKEN)
            .collect();
        assert_eq!(spans.len(), 6, "the template has six placeholders to fill");
        assert_eq!(report.problems.len(), spans.len(), "{:#?}", report.problems);
        for span in spans {
            let line = line_of(&text, span);
            assert!(
                report
                    .problems
                    .iter()
                    .any(|p| p.line == line && p.message.contains(span)),
                "no problem for {span} on line {line}: {:#?}",
                report.problems
            );
        }
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn the_filled_template_is_clean() {
        let report = lint_direction(&filled());
        assert!(report.is_clean(), "{:#?}", report.problems);
        assert!(report.warnings.is_empty(), "{:#?}", report.warnings);
    }

    #[test]
    fn each_problem_kind_is_reported_once_on_the_right_file_line() {
        let base = filled();
        let cases: Vec<(&str, String, &str, &str)> = vec![
            (
                "control maybe",
                base.replacen("- control: run", "- control: maybe", 1),
                "- control: maybe",
                "control",
            ),
            (
                "control run pause",
                base.replacen("- control: run", "- control: run pause", 1),
                "- control: run pause",
                "control",
            ),
            (
                "control absent",
                base.replacen("- control: run\n", "", 1),
                "## Direction",
                "control",
            ),
            (
                "empty intent",
                base.replacen("- intent: filled", "- intent:", 1),
                "- intent:",
                "intent",
            ),
            (
                "no done-when",
                base.replacen("- done-when: filled\n", "", 1),
                "### I-01",
                "done-when",
            ),
            (
                "empty done-when",
                base.replacen("- done-when: filled", "- done-when:", 1),
                "- done-when:",
                "done-when",
            ),
            (
                "no budget",
                base.replacen("- budget: dispatches=4, gap-rounds=2, minutes=90\n", "", 1),
                "### I-01",
                "budget",
            ),
            (
                "budget x",
                base.replacen("dispatches=4, gap-rounds=2, minutes=90", "dispatches=x", 1),
                "- budget:",
                "dispatches=x",
            ),
            (
                "budget tokens",
                base.replacen("dispatches=4, gap-rounds=2, minutes=90", "tokens=3", 1),
                "- budget:",
                "tokens",
            ),
            (
                "after unknown",
                base.replacen("- after: none", "- after: I-09", 1),
                "- after:",
                "I-09",
            ),
            (
                "after garbage",
                base.replacen("- after: none", "- after: whenever", 1),
                "- after:",
                "after",
            ),
            (
                "hold maybe",
                base.replacen("- hold: false", "- hold: maybe", 1),
                "- hold:",
                "hold",
            ),
            (
                "run-budget hours",
                base.replacen("hours=4", "hours=two", 1),
                "- run-budget:",
                "hours",
            ),
            (
                "human-verification auto",
                base.replacen(
                    "- human-verification: defer",
                    "- human-verification: auto",
                    1,
                ),
                "- human-verification:",
                "human-verification",
            ),
        ];
        for (name, text, anchor, word) in cases {
            let p = only_problem(&text);
            assert_eq!(p.line, line_of(&text, anchor), "{name}: {p}");
            assert!(p.message.contains(word), "{name}: {p}");
            assert!(p
                .to_string()
                .starts_with(&format!("DIRECTION.md:{}: ", p.line)));
        }
    }

    #[test]
    fn a_heading_under_items_that_is_not_an_item_is_one_problem() {
        let text = format!("{}\n### Notes\n\n- foo: bar\n", filled());
        let p = only_problem(&text);
        assert_eq!(p.line, line_of(&text, "### Notes"));
        assert!(p.message.contains("I-<n>"), "{p}");
    }

    #[test]
    fn a_duplicate_item_id_is_one_problem_on_the_second_heading() {
        let text = format!("{}{}", filled(), item("I-01", "none"));
        let p = only_problem(&text);
        assert_eq!(p.line, line_of(&text, "### I-01 more"));
        assert!(p.message.contains("duplicate"), "{p}");
    }

    #[test]
    fn an_item_that_names_itself_in_after_is_a_cycle() {
        let text = filled().replacen("- after: none", "- after: I-01", 1);
        let p = only_problem(&text);
        assert!(p.message.contains("cycle"), "{p}");
    }

    #[test]
    fn a_two_item_cycle_is_one_problem() {
        let text = format!(
            "{}{}",
            filled().replacen("- after: none", "- after: I-02", 1),
            item("I-02", "I-01")
        );
        let p = only_problem(&text);
        assert!(
            p.message.contains("cycle") && p.message.contains("I-01") && p.message.contains("I-02"),
            "{p}"
        );
    }

    #[test]
    fn an_after_naming_a_later_item_is_valid() {
        let text = format!(
            "{}{}",
            filled().replacen("- after: none", "- after: I-02", 1),
            item("I-02", "none")
        );
        let report = lint_direction(&text);
        assert!(report.is_clean(), "{:#?}", report.problems);
    }

    #[test]
    fn a_budget_without_gap_rounds_is_valid_and_gap_rounds_above_two_only_warns() {
        let base = filled();
        let no_gap = base.replacen("dispatches=4, gap-rounds=2, minutes=90", "dispatches=4", 1);
        assert!(lint_direction(&no_gap).is_clean());

        let text = base.replacen("gap-rounds=2", "gap-rounds=3", 1);
        let report = lint_direction(&text);
        assert!(report.is_clean(), "{:#?}", report.problems);
        assert_eq!(report.warnings.len(), 1, "{:#?}", report.warnings);
        assert_eq!(report.warnings[0].line, line_of(&text, "- budget:"));
        assert_eq!(
            report.warnings[0].as_warning(),
            format!(
                "DIRECTION.md:{}: warning: gap-rounds 3 exceeds the launcher's default gap cap 2; relaunch with --cap-gap-rounds 3",
                report.warnings[0].line
            )
        );
    }

    #[test]
    fn brace_text_that_is_not_a_template_placeholder_is_not_a_problem() {
        let text = filled().replacen(
            "- done-when: filled",
            "- done-when: `echo ${HOME}` prints {\"a\":1} and {a,b}",
            1,
        );
        assert!(lint_direction(&text).is_clean());
    }

    #[test]
    fn a_placeholder_inside_an_html_comment_is_not_a_problem() {
        let comment = format!("<!-- {} -->\n", template_placeholders()[1]);
        let text = format!("{comment}{}", filled());
        assert!(lint_direction(&text).is_clean());
        let multi = filled().replacen(
            "## Items\n",
            &format!("## Items\n<!--\n{}\n-->\n", template_placeholders()[1]),
            1,
        );
        assert!(lint_direction(&multi).is_clean());
    }

    #[test]
    fn an_empty_body_reports_both_missing_sections() {
        let report = lint_direction("");
        assert_eq!(report.problems.len(), 2, "{:#?}", report.problems);
        assert!(report
            .problems
            .iter()
            .any(|p| p.message.contains("## Direction")));
        assert!(report
            .problems
            .iter()
            .any(|p| p.message.contains("## Items")));
    }

    #[test]
    fn items_with_no_item_reports_no_items() {
        let text = "## Direction\n\n- control: run\n\n## Items\n\nnothing yet\n";
        let p = only_problem(text);
        assert_eq!(p.line, 5);
        assert!(p.message.contains("no items"), "{p}");
    }

    #[test]
    fn findings_come_sorted_by_line_then_message() {
        let text = filled()
            .replacen("- hold: false", "- hold: maybe", 1)
            .replacen("- control: run", "- control: maybe", 1)
            .replacen("- intent: filled", "- intent:", 1);
        let report = lint_direction(&text);
        assert!(report.problems.len() >= 3);
        let mut sorted = report.problems.clone();
        sorted.sort();
        assert_eq!(report.problems, sorted);
    }

    #[test]
    fn frontmatter_lines_are_counted_in_file_line_numbers() {
        let block = "---\ngenerated_by: driverctl\nrun_id: \"x\"\n---\n";
        let text = format!(
            "{block}{}",
            filled().replacen("- hold: false", "- hold: maybe", 1)
        );
        let p = only_problem(&text);
        assert_eq!(p.line, line_of(&text, "- hold: maybe"));
        assert!(p.line > 4);
    }
}
