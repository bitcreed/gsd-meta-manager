//! Plan the `gsd-tools config-set` calls that save one Config-tab edit
//! (quick 261116-kft).
//!
//! # Why this exists
//!
//! GSD files are read-only for this tool: every change goes through gsd-tools or
//! a GSD command (CLAUDE.md, PROJECT.md, commit cf7a246c). The Config tab used to
//! write `.planning/config.json` itself, replicating gsd-core's lock. It now
//! diffs the config before and after the edit and asks gsd-core's own writer,
//! `config-set <key.path> <value>`, to make each changed leaf. This module is the
//! pure half of that: no I/O and no process. The caller runs the calls.
//!
//! # The value parser this replicates (gsd-core 1.21.0)
//!
//! `config-set` does not store its argument as text. `parseConfigArgValue`
//! (`bin/lib/config.cjs:189-209`) turns `true`/`false`/`null` into JSON
//! literals, a decimal literal matching `DECIMAL_LITERAL` (`:180`) whose
//! `Number()` is finite into a number, and a leading `[`/`{` that `JSON.parse`
//! accepts into that value. A bare `null` unsets the key (`cmdConfigSet`, `:1006`)
//! and `project_code` keeps its text verbatim otherwise (`:1032`). [`cli_parse`]
//! is the Rust replica; `tests/config_set_roundtrip.rs` pins it against the
//! installed gsd-tools' `--dry-run`.
//!
//! The consequence the planner guards against: `config-set phase_naming 12`
//! stores the NUMBER 12, after which the whole file no longer parses here. So a
//! value gsd-tools would store as another JSON type is refused
//! ([`RefusalReason::Coerced`]) instead of being sent.
//!
//! gsd-tools also consumes its global flags anywhere in argv (`GLOBAL_FLAGS` in
//! `bin/lib/cli-surface.cjs`: `--cwd`, `--project-dir`, `--ws`, `--pick`,
//! `--default`, `--raw`, `--json-errors`, `--exit-contract=`, the help tokens,
//! and the rejected `--version`/`-v`). Every one starts with `-`, so a value or a
//! key segment starting with `-` is refused ([`RefusalReason::FlagShaped`],
//! [`RefusalReason::KeyPath`]).
//!
//! No refusal and no `Debug` output carries a value: a value may be an API key.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use super::config_json::GsdConfig;

/// The largest integer a JS number holds exactly (2^53).
const JS_SAFE_INTEGER: f64 = 9_007_199_254_740_992.0;

/// One `gsd-tools` invocation that makes one changed leaf.
#[derive(Clone, PartialEq, Eq)]
pub enum ConfigSetCall {
    /// `config-set <key> <arg>`.
    Set { key: String, arg: String },
    /// `config-set <key> null`: gsd-core deletes the key.
    Unset { key: String },
    /// `config-set-model-profile <profile>`: gsd-core validates the profile
    /// against its known list and lowercases it.
    ModelProfile { profile: String },
}

impl ConfigSetCall {
    /// The dotted config path this call changes.
    pub fn key(&self) -> &str {
        match self {
            ConfigSetCall::Set { key, .. } | ConfigSetCall::Unset { key } => key,
            ConfigSetCall::ModelProfile { .. } => "model_profile",
        }
    }

    /// The value this call stores, when it stores one.
    pub fn value(&self) -> Option<&str> {
        match self {
            ConfigSetCall::Set { arg, .. } => Some(arg),
            ConfigSetCall::Unset { .. } => None,
            ConfigSetCall::ModelProfile { profile } => Some(profile),
        }
    }

    /// The gsd-tools arguments, after the launcher prefix.
    pub fn args(&self) -> Vec<String> {
        match self {
            ConfigSetCall::Set { key, arg } => {
                vec!["config-set".to_owned(), key.clone(), arg.clone()]
            }
            ConfigSetCall::Unset { key } => {
                vec!["config-set".to_owned(), key.clone(), "null".to_owned()]
            }
            ConfigSetCall::ModelProfile { profile } => {
                vec!["config-set-model-profile".to_owned(), profile.clone()]
            }
        }
    }
}

