# Phase 22: Container Execution Target - Context

**Gathered:** 2026-09-16
**Status:** Ready for planning

> **Unattended run.** The human operator was unavailable. Every decision below was
> inferred from the planning artifacts and the codebase rather than chosen by the user.
> The inferred set is enumerated in `<inferred_decisions>` at the end of this file and
> is owed an audit before or during `/gsd-plan-phase 22 --research-phase`. Nothing here
> is a scope change: the phase boundary is exactly ROADMAP § Phase 22.

<domain>
## Phase Boundary

A driven run can execute the `claude` CLI **inside a container** instead of on the host,
on either docker or rootless podman, with no user-facing configuration difference between
the two runtimes — and the driver's own code path stays the host path apart from target
selection.

**In scope:** CTNR-01 through CTNR-05 — the `ExecutionTarget::Container` variant and its
argv/mount recipe, container-runtime auto-detection with verified identity, subscription
auth inside the container without mounting the host credential store, TUI start/stop/resume
of a containerized session, and resume parity with the host path.

**Out of scope (named so planning does not drift into them):** the gate-policy modes
(`skip`/`defer`/`auto`) — that is Phase 23; the D-R-P-E-V router and run bounds — Phase 20;
remote/CI execution targets; a second `Executor` implementor; a Unix-socket control
fast-path (deferred by REQUIREMENTS since Phase 15); publishing the container image to a
registry as a distributed artifact.

</domain>

<decisions>
## Implementation Decisions

### Runtime detection and identity (CTNR-02)

- **D-22-01: Runtime identity is read from the response, never inferred from which
  constructor was called.** `bollard::connect_with_podman_defaults()` was *observed*
  returning a healthy handle to a **docker** daemon with no error and no warning
  (`22-SPIKE-OQ5.md` § F5, Trap 2). Identity comes from `version().components[].name` —
  `"Podman Engine"` vs `"Engine"` — or equivalently the `Libpod-Api-Version` header on
  `/_ping`. A run whose runtime identity cannot be read **refuses to start** rather than
  guessing; guessing wrong emits `--userns=keep-id` at docker (rejected) or writes
  root-owned files into the user's project tree (spike rows H and F).
  — **Reversibility:** costly — the detection result is carried in the `ExecutionTarget`
  payload and matched exhaustively at `build_argv` and the spawn closure; loosening it later
  means revisiting every construction site and the guard test that fences them.

- **D-22-02: `podman info`'s `RemoteSocket.exists` is never trusted.** The spike observed
  `exists: true` while the socket file, and its parent directory, were both absent
  (§ F5, Trap 1). Probe by connecting, not by asking.

- **D-22-03: Auto-detect means "no user-facing configuration", not "one argv for both".**
  The spike proved no such argv exists (§ F2 note 2, § *What this means for Phase 22*).
  Success criterion 1 is satisfied by the driver branching internally on a **verified**
  runtime, and the plan must not chase a single portable invocation.

### The transport split — argv prefix for the run, API for detection and lifecycle

- **D-22-04: The driven run stays a child process spawned through an argv prefix**
  (`docker run -i …` / `podman run -i …` in front of the existing `claude` argv). This is the
  ROADMAP's "argv prefix plus path map swap" and it is what keeps success criterion 4 literally
  true: the stream-json duplex still rides `ChildStdin`/`ChildStdout`, `process-wrap`'s process
  group still exists, and Phase 15's framing, gate, outcome and interjection code is untouched.
  Attaching over the Docker API instead would replace the child-process plumbing that
  `src/executor/claude.rs` is built around, and the code path would no longer be identical.
  — **Reversibility:** costly — reversing means re-plumbing the executor's stdio seam.

- **D-22-05: Detection, inspect and lifecycle (stop/remove) go through the Docker-compat
  REST API via bollard.** The spike showed the API returns **identical Docker-shaped payloads
  from both runtimes** (§ F4), so the `ps --format json` normalization the ROADMAP budgeted for
  (§ F3: `Id` vs `ID`, `Names` array vs string, `Command` array vs string) **is not written at
  all**. That is a deliberate deletion of planned work, recorded here so a planner does not
  re-add it.

