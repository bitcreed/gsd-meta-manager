# Phase 16: Run Journal & State Substrate - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("best well-reasoned guess is good enough; don't ask questions"). Every decision
below is Claude's Discretion and is listed explicitly so it can be corrected at the end.
Decisions marked **[LOCKED-BY-RESEARCH]** were settled by the committed research pass
(`.planning/research/{SUMMARY,STACK,ARCHITECTURE,PITFALLS}.md`) and are carried in as
given — do not relitigate them during planning.

<domain>
## Phase Boundary

Every run leaves a durable, redacted, cheap-to-read record on disk that outlives the
processes that wrote it.

In scope: OBS-01 (append-only on-disk journal that survives process death and is the source
of truth for run state), OBS-06 (driver journal writes do not trigger full project
re-parses), SAFE-04 (credentials redacted **when captured** into the run log, not when
rendered).

Also in scope, because nothing earlier can own them and something later would be too late:

- The `.planning/meta-manager/runs/<run-id>/` layout, its run-id format, and its
  `.gitignore` posture — including the answer to the carried-in open question OQ9 (which
  artifacts are committed).
- `Action::FileChanged` gaining a `changed_path` field. Phase 15's context explicitly
  defers this here: *"`FileChanged` currently carries only `project_path` — it has no way
  to tell driver writes from GSD writes. Adding `changed_path` is Phase 16's change."*
- Path classification in `src/watcher.rs` plus the cheap byte-offset tail route in
  `src/app.rs` — the whole of "free watching has a bill".
- The redaction filter, positioned in the capture path and enforced by a type rather than
  by review discipline.
- Run-log growth bounds: per-event payload cap, per-run byte cap, retention of the last N
  runs.
- The journal event schema, including `kind` values Phase 20 will emit, so that phase adds
  no schema migration.

Out of scope — each has a named later owner:

- Detached spawn (`gsd-meta-manager drive <alias>`), `Commands::Drive`, PID liveness,
  crash **reconciliation** (Phase 16 writes the facts reconciliation reads; it does not do
  the reconciling), the kill switch, dry-run, `flock`, the `driver_opt_in` record and its
  UI → **Phase 17**.
- The Driver tab, live render, ring buffers, the three-state injection UI, dashboard
  badges, and the `inbox.jsonl` TUI→driver channel → **Phase 18**. Phase 16 ships **no UI**.
- Pre-push secret scanning, the `--disallowedTools` credential-read denylist, the branch
  namespace envelope, scoped git credentials → **Phase 19**. SAFE-04 is the *capture-path*
  half only; the *tool-boundary* half (deny `Bash(env*)`, `Read(**/.env*)`) is Phase 19's
  and is the control that fires before a secret ever enters context.
- `decide()`, the D-R-P-E-V router, run-level step/no-progress caps, the quota-floor park
  → **Phase 20**. Phase 16 defines the `decided` / `parked` event kinds; Phase 20 emits them.
- LLM goal decomposition, untrusted-content delimiting → **Phase 21**.
- `ExecutionTarget::Container`, `PathMap`, path translation in journal records → **Phase 22**.

</domain>

<decisions>
## Implementation Decisions

### Journal layout, format, and durability

- **D-01:** **[LOCKED-BY-RESEARCH]** The layout is
  `.planning/meta-manager/runs/<run-id>/` inside the **driven project's** `.planning/`,
  containing `run.json` and `journal.jsonl`, with a sibling `runs/active` pointer file.
  `meta-manager/` is an existing app namespace, not a new one — `queue_md.rs:178` already
  owns `.planning/meta-manager/QUEUE.md` and already writes into third-party repos, so
  writing here needs no new justification and inherits an established precedent.

- **D-02:** Run ids are `<utc-timestamp>-<suffix>`, e.g. `2026-07-28T14-03-11Z-a3f9`:
  RFC3339 with `:` replaced by `-` (colons are legal on Linux but hostile on other
  filesystems and in shell paths), plus the first four hex chars of the run's session UUID.
  The format is chosen so **lexicographic sort equals chronological sort** — that is what
  makes retention pruning (D-22) and "find the newest run" a directory listing rather than
  a parse of every `run.json`. `chrono` is already a dependency.