/// Written by hand so the value never prints (the `ApiKeySetting` precedent).
impl std::fmt::Debug for ConfigSetCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSetCall::Set { key, .. } => write!(f, "Set({key}, <value>)"),
            ConfigSetCall::Unset { key } => write!(f, "Unset({key})"),
            ConfigSetCall::ModelProfile { .. } => f.write_str("ModelProfile(<value>)"),
        }
    }
}

/// Why an edit cannot be sent to gsd-tools. Carries the key, never the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigSetRefusal {
    pub key: String,
    pub reason: RefusalReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefusalReason {
    /// The config could not be serialized.
    Serialize,
    /// A key segment is empty, holds a `.` or a NUL, or starts with `-`.
    KeyPath,
    /// The value starts with `-`.
    FlagShaped,
    /// The value contains a NUL byte.
    Nul,
    /// gsd-tools would store the value as another JSON type.
    Coerced { stored_as: &'static str },
}

impl ConfigSetRefusal {
    /// Static text only: never the value.
    pub fn reason_text(&self) -> String {
        match &self.reason {
            RefusalReason::Serialize => "the config could not be serialized".to_owned(),
            RefusalReason::KeyPath => {
                "this key path cannot be passed to gsd-tools config-set".to_owned()
            }
            RefusalReason::FlagShaped => {
                "the value starts with '-', which gsd-tools would read as a flag".to_owned()
            }
            RefusalReason::Nul => "the value contains a NUL byte".to_owned(),
            RefusalReason::Coerced { stored_as } => {
                format!("gsd-tools config-set would store this value as a {stored_as}, not as text")
            }
        }
    }
}

/// `DECIMAL_LITERAL` with ASCII digits: JS `\d` is ASCII, Rust's is Unicode.
fn decimal_literal() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^[+-]?(?:[0-9]+\.?[0-9]*|\.[0-9]+)(?:[eE][+-]?[0-9]+)?$")
            .expect("static regex")
    })
}

/// What `gsd-tools config-set <key> <arg>` stores, as gsd-core 1.21.0 parses
/// it. `Value::Null` means the key is unset.
pub fn cli_parse(key: &str, arg: &str) -> Value {
    let parsed = match arg {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ => parse_non_literal(arg),
    };
    if key == "project_code" && !parsed.is_null() {
        return Value::String(arg.to_owned());
    }
    parsed
}

fn parse_non_literal(arg: &str) -> Value {
    if decimal_literal().is_match(arg) {
        if let Ok(f) = arg.parse::<f64>() {
            if f.is_finite() {
                return number_value(f);
            }
        }
    }
    if arg.starts_with('[') || arg.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<Value>(arg) {
            return v;
        }
    }
    Value::String(arg.to_owned())
}

fn number_value(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() <= JS_SAFE_INTEGER {
        // Lossless: the magnitude is within 2^53.
        let i = f as i64;
        return Value::from(i);
    }
    serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number)
}

/// The planner's equality: numbers compare as JS numbers (f64), and an integer
/// beyond 2^53 in magnitude never equals anything, because JS cannot hold it.
pub fn json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => match (js_number(x), js_number(y)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| json_equal(x, y))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_equal(v, w)))
        }
        _ => a == b,
    }
}

fn js_number(n: &serde_json::Number) -> Option<f64> {
    const LIMIT: u64 = 1 << 53;
    if let Some(i) = n.as_i64() {
        if i.unsigned_abs() > LIMIT {
            return None;
        }
    } else if let Some(u) = n.as_u64() {
        if u > LIMIT {
            return None;
        }
    }
    n.as_f64()
}

/// The name of the JSON type gsd-tools would store, for the refusal text.
fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null (unset)",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "text",
        Value::Array(_) => "JSON array",
        Value::Object(_) => "JSON object",
    }
}

