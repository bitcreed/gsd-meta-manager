//! The DIRECTION lint behind `driver check` (Phase 29; D-11, DRIVE-03).
//!
//! `driver check <project> <run-id>` turns the human-written DIRECTION body into a
//! machine-checkable plan: it reports, with file line numbers, everything that
//! keeps the file from being a plan the driver can follow, before any launch (the
//! launch preflight reuses [`lint_direction`]).
//!
//! # One reader with the store (WR-01)
//!
//! [`parse_direction`](crate::driver::direction::parse_direction) is what the store
//! reads (the gate's control word, the items `driverctl log` accepts, the
//! frontmatter counts). This lint is a separate, pure, strict reader that does no
//! I/O, but the two share the comment reader ([`blank_comments`]) and the section
//! reader ([`section_heading`]), so they cannot disagree about what is an item or
//! a control line: a commented-out item is invisible to both, and a `### I-n`
//! heading or a `- control:` line outside its section, or a second `- control:`
//! line, is reported here instead of being silently honoured or ignored.
//!
//! # Placeholders (Pitfall 16)
//!
//! A placeholder is one of the exact brace spans in the shipped template body
//! outside HTML comments ([`template_placeholders`]), never "anything in braces":
//! `${HOME}`, `{"a":1}` and `{a,b}` are legitimate text in a `done-when`. HTML
//! comments are blanked (byte for byte, newlines kept) before any matching, so a
//! placeholder inside a comment is never a problem and line numbers stay true.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

use super::DIRECTION_TEMPLATE;
use crate::cli::DEFAULT_CAP_GAP_ROUNDS;
use crate::driver::direction::{blank_comments, section_heading, split_frontmatter, Section};
use crate::text::render_for_terminal;

/// The launcher's default gap-round cap (`driver launch --cap-gap-rounds`). An item
/// budget above it only holds if the human relaunches with a higher cap.
const DEFAULT_GAP_CAP: u64 = DEFAULT_CAP_GAP_ROUNDS as u64;

/// The `control` words DIRECTION.md may carry.
const CONTROL_WORDS: [&str; 3] = ["run", "pause", "stop"];

/// The keys an item `budget` may carry.
const BUDGET_KEYS: [&str; 3] = ["dispatches", "minutes", "gap-rounds"];

/// The keys a `run-budget` may carry.
const RUN_BUDGET_KEYS: [&str; 2] = ["dispatches", "hours"];

static KEY_VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^- ([A-Za-z][A-Za-z-]*):[ \t]*(.*?)[ \t]*$").expect("static regex")
});
static ITEM_HEADING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^### (I-\d+)[ \t]+(\S.*?)[ \t]*$").expect("static regex"));
static STRAY_ITEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^### (I-\d+)\b").expect("static regex"));
static ITEM_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^I-\d+$").expect("static regex"));
static BRACE_SPAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{[^{}\n]*\}").expect("static regex"));

/// One lint finding: a file line (frontmatter lines counted) and a message.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// 1-based line in DIRECTION.md.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

/// Both printed forms go through [`render_for_terminal`] (WR-03): a message quotes
/// text from DIRECTION.md (a value, a heading, a piece of a list), and that file can
/// arrive with a cloned repository carrying escape sequences, C1 controls or bidi
/// overrides. `message` itself stays the raw text so callers and tests can compare it.
impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DIRECTION.md:{}: {}",
            self.line,
            render_for_terminal(&self.message)
        )
    }
}

impl Finding {
    /// The warning form: `DIRECTION.md:<line>: warning: <message>`.
    pub fn as_warning(&self) -> String {
        format!(
            "DIRECTION.md:{}: warning: {}",
            self.line,
            render_for_terminal(&self.message)
        )
    }
}

/// The result of [`lint_direction`]; both lists are sorted by (line, message).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintReport {
    /// Findings that make DIRECTION.md an invalid plan.
    pub problems: Vec<Finding>,
    /// Findings worth a look that do not change the verdict (exit code).
    pub warnings: Vec<Finding>,
}

impl LintReport {
    /// No problems (warnings do not count).
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
    }
}

/// The brace spans of the shipped template body outside HTML comments, in order,
/// without repeats: the unfilled placeholders (`{run-id}` is the one `driver new`
/// fills itself).
pub fn template_placeholders() -> Vec<&'static str> {
    static SPANS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let blanked = blank_comments(DIRECTION_TEMPLATE);
        let mut spans: Vec<&'static str> = Vec::new();
        for m in BRACE_SPAN.find_iter(&blanked) {
            // `blanked` has the template's offsets, so the span can be sliced from
            // the 'static original.
            let span = &DIRECTION_TEMPLATE[m.start()..m.end()];
            if !spans.contains(&span) {
                spans.push(span);
            }
        }
        spans
    });
    SPANS.clone()
}

