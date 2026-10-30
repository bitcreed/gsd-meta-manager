# Phase 21: LLM Goal Layer & Prompt-Injection Hardening - Pattern Map

**Mapped:** 2026-08-19
**Files analyzed:** 14 (7 create, 7 modify)
**Analogs found:** 13 / 14

Phase 20 is the freshest analog source and is weighted accordingly: `router.rs`, `bounds.rs`,
`rate_limit.rs`, `run.rs`'s `Terminal`, and `dry_run.rs`'s pinned-honesty test all shipped or were
revised in it. Its code review found a Critical hiding behind a guard test that compared two
constants instead of the resolved value; the counter-pattern that came out of that review is quoted
below (§ Shared Pattern D) and every guard this phase adds must follow it.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/driver/escalate.rs` (NEW) | pure classifier + taxonomy | request-response (over a transport payload) | `src/driver/rate_limit.rs` | exact |
| `src/driver/goal.rs` (NEW) | pure validator + refusal taxonomy | transform | `src/driver/router.rs` + `src/driver/bounds.rs::resolve` | exact |
| `src/driver/untrusted.rs` (NEW) | utility (prompt builder) | transform | `src/driver/dry_run.rs` (pinned text) + `envelope/policy.rs` | role-match |
| `src/executor/claude.rs` (MOD) | spawn seam / argv+env builder | process spawn | itself (`build_argv` `:239-296`, spawn closure `:450-497`) | in-file |
| `src/executor/stream_json.rs` (MOD) | wire model | transform | its own tolerant-parse posture | in-file |
| `src/executor/outcome.rs` (MOD) | classifier | transform | its existing `(subtype, terminal_reason)` arms | in-file |
| `src/registry.rs` + `src/journal/mod.rs` (MOD) | digest utility | file-I/O | `journal::argv_digest` `:543-554` | exact |
| `src/config.rs` (MOD, `DriverOptIn`) | model / config record | CRUD | `DriverOptIn` `:78-124` itself (every field after the first is `#[serde(default)]`) | in-file |
| `src/ui/screens/driver_confirm.rs` (MOD) | TUI screen | request-response | itself + `dry_run::render` section contract | role-match |
| `src/driver/run.rs` (MOD) | orchestrator loop | event-driven | its own `'iterations` loop `:1901-1948` | in-file |
| `src/driver/mod.rs` / `src/cli.rs` (MOD) | seam / CLI args | request-response | `bounds::resolve` call at `mod.rs:396`, `cli.rs:73-127` | exact |
| `tests/driver_injection_corpus.rs` (NEW) | test (e2e, hostile fixture) | file-I/O + subprocess | `tests/envelope_hook_refusals.rs` + `tests/envelope_wiring.rs` | role-match |
| `tests/driver_escalation_cap.rs` (NEW) | test | event-driven | `tests/driver_rate_limit.rs` / `tests/envelope_wiring.rs::parked_events` | exact |
| `tests/spawn_seam_guard.rs` (MOD) | test (source-scanning guard) | batch | itself `:399-440` | in-file |
| `Cargo.toml` (MOD, `sha2`) | config | — | the `process-wrap` / `rustix` comment convention `:36-50` | exact |

---

## Pattern Assignments

### `src/driver/escalate.rs` (NEW — the fifth sibling taxonomy)