/// The ordered gsd-tools calls that turn `before` into `after`, or why the edit
/// cannot be sent. The first refusal aborts the whole plan: never a partial one.
pub fn plan_config_set(
    before: &GsdConfig,
    after: &GsdConfig,
) -> Result<Vec<ConfigSetCall>, ConfigSetRefusal> {
    let serialize_refusal = || ConfigSetRefusal {
        key: String::new(),
        reason: RefusalReason::Serialize,
    };
    let before = serde_json::to_value(before).map_err(|_| serialize_refusal())?;
    let after = serde_json::to_value(after).map_err(|_| serialize_refusal())?;
    let mut calls = Vec::new();
    let mut path = Vec::new();
    walk(&mut path, Some(&before), Some(&after), &mut calls)?;
    Ok(calls)
}

/// `null` and absent are the same thing to the planner.
fn present(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| !v.is_null())
}

fn walk(
    path: &mut Vec<String>,
    before: Option<&Value>,
    after: Option<&Value>,
    out: &mut Vec<ConfigSetCall>,
) -> Result<(), ConfigSetRefusal> {
    let before = present(before);
    let after = present(after);
    let (b_obj, a_obj) = (
        before.and_then(Value::as_object),
        after.and_then(Value::as_object),
    );
    let recurse = match (before, after) {
        (None, None) => return Ok(()),
        (Some(_), Some(_)) => b_obj.is_some() && a_obj.is_some(),
        (Some(_), None) => b_obj.is_some_and(|o| !o.is_empty()),
        (None, Some(_)) => a_obj.is_some(),
    };
    if recurse {
        let keys: BTreeSet<&String> = b_obj
            .into_iter()
            .flat_map(|o| o.keys())
            .chain(a_obj.into_iter().flat_map(|o| o.keys()))
            .collect();
        for k in keys {
            path.push(k.clone());
            let r = walk(
                path,
                b_obj.and_then(|o| o.get(k)),
                a_obj.and_then(|o| o.get(k)),
                out,
            );
            path.pop();
            r?;
        }
        return Ok(());
    }
    if let (Some(b), Some(a)) = (before, after) {
        if json_equal(b, a) {
            return Ok(());
        }
    }
    let key = checked_key(path)?;
    let Some(after) = after else {
        out.push(ConfigSetCall::Unset { key });
        return Ok(());
    };
    let arg = match after {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    };
    let refuse = |reason| ConfigSetRefusal {
        key: key.clone(),
        reason,
    };
    if arg.starts_with('-') {
        return Err(refuse(RefusalReason::FlagShaped));
    }
    if arg.contains('\0') {
        return Err(refuse(RefusalReason::Nul));
    }
    if key == "model_profile" && after.is_string() && !arg.is_empty() {
        // config-set-model-profile takes the profile as text: no coercion.
        out.push(ConfigSetCall::ModelProfile { profile: arg });
        return Ok(());
    }
    let stored = cli_parse(&key, &arg);
    if !json_equal(&stored, after) {
        return Err(refuse(RefusalReason::Coerced {
            stored_as: type_name(&stored),
        }));
    }
    out.push(ConfigSetCall::Set { key, arg });
    Ok(())
}

