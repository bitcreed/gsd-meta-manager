# Phase 29: Master Prompt & Run Templates - Context

**Gathered:** 2026-10-19
**Status:** Ready for planning
**Mode:** unattended (operator unavailable; equivalent to `--auto`). Every decision not taken verbatim from
ROADMAP / REQUIREMENTS / Phase 27-28 hand-offs / spike evidence is tagged `[inferred — audit]`.

<domain>
## Phase Boundary

Ship the driver's contract inside gsd-mm. Today the launcher (Phase 28) takes the master prompt from a
required `--prompt-file`, and run dirs are built by hand from spike templates. This phase delivers:

- the **embedded, versioned master prompt** (port of spike-003 prompt v0.4), with its version recorded on the
  run (PRMT-01);
- **`gsd-meta-manager driver new <project> <slug>`**, which mints the run dir with a self-ignoring `.gitignore`,
  DIRECTION.md from the template and LOG.md carrying its grammar inline (PRMT-02, DRIVE-03);
- **explicit stop rules and their written precedence**, in the prompt and in the template's standing rules
  (PRMT-03);
- **startup reconciliation and `control` honouring**, proven by a scripted crash-and-relaunch and a mid-dispatch
  `control: pause` (PRMT-04);
- **`gsd-meta-manager next <project> --json`**, exposing the Phase 20 router as an advisory hint (PRMT-05);
- the **prompt half of DRIVE-01** (re-mapped; Phase 28 evidenced the launcher half in 28-09 d33).

Out of scope, owned elsewhere:
- the Driver tab on DIRECTION.md (list runs, launch/steer UI, edit the body): Phase 30;
- deleting/rewiring Phase 17/19/21 code (`drive`, envelope goal layer): Phase 31;
- launcher internals (caps, sandbox, auth, push): Phase 28, reused unchanged except where named below;
- the gap-round counter itself: Phase 27 store (D-26).
</domain>

<decisions>
## Implementation Decisions

### Musk pass (discuss:pre)
- No scope cuts. PRMT-01..05 and DRIVE-03 trace to REQUIREMENTS (2026-10-14 re-plan) and spike MANIFEST
  requirements; SC1-SC5 each restate an owned requirement; the carried risk is operator-accepted (2026-10-18).
  Lean lane not taken: several subsystems (embedded assets, CLI, driverctl store, launcher gate, router CLI,
  paid acceptance), well over 3 tasks. `musk milestone-check` reported `run: false`.

### Master prompt: embedding & versioning (PRMT-01, SC1)
- **D-01:** The prompt is a Markdown asset in the repo (e.g. `src/launcher/assets/MASTER-PROMPT.md`), compiled in
  with `include_str!`, plus a `pub const DRIVER_PROMPT_VERSION` in Rust. The asset carries the same version in
  its header comment and a test asserts the two agree. Placeholders keep the spike form (`{RUN_DIR}`,
  `{PROJECT_ROOT}`, `{MODE}`) and go through the existing Phase 28 render path. `[inferred — audit]` for the path.
- **D-02:** Version scheme: `driver-prompt/1.0.0` (semver; the spike drafts v0.1-v0.4 are history). A change to
  rules or the report contract bumps minor or major; wording-only bumps patch. The v0.4 changelog comment is
  replaced by a short in-file changelog starting at 1.0.0 that cites v0.4 as its base. `[inferred — audit]`
- **D-03:** The version is recorded on the run in **two** places: an additive frontmatter key
  `prompt_version` in the Phase 27 schema v1 emitter + typed reader (`#[serde(default)]`, no version bump — 27
  D-19 additive rule, same pattern as 28 D-30 `sandbox`), and a `prompt=` field on the launcher-written
  `RUN launch` LOG line. — **Reversibility:** costly — schema key persisted in run dirs and read by Phase 30.
  `[inferred — audit]`
- **D-04:** `--prompt-file` (Phase 28 D-03, today required) becomes an optional, **hidden** override. When used,
  the recorded version is `custom:<sha256-8 of the file bytes>`, so a run never claims the shipped version for a
  prompt it did not use. `[inferred — audit]`
- **D-05:** Content = spike-003 v0.4 (`.planning/spikes/003-divergent-gap-loop-validation/MASTER-PROMPT.md`)
  carried with its D1, D3, D1c, D2 and OQ-R1 clauses (MANIFEST requirement), plus the SC1 list made explicit as
  numbered hard rules: one fresh subagent per GSD step; the 11-field report (STATUS, ITEM, STEP, COMMITS, WHERE,
  CHANGED, CHECKS, DONE_WHEN, FINDINGS, NEEDS_HUMAN, SUGGESTED_NEXT), at most 15 lines, recorded verbatim;
  nested `isolation: "worktree"` pass-through; the async no-respawn rule (v0.4 Loop step 4 + worker brief); every
  record through `driverctl`. Plus the new stop-rule precedence block (D-12) and the advisory router hint (D-22).
