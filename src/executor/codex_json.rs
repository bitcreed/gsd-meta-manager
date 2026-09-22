//! The Codex `exec --json` wire model, mapped onto the executor's existing turn
//! and outcome types.
//!
//! Codex's non-interactive mode prints one JSON object per line on stdout. The
//! shapes below were captured live against codex-cli 0.155.1 (fixtures under
//! `tests/fixtures/codex/`). The mapping is deliberately onto what the Claude
//! path already has, so the outcome derivation stays ONE function:
//!
//! * the first `thread.started` is the start gate (there is no capability
//!   announcement to validate — Codex advertises none);
//! * `turn.completed` / `turn.failed` become a synthesized
//!   [`ResultMessage`], so `derive_run_outcome_from_envelopes` classifies a
//!   Codex turn exactly as it classifies a Claude one (exit code and disk
//!   delta included);
//! * everything else is journaled as a labelled output line and interpreted no
//!   further.
//!
//! **An `item.completed` whose item type is `error` is a WARNING, not a
//! failure.** The failure probe emitted one before `turn.started` for a model
//! metadata fallback; only `turn.failed` or the exit code classify a run.
//!
//! **Nothing may be derived from the absence of an item.** A sandbox-denied
//! command was observed to produce no `command_execution` item at all, so the
//! item stream is a narration, not a tool log.

use serde::Deserialize;
use serde_json::Value;

use crate::executor::gate::GateOutcome;
use crate::executor::stream_json::ResultMessage;

/// One line of `codex exec --json` output.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum CodexEvent {
    /// The session exists. Carries the thread id, which is also the session
    /// id `codex exec resume` accepts and the suffix of the rollout file name.
    #[serde(rename = "thread.started")]
    ThreadStarted {
        /// The thread (session) id.
        thread_id: String,
    },
    /// A turn began.
    #[serde(rename = "turn.started")]
    TurnStarted,
    /// A turn ended cleanly.
    #[serde(rename = "turn.completed")]
    TurnCompleted {
        /// Token counts. Carries no cost field. Not journaled yet.
        #[serde(default)]
        usage: Option<Value>,
    },
    /// A turn ended in failure.
    #[serde(rename = "turn.failed")]
    TurnFailed {
        /// `{"message": "..."}` as observed; kept as a value so an unexpected
        /// shape still classifies the turn as failed instead of becoming an
        /// unparseable line.
        #[serde(default)]
        error: Option<Value>,
    },
    /// An item began.
    #[serde(rename = "item.started")]
    ItemStarted {
        /// The item, unmodelled.
        item: Value,
    },
    /// An item changed.
    #[serde(rename = "item.updated")]
    ItemUpdated {
        /// The item, unmodelled.
        item: Value,
    },
    /// An item finished.
    #[serde(rename = "item.completed")]
    ItemCompleted {
        /// The item, unmodelled.
        item: Value,
    },
    /// A top-level error line. Observed immediately before `turn.failed`.
    #[serde(rename = "error")]
    Error {
        /// The error text, as Codex rendered it.
        #[serde(default)]
        message: Option<String>,
    },
    /// A well-formed line of a type this build does not model. Forward-compat:
    /// carried, never fatal.
    #[serde(other)]
    Unknown,
}

/// Parse one line. A torn or invalid line is an `Err`, which the reader turns
/// into the existing `Unparseable` event.
pub fn parse_codex_line(raw: &str) -> Result<CodexEvent, serde_json::Error> {
    serde_json::from_str(raw)
}

/// What one parsed line means to the coordinator.
#[derive(Debug, Clone)]
pub enum CodexStep {
    /// A `thread.started`. The coordinator gates on the first one.
    ThreadStarted {
        /// The thread (session) id.
        thread_id: String,
    },
    /// A turn boundary, synthesized into the Claude `result` shape.
    TurnEnded(Box<ResultMessage>),
    /// A line to journal under `stream`.
    Output {
        /// `codex:<...>`; always drawn from a fixed alphabet (see
        /// [`item_type_label`]).
        stream: String,
        /// The line's human-readable text.
        text: String,
    },
    /// A type this build does not model.
    Unknown,
}

/// Per-run parser state, owned by the reader task.
#[derive(Debug, Default)]
pub struct CodexStreamState {
    /// The thread id from the first `thread.started`, stamped onto every
    /// synthesized turn as its `session_id`.
    pub thread_id: Option<String>,
}

