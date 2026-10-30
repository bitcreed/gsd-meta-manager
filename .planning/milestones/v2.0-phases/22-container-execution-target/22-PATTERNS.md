# Phase 22: Container Execution Target - Pattern Map

**Mapped:** 2026-09-16
**Files analyzed:** 25 (10 new, 15 modified)
**Analogs found:** 24 / 25

> Every analog path below was checked with `git ls-files` and is **tracked source** in this
> repository. No path resolves into `.gsd/` (the untracked capability mirror present in this
> working tree) or any other install/runtime mirror.

---

## File Classification

### New files

| New file | Role | Data flow | Closest analog | Match |
|---|---|---|---|---|
| `src/executor/container/mod.rs` | model / capability token | transform | `src/executor/mod.rs:134-257` (`DrivableProject`) | **exact** |
| `src/executor/container/detect.rs` | service (probe) | request-response | `src/executor/gate.rs:157-260` (`validate_first_init` + `check_*`) | **exact** |
| `src/executor/container/argv.rs` | utility (pure builder) | transform | `src/executor/claude.rs:239-325` (`build_argv`) | **exact** |
| `src/executor/container/preflight.rs` | middleware / gate | request-response | `src/executor/gate.rs:157-260` + `src/error.rs:230-342` | **exact** |
| `src/executor/container/lifecycle.rs` | service (teardown) | request-response | `src/driver/kill.rs:288-366` (`stop_run`) | role-match |
| `tests/container_argv.rs` | test (pure) | transform | `tests/driver_model_seam.rs:104-133`, `tests/envelope_wiring.rs:914-960` | **exact** |
| `tests/container_preflight.rs` | test (pure refusals) | transform | `src/executor/gate.rs:262-330` (in-source gate tests) | **exact** |
| `tests/container_envelope_gap.rs` | test (real git, no runtime) | file-I/O | `tests/envelope_wiring.rs` (TempDir + `BIN` const, e.g. `:962-994`) | role-match |
| `tests/container_e2e.rs` | test (needs a runtime) | event-driven | `tests/driver_injection_corpus.rs:55-80`, `:1332-1440` (`#[ignore]` census) | **exact** |
| `containers/Dockerfile` (path is discretion) | config / build artifact | batch | **none** — see § No Analog Found | — |

### Modified files

| Modified file | Role | Data flow | Analog / precedent to copy |
|---|---|---|---|
| `src/executor/mod.rs:259-270` | model (enum) | transform | its own doc + `SpawnProfile` at `:272-322` |
| `src/executor/claude.rs:239-246` | utility | transform | the existing `match options.target` arm shape |
| `src/executor/claude.rs:366-390, 451-480` | service (constructor) | request-response | `ClaudeExecutor::new` / `with_program` |
| `src/executor/claude.rs:480-569` | service (spawn closure) | request-response | `match &profile` at `:511-548` (the second tripwire, C-3) |
| `src/driver/run.rs:954`, `:1760-1773` | service | CRUD (durable) | `make_run_record`'s field comments at `:909-975` |
| `src/journal/mod.rs:1170-1180`, `:1449-1568`, fixtures `:2989/3215/3261/3568` | model + fixtures | CRUD (durable) | `RunRecord.target_phase` at `:1481-1495` (serde-default migration posture) |
| `src/envelope/mod.rs:537` | test fixture | — | same label decision as journal fixtures |
| `src/config.rs:31-64` | model (registry) | CRUD | `RegisteredProject.driver_opt_in` at `:35-47` |
| `src/cli.rs:48-273` (`Drive` arm) | route (parser) | request-response | `--run-id` / `--target-phase` field comments at `:64-179` |
| `src/driver/mod.rs:139-341`, `:418-490` | model + parse boundary | transform | `DriveArgs::from_argv`'s exhaustive destructure |
| `src/driver/spawn.rs:58-80` (`drive_argv`) | utility | transform | the `goal` optional-flag append at `:75-78` |
| `src/app.rs:1882-1977` (+ the stop counterpart at `:1979+`) | controller (TUI) | event-driven | `start_driver_run` itself — no second spawn path (D-22-19) |
| `src/error.rs:230-342` | model (errors) | transform | `CapabilityError` verbatim shape |
| `Cargo.toml:46-48`, `:25-110` | config | batch | the `sha2` block at `:59-79` |
| `tests/spawn_seam_guard.rs` | test (guard) | transform | `drivable_project_has_exactly_two_constructors_and_private_fields` at `:375-445` |

---

## Pattern Assignments

### `src/executor/container/mod.rs` (model / capability token, transform)

**Analog:** `src/executor/mod.rs:134-257` — `DrivableProject`. This is the D-22-15 precedent and
CONTEXT.md names it by line.

**Struct shape** (`src/executor/mod.rs:160-164`) — private fields, derives:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivableProject {
    alias: String,
    root: PathBuf,
}
```

`VerifiedRuntime` copies this literally: no `pub` on any field, and the same four derives (the
guard test at `tests/spawn_seam_guard.rs:427-444` fails on any `pub ` line inside the struct body).

**One production constructor + one `#[doc(hidden)]` hatch** (`:179-246`). Copy both the split and
the register of the hatch's doc — the name is the alarm:

```rust
    /// Construct a token **without** a validated opt-in record.
    ///
    /// **Test and development only, and the name is the alarm.** ...
    /// What keeps it honest instead is
    /// `tests/spawn_seam_guard.rs::the_escape_hatch_has_no_call_site_in_src`,
    /// which proves mechanically that it has zero non-comment occurrences under
    /// `src/` outside this definition. **A comment is not a guard; that test
    /// is.**
    #[doc(hidden)]
    pub fn for_testing_bypassing_opt_in(...) -> Self
```

**Ordered refusals before anything is created** (`:190-216`): three `let ... else { return Err(..) }`
blocks, each naming the observed value, ordered so a later refusal cannot mask an earlier cause
("Ordered after `RootUnusable` deliberately: reading files under a root that is not a directory
would report a drift for a project whose real problem is that it has moved"). `VerifiedRuntime::probe`
orders: socket-not-found → version unreadable → `components` absent → unknown engine.

**Untrusted wrapping of any echoed value** (`:192`):

```rust
alias: crate::text::Untrusted::from_untrusted_source(alias.to_string()),
```

Apply to the engine names read off the wire and to any user-supplied image name echoed in a refusal.

**Module-root doc idiom** (`src/executor/mod.rs:1-48`): the module root states the two or three
*measured* facts that govern every type below, with the measurement named. `container/mod.rs`'s
header carries F5 Trap 2 and the F2 row letters the same way.

---

### `src/executor/container/detect.rs` (service, request-response)

**Analog:** `src/executor/gate.rs:157-260`.

**A pure function over a parsed message, with the I/O outside it** (`gate.rs:157-202`). This is
what makes every refusal testable with no process — and it is exactly RESEARCH § Testability's
recommendation to keep `identify(&SystemVersion)` pure so the bollard call is the only untested line:

```rust
/// Validate the **first** `system/init` and decide whether the run may begin.
///
/// Returns the facts to record, or a typed error naming the concrete observed
/// value that failed. Nothing here writes to stdin, spawns anything, or touches
/// the filesystem — it is a pure function over a parsed message, so every
/// fail-closed path is testable without a process.
///
/// **Fail-closed on absence, everywhere.** ... Each of those
/// absences is the exact shape a silent upstream regression takes.
pub fn validate_first_init(
    init: &InitMessage,
    requested_permission_mode: PermissionMode,
) -> Result<GateOutcome, CapabilityError> {
    check_version(init.claude_code_version.as_deref())?;
    check_capabilities(&init.capabilities)?;
    check_auth_path(init.api_key_source.as_deref())?;
    ...
}
```

**Absent field is a refusal, never a fall-through** (`gate.rs:205-213, 250-260`):

```rust
fn check_auth_path(observed: Option<&str>) -> Result<(), CapabilityError> {
    if observed == Some(SUBSCRIPTION_API_KEY_SOURCE) {
        Ok(())
    } else {
        Err(CapabilityError::AuthPathChanged {
            observed: observed.map(str::to_string),
            expected: SUBSCRIPTION_API_KEY_SOURCE,
        })
    }
}
```

`SystemVersion.components == None` maps onto exactly this arm: `IdentityUnreadable`, never "assume
docker" (RESEARCH C-2).

**Named constant with the rationale as its doc** (`gate.rs:77-81`) — the register RESEARCH § Open
Question 3 asks for on the volume name, and the one `PODMAN_ENGINE` / `DOCKER_ENGINE` use:

```rust
/// The `apiKeySource` value that means the subscription/OAuth path is alive.
///
/// Named rather than inlined because it is the entire content of the D-08
/// regression guard, and a future reader must be able to find it.
pub const SUBSCRIPTION_API_KEY_SOURCE: &str = "none";
```

**Warn-and-proceed vs refuse, distinguished explicitly** (`gate.rs:221-228`): a version above the
tested maximum logs `tracing::warn!` and proceeds. The container pre-flight's "which network backend
was chosen" report is that same class — reported, not refused.

**No process spawn.** `tests/spawn_seam_guard.rs:447-488` is bidirectional: a new `Command::new(` in
`src/executor/container/**` fails the allowlist test, and D-22-19 forbids weakening it. Every
one-shot probe (`claude auth status`, `<binary> --version`) therefore goes through bollard's
create/start/logs API, not through `Command::new("podman")`.

---

### `src/executor/container/argv.rs` (utility, transform)

**Analog:** `src/executor/claude.rs:239-325` — `build_argv`.

**Returns a value; pushes through one helper** (`claude.rs:239-253, 327-329`):

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
}

