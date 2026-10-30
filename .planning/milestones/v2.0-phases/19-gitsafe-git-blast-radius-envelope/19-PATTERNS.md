# Phase 19: GITSAFE — Git & Blast-Radius Envelope - Pattern Map

**Mapped:** 2026-08-17
**Files analyzed:** 17 (new + modified)
**Analogs found:** 16 / 17

Line numbers are as of HEAD `740e62f`, matching CONTEXT.md's `<code_context>`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/envelope/mod.rs` (new) | module root / doc contract | — | `src/driver/dry_run.rs:1-41`, `src/journal/redact.rs:1-57` | exact (module-doc-as-contract) |
| `src/envelope/policy.rs` (new) | pure decision fn | transform | `src/journal/mod.rs:198-232` (`is_plain_run_id`), `src/executor/outcome.rs`-style pure fns | exact |
| `src/envelope/scan.rs` (new) | utility / detector | file-I/O + transform | `src/journal/redact.rs:65-158` (`PARTS`/`RE`) | exact (reuses the table) |
| `src/envelope/hooks.rs` (new) | generator + subprocess entry | file-I/O | `src/journal/writer.rs:296-364` (`ensure_runs_root`/`create_run_dir`) | role-match |
| `src/envelope/cred.rs` (new) | env builder | process-env | `src/executor/claude.rs:418-435` (spawn closure `CLAUDE*` scrub) | exact |
| `src/envelope/ledger.rs` (new) | append-only store | event-driven / NDJSON append | `src/journal/writer.rs:59-130` (`JournalWriter::open`/`append`) | exact |
| `src/envelope/advisory.rs` (new) | probe + pinned text | request-response (shell-out) | `src/state_reader/git_ops.rs:172-238`, `src/driver/dry_run.rs:45-67` | exact |
| `src/cli.rs` (mod) | CLI parser | request-response | `src/cli.rs:47-152` (`Commands::Drive`) | exact |
| `src/main.rs` (mod) | subcommand dispatch | request-response | `src/main.rs:88-120` (`Drive` arm) | exact |
| `src/config.rs` (mod — `DriverOptIn` + `CredentialSource`) | model / config | CRUD | `src/config.rs:66-92` + `:118-158` + `:183-218` (`migrate`) | exact |
| `src/executor/mod.rs` (mod — `ExecutionOptions` fields) | model | — | `src/executor/mod.rs:276-332` | exact |
| `src/executor/claude.rs` (mod — `build_argv`, spawn closure) | service / spawn seam | process-spawn | `src/executor/claude.rs:223-267`, `:411-443` | exact |
| `src/driver/run.rs` (mod — single `ExecutionOptions` site) | service | — | `src/driver/run.rs:1005-1039` | exact |
| `src/driver/mod.rs` (mod — envelope assertion in refusal chain) | controller | request-response | `src/driver/mod.rs:191-295` (`drive`) | exact |
| `src/driver/dry_run.rs` (mod — honesty section) | view/render | transform | `src/driver/dry_run.rs:45-67`, `:111-182` | exact |
| `src/journal/mod.rs` (mod — `Parked` comment correction, reason taxonomy) | model | — | `src/journal/mod.rs:769-793` | exact |
| `src/journal/redact.rs` (mod — `SecretClass` split of `PARTS`) | utility | transform | `src/journal/redact.rs:65-158` | exact |
| `tests/async_blocking_guard.rs` (new) | source-scanning test | file-I/O | `tests/spawn_seam_guard.rs:1-130` | exact |
| `tests/envelope_*.rs` (new integration tests) | test | file-I/O + process | `tests/driver_dry_run.rs`, `tests/driver_optin.rs`, `tests/driver_lock.rs` | role-match |

---

## Pattern Assignments

### `src/envelope/mod.rs` (module root, doc-as-contract)

**Analog:** `src/driver/dry_run.rs:1-41` and `src/journal/redact.rs:1-57`.

Both open with a numbered rationale block naming the PITFALLS line each rule defends, plus a
"what this module will not grow" section. D-06's three-layer table and the
"this layer does NOT see …" sentence go here in exactly that shape.