- **D-22-06: The API is also how a containerized run is actually stopped.** A process-group
  kill reaches the `docker`/`podman` *client*, not the container; relying on client signal
  forwarding alone would leak a running container past a kill-switch. Cancel therefore stops
  the container by id through the API, and the container id is recorded on the run so a
  reattaching driver or the TUI can reach it. This is the container-shaped extension of
  `src/driver/kill.rs`, not a replacement for it.

### Mount, uid and path parity (CTNR-05)

- **D-22-07: The mount argv branches on the verified runtime, using the spike's measured
  recipe** (§ F2): rootless podman gets `--userns=keep-id`; rootful docker gets
  `--user $UID:$GID`. Each flag is wrong or rejected on the other runtime — `--user 1000:1000`
  on rootless podman fails with `Permission denied` (row B), and `--userns=keep-id` on docker
  fails with `invalid USER mode` (row H). The Docker habit is exactly backwards on podman.

- **D-22-08: The project root is mounted at the *identical absolute path* it has on the
  host.** `--resume` scopes session lookup to the project directory and its worktrees, so any
  path translation breaks resume; identical paths also mean the journal, inbox and lock paths
  the host-side driver writes and the container-side agent sees agree without a mapping table.
  "Path map swap" in the ROADMAP is therefore an identity map, and that is the decision.

### Credentials (CTNR-03, success criterion 2)

- **D-22-09: The host `~/.claude` is never bind-mounted.** Credentials live in a **named
  volume** owned by this tool, which is what makes them survive an image rebuild (criterion 2's
  second half). An anonymous volume or a bind mount into the host home both fail that criterion
  — the first does not survive, the second is the thing criterion 2 forbids.
  — **Reversibility:** costly — once users have authenticated into a named volume, changing its
  name or internal layout is a migration of user state: they must log in again.

- **D-22-10: `CLAUDE_CONFIG_DIR` is set to the same path the volume is mounted at.** This is
  the ROADMAP's two-file credential trap: `~/.claude.json` lives *outside* `~/.claude`, so a
  volume at `~/.claude` alone does not keep the session signed in. One directory, one volume,
  one variable.

- **D-22-11: Proof that criterion 2 holds is inherited, not newly asserted.** Phase 15's
  capability gate already **refuses** a run whose `apiKeySource` is anything but `"none"`
  (`src/executor/gate.rs`, D-08). A container that fell back to an `ANTHROPIC_API_KEY` would be
  refused at the first `system/init` by existing code. The phase should lean on that refusal as
  the mechanical evidence rather than adding a parallel check that can drift from it.

- **D-22-12: First-time authentication is an explicit, separate, interactive one-shot** —
  never something a driven run performs. A driven run that finds the volume unauthenticated
  **refuses with a named remedy**; it does not open a login flow inside an unattended run.
  Whether that one-shot is `claude setup-token` or an interactive `/login` is left to research
  (see `<open_questions>`).

### Pre-flight capability gate for the container target