- **D-06:** The report shape is already bounded by `driverctl` (`MAX_REPORT_LINES = 15`,
  `src/driverctl/protocol.rs:43`). No new rejection of field order or names is added: a rejected report pushes
  the driver to "fix" a worker's words, which breaks "verbatim". A golden test asserts the prompt's field list
  matches the 11 fields and the protocol limit. `[inferred — audit]`
- **D-07:** A golden/lint test over the embedded prompt asserts each SC1 rule and each stop rule is present
  (string anchors), every `driverctl` verb it shows parses against the real clap/protocol definitions (so the
  prompt cannot drift from Phase 27 D-01), and no AskUserQuestion / tmux / stdin instruction exists.

### `driver new` and the templates (PRMT-02, DRIVE-03, SC2)
- **D-08:** `gsd-meta-manager driver new <project> <slug>` joins the Phase 28 `driver` group (28 D-01). Run id
  `<yyyymmdd>-<slug>` with the **UTC** date; slug validated by Phase 27 `valid_run_id` on the composed id. An
  existing run dir with that id is **refused** (typed error naming it), never suffixed or overwritten.
  `[inferred — audit]` for UTC and refuse-on-collision.
- **D-09:** Creates `.planning/meta-manager/drivers/<run-id>/` in the main checkout with:
  - `.gitignore` containing `*` (self-ignoring; the project's own `.gitignore` is never edited — non-intrusive
    constraint);
  - `DIRECTION.md` from the embedded template: the human body (`## Direction` with `control: run|pause|stop`,
    goal, worker-model, run-budget, isolation, `### Standing rules`; `## Items` with one example item carrying
    intent, done-when, budget, start-with, after, hold, answer), based on spike-002 `templates/DIRECTION.md`;
  - `LOG.md` with the grammar comment inline (spike-002 `templates/LOG.md`, updated for the Phase 27/28
    events actually emitted, incl. `stalled` and the `prompt=` launch field).
  The initial tool frontmatter is produced by the **Phase 27 emitter** in its never-launched `starting` state
  (27 D-16 "template" fixture, 27 D-22 not-crashed rule), never hand-written. `[inferred — audit]`
- **D-10:** "One command from a ready DIRECTION.md" = after the human edits the body, `driver launch <project>
  <run-id>` with no `--prompt-file`. `driver new` prints the DIRECTION.md path and that exact launch command.
  It does not open an editor (body editing UI is Phase 30). `[inferred — audit]`
- **D-11:** DRIVE-03 ("machine-checkable plan the user can review before the run starts") is met by a
  **launch preflight lint** of the DIRECTION.md body: `control` is one of run|pause|stop; every item has an id,
  intent, done-when and a parseable budget; `after` references exist and are acyclic; no unfilled `{...}`
  template placeholder remains. Failure refuses the launch before any spend, naming each problem. The same lint
  is exposed as `driver new`'s follow-up `driver check <project> <run-id>` (read-only). The lint parses only the
  `- key: value` lines the template documents. `[inferred — audit]` for the separate `check` verb.