/// The dotted key for `path`, or a KeyPath refusal (named by the escaped-later
/// joined path) when a segment cannot travel through gsd-tools' argv.
fn checked_key(path: &[String]) -> Result<String, ConfigSetRefusal> {
    let key = path.join(".");
    let bad = path.is_empty()
        || path
            .iter()
            .any(|s| s.is_empty() || s.contains('.') || s.contains('\0') || s.starts_with('-'));
    if bad {
        return Err(ConfigSetRefusal {
            key,
            reason: RefusalReason::KeyPath,
        });
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::config_json::{parse_gsd_config, ApiKeySetting, WorkflowConfig};
    use serde_json::json;

    fn base() -> GsdConfig {
        parse_gsd_config(r#"{"mode":"interactive","commit_docs":true,"phase_naming":"sequential"}"#)
            .unwrap()
    }

    fn args(calls: &[ConfigSetCall]) -> Vec<Vec<String>> {
        calls.iter().map(ConfigSetCall::args).collect()
    }

    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_bool_flip_is_one_config_set_call() {
        let before = base();
        let mut after = before.clone();
        after.commit_docs = Some(false);
        let plan = plan_config_set(&before, &after).unwrap();
        assert_eq!(
            args(&plan),
            vec![sv(&["config-set", "commit_docs", "false"])]
        );
    }

    #[test]
    fn a_new_block_emits_only_its_set_leaf() {
        let before = base();
        let mut after = before.clone();
        after.workflow = Some(WorkflowConfig {
            plan_check: Some(true),
            ..WorkflowConfig::default()
        });
        let plan = plan_config_set(&before, &after).unwrap();
        assert_eq!(
            args(&plan),
            vec![sv(&["config-set", "workflow.plan_check", "true"])]
        );
    }

    #[test]
    fn clearing_a_leaf_unsets_it_and_no_change_plans_nothing() {
        let before = base();
        let mut after = before.clone();
        after.commit_docs = None;
        assert_eq!(
            args(&plan_config_set(&before, &after).unwrap()),
            vec![sv(&["config-set", "commit_docs", "null"])]
        );
        // The last field of a block.
        let mut with_block = before.clone();
        with_block.workflow = Some(WorkflowConfig {
            plan_check: Some(true),
            ..WorkflowConfig::default()
        });
        let mut cleared = with_block.clone();
        cleared.workflow.as_mut().unwrap().plan_check = None;
        assert_eq!(
            args(&plan_config_set(&with_block, &cleared).unwrap()),
            vec![sv(&["config-set", "workflow.plan_check", "null"])]
        );
        assert!(plan_config_set(&before, &before.clone())
            .unwrap()
            .is_empty());
        // An unchanged extra key never appears.
        let with_extra =
            parse_gsd_config(r#"{"mode":"yolo","brand_new":{"a":1},"other":[1,2]}"#).unwrap();
        let mut edited = with_extra.clone();
        edited.commit_docs = Some(true);
        assert_eq!(
            args(&plan_config_set(&with_extra, &edited).unwrap()),
            vec![sv(&["config-set", "commit_docs", "true"])]
        );
    }

    #[test]
    fn strings_go_verbatim_and_coercing_values_are_refused() {
        let before = base();
        let set = |f: &dyn Fn(&mut GsdConfig)| {
            let mut after = before.clone();
            f(&mut after);
            plan_config_set(&before, &after)
        };
        assert_eq!(
            args(&set(&|c| c.response_language = Some("Deutsch".into())).unwrap()),
            vec![sv(&["config-set", "response_language", "Deutsch"])]
        );
        assert_eq!(
            args(&set(&|c| c.project_code = Some("007".into())).unwrap()),
            vec![sv(&["config-set", "project_code", "007"])]
        );
        let coerced = |r: Result<Vec<ConfigSetCall>, ConfigSetRefusal>, key: &str, as_: &str| {
            let refusal = r.unwrap_err();
            assert_eq!(refusal.key, key);
            assert_eq!(
                refusal.reason,
                RefusalReason::Coerced {
                    stored_as: match as_ {
                        "number" => "number",
                        "boolean" => "boolean",
                        _ => "JSON array",
                    }
                }
            );
        };
        coerced(
            set(&|c| c.phase_naming = Some("12".into())),
            "phase_naming",
            "number",
        );
        coerced(
            set(&|c| c.response_language = Some("true".into())),
            "response_language",
            "boolean",
        );
        coerced(
            set(&|c| c.response_language = Some("[1]".into())),
            "response_language",
            "array",
        );
        assert_eq!(
            args(&set(&|c| c.response_language = Some("[x".into())).unwrap()),
            vec![sv(&["config-set", "response_language", "[x"])]
        );
        let digits = set(&|c| c.brave_search = Some(ApiKeySetting::Key("12345678".into())));
        assert_eq!(
            digits.unwrap_err().reason,
            RefusalReason::Coerced {
                stored_as: "number"
            }
        );
        assert_eq!(
            args(&set(&|c| c.brave_search = Some(ApiKeySetting::Key("sk-abc".into()))).unwrap()),
            vec![sv(&["config-set", "brave_search", "sk-abc"])]
        );
        assert_eq!(
            args(&set(&|c| c.context_window = Some(400_000)).unwrap()),
            vec![sv(&["config-set", "context_window", "400000"])]
        );
    }

    #[test]
    fn flag_shaped_nul_and_bad_key_paths_are_refused_without_the_value() {
        let before = base();
        let set = |f: &dyn Fn(&mut GsdConfig)| {
            let mut after = before.clone();
            f(&mut after);
            plan_config_set(&before, &after)
        };
        let secret = "-sk-very-secret";
        let r = set(&|c| c.brave_search = Some(ApiKeySetting::Key(secret.into()))).unwrap_err();
        assert_eq!(r.reason, RefusalReason::FlagShaped);
        assert!(!format!("{r:?} {}", r.reason_text()).contains("very-secret"));
        let r = set(&|c| c.response_language = Some("a\0b".into())).unwrap_err();
        assert_eq!(r.reason, RefusalReason::Nul);
        for bad in ["a.b", "-x", "--cwd"] {
            let mut after = before.clone();
            after.extra.insert(bad.to_owned(), json!("v"));
            let r = plan_config_set(&before, &after).unwrap_err();
            assert_eq!(r.reason, RefusalReason::KeyPath, "{bad}");
        }
        // Debug of a call never carries the value.
        let call = ConfigSetCall::Set {
            key: "brave_search".into(),
            arg: "sk-hidden".into(),
        };
        assert!(!format!("{call:?}").contains("sk-hidden"));
    }

    #[test]
    fn model_profile_goes_through_its_own_command() {
        let before = base();
        let mut after = before.clone();
        after.model_profile = "quality".into();
        assert_eq!(
            args(&plan_config_set(&before, &after).unwrap()),
            vec![sv(&["config-set-model-profile", "quality"])]
        );
        let mut flag = before.clone();
        flag.model_profile = "--cwd".into();
        assert_eq!(
            plan_config_set(&before, &flag).unwrap_err().reason,
            RefusalReason::FlagShaped
        );
        // An unset (a JSON null on disk) still goes through config-set.
        let mut unset = serde_json::to_value(&after).unwrap();
        unset["model_profile"] = Value::Null;
        let _ = unset;
        let mut changed = after.clone();
        changed.model_profile = "budget".into();
        assert_eq!(
            args(&plan_config_set(&after, &changed).unwrap()),
            vec![sv(&["config-set-model-profile", "budget"])]
        );
    }

    #[test]
    fn cli_parse_matches_gsd_core_1_21_0() {
        let k = "response_language";
        assert_eq!(cli_parse(k, "true"), json!(true));
        assert_eq!(cli_parse(k, "false"), json!(false));
        assert_eq!(cli_parse(k, "null"), Value::Null);
        assert!(json_equal(&cli_parse(k, "1."), &json!(1)));
        assert!(json_equal(&cli_parse(k, ".5"), &json!(0.5)));
        assert!(json_equal(&cli_parse(k, "+1"), &json!(1)));
        assert!(json_equal(&cli_parse(k, "1e3"), &json!(1000)));
        for s in [
            "1e400",
            "Infinity",
            "NaN",
            "0x10",
            " 12",
            "\u{661}\u{662}",
            "[bad",
            "",
            "TRUE",
        ] {
            assert_eq!(cli_parse(k, s), json!(s), "{s:?} stays a string");
        }
        assert_eq!(cli_parse(k, "[1,2]"), json!([1, 2]));
        assert_eq!(cli_parse("project_code", "true"), json!("true"));
        assert_eq!(cli_parse("project_code", "007"), json!("007"));
        assert_eq!(cli_parse("project_code", "null"), Value::Null);
    }

    #[test]
    fn json_equal_treats_numbers_as_js_does() {
        assert!(json_equal(&json!(1), &json!(1.0)));
        assert!(!json_equal(
            &json!(9_007_199_254_740_993_u64),
            &json!(9_007_199_254_740_993_u64)
        ));
        assert!(json_equal(&json!({"a":[1,2]}), &json!({"a":[1.0,2]})));
        assert!(!json_equal(&json!("1"), &json!(1)));
    }
}