**Module doc pattern** (`redact.rs:14-49`):
```rust
//! Three things about the pattern table are load-bearing and were settled by
//! executing it rather than by reading it:
//!
//! 1. **Order.** …
//! 2. **One alternation, one pass.** …
//! 3. **Idempotence is a test, not an argument.** …
//!
//! ## The honest limit (D-25)
//!
//! A pattern redactor **cannot** catch an arbitrary high-entropy secret …
//!
//! ## What this module will not grow (D-26)
//!
//! There is no raw sidecar file, no unredact path, and no verbose mode that
//! skips the filter.
```

`dry_run.rs:31-36` is the model for stating a mechanically-proved read-only claim:
```rust
//! **D-23: zero git writes and zero agent spawns, proved mechanically.** …
//! `tests/driver_dry_run.rs` proves it rather than asserting it:
//! `git reflog`, every ref and the whole `.git` directory listing are captured
//! before and after and compared, and a tripwire program supplied as the agent
//! leaves an evidence file if it is ever executed.
```

---

### `src/envelope/policy.rs` (pure decision functions, transform)

**Analog:** `src/journal/mod.rs:198-232` — `is_plain_run_id`. It is the project's model for a
pure, token-level, filesystem-free validator with the declined alternative recorded in the doc.

**Pure-validator pattern** (`journal/mod.rs:220-232`):
```rust
pub fn is_plain_run_id(run_id: &str) -> bool {
    if run_id.is_empty() {
        return false;
    }
    let mut components = Path::new(run_id).components();
    let Some(Component::Normal(name)) = components.next() else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }
    name == std::ffi::OsStr::new(run_id)
}
```
D-03 reuses this predicate for the `<alias>` path component (renaming it if it reads too
run-id-specific — the rename must sweep `src/journal/mod.rs:1551-1568`, `src/driver/mod.rs:~288`
and `tests/journal_run_paths.rs`).

**Fallible-by-signature pattern** (`journal/mod.rs:252-280`) — the shape `classify_git` and
`EnvelopePolicy::resolve` should copy when a caller must be conscripted by the compiler:
```rust
/// **The `Option` is the whole of the WR-02 fix, and its shape was chosen so a
/// human does not have to find the call sites** (D-27). …
/// A validation helper that merely *existed* would have been called at three of
/// the five sites, which is the failure mode this signature forecloses:
/// returning `Option` conscripts the compiler into enumerating every caller.
/// No infallible variant is kept alongside it, because keeping one is exactly
/// how the next caller escapes validation.
pub fn run_paths(planning_dir: &Path, run_id: &str) -> Option<RunPaths> {
    if !is_plain_run_id(run_id) { return None; }
    …
}
```

**Implicit-destination resolution:** do NOT re-derive. Call
`crate::state_reader::git_ops::push_refspecs(root) -> PushPreview` (`git_ops.rs:351`), which
already resolves `branch.<b>.remote`, `remote.pushDefault`, `remote.<r>.push` and
`push.default` with no network and no credential. `PushPreview { remote, url, refspecs, note }`
is constructed in `dry_run.rs:196-201`.

**Exhaustive-match tripwire** to preserve when adding a new enum (`claude.rs:226-230`):
```rust
// Exhaustive on purpose: Phase 22's `Container` variant must land here as a
// compile error rather than as a silently host-shaped argv.
match options.target {
    ExecutionTarget::Host => {}
}
```

---

### `src/envelope/scan.rs` (detector, file-I/O + transform)

**Analog:** `src/journal/redact.rs:65-158`. **Reuse `PARTS`; do not fork it** (D-11/D-12).

**The table and its compiled alternation** (`redact.rs:70-158`) — the exact structure the
`SecretClass` split edits:
```rust
/// `(group name, pattern, replacement literal)`, **in significant order**.
const PARTS: &[(&str, &str, &str)] = &[
    ("pem",  r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----", "[REDACTED:private-key]"),
    ("authz", r#"(?i:authorization)[ \t]*[:=][ \t]*"?[^\r\n"]{4,}"#, "[REDACTED:authorization]"),
    …
    // ---- WR-15: the DASH-ENCODED forms must come BEFORE the slash forms.
    ("dhome", r"-(?:home|Users|root)-[^/\s\\\x22]*", "-home-redacted-project"),
    ("shome", r#"/(?:home|Users|var/home)/[^/\s"':,)\[\]}\\]+"#, "/home/[REDACTED:user]"),
];

static RE: LazyLock<Regex> = LazyLock::new(|| {
    let alt = PARTS
        .iter()
        .map(|(name, pattern, _)| format!("(?P<{name}>{pattern})"))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&alt).expect("redaction alternation must compile")
});
```
The tuple becomes a 4-tuple or a struct with `SecretClass`. **Order is load-bearing**
(`redact.rs:124-127` WR-15 note): a naive reorder regressed seven of eight fixtures. The
scanner's own alternation must be built from the `Credential`-tagged subset only, and the
existing idempotence test over the corpus must still pass unchanged.