- **D-03:** **[LOCKED-BY-RESEARCH]** `journal.jsonl` is append-only NDJSON: one JSON object
  per line, one trailing newline, no blank lines — byte-identical in shape to the Phase 15
  transcript fixtures, so the same reading discipline applies. Every record carries `ts`,
  a **monotonic** `seq` starting at 1, and a `kind`. `seq` exists so a tailing reader can
  detect gaps; a gap is reported, never treated as a parse failure.

- **D-04:** No `fsync` per event. Open with `OpenOptions::append(true)` and write each
  record as **one** `write_all` of a single buffer ending in `\n`, then flush the userspace
  buffer. Rationale: `fsync` per event would dominate the cost of a write path that fires
  every few seconds for hours, and it buys almost nothing here because the durability model
  is already "the reader tolerates a torn final line" (D-25). O_APPEND plus whole-line
  writes plus a tolerant reader is the same contract the research picked JSONL *for*.

- **D-05:** `run.json` is rewritten **atomically** via `tempfile::NamedTempFile::new_in` +
  `persist`, mirroring `src/config.rs:70-87` exactly. Not the `.tmp`-path + `fs::rename`
  variant in `queue_md.rs:220-224`: a fixed `.tmp` name collides if two writers ever race,
  and `config.rs` is the idiom research named. Create the run directory with
  `create_dir_all` before the first write, as `save_queue` does for `meta-manager/`.

- **D-06:** **`run.json` is written exactly twice, and both writes happen at a moment when
  no agent can be mid-commit.** Write one, at run start: the immutable record — run id,
  goal prompt, the GSD command, `ExecutionTarget`, the opt-in record, `started_at`, session
  id, pid/pgid, `claude_code_version`, argv digest. Write two, at terminal transition: the
  same document with `ended_at` and the derived `RunOutcome` stamped in. Nothing else ever
  rewrites it. This is a deliberate refinement of the research default rather than a
  contradiction of it — see D-07 for why the immutability is what makes committing safe.

- **D-07:** **OQ9 is decided: `run.json` is committed; `journal.jsonl`, `active`, and every
  future per-run file are gitignored.** Reasoning, recorded because the question was
  carried in explicitly:

  *For committing:* PROJECT.md's requirement is that the originating goal prompt is
  *legible later*, and OBS-03/OBS-05 are about after-the-fact review. A committed
  `run.json` is the only artifact that survives a fresh clone and is visible to a
  collaborator who was not at the terminal.

  *Against:* the **Aider "garbage commits" hazard** is real and specific here. The driven
  agent runs `git` inside the very worktree that contains `run.json`. Research's default
  shape — `run.json` rewritten on every status change — leaves that file perpetually dirty
  for the whole run, so any `git add -A` / `git commit -am` the agent issues sweeps a
  **mid-run** snapshot into an unrelated commit, recording `status: running` in history
  forever, and does so on a race between `add` and `commit`. That is exactly the
  meaningless-churn failure Aider users report.

  *Resolution:* keep the research default (commit the small record, ignore the big log) and
  neutralise the hazard with D-06's immutability. Write one lands before the agent is
  spawned; write two lands after it has exited. The worktree is never dirty with a
  *changing* `run.json` while an agent holds git. **Forward constraint for Phase 20:** when
  a run becomes a multi-invocation D-R-P-E-V loop, the terminal `run.json` write must still
  happen after the *last* `claude` invocation exits, not between steps. Per-step state
  belongs in the journal, which is ignored.

- **D-08:** The ignore entry is written at **run-directory creation time**, idempotently,
  not at opt-in time. Research says "at opt-in time"; opt-in is Phase 17's, and waiting for
  it would leave a window in which a journal exists and is not ignored — and a log written
  before its protection lands is exactly the class of mistake SAFE-04 exists to prevent.
  Write `.planning/meta-manager/runs/.gitignore` only if absent:

  ```gitignore
  # Written by gsd-meta-manager. Driver transcripts are local-only; the per-run
  # run.json (goal + outcome) is committed so the goal stays legible later.
  *
  !*/
  !.gitignore
  !*/run.json
  ```

  `!*/` is load-bearing: git will not re-include a file inside an excluded directory, so
  the directory must be un-excluded before `!*/run.json` can take effect. Verify this
  pattern with a real `git check-ignore` test, not by reading it.

### The OBS-06 fix — "free watching has a bill"

- **D-09:** **[LOCKED-BY-RESEARCH]** `Action::FileChanged` gains `changed_path: PathBuf`
  and keeps `project_path`. Both are needed: `project_path` is what the alias lookup in
  `app.rs:317-322` keys on, and `changed_path` is the classification input. `watcher.rs`
  already has the individual path in hand at the send site (`for path in &event.paths`), so
  the producer side is a one-line change; the consumer side is the phase's real work.