- **D-22-13: Host prerequisites are checked and reported *by name*, up front, in the same
  shape as Phase 15's capability gate.** The spike found two that a naive implementation surfaces
  as opaque container-create failures: rootless podman 5.x defaults networking to **`pasta`**
  (the `passt` package) and fails with `could not find pasta` when it is absent (§ F1, which is
  exactly this host's state), and **`podman.socket` is not running by default** so there is no
  API socket until `systemctl --user start podman.socket` creates it (§ F5). A driven run needs
  egress, so the phase must either require `passt` or pass `--network=slirp4netns` explicitly —
  and say which, by name, before anything is created.

### Image posture

- **D-22-14: The image pins the CLI version, disables the auto-updater, and runs as a
  non-root user, with egress restricted by an allowlist** (ROADMAP § Phase 22 risks, adopted
  verbatim). The non-root choice is precisely why `--userns=keep-id` matters: the spike's row D
  shows keep-id remaps a non-root image user to the host uid so mounted writes stay host-owned,
  while rootless podman's *default* (row A) is already correct only for a root-default image.

### The `ExecutionTarget::Container` shape

- **D-22-15: `ExecutionTarget::Container` carries the verified runtime identity as a type
  that cannot be constructed without a successful probe** — the same type-as-capability idiom
  as `DrivableProject` (`src/executor/mod.rs`, D-23), where private fields and a single
  production constructor make the compiler, not a code review, the enforcement. This turns
  D-22-01 into a structural property: a `Container` target that was never identity-verified is
  unrepresentable.
  — **Reversibility:** costly — the enum is matched exhaustively with no wildcard at
  `src/executor/claude.rs:244` and at the spawn closure, both of which carry comments saying
  Phase 22's variant must land there as a compile error; a later shape change touches every one.

- **D-22-16: The journal's `target` string stops being a `Debug` rendering.**
  `src/driver/run.rs:954` currently writes `format!("{:?}", options.target)`, which yields
  `"Host"` today and would write `Container { image: "…", runtime: Podman }` — a durable on-disk
  value derived from a derived trait — the moment the variant gains fields. It becomes an
  explicit, stable label. Note the existing disagreement the plan must resolve deliberately
  rather than by accident: the fixtures at `src/journal/mod.rs:2989,3215,3261,3568` and
  `src/envelope/mod.rs:537` all say `"host"` lowercase while the producer emits `"Host"`.
  — **Reversibility:** one-way — `journal.jsonl` records are durable, are tailed by the TUI, and
  are read back by `driver::reconcile` for runs that already exist on disk; changing the rendered
  value later means old runs read wrong or a migration.

### Envelope parity inside the container

- **D-22-17: A containerized run whose Phase 19 git envelope cannot be shown live inside the
  container must refuse to start.** This is not optional polish: `src/envelope/hooks.rs`
  generates its `pre-push` stub naming `std::env::current_exe()` — a **host** path — and hands
  git `core.hooksPath` pointing at it. Git running inside the container would find a stub
  pointing at a binary that is not there, and the push boundary would be silently absent, with
  the run otherwise looking normal. Criterion 4's "identical apart from target selection" is
  false if the envelope is one of the differences. The *mechanism* is left to research (see
  `<open_questions>`); the fail-closed posture is locked here.

### Target selection and the TUI (CTNR-04)

- **D-22-18: Target is a per-project setting with a per-run argv override, and the override
  wins.** Per-project belongs on the registry entry, which already tolerates unknown fields on a
  downgrade (`src/config.rs`, `#[serde(flatten)] extra`) so an older binary will not delete it.
  The argv flag rides `gsd-meta-manager drive` and therefore reaches both entry points — TUI and
  hand-typed — through the one spawn seam, which is what makes CTNR-04 a property of the driver
  rather than of one screen.

- **D-22-19: The TUI gains no second spawn path.** Start/stop/resume of a containerized
  session flow through the existing Drive-Start Path (`Action::DriverStartRequested` →
  `App::start_driver_run` → `driver::spawn::spawn_detached`) and the existing filesystem
  observation path. `tests/spawn_seam_guard.rs` must stay green unmodified; if it needs
  weakening, the design is wrong.

### Claude's Discretion

- Exact naming of the runtime-identity type, the label strings in D-22-16, the volume name,
  and the argv flag spelling for D-22-18.
- Whether bollard is added as a dependency or the API is spoken directly over the socket —
  though the spike drove bollard 0.21.1 against both runtimes unmodified (§ F4), so bollard is
  the evidenced default and the graph cost should be *measured* and recorded in `Cargo.toml` in
  this repository's established idiom (see the `sha2` and `rustix` comment blocks).
- Test decomposition, file layout, and wave/plan splitting.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Binding for this phase
- `.planning/phases/22-container-execution-target/22-SPIKE-OQ5.md` — **the binding input.**
  Resolves the MUST-SPIKE empirically against podman 5.7.0 rootless and docker 29.1.3 rootful.
  Findings F1 (`pasta` missing), F2 (the uid/mount matrix), F3 (`ps --format json` divergence),
  F4 (the API erases it), F5 (two silent auto-detect traps). Read F2's table and F5's Trap 2
  before writing any detection or mount code.
- `.planning/ROADMAP.md` § *Phase 22: Container Execution Target* (lines 676–697) — goal, the
  four success criteria, the five phase risks, the `Research: yes` flag.
- `.planning/REQUIREMENTS.md` § *Containerized Sessions (CTNR)* (lines 91–97) — CTNR-01…CTNR-05
  verbatim, and the traceability rows at 171–175.

### Prior phase decisions that constrain this one
- `.planning/phases/15-transport-foundation/15-CONTEXT.md` § *deferred* (lines 540–556) — D-21
  introduced `ExecutionTarget` with a `Host` variant only and deferred `Container`, `PathMap`,
  bollard, podman, `setup-token` auth and `ps --format json` normalization to this phase.
- `.planning/codebase/ARCHITECTURE.md` § *Drive-Start Path* (lines 155–167), § *Key
  Abstractions* (168–199), § *Architectural Constraints* (217–225) — the five-step spawn path,
  the capability-token pattern, the single-spawn-seam and platform-gating constraints.

### Code this phase modifies or must not break
- `src/executor/mod.rs:259–270` — `ExecutionTarget`, with the comment naming Phase 22's variant.
- `src/executor/mod.rs:134–257` — `DrivableProject`, the type-as-capability precedent D-22-15 follows.
- `src/executor/claude.rs:239–325` — `build_argv`, and the exhaustive `match options.target` at
  `:244` that is designed to break here.
- `src/executor/claude.rs:451–540` — `start_run`'s spawn closure: `current_dir`, the `CLAUDE*`
  scrub, and the comment declaring this the ONE place the child's environment is built.
- `src/executor/gate.rs:135–200` — `GateOutcome` / `validate_first_init`; the `apiKeySource`
  refusal D-22-11 leans on, and the shape D-22-13's pre-flight report should echo.
- `src/journal/mod.rs:1170–1180` — `JournalEvent::RunStarted { target: String }`.
- `src/driver/run.rs:954` — `target: format!("{:?}", options.target)`, the D-22-16 defect.
- `src/envelope/hooks.rs` — `std::env::current_exe()` hook-stub generation; the D-22-17 hazard.
- `src/config.rs:32–64` — `RegisteredProject`, including `#[serde(flatten)] extra`'s
  downgrade-preserves-unknown-fields property that D-22-18 relies on.
- `tests/spawn_seam_guard.rs` — must stay green unmodified (D-22-19).
- `Cargo.toml` — the established idiom of recording a new dependency's **measured** graph cost
  in a comment (`sha2`, `rustix`, `process-wrap` blocks) applies to bollard.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`ExecutionTarget` enum** (`src/executor/mod.rs:266`): already exists with a single `Host`
  variant, explicitly reserved for this phase. Adding `Container` is the variant addition it
  was designed for, not a signature change.
- **Exhaustive no-wildcard matches** at `src/executor/claude.rs:244` and in the spawn closure:
  these are pre-placed tripwires — adding the variant produces compile errors exactly where a
  decision is owed, which is the intended planning aid.
- **Phase 15 capability gate** (`src/executor/gate.rs`): fail-closed-on-absence refusals with
  the concrete observed value named. Both the container pre-flight (D-22-13) and the auth proof
  (D-22-11) reuse it rather than duplicating it.
- **`DrivableProject`** (`src/executor/mod.rs:161`): the private-fields/one-production-constructor
  capability token that D-22-15 copies for verified runtime identity.
- **`process-wrap` + process-group kill** (`src/driver/kill.rs`): survives D-22-04 unchanged,
  but does **not** reach into the container — hence D-22-06.

### Established Patterns
- **Single spawn seam**, enforced by `tests/spawn_seam_guard.rs` walking `src/` — mechanical,
  not review discipline. Any container path that needs a second seam is disqualified.
- **Process boundary is the filesystem** (D-03): TUI and driver share nothing but files. A
  container that mounts the project root at the identical path (D-22-08) keeps that true across
  the new boundary too.
- **Typed refusals over silent degradation**: every non-Unix path returns
  `DriveError::UnsupportedPlatform`; `#[cfg(unix)]` gates `executor::claude`, `driver::run`,
  `driver::spawn`, `driver::kill`, `driver::lock`. The container target lands inside that gate.
- **Durable formats are never a derived-trait rendering** — the reason D-22-16 is a decision
  and not a cleanup.
- **A new dependency's graph cost is measured and written down** in `Cargo.toml`, with the
  declined alternative named (see the `sha2` block).

### Integration Points
- `build_argv` (`src/executor/claude.rs:239`) — where the container argv prefix is composed.
- `start_run`'s `CommandWrap::with_new` closure (`:480`) — `current_dir`, env scrub, the
  declared single place the child environment is built; the mount/userns flags and
  `CLAUDE_CONFIG_DIR` land in or beside it.
- `driver::run::execute_run` → `run.json` / `journal.jsonl` — where the container id and the
  rendered target are recorded.
- `App::start_driver_run` (`src/app.rs:1810`) and `driver::spawn::drive_argv` — where the
  per-run target override reaches the detached driver.
- `src/cli.rs`'s `Drive` arm — where the override is parsed, with the repository's convention
  that validation lives in `driver::DriveArgs::from_argv` rather than in a clap `value_parser`.

</code_context>

<specifics>
## Specific Ideas

- The spike's F2 table is the literal recipe; it should be cited by file and row in whatever
  code implements the mount branch, so a future reader can see the claim was measured on real
  runtimes rather than read from Docker-centric docs (which is precisely how this phase's
  LOW-confidence claims arose in the first place).
- Prefer stating a refusal's remedy by name — "install `passt`", "run
  `systemctl --user start podman.socket`" — over a generic "container runtime unavailable".
  The spike's whole F1/F5 point is that the default failure text points nowhere near the cause.

</specifics>

<open_questions>
## Open Questions for `--research-phase`

These are deliberately *not* decided here; they are what `Research: yes` is for.

1. **Which first-login mechanism** satisfies CTNR-03 in a container — `claude setup-token`, an
   interactive `/login` in a one-shot container, or something else — and what each writes, and
   where, relative to `CLAUDE_CONFIG_DIR` (D-22-12).
2. **How the Phase 19 envelope is made live inside the container** (D-22-17). Two candidates:
   ship the same-version `gsd-meta-manager` binary in the image and generate the hook stub with
   the container-side path; or argue that the egress allowlist plus the absence of any git
   credential in the container makes the push boundary unreachable by construction. Research
   should establish which is actually true here, not pick the cheaper one.
3. **Whether `--network=slirp4netns` is sufficient** for the CLI's egress on rootless podman, or
   `passt` is a hard requirement (D-22-13). The spike ran every probe with `--network=none` and
   therefore never exercised egress at all.
4. **bollard's exact graph cost**, measured (`cargo add --dry-run` / lockfile delta), for the
   `Cargo.toml` comment this repository requires.
5. **Whether an egress allowlist is enforceable** on both runtimes without privileged flags —
   the spike ran with no `--privileged`, no `--cap-add`, no `sudo`, and that fence should hold.

</open_questions>

<inferred_decisions>
## Inferred Decisions (audit required)

The human was unavailable. Each line below was chosen by the agent from the artifacts and the
codebase, not by the user. Anything here can be overridden at plan time at low cost unless the
decision's own reversibility rating says otherwise.

- **D-22-04/D-22-05 — the hybrid transport split.** The ROADMAP says "argv prefix"; the spike
  says "go through the API, not the CLI". Read literally they conflict. Resolved as: argv prefix
  for the *run* (it is what preserves criterion 4), API for *detection and lifecycle* (it is
  what erases F3). The spike's own closing sentence supports this — "the argv-level branch of
  Finding F2 remains … create-time parameters, not a CLI-parsing concern" — but the synthesis is
  the agent's.
- **D-22-06 — stopping the container through the API.** Follows from the process-group kill not
  reaching past the client; not stated in any artifact.
- **D-22-08 — identity path mapping.** The ROADMAP requires "mount-path parity"; reading that
  as *identical absolute paths* (rather than a translation table) is the agent's reading.
- **D-22-09 — named volume specifically.** Criterion 2 requires surviving a rebuild; "named
  volume" is the agent's mechanism choice.
- **D-22-12 — a driven run refuses rather than logging in.** Not stated anywhere; inferred from
  the repository's fail-closed posture and the fact that unattended runs cannot complete an
  interactive OAuth flow.
- **D-22-15 — verified-identity-as-a-type.** The spike requires verification; making it
  structurally unrepresentable to skip it is the agent's design choice, modelled on D-23.
- **D-22-16 — the journal `target` rendering.** A defect the agent found while scouting
  (`src/driver/run.rs:954`), not raised in any artifact. Folding its fix into this phase is a
  judgement call: it is arguably a separate quick task, but the variant addition is what makes
  it bite, and the record is one-way.
- **D-22-17 — envelope-in-container fail-closed.** The hazard is real
  (`src/envelope/hooks.rs` bakes a host path) and unmentioned in the ROADMAP. Treating it as
  in-scope for Phase 22 rather than deferring it is the agent's call, justified by criterion 4.
  **This is the most audit-worthy item here** — it is the one decision that could enlarge the
  phase's work materially.
- **D-22-18 — per-project setting with argv override winning.** Precedence was not specified
  anywhere; chosen to match Phase 23's own stated precedence shape (per-run argv **and**
  per-project config, documented precedence).