/// What a `### ` heading under `## Items` turned out to be.
struct Item {
    id: String,
    heading_line: usize,
    /// First occurrence of each `- key: value` line: key -> (line, value).
    fields: BTreeMap<String, (usize, String)>,
}

/// A non-negative decimal integer (no sign, no fraction, no overflow).
fn parse_count(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

/// Check a `k=v, k=v` list against `allowed`; returns the integer pairs or pushes
/// one problem per bad piece on `line`.
fn check_pairs(
    what: &str,
    value: &str,
    allowed: &[&str],
    line: usize,
    problems: &mut Vec<Finding>,
) -> Vec<(String, u64)> {
    let mut pairs = Vec::new();
    let mut bad = |message: String| problems.push(Finding { line, message });
    if value.is_empty() {
        bad(format!(
            "{what} has no pairs; expected {}",
            pair_hint(allowed)
        ));
        return pairs;
    }
    for piece in value.split(',') {
        let piece = piece.trim();
        let Some((key, number)) = piece.split_once('=') else {
            bad(format!(
                "{what} piece `{piece}` is not <key>=<integer>; expected {}",
                pair_hint(allowed)
            ));
            continue;
        };
        let (key, number) = (key.trim(), number.trim());
        if !allowed.contains(&key) {
            bad(format!(
                "{what} has unknown key `{key}`; expected {}",
                pair_hint(allowed)
            ));
            continue;
        }
        match parse_count(number) {
            Some(n) => pairs.push((key.to_owned(), n)),
            None => bad(format!(
                "{what} pair `{key}={number}` is not a non-negative integer"
            )),
        }
    }
    pairs
}

fn pair_hint(allowed: &[&str]) -> String {
    format!("comma-separated integer pairs of {}", allowed.join("|"))
}

/// Lint a whole DIRECTION.md (frontmatter optional, split off with
/// [`split_frontmatter`]); every finding carries its file line number.
pub fn lint_direction(text: &str) -> LintReport {
    let (_, body) = split_frontmatter(text);
    let skipped = &text[..text.len() - body.len()];
    let first_line = skipped.matches('\n').count() + 1;
    let blanked = blank_comments(body);
    let lines: Vec<(usize, &str)> = blanked
        .lines()
        .enumerate()
        .map(|(i, l)| (first_line + i, l))
        .collect();

    let mut problems: Vec<Finding> = Vec::new();
    let mut warnings: Vec<Finding> = Vec::new();

    let mut section = Section::Preamble;
    let mut direction_line: Option<usize> = None;
    let mut items_line: Option<usize> = None;
    let mut controls: Vec<(usize, String)> = Vec::new();
    let mut items: Vec<Item> = Vec::new();
    let mut seen_ids: BTreeMap<String, usize> = BTreeMap::new();
    let mut headings_under_items = 0usize;

    for &(n, raw) in &lines {
        let line = raw.trim_end();
        if let Some(next) = section_heading(line) {
            match next {
                Section::Direction => {
                    direction_line.get_or_insert(n);
                }
                Section::Items => {
                    items_line.get_or_insert(n);
                }
                Section::Preamble | Section::Other => {}
            }
            section = next;
            continue;
        }
        if section != Section::Items {
            if let Some(caps) = STRAY_ITEM.captures(line) {
                problems.push(Finding {
                    line: n,
                    message: format!(
                        "item heading {} is outside `## Items`; the driver ignores it",
                        &caps[1]
                    ),
                });
            }
        }
        if section == Section::Items && line.starts_with("### ") {
            headings_under_items += 1;
            if let Some(caps) = ITEM_HEADING.captures(line) {
                let id = caps[1].to_owned();
                if let Some(first) = seen_ids.get(&id) {
                    problems.push(Finding {
                        line: n,
                        message: format!("duplicate item id {id} (first defined on line {first})"),
                    });
                } else {
                    seen_ids.insert(id.clone(), n);
                }
                items.push(Item {
                    id,
                    heading_line: n,
                    fields: BTreeMap::new(),
                });
            } else {
                problems.push(Finding {
                    line: n,
                    message: format!(
                        "an item heading must be `### I-<n> <title>` (found `{}`)",
                        line.trim()
                    ),
                });
                // Lines under a malformed heading belong to no item.
                items.push(Item {
                    id: String::new(),
                    heading_line: n,
                    fields: BTreeMap::new(),
                });
            }
            continue;
        }
        let Some(caps) = KEY_VALUE.captures(line) else {
            continue;
        };
        let (key, value) = (caps[1].to_owned(), caps[2].to_owned());
        match (section, key.as_str()) {
            (Section::Direction, "control") => controls.push((n, value)),
            (_, "control") => problems.push(Finding {
                line: n,
                message: "`- control:` is outside `## Direction`; the driver still reads it \
                          (a `pause` or `stop` there holds the run), so move it"
                    .to_owned(),
            }),
            (Section::Direction, "run-budget") => {
                check_pairs("run-budget", &value, &RUN_BUDGET_KEYS, n, &mut problems);
            }
            (_, "human-verification") => {
                if !matches!(value.as_str(), "defer" | "skip") {
                    problems.push(Finding {
                        line: n,
                        message: format!(
                            "human-verification must be `defer` or `skip` (found `{value}`)"
                        ),
                    });
                }
            }
            (Section::Items, _) => {
                if let Some(item) = items.last_mut().filter(|i| !i.id.is_empty()) {
                    item.fields.entry(key).or_insert((n, value));
                }
            }
            _ => {}
        }
    }

    // Sections.
    if direction_line.is_none() {
        problems.push(Finding {
            line: 1,
            message: "missing the `## Direction` section".to_owned(),
        });
    }
    if items_line.is_none() {
        problems.push(Finding {
            line: 1,
            message: "missing the `## Items` section".to_owned(),
        });
    }
    if let Some(at) = direction_line {
        if controls.is_empty() {
            problems.push(Finding {
                line: at,
                message: "no `- control:` line under `## Direction`".to_owned(),
            });
        }
    }
    for (n, _) in controls.iter().skip(1) {
        problems.push(Finding {
            line: *n,
            message: "more than one `- control:` line; keep exactly one (the driver holds the \
                      run if any of them says `pause` or `stop`)"
                .to_owned(),
        });
    }
    for (n, value) in &controls {
        if !CONTROL_WORDS.contains(&value.as_str()) {
            problems.push(Finding {
                line: *n,
                message: format!(
                    "control must be exactly one of {} (found `{value}`)",
                    CONTROL_WORDS.join(", ")
                ),
            });
        }
    }
    if let Some(at) = items_line {
        if headings_under_items == 0 {
            problems.push(Finding {
                line: at,
                message: "no items under `## Items`".to_owned(),
            });
        }
    }

    // Items.
    let known: BTreeSet<&str> = seen_ids.keys().map(String::as_str).collect();
    let mut edges: BTreeMap<&str, (usize, Vec<&str>)> = BTreeMap::new();
    for item in items.iter().filter(|i| !i.id.is_empty()) {
        let id = item.id.as_str();
        let head = item.heading_line;
        for key in ["intent", "done-when"] {
            match item.fields.get(key) {
                Some((_, v)) if !v.is_empty() => {}
                Some((n, _)) => problems.push(Finding {
                    line: *n,
                    message: format!("{id}: `{key}` is empty"),
                }),
                None => problems.push(Finding {
                    line: head,
                    message: format!("{id}: missing `- {key}:`"),
                }),
            }
        }
        match item.fields.get("budget") {
            None => problems.push(Finding {
                line: head,
                message: format!("{id}: missing `- budget:`"),
            }),
            Some((n, v)) => {
                let before = problems.len();
                let pairs = check_pairs("budget", v, &BUDGET_KEYS, *n, &mut problems);
                for finding in &mut problems[before..] {
                    finding.message = format!("{id}: {}", finding.message);
                }
                for (key, count) in pairs {
                    if key == "gap-rounds" && count > DEFAULT_GAP_CAP {
                        warnings.push(Finding {
                            line: *n,
                            message: format!(
                                "gap-rounds {count} exceeds the launcher's default gap cap \
                                 {DEFAULT_GAP_CAP}; relaunch with --cap-gap-rounds {count}"
                            ),
                        });
                    }
                }
            }
        }
        if let Some((n, v)) = item.fields.get("hold") {
            if !matches!(v.as_str(), "true" | "false") {
                problems.push(Finding {
                    line: *n,
                    message: format!("{id}: hold must be `true` or `false` (found `{v}`)"),
                });
            }
        }
        if let Some((n, v)) = item.fields.get("after") {
            if v != "none" {
                let mut targets = Vec::new();
                for part in v.split(',').map(str::trim) {
                    if !ITEM_ID.is_match(part) {
                        problems.push(Finding {
                            line: *n,
                            message: format!(
                                "{id}: after must be `none` or a comma list of item ids \
                                 (found `{v}`)"
                            ),
                        });
                        targets.clear();
                        break;
                    }
                    if known.contains(part) {
                        targets.push(part);
                    } else {
                        problems.push(Finding {
                            line: *n,
                            message: format!("{id}: after names {part}, which is not an item"),
                        });
                    }
                }
                // The first definition of an id owns its edges.
                edges.entry(id).or_insert((*n, targets));
            }
        }
    }
    problems.extend(cycle_problems(&items, &edges));

    // Unfilled placeholders, anywhere in the body outside comments.
    let placeholders = template_placeholders();
    for &(n, line) in &lines {
        for span in &placeholders {
            if line.contains(span) {
                problems.push(Finding {
                    line: n,
                    message: format!("placeholder still present: {span}"),
                });
            }
        }
    }

    problems.sort();
    problems.dedup();
    warnings.sort();
    warnings.dedup();
    LintReport { problems, warnings }
}

/// The first `- control: <word>` line under `## Direction`: its 1-based file line and
/// its value, read the way [`lint_direction`] reads it (comments blanked, frontmatter
/// lines counted). `None` when there is no such line.
///
/// The launch preflight uses it to point its "control is pause" refusal at the line
/// the human has to change (RESEARCH Open Question 2).
pub fn control_line(text: &str) -> Option<(usize, String)> {
    let (_, body) = split_frontmatter(text);
    let skipped = &text[..text.len() - body.len()];
    let first_line = skipped.matches('\n').count() + 1;
    let blanked = blank_comments(body);
    let mut in_direction = false;
    for (i, raw) in blanked.lines().enumerate() {
        let line = raw.trim_end();
        if line.starts_with("## ") || line == "##" {
            in_direction = line.trim() == "## Direction";
            continue;
        }
        if !in_direction {
            continue;
        }
        if let Some(caps) = KEY_VALUE.captures(line) {
            if &caps[1] == "control" {
                return Some((first_line + i, caps[2].to_owned()));
            }
        }
    }
    None
}

/// One problem per dependency cycle in the `after` graph (a self-edge is a cycle),
/// reported on the `after` line of the cycle's first member in file order.
fn cycle_problems(items: &[Item], edges: &BTreeMap<&str, (usize, Vec<&str>)>) -> Vec<Finding> {
    // File order of first definitions: the DFS order and the rotation key.
    let mut order: Vec<&str> = Vec::new();
    for item in items.iter().filter(|i| !i.id.is_empty()) {
        if !order.contains(&item.id.as_str()) {
            order.push(item.id.as_str());
        }
    }
    let position = |id: &str| order.iter().position(|o| *o == id).unwrap_or(usize::MAX);

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unseen,
        OnPath,
        Done,
    }
    fn visit<'a>(
        node: &'a str,
        edges: &BTreeMap<&'a str, (usize, Vec<&'a str>)>,
        marks: &mut BTreeMap<&'a str, Mark>,
        path: &mut Vec<&'a str>,
        cycles: &mut Vec<Vec<&'a str>>,
    ) {
        marks.insert(node, Mark::OnPath);
        path.push(node);
        if let Some((_, targets)) = edges.get(node) {
            for &next in targets {
                match marks.get(next).copied().unwrap_or(Mark::Unseen) {
                    Mark::Unseen => visit(next, edges, marks, path, cycles),
                    Mark::OnPath => {
                        let from = path.iter().position(|p| *p == next).unwrap_or(0);
                        cycles.push(path[from..].to_vec());
                    }
                    Mark::Done => {}
                }
            }
        }
        path.pop();
        marks.insert(node, Mark::Done);
    }

    let mut marks: BTreeMap<&str, Mark> = BTreeMap::new();
    let mut cycles: Vec<Vec<&str>> = Vec::new();
    for &start in &order {
        if marks.get(start).copied().unwrap_or(Mark::Unseen) == Mark::Unseen {
            let mut path = Vec::new();
            visit(start, edges, &mut marks, &mut path, &mut cycles);
        }
    }

    let mut seen: BTreeSet<Vec<&str>> = BTreeSet::new();
    let mut findings = Vec::new();
    for mut cycle in cycles {
        // Start the cycle at its first member in file order.
        let first = (0..cycle.len())
            .min_by_key(|i| position(cycle[*i]))
            .unwrap_or(0);
        cycle.rotate_left(first);
        let mut members = cycle.clone();
        members.sort_unstable();
        if !seen.insert(members) {
            continue;
        }
        let line = edges.get(cycle[0]).map_or(1, |(n, _)| *n);
        let mut shown = cycle.clone();
        shown.push(cycle[0]);
        findings.push(Finding {
            line,
            message: format!("dependency cycle in after: {}", shown.join(" -> ")),
        });
    }
    findings
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
    fn control_line_reports_the_file_line_and_word_of_the_control() {
        let text = filled();
        let want = line_of(&text, "- control:");
        assert_eq!(control_line(&text), Some((want, "run".to_owned())));
        let paused = text.replacen("- control: run", "- control: pause", 1);
        assert_eq!(control_line(&paused), Some((want, "pause".to_owned())));
    }

    #[test]
    fn control_line_ignores_comments_and_other_sections() {
        let text =
            "<!-- - control: stop -->\n## Items\n- control: stop\n## Direction\n- control: run\n";
        assert_eq!(control_line(text), Some((5, "run".to_owned())));
        assert_eq!(control_line("## Direction\n- goal: x\n"), None);
    }

    #[test]
    fn control_line_counts_frontmatter_lines() {
        let text = "---\nstatus: new\n---\n## Direction\n- control: stop\n";
        assert_eq!(control_line(text), Some((5, "stop".to_owned())));
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

    // -- WR-01: the lint and the store's parser read one plan

    /// The ids the store would accept (`parse_direction`) and the ids the lint saw.
    fn store_ids(text: &str) -> Vec<String> {
        let (_, body) = split_frontmatter(text);
        crate::driver::direction::parse_direction(body)
            .items
            .into_iter()
            .map(|i| i.id)
            .collect()
    }

    #[test]
    fn a_multi_line_commented_out_item_is_invisible_to_the_lint_and_the_store() {
        let text = format!(
            "{}\n<!--\n### I-09 parked idea\n\n- intent: x\n- done-when: y\n- budget: dispatches=1\n-->\n",
            filled()
        );
        assert!(lint_direction(&text).is_clean());
        assert_eq!(store_ids(&text), ["I-01"], "the store does not count I-09");
    }

    #[test]
    fn a_stray_item_heading_outside_items_is_flagged_and_not_a_store_item() {
        let text = filled().replacen(
            "- control: run\n",
            "- control: run\n\n### I-09 stray\n\n",
            1,
        );
        let p = only_problem(&text);
        assert_eq!(p.line, line_of(&text, "### I-09"));
        assert!(
            p.message.contains("I-09") && p.message.contains("outside `## Items`"),
            "{p}"
        );
        assert_eq!(store_ids(&text), ["I-01"], "the store ignores it as well");

        // A later `## ` section is no better.
        let later = format!("{}\n## Notes\n\n### I-07 notes\n", filled());
        let p = only_problem(&later);
        assert!(p.message.contains("I-07"), "{p}");
        assert_eq!(store_ids(&later), ["I-01"]);
    }

    #[test]
    fn a_finding_quoting_hostile_direction_text_prints_no_control_or_bidi_character() {
        // WR-03: ESC, a C1 CSI, a bell and a right-to-left override in a DIRECTION value.
        let hostile = "\u{1b}[31mred\u{9b}2J\u{7}x\u{202e}gnp.exe";
        let text = filled().replacen("- control: run", &format!("- control: {hostile}"), 1);
        let report = lint_direction(&text);
        let finding = report
            .problems
            .iter()
            .find(|p| p.message.contains("control must be"))
            .expect("the bad control word is a problem");
        assert!(
            finding.message.contains('\u{1b}'),
            "the raw message is kept"
        );
        for shown in [finding.to_string(), finding.as_warning()] {
            for c in shown.chars() {
                assert!(
                    !c.is_control() && c != '\u{202e}',
                    "{shown:?} carries {c:?}"
                );
            }
            assert!(shown.starts_with("DIRECTION.md:"), "{shown:?}");
        }
    }

    #[test]
    fn a_duplicate_control_line_is_flagged_on_the_second_line() {
        let text = filled().replacen("- control: run\n", "- control: run\n- control: pause\n", 1);
        let p = only_problem(&text);
        assert_eq!(p.line, line_of(&text, "- control: pause"));
        assert!(p.message.contains("more than one `- control:`"), "{p}");
    }

    #[test]
    fn a_control_line_outside_direction_is_flagged() {
        let text = format!("{}\n## Notes\n\n- control: pause\n", filled());
        let report = lint_direction(&text);
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.line == line_of(&text, "- control: pause")
                    && p.message.contains("outside `## Direction`")),
            "{:#?}",
            report.problems
        );
    }
}
