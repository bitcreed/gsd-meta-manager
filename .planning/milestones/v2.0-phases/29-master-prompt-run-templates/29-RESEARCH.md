# Phase 29: Master Prompt & Run Templates - Research

**Researched:** 2026-10-19
**Domain:** Rust CLI + embedded prompt asset + driverctl store/gate rules + paid real-model acceptance
**Confidence:** HIGH for code seams (every one read this session); MEDIUM for live-model behaviour and cost (measured on earlier runs, not re-run)

<user_constraints>
## User Constraints (from CONTEXT.md)

> Copied verbatim from `29-CONTEXT.md` `<decisions>` and `<deferred>`. **Coordinator corrections (authoritative
> over CONTEXT.md where they conflict)** follow the copy.

### Locked Decisions

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

### Deferred Ideas (OUT OF SCOPE)
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

### Coordinator corrections (authoritative over the copy above)
1. **The l3 deferred item is STALE.** The l3 harness pacing fix already landed in Phase 28 (commits `e087343`,
   `8e97eb1`; evidence `63fed30`, "l3 acceptance evidence — PASS (deny after gap count 3)"). It is neither planned
   nor researched here. [VERIFIED: `git log` this session shows `63fed30 test(28): l3 acceptance evidence — PASS
   (deny after gap count 3)`, `8e97eb1 fix(28): l3 gap writer paces above the 45 s coalesce window…`,
   `e087343 fix(28): pace the l3 gap writer…`]
2. **Paid real-model runs are planned against a proposed $25 ceiling, every paid step behind a cost-consent
   checkpoint** (operator sign-off pending). See `## Paid-Run Plan` for the cheapest sufficient set and the
   offline-first gates.