impl CodexStreamState {
    /// Map one event onto a coordinator step.
    pub fn step(&mut self, event: CodexEvent) -> CodexStep {
        match event {
            CodexEvent::ThreadStarted { thread_id } => {
                if self.thread_id.is_none() {
                    self.thread_id = Some(thread_id.clone());
                }
                CodexStep::ThreadStarted { thread_id }
            }
            CodexEvent::TurnStarted => CodexStep::Output {
                stream: "codex:turn.started".to_string(),
                text: String::new(),
            },
            CodexEvent::TurnCompleted { .. } => CodexStep::TurnEnded(Box::new(ResultMessage {
                subtype: "success".to_string(),
                is_error: false,
                terminal_reason: Some("completed".to_string()),
                session_id: self.thread_id.clone(),
                ..Default::default()
            })),
            CodexEvent::TurnFailed { error } => {
                let message = error
                    .as_ref()
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                CodexStep::TurnEnded(Box::new(ResultMessage {
                    subtype: "error_during_execution".to_string(),
                    is_error: true,
                    session_id: self.thread_id.clone(),
                    errors: message.into_iter().collect(),
                    ..Default::default()
                }))
            }
            CodexEvent::Error { message } => CodexStep::Output {
                stream: "codex:error".to_string(),
                text: message.unwrap_or_default(),
            },
            CodexEvent::ItemStarted { item } => item_output("started", &item),
            CodexEvent::ItemUpdated { item } => item_output("updated", &item),
            CodexEvent::ItemCompleted { item } => item_output("completed", &item),
            CodexEvent::Unknown => CodexStep::Unknown,
        }
    }
}

fn item_output(phase: &str, item: &Value) -> CodexStep {
    CodexStep::Output {
        stream: format!("codex:{phase}:{}", item_type_label(item)),
        text: item_text(item),
    }
}

/// The item's `type`, when it is a plain label; `other` otherwise.
///
/// The value is wire data from a process that reads third-party repository
/// content, and it becomes a journal stream label the TUI paints. So it is
/// accepted only from `[a-z0-9_]{1,32}` — every observed item type fits — and
/// anything else collapses to one fixed word rather than being escaped.
pub fn item_type_label(item: &Value) -> &str {
    match item.get("type").and_then(Value::as_str) {
        Some(kind)
            if !kind.is_empty()
                && kind.len() <= 32
                && kind
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') =>
        {
            kind
        }
        _ => "other",
    }
}

/// The item's readable text: `text`, else `message`, else `command` with its
/// exit code when known, else the item's compact JSON.
fn item_text(item: &Value) -> String {
    if let Some(text) = item.get("text").and_then(Value::as_str) {
        return text.to_string();
    }
    if let Some(message) = item.get("message").and_then(Value::as_str) {
        return message.to_string();
    }
    if let Some(command) = item.get("command").and_then(Value::as_str) {
        return match item.get("exit_code").and_then(Value::as_i64) {
            Some(code) => format!("{command} (exit {code})"),
            None => command.to_string(),
        };
    }
    item.to_string()
}

