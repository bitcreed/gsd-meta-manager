use clap::{Parser, Subcommand};
// Only the debug-only `claude_args` field names it, so the import carries the
// same gate the field does; an ungated one would be an unused import in release.
#[cfg(debug_assertions)]
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "gsd-meta-manager",
    version,
    about = "TUI command center for GSD projects"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config file (overrides default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a GSD project to the registry
    Add {
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
        /// Optional alias (defaults to last folder component of path)
        alias: Option<String>,
    },
    /// Remove a project from the registry
    Remove {
        /// Alias of the project to remove
        alias: String,
    },
    /// List all registered projects
    List,
    // Every `///` below is rendered verbatim by clap as help text, so the
    // rationale for each field lives in `//` comments like this one and only the
    // one-line description a user needs is a doc comment.
    //
    // The variant carries no `#[cfg]` on purpose (D-05): driving is Unix-only,
    // but the subcommand parses on every platform and the *handler* returns a
    // typed unsupported-platform error. An accepted limitation that surfaces as
    // "unknown subcommand" is indistinguishable from a bug.
    /// Run one GSD command against a project that has opted in to being driven
    Drive {
        /// Alias of the project to drive; it must carry a driver opt-in record
        alias: String,
        // Exactly one command. The decision router is Phase 20's, so there is
        // deliberately no way to pass a sequence.
        /// The single GSD command to run, e.g. `/gsd-progress`
        #[arg(long)]
        command: String,
        // The TUI supplies it so it knows what to look for afterwards (D-03).
        //
        // It is **not** generated when absent, and this comment used to say it
        // was. A comment that describes a mode the code refuses is the next
        // reader's bug: a driver-generated id appears on no caller's argv, which
        // is exactly what made a run invisible to `driver::liveness::probe` —
        // reported crashed by every scan, and un-stoppable because a stop
        // answered already-gone without signalling (CR-04). `driver::drive`
        // refuses a real run with no id before anything is created.
        // It must also be a **single plain path component**, and that refusal
        // likewise lives in `driver::drive` rather than in a `value_parser`
        // here: the id reaches the run body from a hand-typed invocation, from
        // the TUI's argv builder, and from `writer::read_active_run` — a
        // validator wired to this one flag would guard the one path that is
        // already the least interesting (D-27, WR-02).
        /// Run id to record this run under; required unless `--dry-run`
        #[arg(long)]
        run_id: Option<String>,
        /// Report what the run would do without executing anything
        #[arg(long)]
        dry_run: bool,
        // Interpreting a goal is Phase 21's; this phase only records it.
        //
        // `allow_hyphen_values` because this phase turned the goal into a
        // free-text field a human types (WR-04). Without it, clap in the CHILD
        // reports "a value is required for '--goal <GOAL>' but none was
        // supplied" for anything beginning with `-` and exits non-zero — and the
        // child's stdio is `/dev/null`, so nothing is visible: the TUI has
        // already shown "Driving {alias} — run {id}" and inserted an optimistic
        // `ObservedRun { liveness: Alive }`, which disappears at the next scan
        // with no error anywhere. `-- do not touch main` and `-v2 migration
        // notes` are entirely plausible free text.
        //
        // Safe here in a way it would not be on `--command`: the goal is the
        // LAST operand the argv builder emits, so there is no following flag for
        // a hyphen-led value to swallow.
        /// Free-text goal recorded into the run record, never interpreted
        #[arg(long, allow_hyphen_values = true)]
        goal: Option<String>,
        /// Test and development only: the program to spawn instead of `claude`
        ///
        /// It exists because the driver is a separate process that constructs
        /// its own executor, so `ClaudeExecutor::with_program` does not reach
        /// it. The TUI's own spawn argv never emits this flag. A hidden flag
        /// rather than an environment variable is deliberate: an env var is
        /// inherited by children, so a stray `GSD_*` in the user's shell would
        /// silently reach a TUI-spawned driver — the same class of leak
        /// `src/executor/claude.rs:333-352` already scrubs `CLAUDE*` for —
        /// whereas a flag must be passed on purpose by a caller whose argv
        /// builder is itself unit-tested.
        ///
        /// **`hide = true` removes a flag from `--help`, not from the parser,
        /// and that is why the `#[cfg(debug_assertions)]` below is the part
        /// that matters (D-30, WR-16).** Without it, any caller of the
        /// *released* binary can make the "driver" exec an arbitrary program
        /// with an opted-in project as its cwd, and the journal records the
        /// result as an ordinary run. The env-var reasoning above is correct
        /// and unchanged; it argues against an env var, and it never argued for
        /// shipping the flag. The spawn seam is guarded against *emitting*
        /// these flags — nothing guards against a human or a script passing
        /// them, so the parser entry itself is what has to go.
        ///
        /// **The consequence, decided rather than discovered: `cargo test
        /// --release` no longer builds the integration tests that pass these
        /// flags, and that is the trade this project chose.** The gate is
        /// `cargo build && cargo test && cargo clippy -- -D warnings` and it
        /// does not run release tests; the nine `tests/fixtures/fake-claude*.sh`
        /// stand-ins are debug-only test infrastructure by nature; and shipping
        /// a flag shaped like remote code execution in the released binary to
        /// keep a release-mode test path is the wrong trade. Nobody should
        /// "fix" this by widening the cfg —
        /// `tests/spawn_seam_guard.rs::the_agent_program_override_fields_are_debug_only`
        /// fails if anybody does, because a `#[cfg]` a later refactor quietly
        /// widens is indistinguishable from never having added it.
        ///
        /// A debug build keeps the flag, so the cfg alone would leave one gap:
        /// there, a stand-in run still reads as a real one on disk. That gap is
        /// closed by the other half of D-30 — `src/driver/run.rs` journals
        /// `Diagnostic { code: "agent_program_overridden" }` before the run's
        /// first exec record whenever this flag is used.
        #[cfg(debug_assertions)]
        #[arg(long, hide = true)]
        claude_program: Option<PathBuf>,
        /// Test and development only: leading arguments for `--claude-program`
        ///
        /// Repeatable. Placed before the executor's own generated argv, which is
        /// how the checked-in shell stand-ins receive their transcript and exit
        /// code. Hidden for the same inheritance reason as `--claude-program`,
        /// and absent from a release build's parser for the same D-30 reason:
        /// it is the payload half of the same override, so leaving it parseable
        /// while gating its program would be a gate with a hole in it. The
        /// accepted `cargo test --release` consequence recorded above covers
        /// both fields; it is one decision, not two.
        #[cfg(debug_assertions)]
        #[arg(long, hide = true)]
        claude_args: Vec<OsString>,
    },
    // The envelope's enforcement points re-enter this same binary rather than
    // living in a generated shell script (D-04), so they need a parser entry.
    //
    // **`hide = true` alone is the right treatment here, and
    // `#[cfg(debug_assertions)]` would be wrong.** That is the opposite of the
    // lesson `--claude-program` above teaches, so the difference has to be
    // stated rather than inferred: a git hook stub generated by a *release*
    // build re-enters a *release* binary, so a subcommand that exists only in a
    // debug build is a hook that exits 2 with "unrecognized subcommand" in every
    // shipped configuration — which git reports as a hook failure and which
    // therefore blocks every push, including the ones inside the namespace. The
    // cfg on `--claude-program` removes a capability shaped like arbitrary code
    // execution; hiding this one only keeps `--help` honest about what a user is
    // meant to type.
    /// Internal: envelope enforcement re-entered by a generated git hook
    #[command(hide = true)]
    Envelope {
        #[command(subcommand)]
        action: EnvelopeAction,
    },
}

/// The enforcement points a generated hook stub re-enters this binary through.
///
/// One variant per hook; later plans in this phase add their own. Each returns a
/// process exit code, because for a git hook the exit code **is** the control
/// (D-25) — never a message a reader has to interpret.
#[derive(Subcommand)]
pub enum EnvelopeAction {
    /// Classify the refs git supplies on stdin against the reserved namespace
    PrePush {
        /// Registry alias whose reserved push namespace applies to this push
        alias: String,
        // The generated stub passes `"$0"` here, never a path baked in at
        // generation time (D-10). A baked path travels with a copy of the file,
        // so a relocated stub would hand back the original's path and certify
        // itself; `$0` is the path the shell was actually invoked as, which a
        // copy cannot forge by being copied.
        /// Path this hook was invoked from, checked against the envelope's own
        #[arg(long)]
        hook_path: PathBuf,
    },
}