**Named-group match identification** — the scanner needs the rule *name* (D-15: report file,
line, rule name, never the matched text). `redact.rs` already uses `Captures` with the
`(?P<name>…)` groups from the same builder; reuse that mechanism rather than re-scanning
per rule.

**Skip-list-is-part-of-the-result (D-14):** no direct analog. The nearest posture is
`PushPreview.note` (`git_ops.rs`, rendered at `dry_run.rs:163-177`), where "nothing to report"
is data carried in the same struct and always printed:
```rust
if report.push.refspecs.is_empty() {
    match &report.push.note {
        Some(note) => lines.push(format!("  {note}")),
        None => lines.push("  No push would occur from this state.".to_string()),
    }
}
```

---

### `src/envelope/hooks.rs` (generator + subprocess entry, file-I/O)

**Analog:** `src/journal/writer.rs:296-364` (`ensure_runs_root`, `create_run_dir`,
`parent_excludes_run_record`) for directory-tree creation with `anyhow::Context`, and
`src/config.rs:234-249` for the atomic-write idiom the `gitconfig`/`settings.json` generation
should copy:

```rust
pub fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir).context("Failed to create temp file for config")?;
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    tmp.write_all(json.as_bytes())?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;
    Ok(())
}
```
D-07's write-then-deserialize-and-compare check for `settings.json` sits immediately after the
`persist`, and refuses the run on mismatch.

**Envelope base directory** — copy `src/main.rs:19-21` verbatim in shape (D-02):
```rust
let log_dir = dirs::data_local_dir()
    .map(|d| d.join("gsd-meta-manager"))
    .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
```
Note the fallback differs from `Config::default_path` (`config.rs:167-172`, which falls back to
a relative `.config`). For the envelope, a fallback that lands inside a repo would defeat D-02 —
decide the fallback explicitly and say so.

**Spawn-allowlist obligation:** any file under `src/` that shells out must be added to
`SPAWN_ALLOWLIST` in `tests/spawn_seam_guard.rs:37-70` **in the same commit**. The markers
scanned are `Command::new(`, `CommandWrap::with_new(`, `process_group(`
(`spawn_seam_guard.rs:73`). Expect `src/envelope/hooks.rs`, `scan.rs`, `advisory.rs` and
`cred.rs` to need entries with a one-line "No agent" justification, matching the existing
comment style:
```rust
// Git reads that back project state. No agent.
"src/state_reader/git_ops.rs",
```

---

### `src/envelope/cred.rs` (child environment builder, process-env)

**Analog:** `src/executor/claude.rs:418-435` — the one place the child's environment is built.
The D-16 scrub extends this block; nothing else in the tree may set child env.

```rust
let mut wrap = CommandWrap::with_new(&program, |cmd| {
    cmd.args(&argv)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // The TUI is plausibly launched from inside a Claude Code session,
    // so inherited CLAUDE* variables would leak into the driven child
    // and change `-p` behaviour in ways that look like "works on my
    // machine". Scrub them all, then set the one we mean to set.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("CLAUDE") {
            cmd.env_remove(&key);
        }
    }
    cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);
});
wrap.wrap(ProcessGroup::leader());
wrap.wrap(KillOnDrop);
```
The scrub-then-set shape (remove first, set second) is the pattern for `SSH_AUTH_SOCK`/
`SSH_AGENT_PID` removal followed by `GIT_SSH_COMMAND`, `GIT_CONFIG_GLOBAL`,
`GIT_CONFIG_SYSTEM`, `GIT_TERMINAL_PROMPT=0`, `GH_CONFIG_DIR` and the
`GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n` triplet.

**Preferred factoring for D-32:** build the env as a pure `Vec<(OsString, Option<OsString>)>`
(or a typed `EnvelopeEnv`) in `cred.rs` and have the closure apply it, so
"assert on the constructed child environment" is a unit test with no spawn — the same reason
`build_argv` is a free function separate from the spawn closure.

---

### `src/envelope/ledger.rs` (append-only NDJSON, event-driven)