fn push(argv: &mut Vec<OsString>, arg: impl AsRef<OsStr>) {
    argv.push(arg.as_ref().to_os_string());
}
```

The container prefix builder is a sibling free function with the same signature shape
(`fn container_prefix(runtime: &VerifiedRuntime, mounts: &ContainerMounts, env: &EnvelopeEnv) -> Vec<OsString>`)
so it is unit-testable with no process — the reason `build_argv` and `EnvelopeEnv` are both values.

**Flag-shape comments that record the measured reason** (`claude.rs:267-272, 282-289`). Copy this
register for each F2 flag; the specifics section of CONTEXT.md requires citing the spike by row:

```rust
    if let Some(session) = &options.resume_session {
        // Always with an explicit value: a bare `-r` opens an interactive
        // picker, which under `-p` with no TTY is at best an error.
        push(&mut argv, "--resume");
        push(&mut argv, session);
    }
```

**One producer, two consumers, for the `-e` translation** — `src/envelope/cred.rs:123-174`:

```rust
/// One child-environment instruction: **remove** the variable (`None`) or
/// **set** it (`Some`).
pub type EnvelopeVar = (OsString, Option<OsString>);

/// The child's environment as a value, so it can be asserted on without
/// spawning anything (D-32).
///
/// **The `Option` is the design, not an implementation detail.** ...
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeEnv(Vec<EnvelopeVar>);