- **Todo folding deviates from `--auto`'s literal rule.** `--auto` folds every todo scoring
  ≥ 0.4; the matcher returned 14 for phase 22 and **none is container-related** (they match on
  generic keywords such as "file", "path", "phase", "config"). Folding them would be exactly the
  scope creep the workflow's scope guardrail forbids — one of them ("verify-work gate policy
  must be configurable") is literally Phase 23's requirement. **Zero folded.** See
  `<deferred>`.

</inferred_decisions>

<deferred>
## Deferred Ideas

- **Remote/CI execution targets.** The `ExecutionTarget` enum will accept a third variant; this
  phase adds exactly one.
- **A second `Executor` implementor.** Explicitly rejected by `src/executor/mod.rs:78` and by the
  ROADMAP's own risk note — Container is a variant, not a backend.
- **Publishing the container image as a distributed release artifact.** Building an image this
  tool can run is in scope; shipping it to a registry is a release-process concern.
- **`ps --format json` normalization.** Not deferred — **deleted** by D-22-05. Recorded here so
  it is not resurrected as "missing work".

### Reviewed Todos (not folded)

All 14 matches from `todo.match-phase 22` were reviewed and **none folded** — the matcher scored
on generic keywords, not on container relevance.

- *The verify-work gate policy must be configurable — skip | defer | auto-validate* (0.9) —
  this is **Phase 23**'s CTRL-08 verbatim; folding it would move a phase's requirement.
- *Invalidate browser file cache after $EDITOR exits* (0.9), *Phases panel marker/grey disagrees
  with the disk-inferred stage* (0.9), *Show plan token estimate and actual counts* (0.9),
  *Visualize execution waves per phase in roadmap* (0.9), *Badge glyphs may misalign by one cell*
  (0.7), *Driver tab renders blank pipeline row at heights 8-13* (0.5), *Surface destructive
  confirmations in a modal popup* (0.5), *Backlog tab shows empty despite non-zero count* (0.5),
  *Git view — show co-author model on Enter* (0.3), *Add `b` keybinding for backlog* (0.3),
  *Make queue input truly multi-line* (0.3) — all TUI/display concerns unrelated to containers.
- *Sync gsd-core config and expose new options* (0.9) — config surface, unrelated.
- *`driver_reattach` liveness probe races the run.json write* (0.6) — a real driver defect and
  the nearest miss of the set, but it is a host-path race that exists today and is not created
  or changed by this phase. Left in the todo list.

</deferred>

---

*Phase: 22-container-execution-target*
*Context gathered: 2026-09-16*