/// The gate facts recorded for a Codex run.
///
/// Codex announces a thread and nothing else: no capability list, no version,
/// no auth path, no permission mode. Every Claude-only field is therefore
/// absent rather than invented.
pub fn gate_outcome_for_thread(thread_id: &str) -> GateOutcome {
    GateOutcome {
        session_id: Some(thread_id.to_string()),
        capabilities: Vec::new(),
        claude_code_version: None,
        api_key_source: None,
        permission_mode: None,
        permission_mode_confirmed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: &str = include_str!("../../tests/fixtures/codex/01-exec-success.jsonl");
    const FAILURE: &str = include_str!("../../tests/fixtures/codex/02-exec-failure.jsonl");

    fn steps(transcript: &str) -> Vec<CodexStep> {
        let mut state = CodexStreamState::default();
        transcript
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| state.step(parse_codex_line(line).expect("every fixture line parses")))
            .collect()
    }

    #[test]
    fn the_success_transcript_opens_a_thread_narrates_and_ends_one_successful_turn() {
        let steps = steps(SUCCESS);
        assert_eq!(steps.len(), 7);
        assert!(
            matches!(&steps[0], CodexStep::ThreadStarted { thread_id }
                if thread_id == "01a0ca2f-8a0c-72c0-87fb-11c28739d140"),
            "got {:?}",
            steps[0]
        );
        for step in &steps[1..6] {
            assert!(matches!(step, CodexStep::Output { .. }), "got {step:?}");
        }
        assert!(
            matches!(&steps[5], CodexStep::Output { stream, text }
                if stream == "codex:completed:agent_message" && text == "DONE"),
            "got {:?}",
            steps[5]
        );
        assert!(
            matches!(&steps[4], CodexStep::Output { stream, text }
                if stream == "codex:completed:command_execution"
                    && text == "/usr/bin/bash -lc 'cat note.txt' (exit 0)"),
            "got {:?}",
            steps[4]
        );
        let CodexStep::TurnEnded(result) = &steps[6] else {
            panic!("the last line ends the turn, got {:?}", steps[6]);
        };
        assert_eq!(result.subtype, "success");
        assert!(!result.is_error);
        assert_eq!(result.terminal_reason.as_deref(), Some("completed"));
        assert_eq!(
            result.session_id.as_deref(),
            Some("01a0ca2f-8a0c-72c0-87fb-11c28739d140")
        );
        assert_eq!(result.total_cost_usd, None, "Codex reports no cost");
    }

    #[test]
    fn the_failure_transcript_ends_one_failed_turn_carrying_the_message() {
        let steps = steps(FAILURE);
        let CodexStep::TurnEnded(result) = steps.last().expect("non-empty") else {
            panic!("the last line ends the turn, got {:?}", steps.last());
        };
        assert_eq!(result.subtype, "error_during_execution");
        assert!(result.is_error);
        assert_eq!(
            result.session_id.as_deref(),
            Some("01a0ca2f-c735-7202-936b-f2ffba031bdb")
        );
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].contains("no-such-model-xyz"),
            "got {:?}",
            result.errors
        );
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, CodexStep::Output { stream, .. }
                if stream == "codex:error")),
            "the top-level error line is journaled under its own label"
        );
    }

    #[test]
    fn an_error_item_before_the_turn_is_a_warning_and_never_a_turn_end() {
        let steps = steps(FAILURE);
        assert!(
            matches!(&steps[1], CodexStep::Output { stream, text }
                if stream == "codex:completed:error" && text.contains("fallback metadata")),
            "got {:?}",
            steps[1]
        );
        let turn_ends = steps
            .iter()
            .filter(|step| matches!(step, CodexStep::TurnEnded(_)))
            .count();
        assert_eq!(turn_ends, 1, "only turn.failed classifies the run");
    }

    #[test]
    fn an_unknown_type_is_carried_as_unknown_and_a_torn_line_is_an_error() {
        let mut state = CodexStreamState::default();
        let event = parse_codex_line(r#"{"type":"future.thing","x":1}"#).expect("parses");
        assert!(matches!(state.step(event), CodexStep::Unknown));
        assert!(parse_codex_line(r#"{"type":"turn.compl"#).is_err());
        assert!(parse_codex_line("not json").is_err());
    }

    #[test]
    fn a_hostile_item_type_labels_as_other() {
        let long = "a".repeat(200);
        for hostile in [
            "agent\u{1b}[2Jmessage",
            long.as_str(),
            "Agent_Message",
            "",
            "a:b",
        ] {
            let item = serde_json::json!({ "type": hostile, "text": "x" });
            assert_eq!(item_type_label(&item), "other", "{hostile:?}");
        }
        let item = serde_json::json!({ "type": 7 });
        assert_eq!(item_type_label(&item), "other");
        assert_eq!(
            item_type_label(&serde_json::json!({ "type": "file_change" })),
            "file_change"
        );
    }

    #[test]
    fn an_item_with_no_text_falls_back_to_its_compact_json() {
        let mut state = CodexStreamState::default();
        let event =
            parse_codex_line(r#"{"type":"item.updated","item":{"type":"todo_list","items":[]}}"#)
                .expect("parses");
        let CodexStep::Output { stream, text } = state.step(event) else {
            panic!("an item is output");
        };
        assert_eq!(stream, "codex:updated:todo_list");
        assert_eq!(text, r#"{"items":[],"type":"todo_list"}"#);
    }

    #[test]
    fn the_gate_outcome_carries_the_thread_and_nothing_claude_only() {
        let facts = gate_outcome_for_thread("t-1");
        assert_eq!(facts.session_id.as_deref(), Some("t-1"));
        assert!(facts.capabilities.is_empty());
        assert_eq!(facts.claude_code_version, None);
        assert_eq!(facts.api_key_source, None);
        assert_eq!(facts.permission_mode, None);
        assert!(!facts.permission_mode_confirmed);
    }
}