- **D-10:** **The watcher's per-root dedup must become per-`(root, classification)`, and
  this is a correctness fix, not a tidy-up.** Today `seen.insert(root.to_path_buf())` means
  the **first** path for a project root in a debounce batch wins and every later path for
  that root is silently dropped. Once journal appends start arriving, a single 200ms
  debounce batch can contain a journal append *and* a `STATE.md` write — and if the journal
  append is first, the `STATE.md` write is swallowed and the project never re-parses. That
  is a regression that classification *introduces*, so it must be handled in the same
  change: dedup on the pair, so each root can emit at most one driver-path event and one
  planning-path event per batch.

- **D-11:** Classification is a **pure function** with no I/O:
  `classify_change(project_root, changed_path) -> ChangeKind`, where `ChangeKind` is
  `DriverJournal { run_id }` (the path is under `.planning/meta-manager/runs/<id>/`) or
  `Planning` (everything else, which keeps today's behaviour as the default). It must be
  pure so it can be tested exhaustively against path shapes the way `extract_project_root`
  already is at `watcher.rs:105-110`, and so it cannot stat the filesystem on the
  debouncer's callback thread. Note that `extract_project_root` already resolves
  arbitrarily-deep `.planning/` paths and `watch()` is `RecursiveMode::Recursive`, so no
  new watcher registration is required — this really is the only cost of "free".

- **D-12:** A `DriverJournal` change routes to a new lightweight action
  (`Action::DriverJournalAppended { alias, run_id }` or equivalent) whose handler performs
  a **byte-offset tail** and **never** calls `parse_project_state`. Only `Planning` changes
  take the existing re-parse route. This is the literal content of OBS-06 and of success
  criterion 2.

- **D-13:** The tail's offset is stored in a **sibling map on `AppContext`**, keyed by
  `(alias, run_id)`, alongside `last_refresh` / `archive_cache` / `run_states`. It must not
  live on `ProjectState` (D-20). The tail reads from the stored offset to EOF, parses only
  **complete** lines, and advances the offset to just past the last `\n` it consumed — a
  trailing partial line is left unconsumed and re-read on the next append, which is what
  makes a torn write self-healing rather than a lost event.

- **D-14:** The driver route must **not** share `last_refresh` with the re-parse route.
  Sharing the 500ms dedup map would let a journal append suppress a genuine `STATE.md`
  re-parse for 500ms — trading one performance bug for a correctness bug. Classification
  happens **before** the dedup check, and the driver arm returns without touching
  `last_refresh`.

- **D-15:** No extra throttle on the tail path, and the reason is worth writing down: a
  tail read costs bytes-appended-since-last-read, so its cost is proportional to actual new
  data rather than to project size. That is the whole difference from `parse_project_state`,
  which reads STATE.md, ROADMAP.md, QUEUE.md, HANDOFF, every phase directory's disk
  inference and every workstream on every fire. The 200ms debounce already floors the rate.

- **D-16:** The tail runs on `spawn_blocking` with its result returned as an `Action` on a
  cloned `tx`, following the idiom already used at `app.rs:339-346` and for session
  detection. No file I/O on the render thread.

- **D-17:** Success criterion 2 ("as responsive as when idle") needs a **mechanical**
  measurement, not a claim. The cheapest honest one: a test that appends N events to a
  journal in a temp project and asserts the classification/tail route performs **zero**
  `parse_project_state` calls (e.g. by counting through a seam or by asserting no
  `ProjectStateLoaded` action is produced), plus a comparative timing assertion with a
  generous margin. Counting the re-parses is the load-bearing assertion; the timing number
  is corroboration and must not be the only evidence.

### State placement

- **D-18:** **[LOCKED-BY-RESEARCH]** Driver run state stays **off** `ProjectState`.
  `ProjectState` derives `PartialEq` and `app.rs:363-371` uses that equality to suppress
  the "Updated: {alias}" status message — a deliberate v1.4 feature. Driver state changes
  every few seconds (one 68-second spike turn emitted 22 `thinking_tokens` events), so a
  field there would flood the status bar for an entire multi-hour run and defeat the
  suppression outright.

- **D-19:** The `run_states: HashMap<String, RunState>` sibling map **already exists** —
  Phase 15 shipped it at `src/ui/screens/mod.rs:147` with `RunState` in
  `src/executor/mod.rs:587`. Phase 16 **extends** it; it does not recreate it and does not
  introduce a second parallel map for the same concept.

- **D-20:** No new `AppContext` field may be a handle. Journal offsets, run ids and counts
  only — `Action` derives `Clone`, a `File`/`JoinHandle` is not `Clone`, and the enum is
  already boxed for `clippy::large_enum_variant` (`ProjectStateLoaded` carries
  `Box<ProjectState>` for exactly that reason). Any new variant with a comparably large
  payload must be boxed or the build fails on `-D warnings`.

### Redaction (SAFE-04)

- **D-21:** **[LOCKED-BY-RESEARCH]** Redaction happens in the **capture path**, before
  bytes reach the file, and never in `src/ui/`. PITFALLS is unambiguous: retrofitting means
  every log written before the retrofit stays unredacted forever, and the user will `cat`
  or attach the file.

- **D-22:** **Enforce it with a type, not with discipline.** The journal writer accepts
  only a `Redacted<...>` newtype whose sole constructor is the redactor, so there is no
  compilable path that writes unredacted text. This is the same enforcement idiom the
  project already chose for `DrivableProject` in Phase 15 — the compiler, not a code
  review, is what holds the seam. Phase 15's own review flagged the gap this closes:
  *"No test asserts the absence of a future logging call… keeping it true is a review
  discipline, not a mechanically enforced one. Phase 16 owns redact-at-capture and should
  add the mechanical guard."*

- **D-23:** **Redact by walking the `serde_json::Value` tree, rewriting every string leaf
  and every object key, then serialising** — not by regex-replacing the finished line.
  Both positions are structurally exhaustive (and exhaustiveness is the actual lesson of
  WR-15, where the leak was inside a payload nobody thought to scan), but the tree walk
  cannot produce invalid JSON, whereas post-serialisation replacement can corrupt escaping.
  Keys are scanned too: `memory_paths`-style maps put paths in key position. Cost is one
  extra `Value` round-trip per event, which is free at ~1 event/sec. Belt and braces: a
  test parses every written line back to confirm validity.

- **D-24:** The pattern set must cover **both** encodings of a home path — this is the
  concrete Phase 15 leak (`WR-15`, commit `0a9b6d8`) that survived two scans:
  1. **Slash form** — `/home/<user>/...`, and `/Users/<user>/...` for macOS.
  2. **Dash-encoded form** — Claude Code's session-directory shape,
     `-home-<user>-projects-...` and `-tmp-claude-<uid>-`. A `/home/<user>` grep alone is
     **not** evidence of a clean capture. The redacted output should preserve the *shape*
     (`-home-redacted-project`) so downstream structure stays realistic.
  3. **Credential shapes** — `sk-ant-…`, generic `sk-…`, GitHub `gh[pousr]_…`, AWS
     `AKIA…`, `Bearer <token>`, `Authorization:` header values, JWTs (`eyJ…`), PEM private
     key blocks, URL userinfo (`https://user:pass@host`), and env-assignment shapes where
     the name matches `*_TOKEN|*_SECRET|*_KEY|*_PASSWORD|ANTHROPIC_API_KEY`.
  Replacements are fixed literals of the form `[REDACTED:<kind>]`, containing no quote,
  backslash or newline.

- **D-25:** **Tune toward over-redaction.** A false positive costs a slightly less readable
  log; a false negative is a persistent secret sink under `.planning/`. Record the honest
  limit in the module docs: a pattern redactor cannot catch an arbitrary high-entropy
  secret with no recognisable shape. SAFE-04 is a capture-path control, not a guarantee —
  Phase 19's tool-boundary denial is the complementary control that fires *before* the
  secret enters context, and neither substitutes for the other.

- **D-26:** **No raw sidecar, no unredact path, no "verbose mode" that skips the filter.**
  Any of the three would reintroduce the exact file SAFE-04 exists to prevent.

- **D-27:** Redaction tests assert on the **bytes on disk**, not on an in-memory value —
  reading the file back and searching for each planted secret. An in-memory assertion is
  the same category of vacuous check as grepping filtered `cargo` output for `warning:`.
  Include the dash-encoded form as its own planted case, since that is the one that has
  historically slipped through.

- **D-28:** Any `tracing` call inside the journal modules routes through the same redactor
  or carries no event content at all. `claude.rs:1263` already documents the byte-counts-
  only discipline for the executor; Phase 16 makes it real for its own modules rather than
  widening the audit.

### Durability, growth, and retention

- **D-29:** Ordering is the durability contract: an event is written before the driver
  proceeds past the action it describes, so "the last event before death" (success
  criterion 1) is genuinely the last thing that happened rather than whatever a buffer
  happened to hold. A dedicated writer task with an ordered channel is fine; a fire-and-
  forget spawn per event is not.

- **D-30:** The reader is tolerant in three specific ways, mirroring `stream_json.rs`'s
  `#[serde(tag = "type")]` + catch-all idiom: a **torn final line** is skipped, not fatal;
  an **unknown `kind`** is carried as raw, not fatal (forward compat for Phase 20's event
  kinds); a **`seq` gap** is reported as a diagnostic, not an error. Model `JournalEvent`
  as `#[serde(tag = "kind")]` with a catch-all variant and never `deny_unknown_fields`.

- **D-31:** Growth is bounded at two levels, because the two failure modes are different.
  Per-event: cap payload text at a few KB — the Phase 15 fixtures showed 4-8 KB base64
  thinking signatures dominating file size, so this alone removes most of the volume. Per-
  run: a total byte cap (a defensible starting value, made configurable, in the tens of MB
  — a 4h run is tens of MB of JSON) after which the writer emits one `journal_truncated`
  event and continues writing **only** lifecycle/decision/outcome events, never content.
  The run must always be able to write its terminal record; silently dropping the ending is
  the one failure OBS-01 cannot tolerate.

- **D-32:** Retention prunes at **run start**, not at run exit, keeping the last N complete
  runs per project (default 10, configurable). An exit-time prune is skipped by exactly the
  crash the phase is built to survive. Never prune the active run, and never prune a run
  with no `ended_at` — that record is precisely what Phase 17's crash reconciliation reads.

- **D-33:** Journal the **dropped-event count** from Phase 15's bounded drain. Plan 15-08's
  summary hands this over explicitly: *"a run that silently lost 40 events is materially
  different from one that lost none, and the count is the only signal that distinguishes
  them."* Today it is a `tracing::warn!` and nothing else.

- **D-34:** Crash survival is tested **mechanically**: write N events, `SIGKILL` the writing
  process, reopen, assert every event up to the kill point is readable and the file parses.
  `tests/executor_lifecycle.rs` and the `tests/fixtures/fake-claude-*.sh` harness already
  do process-group teardown, so the machinery exists and should be reused rather than
  rebuilt.

### Module layout and wiring

- **D-35:** New module `src/journal/`, mirroring `src/executor/`'s four-submodule shape:
  `mod.rs` (the `JournalEvent` model, run-id, layout paths, the domain surface later phases
  build against), `writer.rs` (append writer, atomic `run.json`, the `.gitignore` seam),
  `reader.rs` (tail-from-offset, tolerant parse, gap detection), `redact.rs` (patterns plus
  the `Redacted` newtype). Classification (D-11) lives with `extract_project_root` in
  `watcher.rs` or in `journal/mod.rs` — planner's call; it must be pure and directly tested
  either way.

- **D-36:** Phase 16 wires the journal as a **consumer** of the existing `ExecutionEvent`
  stream (`ExecEvent` on the bounded `exec_tx` channel, `src/main_loop.rs:69-77`) plus its
  own run-start/run-end lifecycle events. It does **not** build the observe→decide→act
  loop, detached spawn, or any rendered surface. The `kind` set includes `decided` and
  `parked` in the schema so Phase 20 needs no migration; Phase 16 emits only the kinds it
  can actually produce, and says so.

- **D-37:** `claude_code_version`, `session_id`, capabilities and the derived `RunOutcome`
  are read off the existing `ExecutionHandle` / `GateOutcome` / outcome types — Phase 15
  recorded all four specifically so Phase 16 could journal them **without a signature
  change**. Do not widen those types. Carry the `run_cost_usd` notional-cost caveat from
  15-05 into the journal's field docs rather than restating cost as if it were billed.

### Testing and gate hygiene

- **D-38:** The project gate is `cargo build && cargo test && cargo clippy -- -D warnings`.
  `cargo clippy --all-targets` has exactly **5 pre-existing** lints (browser.rs ×3,
  project_creator.rs ×1, state_reader/mod.rs ×1). They are out of scope; the count must not
  grow. Baseline at phase start: **363 tests passing** (308 lib + 11 + 7 + 12 + 25).

- **D-39:** `cargo` output in this environment is filtered by an `rtk` summarising wrapper,
  so raw `warning:` and `test result:` lines are absent. **Any acceptance criterion that
  greps for those markers passes vacuously.** Use `rtk proxy cargo …` wherever a criterion
  depends on raw output — and prefer criteria that assert on artifacts (file bytes, counted
  calls) over criteria that grep build logs.

### Claude's Discretion

- Plan granularity and wave structure. A defensible split is: (1) journal model + writer +
  `run.json` + layout + gitignore; (2) `redact.rs` + the `Redacted` type + on-disk tests;
  (3) `changed_path` + classification + the watcher dedup fix; (4) the tail reader + the
  `app.rs` route + the OBS-06 measurement. (2) should not trail (1) by much — the writer
  must not exist unredacted even for one commit.
- Whether classification lives in `watcher.rs` or `journal/mod.rs` (D-35).
- The concrete default values for the per-event payload cap, the per-run byte cap and the
  retention count (D-31, D-32) — pick defensible starting numbers, name them as constants,
  and make them configurable. No tuning data exists; that is a v2.1 concern.
- Exact `JournalEvent` variant names and field shapes, beyond `ts`/`seq`/`kind`.
- Whether `Redacted` is generic or a concrete `RedactedLine`.
- Exact test names and placement, following the existing `#[cfg(test)] mod tests` convention
  plus `tests/` integration files where a real process or real filesystem is needed.
- Whether the `active` pointer is a file or is derived from a directory listing (D-02 makes
  the listing sortable, so either is defensible; the file is cheaper to poll, the listing
  cannot go stale after a crash).

</decisions>

<code_context>
## Existing Code Insights

Every anchor below was read directly against the current tree.

**The watcher (producer side of OBS-06)**
- `src/watcher.rs:21-34` — `extract_project_root` walks up to `.planning` at arbitrary
  depth; `watch()` at `:71-79` uses `RecursiveMode::Recursive`. Together these mean a write
  to `.planning/meta-manager/runs/<id>/journal.jsonl` already fires `FileChanged` with
  **zero new watcher code**. The existing test at `:105-110` proves the depth behaviour.
- `src/watcher.rs:45-59` — the debounced callback. `let mut seen = HashSet::new()` then
  `if seen.insert(root.to_path_buf())` is the per-root dedup D-10 has to change. The
  individual `path` is in scope at the `tx.send` site, which is why adding `changed_path`
  costs one line here.
- 200ms debounce window via `new_debouncer(Duration::from_millis(200), None, …)`.

**The consumer side**
- `src/action.rs:5-45` — `#[derive(Debug, Clone)] pub enum Action`, 10 variants.
  `FileChanged { project_path }` at `:9-11`. `ProjectStateLoaded` boxes `ProjectState` with
  the comment *"Boxed: ProjectState dwarfs every other variant
  (clippy::large_enum_variant)"* — the constraint D-20 restates.
- `src/app.rs:316-358` — the `FileChanged` arm: alias lookup by `proj.path == project_path`,
  the 500ms `last_refresh` dedup at `:325-331`, then `spawn_blocking(parse_project_state)`
  at `:339-346`, then an auto-watch fallback at `:350-356`. This whole arm is what
  classification has to fork.
- `src/app.rs:359-379` — `ProjectStateLoaded`: `*old_state != *state` is the equality
  D-18 must not break.
- `src/state_reader/mod.rs:102-240` — `parse_project_state`: STATE.md, ROADMAP.md, QUEUE.md,
  HANDOFF, per-phase disk inference, workstreams, plus a `git_ops::project_last_activity`
  shell-out. Synchronous, idempotent, never panics — and far too expensive to run twice a
  second for hours.

**State placement**
- `src/ui/screens/mod.rs:113-156` — `AppContext`. `run_states: HashMap<String, RunState>`
  already exists at `:147` with the D-18 rationale written into its doc comment;
  `last_refresh` (`:149`) and `archive_cache` (`:155`) are the sibling-map precedent D-13
  follows.
- `src/executor/mod.rs:586-599` — `RunState { Idle, Starting, Running, Stopping,
  Finished(Box<RunOutcome>) }`, already `PartialEq` and boxed.
- `src/app.rs:177-206` — `apply_exec_event`, the existing `ExecutionEvent` → `RunState`
  reducer. The journal consumer sits alongside this, not instead of it.

**What Phase 15 already recorded for this phase to journal**
- `src/executor/mod.rs:420-474` — `ExecutionEvent` (`SessionStarted`, `Message`, `Unknown`,
  `Unparseable`, `LineTruncated`, `Stderr`, `TurnCompleted`, `Cost`, `Exited`).
- `src/executor/mod.rs:514-577` — `RunOutcome`, 9 variants.
- `ExecutionHandle` (`:~330-360`) carries `session_id`, `capabilities`, `pgid` and
  `claude_code_version` — the last with the doc *"recorded so Phase 16 can journal it
  without a signature change (D-07)"*.
- `src/main_loop.rs:49-77` — `EXEC_BATCH = 64`, `EXEC_CHANNEL_CAPACITY = 8192`, and
  `ExecEvent { alias, event }`. The bounded drain here is what produces D-33's dropped-event
  count.

**Write idioms already in the codebase**
- `src/config.rs:70-87` — `NamedTempFile::new_in(dir)` + `write_all` + `persist`. D-05's
  model.
- `src/state_reader/queue_md.rs:207-229` — `save_queue`: `create_dir_all(meta_dir)` then a
  fixed `QUEUE.md.tmp` + `rename`. Precedent for writing into `.planning/meta-manager/` in a
  third-party repo; **not** the atomic-write idiom to copy (fixed temp name).
- `src/app.rs:295-300, 339-346` — `spawn_blocking` + result-as-`Action` on a cloned `tx`.
  D-16's model.

**Redaction precedent and the leak to learn from**
- `tests/fixtures/transcripts/README.md`, "Redaction record (D-25)" — documents the four
  transforms and the WR-15 retrofit verbatim: the original sweep matched only the slash form
  and missed the dash-encoded `-home-<user>-<repo>` / `-tmp-claude-<uid>-` shapes in **seven
  of eight** fixtures. Commit `0a9b6d8` is the fix. The README's own conclusion is D-24's
  rule: *"Any future redaction sweep must scan both encodings."*
- `src/executor/claude.rs:56-62, 1260-1266` — the byte-counts-only tracing discipline and
  the explicit note that redact-at-capture lands in Phase 16.
- `src/executor/stream_json.rs` — the `#[serde(tag = "type")]` + catch-all tolerant-parsing
  shape D-30 mirrors.

**Dependencies and gates**
- `Cargo.toml` — `rust-version = "1.87"`, version `1.6.0`. `serde_json`, `chrono`,
  `tempfile`, `uuid`, `regex`, `notify` 8 + `notify-debouncer-full` 0.7 are all already
  present. **Phase 16 should need no new dependency**; if the planner reaches for one, that
  is a signal to re-check, not a routine addition.
- `cargo build && cargo test && cargo clippy -- -D warnings` must pass. 5 pre-existing
  `--all-targets` lints, count must not grow (D-38). 363 tests at phase start.
- `.gitignore` at repo root is two lines (`/.claude/worktrees/`, `/target/`); there is no
  `.planning/.gitignore` today, so D-08 creates the first one.

</code_context>

<specifics>
## Specific Ideas

### The layout, as research drew it

```
.planning/meta-manager/                    ← app namespace already exists (queue_md.rs:178)
└── runs/
    ├── .gitignore                         ← D-08. Written once, idempotently.
    ├── active                             ← one line: <run-id>, or absent. Cheap to poll.
    └── 2026-07-28T14-03-11Z-a3f9/
        ├── run.json                       ← COMMITTED. Written exactly twice (D-06).
        ├── journal.jsonl                  ← GITIGNORED. Append-only, one event per line.
        └── inbox.jsonl                    ← Phase 18. Not this phase's file.
```

### Event schema sketch (from ARCHITECTURE §4.5, to be firmed up in planning)

```jsonc
{"ts":"…","seq":1,"kind":"run_started","goal":"…","dry_run":false,"target":"host"}
{"ts":"…","seq":2,"kind":"observed","phase":"14","drpev":["Complete","Skipped","Current","NotStarted","NotStarted"]}
{"ts":"…","seq":3,"kind":"decided","by":"policy","command":"/gsd:execute-phase 14","rationale":"…"}
{"ts":"…","seq":4,"kind":"exec_started","session_id":"<uuid>","argv_digest":"sha256:…"}
{"ts":"…","seq":5,"kind":"exec_event","stream":"assistant","text":"…"}
{"ts":"…","seq":6,"kind":"interjected","text":"…","delivered":true}
{"ts":"…","seq":7,"kind":"exec_finished","exit":0,"cost_usd":1.83,"duration_s":420}
{"ts":"…","seq":8,"kind":"parked","reason":"verification_gaps_found","needs":"human"}
```

`observed` / `decided` / `parked` are **Phase 20's** to emit. They are in the schema now so
that phase adds no migration, and the reader tolerates unknown kinds regardless (D-30).
`exec_event` is where redaction earns its keep — that is the field carrying agent prose,
tool results, and anything the agent read out of a config file.

### Why the "bill" is not theoretical

`parse_project_state` reads every planning artifact and shells out to git for last-activity.
The 500ms dedup at `app.rs:325-331` caps that at **2 full re-parses per second,
indefinitely** — for a four-hour run, roughly 28 800 of them. The fix is not an
optimisation; it is the difference between a usable TUI and an unusable one during exactly
the workflow this milestone exists to enable.

### The three properties the phase is actually judged on

1. **Survives death.** Kill the writer mid-run; the journal still shows every completed
   step and the last event before death. Tested with a real signal (D-34), not a mock.
2. **Costs nothing to watch.** An hour of appends leaves navigation as responsive as idle,
   proven by counting re-parses (D-17), not by asserting it.
3. **Redacted on disk.** A planted credential — and a planted dash-encoded home path — is
   already `[REDACTED:…]` in the file, verified by reading the bytes back (D-27).

</specifics>

<deferred>
## Deferred Ideas

Each considered and deliberately pushed out, with its owner named:

- **Detached spawn, `Commands::Drive`, PID liveness probing, crash reconciliation, the
  kill switch, dry-run, `flock` single-run lock, the `driver_opt_in` record and its UI**
  → Phase 17. Phase 16 *writes the facts* reconciliation will read (pid/pgid and the
  absent-`ended_at` signal, D-06/D-32); it does not reconcile.
- **`inbox.jsonl`, the TUI→driver interjection channel, and STEER-03's
  survive-a-restart-between-queue-and-delivery behaviour** → Phase 18.
- **The Driver tab, live stream render, ring-buffered output, dashboard badges, the
  three-state delivery display, and any rendering of journal content** → Phase 18. Phase 16
  ships no UI; the tail exists to keep state cheap, not to display it.
- **Pre-push secret scan (gitleaks or equivalent), the `--disallowedTools` credential-read
  denylist, the branch-namespace push allowlist, scoped git credentials** → Phase 19. Only
  the capture-path half of SAFE-04 is here.
- **`decide()`, the D-R-P-E-V router, lifting `derive_all_stage_statuses` out of
  `src/ui/screens/detail.rs`, run-level step/no-progress/wall-clock caps, the quota-floor
  park** → Phase 20. Phase 16 defines `observed`/`decided`/`parked` as schema only.
- **LLM goal decomposition, `--json-schema`, untrusted-content delimiting** → Phase 21.
- **`ExecutionTarget::Container`, `PathMap`, and container↔host path translation inside
  journal records** → Phase 22. Journal paths are host paths in v2.0.
- **SQLite for the journal** — considered and rejected in research: a dependency, a schema
  migration story and a lock story, to serve a single-writer/single-reader append log.
  Revisit only if querying run *history* becomes a feature.
- **A Unix control socket (`control.sock`)** — deferred to a later milestone by
  REQUIREMENTS. The durable file inbox covers the requirement; the socket is a latency
  optimisation for a workflow whose unit of work is measured in minutes.
- **Live re-streaming of a run's output after TUI restart** — impossible by construction
  (REQUIREMENTS "out of scope"): once the TUI exits the child's stdout pipe is gone.
  Reattachment is journal-based and read-only, which is exactly what OBS-01 scopes.
- **Fixing the 5 pre-existing `--all-targets` clippy lints** — unrelated; the count must not
  grow but is not this phase's to shrink.
- **Fixing the `pending_editor` blocking shell-out on the render thread** — carried over
  from Phase 15's D-18 as a known wart. Still not this phase's.
- **`session_detector.rs`'s `pts/1` vs `pts/11` TTY substring match** — a real bug, still
  belonging to the tmux attach path. Capture as a quick task.

</deferred>