impl EnvelopeEnv {
    /// Every instruction, in the order a caller should apply them.
    pub fn entries(&self) -> &[EnvelopeVar] { &self.0 }
```

The `-e` flags are derived by iterating `envelope.entries()` and emitting only the `Some` arms —
`None` (removal) entries need no translation because a container inherits nothing. Do **not**
rebuild the list from a second source.

---

### `src/executor/container/preflight.rs` (middleware / gate, request-response)

**Analog:** `src/executor/gate.rs:133-202` for the outcome shape, `src/error.rs:230-342` for the
refusal type.

**Outcome struct: facts to record, one doc line each** (`gate.rs:133-155`):

```rust
/// What the gate validated, recorded onto the run handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutcome {
    /// The session UUID the CLI reported.
    pub session_id: Option<String>,
    ...
    /// `"none"` means the subscription/OAuth path is alive (D-08).
    pub api_key_source: Option<String>,
```

`PreflightOutcome` carries: the socket path that answered, the runtime kind, the server/api
versions, the network backend chosen, the observed image-side binary version, and `auth_method`.

**Error enum: every variant a refusal, each `Display` naming the observed value**
(`src/error.rs:230-342`). Copy the doc header verbatim in register:

```rust
/// **Every variant here is a refusal, never a warning.** A refusal that becomes
/// advice is worse than no gate at all, because the user believes they were
/// protected (TRANS-04). Each `Display` names the concrete observed value
/// rather than saying a check "did not match", so the diagnostic is actionable
/// without re-running anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    VersionBelowFloor { observed: String, floor: String },
    VersionUnreadable { observed: Option<String> },
    AuthPathChanged { observed: Option<String>, expected: &'static str },
}
```

with the never-print-a-bare-`None` helper at `:307-313`:

```rust
fn render_observed(observed: &Option<String>) -> String {
    match observed {
        Some(value) => format!("`{value}`"),
        None => "no such field".to_string(),
    }
}
```

and `Display` bodies in this exact voice (`:324-337`):

```rust
            Self::VersionUnreadable { observed } => write!(
                f,
                "the claude CLI did not report a readable version ({}); refusing rather than assuming it is new enough",
                render_observed(observed)
            ),
```

**The remedy-by-name addition.** CONTEXT.md `<specifics>` requires "install `passt`",
`systemctl --user start podman.socket`, the `claude auth login --claudeai` one-shot, "rebuild on a
glibc base" — carried as a `remedy: String` field (or a `fn remedy(&self) -> String`) on each
variant, rendered by `Display` after the observed value. There is no existing analog for the remedy
field itself; the observed-value half is `CapabilityError` verbatim.

**Where the new errors live:** `src/error.rs` holds every cross-module error type
(`CapabilityError`, `OptInError`, `SpawnError`, `DriveError` with `From` impls at `:183-188`,
`:1088-1090`). Put `RuntimeProbeError` / `PreflightError` there, with a `From` into `SpawnError` or
`DriveError` in the same commit, following `impl From<CapabilityError> for SpawnError` at `:183`.

---

### `src/executor/container/lifecycle.rs` (service, request-response)

**Analog:** `src/driver/kill.rs:288-366` — `stop_run`. D-22-06 is the container-shaped extension of
this, not a replacement.

**The ordered-steps doc, where the order IS the decision** (`kill.rs:288-314`):

```rust
/// Stop the run led by `pgid`, and confirm it is gone (D-06 steps 1 and 4).
///
/// In order, and the order is the decision:
///
/// 1. **Liveness first, before any signal** ...
/// 2. **Resolve the target from the kernel** ... The recorded `pgid` arrives from a file inside
///    the driven project, which the agent can write; the group actually signalled is the one
///    `/proc` reports, and a disagreement sends nothing at all (CR-02, D-04).
/// 3. **SIGTERM to that group** ...
/// 4. **Wait out [`DRIVER_TEARDOWN_GRACE`]** ...
/// 5. **Escalate** to the uncatchable signal if the grace expired, with a `tracing::warn!` ...
///
/// **Never call this on the render thread.** The grace is twelve seconds; the
/// caller dispatches it on a task and takes the result back as an `Action`
/// (TRANS-03).
pub async fn stop_run(pid: u32, pgid: u32, run_id: &str, arm: ReapArm) -> StopOutcome
```

`stop_container` copies: (1) probe by inspect before acting, (2) stop with a bounded timeout,
(3) `tracing::warn!` on escalation to `remove --force`, (4) a typed `StopOutcome`-shaped result whose
`AlreadyGone` arm is distinct from `SignalFailed` — `kill.rs:316-323` is emphatic that
"already gone" and "could not be determined" must not collapse.

**Never invent a state** (`kill.rs:360-365`):

```rust
    // The result is deliberately not branched on: SIGKILL cannot be caught, so a
    // pid still present after this bound is one whose reap has not been observed
    // yet, not one that survived. Reporting it as anything other than
    // `ExitedAfterKill` would invent a state.
```

**Never on the render thread** — the TUI dispatches container stop on a task and takes the result
back as an `Action`, exactly as `App::stop_driver_run` does today (`src/app.rs:1979+`).

---

### `src/executor/mod.rs` — the `Container` variant (modified)

**Current text** (`:259-270`), which the variant addition must update rather than leave stale:

```rust
/// Where a run executes.
///
/// One variant today (D-21). Phase 22 adds `Container`, which is a variant
/// addition rather than a signature change — a one-line cost now against a
/// touch-every-call-site cost later. Derives match `DetailSubView` in
/// `src/app.rs`, the repository's precedent for a plain grow-over-time enum.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ExecutionTarget {
    /// Run directly on this machine.
    #[default]
    Host,
}
```

**Neighbouring precedent for a discriminant that must stay exhaustive** — `SpawnProfile`'s doc at
`:272-322` ("**Exhaustive at every match site, no wildcard**… a third profile must land as a compile
error at `build_argv` and at the spawn closure"). Copy that sentence shape for the `Container` arm,
**and note C-3**: the spawn-closure match is on `options.profile`, not on the target, so the second
tripwire does not exist yet and must be added in the same commit.

---

### `src/executor/claude.rs` — the argv arm, the constructor, the closure (modified)

**The arm that breaks** (`:242-246`) — the comment must be rewritten, not deleted, in the commit
that satisfies it (this repo's standing rule; see `src/driver/dry_run.rs:78-83` precedent cited at
`src/cli.rs:192-194`).

**A named production constructor, not `with_program`** — its doc (`:377-390`) is what makes reuse
wrong:

```rust
    /// Drive a different program, with fixed leading arguments placed before
    /// the generated argv.
    ///
    /// This is how the transcript-replaying stand-in is driven in tests without
    /// spawning a real `claude`, and it keeps the fixture out of the production
    /// path entirely — no Cargo feature, no fixture branch in `main`.
    pub fn with_program(program: impl Into<PathBuf>, leading_args: Vec<OsString>) -> Self {
```

Add `ClaudeExecutor::for_target(&ExecutionTarget) -> Self` beside `new()` (`:366-375`), with the
same three-field construction. The composition site it feeds is already right (`:469-470`):

```rust
        let mut argv = self.leading_args.clone();
        argv.extend(build_argv(&options));
```

**The spawn closure** (`:480-569`). Two things to copy and one to amend:

- `current_dir(&cwd)` + the three `Stdio::piped()` calls at `:481-485` stay byte-identical — under a
  container target `cwd` is still the host project root and the duplex still rides the client's stdio.
- The `CLAUDE*` scrub loop at `:502-506` and the `EnvelopeEnv` application at `:557-568` stay
  untouched (RESEARCH § Pattern 3: leave the closure exactly as it is).
- The claim at `:498-501` — *"**This closure is the ONE place in the tree that builds the child's
  environment**"* — becomes ambiguous for a container target and must be amended **in the same
  commit**, naming the container case. `tests/spawn_seam_guard.rs:1062` already has a test named
  `the_spawn_closure_comment_no_longer_claims_one_variable_is_set`, i.e. this file's comments are
  themselves under test; check it before rewording.
- Add the second exhaustive `match &target` inside the closure, modelled on the `match &profile` at
  `:511-548`, purely to restore the tripwire (C-3):

```rust
            // Exhaustive, no wildcard — the same discipline `build_argv` uses on
            // this discriminant.
            match &profile {
                SpawnProfile::Executor => {}
                SpawnProfile::ModelSeam { .. } => { ... }
            }
```

---

### `src/driver/run.rs` — the durable label and target selection (modified)

**The D-22-16 defect** (`:954`), sitting among fields whose comments explain *why the value is what
it is* — match that register for the replacement:

```rust
        bounds: Some(journal::RecordedBounds { ... }),
        target: format!("{:?}", options.target),
        // The field Phase 16 reserved at `src/journal/mod.rs:475` specifically
        // so this phase adds no migration.
        opt_in: entry.driver_opt_in.as_ref().map(|record| record.opted_in_at.clone()),
```

Nearby precedent for "the resolved value, never the constant" (`:946-953`) is the same argument the
explicit label needs: *"a reader answering 'what was this run allowed to do?' must not have to work
out which binary produced the record"*.

**Where the target is chosen** (`:1760-1773`) — `iteration_options` is the one constructor of
`ExecutionOptions` for a real iteration and uses `..Default::default()`; the resolved target is
threaded in here, never re-derived per iteration:

```rust
fn iteration_options(
    envelope_settings: &Path,
    envelope_env: &EnvelopeEnv,
    run_bounds: &bounds::RunBounds,
    elapsed: Duration,
) -> ExecutionOptions {
    ExecutionOptions {
        envelope_disallowed_tools: policy::disallowed_tools(),
        envelope_settings: Some(envelope_settings.to_path_buf()),
        envelope_env: Some(envelope_env.clone()),
        wall_clock_cap: bounds::iteration_wall_clock_cap(run_bounds, elapsed),
        ..Default::default()
    }
}
```

`ExecutionOptions::default()` sets `target: ExecutionTarget::Host` at `src/executor/mod.rs:518`.

---

### `src/journal/mod.rs` — `container_id` and the label (modified)

**The mandatory doc word.** `run_record_fields_all_declare_their_scope` (`:3147`) parses this
struct's own source (`:3135-3145`) and fails unless each `pub <name>:` line's preceding doc block
contains "run-scoped" or "iteration-scoped":

```rust
    fn run_record_struct_body() -> &'static str {
        const SOURCE: &str = include_str!("mod.rs");
        let start = SOURCE.find("pub struct RunRecord {") ...
```

**The serde migration posture to copy for `container_id`** (`:1481-1495`):

```rust
    /// The phase a routed run was driving toward, or `None` in command mode.
    ///
    /// **Run-scoped** — the target bounds the whole sequence and no iteration
    /// changes it; ...
    ///
    /// `Option` plus `#[serde(default)]` per the serde migration posture: a
    /// record written before Phase 20 has no such key and must still load, and
    /// `run.json` carries **no version discriminator** to hang a migration from,
    /// so tolerance on read is the only mechanism available.
    /// `a_pre_phase_20_record_still_loads_with_the_new_fields_defaulted` proves
    /// it against a byte literal of the old shape rather than against a record
    /// this build produced.
    #[serde(default)]
    pub target_phase: Option<String>,
```

**The event's `target` field** (`:1170-1180`) and the record's (`:1568`) are the two producers; the
five existing lowercase fixtures are `src/journal/mod.rs:2989`, `:3215`, `:3261`, `:3568` and
`src/envelope/mod.rs:537`, each literally `target: "host"` / `"target": "host"`.

---

### `src/config.rs` — the per-project target setting (modified)

**Analog:** `RegisteredProject.driver_opt_in` at `:31-64`, in the same struct.

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisteredProject {
    pub path: PathBuf,
    pub added: String,
    /// The user's driver opt-in, or `None` for a project that may only be read.
    ///
    /// `#[serde(default)]` — the first serde attribute on this struct — makes
    /// every pre-Phase-17 `config.json` load unchanged ...
    /// Adding this field breaks the struct literals in `src/registry.rs`, and
    /// **that breakage is the feature** ...
    #[serde(default)]
    pub driver_opt_in: Option<DriverOptIn>,
    /// Every field of this entry that this build does not model.
    /// ... A downgrade must preserve, never delete.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}
```

Three properties the new field copies: `Option` + `#[serde(default)]`; the deliberate breakage of
`src/registry.rs`'s struct literals as a compile-time forcing function; and the fact that `extra`'s
`#[serde(flatten)]` is what lets an older binary round-trip it (D-22-18's downgrade property).
`CONFIG_SCHEMA_VERSION` at `:21` — read its doc before deciding whether this add bumps it (it bumped
for opt-in; a `#[serde(default)]` add arguably does not).

---

### `src/cli.rs` + `src/driver/mod.rs` — the per-run override (modified)

**Analog:** `src/cli.rs:74-85` states the convention CONTEXT.md names:

```rust
        // **The fields above are the RAW side of a parse boundary, and they stay
        // raw on purpose** (21-15). Clap parses a command line; it does not
        // judge payloads. Every string flag on this subcommand crosses into the
        // driver through `driver::RawDriveArgs` and
        // `driver::DriveArgs::from_argv`, which is the one place a value
        // carrying nothing a reader could see is refused ... Adding a
        // `value_parser` here would be a second judge of the same property on
        // one of the several paths that build a `DriveArgs`.
```

So: the target flag and any image-name flag are `Option<String>` on the clap struct with no
`value_parser`, mirrored onto `RawDriveArgs` (`src/driver/mod.rs:314-341`), and judged in
`from_argv`.

**The parse boundary** (`src/driver/mod.rs:418-490`) — the exhaustive destructure is the mechanism
and a new field must be classified there or it will not compile:

```rust
    pub fn from_argv(raw: RawDriveArgs) -> Result<DriveArgs, DriveError> {
        // NO `..` REST-PATTERN. See the doc above: this listing is the
        // mechanism, and a rest-pattern would let a new argv field arrive
        // unclassified and unprotected, which is the five-time losing bet.
        let RawDriveArgs { alias, command, target_phase, ..., goal, ... } = raw;

        let command = argv_visible(command, |_| DriveError::NoCommandSource)?;
        let run_id = argv_visible(run_id, |run_id| DriveError::RunIdInvalid { run_id })?;
```

with the one-helper-not-five-copies rule at `:343-373`:

```rust
fn argv_visible(
    value: Option<String>,
    refuse: impl FnOnce(crate::text::Untrusted) -> DriveError,
) -> Result<Option<payload::NonBlank>, DriveError> {
```

The image name's *structural* validation (non-empty, no leading `-`, no traversal) is a second check
beside `argv_visible`, in the register of the `is_plain_path_component` note at `:173-182` — and
`DriveArgs`'s doc at `:126-137` says the domain is the struct, so the field is a `NonBlank`.

**Argv emission** (`src/driver/spawn.rs:58-80`) — append an optional flag exactly as `goal` is:

```rust
    if let Some(goal) = goal {
        argv.push(OsString::from("--goal"));
        argv.push(OsString::from(goal));
    }
    argv
```

Note `drive_argv`'s signature is positional today; adding a fifth optional parameter touches
`src/app.rs:1911` and the `tests/driver_*.rs` callers listed in its doc at `:53-57`.

---

### `src/app.rs` — TUI start/stop (modified)

**Analog:** `App::start_driver_run` at `:1882-1977` — and D-22-19 says it is the *only* path:

```rust
        let run_id = crate::journal::new_run_id(chrono::Utc::now(), &uuid::Uuid::new_v4());
        let argv = drive_argv(&config_path, alias, command, &run_id, goal);

        match spawn_detached(&project_root, &argv) {
```

Note `run_id` is generated here, before spawn — which is what makes RESEARCH § Pattern 2's
`--name <run-id>` deterministic and knowable before the container exists, with no third `run.json`
write. The optimistic `ObservedRun` insert at `:1929-1949` is where a container-aware TUI would carry
the container name (it is the run id, so it carries nothing new).

---

### `tests/container_e2e.rs` (test, needs a runtime)

**Analog:** `tests/driver_injection_corpus.rs`.

**The availability rule, stated in the file header** (`:66-76`):

```
// The class arms and the control pair spawn the real `claude` binary and need an
// authenticated subscription, so they are `#[ignore]`d. **`#[ignore]` is for
// availability and never for the assertion**: run explicitly they FAIL LOUDLY
// with a diagnostic naming what was missing rather than skipping, because a
// security proof that silently passes in an environment without credentials is
// the exact vacuity this plan exists to eliminate (T-21-32).
```

**The census that pins the ignored set** (`:1332-1420`):

```rust
    assert_eq!(
        ignore_attributes, 10,
        "this file must carry exactly TEN line-anchored `#[ignore]` attributes: \
         seven class arms, two suppression controls, and the arms' own \
         non-vacuity meta-check. Found {ignore_attributes}. A raw substring count \
         of the attribute over this file reads strictly higher ... so a census built on \
         containment would assert ten while MEASURING the larger set."
    );
```

Copy the census verbatim in shape (line-anchored detection, not `contains`), with this file's own
count and its own sentence about what the arms are.

**The env-gated self-skipping idiom is the ANTI-pattern here.** `tests/state_reader_test.rs:922-927`
does exist:

```rust
#[ignore = "reads a real file outside the repository; opt in with GSD_META_MANAGER_REAL_STATE_MD"]
fn a_real_foreign_state_md_reads_read_only() {
    let Ok(path) = std::env::var("GSD_META_MANAGER_REAL_STATE_MD") else {
        println!("GSD_META_MANAGER_REAL_STATE_MD unset — nothing to read");
        return;
    };
```

RESEARCH § Testability is explicit: do **not** use it for the container arms — a silently skipped
security test is the failure this repo's conventions are written against. Use the corpus's
fail-loudly `#[ignore]`.

---

### `tests/container_argv.rs` / `tests/container_preflight.rs` (tests, pure)

**Analog for argv assertions:** `tests/driver_model_seam.rs:104-133` — derive the expectation from
the production function, never from a literal:

```rust
/// Every argv word `build_argv` produced, as lossy UTF-8.
fn words(options: &ExecutionOptions) -> Vec<String> {
    build_argv(options).into_iter().map(...).collect()
}
...
        "build_argv returned nothing, so every assertion below would be about an \
```

plus the non-vacuity assertion pattern (assert the list is non-empty before asserting about it), and
`tests/envelope_wiring.rs:914-960`'s note that the exhaustive match itself is a **compile** property
and only its observable consequence is assertable from a test crate.

**Analog for refusal tests:** `src/executor/gate.rs:262-330` in-source tests — a `healthy()` fixture
varied one field at a time, with `None` meaning "absent from the JSON entirely":

```rust
    /// Build an `InitMessage` from its three gate-relevant fields.
    ///
    /// `None` means the field is **absent from the JSON entirely**, which is a
    /// different thing from present-and-empty and is exactly what the
    /// fail-closed cases need to distinguish.
    fn init(capabilities: Option<&[&str]>, version: Option<&str>, api_key_source: Option<&str>) -> InitMessage
```

A `SystemVersion` fixture built the same way (absent `components` vs empty vs unknown name) is the
whole of the identity test matrix.

---

### `Cargo.toml` — the bollard block (modified)

**Analog:** the `sha2` block at `:59-79` — CONTEXT.md names it, and RESEARCH § 4e already drafted the
text. The properties to keep:

```toml
# SHA-256 for the driver opt-in's re-confirmation digest (C-4). A maintained
# RustCrypto implementation rather than a third hand-rolled hash in this tree ...
# The declined alternative was keeping FNV-1a and pinning an honesty sentence ...
# Graph cost, measured rather than borrowed: 7 newly locked packages —
# sha2 0.11.0, digest 0.11.3, ... NOTE: this is deliberately NOT the `rustix` case above.
sha2 = "0.11.0"
```

i.e. (1) what it is for and which decision it implements, (2) the **declined alternative, named**,
(3) the **measured** graph cost with every crate listed, (4) an explicit note where the honest answer
differs from a neighbouring entry's. The `rustix` block at `:51-58` is the contrasting "zero new
units" case and the `icu_properties` block at `:80-110` carries the `cargo update` sentence RESEARCH
recommends echoing.

**The stale sentence this add falsifies** (`:46-48`) must be corrected in the same commit:

```toml
# `tokio1` is NOT a default feature and the crate is inert without it.
# No line-framing crate is added: NDJSON framing uses tokio::io::BufReader::lines()
# plus an explicit byte-length bound, so tokio-util is deliberately not a dependency.
process-wrap = { version = "10.0.0", features = ["tokio1"] }
```

**The MSRV floor** (`:5-15`) is derived, not chosen, and the re-measure command is in the comment:

```toml
#   cargo metadata --format-version 1 --locked \
#     | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1
rust-version = "1.88"
```

---

### `tests/spawn_seam_guard.rs` (must stay green **unmodified** — D-22-19)

The guard that the new capability token should be covered by, if the planner chooses to extend it
(RESEARCH § Pattern 1 suggests it; D-22-19 forbids *weakening* it, and an additive assertion is not a
weakening — call this out as a decision rather than assuming it):

```rust
#[test]
fn drivable_project_has_exactly_two_constructors_and_private_fields() {
    ...
    let public: Vec<_> = body.iter().filter(|(_, line)| line.trim_start().starts_with("pub ")) ...
    assert!(
        public.is_empty(),
        "every field of `DrivableProject` must be private: private fields are what stop \
         a caller assembling the token without passing a constructor, which is the whole \
         reason it is a type rather than a bool. Public fields:{}",
        render(&public)
    );
}
```

Two existing assertions constrain this phase whether or not anything is added:

- `every_process_spawn_site_in_src_is_on_the_allowlist` (`:447-488`) fails in **both** directions —
  a new `Command::new(` / `CommandWrap::with_new(` / `process_group(` in any new file, *and* a stale
  allowlist entry. The container modules must spawn nothing.
- `the_agent_program_override_fields_are_debug_only` (`:529`) scans `AGENT_OVERRIDE_FIELDS =
  ["claude_program:", "claude_args:"]` (`:150-157`) and will **not** fire on a new `target:` or
  `container_image:` field — RESEARCH § Pattern 4 names that as a gap worth closing deliberately.

---

## Shared Patterns

### Fail-closed typed refusal that names the observed value and the remedy
**Source:** `src/error.rs:230-342` (shape) + `src/executor/gate.rs:157-260` (use).
**Apply to:** `container/detect.rs`, `container/preflight.rs`, `container/lifecycle.rs`, every new
variant in `src/error.rs`.
Absence is a refusal; a refusal never becomes advice; the `Display` quotes the observed value via
`render_observed`; add the remedy string (CONTEXT.md `<specifics>`).

### Type-as-capability: private fields, one production constructor, one named hatch
**Source:** `src/executor/mod.rs:134-257`; guarded by `tests/spawn_seam_guard.rs:375-445`.
**Apply to:** `VerifiedRuntime` in `container/mod.rs`.

### Exhaustive match, no wildcard, with the comment saying what must break here
**Source:** `src/executor/claude.rs:242-246` and `:295-322`; `src/executor/mod.rs:272-282`.
**Apply to:** the new `ExecutionTarget::Container` arm in `build_argv`, and the *new* second match
inside the spawn closure (C-3 — it does not exist today).

### Return a value, not a mutated `Command` — so tests need no process
**Source:** `src/envelope/cred.rs:127-148` (`EnvelopeEnv`), `src/executor/claude.rs:239-329`
(`build_argv`).
**Apply to:** the container prefix builder, the mount list, the `-e` translation.
One producer, two consumers: the `-e` flags derive from the same `EnvelopeEnv` the closure applies.

### Durable on-disk fields: `Option` + `#[serde(default)]`, tolerance on read, scope word in the doc
**Source:** `src/journal/mod.rs:1481-1495` (`target_phase`), `src/config.rs:46-63`
(`driver_opt_in` + `extra`).
**Apply to:** `RunRecord.container_id`, the per-project target on `RegisteredProject`, the D-22-16
label (the reader must accept both spellings for records already on disk).

### Untrusted wrapping at the point a value is put into a refusal
**Source:** `src/executor/mod.rs:183-193`, `src/driver/mod.rs:355-372`.
**Apply to:** engine names read off the API, the image name, the socket path, anything echoed back.

### Payload validation lives at `DriveArgs::from_argv`, never in a clap `value_parser`
**Source:** `src/cli.rs:74-85` (the statement), `src/driver/mod.rs:418-490` (the boundary).
**Apply to:** the target flag and the image-name flag.

### A stale comment is corrected in the commit that falsifies it
**Source:** `src/cli.rs:183-194`, `src/driver/mod.rs:405-411`, `src/envelope/hooks.rs:156-168`.
**Apply to:** `ExecutionTarget`'s "one variant today"; `claude.rs:498-501`'s ONE-place-env claim;
`Cargo.toml:46-48`'s tokio-util sentence; CONTEXT.md's two-tripwire claim (C-3).

### Named constant carrying its own rationale, pinned by a test
**Source:** `src/executor/gate.rs:61-81`.
**Apply to:** `PODMAN_ENGINE` / `DOCKER_ENGINE`, the named volume, `CLAUDE_CONFIG_DIR`'s mount point,
the probe timeout.

### Mirror a private constant where the reasoning happens, and assert the relationship
**Source:** `src/driver/kill.rs:372-389` (`CLAUDE_GROUP_GRACE` mirrored from
`src/executor/claude.rs:107`, asserted by `the_driver_grace_exceeds_the_claude_group_grace_plus_slack`).
**Apply to:** the container stop timeout vs the driver teardown grace vs the claude group grace —
three graces now, and their ordering needs the same kind of pin.

### Envelope generation stays on the host; every stub names `current_exe()`
**Source:** `src/envelope/hooks.rs:91-101` (`install`), `:242-294` (`assert_provenance_in`),
`src/envelope/cred.rs:619-639` (`build_env` / `build_env_in`).
**Apply to:** the two identical-path mounts and the `HOME` / `XDG_DATA_HOME` pass-through.
`assert_provenance_in` compares **canonicalised paths, not versions** (`:273-291`), which is exactly
why the binary is bind-mounted rather than baked (RESEARCH § 2e); and `:245-251` already refuses a
`current_exe()` that no longer exists, which a mid-run `cargo build` across the mount can produce.

---

## No Analog Found

| File | Role | Data flow | Reason |
|---|---|---|---|
| `containers/Dockerfile` (path is discretion) | config / build artifact | batch | The repository contains **no** container image, no Dockerfile, no OCI build of any kind — `git ls-files` finds only `.github/workflows/release.yml` as a non-Rust build artifact. Use RESEARCH § Pattern 7's table for content. The nearest *conventions* to borrow are: the checked-in executable-text idiom of `tests/fixtures/fake-claude*.sh` (a header comment stating what the artifact is for and what it must not do), the "pin it and say why" register of `Cargo.toml:5-15`'s FLOOR PROVENANCE block for the CLI version pin, and D-07's never-the-sole-carrier rule (`src/executor/claude.rs:225-235`) for disabling the auto-updater in **both** carriers. |

Partial-analog notes (an analog exists but does not cover one property):

- **The remedy-by-name field** on a refusal has no in-tree precedent; `CapabilityError` covers the
  observed-value half only.
- **A bollard/async external-API client** has no in-tree precedent (`src/envelope/advisory.rs` is the
  nearest external-service caller, but it shells out to `gh` rather than speaking a socket API). Copy
  its *posture* — read-only, bounded by its own budget, run once at run start, never on the guard's
  per-tool-call path (`tests/spawn_seam_guard.rs:76-79`) — not its transport.

---

## Metadata

**Analog search scope:** `src/` (all modules), `tests/` (all 60 integration test files), `Cargo.toml`,
`.github/workflows/`, repository root.
**Files scanned:** ~90 (grep/census) of which 20 read in full or in targeted ranges.
**Tracked-source verification:** `git ls-files` run over every analog path cited above; all tracked.
**Pattern extraction date:** 2026-09-16