**Analog:** `src/journal/writer.rs:59-130`.

```rust
pub fn open(journal_path: &Path) -> anyhow::Result<Self> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal_path)
        .with_context(|| format!("Failed to open journal at {}", journal_path.display()))?;
    Ok(Self { file, path: journal_path.to_path_buf(), next_seq: 1, bytes_written: 0, … })
}
```
`create(true).append(true)` plus one-JSON-object-per-line is the shipped NDJSON discipline;
D-19's torn-write tolerance follows from it. The reader side's tolerant-parse posture is
`src/journal/reader.rs:196-211` (`JournalRecord.rest`) — a malformed trailing line is skipped,
not fatal.

**Write-before-permit (D-20):** the ordering rationale belongs in a doc comment in the shape of
`writer.rs:101-107`:
```rust
/// **The asymmetry at the cap is the decision (D-31).** …
/// but keeps writing every lifecycle, decision, diagnostic and
/// outcome event forever, because the run must always be able to write its
/// terminal record, and silently dropping the ending is the one failure
/// OBS-01 cannot tolerate.
```

---

### `src/envelope/advisory.rs` (remote probe + pinned honesty text)

**Analog A — the read-only shell-out:** `src/state_reader/git_ops.rs:189-238`.
```rust
fn git_read_raw(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() { return None; }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
```
`--no-optional-locks` is load-bearing (`git_ops.rs:174-181`: `git diff` opportunistically writes
`.git/index`). Every envelope git read must carry it. Failure is reported as data (`None` /
`unknown(reason)`), never by unwinding — which is exactly D-26's `protected | unprotected |
unknown(reason)`.

**Analog B — the pinned constant with a test:** `src/driver/dry_run.rs:45-67`.
```rust
/// The refspecs header, and the pinned-contract rule for all three.
///
/// **These three constants are a contract, not decoration.**
/// `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs`
/// asserts all three appear, in this order, so a section cannot silently
/// disappear the way PITFALLS:69 warns about. Changing their text is a
/// user-visible output change and breaks anyone scripting against the preview.
pub const SECTION_REFSPECS: &str = "== Push refspecs this state would produce ==\n\
    Computed locally from git config (branch.<b>.remote, remote.pushDefault,\n\
    remote.<r>.push, push.default). No network was contacted, no credential was\n\
    used, and git's push subcommand was never invoked in any form.";
