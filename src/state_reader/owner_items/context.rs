//! Layer A for CONTEXT open questions: a port of gsd-core 1.21.0's
//! `deriveOpenQuestions`, `deriveOpenQuestionsDigest` and `scanContextQuestions`
//! (`audit.cjs` 197-262, 764-797, 943-1005) with `sanitizeForDisplay`
//! (`security.cjs` 478-510), for the `audit-open` oracle (Phase 35, D-04, D-10).
//!
//! # Numbered facts
//!
//! 1. **Which questions.** A non-empty frontmatter `open_questions` list wins, each
//!    entry as a FAILSAFE string (an entry that is not a scalar reads as the empty
//!    string) through [`sanitize_for_display`]. Otherwise the first level-2 heading
//!    whose trimmed, lower-cased text starts with `open questions` opens a
//!    level-bounded section. A body that is empty or only `none` (any case) holds no
//!    questions. Else every trimmed line is kept when it is not empty, not a bare `-`
//!    or `*`, and starts with `-`, `*` or an ASCII digit or contains `?`.
//! 2. **Sanitizing.** Invisible characters are stripped, role tags and instruction
//!    markers are neutralized and lines that mimic a protocol leak are dropped from
//!    a question (the question itself stays, possibly empty, as upstream keeps it).
//! 3. **Digest.** `sha256` over the concatenation of `<len>:<question>` where `<len>`
//!    counts UTF-16 code units (JavaScript's `String.length`), over the complete
//!    sanitized list. A file whose `audit_acknowledged.questions_digest` equals it is
//!    acknowledged.
//! 4. **Which files.** Every real subdirectory of each tree's `phases/` (the root and
//!    every workstream, no milestone filter) and every archived phase directory of
//!    the root and all workstreams. Names containing `-CONTEXT` and ending in `.md`,
//!    contained in their phase directory, read through the size cap. A file without
//!    questions is omitted.
//! 5. **Fail-safe, beyond upstream.** A file that cannot be read stays in the list
//!    marked unreadable (upstream skips it).

use std::path::Path;
use std::sync::{Arc, LazyLock};

use regex::Regex;
use serde_yml::{Mapping, Value};
use sha2::{Digest, Sha256};

use super::scan::{
    archived_targets, contained, dir_token, open_checked, posix_relative, read_text,
    subdirectories, Target,
};
use super::uat::js_trim;
use super::{frontmatter, markdown, ContextFile, ParseCache, Scope, CONTEXT_CACHE};
use crate::state_reader::ProjectState;

/// What the text of one CONTEXT document alone determines.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedContext {
    pub questions: Vec<String>,
    pub status: String,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub acknowledged: bool,
}

/// JavaScript's `\s` as a regex class body.
const JS_SPACE: &str = r"\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}";

/// `word` matched ignoring ASCII case only (a JavaScript regex without the `u` flag
/// folds nothing else), spelled as one class per letter.
fn ascii_ci(word: &str) -> String {
    word.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
            } else {
                c.to_string()
            }
        })
        .collect()
}

struct Sanitizers {
    invisible: Regex,
    role_tag: Regex,
    marker: Regex,
    llama: Regex,
    leak_to: Regex,
    leak_token: Regex,
}

static SANITIZERS: LazyLock<Sanitizers> = LazyLock::new(|| {
    let roles = format!(
        "(?:{}|{}|{}|{})",
        ascii_ci("system"),
        ascii_ci("assistant"),
        ascii_ci("human"),
        ascii_ci("user")
    );
    let leak_roles = format!(
        "(?:{}|{}|{})",
        ascii_ci("assistant"),
        ascii_ci("user"),
        ascii_ci("system")
    );
    let build = |pattern: String| Regex::new(&pattern).expect("a valid static pattern");
    Sanitizers {
        invisible: build("[\\x{200B}-\\x{200F}\\x{2028}-\\x{202F}\\x{FEFF}\\x{00AD}]".to_owned()),
        role_tag: build(format!("<(/?)[{JS_SPACE}]*{roles}[{JS_SPACE}]*>")),
        marker: build(format!(
            "\\[(/?)({}|{})\\]",
            ascii_ci("system"),
            ascii_ci("inst")
        )),
        llama: build(format!(
            "<</?[{JS_SPACE}]*{}[{JS_SPACE}]*>>",
            ascii_ci("sys")
        )),
        leak_to: build(format!(
            "^[{JS_SPACE}]*{leak_roles}[{JS_SPACE}]+to=[^:{JS_SPACE}]+:[^\\n]+$"
        )),
        leak_token: build(format!(
            "^[{JS_SPACE}]*<\\|{leak_roles}[^|]*\\|>[{JS_SPACE}]*$"
        )),
    }
});