3. **Human unavailable:** every inferred choice in this research is marked `[inferred — audit]`.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PRMT-01 | Master prompt ships embedded and versioned, version recorded on the run; carries the 5 SC1 rules | §Pattern 1 (asset + const + render seam), §Pattern 2 (`prompt_version` via `LaunchSpec` → frontmatter + `RUN launch` line), §Pattern 3 (prompt lint), Pitfalls 1-4 |
| PRMT-02 | `driver new` mints `<yyyymmdd>-<slug>`, creates run dir with self-ignoring `.gitignore`, DIRECTION.md, LOG.md with grammar inline | §Pattern 4 (`driver new`), Pitfalls 5-8, §Code Examples "driver new" |
| PRMT-03 | Stop rules + precedence in prompt and template; OQ-R1 default | §Pattern 3 (shared precedence block, equality test), Open Question 1 (gap-rounds lift wording) |
| PRMT-04 | Startup reconciliation to exactly one of done/resume/reset/park; `control` edit honoured before next dispatch | §Pattern 5 (store D-17 rule), §Pattern 6 (gate D-18 control deny), Pitfalls 9-12, §Validation (deterministic + Haiku live) |
| PRMT-05 | `next <project> --json` exposes Phase 20 router as advisory hint, cited as advisory in prompt | §Pattern 7 (`next`), Pitfalls 13-15 |
| DRIVE-03 (re-mapped) | Machine-checkable plan the user reviews before the run starts | §Pattern 4 (`driver check` + launch preflight lint), Pitfalls 6, 16 |
| DRIVE-01 (prompt half, re-mapped) | Goal stated once; driver pursues it across multiple GSD commands | §Paid-Run Plan run O1 (one Opus run is both D-23 and DRIVE-01 evidence) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading must not require running Claude/GSD**; **non-intrusive** (never interfere with running GSD instances; the
  run dir self-ignores, the project's own `.gitignore` is never edited); **portability** (no hard-coded user setup).
- Stack is fixed: Rust 1.89+ (MSRV in `Cargo.toml`: `rust-version = "1.89"`), clap 4, serde/serde_json, `sha2`, `chrono`,
  `regex`, `tempfile`, `rustix` — all already dependencies. **No new crate is needed for this phase.**
- Logs go to a file via `tracing`, never stdout (ratatui owns the terminal). Note: `next` must not even create that file
  (D-21) — see Pitfall 13.
- `std::sync::Mutex` must not be held across `.await` — irrelevant here: the launcher, `driver *` and `next` are sync.
- **GSD workflow enforcement:** edits only through GSD commands (the executor's plans).
- **Testing:** `rtk proxy cargo test --no-fail-fast` (rtk output filtering and fail-fast can fake results; memory notes).
  The lone `envelope::policy` git-version witness failure is expected locally (this machine: git 2.53.0; constant pinned
  for CI).
- **Release process** (not this phase's job) uses `./scripts/pre-tag-check.sh --container`; nothing here may break its
  five gates (tag/version, MSRV, release build, tests, clippy `-D warnings`).
- Repo guard: `spawn_seam_guard` forbids `#[serde(deny_unknown_fields)]` anywhere under `src/` (see `protocol.rs:105-109`
  comment) — the new `next` JSON and lint types must not use it.

## Summary

Phase 29 is mostly **wiring onto seams that already exist**. The Phase 28 launcher already renders `{RUN_DIR}`,
`{PROJECT_ROOT}`, `{MODE}` into a template (`render_prompt`, `src/launcher/sandbox.rs:373-383`) and only needs its source
swapped from a required `--prompt-file` to an `include_str!` asset with a version const. The Phase 27 store already has the
per-launch additive-key pattern (`sandbox`, cleared or set on every `begin_launch`) that `prompt_version` should copy
exactly, and already writes the `RUN | launch` line whose kv list gets one more entry. The Phase 28 gate is a pure function
(`decide_dispatch`) that needs one more input (the DIRECTION `control` word). The Phase 20 router is pure and replay-tested;
`next` is a thin read-only CLI around `parse_project_state` + `router::decide`. The v0.4 prompt's `driverctl` block is
**byte-identical** to the v0.2 fixture that `tests/driverctl_client.rs` already lifts and runs against the real CLI, so the
D-07 "verbs parse" guard is a re-pointing of an existing test, not new machinery.

The work that is genuinely new: (a) the `driver new` / `driver check` verbs and a DIRECTION body **lint** (the store's
`parse_direction` is deliberately lenient and silently ignores bad items, so the lint must be a separate, stricter parser);
(b) the D-17 store rule (track items `running` at launch start; reject `dispatched` until each has one `recovered`; reject a
second `recovered` per launch) — which must *not* reject a first `recovered` for a non-running item, because an existing
Phase 27 test does exactly that; (c) the 1.0.0 prompt text itself; (d) the paid acceptance harness cases.

The paid budget is dominated by one real-Opus run (D-23 + DRIVE-01). The closest measured analogue, `r261a` (v0.4 rules
on, real Opus, seed with moving target, parked `divergence` at the third `gaps_found`, no deny), cost **$7.5053** with
workers at 88.5 % of it; `--max-budget-usd` counts subagent spend, so a $10 per-attempt budget gives ~33 % headroom. Two
Opus attempts at $10 plus a ≤ $2 Haiku set fits the proposed $25 ceiling with ~$3 margin.

**Primary recommendation:** Build and prove everything offline first (prompt lint, `driver new`/`check`, store D-17, gate
D-18, `next`, fake-driver crash/pause e2e, harness selftest + zero-spend rehearsal), then run four Haiku cases (≈ cents
each, ≤ $2 ledgered), then exactly one Opus run at `--max-budget-usd 10` behind a committed pre-registered rubric and a
cost-consent checkpoint, with at most one retry.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Embedded versioned prompt | Host launcher (`src/launcher/`, compile-time asset) | — | The launcher renders and passes the prompt as `claude` argv; the sandbox never sees the asset file |
| Recording `prompt_version` | driverctl store (`begin_launch`) | Frontmatter schema (`driver_frontmatter.rs`) | The store is the single writer of `state.json`, LOG and frontmatter (27 D-01) |
| `driver new` / `driver check` / launch preflight | Host CLI (`driver_cli.rs`) over a pure lint module | Store emitter (initial frontmatter) | Writes into the main checkout before any launch; must run outside the sandbox |
| Stop-rule precedence | Prompt (model-side) | Template standing rules (human-visible copy) | Rules are model behaviour; launcher caps are the backstop |
| "Exactly one recovered" | driverctl store (acceptance rule) | Prompt Startup step 3 | Mechanical rule = store; choosing the action = model |
| `control: pause/stop` honoured | Prompt (re-read before dispatch) | Launcher gate (PreToolUse deny) | Backstop lives outside the sandbox where the model cannot bypass it |
| Router hint | Host binary `next` (read-only) invoked from inside the sandbox | Prompt (cites as advisory) | Pure function of on-disk state; must work with ro `$HOME` |
| Paid acceptance | Host bash harness (ledger, prereg gate) | CI-safe Rust tests of the harness | Pattern established by 28-09 |

## Standard Stack

### Core (all already in `Cargo.toml` — nothing to install)
| Library | Version (Cargo.toml) | Purpose | Why Standard |
|---------|---------|---------|--------------|
| clap (derive) | `4` | `DriverAction::New`, `DriverAction::Check`, `Commands::Next`; hidden `--prompt-file` | Existing CLI framework; `#[arg(long, hide = true)]` pattern used throughout `cli.rs` |
| sha2 | `0.11.0` | `custom:<sha256-8>` of a `--prompt-file` | Already used by `serve::hash8` (`Sha256::digest`) [VERIFIED: src/driverctl/serve.rs:248-251] |
| chrono | `0.4` | UTC `yyyymmdd` for the run id | Already used by store/launcher (`Utc::now()`) |
| serde_json | `1` | `next --json` output | Build a `serde_json::json!` object (Decision is not `Serialize`) |
| regex | `1` | DIRECTION lint | Same crate `direction.rs` uses |
| tempfile | `3` | Tests | Existing |

### Supporting (in-repo modules to reuse — do not re-implement)
| Module | What to reuse |
|--------|---------------|
| `launcher::sandbox::render_prompt` | Placeholder substitution + `Begin now:` line |
| `launcher::run::check_prompt` | Empty / oversize (`MAX_PROMPT_BYTES`) refusal — run it on the embedded asset too |
| `driverctl::serve::{valid_run_id, validate_run_dir, DRIVERS_REL}` | Run id regex and run-dir containment |
| `driver::direction::{split_frontmatter, parse_direction, body_sane}` | Frontmatter/body split; the store-side item view |
| `state_reader::driver_frontmatter::DriverFrontmatter::emit` | Initial `starting` frontmatter for `driver new` |
| `driverctl::log::parse_log` | Test: the LOG.md template parses to zero events |
| `state_reader::parse_project_state` + `driver::router::decide` | `next` |
| `launch_target::resolve_launch_target` + `config::load_config` | Alias resolution (read-only; `load_config` returns `Config::new()` for a missing file, never writes) |
| `tests/launcher_support/mod.rs` `Fixture` + fake `claude` scenarios | Deterministic crash/pause e2e |
| `scripts/launcher-acceptance.sh` (`prereg_gate`, `ceiling_ok`, `record_costs`, `evidence_gates`, `host_preconditions`) | Paid harness |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `include_str!` asset | Prompt installed under data dir | Rejected by CONTEXT (version drift, second artifact) |
| Separate stricter lint parser | Tighten `parse_direction` | **Do not** tighten `parse_direction`: the store and 17 spike fixtures depend on its leniency (it "never fails") |
| New Phase-29 harness script | Add cases to `launcher-acceptance.sh` | That script hard-codes `PH=…28…`, `MAX_CEILING=20`, the 28 ledger and `--prompt-file` on every launch; mixing ledgers is error-prone. See Pattern 8. |

**Installation:** none.

**Version verification:** no new packages; existing versions read from `Cargo.toml` this session.

## Package Legitimacy Audit

No external package is added by this phase (all needs are met by existing dependencies: clap, sha2, chrono, serde_json,
regex, tempfile, rustix).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 human ──(1) driver new <proj> <slug>──▶ [driver_cli: new]
                                          │ resolve alias (registry, ro)
                                          │ mint <UTC yyyymmdd>-<slug>, valid_run_id, refuse if exists
                                          ▼
                       <proj>/.planning/meta-manager/drivers/<run-id>/
                         .gitignore("*")   DIRECTION.md(= emitter frontmatter[starting] + template body)
                         LOG.md(header + grammar comment, 0 events)
                                          │ prints DIRECTION path + exact launch command
 human edits body ───────────────────────▶│
 human ──(2) driver check (optional) ────▶ [lint(body)] ─▶ problems list / OK   (read-only)
 human ──(3) driver launch <proj> <id> ──▶ [launch preflight: lint(body)] ──fail──▶ refuse, nothing written
                                          │ ok
                                          ▼
                     prompt = embedded asset (or hidden --prompt-file)  → version = driver-prompt/1.0.0 | custom:<sha8>
                     render_prompt(RUN_DIR, PROJECT_ROOT, MODE) → check_prompt
                                          ▼
                     store.begin_launch(LaunchSpec{…, prompt_version})
                        ├─ awaiting_recovery := items with state=running        (D-17)
                        ├─ extra.prompt_version := v  (frontmatter `prompt_version`)
                        └─ LOG: RUN | launch | n=… | mode=… | model=… | prompt=v
                                          ▼
                     bwrap+scope ─▶ claude -p (driver, embedded prompt)
                        │  driverctl log start / reconcile / recovered ─▶ store
                        │       store rejects `dispatched` while awaiting_recovery ≠ ∅   (D-17)
                        │       store rejects 2nd `recovered` for one item this launch   (D-17)
                        │  gsd-meta-manager next <PROJECT_ROOT> --json ─▶ JSON {advisory:true,…}   (read-only)
                        │  Agent(worker) ─▶ PreToolUse hook ─▶ store.gate:
                        │        tripped? ─▶ deny | control∈{pause,stop}? ─▶ deny "control=…" (D-18)
                        │        gap cap L+1? ─▶ deny | dispatch cap? ─▶ deny | allow
                        │  worker report (≤15 lines) ─▶ driverctl log report (verbatim) ─▶ verified ─▶ status
                        └─ end --reason all-done|all-parked|pause|stop|run-budget|fresh-exit
```

### Recommended Project Structure

```
src/
├── launcher/
│   ├── assets/
│   │   └── MASTER-PROMPT.md          # driver-prompt/1.0.0 (include_str!)
│   ├── prompt.rs                     # DRIVER_PROMPT_VERSION, MASTER_PROMPT, PromptSource{Embedded,Custom}, version_of()
│   └── (run.rs, gate.rs edited)
├── driver_templates/                 # [inferred — audit] (CONTEXT discretion)
│   ├── mod.rs                        # mint_run_id(), create_run_dir(), TEMPLATE_* consts
│   ├── lint.rs                       # lint_direction(body) -> Vec<Problem>  (pure)
│   └── assets/{DIRECTION.md, LOG.md}
├── next_cli.rs                       # `next` (dispatched in main BEFORE init_tracing)
└── driver_cli.rs                     # + new_main, check_main; launch_main: optional prompt, preflight
tests/
├── driver_prompt.rs                  # golden/lint over the embedded prompt
├── driver_new.rs                     # binary e2e: new, check, collision, .gitignore staging proof
├── launcher_reconcile.rs             # fake driver: SIGKILL mid-dispatch + relaunch matrix; pause mid-dispatch
├── next_cli.rs                       # JSON shape, read-only proof, alias/path, default target
└── prompt_acceptance.rs              # CI-safe proofs of the Phase 29 harness (no spend)
tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md   # test-only variant (D-24), derived + checked
scripts/prompt-acceptance.sh          # Phase 29 paid cases (own ledger, ceiling 25)
```

### Pattern 1: Embedded asset + version const + render seam (PRMT-01, D-01, D-04)

**What:** One `const MASTER_PROMPT: &str = include_str!("assets/MASTER-PROMPT.md")`, one
`pub const DRIVER_PROMPT_VERSION: &str = "driver-prompt/1.0.0"`, a header comment in the asset
(`<!-- driver-prompt/1.0.0 … -->`), and a test that extracts the header version and compares.
`LaunchOptions.prompt_file: PathBuf` becomes `Option<PathBuf>`; `launch()` and `launch_detached()` resolve the template as
`match prompt_file { None => (MASTER_PROMPT.to_owned(), DRIVER_PROMPT_VERSION.to_owned()), Some(p) => (read(p), format!("custom:{}", sha8(bytes))) }`
and pass both through `check_prompt` (run it on the embedded asset too — cheap and keeps one code path).

Evidence for the seam:
- `render_prompt` substitutes exactly `{RUN_DIR}`, `{PROJECT_ROOT}`, `{MODE}` and appends `Begin now: run Startup, then the
  Loop, for RUN_DIR=…` [VERIFIED: src/launcher/sandbox.rs:373-383, quoted:
  `.replace("{RUN_DIR}", &run_dir)` / `.replace("{PROJECT_ROOT}", &project.display().to_string())` / `.replace("{MODE}", mode)`].
- `MAX_PROMPT_BYTES` is `120 * 1024` [VERIFIED: src/launcher/sandbox.rs:339, `pub const MAX_PROMPT_BYTES: usize = 120 * 1024;`];
  v0.4 is 12.8 KB, so a 1.0.0 of ~16-20 KB is far below it.
- `--prompt-file` is today `#[arg(long)] pub prompt_file: PathBuf,` [VERIFIED: src/cli.rs:498-500] → becomes
  `#[arg(long, hide = true)] pub prompt_file: Option<PathBuf>`.
- `launch_argv` pushes `--prompt-file` unconditionally [VERIFIED: src/launcher/run.rs:1078-1079, `argv.push("--prompt-file".into());
  argv.push(opts.prompt_file.as_os_str().to_owned());`] → must push only when `Some`, and the existing
  `launch_argv_round_trips_through_the_real_parser` / `launch_argv_carries_every_flag_and_the_daemon_markers` tests
  (run.rs:1584-1642) must be updated for both shapes.
- `launch_detached` canonicalizes and reads the prompt file in the parent before spawning the daemon
  [VERIFIED: src/launcher/run.rs:1155-1171] → for `None`, skip the canonicalize and check the embedded asset.

### Pattern 2: `prompt_version` as a per-launch additive key (D-03)

Copy the `sandbox` mechanics exactly:
- `LaunchSpec` gains `pub prompt_version: Option<String>` (it is `#[derive(Default)]` [VERIFIED: src/driverctl/store.rs:264-277]).
- `begin_launch` sets or **removes** `extra["prompt_version"]` per launch, exactly like `EXTRA_SANDBOX`
  [VERIFIED: src/driverctl/store.rs:837-849 — "D-30: the posture belongs to THIS launch. A key left by an earlier launch …
  must not describe it."], and adds `("prompt", spec.prompt_version.as_deref().map(header_safe))` to the `RUN | launch` kv
  [VERIFIED: src/driverctl/store.rs:894-901, today `vec![("n", …), ("mode", Some(launcher_mode)), ("model", model)]`].
- `DriverFrontmatter` gains `#[serde(default)] pub prompt_version: Option<String>`, emitted **only when set**, right after
  `sandbox_missing`, like `sandbox` [VERIFIED: src/state_reader/driver_frontmatter.rs:379-386 and 597-605]. `frontmatter()`
  reads it from `state.extra` like `sandbox` [VERIFIED: src/driverctl/store.rs:715-731]. No schema version bump (D-19 rule
  in the module header, driver_frontmatter.rs:38-44).
- `driverctl serve` passes `None` (it is not the sandbox launcher) → key removed, line carries no `prompt=`.

### Pattern 3: The prompt lint/golden test (D-06, D-07, D-12, D-22)

Tests over `MASTER_PROMPT` (the constant, not a file path):
1. **Version agreement:** header comment version == `DRIVER_PROMPT_VERSION`.
2. **SC1 anchors** (each a numbered hard rule with a stable phrase): one fresh subagent per GSD step; report fields;
   `isolation: "worktree"`; no `run_in_background` + SendMessage-to-same-worker no-respawn; "every record goes through
   `driverctl`".
3. **Report contract:** the worker brief lists exactly the 11 field names in order (`STATUS:` … `SUGGESTED_NEXT:`) and the
   phrase "at most {MAX_REPORT_LINES} lines" uses the protocol constant [VERIFIED: src/driverctl/protocol.rs:43,
   `pub const MAX_REPORT_LINES: usize = 15;`]. `FINDINGS:` must stay at line start — the store extracts the count with
   `strip_prefix("FINDINGS:")` [VERIFIED: src/driverctl/protocol.rs:441-452].
4. **driverctl verbs parse:** re-use the lifter in `tests/driverctl_client.rs:611-724` (it lifts the fenced block under
   `## Recording events: \`driverctl\` (the only way)`, replaces `<a|b>` with the first alternative and specific prose
   placeholders, and runs each line through the real binary) against the embedded prompt as a second subject. The v0.4
   block is byte-identical to that fixture's block [VERIFIED: `diff` of the two `## Recording events`…`## Hard rules`
   sections this session printed no difference], so keep the heading text and the placeholder spellings
   (`"<one GSD command>"`, `"<short, one line>"`, `"<one line>"`, `[--rule <rule>]`) or extend the lifter.
5. **Every `--rule` / `--reason` / `--action` word in the prompt is in the protocol enums:**
   `RULE_ENUM` = `"budget", "gap-rounds", "divergence", "no-progress", "needs-human", "scope", "run-budget", "launcher-cap"`
   [VERIFIED: src/driverctl/protocol.rs:46-55]; `END_REASONS` words `"all-done"`, `"all-parked"`, `"pause"`, `"stop"`,
   `"run-budget"`, `"fresh-exit"` [VERIFIED: protocol.rs:59-66]; `RECOVER_ACTIONS` = `&["done", "resume", "reset", "park"]`
   [VERIFIED: protocol.rs:75].
6. **Precedence block equality:** the numbered 8-rule list appears byte-identically in the prompt and in the DIRECTION
   template's `### Standing rules` (extract between explicit marker lines, e.g. `<!-- stop-rules:begin -->` /
   `<!-- stop-rules:end -->`, in both assets).
7. **Negative anchors done right:** see Pitfall 2 — "no AskUserQuestion / tmux / stdin instruction" must accept the
   *prohibitions* v0.4 already contains.
8. **Advisory anchor:** the single `next … --json` mention is on a line (or sentence) that also contains "advisory".
9. **Stripped variant (D-24) is derived, not hand-edited:** `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md` must equal
   a deterministic transform of `MASTER_PROMPT` (remove the gap-rounds and divergence rule items, add the v0.4-run2 sentence
   "Gap closure has no round limit in this run…"), mirroring `make_variant.py --check` from spike 003. The test fails if the
   prompt changes without regenerating the variant.

### Pattern 4: `driver new`, templates, `driver check`, launch preflight (PRMT-02, DRIVE-03)

`driver new <project> <slug>`:
1. `resolve_project` (registry) → project root (host only).
2. `run_id = format!("{}-{slug}", Utc::now().format("%Y%m%d"))`; refuse unless `valid_run_id(&run_id)`
   (`RUN_ID_RE = r"^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$"` [VERIFIED: src/driverctl/serve.rs:98]; the 9-char date prefix leaves
   ≤ 71 chars for the slug).
3. Containment: refuse when `.planning`, `.planning/meta-manager` or `.planning/meta-manager/drivers` is a symlink, or when
   the canonical drivers dir is not under the canonical project (Pitfall 7). Then `create_dir_all(drivers)` and
   **`fs::create_dir(run_dir)`** (not `_all`) so `EEXIST` is the atomic collision refusal (typed error naming the dir).
4. Files with `OpenOptions::new().write(true).create_new(true)`: `.gitignore` = `"*\n"`, `LOG.md` = template, `DIRECTION.md`
   = `fm.emit()` + template body where `fm` is the never-launched `starting` block (status `starting`, `launches: 0`, no
   connections, null `last_event`, items pending) built from the same code path as `RunStore::frontmatter()` — factor that
   body into a pure `fn frontmatter_for(state: &RunState, direction: &DirectionInfo) -> DriverFrontmatter` and call it with
   `RunState::new(run_id, project, now)`. Do **not** write `state.json` (the store creates it on first open:
   `open_with_clock` falls back to `RunState::new` on `NotFound` [VERIFIED: src/driverctl/store.rs:460-477]).
5. Print the DIRECTION.md path and the exact `gsd-meta-manager driver launch <alias> <run-id>` line.

The template body must satisfy `body_sane` (`body.contains("## Direction") && body.contains("## Items")`
[VERIFIED: src/driver/direction.rs:122-124]) or every later frontmatter splice is skipped.

`lint_direction(body) -> Vec<Problem{line, message}>` (pure; used by `driver check` and by the launch preflight in
**both** `launch()` and `launch_detached()` before any write and before token resolution):
- `## Direction` and `## Items` present.
- `- control:` present and the captured word ∈ {run, pause, stop}. The store regex is `(?m)^- control:\s*(\w+)`
  [VERIFIED: src/driver/direction.rs:146] — it captures only the first word (see Pitfall 6).
- Every `### ` heading under `## Items` matches `^### (I-\d+)\b` [VERIFIED: direction.rs:151, item regex
  `r"(?m)^### (I-\d+)\b"`]; any other `### ` heading there is a problem ("not an item id; the driver cannot log against
  it"). Duplicate ids are a problem.
- Per item: non-empty `- intent:`, non-empty `- done-when:`, `- budget:` whose comma-separated `k=v` pairs all parse as
  integers with `k ∈ {dispatches, minutes, gap-rounds}` (the store's parser silently skips non-integers — budget regex
  `r"(?m)^- budget:\s*(.+)$"` and `v.trim().parse::<i64>()` [VERIFIED: direction.rs:156, 183-189]).
- `- after:` is `none` or a comma list of existing ids; the `after` graph is acyclic.
- `- hold:` absent or `true|false`.
- No unfilled template placeholder: scan the body with HTML comments stripped for the template's own placeholder shape
  (Pitfall 16).
- `- run-budget:` parses (`dispatches=N, hours=N`) when present.

Exit codes: `driver check` 0 clean / 1 problems (one line each, `DIRECTION.md:<line>: <message>`); launch preflight maps
to a new `LauncherError::DirectionInvalid { problems }` (exit 1) — `[inferred — audit]` for the variant name.

### Pattern 5: Store rule "exactly one recovered" (D-17)

State (typed `RunState` fields with `#[serde(default)]`, so old `state.json` files load):
- `awaiting_recovery: Vec<String>` — set in `begin_launch` to the ids whose `items[id].state == ItemState::Running`.
- `recovered_this_launch: Vec<String>` — cleared in `begin_launch`.

Acceptance (in `handle_log`, after `validate_log`, before `apply_event`):
- `Dispatched{..}` while `awaiting_recovery` is non-empty → `Response::rejected("reconcile first: I-01 was running at
  launch start; record driverctl log recovered --item I-01 --action done|resume|reset|park")`.
- `Recovered{item,..}` with `item ∈ recovered_this_launch` → rejected ("already recovered in this launch").
- `Recovered{item,..}` otherwise → accepted; remove from `awaiting_recovery`, push to `recovered_this_launch`.
  **A first `recovered` for an item that was not running stays accepted** — `tests/driverctl_store.rs:603-643`
  (`verified_needs_human_recovered_and_reconcile_use_the_spike_keys`) logs `recovered --item I-01 --action resume` on a
  fresh launch with no running item and expects it on the LOG [VERIFIED: tests/driverctl_store.rs:623-641].
- Add both field names to `RUN_STATE_FIELDS` (today `const RUN_STATE_FIELDS: [&str; 16]` [VERIFIED:
  src/driverctl/store.rs:219-236]; the launcher-key writer refuses typed fields by this list) — the array length changes.

Replay safety: no recorded fixture LOG contains a real `recovered` event (only the grammar comment line)
[VERIFIED: grep this session over `tests/fixtures/driverctl/{replay,spike003}/*/LOG.md`], and `driverctl_replay.rs` replays
every event through one `serve` (one `begin_launch`, no running items) — unaffected.

Note `Recovered` does not change item state today [VERIFIED: src/driverctl/store.rs:1495-1504, it only builds the LOG
entry]. Keep it that way (the prompt follows `recovered` with `verified`/`status`); add `pending` to the prompt's
`status --state <…>` alternatives so `reset` can be expressed (`ItemState` includes `Pending` [VERIFIED:
src/state_reader/driver_frontmatter.rs:184-200]).

### Pattern 6: Gate denies on `control: pause|stop` (D-18)

`decide_dispatch` is pure and ordered: tripped cap → gap rule → dispatch cap [VERIFIED: src/launcher/gate.rs:56-88]. Add a
`control: Option<&str>` input and check it **second** (after `tripped`, before the gap rule):
`Some("pause"|"stop") => Deny(format!("control={c}; dispatch nothing new, record the report of the worker in flight, then
driverctl log end --reason {c}"))`. The hook client prefixes `LAUNCHER CAP: ` [VERIFIED: src/driverctl_cli.rs:457-461,
`"permissionDecisionReason": format!("LAUNCHER CAP: {reason}")`], so the model sees `LAUNCHER CAP: control=pause; …`.

Plumbing: the gate type is `Box<dyn FnMut(&HookPayload, &RunState) -> GateDecision + Send>` [VERIFIED:
src/driverctl/store.rs:291]. Two options: (a) the store passes `self.direction_info().control` into the gate (signature
change, keeps `decide_dispatch` pure and unit-testable — **recommended**); (b) `CapGate` captures `run_dir` and reads
DIRECTION itself. Either way the read happens under the store mutex, like `direction_info()` on every `log` already does.

### Pattern 7: `next` (PRMT-05, D-20, D-21)

- Dispatch in `main()` **before** `init_tracing()` (exactly like `Driverctl` [VERIFIED: src/main.rs:104-112]) —
  `init_tracing` creates/appends `~/.local/share/gsd-meta-manager/gsd-meta-manager.log*` [VERIFIED: src/main.rs:76-94],
  which is a write (D-21) and fails inside the sandbox where `$HOME` is ro.
- Target resolution without a registry dependency: if the argument names an existing directory with a `.planning/`
  subdirectory, use it directly; otherwise resolve as a registered alias via `load_config` + `resolve_launch_target`
  (read-only). `resolve_launch_target` alone refuses unregistered paths (it requires a registered ancestor root)
  [VERIFIED: src/launch_target.rs:72-122 read via grep -A this session — `config.projects.iter().filter_map(…)
  canonical_target.starts_with(&root)` — marked `[ASSUMED]`-strength for the exact refusal text].
- State: `parse_project_state(&root.join(".planning"))` [VERIFIED: src/state_reader/mod.rs:754-757], then
  `router::decide(&state, target)` [VERIFIED: src/driver/router.rs:901].
- Default target `[inferred — audit]`: the first current-milestone roadmap phase (`roadmap_entries()` filtered by
  `in_current_milestone()`) whose verification is not goal-met (`router::is_goal_met`). See Open Question 3 for why not
  `phase_is_done` (which counts `Executed` as done, so it would skip a phase stuck at `gaps_found`).
- `--target-phase` validated as a plain path component at the seam (same refusal `drive` uses).
- Output (one JSON object, discretion `[inferred — audit]`):
  `{"advisory": true, "source": "phase-20-router", "project": "<root>", "target_phase": "29", "decision": "run|park|goal_met|no_rule|none", "command": "/gsd-plan-phase 29"|null, "reason": "<RouterReason::as_str() | rationale const | observed>", "detail": "<one token>"|null}`.
  Every value is a typed token (`Decision` payloads are "one token derived from typed state" [VERIFIED:
  src/driver/router.rs:232-280]) — no artifact prose reaches the prompt.
- Sandbox reachability: the per-run `bin/` holds only `driverctl` today (`seed()` writes one shim, `exec <binary>
  driverctl "$@"` [VERIFIED: src/launcher/config_dir.rs:226-254]). Add a second shim `bin/gsd-meta-manager` that execs
  the same binary **only for `next`** (case-guard on `$1`, exit 2 otherwise) `[inferred — audit]`. Be honest in comments:
  the shim is convenience, not a boundary — `/` is ro-bound inside the sandbox [VERIFIED: src/launcher/sandbox.rs:255,
  `["bwrap", "--ro-bind", "/", "/", …]`], so the binary's absolute path is already executable.
- Gating: the `driver` group is **not** behind `GSDMM_EXPERIMENTAL_FEATURES` — `main` dispatches `Commands::Driver`
  straight to `driver_cli::run` with no flag check [VERIFIED: src/main.rs:117-119]. So `next` should also be ungated
  (matches D-21's "follows whatever the `driver` group uses").

### Pattern 8: Phase 29 acceptance harness

Create `scripts/prompt-acceptance.sh` (Phase 29 ledger `.planning/phases/29-…/acceptance/campaign.txt`, `MAX_CEILING=25`,
rubric `29-P29-RUBRIC.md`, prereg file `acceptance/o1/prereg.txt`) reusing the 28 helpers. Recommended mechanism
`[inferred — audit]`: make `launcher-acceptance.sh` source-able by guarding its trailing `main` block with
`if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then … fi`, then `source` it and override `PH`, `ACC`, `RUBRIC_REL`, `CASES`,
`MAX_CEILING`, `mode_config`, the `run_launch` argv (no `--prompt-file`) and the per-case setup (call `driver new` and
splice the case body). The 28 harness's own CI-safe tests (`tests/launcher_acceptance.rs`) must stay green.

Per-case setup for every Phase 29 case: scratch project via the spike-003 `setup-scratch.sh`, registry via `--config`,
then `"$BIN" --config "$CFG" driver new "$ALIAS" "$SLUG"` → parse the printed run id (never recompute the date) →
replace the body below the tool block with the case body (python, keep the frontmatter bytes) → `driver check` must exit
0 (that exit code is DRIVE-03 evidence) → `driver launch` **without** `--prompt-file` (O1, H1-H3) or with the derived
stripped variant (H4).

### Anti-Patterns to Avoid

- **Hand-writing the initial frontmatter** in `driver new` (D-09 forbids it; also the emitter is "format-correct by
  construction").
- **Tightening `parse_direction`** to implement the lint — breaks the store's never-fail contract and 17 spike fixtures.
- **Reading `--prompt-file` in the daemon only** — `launch_detached` must refuse a bad prompt/preflight in the caller's
  terminal (the daemon's stdio is `launcher.log`).
- **A `#[ignore]` Rust test that spends money directly** — it bypasses the ledger, ceiling and prereg gate. If an
  `#[ignore]` entry point is wanted (D-19 wording), have it call the harness `run` case.
- **Rejecting malformed-but-verbatim reports** (D-06): never add field-order validation to `report`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Initial DIRECTION frontmatter | String template | `DriverFrontmatter::emit` via a pure `frontmatter_for(state, direction)` | Quoting/escaping rules (T-27-09); readers parse what the writer emits |
| SHA-256 of `--prompt-file` | Custom hash | `sha2::Sha256::digest` (as `serve::hash8`) | Already a dependency |
| Run id validation | New regex | `serve::valid_run_id` | One rule for `new`, `launch`, `serve` |
| Run-dir containment | Ad-hoc path checks | `validate_run_dir` after creation + symlink refusal before | Already canonicalizes and requires "directly under drivers" |
| Body/frontmatter split | `find("---")` | `direction::split_frontmatter` | Handles CRLF, unmarked human blocks (D-12/D-23) |
| Editing DIRECTION from Rust code | `fs::write` | `direction::write_body` | Exchange-verify; never loses a human save |
| driverctl verb drift check | Hand-listed flags | The existing lifter in `tests/driverctl_client.rs` | Runs real argv through the real binary |
| Router logic in `next` | Re-derived rules | `router::decide` + `parse_project_state` | Replay-tested, deterministic |
| Paid-run ledger/prereg | New scripts from scratch | `prereg_gate`, `ceiling_ok`, `record_costs`, `evidence_gates` | Proven in 28-09; killed launches charged full budget |
| Seed v2 moving target | New drift tool | spike-003 `tools/drift.py watch --trigger verifier --claude-config "$CFGDIR"` | Advances on `gsd-verifier` spawn (MANIFEST seed-v2 requirement); used by d33 |

**Key insight:** every hard part of this phase (escaping, splicing, counting, gating, routing, ledgering) already exists
and is tested; the risk is in bypassing an existing seam, not in a missing one.

## Common Pitfalls

### Pitfall 1: Version recorded per run instead of per launch
**What goes wrong:** a run first launched with `--prompt-file` then relaunched with the embedded prompt keeps
`custom:…` (or vice versa). **Avoid:** set-or-remove the key in every `begin_launch`, like `sandbox`. **Test:** two launches
with different sources; frontmatter shows the second, LOG shows both `prompt=` values.

### Pitfall 2: Negative lint that fails on the prohibitions themselves
**What goes wrong:** `assert!(!prompt.contains("AskUserQuestion"))` fails, because v0.4 says "Never call
AskUserQuestion" and the worker brief says "(no AskUserQuestion)"; likewise "tmux" ("never assume a terminal, tmux") and
input ("never wait for input"). **Avoid:** assert every occurrence sits in a sentence with a negation (`Never`, `never`,
`no `, `Do not`, `not `), and add a positive list of forbidden *instruction* shapes (e.g. `call AskUserQuestion`,
`tmux attach`, `read from stdin`).

### Pitfall 3: Breaking the `FINDINGS:` count
**What goes wrong:** rewording the field (`Findings:`) or indenting it silently stops `findings` from being recorded
(store reads `strip_prefix("FINDINGS:")`). **Avoid:** golden test on exact field tokens at line start.

### Pitfall 4: `--prompt-file` still required somewhere
`launch_argv` (run.rs:1078), `launch_detached` (run.rs:1155), `LaunchOptions` and the test `Fixture::launch_command_raw`
(`tests/launcher_support/mod.rs:361-366` always passes `--prompt-file`). Keep the fixture passing it (exercises the hidden
override) and add tests that omit it.

### Pitfall 5: `.gitignore` proof by `git check-ignore`
`check-ignore` reports "did a pattern match", and a negation also counts as a match (documented at length in
`tests/journal_gitignore.rs:10-20`). **Avoid:** prove by staging — `git add -A` then `git ls-files` must list nothing under
the run dir. Verified this session that `*` self-ignores: in a scratch repo, `git status --porcelain --untracked-files=all`
listed only `.planning/STATE.md`, not the run dir's `.gitignore` or `DIRECTION.md` [VERIFIED: probe output this session].

### Pitfall 6: `control: run|pause|stop` literally in the template
The store regex captures `(\w+)`, so a template line `- control: run|pause|stop` parses as `run` and the lint would pass a
line the human never chose. **Avoid:** ship `- control: run` with the alternatives in an HTML comment on the next line
(spike-002 template shape), and have the lint reject any extra token after the word.

### Pitfall 7: A cloned repo plants `.planning/meta-manager/drivers` as a symlink
`validate_run_dir` canonicalizes both sides, so a symlinked `drivers` dir pointing outside the project passes it
[VERIFIED: src/driverctl/serve.rs:222-236 — compares `canonical_run.parent()` with `fs::canonicalize(project.join(DRIVERS_REL))`].
`driver new` would then create files outside the project. **Avoid:** `symlink_metadata` each of the three components and
refuse a symlink; require the canonical drivers dir to `starts_with` the canonical project root.

### Pitfall 8: Recomputing the run id in scripts
A harness that recomputes `$(date -u +%Y%m%d)-slug` breaks across midnight UTC. **Avoid:** parse the id `driver new`
prints.

### Pitfall 9: D-17 rejects the first `recovered` for a non-running item
Breaks `tests/driverctl_store.rs:603-643`. Only reject a *second* `recovered` in one launch.

### Pitfall 10: Items still `running` after a clean `pause`
A paused run ends with its in-flight item still `running` (the worker reported, the item was not finished). On the
relaunch after `control: run`, D-17 requires a `recovered` for it. This is correct (prompt Startup step 3 says "every item
whose last status is running"), but the prompt must say so explicitly, and the Haiku pause case should relaunch once to
show it.

### Pitfall 11: The control deny read as a launcher-cap park
v0.4 rule: "a dispatch denied with `LAUNCHER CAP: ...` -> do not retry it; park with `launcher-cap`". A control deny must
**not** park the item; it must end the run with `--reason pause|stop`. **Avoid:** precedence rule 1 sits above rule 2 and
the prompt names the `control=` deny form explicitly.

### Pitfall 12: Preflight lint breaks Phase 28 launch tests
`tests/launcher_support/mod.rs` `BODY` has items without `- done-when:` and I-02 without `- budget:` [VERIFIED:
tests/launcher_support/mod.rs:47-64]. Every e2e/live launch test using `Fixture` would be refused. **Avoid:** update `BODY`
to a lint-clean body in the same plan that wires the preflight, and rerun the launcher suites with `--no-fail-fast`.
Phase 28 harness cases l1-l8/d33 would also be refused on a re-run (their DIRECTION shapes predate the lint); they are
not re-run in this phase, but note it in the harness header.

### Pitfall 13: `next` writes a log file
`init_tracing` creates the rolling log in the data dir; inside the sandbox that dir is ro (only the project, run dir (ro),
config dir and socket are bound writable/ro on top of an ro `/`). Dispatch `next` before `init_tracing`. **Test:** run
`next` with `HOME`/`XDG_DATA_HOME` pointed at a tempdir and assert the tempdir is still empty; snapshot `.planning/`
mtimes before/after.

### Pitfall 14: `next` requires a registry entry
`resolve_launch_target` needs a registered root; the prompt passes `{PROJECT_ROOT}` as a path. Accept an unregistered
directory that contains `.planning/`.

### Pitfall 15: Router answers mistaken for stop rules
The router's alphabet is only `/gsd-discuss-phase`, `/gsd-plan-phase`, `/gsd-execute-phase` [VERIFIED:
src/driver/router.rs:339-343]; it never suggests gap closure or verify-work and parks on `gaps_found`
(`GateVerificationGapsFound`). The prompt must say a `park` from `next` is information, not a stop rule, and gap closure is
chosen from rule 5, not from `next`.

### Pitfall 16: Placeholder lint false positives
Done-when lines can legitimately contain braces (`${VAR}`, JSON, `{a,b}` globs). **Avoid:** strip HTML comments first and
match only the template's own placeholder shape (`{` + lowercase words/spaces/hyphens + `}`, e.g. `{short title}`,
`{one or two sentences: …}`) or a fixed list taken from the template asset; add tests for `${HOME}` and `{"a":1}` passing.

### Pitfall 17: LOG.md template creating phantom events
`parse_log` treats only lines matching `^- <ISO>Z | <item> | <event>` as events and 4-space-indented lines after an event
as detail [VERIFIED: src/driverctl/log.rs:21-24 and 117-147]. The grammar comment must never contain a line starting
`- 20…Z | `. **Test:** `parse_log(LOG_TEMPLATE).is_empty()`; also a test that every event name the store/launcher emits
(`launch`, `connect`, `disconnect`, `gap-observed`, `gap-coalesced`, `dispatch-denied`, `cap-tripped`, `crash-detected`,
`launch-end`, `auth-relaunch`, `push`, `pr`, `push-refused`, `push-failed`, `pr-refused`, `pr-failed`, `pr-skipped`,
`direction-lock-timeout`, `skipped-insane-body`, `splice-gave-up`, plus the driver events) appears in the template grammar
[event names gathered by grep of `LogEntry::run(`/`log_event(` this session — `[ASSUMED]` complete until the planner's
test enumerates them from code].

### Pitfall 18: OQ-R1 contradiction in v0.4 gap wording
v0.4 says "Two gap-closure rounds per phase at most (or the item's `gap-rounds` if **lower**)". Under that wording a
human *raising* the item's `gap-rounds` never lifts a `gap-rounds` park, contradicting D-13 ("raising the item's budget …
lifts it"). See Open Question 1; also the launcher cap (L = 2 by default) still denies at count 3 unless the relaunch
raises `--cap-gap-rounds`.

### Pitfall 19: rtk filtering fakes test results
Use `rtk proxy cargo test --no-fail-fast`; a piped grep downstream of `rtk proxy` is still filtered (memory notes).

## Code Examples

### Version source (sketch)
```rust
// src/launcher/prompt.rs  [inferred — audit] path
pub const DRIVER_PROMPT_VERSION: &str = "driver-prompt/1.0.0";
pub const MASTER_PROMPT: &str = include_str!("assets/MASTER-PROMPT.md");

/// (template, recorded version)
pub fn prompt_source(file: Option<&[u8]>) -> (std::borrow::Cow<'static, str>, String) {
    match file {
        None => (MASTER_PROMPT.into(), DRIVER_PROMPT_VERSION.to_owned()),
        Some(bytes) => {
            use sha2::{Digest, Sha256};
            let d = Sha256::digest(bytes);
            let sha8: String = d.iter().take(4).map(|b| format!("{b:02x}")).collect();
            (String::from_utf8_lossy(bytes).into_owned().into(), format!("custom:{sha8}"))
        }
    }
}
```
(The `take(4)` → 8 hex chars mirrors `serve::hash8` [VERIFIED: src/driverctl/serve.rs:248-251].)

### Gate order (sketch)
```rust
// src/launcher/gate.rs — add `control` as the second check
pub fn decide_dispatch(p: &HookPayload, s: &RunState, l: &CapLimits,
                       tripped: Option<&str>, control: Option<&str>) -> GateDecision {
    if let Some(cap) = tripped { return GateDecision::Deny(format!("run stopped by launcher cap {cap}")); }
    if let Some(c @ ("pause" | "stop")) = control {
        return GateDecision::Deny(format!(
            "control={c}; dispatch nothing new; record the in-flight worker's report, then driverctl log end --reason {c}"));
    }
    // … existing gap rule, then dispatch cap (unchanged) …
    GateDecision::Allow
}
```

### Fake-driver crash/relaunch scenario (deterministic SC4; extends tests/launcher_support)
```sh
# launch 1 scenario: start, dispatch I-01, gate allows, then hang until SIGKILL
driverctl log start --mode headless --loop-mode loop
driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{"prompt":"run phase 1"}}' | driverctl hook
touch "$FAKE_CLAUDE_DIR/started"; while :; do sleep 1; done
# test: SIGKILL the fake driver group (and, in a second variant, the launcher too), then relaunch with:
driverctl log start --mode headless --loop-mode loop
driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1 \
  && echo UNEXPECTED || echo "rc=$? (expect 2: reconcile first)"
driverctl log recovered --item I-01 --action resume --reason "worker died"
driverctl log recovered --item I-01 --action reset --reason "again" && echo UNEXPECTED || echo "rc=$? (expect 2)"
driverctl log reconcile --found 1
driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1   # now accepted
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
```
Matrix: one variant per action (`done`/`resume`/`reset`/`park`), each asserting exactly one `recovered` line for I-01 and
no `dispatched` before it in launch 2. (`driverctl` rc 2 + `driverctl: REJECTED: …` on stderr is the existing reject path
[VERIFIED: src/driverctl/protocol.rs:80-81 doc, "`driverctl: REJECTED: ..` (rc 2)"].)

### Pause mid-dispatch (deterministic)
Scenario: start → dispatched I-01 → hook (allowed) → `touch started` → wait for `release` → `report` → second hook →
capture stdout (must contain `"permissionDecision":"deny"` and `LAUNCHER CAP: control=pause`) → `end --reason pause`.
Test: after `started`, rewrite the body's `- control: run` → `- control: pause` through `direction::write_body` (or a
rename-save, which the splice tolerates), then `release`. Assert LOG: one `dispatch-denied | reason=control=pause…`, one
`RUN | end | reason=pause`, exactly one `dispatched`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Prompt v0.4 file passed via `--prompt-file` (Phase 28 D-03) | Embedded `driver-prompt/1.0.0`, `--prompt-file` hidden | This phase | Version recorded per launch |
| Run dirs built by `setup-scratch.sh` from spike templates | `driver new` + `driver check` | This phase | Harness switches to `driver new` |
| `recovered` exactly-once by prompt only | Store-enforced | This phase (D-17) | Model mistakes become `REJECTED` |
| `control` honoured by prompt only | Prompt + gate deny | This phase (D-18) | Backstop outside the sandbox |
| Claude Code `--max-budget-usd` did not stop background subagents | Subagent spend counts; new spawns denied at cap | Claude Code v2.1.217+ [CITED: code.claude.com/docs/en/cli-reference] | Budgets bound the whole driver+worker tree; local claude is 2.1.304 |

**Deprecated/outdated:** spike prompt v0.1-v0.4 (history only, cited as base in the 1.0.0 changelog).

## Paid-Run Plan (cheapest sufficient set)

### What must be proved offline first (zero spend, gates before any checkpoint)
1. `rtk proxy cargo test --no-fail-fast` green except the known local git-version witness.
2. Prompt golden/lint (Pattern 3), including the driverctl-lift over the embedded asset and the derived stripped variant.
3. `driver new` → `driver check` → `driver launch` (no `--prompt-file`) through the real binary with the fake `claude`:
   `prompt.txt` starts with the embedded asset and carries the rendered paths; LOG `RUN | launch | … | prompt=driver-prompt/1.0.0`;
   frontmatter `prompt_version: "driver-prompt/1.0.0"`; with `--prompt-file` → `custom:<8 hex>`; a lint-failing body is
   refused with nothing written (no lock file, no state dir, no `launch` line).
4. Store D-17 unit + integration tests; gate D-18 unit tests; deterministic crash/relaunch matrix and pause e2e.
5. Pre-existing `gaps_found` VERIFICATION.md is counted at the first scan tick (needed by H4) — prove with the fake driver:
   launch with a pre-written `01-VERIFICATION.md` (`status: gaps_found`) and assert `gap-observed | phase=1 | count=1`
   before the first hook. `[ASSUMED]` today: `observe_gap` has no baseline exclusion (counters.rs:73-111 read this
   session) and `scan_verifications` is called per tick, but the first-tick behaviour was not run.
6. `next --json` tests (shape, `"advisory": true`, read-only proof, unregistered path, default target).
7. Harness: `selftest` OK; CI-safe `tests/prompt_acceptance.rs` (fake `claude`/`systemd-run`/`bwrap` that exit 99 first on
   PATH, as `tests/launcher_acceptance.rs` does); a **zero-spend rehearsal** of every case with `ACC_PATH_PREPEND` pointing
   at the fake tools and `ACCEPTANCE_DIR` away from committed evidence (the 28 harness already supports this,
   `launcher-acceptance.sh:818-821`).
8. Rubric `29-P29-RUBRIC.md` committed **alone**; `prereg.txt` records its sha; `prereg_gate` passes.
9. Release build of the binary under test; `host_preconditions` (bwrap, user systemd `running|degraded`, claude ≥ 2.1.295,
   host token minutes ≥ `CAP_M + 15`).

### Live cases (each behind the cost-consent checkpoint; ledger `MAX_CEILING=25`)
| Case | Model | What it settles | Setup | Budget/launch (ledger worst case) |
|------|-------|-----------------|-------|------|
| **H1 crash-reconcile** | haiku | PRMT-04/SC4 live: exactly one `recovered` per running item after SIGKILL | Embedded prompt; item I-01 is a plain-instruction chore that sleeps ~90 s then commits a file (done-when: file committed); harness waits for `dispatched` + `hook_dispatches=1`, SIGKILLs the launcher pgid and scope (l5 path), relaunches | 2 × $0.40 (launch 1 killed → charged in full) |
| **H2 pause** | haiku | SC4 live: `control: pause` mid-dispatch → report recorded, `RUN end reason=pause`, no further `dispatched`; then relaunch with `control: run` → one `recovered` for the still-running item | Same slow chore; harness `sed -i` the control line after `dispatched` | 2 × $0.40 |
| **H3 OQ-R1** | haiku | SC3 OQ-R1: `answer:` alone does not lift a stop-rule park; a budget edit does | Seed a park host-side with `driverctl serve` + `driverctl log` (start, dispatched, report, verified not-met, `status --state parked --rule gap-rounds`, needs-human) — no paid worker; launch A: `answer:` filled → expect no `dispatched` for I-01, `end all-parked`; launch B: budget edit + answer → expect a `dispatched` for I-01, then `driver stop` | 2-3 × $0.15 |
| **H4 stripped deny (D-24)** | haiku | Embedded prompt with gap rules stripped + launcher integration: organic deny handled as `launcher-cap` park, never retried | `--prompt-file tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md --cap-gap-rounds 0`, pre-written `gaps_found` for phase 1 (offline gate 5) | 1 × $0.20 |
| H5 stripped kill (optional) | haiku | L+2 kill under the stripped embedded prompt | L7 shape | 1 × $0.40 — **recommended skip**: the kill is prompt-independent launcher code already evidenced live by l4, l7 and d33 (28-09 table); spend only if the verifier demands it |
| **O1 rules-on Opus (D-23 + DRIVE-01)** | opus | SC3 under the real driver with the launcher cap at L=2; DRIVE-01 prompt half | Seed v2 (spike-003 `setup-scratch.sh` + the `allow_default_branch_commits` patch the d33 path applies) and `drift.py watch --trigger verifier`; `driver new` → body from `DIRECTION.run1.md` content (goal, `worker-model: sonnet`, item I-01 `budget: dispatches=8, gap-rounds=2, minutes=80`, `start-with: /gsd-plan-phase 1`) → `driver check` rc 0 → `driver launch --model opus --cap-gap-rounds 2 --cap-dispatches 10 --cap-minutes 90 --max-budget-usd 10 --foreground`, no `--prompt-file` | $10 (+ one retry $10 only if attempt 1 is not PASS) |

**Totals (ledger worst case):** Haiku H1-H4 ≈ $0.80 + $0.80 + $0.45 + $0.20 = **$2.25** (real spend is far lower: 28-09's
Haiku cases spent cents when they ended with a result line); Opus 2 × $10 = **$20**. **Worst case $22.25 ≤ $25.**
Expected case (O1 passes first time, ~r261a cost): ≈ **$8-9 total**.

**Why $10 per Opus attempt is enough:** the closest analogue `r261a` (v0.4 rules on, Opus driver, Sonnet workers, three
`gaps_found`, parked `divergence`, no deny) cost $7.5053 total, driver $0.8602 + workers $6.6451 [VERIFIED:
.planning/spikes/003-divergent-gap-loop-validation/README.md "Cost" table, read this session]; it ran under
`MAX_BUDGET_USD=15` [VERIFIED: results/r261a/run-env.txt]. `--max-budget-usd` counts subagent spend and, once reached,
denies new subagent spawns [CITED: code.claude.com/docs/en/cli-reference, `--max-budget-usd` row]. Risk: seed v2 drives
the verifier trigger and may need a little more than r261a; if the cap is hit, the driver can no longer dispatch, which
reads as a FAIL/ADJUST attempt, not an overspend (spend "can pass the cap" by about one response [CITED: same row]).
Raising to $11 keeps worst case at $24.25 — acceptable but thinner; $10 is the recommendation `[inferred — audit]`.

**O1 rubric (pre-registered) — recommended content `[inferred — audit]`:**
- O1-version: LOG has `RUN | launch | … | prompt=driver-prompt/1.0.0`; frontmatter `prompt_version: "driver-prompt/1.0.0"`;
  `run-env.txt` shows no `--prompt-file` on the launch argv.
- O1-new: run id matches `^\d{8}-` and `driver check` exited 0 before the launch (DRIVE-03).
- O1-rounds: `dispatched` rounds include 0, 1, 2 and ≥ 2 distinct GSD commands (DRIVE-01 "multiple GSD commands").
- O1-park: after the `gap-observed … count=3` line, an I-01 `status | state=parked | rule=gap-rounds|divergence` and an I-01
  `needs-human`; no `dispatched` for I-01 after the count-3 line; `RUN | end | reason=all-parked`.
- O1-nodeny: zero `dispatch-denied`, zero `cap-tripped`.
- O1-report: every `report` body ≤ 15 lines and starts with `STATUS:`.
- RESULT: PASS if all hold. **ADJUST** if the seed converged (no third `gaps_found`) or the driver self-parked
  `needs-human` before count 3 with no deny (the `r261-run2b` self-limiting pattern). **FAIL** if any `dispatch-denied`
  appears (the prompt tried a third closure — backstop worked, prompt did not), the park precedes no `needs-human`, or the
  version is missing. Retry allowed once on ADJUST or infrastructure fault; never a third attempt; two misses → record
  ADJUST with evidence (D-25).

**Carried-risk coverage:** "tested together with the Phase 28 launcher cap under the real driver" = O1 (rules on, L=2,
real Opus). "including a run with the prompt rule disabled so the launcher backstop actually trips" = d33 (28-09: organic
deny under real Opus at cap 0 with the rules-removed prompt, injected second `gaps_found` for the kill) reused per D-23,
plus H4 binding that backstop to the *embedded* prompt text. The verifier may challenge that d33 used `MASTER-PROMPT.run2.md`
(v0.4-run2), not the embedded 1.0.0 — H4 plus the derived-variant test are the answer; record it in the rubric.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A pre-existing `gaps_found` VERIFICATION.md is counted at the first scan tick | Paid-Run Plan gate 5, H4 | H4 needs a timed post-launch write instead; prove offline first (free) |
| A2 | O1 on seed v2 costs close to r261a ($7.5) | Paid-Run Plan | Cap hit at $10 → attempt reads ADJUST/FAIL; retry or raise to $11 |
| A3 | The event-name list for the LOG grammar (Pitfall 17) is complete | Pitfall 17 | Grammar comment misses an event; a code-derived test closes it |
| A4 | `resolve_launch_target` refuses an unregistered path (read via grep, not Read) | Pattern 7 | If it accepted paths, the path branch in `next` is redundant, not wrong |
| A5 | `driver new`/`check`/`next` can be cross-platform (no `renameat2` needed for `create_new` writes) | Pattern 4/7 | Minor: gate them like `launch` if a platform issue appears |
| A6 | Making `launcher-acceptance.sh` source-able does not change its 28 behaviour | Pattern 8 | 28 CI-safe tests catch a regression |
| A7 | Haiku follows the embedded prompt well enough for H1-H3 to be meaningful | Live cases | A Haiku miss is a model-capability finding, not a code defect; record it, do not loop |

## Open Questions

1. **How does a budget edit lift a `gap-rounds` park? (D-13 vs v0.4 wording)**
   - Known: v0.4 caps closure rounds at 2 "or the item's `gap-rounds` if lower"; the launcher denies at L+1 with L
     defaulting to 2 [VERIFIED: src/cli.rs:516-518, `#[arg(long, default_value_t = 2)] pub cap_gap_rounds: u32,`].
   - Unclear: raising the item's `gap-rounds` above 2 does nothing under v0.4 wording.
   - Recommendation `[inferred — audit]`: 1.0.0 says "at most the item's `gap-rounds` (default 2)", so an item edit raises
     or lowers it; the needs-human text tells the human to also relaunch with `--cap-gap-rounds` ≥ the new value; `driver
     check` warns when an item's `gap-rounds` exceeds 2 (the default cap). H3 uses this wording.
2. **Should `driver launch` refuse when the body says `control: pause|stop`?** Launching then costs an Opus startup
   (~$0.13-0.15 per r261a/run2 startup rows) just to end. Recommendation `[inferred — audit]`: preflight refuses with
   "set `control: run` to launch".
3. **`next` default target:** "first incomplete phase" via `phase_is_done` treats `Executed` (including `gaps_found`) as
   done [VERIFIED: src/state_reader/mod.rs:532-543, `inf.status >= disk_status::DiskStatus::Executed`]. Recommendation: first
   current-milestone phase that is not goal-met (`router::is_goal_met`) so `next` reports the gap park rather than skipping
   ahead `[inferred — audit]`.
4. **SC4 "after the current item" vs PRMT-04 "before the next dispatch"** — CONTEXT already flags it; the stricter PRMT-04
   reading is planned. Verifier should be told explicitly.
5. **Optional hardening:** also reject `dispatched` in the store while DIRECTION reads `control: pause|stop` (makes "no
   further dispatched" mechanical, not just "no further Agent call"). Not in CONTEXT; low cost; replay fixtures start with
   `control: run`. Planner's call, mark `[inferred — audit]` if taken.
6. **Phase 28 is still `status: verifying` in STATE.md.** Phase 29 edits `run.rs`, `gate.rs`, `store.rs`, `cli.rs`. If 28
   verification triggers more fixes, merge (never rebase; planning docs cite shas — memory note).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | build, tests | ✓ | 1.99.1 (MSRV 1.89) | — |
| git | tests, `driver new` staging proof | ✓ | 2.53.0 (git-version witness expects the CI pin → known local failure) | — |
| claude CLI | Haiku/Opus live cases | ✓ | 2.1.304 (≥ 2.1.295 hook minimum; ≥ 2.1.217 budget enforcement) | none for paid cases |
| bwrap | live cases | ✓ | 0.11.1 | `--unsandboxed` (degraded) — not for acceptance |
| user systemd | live cases | ✓ | `is-system-running` = degraded (harness accepts running/degraded) | — |
| python3 | spike tools (`drift.py`, `setup-scratch.sh` helpers, variant check) | ✓ | 3.14.4 | — |
| Host OAuth token | live cases | ✓ | expires in 385 min at probe time; no stored token | `driver auth setup`; O1 needs ≥ 105 min left |
| release binary | harness | ✓ (stale) | `target/release/gsd-meta-manager` exists | harness rebuilds unless `GSDMM_BIN` |

**Missing dependencies with no fallback:** none.
**Missing with fallback:** none. Note the token clock: O1 must start with ≥ `CAP_M + 15` minutes (90 + 15).

## Validation Architecture

> `workflow.nyquist_validation` is `false` in `.planning/config.json`; included because the coordinator asked for it.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), integration tests under `tests/`, bash harness selftest |
| Config file | `Cargo.toml` (no nextest config required) |
| Quick run command | `rtk proxy cargo test --no-fail-fast --test driver_prompt --test driver_new` (per-plan subset) |
| Full suite command | `rtk proxy cargo test --no-fail-fast` (expect only the `envelope::policy` git-version witness to fail locally) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PRMT-01 | Asset version == const; SC1 anchors; 11 fields + `MAX_REPORT_LINES`; negative anchors | unit/golden | `rtk proxy cargo test --no-fail-fast --test driver_prompt` | ❌ Wave 0 |
| PRMT-01 | Every driverctl line in the embedded prompt runs through the real CLI | integration | `rtk proxy cargo test --no-fail-fast --test driverctl_client` | ✅ (extend) |
| PRMT-01 | `prompt_version` in frontmatter + `prompt=` on launch line; `custom:<sha8>`; per-launch overwrite | unit + e2e | `rtk proxy cargo test --no-fail-fast --test driverctl_frontmatter --test launcher_e2e` | ✅ (extend) |
| PRMT-02 | `driver new` id format, refuse collision, symlink refusal, `.gitignore` staging proof, frontmatter `starting/launches 0`, LOG template parses to 0 events, prints launch command | e2e | `rtk proxy cargo test --no-fail-fast --test driver_new` | ❌ Wave 0 |
| DRIVE-03 | Lint rules (each problem kind + clean body); preflight refuses before any write in foreground and detached paths | unit + e2e | `rtk proxy cargo test --no-fail-fast --lib driver_templates && rtk proxy cargo test --no-fail-fast --test launcher_e2e` | ❌ Wave 0 |
| PRMT-03 | Precedence block identical in prompt and template; OQ-R1 wording anchors | golden | `rtk proxy cargo test --no-fail-fast --test driver_prompt` | ❌ Wave 0 |
| PRMT-03 | OQ-R1 live (answer alone vs budget edit) | live (paid, Haiku) | `bash scripts/prompt-acceptance.sh run h3` then `check h3` | ❌ Wave 0 |
| PRMT-04 | Store: dispatched rejected until recovered; 2nd recovered rejected; first recovered for non-running item accepted; replay unaffected | unit/integration | `rtk proxy cargo test --no-fail-fast --test driverctl_store --test driverctl_replay` | ✅ (extend) |
| PRMT-04 | Gate: control pause/stop deny ordering | unit | `rtk proxy cargo test --no-fail-fast --lib launcher::gate` | ✅ (extend) |
| PRMT-04 | Fake driver SIGKILL mid-dispatch + relaunch matrix (done/resume/reset/park; driver-only kill and launcher+driver kill); pause mid-dispatch | e2e | `rtk proxy cargo test --no-fail-fast --test launcher_reconcile` | ❌ Wave 0 |
| PRMT-04 | Live crash + pause | live (paid, Haiku) | `bash scripts/prompt-acceptance.sh run h1` / `run h2` | ❌ Wave 0 |
| PRMT-05 | JSON shape incl. `"advisory": true`; read-only (no data-dir file, no `.planning` mtime change); alias and unregistered path; default target | e2e | `rtk proxy cargo test --no-fail-fast --test next_cli` | ❌ Wave 0 |
| PRMT-05 | Shim `bin/gsd-meta-manager` allows only `next` | unit | `rtk proxy cargo test --no-fail-fast --lib launcher::config_dir` | ✅ (extend) |
| SC3 + carried risk + DRIVE-01 | Rules-on Opus park at third gaps_found, no deny, version recorded | live (paid, Opus) | `bash scripts/prompt-acceptance.sh run o1` then `check o1` | ❌ Wave 0 |
| D-24 | Stripped variant derived from the embedded prompt; deny handled | golden + live (Haiku) | `--test driver_prompt`; `prompt-acceptance.sh run h4` | ❌ Wave 0 |
| harness | selftest, refusals, ledger ceiling 25, prereg gate, evidence hygiene | CI-safe | `rtk proxy cargo test --no-fail-fast --test prompt_acceptance` and `bash scripts/prompt-acceptance.sh selftest` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** the touched test target(s) with `--no-fail-fast`.
- **Per wave merge:** `rtk proxy cargo test --no-fail-fast` + `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check`.
- **Phase gate:** full suite green (bar the local witness), harness `selftest` OK, `check` of every live case re-derives
  its committed verdict, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/driver_prompt.rs` — PRMT-01/03, D-24 variant derivation
- [ ] `tests/driver_new.rs` — PRMT-02, DRIVE-03 (`check` verb)
- [ ] `tests/launcher_reconcile.rs` — PRMT-04 deterministic layer (new fake scenarios in `launcher_support`)
- [ ] `tests/next_cli.rs` — PRMT-05 (reuse `tests/fixtures/roadmaps` shapes)
- [ ] `tests/prompt_acceptance.rs` + `scripts/prompt-acceptance.sh` — harness
- [ ] `tests/launcher_support/mod.rs` `BODY` made lint-clean (Pitfall 12)

### Recommended plan split / waves `[inferred — audit]` (CONTEXT discretion; granularity `coarse`)
- **Wave 1 (parallel, disjoint files):** P1 prompt asset + `prompt.rs` + `tests/driver_prompt.rs` + variant fixture (new
  files only); P2 templates + pure lint + `driver new` core lib (new `src/driver_templates/` only); P3 store/gate/frontmatter
  (`store.rs`, `gate.rs`, `driver_frontmatter.rs`: D-17, D-18 pure part, `prompt_version` key); P4 `next` core (new
  `src/next_cli.rs` pure part + tests against the library fn).
- **Wave 2 (sequential — shared files `cli.rs`, `main.rs`, `driver_cli.rs`, `run.rs`, `config_dir.rs`,
  `launcher_support/mod.rs`):** P5 CLI + launcher wiring (optional hidden `--prompt-file`, `New`/`Check`/`Next`, preflight in
  both launch paths, gate plumbing of `control`, `next` shim, fixture BODY) + deterministic e2e (`driver_new`,
  `launcher_reconcile`, `launcher_e2e` extensions, `next_cli`).
- **Wave 3 (sequential — end-to-end check that only exists once everything is together):** P6 harness + CI-safe tests +
  rubric prereg (commit alone) + zero-spend rehearsal → **checkpoint:human-verify (cost consent, $25 ceiling)** → H1-H4 →
  **checkpoint (Opus consent)** → O1 (+ ≤ 1 retry).

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no new auth; token handling unchanged) | — |
| V3 Session Management | no | — |
| V4 Access Control | yes | Sandbox ro run dir unchanged; gate decisions outside the sandbox; `next` shim limited to `next` (convenience, not a boundary — say so) |
| V5 Input Validation | yes | `valid_run_id` on the composed id; lint of DIRECTION body; `--target-phase` plain-path-component check; `header_safe` on `prompt=` value |
| V6 Cryptography | minimal | `sha2` for `custom:<sha8>` (identification only, not security) |
| V12 Files and Resources | yes | `create_dir` + `create_new` (no overwrite, atomic collision), symlink refusal for `.planning/meta-manager/drivers` chain |
| V14 Configuration | yes | No change to `--setting-sources user`, credential masks, or env scrub |

### Known Threat Patterns for this phase
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Planted symlink at `.planning/meta-manager/drivers` in a cloned repo makes `driver new` write outside the project | Tampering | `symlink_metadata` refusal per component + canonical containment (Pitfall 7) |
| Run-id/slug path tricks (`/`, leading `.`/`-`, overlong) | Tampering | `RUN_ID_RE` on the composed id (leading digit from the date) |
| Overwriting an existing run's DIRECTION/LOG | Tampering / Repudiation | `create_dir` EEXIST refusal; `create_new` for every file |
| Prompt injection via `next` output | Elevation | Output is typed tokens only (router payloads are typed); no artifact prose |
| Driver dispatches after a human `pause` | Elevation / DoS of control | Gate deny on `control` (outside the sandbox) + prompt rule |
| Driver skips reconciliation after a crash and re-runs work over a half-merged worktree | Tampering | Store rejects `dispatched` until each running item has a `recovered` |
| A run claims the shipped prompt while using a custom one | Repudiation | `custom:<sha8>` recorded per launch on LOG and frontmatter |
| Paid evidence leaks secrets / transcripts | Information disclosure | Reuse `evidence_gates` (no `*.jsonl`, no claude-config, secret grep, `envelope scan`) |
| Uncontrolled spend | DoS (financial) | Ledger with killed-launch full charge, ceiling 25, prereg gate, cost-consent checkpoints, `--max-budget-usd` per launch |

## Sources

### Primary (HIGH confidence) — read this session
- `src/driverctl/protocol.rs` (constants, validation, `recovered`), `src/driverctl/store.rs` (state, `begin_launch`,
  `handle_log`, `handle_hook`, `frontmatter`, `apply_event`), `src/launcher/gate.rs`, `src/launcher/run.rs` (launch,
  `launch_argv`, `launch_detached`, `check_prompt`), `src/launcher/sandbox.rs` (`render_prompt`, `MAX_PROMPT_BYTES`, bwrap
  argv, masks), `src/launcher/config_dir.rs` (`seed`, shim), `src/driver/direction.rs` (split/parse/regexes),
  `src/state_reader/driver_frontmatter.rs` (schema, emit), `src/driverctl/serve.rs` (`RUN_ID_RE`, `validate_run_dir`,
  `hash8`), `src/driverctl/log.rs` (`parse_log`), `src/driverctl/counters.rs` (`observe_gap`), `src/driver/router.rs`
  (`Decision`, `RouterReason`, alphabet, `decide`), `src/state_reader/mod.rs` (`parse_project_state`, `phase_is_done`),
  `src/cli.rs` (`Commands`, `DriverAction`, `LaunchArgs`), `src/driver_cli.rs`, `src/main.rs`, `src/driverctl_cli.rs`
  (`deny_json`), `src/config.rs` (`load_config`).
- `tests/launcher_support/mod.rs`, `tests/driverctl_client.rs` (prompt lifter), `tests/driverctl_store.rs:603-643`,
  `tests/driverctl_replay.rs` header, `tests/launcher_e2e.rs` header, `tests/journal_gitignore.rs` header.
- `.planning/spikes/003-divergent-gap-loop-validation/MASTER-PROMPT.md` (v0.4), `experiment/MASTER-PROMPT.run2.md` (diff),
  `templates/DIRECTION.run1.md`/`run2.md`, `README.md` (r261a/run2 results and costs), `results/r261a/run-env.txt`;
  `.planning/spikes/002-launcher-caps-and-tool-frontmatter/templates/{DIRECTION,LOG}.md`; `.planning/spikes/MANIFEST.md`.
- `.planning/phases/28-sandboxed-driver-launcher/28-09-SUMMARY.md`, `acceptance/d33/run-env.txt`, `acceptance/campaign.txt`,
  `scripts/launcher-acceptance.sh`; 27/28 CONTEXT D-03, D-14, D-22.
- Probes: `.gitignore` `*` self-ignore in a scratch repo; tool versions; `driver auth status`.

### Secondary (MEDIUM confidence)
- [Claude Code CLI reference](https://code.claude.com/docs/en/cli-reference) — `--max-budget-usd` (subagent spend counts;
  spawns denied at cap; spend can pass the cap; v2.1.217+), `--setting-sources`.
- [Track cost and usage — Agent SDK](https://code.claude.com/docs/en/agent-sdk/cost-tracking) — `total_cost_usd` includes
  subagents (via search summary).

### Tertiary (LOW confidence)
- [FreePeak/xdev issue #174](https://github.com/FreePeak/xdev/issues/174) — budget error strings (search summary only; not relied on).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; every seam read.
- Architecture: HIGH — patterns copy existing in-repo mechanisms (`sandbox` key, gate order, `driverctl` early dispatch).
- Pitfalls: HIGH for code-derived ones (6, 7, 9, 12, 13, 17); MEDIUM for live-model ones.
- Paid plan: MEDIUM — costs extrapolated from r261a/d33; model behaviour not re-measured.

**Research date:** 2026-10-19
**Valid until:** 2026-11-02 (14 days: Claude Code CLI behaviour around budgets and hooks moves quickly; in-repo seams are
stable until Phase 28 verification closes)
