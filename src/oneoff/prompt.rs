//! The one-off prompt (C-15, T-f5y-08).
//!
//! `/gsd-quick <task>` or `/gsd-fast <task>` on the first line, then a fixed reporting
//! instruction. A todo's body is never inlined: the prompt names its title (one
//! sanitised line) and its path. Typed text is passed verbatim after the command, so
//! it must be one line of at most [`MAX_TASK_TEXT`] characters with no control or
//! invisible formatting character.

use crate::text::carries_invisible_formatting;

use super::todos::{sanitize_title, valid_todo_file_name, COMPLETED_REL, PENDING_REL};
use super::{Mode, TaskSpec, BRANCH_PREFIX};

/// The longest typed task, in characters.
pub const MAX_TASK_TEXT: usize = 2000;

/// The exact report usage line the agent is told to run.
pub const REPORT_USAGE: &str = "gsd-meta-manager taskctl report --status <done|partial|blocked|failed> --headline \"<one line, max 120 chars>\" [--changed \"<item>\"]... [--verified \"<command>: <result>\"]... [--followup \"<item>\"]...";

/// Judge a typed task: non-empty after trimming, at most [`MAX_TASK_TEXT`] characters,
/// no control or invisible formatting character.
pub fn check_task_text(text: &str) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("the task is empty".to_owned());
    }
    let count = text.chars().count();
    if count > MAX_TASK_TEXT {
        return Err(format!(
            "the task is {count} characters; at most {MAX_TASK_TEXT} are allowed"
        ));
    }
    if text.chars().any(char::is_control) || carries_invisible_formatting(text) {
        return Err(
            "the task must be one line with no control or invisible formatting characters"
                .to_owned(),
        );
    }
    Ok(())
}

/// The prompt for one run.
pub fn build(mode: Mode, task: &TaskSpec, run_id: &str) -> Result<String, String> {
    let branch = format!("{BRANCH_PREFIX}{run_id}");
    let mut prompt = String::new();
    let todo_move = match task {
        TaskSpec::Text(text) => {
            check_task_text(text)?;
            prompt.push_str(&format!("{} {text}\n", mode.command()));
            None
        }
        TaskSpec::Todo { file_name, title } => {
            if !valid_todo_file_name(file_name) {
                return Err(format!("{file_name:?} is not a todo file name"));
            }
            let title = sanitize_title(title);
            let title = if title.is_empty() {
                file_name.trim_end_matches(".md").to_owned()
            } else {
                title
            };
            prompt.push_str(&format!(
                "{} {title}\ndetails: {PENDING_REL}/{file_name}\n",
                mode.command()
            ));
            Some(file_name)
        }
    };
    prompt.push_str(&format!(
        "\nHow to work and report (fixed instructions from the dispatcher):\n\
         - The human is unavailable. Decide from the existing planning artifacts and mark every inferred decision `[inferred — audit]`.\n\
         - Work and commit only on branch `{branch}`, which is already checked out. Never push.\n\
         - When you are finished, run exactly:\n  {REPORT_USAGE}\n  Each list holds at most 5 items; each item is one line of at most 200 characters.\n\
         - If that command exits non-zero, read its message, fix the arguments and run it again.\n"
    ));
    if let Some(file_name) = todo_move {
        prompt.push_str(&format!(
            "- If and only if the status is done, run `git mv {PENDING_REL}/{file_name} {COMPLETED_REL}/{file_name}` and commit it.\n"
        ));
    }
    Ok(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn todo() -> TaskSpec {
        TaskSpec::Todo {
            file_name: "2026-11-03-fix.md".to_owned(),
            title: "Fix the parser".to_owned(),
        }
    }

    #[test]
    fn quick_text_starts_with_the_command_and_the_text() {
        let prompt = build(Mode::Quick, &TaskSpec::Text("fix x".to_owned()), "oneoff-1").unwrap();
        assert!(prompt.starts_with("/gsd-quick fix x\n"), "{prompt}");
        assert!(!prompt.contains("details:"));
        assert!(!prompt.contains("git mv"));
    }

    #[test]
    fn fast_uses_the_fast_command() {
        let prompt = build(Mode::Fast, &TaskSpec::Text("x".to_owned()), "oneoff-1").unwrap();
        assert!(prompt.starts_with("/gsd-fast x\n"), "{prompt}");
    }

    #[test]
    fn a_todo_names_its_path_and_the_move() {
        let prompt = build(Mode::Quick, &todo(), "oneoff-1").unwrap();
        assert!(prompt.starts_with("/gsd-quick Fix the parser\n"));
        assert!(prompt.contains("details: .planning/todos/pending/2026-11-03-fix.md"));
        assert!(prompt.contains(
            "git mv .planning/todos/pending/2026-11-03-fix.md .planning/todos/completed/2026-11-03-fix.md"
        ));
    }

    #[test]
    fn both_carry_the_usage_retry_and_marker_rules() {
        for task in [todo(), TaskSpec::Text("t".to_owned())] {
            let prompt = build(Mode::Quick, &task, "oneoff-9").unwrap();
            assert!(prompt.contains("gsd-meta-manager taskctl report --status"));
            assert!(prompt.contains("run it again"));
            assert!(prompt.contains("[inferred — audit]"));
            assert!(prompt.contains("gsd-oneoff/oneoff-9"));
        }
    }

    #[test]
    fn bad_text_is_refused() {
        let long = "a".repeat(MAX_TASK_TEXT + 1);
        for bad in ["", "   ", long.as_str(), "a\u{7}b", "a\nb", "a\u{202E}b"] {
            assert!(
                build(Mode::Quick, &TaskSpec::Text(bad.to_owned()), "oneoff-1").is_err(),
                "{bad:?}"
            );
        }
        let ok = "a".repeat(MAX_TASK_TEXT);
        assert!(build(Mode::Quick, &TaskSpec::Text(ok), "oneoff-1").is_ok());
    }

    #[test]
    fn a_todo_title_is_one_clean_line() {
        let task = TaskSpec::Todo {
            file_name: "t.md".to_owned(),
            title: format!("{}\nsecond line", "x".repeat(300)),
        };
        let prompt = build(Mode::Quick, &task, "oneoff-1").unwrap();
        let first = prompt.lines().next().unwrap();
        assert_eq!(first, format!("/gsd-quick {}", "x".repeat(200)));
    }

    #[test]
    fn a_bad_todo_name_is_refused() {
        let task = TaskSpec::Todo {
            file_name: "../x.md".to_owned(),
            title: "t".to_owned(),
        };
        assert!(build(Mode::Quick, &task, "oneoff-1").is_err());
    }
}