### Stop rules and precedence (PRMT-03, SC3)
- **D-12:** The prompt and the template's standing rules carry the **same** numbered list, with written
  precedence (first match names the park; listed highest first):
  1. `control: stop` / `control: pause` (human control) — ends the run, not a park;
  2. a launcher denial (`LAUNCHER CAP: …`) → `launcher-cap`, never retried;
  3. run budget → `run-budget`, ends the run;
  4. divergence → `divergence` (outranks gap-rounds when both hold — spike D1c);
  5. gap rounds (at most 2 closure rounds per phase, or the item's lower `gap-rounds`) → `gap-rounds`;
  6. item budget (dispatches / minutes) → `budget`;
  7. no progress (two consecutive dispatches, no new commit, no change in done-when evidence) → `no-progress`;
  8. needs-human (scope change, hardware, credentials, sign-off, contradiction) → `needs-human`.
  Ordering 2-8 is `[inferred — audit]`; divergence-over-gap-rounds is spike evidence.
- **D-13:** **OQ-R1 default, written out:** a human `answer:` lifts only a `needs-human` park. Parks under rules
  2-7 are lifted only by a budget edit — raising the item's budget, the run budget or the launcher caps — and
  stay parked when only `answer:` is filled. A `gap-rounds` / `divergence` park also logs `needs-human` (spike
  D2) so the human knows a budget edit plus an answer is what resumes it. `[inferred]` (from REQUIREMENTS).
- **D-14:** The prompt's gap rule reads the launcher's coalesced count (`driverctl show` → `caps.gap_rounds`) as
  authoritative (v0.4 wording, Phase 27 D-26, 45 s coalescing). The launcher gap cap stays a backstop: default
  launch cap **L = 2** (prompt stops at the third `gaps_found`, launcher denies at L+1 = 3 and kills at
  L+2 = 4 — 28 D-14 numbers unchanged). `[inferred — audit]` for L = 2 as the shipped default.
- **D-15:** GSD's own stall-watch (`planner.stall_detection_enabled`) is **not** set by gsd-mm (non-intrusive
  constraint); the template mentions it as an optional project setting the human may enable. The early-worker
  case stays covered by the v0.4 SendMessage rule plus the Phase 28 Stop guard. `[inferred — audit]`
- **D-16:** Folded todo "verify-work gate policy configurable": the template's standing rules carry a
  human-verification line with **`defer` as the default** (a `human_needed` verification parks `needs-human`),
  and `skip` available only as an explicit, human-written standing rule the driver records in its `verified`
  evidence. `auto` (DRIVE-07) stays deferred. No router change in this phase. `[inferred — audit]`

### Reconciliation & control (PRMT-04, SC4)
- **D-17:** Reconciliation stays model-side (prompt Startup step 3, `recovered --action done|resume|reset|park`),
  but "exactly one" is made **mechanical** in the Phase 27 store: after a launch that follows a launch with
  `running` items, the store rejects (`REJECTED`, rc 2) a `dispatched` event while any item that was `running`
  at launch start lacks a `recovered` event, and rejects a second `recovered` for the same item in that launch.
  `[inferred — audit]` — **Reversibility:** costly — changes store acceptance rules replayed by Phase 27 fixtures
  (they must still pass; spike LOGs that reconcile correctly are unaffected).
- **D-18:** `control` is honoured **before the next dispatch**, with a launcher-side backstop: the Phase 28
  PreToolUse gate also denies a driver-level `Agent|Task` dispatch when the DIRECTION body reads
  `control: pause|stop` (deny message `LAUNCHER CAP: control=<value>`), so a model that misses the re-read still
  cannot start new work. The in-flight worker finishes, its report is recorded, then the driver ends
  `--reason pause|stop`. SC4's "ends the run after the current item" is read as "after the current dispatch"
  (PRMT-04 wording, which is stricter). `[inferred — audit]` — flagged: SC4 and PRMT-04 differ in wording.
- **D-19:** The SC4 proof is two-layered:
  - **deterministic** (default `cargo test`, CI-safe): a scripted fake driver (stub `claude` replaying a
    recorded stream, the Phase 28 e2e pattern) is SIGKILLed mid-dispatch, relaunched, and must emit exactly
    one `recovered` per running item before it can dispatch (D-17 enforces it); a fixture matrix covers each of
    done / resume / reset / park. A pause written mid-dispatch is denied at the next dispatch by the gate (D-18);
  - **live, cheap** (opt-in `#[ignore]`, Haiku driver, cents): the embedded prompt against a scratch project,
    SIGKILL during a worker, relaunch, assert one `recovered` per running item; and a `control: pause` edit
    while a worker runs, assert `RUN end reason=pause` after that report with no further `dispatched`.
  `[inferred — audit]`

### Router hint (PRMT-05, SC5)
- **D-20:** New top-level `gsd-meta-manager next <project> [--target-phase N] --json`. It reads project state
  with the existing `state_reader`, calls `driver::router::decide` (src/driver/router.rs:901) and prints the
  `Decision` as JSON with an explicit `"advisory": true` field and the router's reason. Without
  `--target-phase` it targets the first incomplete phase of the current milestone. `[inferred — audit]`
- **D-21:** `next` is strictly read-only (no lock, no journal, no file writes) and accepts a registered alias or
  a path, so it works inside the sandbox where the per-run config dir has no gsd-mm registry; host root is
  read-only there, so the binary is reachable via the existing `bin/` PATH entry. Gating follows whatever the
  `driver` group uses (`GSDMM_EXPERIMENTAL_FEATURES`); planner checks. `[inferred — audit]`