**Analog:** `src/driver/rate_limit.rs` (the fourth sibling; it records this exact "should it be a
fifth `BoundsReason` arm?" question being asked and answered).

**Module-doc pattern — purity, the injected clock, and the sibling declaration** (`rate_limit.rs:1-33`):

```rust
//! The subscription quota signal: classifying one retained `rate_limit_event`.
//!
//! **This module performs no I/O of its own and reads no clock**, in the same
//! register and for the same reason as [`super::bounds`] and [`super::router`]:
//! the caller retains the payload off the stream and hands in the instant to
//! judge a reset time against (D-11). That is what makes the sanity bound below
//! testable one second either side without waiting thirty days...
//!
//! **The payload is untrusted input.** It arrives from a process that itself
//! consumed untrusted repository content, so everything here is tolerant
//! parsing: no strict unknown-field rejection ... and no `unwrap` on a wire field.
//!
//! **The reason vocabulary is closed and greppable**, one `pub const REASON_*`
//! per [`QuotaReason`] arm through [`QuotaReason::as_str`], mirroring
//! `src/envelope/policy.rs`'s `ParkReason` shape. It is a **fourth sibling
//! taxonomy** beside the envelope's `ParkReason`, the router's `RouterReason`
//! and the bounds' `BoundsReason` — never an extension of any of them, and in
//! particular not a fifth `BoundsReason` arm: ...
```

Copy this verbatim in structure. `escalate.rs` must say *fifth sibling taxonomy*, name all four
existing siblings, and state the axis that makes it a sibling (research § "The escalation cap's
home": the bounds are facts about *whether the run is progressing*; the escalation count is a fact
about *how much the model was consulted*). Note **why the caller passes `now`** — the same reason
`rate_limit.rs` gives — because the corpus and cap fixtures depend on it.

**Taxonomy shape** (`rate_limit.rs:79-166`, identical in `bounds.rs:36-73` and `router.rs:70-190`):

```rust
/// The stable identifier a quota park writes into `JournalEvent::Parked`.
pub const REASON_QUOTA_REJECTED: &str = "quota_rejected";

/// Why a run parked on a subscription quota.
///
/// One arm today, and it is spelled as an enum rather than as a bare constant
/// for the reason its three sibling taxonomies are: a second quota condition ...
/// is a new arm here and a compile error at every site that has to classify it,
/// rather than a second literal minted at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaReason {
    Rejected,
}

impl QuotaReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant, no wildcard.
    pub fn as_str(&self) -> &'static str {
        match self {
            QuotaReason::Rejected => REASON_QUOTA_REJECTED,
        }
    }
}
```

Target shape (three reasons per research): `escalation_cap_reached`, `escalation_action_refused`,
`escalation_output_unusable`, all with prefix `escalation_`.

**Bounded-rendering pattern for the verbatim refused action** — the "record it verbatim, but bound
what reaches a journal line" split is already solved at `rate_limit.rs:126-135`:

```rust
/// The longest observed window string that reaches the journal.
///
/// The window string is untrusted wire content and the journal lands in
/// `.planning/`, a directory users commit (SAFE-04, T-17-05, T-20-27). The
/// **type** carries the observed value verbatim, which is what
/// "never mapped, never dropped" requires; this bound applies only where the
/// value is rendered into a record, alongside control-character removal, so an
/// unbounded or line-breaking value cannot be written through a record whose
/// one-line-per-entry shape every reader depends on.
pub const MAX_OBSERVED_WINDOW_CHARS: usize = 64;
```

This is the pattern for CONTEXT.md's "record the named-but-refused action verbatim": type carries it
whole, the journal rendering is length-bounded and control-char-stripped. It also carries the
`QuotaWindow::Unknown(Option<String>)` distinction (`rate_limit.rs:182-191`) — "the wire named
something we do not recognise" vs "there was nothing to name" — which the escalation's malformed-vs-
absent `structured_output` split needs.

**Cap-resolution pattern** — `bounds::resolve` (`bounds.rs:225-247`) is the analog for OQ3 (refuse a
cap ≥ the resolved step cap):

```rust
pub fn resolve(
    max_steps: Option<u32>,
    wall_clock_cap_secs: Option<u64>,
) -> Result<RunBounds, BoundsRefusal> {
    let max_steps = max_steps.unwrap_or(DEFAULT_MAX_STEPS);
    if max_steps == 0 {
        return Err(BoundsRefusal::ZeroStepCap);
    }
    let wall_clock_cap = match wall_clock_cap_secs {
        None => DEFAULT_RUN_WALL_CLOCK_CAP,
        Some(0) => return Err(BoundsRefusal::ZeroWallClockCap),
        Some(secs) if secs > MAX_WALL_CLOCK_CAP_SECS => {
            return Err(BoundsRefusal::WallClockOutOfRange { secs })
        }
        Some(secs) => Duration::from_secs(secs),
    };
    Ok(RunBounds { max_steps, wall_clock_cap })
}
```

Note the escalation cap must be resolved against the **resolved `max_steps`**, not against
`DEFAULT_MAX_STEPS` — comparing two constants is exactly the Phase 20 Critical (§ Shared Pattern D).
The refusal-constant justification to copy is `bounds.rs:124-134` (`MAX_WALL_CLOCK_CAP_SECS`): *"a
ceiling is what makes 'there is no disablement' a property of the parser rather than of the caller's
restraint."*

**Refusal-enum-to-seam-error pattern** (`bounds.rs:165-172`):

```rust
/// Why a proposed set of bounds was refused before the run existed.
///
/// Each maps to one typed `DriveError` variant at the seam in `driver::drive`,
/// mirroring the `run_id` precedent: the refusal happens where a bad value first
/// arrives, not inside a `value_parser` wired to one of the three paths that can
/// supply it (a hand-typed invocation, the TUI's argv builder, a re-read record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsRefusal { ... }
```

---

### `src/driver/goal.rs` (NEW — plan type, schema, legality predicate)

**Analog:** `src/driver/router.rs` for the alphabet-as-data + constructive-guarantee shape.

**The alphabet and the one path from an action to a string** (`router.rs:296-384`) — this is the
whole of SAFE-08 and `goal.rs` must consume it, never restate it:

```rust
pub const SAFE_COMMAND_ALPHABET: &[&str] = &[
    COMMAND_DISCUSS_PHASE,
    COMMAND_PLAN_PHASE,
    COMMAND_EXECUTE_PHASE,
];

/// A forward-motion action, and the one command verb it builds.
///
/// The indirection is what makes the alphabet a *constructive* guarantee rather
/// than a documented intention: a rule row names an action, an action names a
/// verb from [`SAFE_COMMAND_ALPHABET`], and there is no path from a row to a
/// string that does not pass through here.
pub enum RouterAction { Discuss, Plan, Execute }

impl RouterAction {
    /// The bare verb, always a member of [`SAFE_COMMAND_ALPHABET`].
    ///
    /// Exhaustive with no wildcard, so a fourth action is a compile error here
    /// and therefore a deliberate widening of the alphabet.
    pub fn verb(self) -> &'static str { ... }

    /// Every arm, in declaration order — the enumeration the both-directions
    /// guard walks.
    pub const ALL: &'static [RouterAction] = &[
        RouterAction::Discuss, RouterAction::Plan, RouterAction::Execute,
    ];
}
```

`RouterAction::ALL` already exists — the re-parse in research Pattern 1 uses it directly.
`command_for` is **private** (`router.rs:373`) and its doc (`:362-372`) records WR-09, where an
unvalidated token was rendered into something that reads as a pasteable command line by the *preview*
path. That is the precedent for CONTEXT.md's "no shell string is ever constructed from model output,
including for logging or preview".

The `SAFE_COMMAND_ALPHABET` doc (`:309-326`) also gives the both-directions-guard sentence to reuse
verbatim in the new tests: *"an allowlist wider than the truth it describes is the failure that shape
exists to catch (T-20-17)."*

**Refusal-reason taxonomy** for an irreducible goal: same shape as `escalate.rs` above; prefix
`goal_`.

---

### `src/driver/untrusted.rs` (NEW — the untrusted-content envelope)

**Analog:** `src/driver/dry_run.rs` for build-enforced text; `rate_limit.rs` for "the payload is
untrusted input".

No existing module constructs a prompt, so this is the weakest analog in the phase. What transfers:

1. The module doc must state the two-part mitigation (CSPRNG nonce + JSON escaping) with the
   declined alternative named, per the rationale-in-code convention.
2. `uuid` v4 is already a dependency (`Cargo.toml:42`, `features = ["v4", "serde"]`) — no new dep.
3. The enumerated untrusted-string set (research Pattern 2 table: `RoadmapPhase.name/.description`,
   `ProjectState.status/current_phase_name/milestone/pause_context`) needs a source-scanning guard,
   not a comment — see Shared Pattern C.
4. A `const BOUNDARY: &str` is forbidden (research Pitfall 5 warning sign). Contrast with
   `dry_run.rs`'s pinned constants, which are *user-facing output* and therefore correctly pinned —
   the two are not the same thing and the module doc should say so.

---

### `src/executor/claude.rs` (MOD — argv profile + child env)

**Analog:** itself. Both edit sites are single, audited, and already documented.

**argv site** (`:239-296`, with the `--strict-mcp-config` line at `:261`):

```rust
pub fn build_argv(options: &ExecutionOptions) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();

    // Exhaustive on purpose: Phase 22's `Container` variant must land here as a
    // compile error rather than as a silently host-shaped argv.
    match options.target {
        ExecutionTarget::Host => {}
    }

    push(&mut argv, "-p");
    ...
    push(&mut argv, "--permission-mode");
    push(&mut argv, options.permission_mode.as_flag_value());
    push(&mut argv, "--strict-mcp-config");
    ...
}
```

The `SpawnProfile` discriminant follows the `match options.target { ExecutionTarget::Host => {} }`
shape exactly — exhaustive, no wildcard, so a third profile is a compile error. `--strict-mcp-config`
stays unconditional at `:261`; `--mcp-config` is never pushed (C-2 — this is a guard, not a feature).

The `build_argv` doc already carries the rationale for `--strict-mcp-config` and, crucially, the
argv-vs-file argument that justifies asserting on argv at all:

```rust
/// **Why the envelope's tool denylist rides here rather than only in the
/// settings file (D-06 layer 1, D-07).** ... The decisive property is not the
/// cost though: it is that **argv cannot be silently dropped, because it is
/// argv rather than a file**. A `--settings` file that fails validation is
/// silently ignored in print mode with no error shown ...
/// `tests/envelope_pr_cap.rs`'s
/// `every_pattern_the_settings_file_denies_is_also_carried_on_argv` is what
/// keeps the two lists from drifting apart.
```

Also copy the comma-joining note at `:282-289` if the seam ever passes a list value.

**env site** — `:450-497`, and its doc already answers the `CLAUDE*`-scrub interaction the phase
must resolve:

```rust
// The TUI is plausibly launched from inside a Claude Code session,
// so inherited CLAUDE* variables would leak into the driven child
// and change `-p` behaviour in ways that look like "works on my
// machine". Scrub them all, then set the one we mean to set.
//
// ... **This closure is the ONE place in the tree that builds the child's
// environment**, so the envelope is applied here and nowhere else; a second
// applier is a second thing that can disagree about what the child inherits.
for (key, _) in std::env::vars_os() {
    if key.to_string_lossy().starts_with("CLAUDE") {
        cmd.env_remove(&key);
    }
}
cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);
```

`CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` is the exact precedent: a `CLAUDE*` name scrubbed by the loop
and then re-set immediately after. `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1` goes on the same line pattern.
`MAX_STRUCTURED_OUTPUT_RETRIES=1` does **not** start with `CLAUDE` and so survives the scrub — it
must be set explicitly to deny an ambient value (research Pitfall 2), and it belongs beside the
`envelope_env` application at `:485-496`, whose doc explains why removal and assignment are distinct
instructions.

---

### `src/registry.rs` + `src/journal/mod.rs` (MOD — the `sha256:` digest)

**Analog:** `journal::argv_digest` (`src/journal/mod.rs:543-554`) — the function whose own doc is the
reason C-4 exists.

```rust
/// A non-cryptographic identity digest of a spawned command line.
///
/// FNV-1a 64 over `argv` joined by the ASCII unit separator, rendered as
/// `fnv1a64:` plus 16 lowercase hex digits. Two facts about it are deliberate:
///
/// - **It is not a security control.** It exists so two runs can be compared for
///   "same command line" in a `run.json`, nothing more. Nothing authenticates,
///   authorises or trusts anything on the strength of this value.
/// - **It is implemented inline because this phase adds zero dependencies.**
///   No hashing crate is present in `Cargo.toml` and the digest does not warrant
///   adding one.
pub fn argv_digest(argv: &[String]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let joined = argv.join("\u{1f}");
    let mut hash = OFFSET_BASIS;
    for byte in joined.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}
```

The `format!("{prefix}:{hex}")` shape is the migration path: a new `sha256_digest` returns
`format!("sha256:{hex}")`, and `fnv1a64:`-prefixed values read as legacy. `argv_digest` stays
unchanged and keeps its "not a security control" doc — the new function is a sibling, and the second
bullet above (*"this phase adds zero dependencies"*) must be corrected in the same commit that adds
`sha2`, per the `dry_run.rs:78-83` precedent for falsified doc text.

**The consumer** — `registry.rs:133-150`:

```rust
/// A digest of `<project_root>/CLAUDE.md` as it stands right now.
///
/// Reuses [`crate::journal::argv_digest`], which adds no hashing code and no
/// dependency. Two facts about that function are the reason it is the right one
/// here ...: it is FNV-1a based, and it is
/// **explicitly not a security control**. That is exactly what this needs — a
/// cheap identity fingerprint for *drift detection*, never an integrity check.
///
/// **Phase 17 records the digest and acts on nothing.** Phase 21 re-confirms the
/// opt-in when the file drifts ...
fn claude_md_digest(project_root: &Path) -> Option<String> {
    let contents = std::fs::read_to_string(project_root.join("CLAUDE.md")).ok()?;
    Some(crate::journal::argv_digest(&[contents]))
}
```

Every sentence of that doc becomes false under C-4 option (a) and must be rewritten in the same
commit. `None` on absent/unreadable stays — *"both are ordinary states for a project, not failures."*

**The single writer** (`registry.rs:92-131`) is where `prompt_inputs: Vec<PromptInput>` gets
populated; its doc is the constraint to preserve:

```rust
/// **This is the only function outside tests that constructs a [`DriverOptIn`],
/// and that uniqueness is the point (D-14).** Because a record can come into
/// existence in exactly one place, a `Some(record)` sitting in a `config.json` is
/// proof of a deliberate user action rather than a bit that could have been
/// flipped from anywhere.
```

and the field-by-field construction convention (`:118-128`): *"Every envelope field is named
explicitly as `None` rather than reached for through `..Default::default()` ... the next field added
must break this line, not be silently absorbed by it."* The new field must be named there.

---

### `src/config.rs` (MOD — `DriverOptIn` grows `prompt_inputs`)

**Analog:** `DriverOptIn` itself (`:66-124`). Every field after `opted_in_at` carries
`#[serde(default)]`, which is what makes an addition non-breaking:

```rust
    /// A digest of the project's `CLAUDE.md` as it stood at opt-in time.
    ///
    /// Recorded now, acted on by **nothing** in this phase. Phase 21 re-confirms
    /// the opt-in on drift.
    pub claude_md_digest: Option<String>,
    ...
    /// **Never trusted raw.** `envelope::policy::validate_namespace` refuses
    /// every shape that would disable the control ... and
    /// `envelope::policy::EnvelopePolicy::resolve` degrades an invalid value to
    /// the default rather than widening the boundary. An absent value and a
    /// rejected value therefore mean the same thing, which is the safe thing.
    #[serde(default)]
    pub branch_namespace: Option<String>,
```

`branch_namespace`'s "an absent value and a rejected value mean the same thing, which is the safe
thing" is the model for how a missing or unparseable `prompt_inputs` entry must behave: treat as
drift, re-confirm.

The struct doc at `:74-77` (*"Phase 21 re-confirms the opt-in when `CLAUDE.md` drifts"*) becomes
past-tense in this phase and must be updated rather than left describing the future.

---

### `src/driver/run.rs`, `src/driver/mod.rs`, `src/cli.rs` (MOD)

**The seam site** — `run.rs:1925-1948`, where the ambiguity seam fires:

```rust
let (command, rationale) = match router::decide(&observed, target_phase) {
    router::Decision::Run { command, rationale } => (command, rationale),
    router::Decision::Park { reason, detail } => {
        terminal = Terminal::Parked { reason, detail };
        break 'iterations;
    }
    router::Decision::NoRule { observed } => {
        terminal = Terminal::Parked {
            reason: router::RouterReason::NoRule,
            detail: observed,
        };
        break 'iterations;
    }
    router::Decision::GoalMet => { terminal = Terminal::GoalMet; break 'iterations; }
};
```

The `NoRule` arm is the one that gains the seam. Note the comment above it (`:1915-1924`): *"Pure: no
I/O, no model call. The state was read above and is handed in (D-11)"* — that sentence becomes false
for the `NoRule` branch and must be corrected in the same commit (`dry_run.rs:78-83` precedent again).
The `observed` value carried into `detail` is exactly the "router's observed state only" input
CONTEXT.md permits the seam.

**`Terminal`** — `run.rs:426-443`. Research Pitfall 6 is confirmed by its doc:

```rust
/// How a run ended, with no unclassified arm (DRIVE-06, D-25).
///
/// **Five arms, and the absence of a sixth is the requirement.** Criterion 5's
/// *"never as an unclassified 'loop ended'"* is a **type-level** property rather
/// than a logging convention ... Every match on this type is
/// exhaustive with no wildcard, so an arm added later is a compile error at each
/// site that has to classify it.
```

If a sixth arm is added, this paragraph is falsified and must be rewritten; if the escalation park
reuses `Terminal::Parked` with the new taxonomy's `as_str()`, the paragraph stands. Prefer the
latter — the doc at `:435-441` already says *"every reason reaches disk through machinery that
already existed rather than through a third string source"*, which is the parking route the
escalation cap should take. `PARKED_LABEL_PREFIX` (`:424`) is how the reason reaches the terminal
label.

**Seam refusal placement** — `mod.rs:396`:

```rust
bounds::resolve(args.max_steps, args.wall_clock_cap_secs).map_err(DriveError::from)?;
```

with the comment at `:388` explaining the placement (*"`bounds::resolve` is a pure function of two
`Option`s"*) and `:313` grouping it with the other above-the-run refusals. The escalation-cap
resolution goes on the adjacent line, and it needs the resolved `max_steps` in hand, not the default.

**argv** — `cli.rs:110-127` already carries the goal and explains why `allow_hyphen_values` is safe
on it:

```rust
        // Interpreting a goal is Phase 21's; this phase only records it.
        ...
        // Safe here in a way it would not be on `--command`: the goal is the
        ...
        /// Free-text goal recorded into the run record, never interpreted
        goal: Option<String>,
```

and `driver/mod.rs:175-176`:

```rust
    /// Free text recorded into `RunRecord.goal` and never interpreted.
    pub goal: Option<String>,
```

Both doc lines become false in this phase. `--max-escalations` follows `max_steps: Option<u32>`
(`cli.rs:85`) exactly: an `Option`, defaulted and refused in `resolve`, never a clap `value_parser`.

---

### `tests/driver_injection_corpus.rs` (NEW)

**Analogs:** `tests/envelope_hook_refusals.rs` (plant hostile content, assert a mechanical refusal),
`tests/envelope_wiring.rs` (read the record back off disk as a separate process would).

**Plant-and-verify-the-plant** — `envelope_hook_refusals.rs:75-81, 176-218`:

```rust
fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent directory");
    }
    std::fs::write(path, contents).expect("the planted file");
}
```

```rust
    // Ignored by `.gitignore`, so git will never report it as a changed or
    // untracked file — and it is exactly the kind of path a secret gets
    // written to. This is D-33's named blind spot, planted deliberately.
    write(&fx.work, "secrets/prod.pem", PLANTED_PEM);
    assert!(
        git(&fx.work, &["check-ignore", "-q", "secrets/prod.pem"]),
        "the fixture must plant the secret on a path the repository's ignore \
         rules actually cover, or this test proves nothing"
    );
    ...
    assert!(
        stderr.contains("secret_detected"),
        "the refusal must carry its D-24 park reason; stderr was:\n{stderr}"
    );
    assert!(
        !stderr.contains("MIIBOgIBAAJBAKj34GkxFhD9abcdefgh"),
        "the report reproduced the secret it blocked, which is the \
         redact-at-capture bug committed by the code meant to prevent it; \
         stderr was:\n{stderr}"
    );
```

Three things to lift: the *"or this test proves nothing"* precondition assertion; the assertion that
the refusal record names the park reason; and the negative assertion that the record does **not**
reproduce the hostile payload — which is exactly corpus class 5's "assert the refusal record contains
no constructed shell string".

**Read the record back off disk** — `envelope_wiring.rs:58-82`:

```rust
/// Every `Parked` event in the journal **at `journal_path` on disk**, as
/// `(reason, needs)` pairs.
///
/// **Reading the file rather than an in-memory handle is the entire point.** The
/// hook and guard re-entries this file exercises are *other processes* ... and
/// the only thing this process shares with them is the filesystem. A helper that
/// inspected a `JournalRun` this process was holding would prove that this
/// process can write what it just wrote.
///
/// `reader::read_all` is the shipped reader, tolerant parse and all, so a record
/// of a kind this build did not model still arrives rather than being dropped.
fn parked_events(journal_path: &Path) -> Vec<(String, String)> {
    let (records, _diagnostics) = reader::read_all(journal_path).expect("the journal is readable");
    records
        .iter()
        .filter(|record| record.kind == "parked")
        .map(|record| {
            (
                record.rest["reason"].as_str().unwrap_or_default().to_string(),
                record.rest["needs"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}
```

Copy `parked_events` shape wholesale for both new test files; assert the escalation reason strings
through the new taxonomy's `as_str()`, never a literal in the test.

Also `envelope_wiring.rs:6-7` states the file's own standard, which the corpus file should restate:
*"Every fact below is an exit code, a file on disk, a git ref, or a journal event read back out of
`journal.jsonl`. Not one is a sentence somebody wrote about..."*

**Graceful skip** (`envelope_hook_refusals.rs:86, 172-174`): `fn fixture(...) -> Option<Fixture>` and
`let Some(fx) = fixture("...") else { return; };` — the sandbox-tolerant shape. The corpus tests
spawn a real `claude`; the same shape applies when the binary or auth is unavailable, **but only for
availability, never for the assertion itself** — a corpus test that skips silently in CI is the
vacuity class the phase exists to prevent, so pair the skip with a `#[ignore]`-style opt-in or a
loud diagnostic.

---

### `tests/driver_escalation_cap.rs` (NEW)

**Analog:** `tests/driver_rate_limit.rs` (37.8K — the fourth-taxonomy park's test file) plus
`parked_events` above. The cap-park assertion is: drive to the cap, then read
`escalation_cap_reached` out of the on-disk journal, and assert the run did **not** silently degrade
to rules-only (CONTEXT.md).

---

### `tests/spawn_seam_guard.rs` (MOD)

**Analog:** itself, `:399-440` — the both-directions source-scanning guard:

```rust
    let unexpected: Vec<&String> = observed.iter().filter(|p| !allowed.contains(p)).collect();
    assert!(
        unexpected.is_empty(),
        "a process-spawn site appeared in a file that is not on the allowlist. Confirm it \
         takes a capability type and then add it to `SPAWN_ALLOWLIST` in this file, in the \
         same commit (PITFALLS:521). Unexpected: {unexpected:?}"
    );

    let vanished: Vec<&String> = allowed.iter().filter(|p| !observed.contains(p)).collect();
    assert!(
        vanished.is_empty(),
        "an allowlisted file no longer spawns anything, so the allowlist is now wider than \
         the truth it describes. Remove the stale entry: {vanished:?}"
    );

    let claude = files.iter().find(|(path, _)| path == "src/executor/claude.rs")
        .expect("the agent spawn seam must exist");
    assert!(
        executable_lines(claude).any(|(_, line)| line.contains("project: &DrivableProject")),
        "the agent spawn seam must still take the capability type and never a bare path — \
         that signature is what makes the opt-in gate a compile-time property (D-16)"
    );
```

The `executable_lines(...).any(|(_, line)| line.contains(...))` idiom is the exact tool for the new
guards: the seam profile implies `--tools ""`; `--mcp-config` appears nowhere in `src/`; seam A's
constructor has exactly one call site in `src/` and it is outside `'iterations` (research Pitfall 8).

There is also a guard-of-the-guard precedent at `:442-449`
(`a_signal_to_a_process_group_is_not_mistaken_for_a_spawn`) — a test whose subject is the scanner's
own false-positive behaviour. New source-scanning guards should get one.

**Private-fields guard** (`:388-397`) is the analog for the moved approval capability of Pitfall 8:

```rust
    assert!(
        public.is_empty(),
        "every field of `DrivableProject` must be private: private fields are what stop \
         a caller assembling the token without passing a constructor, which is the whole \
         reason it is a type rather than a bool. Public fields:{}",
        render(&public)
    );
```

---

### `Cargo.toml` (MOD — `sha2`)

**Analog:** the existing dependency-comment convention, `Cargo.toml:36-50`:

```toml
# `tokio1` is NOT a default feature and the crate is inert without it. This
# dependency's own manifest declares rust-version = "1.87.0", which is where
# the [package] rust-version floor above comes from.
# No line-framing crate is added: NDJSON framing uses tokio::io::BufReader::lines()
# plus an explicit byte-length bound, so tokio-util is deliberately not a dependency.
process-wrap = { version = "9.1.0", features = ["tokio1"] }
uuid = { version = "1.24", features = ["v4", "serde"] }
# One crate covers BOTH jobs the supervisor needs — process-group signalling
# ... — which is why `fs4` was declined (D-08).
# It resolves to 1.1.4, the exact version already present in Cargo.lock
# transitively via ratatui/crossterm, so it adds zero new compilation units to
# the graph. It is pure Rust with no libc linkage, which matches the deliberate
# no-`nix`/no-`libc` posture documented at `src/executor/claude.rs:102-103`.
rustix = { version = "1.1", features = ["process", "fs"] }
```

The convention: an exact pinned minor (`"9.1.0"`, `"1.24"`, `"1.1"`), explicit `features`, and a
comment block stating **why this crate**, **what was declined**, and **what it costs the graph**.
`sha2` must arrive with the same block, naming the declined alternative (keep FNV-1a + an honesty
sentence, C-4 option (b)). `uuid` v4 is already present at `:42` — the nonce needs no new dependency.
Per CONTEXT.md, resolve the `sha2` version mechanically (`cargo add sha2` / `cargo search sha2`) and
record the resolved version; do not spend a human checkpoint on it (this supersedes research's
`checkpoint:human-verify` gate).

---

## Shared Patterns

### A. Typed reason enum + `REASON_*` const + `as_str()` — FOUR existing siblings

**Sources:** `src/envelope/policy.rs` (`ParkReason`, 7 arms, closed), `src/driver/router.rs:70-190`
(`RouterReason`, 15 arms, two prefixes `router_`/`gate_`), `src/driver/bounds.rs:36-73`
(`BoundsReason`, 4 arms, `bounds_`), `src/driver/rate_limit.rs:80-166` (`QuotaReason`, 1 arm,
`quota_`).
**Apply to:** `escalate.rs` (the fifth), `goal.rs` (refusal reasons).

The shape is identical across all four: one `pub const REASON_* : &str` per arm, a `#[derive(Debug,
Clone, Copy, PartialEq, Eq)]` enum, and an `as_str()` with one arm per variant and **no wildcard**.
`bounds.rs:61-64` states why:

```rust
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant, no wildcard: a new detector is a compile error here
    /// rather than a halt that borrows somebody else's reason string.
```

`router.rs:119-125` states the corollary:

```rust
/// **The gate arms are a closed set, and an un-named case does not get an
/// improvised thirteenth.** ... Minting a reason at a call site is how a taxonomy
/// stops being one, and it is why every arm below returns a `REASON_*` constant
/// rather than a fresh literal.
```

### B. The taxonomy table in `src/journal/mod.rs` MUST gain a fifth row

**Source:** `src/journal/mod.rs:821-847` (the `JournalEvent::Parked` variant's `reason` field).

```rust
        /// **Four sanctioned taxonomies ride this one field, and naming only
        /// one of them would be the same quiet lie this record exists to
        /// prevent.** They are siblings, never extensions of each other:
        ///
        /// | Taxonomy | Prefix on disk | What it means |
        /// |---|---|---|
        /// | [`crate::envelope::policy::ParkReason`] | none (e.g. `force_push_blocked`) | ... **Seven arms, and it stays at seven** ... |
        /// | `crate::driver::router::RouterReason` | `router_` **or** `gate_` | ... |
        /// | `crate::driver::bounds::BoundsReason` | `bounds_` | ... |
        /// | `crate::driver::rate_limit::QuotaReason` | `quota_` | ... Its own taxonomy rather than a fifth `BoundsReason` arm: ... |
        ///
        /// The prefixes make the producer readable from the string alone, and
        /// they are a property of each enum's own `REASON_*` constants rather
        /// than something assembled here. All four reach a terminal record
        /// through the single `parked:` label prefix ...
        reason: String,
```

Editing this is not optional documentation upkeep. Three things change together: the word "Four" →
"Five", a new table row for `crate::driver::escalate::EscalationReason` with prefix `escalation_`,
and "All four" → "All five". The table's own text makes an omission a documented lie.

### C. "A comment is not a guard; the test is" — source-scanning guards

**Source:** `tests/spawn_seam_guard.rs` (25.6K, entirely this pattern).
**Apply to:** the seam-profile/`--tools ""` coupling, the `--mcp-config` absence, the enumerated
untrusted-string set (research A7), the one-call-site rule for seam A (Pitfall 8), and the
`RunRecord` field-scope requirement (`src/journal/mod.rs:2000`,
`run_record_fields_all_declare_their_scope` — new fields must say **Run-scoped** in their doc or the
build fails).

### D. Guard tests must be demonstrated failing — the Phase 20 Critical

**Source:** `src/driver/dry_run.rs:580-595`.

```rust
    #[test]
    fn the_pinned_commands_text_no_longer_claims_one_command_is_always_the_whole_run() {
        // The stale claim, verbatim from before Phase 20. It is spelled out here
        // rather than referenced, because a test that compared the constant with
        // itself could not detect its return.
        assert!(
            !SECTION_COMMANDS.contains("the single --command argument below is"),
            "the routed loop made that sentence false; a user-facing statement \
             the code has falsified is worse than having said nothing (DRIVE-06)"
        );
        assert!(
            SECTION_COMMANDS.contains("FIRST"),
            "and the replacement must state the routed limit rather than being \
             merely vaguer than what it replaced"
        );
    }
```

*"a test that compared the constant with itself could not detect its return"* is the codebase's own
articulation of the Phase 20 Critical. Concretely, for this phase:

- The escalation-cap guard must compare the cap against the **resolved** `max_steps`, not against
  `DEFAULT_MAX_STEPS`. `assert!(MAX_ESCALATIONS < DEFAULT_MAX_STEPS)` is two constants and proves
  nothing about a run launched with `--max-steps 2`.
- The argv guards must run `build_argv(&options)` and inspect the returned `Vec<OsString>`, never
  assert about a constant that `build_argv` also reads.
- Every guard must be shown red against the unfixed behaviour before the fix lands.

### E. The pinned-honesty disclosure — the residual-exposure register (research Q2)

**Source:** `src/driver/dry_run.rs:65-103` (four pinned constants) + `:580-595` (the paired test).

```rust
/// The commands header (**pinned contract** — see [`SECTION_REFSPECS`]).
///
/// **Replaced in Phase 20, in the same commit as the code that falsified it.**
/// The previous text read *"the single --command argument below is the complete
/// and honest sequence for this build"*, which stopped being true the moment
/// `--target-phase` put a run under the decision router. Under CONVENTIONS.md:75
/// a change to this text is a breaking, user-visible output change, so the
/// paired byte-offset ordering assertion moves with it.
pub const SECTION_COMMANDS: &str = "== GSD commands this run would issue ==\n\
    A run is either one supplied --command or a routed sequence the decision\n\
    router chooses per iteration. For a supplied command, the line below is the\n\
    complete and honest sequence. For a routed run only the FIRST command can be\n\
    shown: every command after it is chosen from state this preview does not\n\
    produce, so the rest are not withheld — they do not exist yet.";
```

and the ordering contract at `:97-103`:

```rust
/// **These four constants are a contract, not decoration.**
/// `dry_run::tests::the_rendered_report_carries_all_three_section_headers_in_order`
/// asserts all four appear, in this order, by comparing byte offsets — so a
/// section cannot silently disappear ... and there
/// is exactly **one** place that pins the order rather than two that can
/// disagree.
```

The disclosure this phase owes — *the seam profiles suppress `CLAUDE.md`; the **executor** profile
does not, and `CLAUDE.md` still enters the driven agent's context* — is a `pub const` in this shape,
with a paired test asserting both the presence of the load-bearing words and the absence of any
weaker predecessor text. Note the second half of research Q6 point 2: *"Disclosing a broader set than
is true is as dishonest as disclosing a narrower one"* — the file-disclosure list must distinguish
the two profiles rather than listing everything.

### F. Tolerant wire parsing, never `deny_unknown_fields`

**Source:** `rate_limit.rs:12-22`, cited to `src/executor/stream_json.rs:14-18` and
`src/executor/outcome.rs:9-12`; grepped by `tests/spawn_seam_guard.rs:623`.

```rust
//! **The payload is untrusted input.** It arrives from a process that itself
//! consumed untrusted repository content, so everything here is tolerant
//! parsing: no strict unknown-field rejection (forbidden everywhere under this
//! source tree, and grepped for), no panicking accessor, and no `unwrap` on a
//! wire field. `status` and `rateLimitType` are matched as `&str` with an
//! explicit fallback arm that carries the observed value ...
```

`ResultMessage.structured_output: Option<Value>` follows this exactly — `Option<Value>` for an
unmodelled wire field (the `permission_denials` precedent), no `deny_unknown_fields`, no `unwrap`.

---

## The arrival-assertion pattern (research C-3 / Pitfall 3)

**No existing test asserts that content reached the model.** Searched: all 27 files under `tests/`
for arrival/vacuity language, plus `envelope_wiring.rs`, `envelope_hook_refusals.rs`,
`driver_rate_limit.rs`, `driver_optin.rs`. Nothing exercises a positive "the model observed X" claim
— by design, since no shipped code sends content to a model for a decision. **The plan must
establish this pattern from scratch.**

The nearest structural precedents, all of which are *local* non-vacuity proofs rather than
*model-side* arrival proofs:

**1. `tests/driver_optin.rs:264-311` — the closest match.** A positive precondition, the negative
property, an independent second mechanism, and a trailing "the run really did happen":

```rust
    // Captured before the run, and nothing touches B between here and the
    // comparison — not even a git read, which would refresh `.git/index`.
    let before = fingerprint_tree(&b);

    // An empty fingerprint compares equal to itself, so the comparison below
    // would pass vacuously if the walk silently found nothing. Pin what it must
    // have seen: the project's own file, and the `.git` a wrong-project git
    // operation would disturb.
    assert!(
        before.iter().any(|(path, _, _)| path == ".planning/STATE.md"),
        "the walk must see the project's own files, got {} entries", before.len()
    );
    ...
    let after = fingerprint_tree(&b);
    assert_eq!(before, after, "a run against project A must leave project B byte-identical ...");

    // An independent second check through a different mechanism. A fingerprint
    // helper with a bug — a swallowed `read_dir` error, a wrong root — would
    // otherwise pass silently, and this is a safety test.
    assert_eq!(git_ops::is_dirty(&b), Some(false), "...");

    // And the run really did happen, so the assertions above are not vacuous.
    assert!(
        a.join(".planning").join("meta-manager").is_dir(),
        "the drive must actually have written its run directory under A"
    );
```

**2. `tests/envelope_hook_refusals.rs:180-184`** — *"the fixture must plant the secret on a path the
repository's ignore rules actually cover, or this test proves nothing"* (quoted in full above).

**3. `tests/driver_dry_run.rs:428`** — *"A tripwire that has never been seen to fire proves nothing.
Run the same..."*; and `tests/driver_kill_startup.rs:309-322` — *"test fails loudly instead of passing
vacuously ... half a vacuous pass would hide (CR-05)"*.

**What the plan must add:** the corpus schema gains a field the model fills with the `MARKER-XXXXXX`
tokens it observed; the test asserts *markers present* (arrival) **before** asserting *selected
command unchanged* (property). This is a new pattern in this codebase, and the plan should say so and
name it — `driver_optin.rs`'s four-part shape is the template, but every one of its checks is
filesystem-local and none of them crosses the transport.

---

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/driver/untrusted.rs` | utility (prompt builder) | transform | No module in `src/` constructs a prompt or a model-facing string. `dry_run.rs` renders user-facing text and `rate_limit.rs` handles untrusted wire input, but neither builds a boundary. Follow research Pattern 2 for the mechanism and `dry_run.rs`/`rate_limit.rs` only for doc register and untrusted-input posture. |

Partial gap, worth calling out separately: **`tests/driver_injection_corpus.rs` has a strong
structural analog and no behavioural one.** `envelope_hook_refusals.rs` gives the plant-and-refuse
shape and `envelope_wiring.rs` the read-back-off-disk shape, but no existing test spawns `claude` and
asserts on what the model saw. The arrival-assertion half is new (see above).

---

## Metadata

**Analog search scope:** `src/driver/`, `src/executor/`, `src/journal/`, `src/registry.rs`,
`src/config.rs`, `src/cli.rs`, `tests/` (all 27 integration files by name; 6 read in detail),
`Cargo.toml`.
**Files read in detail:** `src/driver/rate_limit.rs`, `src/driver/bounds.rs`, `src/driver/router.rs`,
`src/driver/run.rs`, `src/driver/dry_run.rs`, `src/executor/claude.rs`, `src/journal/mod.rs`,
`src/registry.rs`, `src/config.rs`, `tests/envelope_pr_cap.rs`, `tests/envelope_hook_refusals.rs`,
`tests/envelope_wiring.rs`, `tests/spawn_seam_guard.rs`, `tests/driver_optin.rs`, `Cargo.toml`.
**Pattern extraction date:** 2026-08-19