/// `sanitizeForDisplay`: zero-width characters stripped, role tags and `[SYSTEM]` /
/// `[INST]` / `<<SYS>>` markers neutralized, protocol-leak lines dropped.
pub(super) fn sanitize_for_display(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let rules = &*SANITIZERS;
    let text = rules.invisible.replace_all(text, "");
    let text = rules
        .role_tag
        .replace_all(&text, |caps: &regex::Captures<'_>| {
            format!("＜{}system-text＞", &caps[1])
        });
    let text = rules
        .marker
        .replace_all(&text, |caps: &regex::Captures<'_>| {
            format!("[{}{}-TEXT]", &caps[1], caps[2].to_ascii_uppercase())
        });
    let text = rules.llama.replace_all(&text, "«SYS-TEXT»");
    text.split('\n')
        .filter(|line| !rules.leak_to.is_match(line) && !rules.leak_token.is_match(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `deriveOpenQuestions`: the complete, sanitized question list of a document.
pub(super) fn derive_open_questions(text: &str, fm: &Mapping) -> Vec<String> {
    let mut questions = Vec::new();
    if let Some(Value::Sequence(entries)) = fm.get("open_questions") {
        questions = entries
            .iter()
            .map(|entry| {
                sanitize_for_display(&frontmatter::failsafe_str(entry).unwrap_or_default())
            })
            .collect();
    }
    if !questions.is_empty() {
        return questions;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let Some(section) = markdown::collect_section(
        &lines,
        |heading| {
            heading.level == 2
                && heading
                    .text
                    .trim()
                    .to_lowercase()
                    .starts_with("open questions")
        },
        true,
    ) else {
        return questions;
    };
    let body = js_trim(&section.body);
    if body.is_empty() || body.eq_ignore_ascii_case("none") {
        return questions;
    }
    body.split('\n')
        .map(js_trim)
        .filter(|line| !line.is_empty() && *line != "-" && *line != "*")
        .filter(|line| {
            line.starts_with(['-', '*'])
                || line.starts_with(|c: char| c.is_ascii_digit())
                || line.contains('?')
        })
        .map(sanitize_for_display)
        .collect()
}

/// `deriveOpenQuestionsDigest`: sha256 hex of `<utf-16 length>:<question>` for every
/// question, concatenated.
pub(super) fn questions_digest(questions: &[String]) -> String {
    let mut hasher = Sha256::new();
    for question in questions {
        hasher.update(format!("{}:{}", question.encode_utf16().count(), question));
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Parse the text of one CONTEXT document.
pub(crate) fn parse_context(text: &str) -> ParsedContext {
    let fm = frontmatter::extract(text);
    let questions = derive_open_questions(text, &fm);
    let status = frontmatter::fm_str(&fm, "status")
        .filter(|status| !status.is_empty())
        .map_or_else(|| "unknown".to_owned(), |status| status.to_lowercase());
    let updated = frontmatter::fm_str(&fm, "updated")
        .filter(|value| !value.is_empty())
        .and_then(|value| frontmatter::moment(&value));
    let acknowledged =
        frontmatter::is_acknowledged(&fm, "questions_digest", &questions_digest(&questions));
    ParsedContext {
        questions,
        status,
        updated,
        acknowledged,
    }
}

/// Every CONTEXT document with open questions, process-free, sorted by `rel_path`.
pub(super) fn scan_context_questions(
    planning_dir: &Path,
    state: &ProjectState,
) -> Vec<ContextFile> {
    scan_with(&CONTEXT_CACHE, planning_dir, state)
}

/// Every phase directory upstream's `listAuditPhaseTargets` reads: the real
/// subdirectories of each tree's `phases/` (no milestone filter), then every archived
/// phase directory.
fn context_targets(planning_dir: &Path, state: &ProjectState) -> Vec<Target> {
    let mut out = Vec::new();
    for (workstream, _) in state.scopes() {
        let tree = match workstream {
            None => planning_dir.to_path_buf(),
            Some(stream) => planning_dir.join("workstreams").join(&stream.name),
        };
        let phases_dir = tree.join("phases");
        for dir in subdirectories(&phases_dir) {
            out.push(Target {
                path: phases_dir.join(&dir),
                dir,
                scope: Scope::Current,
                workstream: workstream.map(|stream| stream.name.clone()),
            });
        }
    }
    out.extend(archived_targets(planning_dir));
    out
}

fn scan_with(
    cache: &ParseCache<(), ParsedContext>,
    planning_dir: &Path,
    state: &ProjectState,
) -> Vec<ContextFile> {
    let Ok(root_real) = planning_dir.canonicalize() else {
        return Vec::new();
    };
    let project_root = planning_dir.parent().unwrap_or(planning_dir);
    let mut files: Vec<ContextFile> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for target in context_targets(planning_dir, state) {
        let Ok(entries) = std::fs::read_dir(&target.path) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.contains("-CONTEXT") && name.ends_with(".md"))
            .collect();
        names.sort();
        let phase = dir_token(&target.dir);
        let in_current_milestone = match &target.scope {
            Scope::Current => state
                .scopes()
                .find(|(stream, _)| {
                    stream.map(|stream| stream.name.as_str()) == target.workstream.as_deref()
                })
                .is_some_and(|(_, scope)| scope.in_current_milestone(&phase)),
            Scope::Archived { .. } => false,
        };
        for name in names {
            let path = target.path.join(&name);
            let Some(real) = contained(&root_real, &target.path, &path) else {
                continue;
            };
            let rel_path = posix_relative(project_root, &path);
            if !seen.insert(rel_path.clone()) {
                continue;
            }
            let mut file = ContextFile {
                rel_path,
                scope: target.scope.clone(),
                workstream: target.workstream.clone(),
                phase: phase.clone(),
                phase_dir: target.dir.clone(),
                in_current_milestone,
                questions: Vec::new(),
                status: "unknown".to_owned(),
                updated: None,
                modified: None,
                acknowledged: false,
                unreadable: None,
            };
            match open_checked(&real) {
                Ok(opened) => {
                    file.modified = opened.modified;
                    let parsed = match cache.get(&real, (), opened.key) {
                        Some(hit) => hit,
                        None => match read_text(opened.file) {
                            Ok(text) => {
                                let parsed = Arc::new(parse_context(&text));
                                cache.insert(real, (), opened.key, Arc::clone(&parsed));
                                parsed
                            }
                            Err(reason) => {
                                file.unreadable = Some(reason);
                                files.push(file);
                                continue;
                            }
                        },
                    };
                    if parsed.questions.is_empty() {
                        continue;
                    }
                    file.questions = parsed.questions.clone();
                    file.status = parsed.status.clone();
                    file.updated = parsed.updated;
                    file.acknowledged = parsed.acknowledged;
                }
                Err(reason) => file.unreadable = Some(reason),
            }
            files.push(file);
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::evict_project;
    use crate::state_reader::parse_project_state;

    fn questions_of(text: &str) -> Vec<String> {
        parse_context(text).questions
    }

    #[test]
    fn a_none_body_holds_no_questions() {
        assert!(questions_of("## Open Questions\n\nNone\n").is_empty());
        assert!(questions_of("## Open Questions\n\n none \n").is_empty());
        assert!(questions_of("## Open Questions\n\nNONE\n").is_empty());
        assert!(questions_of("## Open Questions\n\n\n").is_empty());
    }

    #[test]
    fn a_none_with_a_period_is_not_the_marker_but_is_no_question_either() {
        assert!(questions_of("## Open Questions\n\nNONE.\n").is_empty());
    }

    #[test]
    fn the_heading_may_carry_a_suffix_and_must_be_level_two() {
        let text = "## Open Questions — all three ANSWERED by research\n\n- Is Y ready?\n";
        assert_eq!(questions_of(text), ["- Is Y ready?"]);
        assert!(questions_of("### Open Questions\n\n- Is Y ready?\n").is_empty());
    }

    #[test]
    fn the_line_filter_keeps_markers_digits_and_question_marks() {
        let text = "## Open Questions\n\n-\n1. Pick X\nWhy now?\nPlain prose.\n*\n* star\n";
        assert_eq!(questions_of(text), ["1. Pick X", "Why now?", "* star"]);
    }

    #[test]
    fn the_section_ends_at_the_next_heading_of_its_level_or_higher() {
        let text = "## Open Questions\n\n- A?\n### Sub\n- B?\n## Next\n- C?\n";
        assert_eq!(questions_of(text), ["- A?", "- B?"]);
    }

    #[test]
    fn frontmatter_questions_win_and_an_empty_list_falls_back() {
        let text = "---\nopen_questions: [a, b]\n---\n\n## Open Questions\n\n- body?\n";
        assert_eq!(questions_of(text), ["a", "b"]);
        let text = "---\nopen_questions: []\n---\n\n## Open Questions\n\n- body?\n";
        assert_eq!(questions_of(text), ["- body?"]);
    }

    #[test]
    fn invisible_characters_are_stripped() {
        assert_eq!(
            sanitize_for_display("a\u{200b}b\u{ad}c\u{feff}d\u{2028}e"),
            "abcde"
        );
    }

    #[test]
    fn role_tags_and_markers_are_neutralized() {
        assert_eq!(sanitize_for_display("<system>"), "＜system-text＞");
        assert_eq!(sanitize_for_display("</assistant>"), "＜/system-text＞");
        assert_eq!(sanitize_for_display("< USER >"), "＜system-text＞");
        assert_eq!(sanitize_for_display("[INST]"), "[INST-TEXT]");
        assert_eq!(sanitize_for_display("[/system]"), "[/SYSTEM-TEXT]");
        assert_eq!(sanitize_for_display("<<SYS>>"), "«SYS-TEXT»");
        assert_eq!(sanitize_for_display("<</ sys >>"), "«SYS-TEXT»");
    }

    #[test]
    fn protocol_leak_lines_are_dropped() {
        assert_eq!(sanitize_for_display("assistant to=x: y"), "");
        assert_eq!(sanitize_for_display("<|user|>"), "");
        assert_eq!(
            sanitize_for_display("keep\nassistant to=x: y\nkeep too"),
            "keep\nkeep too"
        );
        assert_eq!(
            sanitize_for_display("say assistant to=x: y"),
            "say assistant to=x: y"
        );
    }

    #[test]
    fn the_digest_is_gsd_cores() {
        assert_eq!(
            questions_digest(&["- Is Y ready?".to_owned()]),
            "f9473be7ec4213c58042ed3c06d0725903da6ca8ed79463b5edb6074dfb4775d"
        );
        assert_eq!(
            questions_digest(&[]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn the_length_prefix_counts_utf16_units() {
        // "- Prüfung ok? 😀" is 15 chars, 16 UTF-16 units, 18 bytes; node prints
        // `crypto.createHash('sha256').update('16:- Prüfung ok? 😀')`.
        let question = "- Prüfung ok? 😀".to_owned();
        assert_eq!(question.encode_utf16().count(), 16);
        assert_eq!(
            questions_digest(&[question]),
            "e84c507007527fe9e3241eebc1794a2ab3611dc6eb7abdce59ccd4356b177e15"
        );
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join(".planning/ROADMAP.md"),
            "# Roadmap\n\n## Phases\n\n- [ ] **Phase 1: Alpha** - a\n\n### Phase 1: Alpha\n\n**Goal**: a\n**Depends on**: Nothing\n",
        );
        write(
            &dir.path().join(".planning/STATE.md"),
            "---\nstatus: in_progress\n---\n\n# State\n",
        );
        dir
    }

    const ONE: &str = "## Open Questions\n\n- Is Y ready?\n";

    fn scan_of(root: &Path) -> Vec<ContextFile> {
        let planning = root.join(".planning");
        evict_project(root);
        scan_with(
            &ParseCache::new(64),
            &planning,
            &parse_project_state(&planning),
        )
    }

    fn rels(files: &[ContextFile]) -> Vec<&str> {
        files.iter().map(|f| f.rel_path.as_str()).collect()
    }

    #[test]
    fn an_active_dir_absent_from_the_roadmap_and_any_context_name_are_read() {
        let dir = project();
        write(
            &dir.path().join(".planning/phases/09-gone/09-CONTEXT.md"),
            ONE,
        );
        write(
            &dir.path()
                .join(".planning/phases/01-alpha/NOTES-CONTEXT.md"),
            ONE,
        );
        write(
            &dir.path().join(".planning/phases/01-alpha/01-CONTEXT.txt"),
            ONE,
        );
        write(
            &dir.path().join(".planning/phases/01-alpha/CONTEXT.md"),
            ONE,
        );
        let files = scan_of(dir.path());
        assert_eq!(
            rels(&files),
            [
                ".planning/phases/01-alpha/NOTES-CONTEXT.md",
                ".planning/phases/09-gone/09-CONTEXT.md"
            ]
        );
        assert_eq!(files[1].phase, "09");
        assert!(files.iter().all(|f| f.questions == ["- Is Y ready?"]));
    }

    #[test]
    fn a_file_without_questions_is_omitted_and_an_acknowledged_one_is_flagged() {
        let dir = project();
        write(
            &dir.path().join(".planning/phases/01-alpha/01-CONTEXT.md"),
            "## Open Questions\n\nNone\n",
        );
        let ack = "---\naudit_acknowledged:\n  milestone: v1\n  at: 2026-01-01\n  questions_digest: f9473be7ec4213c58042ed3c06d0725903da6ca8ed79463b5edb6074dfb4775d\n---\n\n## Open Questions\n\n- Is Y ready?\n";
        write(
            &dir.path().join(".planning/phases/01-alpha/02-CONTEXT.md"),
            ack,
        );
        let files = scan_of(dir.path());
        assert_eq!(rels(&files), [".planning/phases/01-alpha/02-CONTEXT.md"]);
        assert!(files[0].acknowledged);
    }

    #[cfg(unix)]
    #[test]
    fn a_context_symlink_escaping_its_phase_dir_is_absent() {
        let dir = project();
        write(&dir.path().join("outside/00-CONTEXT.md"), ONE);
        std::fs::create_dir_all(dir.path().join(".planning/phases/01-alpha")).unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("outside/00-CONTEXT.md"),
            dir.path().join(".planning/phases/01-alpha/01-CONTEXT.md"),
        )
        .unwrap();
        assert!(scan_of(dir.path()).is_empty());
    }

    #[test]
    fn archived_and_workstream_directories_are_read_with_their_scope() {
        let dir = project();
        write(
            &dir.path()
                .join(".planning/milestones/v0.9-phases/00-legacy/00-CONTEXT.md"),
            ONE,
        );
        write(
            &dir.path().join(".planning/workstreams/ws1/ROADMAP.md"),
            "# Roadmap\n\n## Phases\n\n- [ ] **Phase 1: Stream** - s\n\n### Phase 1: Stream\n\n**Goal**: s\n",
        );
        write(
            &dir.path().join(".planning/workstreams/ws1/STATE.md"),
            "---\nstatus: in_progress\n---\n",
        );
        write(
            &dir.path()
                .join(".planning/workstreams/ws1/phases/01-stream/01-CONTEXT.md"),
            ONE,
        );
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 2);
        let archived = files.iter().find(|f| f.phase == "00").unwrap();
        assert!(matches!(&archived.scope, Scope::Archived { label, .. } if label == "v0.9"));
        assert!(!archived.in_current_milestone);
        let stream = files.iter().find(|f| f.phase == "01").unwrap();
        assert_eq!(stream.workstream.as_deref(), Some("ws1"));
        assert!(stream.in_current_milestone);
    }

    #[test]
    fn a_second_scan_reads_nothing_from_disk() {
        let dir = project();
        write(
            &dir.path().join(".planning/phases/01-alpha/01-CONTEXT.md"),
            ONE,
        );
        let planning = dir.path().join(".planning");
        let state = parse_project_state(&planning);
        let cache = ParseCache::new(64);
        let before = crate::state_reader::owner_items::CACHE_HITS.with(std::cell::Cell::get);
        let first = scan_with(&cache, &planning, &state);
        let second = scan_with(&cache, &planning, &state);
        let after = crate::state_reader::owner_items::CACHE_HITS.with(std::cell::Cell::get);
        assert_eq!(first, second);
        assert_eq!(after - before, 1);
    }
}