- **D-22:** The prompt cites `next --json` once, in the Loop "choose the step" item, as information equal in
  weight to a worker's `SUGGESTED_NEXT`: never binding, never a substitute for checking done-when. A prompt lint
  anchor asserts the word "advisory" accompanies it.

### Acceptance, paid runs and the carried risk (SC3, DRIVE-01)
- **D-23:** The 26.1 carried risk's "prompt-rule-disabled run where the launcher backstop actually trips" was
  closed in Phase 28 (28-09 d33 PASS: organic deny under real Opus with gap cap 0, scope kill verified). Phase
  29 **reuses that evidence** for the backstop half rather than re-buying it, and adds what 28 could not show:
  the **embedded** prompt with its rules **enabled**, paired with the launcher cap at L = 2, under a real Opus
  driver on the spike-003 **seed v2** scratch project. Pass: the driver parks `gap-rounds` or `divergence`
  (with `needs-human`) at the third `gaps_found` with **no** `dispatch-denied` line, launched via `driver new` →
  edit → `driver launch` with no `--prompt-file`, and `prompt_version: driver-prompt/1.0.0` recorded. This one
  run is also the DRIVE-01 prompt-half evidence (goal stated once in DIRECTION.md, multiple GSD commands, terminal
  outcome reported). `[inferred — audit]` — reusing d33 for the disabled half is an inference about what the
  carried risk requires.
- **D-24:** A cheap Haiku pair (cents) also runs the embedded prompt with its gap rules stripped (a test-only
  variant via the hidden `--prompt-file`) against the L4/L7 harness shape, to show the embedded-prompt +
  launcher integration still trips the cap. OQ-R1 gets its own Haiku case: a `gap-rounds`-parked item whose
  `answer:` is filled without a budget edit must not be re-dispatched; after a budget edit it must be.
  `[inferred — audit]`
- **D-25:** Paid-run discipline (26.1 / 28-09 pattern): rubric pre-registered in a committed file (by sha)
  before any spend; one paid Opus run plus at most one retry; campaign ledger charging killed launches their
  full `--max-budget-usd`; executor stops at a cost-consent checkpoint before spending. **Proposed ceiling:
  $25 for the phase** (d33 alone ledgered $9.00; ~$10 per Opus attempt × 2 + Haiku cases). **Needs operator
  sign-off before any paid run.** If both Opus attempts miss the pass rule, record ADJUST with evidence — no loop.
  `[inferred — audit]`

### Claude's Discretion
- Asset location and module layout (`src/launcher/assets/` vs a new `src/driver_templates/`), and whether `next`
  lives in `driver_cli.rs` or a new `next_cli.rs`.
- The JSON shape of `next --json` beyond `advisory`, the chosen command, target phase and reason.
- Plan split and waves: prompt asset + lint, `driver new` + templates + preflight lint, store/gate changes
  (D-17/D-18), and `next` are largely independent; the live/paid acceptance depends on all of them.

### Folded Todos
- "The verify-work gate policy must be configurable — skip | defer | auto-validate" (0.9, driver) — folded as
  the template standing-rule line in D-16 (defer default, explicit skip; auto deferred with DRIVE-07).
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/ROADMAP.md` §"Phase 29: Master Prompt & Run Templates" — goal, SC1-5, carried-risk paragraph
- `.planning/REQUIREMENTS.md` §"Master Prompt & Templates (PRMT): Phase 29" (PRMT-01..05), DRIVE-01 and
  DRIVE-03 traceability rows (re-mapped), CTRL-08 (superseded by PRMT-02/03)
- `.planning/spikes/MANIFEST.md` — the queue-driven-master-agent requirements and every spike-003/26.1 carry
  aimed at Phase 29 (D1/D3/D1c/D2/OQ-R1 wording, stall-watch, seed v2, self-limiting)

### The prompt and templates being ported
- `.planning/spikes/003-divergent-gap-loop-validation/MASTER-PROMPT.md` — v0.4, the base for 1.0.0
- `.planning/spikes/003-divergent-gap-loop-validation/experiment/MASTER-PROMPT.run2.md` — rules-disabled variant
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/templates/DIRECTION.md`, `templates/LOG.md`
- `.planning/spikes/003-divergent-gap-loop-validation/templates/DIRECTION.run1.md`, `DIRECTION.run2.md`
- `.planning/spikes/003-divergent-gap-loop-validation/experiment/` — seed v2 scratch project and run.sh