```
And its ordering test (`dry_run.rs:205-223`):
```rust
#[test]
fn the_rendered_report_carries_all_three_section_headers_in_order() {
    let rendered = render(&report());
    let commands = rendered.find(SECTION_COMMANDS).expect("the commands section must appear");
    …
    assert!(commands < diffstat && diffstat < refspecs, "…fixed order; got offsets …");
}
```
D-27's honesty constant is `SECTION_ENVELOPE` (or similar) added to this same set, with the
existing order test extended to four sections in the same commit.

---

### `src/cli.rs` — the hidden `envelope` subcommand (D-04)

**Analog:** `src/cli.rs:39-56` (`Commands::Drive`). Two conventions are non-negotiable here:

```rust
    // Every `///` below is rendered verbatim by clap as help text, so the
    // rationale for each field lives in `//` comments like this one and only the
    // one-line description a user needs is a doc comment.
    //
    // The variant carries no `#[cfg]` on purpose (D-05): driving is Unix-only,
    // but the subcommand parses on every platform and the *handler* returns a
    // typed unsupported-platform error. An accepted limitation that surfaces as
    // "unknown subcommand" is indistinguishable from a bug.
    Drive {
        alias: String,
        #[arg(long)]
        command: String,
```
1. Rationale in `//`, user-facing text in `///`.
2. `hide = true` removes a flag from `--help`, **not from the parser** (`cli.rs:107-116`). The
   `envelope` subcommand is re-entered by hooks in a *release* build, so it must NOT be
   `#[cfg(debug_assertions)]` — but it is also a policy-enforcement entry point rather than a
   privileged escape hatch, so `hide = true` alone is the right treatment. Say so explicitly in
   the `//` comment, because `cli.rs` currently teaches the opposite lesson about `hide`.
3. Validation of a hand-typed value lives in the handler, not in a `value_parser`
   (`cli.rs:64-70`) — the same reason applies to `<alias>` (D-03).

**`src/main.rs` dispatch** — copy the `Drive` arm's error posture (`main.rs:114-119`):
```rust
if let Err(err) = drive(args, &config).await {
    // The `Add` arm's house shape: user-facing refusals in this
    // binary print and exit, they do not bubble as an anyhow chain.
    eprintln!("Error: {}", err);
    std::process::exit(1);
}
```
A hook subcommand's exit code **is** the control (D-25), so the envelope arms exit with an
explicit non-zero on refusal, before `tui::init()` is ever reached — the same "position in the
match is the mechanism" argument recorded at `main.rs:99-101`.

---

### `src/config.rs` — `DriverOptIn` additions + `CredentialSource` (D-18, D-30)

**Analog:** `src/config.rs:66-92` and the `#[serde(default = "…")]` treatment at `:100-158`.

**Field-with-migration pattern** (`config.rs:35-47`):
```rust
    /// The user's driver opt-in, or `None` for a project that may only be read.
    ///
    /// `#[serde(default)]` — the first serde attribute on this struct — makes
    /// every pre-Phase-17 `config.json` load unchanged, with every project
    /// coming back not-opted-in.
    #[serde(default)]
    pub driver_opt_in: Option<DriverOptIn>,
```

**The default-that-must-not-be-derived lesson** (`config.rs:109-117`) applies to every new
numeric cap. `pr_cap_per_24h`/`pr_cap_per_run` are `Option<u32>` per D-30 with resolution in
`EnvelopePolicy::resolve`, which sidesteps the derived-zero trap — but the two-armed test is
still the model:
```rust
#[test]
fn driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json() {
    assert_eq!(Preferences::default().driver_max_concurrent, 1,
        "a derived `Default` would make this 0, which means 'no run may ever start' (D-18)");
    assert_eq!(
        serde_json::from_str::<Preferences>("{}").expect("…").driver_max_concurrent, 1,
        "an absent key must default to one via `default_driver_max_concurrent`");
}
```

**The migration test is a byte-literal, never a serialised struct** (`config.rs:255-281`):
```rust
    /// **Deliberately a literal, never a serialised `Config`.** Serialising a
    /// `Config` and reading it back tests the round-trip; only a literal written
    /// the way the old binary wrote it tests the *migration* …
    const PRE_PHASE_17_CONFIG: &str = r#"{ "version": 1, "projects": { … } }"#;
```
Phase 19 adds a `PRE_PHASE_19_CONFIG` literal (a v2 config with a `driver_opt_in` carrying only
`opted_in_at`/`claude_md_digest`) and asserts it loads with every new field absent/`None`.
`CONFIG_SCHEMA_VERSION` (`config.rs:21`) does **not** bump — the doc there says version 2 means
exactly one thing, and `#[serde(default)]` fields need no version.

---

### `src/executor/claude.rs` — `build_argv` + spawn closure

**Analog:** itself, `claude.rs:223-267`.
```rust
pub fn build_argv(options: &ExecutionOptions) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();
    match options.target { ExecutionTarget::Host => {} }
    push(&mut argv, "-p");
    …
    push(&mut argv, "--strict-mcp-config");

    if let Some(model) = &options.model {
        push(&mut argv, "--model");
        push(&mut argv, model);
    }
    …
}

fn push(argv: &mut Vec<OsString>, arg: impl AsRef<OsStr>) {
    argv.push(arg.as_ref().to_os_string());
}
```
Rules the envelope's `--disallowedTools` / `--settings` additions inherit: unconditional flags
first in a fixed order, optional flags as `if let Some(...)` blocks after, always
flag-then-value as two pushes, never a shell string. **The pinned argv vector test at
`claude.rs:1719-1731` and `spawn.rs:~290` must be extended in the same commit** (D-28).

---

### `src/driver/mod.rs` — the envelope assertion in the refusal chain

**Analog:** `src/driver/mod.rs:191-295` (`drive`). The invariant to preserve: **exactly one
`DrivableProject::from_registry` call site**, checked by `tests/spawn_seam_guard.rs`.

```rust
    let project = DrivableProject::from_registry(&args.alias, entry)?;

    if args.dry_run { … }

    // Both refusals below are positioned, and the position is the decision.
    if let Some(refusal) = platform_refusal(liveness::LIVENESS_SUPPORTED, args.dry_run) {
        return Err(refusal);
    }
    let Some(run_id) = args.run_id.as_deref() else { return Err(DriveError::RunIdRequired) };
    if !journal::is_plain_run_id(run_id) {
        return Err(DriveError::RunIdInvalid { run_id: run_id.to_string() });
    }
    dispatch(project, &args, entry).await
```
An envelope assertion (`envelope_assertion_failed`, D-24) goes into this chain as a new
positioned refusal with a `//` comment stating *why* it sits where it sits. Take
`&DrivableProject`, never a bare path (`executor/mod.rs:136-174`) — that is what makes "the
envelope only ever runs for an opted-in project" type-level.

**Blocking-work-in-`async fn` pattern** — every synchronous git shell-out this phase adds on an
async path copies `driver/mod.rs`'s dry-run arm verbatim in shape:
```rust
    let cloned = project.clone();
    let report = match tokio::task::spawn_blocking(move || dry_run::build_report(&cloned, &command)).await {
        Ok(report) => report,
        Err(err) => {
            tracing::warn!(panicked = err.is_panic(), "the dry-run report task did not run to completion");
            dry_run::build_report(&project, &args.command)
        }
    };
```
Note the **clone, not move** — a second token construction would be a second `from_registry`
call site.

---

### `src/journal/mod.rs` — first emitter of `Parked` (D-24)

**Analog:** `src/journal/mod.rs:769-793`. The comment to correct, verbatim:
```rust
    /// The run stopped short and needs a human.
    ///
    /// **Schema only in this phase — Phase 20 emits it** (D-36).
    Parked {
        /// Why the run parked.
        reason: String,
        /// What would unpark it.
        needs: String,
    },
```
The reason taxonomy is a `&[&str]` or enum-with-`as_str` next to the variant, in the style of
`JournalEvent::Diagnostic { code, detail }` (`:771-776`) whose `code` is described as "a short
stable identifier for the condition". Emission goes through
`JournalWriter::append` (`writer.rs:111`), which returns the `seq`.

---

### `tests/async_blocking_guard.rs` (D-29)

**Analog:** `tests/spawn_seam_guard.rs:1-130`, copied structurally.

**Header pattern** (`spawn_seam_guard.rs:1-17`):
```rust
// ============================================================================
// The mechanical spawn-seam audit (D-17, PITFALLS:521)
//
// One rule governs every assertion below: **a comment is not a guard; the test
// is.** …
// It is an integration test rather than an in-source one because it reads the
// source tree, and a test that walks `src/` has no business living inside it.
// **The walk covers `src/` only, never `tests/`.** That is what lets this file's
// own prose name the tokens it forbids without invalidating its own gate.
// ============================================================================

const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
```

**Allowlist-with-per-entry-justification** (`:30-70`) — the model for D-29's deliberate-case
allowlist:
```rust
/// **This is a declared allowlist, not a habit.** A later plan in this phase
/// adds `src/driver/spawn.rs`; whoever adds it must add the entry here in the
/// same commit, and that deliberate edit is the entire point …
const SPAWN_ALLOWLIST: &[&str] = &[
    // The agent spawn. The one that takes the capability type.
    "src/executor/claude.rs",
    …
];
```

**Word-boundary matching, not `contains`** (`:75-104`) — D-29's scanner must copy
`calls_marker` rather than substring-match, for the reason recorded there
(`rustix::process::kill_process_group(` contains `process_group(`):
```rust
fn calls_marker(line: &str, marker: &str) -> bool {
    let mut from = 0;
    while let Some(offset) = line[from..].find(marker) {
        let at = from + offset;
        let preceded_by_identifier = line[..at].chars().next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !preceded_by_identifier { return true; }
        from = at + marker.len();
    }
    false
}
```
D-29 additionally requires the file's own doc comment to state **"it is a lint, not a proof"** —
a lexical scanner cannot see through a helper function. The observed-not-theorised justification
to cite is `tests/driver_lock.rs:201-215`.

---

## Shared Patterns

### Rationale-in-code, not in the plan
**Source:** every file read for this map.
**Apply to:** all new files.
Every non-obvious decision carries a `//` or `///` comment naming the decision id (D-nn/WR-nn/
CR-nn), the failure it prevents, and — where applicable — the *declined alternative* and why.
`config.rs:109-117`, `journal/mod.rs:214-219` and `redact.rs:29-49` are the three best examples.
Plans in this phase should require it explicitly; it is the single most consistent convention in
this codebase.

### "A comment is not a guard; the test is"
**Source:** `src/executor/mod.rs:132-135`, `tests/spawn_seam_guard.rs:4-8`.
**Apply to:** every claim Phase 19 makes about what cannot happen.
Where a plan asserts a property (single call site, no leftover file, no `bypassPermissions`),
the plan must name the test that holds it, and the doc comment must name that test back.

### Test naming
**Source:** `dry_run.rs:206`, `config.rs:400`, `journal/writer.rs` tests.
**Apply to:** all new tests.
Full-sentence snake_case names asserting the property:
`the_rendered_report_carries_all_three_section_headers_in_order`,
`a_pre_phase_17_config_loads_with_every_project_not_opted_in`,
`driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`.
Assertion messages state the *consequence*, not the expectation:
```rust
"reading an old config must never enrol a project into being driven, but '{alias}' came back opted in (D-15, FEATURES:236)"
```

### Fixtures
**Source:** `dry_run.rs:262-275`, `config.rs:306-310`.
**Apply to:** all new tests touching disk.
`tempfile::TempDir::new()` / `tempfile::tempdir()`, a `RegisteredProject` literal, then
`DrivableProject::from_registry` — **the production constructor**, deliberately, because
`for_testing_bypassing_opt_in` is fenced out of `src/`:
```rust
let project = DrivableProject::from_registry("preview", &entry).expect("an opted-in real directory");
```
Adding fields to `DriverOptIn` breaks these literals across `src/` and `tests/` — the same
"breakage is the feature" property `config.rs:41-45` records. Expect to update
`src/driver/dry_run.rs:266-269` and every `DriverOptIn { … }` literal under `tests/`.

### Error handling
**Source:** `src/config.rs` (`anyhow::Context` on every I/O), `src/driver/mod.rs` (typed
`DriveError`/`OptInError` for user-facing refusals), `src/state_reader/git_ops.rs` (`Option`,
failure-as-data, never panics).
**Apply to:** `hooks.rs`/`ledger.rs` → `anyhow::Result` with `.with_context(|| format!("… {}", path.display()))`;
`policy.rs`/`advisory.rs` → typed verdict / `Option`, no unwinding;
`driver/mod.rs` refusals → a `DriveError` variant, printed by `main.rs` and exited non-zero.

### Read-only git
**Source:** `src/state_reader/git_ops.rs:189-196`.
**Apply to:** every git invocation the envelope makes outside the hook path —
`git --no-optional-locks -C <root> …`, `.output()`, `None` on non-zero.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| the `PreToolUse` guard entry in `src/envelope/hooks.rs`/`policy.rs` | subprocess entry | request-response over stdin JSON | Nothing in the tree reads a JSON request on stdin and answers with a permission verdict. The closest structural relatives are `src/journal/inbox.rs` (an NDJSON channel between processes) for the line protocol and `src/executor/stream_json.rs` for tolerant JSON-line deserialisation with unknown fields preserved — consult both for shape, but the guard's contract comes from the `claude` hook spec, not from this codebase. Note `src/executor/mod.rs:225-239`'s reproduced 180–240 s `PreToolUse` hang: the guard's own timeout is a first-class requirement, not a nicety. |
| the `file://` bare-remote fixture (D-31) | test harness | process + file-I/O | `tests/driver_dry_run.rs` builds repos in a tempdir and snapshots `.git`, and `src/project_creator.rs` shells out `git init` — but no test in the tree creates a bare remote and pushes to it. Build it as a shared helper in the new integration test file; `tests/driver_dry_run.rs`'s before/after `.git` snapshot is the right companion for proving the envelope's read-only claims. |

---

## Metadata

**Analog search scope:** `src/` (all 62 modules), `tests/` (16 integration test files)
**Files read for excerpts:** `src/cli.rs`, `src/main.rs`, `src/config.rs`, `src/driver/dry_run.rs`,
`src/driver/mod.rs` (drive), `src/driver/run.rs` (options site), `src/executor/mod.rs`
(`DrivableProject`, `ExecutionOptions`), `src/executor/claude.rs` (`build_argv`, spawn closure),
`src/journal/mod.rs` (`is_plain_run_id`, `run_paths`, `Parked`), `src/journal/writer.rs`
(`JournalWriter`), `src/journal/redact.rs` (`PARTS`, `RE`), `src/state_reader/git_ops.rs`
(read helpers), `tests/spawn_seam_guard.rs`
**Pattern extraction date:** 2026-08-17