### Phase 27 (store) and Phase 28 (launcher) this phase builds on
- `.planning/phases/27-driverctl-driver-frontmatter/27-CONTEXT.md` — D-01 CLI frozen, D-05 status enum, D-16
  template fixture, D-19 additive-key rule, D-22 `starting` not crashed, D-26 gap counter
- `.planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md` — D-01 `driver` group, D-03 `--prompt-file`
  hand-off, D-12..D-14 gate/cap numbers, D-17 Stop guard, D-30 additive frontmatter key pattern, D-33/D-34
- `.planning/phases/28-sandboxed-driver-launcher/28-09-SUMMARY.md` — d33 PASS evidence and costs, l3 FAIL carry
- `.planning/phases/28-sandboxed-driver-launcher/28-VERIFICATION.md` — human_needed items (browser login, macOS)
- `.planning/phases/28-sandboxed-driver-launcher/28-D33-RUBRIC.md` — pre-registered rubric pattern to copy
- `.planning/phases/26.1-driver-validation-spike-003-re-run/26.1-VERIFICATION.md` — VALID-02 ADJUST origin

### Reused code
- `src/launcher/run.rs` (prompt render, `--prompt-file`), `src/launcher/gate.rs` (`decide_dispatch`)
- `src/driverctl/protocol.rs` (`MAX_REPORT_LINES`, `recovered`), `src/driverctl/store.rs`
- `src/driver/direction.rs` (splice, `write_body`, `DIRECTION.lock`)
- `src/state_reader/driver_frontmatter.rs` (schema v1 emitter/reader, additive keys)
- `src/driver/router.rs` (`decide`), `src/cli.rs` (`DriverAction`), `src/driver_cli.rs`
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 28 launcher already renders `{RUN_DIR}/{PROJECT_ROOT}/{MODE}` into a prompt template read from
  `--prompt-file` (`src/launcher/run.rs:497`, `:1163`) — swap the source to the embedded asset.
- Phase 27 emitter/reader handle additive frontmatter keys (`sandbox` precedent,
  `src/state_reader/driver_frontmatter.rs:383`, `:600`).
- `driverctl` already validates the report length and the `recovered` actions (`protocol.rs:43`, `:554`).
- `driver::router::decide(state, target_phase) -> Decision` is pure and replay-tested (Phase 20).
- `scripts/launcher-acceptance.sh` + `tests/launcher_acceptance.rs` — the live/paid harness and ledger to extend.

### Established Patterns
- Pure builders + golden tests run everywhere; live/paid checks opt-in `#[ignore]` with captured evidence and a
  pre-registered rubric; cost-consent checkpoint before spend.
- Typed refusals; additive schema keys with `#[serde(default)]`, no version bump.
- `cargo test --no-fail-fast`; the local git-version witness failure is expected.

### Integration Points
- `src/cli.rs` `DriverAction`: add `New`, `Check`; top-level `Commands::Next`.
- `src/launcher/gate.rs`: control-based deny (D-18).
- `src/driverctl/store.rs`: reconcile-before-dispatch rule (D-17).
- LOG `RUN launch` line and frontmatter: `prompt_version` (D-03).
</code_context>

<specifics>
## Specific Ideas

- Measured: real Opus self-parked at gap count 2 even with gap rules removed (`r261-run2b`), and parked
  `divergence` at the third `gaps_found` with rules on (`r261a`). The rules-enabled pass rule in D-23 accepts
  either `gap-rounds` or `divergence` for that reason.
- Seed v2 must advance its moving target on the `gsd-verifier` spawn, not on the executor's SUMMARY commit.
</specifics>

<deferred>
## Deferred Ideas

- l3 harness pacing fix from 28-09 (driver must outlive gap count 3; ~$0.02 on Haiku) — a quick task, not
  Phase 29 scope `[inferred — audit]`.
- `auto` human-verification mode (DRIVE-07) — Future Requirements.
- Opening `$EDITOR` from `driver new`, and any DIRECTION body editing UI — Phase 30.
- Setting GSD `planner.stall_detection_enabled` on driven projects — never by gsd-mm (non-intrusive).
- Codex runtime under the launcher — out of scope (Claude only).

### Reviewed Todos (not folded)
- "Driver tab renders blank pipeline row at terminal heights 8-13" (0.9, ui) — Phase 30.
- "Codex runtime: remainder after the MVP slice" (0.7) — not the Claude driver.
- "Dashboard renders quoted VERIFICATION status as Unknown…" (0.6) — dashboard reader (27 D-27).
- "Make queue input truly multi-line" (0.3) — UI, unrelated.
</deferred>

---

*Phase: 29-master-prompt-run-templates*
*Context gathered: 2026-10-19*
