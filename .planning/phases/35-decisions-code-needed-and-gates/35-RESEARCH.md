# Phase 35: Decisions, code-needed and gates - Research

**Researched:** 2026-11-14
**Domain:** Rust state reader (owner-items Layer A port of gsd-core 1.21.0 `audit-open` + Layer B owner rules), two read-only CLIs (`inbox`, `status`)
**Confidence:** HIGH for the code, upstream behavior and live data (all read or executed this session); MEDIUM for the inferred rules flagged below

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward from Phase 34 (not re-decided)
- Layer A = faithful process-free port of gsd-core 1.21.0, pinned by an oracle in `tests/` only;
  Layer B = the owner's rules, unit-tested, never compared with the oracle (34 D-05, D-08).
- Separate entry point `read_owner_digest`, never inside `parse_project_state`; `now` and
  `stale_days` passed in; per-file (path, mtime, len) parse cache (34 D-14..D-16).
- Capped contained reads (1 MiB), fail-safe "unreadable, never clean", every emitted
  project-authored string through `terminal_capped` / `render_for_terminal` (34 D-17).
- Schema `gsd-meta-manager/inbox` v1 and `status --json` grow by additive keys only; no
  `schema_version` bump; existing key paths in `tests/inbox_cli.rs` and `tests/status_cli.rs` stay
  unchanged (34 D-19, D-22; roadmap SC4).
- Synthetic fixtures with neutral invented text; real projects under `~/projects` are read-only
  sanity checks, never committed (34 D-10).

#### Signal model (OWN-10)
- **D-01:** `Kind` gains `Debug`, `Handoff`, `Checkpoint`, `ContextQuestions` (report §4 names
  `Debug | Handoff | ContextQuestions | Todos`; `Checkpoint` is added for `.continue-here.md`
  rows) **[inferred]**. Todos stay a count, not a group kind (report §6 "todo triage UI beyond a
  count" is deferred). JSON `kind` tokens: `debug`, `handoff`, `checkpoint`, `context_questions`.
- **D-02:** Debug sessions are read natively from the root-scoped `.planning/debug/*.md` (and each
  workstream's own debug dir only if upstream's `debugDirFrom` resolves one there; otherwise root
  only, as report §4 says) as a port of gsd-core `scanDebugSessions` (`audit.cjs` ~278): regular
  `.md` files only, `knowledge-base.md` skipped by exact name, `resolved/` never descended, path
  containment as `requireSafePath`, lone-CR normalized, missing/unparseable frontmatter = status
  `unknown` = open, `resolved`/`complete` closed, `audit_acknowledged` honored with snapshot key
  `status`.
- **D-03:** Debug status normalization (Layer B): trim, lower-case, `-`→`_`. Normalized value
  starting with `awaiting` → **Decide** (D2), row text "awaiting your confirmation", command
  `/gsd-debug`. Any other open status (`root_cause_found`, `diagnosed`, `investigating`, `unknown`,
  …) → **Fix** ("open investigation"), command `/gsd-debug`. Raw status is kept for the oracle and
  emitted as `file_status`.
- **D-04:** CONTEXT open questions: Layer A ports `deriveOpenQuestions` (`audit.cjs` ~197:
  frontmatter `open_questions` array, else the level-2 `## Open Questions` section bullets/`?`
  lines, `none` body = empty) over the same phase targets as `scanContextQuestions` (active
  current-milestone dirs plus archived milestone dirs), acknowledged via `questions_digest` = sha256
  of the length-prefixed encoding (`deriveOpenQuestionsDigest`; `sha2` is already a dependency).
  Layer B drops bullets whose text carries a RESOLVED / ANSWERED / CLOSED marker (D11): case-insensitive
  whole word, as a leading token (after the bullet/number and optional `**`/`~~`/`[x]`) or as a
  `(RESOLVED)`-style tag anywhere in the line **[inferred: exact matcher]**. A file whose every
  question is filtered contributes no group (mailbot 06-CONTEXT case). Each remaining question is one
  item.
- **D-05:** Handoff: reuse the existing `ProjectState` handoff detection (`paused`, `pause_context`,
  `stale_handoff`) so a stale handoff is not owner work, and add reading of
  `human_actions_pending[]` and `blockers[]` from the same `HANDOFF.json` parse. One Decide group per
  non-stale paused handoff ("handoff paused"), its items = the pending human actions and blockers
  (each one item, text capped); a non-paused HANDOFF with non-empty `human_actions_pending[]` still
  yields the group **[inferred]**. Command `/gsd-resume-work` (R10).
- **D-06:** Phase checkpoints: each `phases/<p>/.continue-here.md` of the current milestone with a
  frontmatter `status` that is not terminal yields one Decide item in a `checkpoint` group for that
  phase (fields `status`, `phase`), command `/gsd-resume-work` **[inferred: command]**. The existing
  root-level `continue_here_present` stays as it is for `needs_human`; it is not double-counted
  (the root file is the handoff path, D-05).
- **D-07:** `gaps_found` verifications stay in Fix exactly as Phase 34 builds them (R1). New rule
  **[inferred]**: a `gaps_found` VERIFICATION with zero parseable gap/human items still yields a Fix
  group with `counts.unreadable = 1` ("verification gaps_found, no gaps listed"), never nothing
  (fail-safe principle). Command `/gsd-plan-phase N --gaps`.
- **D-08:** Pending todos: count and oldest age from `oneoff::todos::list_pending` plus each file's
  `created:` frontmatter, else mtime (same fallback order as R9). Emitted as additive per-project
  `hidden.todos_oldest_days` (null when no todos) **[inferred: key name]**; `hidden.todos` keeps its
  Phase 34 meaning. Fixes 34-REVIEW IN-05 on the way: a failed listing reports `null`, not 0
  **[inferred]**.
- **D-09:** Parked driver run and external job stay in the existing `needs_human` sources; the inbox
  does not re-list them (report D1: `❢`/`needs_human` are not widened or duplicated) **[inferred]**.
- **D-10:** Scope: debug sessions and todos are root-scoped (as upstream); CONTEXT questions and
  checkpoints follow the phase scan targets (root and workstreams, current and archived). Archived
  CONTEXT questions land in Old debt like other archived groups (R5).

#### Counts and the milestone gate (OWN-08, extends OWN-13/15)
- **D-11:** `decide` = number of Decide **items** in the current scope (each awaiting debug session,
  each paused handoff counts 1 plus nothing for its listed actions **[inferred]**, each checkpoint,
  each open question). Fills the Phase 34 placeholder `summary.decide` and `status --json`
  `owner.decide` (no new key). Open investigations count in a new additive `fix_debug` (summary and
  `owner`) **[inferred: key name]**, because `fix_phases` counts phases and a debug session has none;
  `fix_phases` keeps its meaning.
- **D-12:** `milestone.closes_after` (per project, omitted in Phase 34, added now):
  `{test, decide, fix, blocked, phases_open: [...], not_built: [...]}` **[inferred: `not_built`
  key]**. Totals are item counts over phases of the **current milestone** that are not complete in
  ROADMAP, after Layer B (R2 supersede, R4 acknowledged, R5 archived excluded, R6 deferred
  excluded). `fix` counts Fix items (failed checks plus gaps of phase-scoped groups), not phases.
  Root-scoped signals (debug, handoff, todos) are not attributed to a phase and are excluded from
  `closes_after` **[inferred]**. `phases_open` = non-complete current-milestone phases that have
  ≥ 1 plan, sorted numerically; phases with no plans go to `not_built` and contribute no owner work
  (R8). Workstreams: the project-level `milestone.closes_after` is the root scope's **[inferred]**;
  whether workstream scopes expose their own is Claude's discretion (additive either way).
- **D-13:** The plain `inbox` checklist gains the DECIDE section and the Fix sub-rows of report §3.3
  (`DECIDE`, `FIX — code needed`) and a per-project `closes after:` line in the header format of
  §3.2; golden files are updated and the old sections stay byte-identical where no new signal is
  present. `--lane decide` and `--lane fix` filter the new groups; `--include todos` keeps listing
  todo titles.

#### Owner-facing names of VERIFICATION items (added scope, orchestrator observation)
- **D-14:** **Bug:** live `inbox` shows `human_verification` entries as gsd-core's flattened mapping,
  e.g. `test: Weekly summary live send (PRICE-14) …, expected: …, why_human: …`, because
  `verification.rs::human_verification_items` uses `normalize_entry(display)` where `display` comes
  from `frontmatter::fm_list` → `flatten_object_list_item`. **Decision:** Layer A (`AuditItem.name`
  as compared with the `audit-uat` oracle) keeps gsd-core's string unchanged. Layer A additionally
  records the entry's structured fields when the entry is a mapping (not compared with the oracle),
  and Layer B builds `OwnerItem` from them: `name` = the entry's `test` field, else `truth`, else
  `name`, else `description` **[inferred: fallback order]**, else the flattened string as today;
  `expected` = the entry's `expected`; new `why_human` = the entry's `why_human`. A scalar entry is
  unchanged. The same applies to `gaps` mapping entries without `truth` (they also fall back to the
  flattened string today) **[inferred]**.
- **D-15:** Output: inbox JSON item `name` carries only the test text; `expected` is filled; a new
  additive item key `why_human` (string or null, through `terminal_capped`) appears on every item
  (null for UAT rows). The plain checklist prints the clean name on the `- [ ]` line (expected /
  why_human not printed in plain mode **[inferred]**, they are in JSON and in the Phase 36/37
  expanded rows). Item `id` derivation must not depend on the changed display name if it currently
  does (check `rules.rs` `ids.next`; keep ids stable or document the change) **[inferred]**. Changing
  a value, not a key, needs no schema bump; README's `inbox` contract documents `why_human`.
- **D-16:** **Regression test required:** a fixture (extend `verification-gaps-found` or add
  `verification-human-mapping`) whose `human_verification` list holds mapping entries with `test`,
  `expected`, `why_human` (plus one scalar entry and one mapping without `test`). Asserts: (a) the
  oracle comparison still passes (Layer A name = flattened string); (b) `inbox --json` item `name`
  equals the bare test text, contains no `expected:` / `why_human:` substring, and `expected` /
  `why_human` hold the separate values; (c) the plain checklist line shows only the test text.

#### Conformance (Layer A of the new signals)
- **D-17:** Extend the oracle pattern to the new Layer A **[inferred]**: `tests/owner_audit_conformance.rs`
  (or a sibling file) runs gsd-core's `audit-open` (`query audit-open --json`/`--raw`, exact spelling
  confirmed during research) on fixtures carrying debug sessions and CONTEXT questions and compares
  the open set (debug: file, status; CONTEXT: file, question_count, and the acknowledged counts)
  with the native port. Layer B (D-03 normalization lanes, D-04 RESOLVED filter) is unit-tested and
  deliberately differs. Same `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` opt-out; zero comparisons fails;
  the `src/` source scan also forbids `audit-open` as a spawned verb. HANDOFF, checkpoints, todos
  and `closes_after` have no upstream oracle and are fixture-tested only.
- **D-18:** One fixture per row of the report §4 signal table (OWN-10 test): paused handoff,
  human actions pending, phase checkpoint, debug awaiting (both `awaiting_human_verify` and
  `awaiting-user`, mixed case), open investigation, `knowledge-base.md` + `resolved/` ignored,
  `gaps_found` verification (with and without listed gaps, D-07), open questions with
  RESOLVED/ANSWERED/CLOSED bullets (one fully answered file), pending todos with `created:` and
  without. Plus a `closes_after` fixture modeled on wordoclock ph 20-25 (synthetic text) asserting
  test 12, blocked 5, fix 1, decide 0, `phases_open` 20-25, and a not-built phase; and a decide-lane
  fixture modeled on sentriq (5 awaiting sessions, across workstreams) and usbee (paused handoff +
  1 human action) for SC1. Live check on the owner's machine (read-only): `inbox --json --lane decide`
  lists sentriq's 5 sessions and usbee's handoff; wordoclock `closes_after` names 20-25.

#### Musk pass
- Milestone pass already done (`.planning/musk/MILESTONE-v2.5.0.md`, `pass_due: false`). Phase pass:
  no cuts. OWN-08 and OWN-10 trace to the report adopted as written, and D-14..D-16 were asked for
  by the orchestrator from live output; tests guarding them are never cut **[inferred]**. Lean lane
  not used: multiple subsystems (reader, rules, two CLIs, README, fixtures, oracle) and more than
  3 tasks.

### Claude's Discretion
- Module split for the new readers (e.g. `owner_items/debug.rs`, `context_questions.rs`,
  `handoff.rs`), type layout of the structured human-verification fields, and whether the new
  signals share the per-file parse cache (they should if cheap).
- Exact Decide/Fix row wording in the plain checklist, within report §3.3's shape.
- Whether workstream scopes expose their own `closes_after` in JSON.
- Number and grouping of plans (coarse granularity).

### Deferred Ideas (OUT OF SCOPE)
- Report §6 deferred list (OWN-F01..): STATE.md Blockers/Concerns parsing, heuristic ruling
  detection, todo triage UI beyond a count, seeds and threads, native acknowledge writer,
  notifications, daily digest.
- Upstream issues to report (not fixed here): `deriveOpenQuestions` counts answered questions (§8
  item 8), debug status drift `awaiting-user` vs `awaiting_human_verify` (§8 item 9).
- 34 VERIFICATION follow-ups not in scope: `App::update(FileChanged)` eviction test (Phase 36 when
  the TUI consumes the cache), IN-02 id collision after truncation, IN-04 byte-bounded cache.

#### Reviewed Todos (not folded)
`todo.match-phase 35` returned the same 17 keyword-only matches Phase 34 reviewed (score 0.2-0.9:
TUI badge for release maps, help popup at 80x24, unregister guard, registry glyphs, launcher-marker
in-flight flag, executor remnants, capture-pump redaction, run-lock hard link, managed-policy shapes,
run-file hardening, unsupervised child, multi-line queue input, ambiguous-wide typography, driverctl
`fm_status` IN-06, driverctl LOG.md redaction, D-17 recovery block, sibling-run tmpfs). None touches
the owner reader or its CLIs, so none is folded despite the ≥ 0.4 auto-fold rule; folding them would
be scope creep **[inferred]**.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OWN-08 | R8: `closes_after` lists lane totals and open phases for the current milestone only; phases with no plans are named "not built", not counted as owner work. | §"closes_after computation" (data sources `in_current_milestone`, `RoadmapPhase.completed`, `DiskInference.plan_count`); wordoclock live numbers verified (test 12 / blocked 5 / fix 1 / decide 0 / phases 20-25); fixture plan in Validation Architecture. |
| OWN-10 | Decide/Fix signals of report §4 read with the stated fields; debug status normalized; `knowledge-base.md` and `resolved/` ignored; RESOLVED/ANSWERED/CLOSED bullets filtered; todo count and oldest age. *Test: one fixture per signal-table row.* | Upstream `scanDebugSessions`, `deriveOpenQuestions`, `deriveOpenQuestionsDigest`, `scanContextQuestions` read and executed; HANDOFF.json and `.continue-here.md` shapes surveyed on the machine; three blocking findings (F1-F3 below) that change D-04/D-05 implementation. |
| OWN-13 (ext) | `inbox --json` schema v1 grows additively. | Exact-set assertions in `tests/inbox_cli.rs` and the README fence/table pins enumerated; list of new key paths. |
| OWN-15 (ext) | `status --json` `owner` grows additively. | `SCHEMA_PATHS`, `agent_skill.md` fence, README fence and table, and `owner_counts_cover_the_current_milestone` exact-JSON assertion identified. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Rust 1.89+ (`rust-version = "1.89"` [VERIFIED: Cargo.toml:18]); ratatui/crossterm/tokio stack; no new dependency is needed for this phase (`sha2 = "0.11.0"` [VERIFIED: Cargo.toml:87], `serde_yml = "0.0.13"` [VERIFIED: Cargo.toml:43], `chrono`, `regex`, `tempfile` already present).
- **State reading must not require running Claude/GSD**: nothing under `src/` spawns gsd-tools; the oracle runs only in `tests/` (enforced by `no_source_file_names_the_audit_oracle_verb`).
- **Non-intrusive**: `inbox`/`status` write nothing and take no lock (`inbox_writes_nothing_anywhere` test exists).
- **Portability**: honor `CLAUDE_CONFIG_DIR`/`CODEX_HOME` (oracle resolution goes through `gsd_install::install_candidates`).
- Logs to file only (tracing); never stdout in TUI paths.
- `std::sync::Mutex` must not be held across `.await` (the parse cache never awaits; keep it that way).
- RTK: run `rtk bypass cargo test ...` whenever a check depends on raw cargo output (it strips `test result:` lines).
- Release gates (pre-tag): `cargo clippy --locked --target T --all-targets -- -D warnings` for every release.yml target, including `x86_64-pc-windows-gnu` and `aarch64-apple-darwin` (34-06-SUMMARY ran both). Unix-only helpers must be `#[cfg(unix)]`-gated with no dead code on Windows.
- GSD workflow: edits happen inside `/gsd:execute-phase`; US spelling.
- User global rules: conventional commits, path-scoped `git commit -- <paths>`, never `git add -A`, end with clean `git status`. (This research task itself must not commit.)

## Summary

Phase 35 extends the Phase 34 owner reader (`src/state_reader/owner_items/`, 7 modules, ~6.3k lines) with four new signal readers (debug sessions, CONTEXT open questions, HANDOFF.json, phase `.continue-here.md`), a todo-age stat, the `closes_after` gate, and a Layer B fix for flattened `human_verification` names. The upstream behavior was read from `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/audit.cjs` (gsd-core 1.21.0) and executed: `node gsd-tools.cjs query audit-open --json` is the oracle spelling and returns `items.debug_sessions[] {slug,status,updated,hypothesis}` and `items.context_questions[] {phase,file,question_count,questions,archived_milestone?}` plus an `acknowledged` map with the same category keys.

Running the existing debug build and the oracle against read-only copies of the real projects produced three findings that **contradict locked decisions as literally written and would make the roadmap success criteria fail**: (F1) every real HANDOFF on the machine, including usbee's, is judged stale by `ProjectState` (`paused == false`), so D-05's "non-stale only" yields no usbee row and SC1 fails; (F2) two of sentriq's debug files have frontmatter that gsd-core's own YAML parse rejects (a quoted `trigger:` followed by more text), so the faithful Layer A reads `crank-missed-in-standby` (written `awaiting_human_verify`) as `unknown`, giving 4 Decide sessions, not 5; (F3) mailbot's real answered questions are written `- **OQ-06-01 — ANSWERED: …**`, so D-04's "leading token" matcher does not filter them and the "1 fully answered file" case fails. Each has a small, contained Layer B fix described below; the roadmap SCs outrank the inferred details of D-04/D-05.

**Primary recommendation:** Add Layer A ports `debug.rs` and `context.rs` (oracle-pinned via a sibling `tests/owner_open_conformance.rs` running `query audit-open --json`), Layer B rules for all new signals in `rules.rs` (or a new `signals.rs`), with three owner-rule divergences: list stale handoffs as Decide with `stale: true`; recover `status:` by line scan when debug frontmatter is unparseable; match RESOLVED/ANSWERED/CLOSED after an optional question-ID label. Keep all new data inside `OwnerDigest` (no new `ProjectState` field) so Phase 36 has one read path.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Debug session / CONTEXT question parsing (Layer A) | State reader (`owner_items`, process-free) | Test-time oracle (`tests/`) | Faithful port pinned by gsd-core; never spawned from `src/` |
| Lanes, normalization, RESOLVED filter, handoff/checkpoint rows, recovery (Layer B) | State reader (`owner_items::rules`) | — | Owner rules, unit-tested only |
| `closes_after`, `decide`, `fix_debug` counts | State reader (`OwnerDigest`) | — | One digest feeds both CLIs and Phase 36 TUI |
| JSON / plain rendering | CLI layer (`src/inbox.rs`, `src/status_cli.rs`) | README / `agent_skill.md` contract | Additive keys only; contract pinned by tests |
| Todo count and oldest age | State reader (digest) | `oneoff::todos` (shared enumeration) | Keeps the TUI on one read path; IN-05 fail-safe |

## Standard Stack

No new crates. Everything needed is in-tree.

| In-tree component | Location | Use in Phase 35 |
|---|---|---|
| Capped, NOFOLLOW, non-blocking open + 1 MiB read | `scan.rs::open_checked`, `read_text`; `const READ_CAP: u64 = 1 << 20;` [VERIFIED: src/state_reader/owner_items/scan.rs:34] | Every new file read (debug, CONTEXT, HANDOFF.json, `.continue-here.md`) |
| Containment | `scan.rs::contained(root_real, phase_dir, file)` [VERIFIED: scan.rs:106-110] | CONTEXT + checkpoint reads (bounded to phase dir; stricter than upstream's `planDir` bound, harmless) |
| Frontmatter port (fence, FAILSAFE, anchor refusal, one repair retry) | `frontmatter.rs::extract`, `extract_checked`, `failsafe_str`, `fm_str`, `is_acknowledged` [VERIFIED: frontmatter.rs:105-134, 224-236, 307-320] | Debug `status`, CONTEXT `open_questions`, checkpoints, todo `created:`; `extract_checked` distinguishes "unparseable" (needed for F2) |
| Markdown sectionizer | `markdown.rs::collect_section(lines, predicate, level_bounded) -> Option<Section>` [VERIFIED: markdown.rs:251-264] | `## Open Questions` (level-bounded, as upstream) and `## Current Focus` |
| JS string helpers | `uat::js_trim`, `uat::js_space`, `uat::normalize_line_endings` (used in scan.rs:163, verification.rs:16) | `deriveOpenQuestions` line trimming, lone-CR normalization |
| SHA-256 | `sha2::{Digest, Sha256}` (pattern in src/launcher/prompt.rs:22, 93-94: `digest.iter()...format!("{b:02x}")`) | `questions_digest` |
| Parse cache | `ParseCache` keyed `(PathBuf, AuditKind)` → `(CacheKey{mtime,len}, Arc<Parsed>)` [VERIFIED: mod.rs:181-262] | Share for debug + CONTEXT (see Pattern 4) |
| Handoff staleness | `ProjectState.paused`, `stale_handoff: Option<StaleHandoff>`, `const HANDOFF_STALE_GRACE_SECS: i64 = 3600;` [VERIFIED: src/state_reader/mod.rs:145, 165, 649] | `stale` flag on handoff groups (not a filter; see F1) |
| Milestone membership | `ProjectState::in_current_milestone(phase_id)` [VERIFIED: mod.rs:411-417]; `current_phase_dirs` [VERIFIED: scan.rs:180-197] | `closes_after`, checkpoints, CONTEXT Layer B |
| Plans on disk | `DiskInference { status, plan_count, summary_count, has_plans, … }` [VERIFIED: disk_status.rs:493-498] via `ProjectState::disk_status_for` [VERIFIED: mod.rs:281-286] | `not_built` = no entry or `plan_count == 0` |
| Todo enumeration | `list_pending`, `valid_todo_file_name`, `PENDING_REL = ".planning/todos/pending"`, `MAX_TODOS: usize = 200`, `READ_PREFIX: u64 = 8 * 1024` [VERIFIED: src/oneoff/todos.rs:20-28] | Count + `created:` age |
| Escaping | `text::terminal_capped` [VERIFIED: src/text.rs:681] | Every new string |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed by this phase. **Packages removed due to [SLOP] verdict:** none. **Packages flagged as suspicious [SUS]:** none.

## Upstream Behavior (gsd-core 1.21.0, read and executed this session)

Source: `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/audit.cjs` (VERSION file reads `1.21.0`).

### `audit-open` oracle invocation [VERIFIED: executed]
```bash
cd <tmp copy of fixture> && GSD_RUNTIME=claude node "$CLAUDE_CONFIG_DIR/gsd-core/bin/gsd-tools.cjs" query audit-open --json
# per workstream: add GSD_WORKSTREAM=<name>; remove GSD_WORKSTREAM/GSD_PROJECT otherwise
```
`--json` selects the JSON object (router strips it: `const wantJson = args.includes('--json');` [VERIFIED: audit-command-router.cjs:69]); without it a human report is printed. `--raw` is accepted but unnecessary. Output is pretty-printed JSON; keep the existing `@file:` fallback from `owner_audit_conformance.rs:141-147`. Shape [VERIFIED: audit.cjs:1219-1235 and executed output]:
```
{ scanned_at, has_open_items, counts{debug_sessions,…,context_questions,…,total},
  acknowledged{same keys}, items{ debug_sessions:[{slug,status,updated,hypothesis}],
  context_questions:[{phase,file,question_count,questions[≤3],archived_milestone?}], … } }
```
A scan failure yields a `{scan_error: true, …}` sentinel item instead of throwing (audit.cjs:1095-1104, 1162-1169).

### `scanDebugSessions` [VERIFIED: audit.cjs:272-340]
- Directory: `debugDir(cwd)` = `<cwd>/.planning/debug`, **always the root**, even under a workstream: `function debugDirFrom(planningBase) { return node_path_1.default.join(planningBase, 'debug'); }` / `function debugDir(cwd) { return debugDirFrom(planningRoot(cwd)); }` [VERIFIED: planning-workspace.cjs:357-362]. So D-02's "workstream debug dir" branch never applies: root only.
- `readdirSync(debugPath, { withFileTypes: true })`, `if (!entry.isFile()) continue;` — a Dirent from `withFileTypes` is lstat-based, so **symlinks and directories (including `resolved/`) are skipped**. `if (!entry.name.endsWith('.md')) continue;` (case-sensitive). `if (entry.name === DEBUG_KNOWLEDGE_BASE_FILENAME) continue;` with `const DEBUG_KNOWLEDGE_BASE_FILENAME = 'knowledge-base.md';` (line 272).
- `requireSafePath(filePath, path.dirname(debugPath), …)` (root = `.planning`).
- `const content = normalizeLineEndings(rawContent); const fm = extractFrontmatter(content, safeFilePath); const status = (fm.status || 'unknown').toLowerCase();` — **lower-cased, NOT trimmed** (line 317). Executed: `status: "resolved "` → `"resolved "` (open), `status: Awaiting-User` → `"awaiting-user"`, `status: 7` → `"7"`, no frontmatter → `"unknown"`.
- `if (status === 'resolved' || status === 'complete') continue;` then `if (isAuditItemAcknowledged(fm, { snapshotKey: 'status', currentValue: status })) { acknowledged++; continue; }` (lines 318-323). Resolved files are never counted as acknowledged.
- `hypothesis` = first trimmed line of the level-bounded `## Current Focus` section body, first 100 chars (lines 325-330). `updated` = `fm.updated || fm.date || ''`.
- Result order is `readdirSync` order (unsorted): compare as sorted sets.

### `deriveOpenQuestions` + `scanContextQuestions` [VERIFIED: audit.cjs:197-218, 259-262, 764-797, 943-1005]
- Questions: if `fm.open_questions` is a non-empty array → `fm.open_questions.map(q => sanitizeForDisplay(String(q)))`. Else the first level-2 heading whose `h.text.trim().toLowerCase().startsWith('open questions')`, level-bounded; `oqBody = section.body.trim()`; empty or `/^\s*none\s*$/i` → `[]`; else `split('\n')`, `trim()` each, drop empty, `'-'` and `'*'`, keep `/^[-*\d]/.test(l) || l.includes('?')`, each through `sanitizeForDisplay`. A heading like `## Open Questions — all three ANSWERED by research` still matches (starts with "open questions").
- `deriveOpenQuestionsDigest(questions)`: `questions.map((q) => \`${q.length}:${q}\`).join('')` → sha256 hex. `q.length` is the **UTF-16 code-unit count**. Verified: `["- Prüfung ok? 😀"]` → length 16 (15 chars), digest `e84c5070…e15`; using the char count gives a different digest `f8b4bd96…dce`.
- `sanitizeForDisplay` = `sanitizeForPrompt` (strip `[\u200B-\u200F\u2028-\u202F\uFEFF\u00AD]`; `<(\/?)\s*(?:system|assistant|human|user)\s*>`gi → `＜{slash}system-text＞`; `\[(\/?)(SYSTEM|INST)\]`gi → `[{slash}{TAG}-TEXT]`; `<<\/?\s*SYS\s*>>`gi → `«SYS-TEXT»`) then drop whole lines matching `/^\s*(?:assistant|user|system)\s+to=[^:\s]+:[^\n]+$/i` or `/^\s*<\|(?:assistant|user|system)[^|]*\|>\s*$/i` [VERIFIED: security.cjs:478-510]. Applied before the digest, so the port must replicate it for acknowledgment parity.
- Targets: `listAuditPhaseTargets(planDir, cwd)` = **every** real subdirectory of `planDir/phases` (sorted, no milestone filter, no ROADMAP lookup) + every archived phase dir of root and all workstreams (`getAllArchivedPhaseDirs(cwd)`, labels `<ws>/<version>` for workstreams). Executed: a `02-beta` dir absent from ROADMAP was scanned. `planDir` is workstream-scoped under `GSD_WORKSTREAM`.
- Files: `files.filter(f => f.includes('-CONTEXT') && f.endsWith('.md'))` — **no `scopeToPhase`** (unlike UAT/VERIFICATION). `requireSafePath(…, planDir, …)`.
- `if (questions.length === 0) continue;` then acknowledged with `snapshotKey: 'questions_digest'`. Item: `phase` = `^(PHASE_NUMBER_TOKEN_SOURCE)` match of the dir name, else the dir name; `file` = basename; `question_count`; `questions` = first 3, each `slice(0, 200)`; `archived_milestone` when archived.
- `String(q)` on a FAILSAFE value: strings as is; a mapping entry becomes `"[object Object]"`, a nested list joins with `,` [ASSUMED: JS semantics, not executed]. Fixtures should use string entries only.

### `scanTodos` [VERIFIED: audit.cjs:553-626]
Root `.planning/todos/pending/*.md` (`todosDir` is root-scoped, planning-workspace.cjs:349-354); presence-only acknowledgment (`presenceOnly: true`). Executed on a wordoclock copy: 11 pending files, `counts.todos: 2`, `acknowledged.todos: 9`. The current `hidden.todos` (from `list_pending`) reports 11 for wordoclock (live `inbox --json` output) because it ignores the marker.

### HANDOFF.json and `.continue-here.md` (no upstream reader in the audit)
HANDOFF template [VERIFIED: $CLAUDE_CONFIG_DIR/gsd-core/workflows/pause-work.md:99-107]: `"blockers": [{"description": …, "type": "technical|human_action|external", "workaround": …}]`, `"human_actions_pending": [{"action": …, "context": …, "blocking": true}]`, plus `status`, `next_action`, `timestamp`, `phase`. Real files: usbee `status: paused`, 6 `human_actions_pending` objects `{action, blocking, context}`, `timestamp 2026-05-14T09:04:46.076Z`; ttbook `status: ready`, 9 actions; cdr `status: paused`, 0 actions; sentriq workstream `v0-13-public-beta` HANDOFF.json has no `status`/`next_action`/`timestamp` (keys `paused_at, workstream, milestone, phases, half_done_tasks, resume_file, resume_block`).
Phase checkpoints (cdr, 3 files) frontmatter: `context: phase`, `phase: 14-3d-engine-label-tweaks`, `task`, `total_tasks`, `status` (`research_complete`, `planning_complete_ready_for_execution`, `planning_incomplete`), `last_updated` (RFC 3339). cdr phases 14 and 15 are `[x]` complete in its ROADMAP; 18.1 is not.

## Findings That Change the Implementation of Locked Decisions

These are measured on the owner's machine (read-only; oracle runs were on copies under the session scratchpad). Each one would make a roadmap success criterion fail if D-04/D-05 were implemented literally. Roadmap SCs are the higher authority; the fixes are owner rules (Layer B) and do not touch Layer A parity.

### F1 — Every real HANDOFF is "stale"; D-05 as written drops usbee (SC1 fails)
- Evidence: `gsd-meta-manager status --json` (worktree debug build) on cdr-configurator, ttbook, sentriq and usbee emits **no** `paused` attention item for any scope [VERIFIED: executed]; `status_cli.rs:143-144` emits `paused` exactly when `scope.paused`. For usbee: HANDOFF `timestamp` 2026-05-14T09:04:46Z, STATE.md `last_updated: "2026-09-17T20:03:55.383Z"` → `StaleHandoffReason::StateNewer` (STATE newer than handoff + 3600 s, mod.rs:785-788). Phases equal (`04` vs `04`), so not `PhaseBehind`.
- Report intent: §3.3 mockup row `usbee handoff paused · 1 human action pending 26 w stale` sits in DECIDE and is counted in `decide 9`; SC1 requires usbee's handoff in `--lane decide`.
- **Recommendation [INFERRED]:** the owner reader reads HANDOFF.json / HANDOFF.md natively per scope (root + each workstream tree, like `ProjectState.scopes()`), and every non-empty handoff yields one Decide `handoff` group. `group.stale = state.stale_handoff.is_some() || age_days > stale_days`. It counts 1 in `decide` (D-11). `paused`/`needs_human`/`❢` stay untouched (D-09, report D1). Row text from HANDOFF `status` (`handoff paused`, `handoff ready`, `handoff` when absent). Command `/gsd-resume-work` (+ ` --ws <name>` for a workstream handoff; `/gsd-resume-work` parses `--ws` [VERIFIED: gsd-core/workflows/resume-project.md:30-31]).

### F2 — gsd-core reads two sentriq sessions as `unknown` (SC1 would show 4, not 5)
- Evidence: oracle on a copy of sentriq: `debug crank-missed-in-standby unknown`, `debug f350-profile-on-ram-gauges unknown`, plus 4 `awaiting_human_verify` [VERIFIED: executed]. Their frontmatter is `status: awaiting_human_verify` / `status: resolved` followed by `trigger: "…" (owner, field run …)` — a quoted scalar followed by more text, which js-yaml rejects; gsd-core's `extractFrontmatter` returned `{}` for both [VERIFIED: executed via `frontmatter.cjs`]. The Rust port behaves the same: `repair_frontmatter_block` skips values that open with a quote [VERIFIED: src/state_reader/state_md.rs:533-535 doc], so `extract_checked` returns `Err(Unparseable)` → status `unknown`. Raw files on the machine: 11 `awaiting_human_verify`, 1 `awaiting-user`, 1 `diagnosed`, 1 `root_cause_found`, 5 `resolved` (top-level `status:` lines across `~/projects/*/*/.planning/debug/*.md`).
- Faithful Layer A + D-03 gives sentriq Decide 4 (baseline, faults, polling, towing) and Fix 2 (crank, f350 — a resolved session).
- **Recommendation [INFERRED] — Layer B status recovery:** when Layer A recorded the frontmatter as unparseable (closed fence, YAML rejected), scan the fence body for a column-0 line `status:[ \t]*(.*)`, strip one pair of surrounding quotes, and use that as the status for Layer B only. Recovered `resolved`/`complete` (after D-03 normalization) → not listed, not counted. Layer A keeps `unknown` (oracle parity; the oracle test compares Layer A only). Result on sentriq: Decide 5, Fix 0. Unit-test both directions and the "no status line" case (stays `unknown` → Fix).

### F3 — Real answered questions put the marker after an ID label (D-04 matcher misses them)
- Evidence: oracle on a copy of mailbot: `06-CONTEXT.md question_count 3`, all lines like `- **OQ-06-01 — ANSWERED: yes, for free.** …`; `03-CONTEXT.md` 14 lines incl. `- **OQ-02 — RESOLVED by H-02 (2026-08-27).** …` and the intro line `**Six of the original nine were answered by the human …**` (counted by upstream because it starts with `*`); `04-CONTEXT.md` 8 incl. `- **OQ-04-01 — \`[OPEN]\`, HUMAN, non-blocking.**` [VERIFIED: executed]. D-04's "leading token after bullet/number and `**`/`~~`/`[x]`" sees `OQ-06-01`, not `ANSWERED`, so the "one fully answered file" (mailbot 06) would still show 3 questions.
- **Recommendation [INFERRED: refines D-04's exact matcher]:** after stripping the bullet (`-`, `*`, `N.`, `N)`), emphasis openers (`**`, `__`, `~~`, `*`, `_`), a checkbox `[x]`/`[ ]`, strip an optional question-ID label `^[A-Za-z]{1,6}[-.\d]*[A-Za-z\d]*\s*[—–:-]+\s*` (matches `OQ-06-01 — `, `Q3: `, `OQ-03.1-02 — `), strip emphasis again, then the line is answered when the first word is `RESOLVED|ANSWERED|CLOSED` (case-insensitive, whole word) **or** the line contains `(RESOLVED…)`/`[ANSWERED]`-style bracketed tag with the word first inside the bracket (case-insensitive). Do not match bare lowercase prose ("were answered") — the mailbot 03 intro line must stay a question (upstream counts it; our layer only removes marked answers). Expected on mailbot: 06 → 0 (no group), 03 → 14 − 5 RESOLVED = 9, 04 → 8. Table-test every real line shape above with neutral text.

### F4 — Other measured facts the plan must honor
- Upstream scans **all** active phase dirs for CONTEXT (no milestone filter), while Phase 34's `current_phase_dirs` filters by milestone. A faithful Layer A must enumerate all real subdirectories of `<tree>/phases`; Layer B keeps current-milestone dirs (Decide) and archived dirs (Old debt) and drops active dirs outside the current milestone [INFERRED] (they belong to no milestone the owner is closing).
- `hidden.todos` is documented `integer` [VERIFIED: README.md:691 `| \`projects[].hidden.todos\` | integer | pending todos, listed or not |`]. D-08's "failed listing reports `null`" changes the type, which README.md:703-705 says bumps `schema_version`. **Recommendation:** keep `hidden.todos` integer (0 on failure) and add additive `hidden.todos_unreadable: bool` (and `todos_oldest_days: null`) [INFERRED: overrides D-08's null].
- `list_pending` stops at `MAX_TODOS` (200); counts above 200 are capped (sentriq has 127, fine). It also ignores `audit_acknowledged` (wordoclock 11 vs upstream 2). D-08 keeps the Phase 34 meaning; flag in Open Questions.
- Item ids for VERIFICATION human entries are `<basename>#<index+1>` from `human_needed(Some((index + 1).to_string()), …)` [VERIFIED: verification.rs:178-180] and `Ids::next` uses `item.test`, never `item.name` [VERIFIED: rules.rs:150-175]. D-14 changes no id.
- The `audit-uat` oracle's item JSON carries `name`: executed on a mapping entry → `"name": "test: Weekly summary live send (PRICE-14), expected: Mail arrives, why_human: Needs a real inbox"`, a scalar entry → its text, a mapping without `test` → `"truth: no test key here, expected: x"` [VERIFIED: executed]. The existing conformance tuple is `(file, test, result, category)` [VERIFIED: tests/owner_audit_conformance.rs:185-208], so names are **not** compared today; D-16(a) needs an explicit name comparison for the new fixture.

## Architecture Patterns

### System Architecture Diagram

```
 project root ──► parse_project_state ──► ProjectState (scopes, milestones, phases, disk statuses, stale_handoff)
      │                                              │
      └──► read_owner_digest(planning, state, now, stale_days)
              │
              ├─ Layer A (process-free ports, cached by (path,mtime,len))
              │    ├─ scan_planning: UAT / VERIFICATION / deferred  (Phase 34, current-milestone dirs + archives)
              │    ├─ scan_debug: .planning/debug/*.md (root only)            ── oracle: audit-open debug_sessions
              │    └─ scan_context: */phases/*/*-CONTEXT.md (ALL active + archives) ── oracle: audit-open context_questions
              │
              ├─ Native readers (no oracle): HANDOFF.json/.md per scope, phases/<p>/.continue-here.md, todo stats
              │
              └─ Layer B rules
                   ├─ R1..R10 (Phase 34) + D-07 empty gaps_found → Fix unreadable=1 + D-14 clean names
                   ├─ debug: recover status (F2) → normalize → awaiting* ⇒ Decide | other open ⇒ Fix (fix_debug)
                   ├─ context: RESOLVED/ANSWERED/CLOSED filter (F3) → Decide (current) | Old debt (archived)
                   ├─ handoff (F1, stale flag) / checkpoint (non-terminal, phase not complete) ⇒ Decide
                   └─ counts: test, decide, fix_phases, fix_debug, blocked, closeable, closes_after(root)
                                   │
                 ┌─────────────────┴─────────────────┐
           inbox.rs build_document / render_text   status_cli.rs owner_json
           (schema v1 additive keys, goldens)      (owner.decide real, fix_debug, closes_after)
```

### Recommended module layout (Claude's discretion; recommended)
```
src/state_reader/owner_items/
├── mod.rs          # + Kind variants, OwnerDigest fields (decide, fix_debug, closes_after, todos), Hidden fields
├── debug.rs        # NEW Layer A: scanDebugSessions port (DebugSession), + recover_status (Layer B helper)
├── context.rs      # NEW Layer A: deriveOpenQuestions, digest, sanitize_for_display port, scanContextQuestions targets
├── handoff.rs      # NEW native: HANDOFF.json/.md per scope, .continue-here.md per phase
├── todos.rs        # NEW: count + oldest age, sharing enumeration with oneoff::todos
├── gate.rs         # NEW Layer B: closes_after
├── rules.rs        # + Layer B lanes for the new kinds, D-07, D-14 owner_item names, counts
└── verification.rs # + structured HumanEntry fields on AuditItem (Layer A, not compared)
```

### Pattern 1: Layer A port, Layer B divergence, both visible in types
**What:** Layer A structs record exactly what upstream computes (`status` raw lower-cased, `questions` unfiltered, `acknowledged`), plus extra facts Layer B needs (`frontmatter_unparseable: bool`, `in_current_milestone: bool`, structured fields). Layer B never mutates Layer A values.
**Example (shape, names are recommendations):**
```rust
pub struct DebugSession {
    pub rel_path: String,            // ".planning/debug/<slug>.md"
    pub slug: String,
    pub status: String,              // (fm.status || "unknown").toLowerCase(), untrimmed — oracle parity
    pub fm_unparseable: bool,        // extract_checked == Err(Unparseable)
    pub recovered_status: Option<String>, // Layer B input (F2), never compared
    pub trigger: Option<String>,     // owner-facing name
    pub hypothesis: String,          // "## Current Focus" first line, 100 chars
    pub updated: Option<DateTime<Utc>>, pub modified: Option<DateTime<Utc>>,
    pub acknowledged: bool, pub unreadable: Option<Unreadable>,
}
pub struct ContextFile {
    pub rel_path: String, pub scope: Scope, pub workstream: Option<String>,
    pub phase: String, pub phase_dir: String, pub in_current_milestone: bool,
    pub questions: Vec<String>,      // full, unclamped, sanitized (digest input)
    pub acknowledged: bool, pub unreadable: Option<Unreadable>, pub updated/modified …
}
```

### Pattern 2: Digest port (UTF-16 length prefix)
```rust
// Source: audit.cjs:259-262 deriveOpenQuestionsDigest
use sha2::{Digest, Sha256};
pub(super) fn questions_digest(questions: &[String]) -> String {
    let mut hasher = Sha256::new();
    for q in questions {
        hasher.update(q.encode_utf16().count().to_string().as_bytes()); // JS q.length
        hasher.update(b":");
        hasher.update(q.as_bytes()); // node hashes the UTF-8 encoding
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
// acknowledged = frontmatter::is_acknowledged(&fm, "questions_digest", &questions_digest(&qs))
// Pin: ["- Is Y ready?"] -> f9473be7ec4213c58042ed3c06d0725903da6ca8ed79463b5edb6074dfb4775d [VERIFIED: node]
```

### Pattern 3: debug Layer A order (matches upstream acknowledged counts)
```rust
// per entry of read_dir(<planning>/debug), sorted by name for determinism:
// skip unless lstat file_type().is_file()   (symlinks + resolved/ skipped, like Dirent.isFile())
// skip unless name.ends_with(".md"); skip name == "knowledge-base.md"
// open_checked (NOFOLLOW, cap) -> read_text (normalizes line endings)
// fm = extract_checked(text); status = fm_str("status").filter(non-empty).unwrap_or("unknown").to_lowercase()
//   NOTE: no trim (upstream line 317); FAILSAFE number 7 -> "7"
// if status == "resolved" || status == "complete" { continue }          // not counted anywhere
// if is_acknowledged(&fm, "status", &status) { acknowledged; still build, flagged }
```
Unreadable file (too large, FIFO, I/O): upstream `platformReadSync` returns null and **skips** it; the fail-safe rule (34 D-17) says "unreadable, never clean". Recommend Layer A records `unreadable` (excluded from the oracle comparison, as Phase 34's `our_tuples` excludes unreadable files) and Layer B lists it as a Fix debug group with `counts.unreadable = 1` [INFERRED].

### Pattern 4: share the parse cache by making it generic
`ParseCache` stores `Arc<Parsed>` where `Parsed = Option<ParsedFile>` and keys by `(PathBuf, AuditKind)`. Recommend `ParseCache<V>` (generic value) with two statics (`PARSE_CACHE`, `OPEN_CACHE: LazyLock<ParseCache<OpenParsed>>`) and `evict_project` evicting both, so the watcher eviction (Phase 36) covers the new files. HANDOFF, checkpoints and todos are few small files; no cache needed.

### Pattern 5: groups for root-scoped signals
Group keys are fixed by schema v1 (`phase` is a string). Recommend: debug group `phase: ""`, `phase_name: <slug>`, `file: ".planning/debug/<slug>.md"`, `file_status: <raw status>`, one item `{id: "<slug>.md#1", name: trigger | hypothesis | slug, result: <normalized status>, category: "unknown"}`; handoff group `phase: <HANDOFF phase if it passes is_command_token, else "">`, `file: ".planning/HANDOFF.json"` (or workstream path), items = actions (`#a<k>`) then blockers (`#b<k>`), name = `action` / `description`, reason = `context` / `workaround`; checkpoint group per phase, one item `#1`, name = the frontmatter `status`; context group per file, items `#q<k>`. `category: "unknown"` for all non-UAT items keeps Layer A's `Category` the upstream token set [INFERRED]. The `fix_phases` loop in `build_digest` (rules.rs:606-620) inserts `(workstream, phase_key(phase))` for every current Fix group — exclude `Kind::Debug` there and count it into `fix_debug`, or `""` becomes a phantom phase.

### Pattern 6: `closes_after` (root scope; per-scope available in the digest)
```text
for phase in state.phases where state.in_current_milestone(&phase.number) && !phase.completed:
    disk = state.disk_status_for(&phase.number)
    if disk.map_or(true, |d| d.plan_count == 0): not_built.push(phase.number) ; continue
    phases_open.push(phase.number)
    over current, unacknowledged, non-root-scoped groups of this phase (workstream None, same_phase_token):
        test    += items of Test groups
        blocked += items of Blocked groups
        fix     += items + counts.unreadable of Fix groups   [INFERRED: unreadable counts as fix work]
        decide  += items of Checkpoint + ContextQuestions groups
sort phases_open / not_built by PhaseNum
```
wordoclock live: Test groups 21 (2), 22 (2), 24 (6), 25 (2) = 12; Blocked 20 (1), 23 (4) = 5; Fix 25 (1) = 1; no CONTEXT questions (oracle `context_questions: 0`) → decide 0; phases 20-25 have plans, 19 is `[x]` [VERIFIED: live inbox --json + ROADMAP read].

### Anti-Patterns to Avoid
- **Filtering stale handoffs out** (F1): yields nothing on the owner's machine.
- **Trimming the debug status in Layer A**: breaks parity for `"resolved "`-style values (executed: upstream keeps the space).
- **Hashing with `chars().count()`**: wrong digest for non-BMP characters; acknowledgment never matches.
- **Using `current_phase_dirs` for CONTEXT in Layer A**: diverges from upstream on any active dir outside the current milestone.
- **Changing `hidden.todos` to nullable**: schema type change.
- **Adding fields to `ProjectState`**: `tests/spawn_seam_guard.rs` censuses its free `String` fields; keep new data in `OwnerDigest`.
- **Parsing `updated:` dates only as RFC 3339 for the new signals**: debug files and some todos use `2026-11-11` / `"2026-…Z"` / `2026-…-05:00`; fixtures would then age by checkout mtime (nondeterministic goldens). Use a helper that accepts RFC 3339 and `%Y-%m-%d` (midnight UTC) for the new signals; leave Phase 34's `frontmatter::updated` unchanged.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Safe file reads | new `read_to_string` calls | `scan::open_checked` + `read_text` | NOFOLLOW, FIFO, 1 MiB cap, lone-CR normalization already proven |
| YAML frontmatter | ad-hoc `status:` parsing for normal files | `frontmatter::extract_checked` (+ the F2 line scan only on `Err`) | FAILSAFE semantics, anchor-bomb refusal, upstream fence rules |
| Section extraction | regex over `## Open Questions` | `markdown::collect_section(lines, pred, true)` | Fence-aware heading tokenizer already ported |
| Acknowledgment | a new marker check | `frontmatter::is_acknowledged(fm, key, current)` | Exact port of `isAuditItemAcknowledged` |
| SHA-256 hex | external `hex` crate | `sha2` + `format!("{b:02x}")` (src/launcher/prompt.rs:93-94 pattern) | No new dependency |
| Phase number compare/sort | string compare | `phase_num::{same_phase, phase_key, PhaseNum}` | `07.1` vs `7.1`, decimals, `M-NN` |
| Command strings | `format!` from raw dir names | `rules::is_command_token` / `is_workstream_name` guards (rules.rs:69-97) | Hostile names never reach a command line |
| Escaping | manual control-char stripping | `text::terminal_capped` | Shared JSON/plain pipeline, pinned by hostile tests |

## Runtime State Inventory

Not a rename/refactor/migration phase. Omitted.

## Common Pitfalls

### Pitfall 1: "Existing key paths stay unchanged" read as "no test edits"
**What goes wrong:** `tests/inbox_cli.rs` asserts **exact** key sets (`keys(&doc["summary"]) == set([...])` lines 162-177, `hidden_keys` line 192, `item_keys` 221-231, error `hidden == json!({...})` 248-251, `key_paths(&doc) == inbox_set()` 729, README fence 746-749, README table one-row-per-path 752-763). `tests/status_cli.rs` pins `SCHEMA_PATHS` (167-218) against `src/agent_skill.md`'s fence (874), README's fence (879) and table (887), and `owner_counts_cover_the_current_milestone` compares the whole `owner` object (583-594).
**How to avoid:** append-only edits: every existing string in `INBOX_PATHS`, `SCHEMA_PATHS`, the `set([...])` lists stays; new paths are added; README fence/table and `agent_skill.md` fence gain the new keys. A plan verification step: `git diff -U0 tests/inbox_cli.rs tests/status_cli.rs | grep '^-' | grep -v '^---'` shows no removed path string.

### Pitfall 2: `the_inbox_key_paths_are_exactly_schema_v1` needs every new path filled
That test (inbox_cli.rs:700-730) builds a document from `uat-gaps-merge` + `supersede-six-phases` and requires `key_paths(doc) == INBOX_PATHS`. Object keys present on every entry (`milestone.closes_after.*`, `hidden.todos_oldest_days`, `items[].why_human`) are filled automatically; any new array-of-objects path must be non-empty in those fixtures.

### Pitfall 3: New fixtures run under the existing `audit-uat` oracle too
`discover()` (owner_audit_conformance.rs:249-258) runs every dir under `tests/fixtures/owner/` with `.planning/` and requires `oracle.expect` ∈ {`same`, `no-phases-dir`, `out-of-set-verification`}. A debug-only fixture without `phases/` must declare `no-phases-dir` (and then yields no Layer A UAT files); a fixture with a `phases/` dir and ROADMAP declares `same` (zero items on both sides still counts as compared).

### Pitfall 4: Goldens shift
`tests/fixtures/owner_golden/inbox.txt` and `inbox-include-all.txt` are rendered from `supersede-six-phases`, `uat-gaps-merge`, `archived-complete-skipped`, `deferred-200`, `uat-ack-match`, `workstreams` (owner_digest.rs:303-315). The header `decide 0` stays unless those fixtures gain Decide signals; the new per-project `closes after:` line changes them. Regenerate with `UPDATE_GOLDEN=1` and review the diff: only added lines may appear.

### Pitfall 5: Root-scoped groups and the plain renderer
`group_line` prints `ph {phase}` and `detail()` falls to `plural(checks, "check", "checks")` (inbox.rs:256-323). Debug/handoff groups need their own label (`debug <slug>`, `handoff`) and wording ("awaiting your confirmation", "open investigation", "handoff paused · N human actions pending", "N open questions in NN-CONTEXT.md", "checkpoint: <status>", "verification gaps_found, no gaps listed").

### Pitfall 6: Workstream oracle runs for CONTEXT
`audit-open` reports `file` as a basename and `phase` as a token, no path [VERIFIED: audit.cjs:993-1000]. Compare per run: root run → root active + all archives; each `GSD_WORKSTREAM` run → that workstream's active dirs (+ the same archives again, deduplicate by `(archived_milestone, phase, file)`). Debug is root-only: compare from the root run only.

### Pitfall 7: Windows cross-clippy
New code using `std::os::unix` must be `#[cfg(unix)]` with no unused items on Windows (`open_checked` already shows the pattern, scan.rs:123-134).

### Pitfall 8: `now` taken per project
`inbox_cli::read_entry` calls `Utc::now()` per project (34-REVIEW IN-05). Take it once in `run` and pass it down so ages in one document agree.

## Code Examples

### `deriveOpenQuestions` port (body branch)
```rust
// Source: audit.cjs:197-218 (behavior verified by executing audit-open on fixtures)
fn body_questions(lines: &[&str]) -> Vec<String> {
    let Some(section) = markdown::collect_section(
        lines,
        |h| h.level == 2 && js_trim(&h.text).to_lowercase().starts_with("open questions"),
        true,
    ) else { return Vec::new() };
    let body = js_trim(&section.body);
    if body.is_empty() || is_none_body(body) { return Vec::new(); } // /^\s*none\s*$/i
    body.split('\n')
        .map(js_trim)
        .filter(|l| !l.is_empty() && *l != "-" && *l != "*")
        .filter(|l| l.starts_with(['-', '*']) || l.starts_with(|c: char| c.is_ascii_digit()) || l.contains('?'))
        .map(sanitize_for_display)
        .collect()
}
```

### Status recovery for unparseable debug frontmatter (Layer B, F2)
```rust
// Only when extract_checked(text) == Err(Unparseable): read the closed fence body line by line.
fn recover_status(fence_body: &str) -> Option<String> {
    fence_body.lines().find_map(|line| {
        let rest = line.strip_prefix("status:")?;          // column 0 only
        let value = rest.trim();
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        (!value.is_empty()).then(|| value.to_owned())
    })
}
fn normalize_debug_status(raw: &str) -> String { raw.trim().to_lowercase().replace('-', "_") } // D-03
```

### D-14 structured human_verification fields
```rust
// verification.rs human_verification_items: keep name = normalize_entry(display) (oracle parity),
// and attach the mapping's fields for Layer B.
pub struct EntryFields { pub test: Option<String>, pub truth: Option<String>, pub name: Option<String>,
                         pub description: Option<String>, pub expected: Option<String>, pub why_human: Option<String> }
// AuditItem gains `fields: Option<EntryFields>` (None for scalar entries and body rows).
// rules::owner_item: name = f.test.or(f.truth).or(f.name).or(f.description).unwrap_or(item.name)
//                    expected = item.expected.or(f.expected); why_human = f.why_human
// Use verification.rs::entry_field (blank -> None, numbers/bools as text) for each key.
```

## State of the Art

| Old Approach (Phase 34) | Current Approach (Phase 35) | Impact |
|---|---|---|
| `summary.decide` / `owner.decide` hard-coded `0` (inbox.rs:193, status_cli.rs:215) | real Decide item count | README rows "0 until the decision signals land" (README.md:505, 632) must be reworded |
| plain header literal `decide 0` (inbox.rs:373) | computed | goldens unchanged while no Decide signal is in golden fixtures |
| `human_verification` mapping shown flattened | clean `name`, `expected`, `why_human` | value change only; `why_human` new key |
| `hidden.todos` from `list_pending().unwrap_or_default()` | same count + `todos_oldest_days` + `todos_unreadable` | IN-05 closed without a type change |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Stale handoffs belong in Decide (counted, `stale: true`) despite D-05's "not owner work" | F1 | If the owner wants them hidden, SC1 cannot be met for usbee; planner must escalate instead |
| A2 | Layer B may recover `status:` from unparseable debug frontmatter | F2 | Without it sentriq shows 4 Decide + 2 Fix; SC1 fails |
| A3 | Answer-marker matcher that skips an ID label (`OQ-06-01 — ANSWERED`) | F3 | Too loose → hides real questions; too strict → mailbot 06 still listed |
| A4 | Active CONTEXT dirs outside the current milestone are dropped by Layer B | F4 | Owner might want them as Old debt |
| A5 | `hidden.todos` stays integer; new `hidden.todos_unreadable` instead of null | F4 | Contradicts D-08 wording (inferred there) |
| A6 | Non-UAT items use `category: "unknown"` and carry meaning in `result` | Pattern 5 | Consumers filtering on category see "unknown" |
| A7 | Fix-lane `closes_after.fix` counts items plus unreadable, gaps merged into their check are not extra | Pattern 6 | mailbot-like phases report 10, not 24 |
| A8 | Checkpoint terminal statuses = `complete`, `completed`, `resolved`, `done`; checkpoints in phases marked complete in ROADMAP are not listed | D-06 | cdr phases 14/15 would otherwise show two leftover checkpoints |
| A9 | Unreadable debug files become a Fix group with `counts.unreadable = 1` (upstream skips them) | Pattern 3 | Divergence is Layer B only; oracle excludes unreadable files |
| A10 | `String(q)` of non-string `open_questions` entries (`[object Object]`) | Upstream | Only exotic fixtures; avoid them |
| A11 | `status --json` `owner` also gains `closes_after` (SC4 "gate data", REQUIREMENTS line 131 "extended (new lanes and `closes_after`)") | Validation | If not wanted, drop one additive key |

## Open Questions

1. **Todo acknowledgment.** Upstream ignores todos carrying `audit_acknowledged` (wordoclock: 9 of 11). D-08 keeps `hidden.todos` = `list_pending` count. Recommendation: keep the count as is (locked) and add `hidden.todos_acknowledged` only if cheap; otherwise defer to Phase 36's "Todos 11 (oldest 96 d)" fold.
2. **Workstream `closes_after`.** Recommendation: compute per scope in `OwnerDigest` (`Vec<(Option<String>, ClosesAfter)>`), emit only the root in JSON now (Phase 36 decides display).
3. **Archived Decide count.** `summary.archived` has `test/blocked/fix`. Recommendation: add `archived.decide` (additive) so the Old-debt fold total includes archived CONTEXT questions.
4. **`/gsd-debug` argument.** `gsd-debug` accepts `continue <slug>` [VERIFIED: skills/gsd-debug/SKILL.md:4 `argument-hint: "[list | status <slug> | continue <slug> | --diagnose] [issue description]"`]. D-03 locks `/gsd-debug`; a `continue <slug>` form (slug through `is_workstream_name`-style alphabet) would be more actionable — leave as-is unless the planner opts in.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node.js | oracle tests | ✓ | v24.21.0 | `GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1` (fails by default) |
| gsd-core gsd-tools.cjs | oracle tests | ✓ | 1.21.0 at `$CLAUDE_CONFIG_DIR/gsd-core/bin/gsd-tools.cjs` | `scripts/install-conformance-oracle.sh` |
| cargo / rustc | build, tests | ✓ | cargo 1.99.1 | — |
| Built binary for live checks | SC1/SC2 live check | ✓ | `target/debug/gsd-meta-manager` (2.4.0 build in worktree) | `cargo build` |
| Real projects (read-only) | live sanity | ✓ | sentriq `/home/blk/projects/flutter/sentriq`, usbee `/home/blk/projects/rust/usbee`, wordoclock `/home/blk/projects/flutter/wordoclock`, mailbot `/home/blk/projects/python/mailbot`, cdr `/home/blk/projects/cdr/3d-configurator/cdr-configurator`, ttbook `/home/blk/projects/python/ttbook` | — |

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; this section is included because the phase brief asks for it.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `cargo test` (integration binaries under `tests/`, unit tests in modules); `tempfile` for trees |
| Config file | none (Cargo.toml) |
| Quick run command | `rtk bypass cargo test --lib owner_items` and `rtk bypass cargo test --test owner_digest` |
| Oracle run | `rtk bypass cargo test --test owner_audit_conformance --test owner_open_conformance -- --nocapture` |
| CLI contract | `rtk bypass cargo test --test inbox_cli --test status_cli` |
| Full suite command | `rtk bypass cargo test --locked --no-fail-fast` then `cargo clippy --locked --all-targets -- -D warnings` (+ `--target x86_64-pc-windows-gnu`, `--target aarch64-apple-darwin` if installed) |

### Fixture per signal-table row (OWN-10) — all synthetic, neutral text, under `tests/fixtures/owner/`
Each fixture has `.planning/ROADMAP.md`, `STATE.md`, a `phases/` dir (so `oracle.expect` = `same`), fixed `updated:`/`created:`/`last_updated` values (no mtime dependence), and an `audit-open.expect` marker (`same`) when it carries debug or CONTEXT files.

| # | Signal row | Fixture (new unless noted) | Layer A oracle | Layer B assertion (test) |
|---|---|---|---|---|
| 1 | Paused handoff | `handoff-paused`: HANDOFF.json `status: paused`, `next_action`, `timestamp` older than STATE `last_updated` (stale by StateNewer, like usbee) | none | one Decide `handoff` group, `stale: true`, command `/gsd-resume-work`, decide +1 (`owner_digest.rs`) |
| 2 | Human actions pending | same file: 1 `human_actions_pending` + 1 `blockers`; plus workstream HANDOFF without `status` | none | items `#a1` (`action`), `#b1` (`description`); non-paused HANDOFF with actions still yields group; ws group carries `--ws` |
| 3 | Phase checkpoint | `checkpoint-phase`: `phases/02-x/.continue-here.md` `status: planning_incomplete` (phase open), `phases/01-y/.continue-here.md` in a ROADMAP-complete phase, one with `status: complete` | none | exactly one checkpoint group (ph 02); root `.continue-here.md` not double-counted |
| 4 | Debug awaiting | `debug-sessions`: `awaiting_human_verify`, `Awaiting-User`, `AWAITING_HUMAN_VERIFY`, one unparseable-frontmatter file whose `status:` line says awaiting (F2) | `audit-open` debug: (slug, status) set + `acknowledged.debug_sessions` | 4 Decide groups, row text "awaiting your confirmation", command `/gsd-debug` |
| 5 | Open investigation | same fixture: `root_cause_found`, `diagnosed`, no-frontmatter (`unknown`), `status: 7`, `"resolved "` | same | Fix groups, `fix_debug` count, `fix_phases` unchanged |
| 6 | Ignored debug files | same fixture: `knowledge-base.md`, `resolved/x.md` (awaiting), a symlink `.md`, `notes.txt`, an acknowledged `investigating` session, an unparseable file whose status line says `resolved` | same (acknowledged = 1) | none of them listed; recovered `resolved` not listed; acknowledged listed only with `--include acknowledged` |
| 7 | Verification gaps | existing `verification-gaps-found` (with gaps) + new `verification-gaps-empty` (`status: gaps_found`, no gaps, no human entries) | `audit-uat` (existing) | empty one → Fix group `counts.unreadable == 1`, command `/gsd-plan-phase N --gaps`, phase not closeable |
| 8 | Open questions | `context-questions`: 01-CONTEXT with `- **Q-01 — RESOLVED by …**`, `- **Q-02 — ANSWERED: …**`, `1. … (CLOSED)`, `- open one?`, `**Intro line answered in prose**`; 02-CONTEXT fully answered; 03-CONTEXT `open_questions:` frontmatter list; 04-CONTEXT acknowledged with a matching `questions_digest` (computed with node at authoring time; include one non-BMP character); an archived `milestones/v1.0-phases/…-CONTEXT.md`; an active dir outside the current milestone; a workstream CONTEXT | `audit-open` context: (run, archived_milestone, phase, file, question_count) + `acknowledged.context_questions` | 01 keeps 2 items (open one + intro line); 02 yields no group; archived → Old debt (`--include archived`); out-of-milestone active dir not listed |
| 9 | Pending todos | `todos-age`: `created: 2026-07-27T07:28:24.815Z`, `created: "…Z"`, `created: 2026-07-27`, one without `created` (mtime set by the test via `filetime`-free `File::set_modified`) | none | `hidden.todos == 4`, `hidden.todos_oldest_days` exact at fixed `now`; unreadable pending dir → `todos: 0`, `todos_unreadable: true` |
| 10 | Parked/external | existing `needs_human` tests | none | unchanged (no inbox group) |
| G | `closes_after` (OWN-08) | `closes-after-milestone`: copy of `supersede-six-phases` + `## Milestones` (v2.1 shipped phase 19 `[x]`, v2.2 current 20-26, v2.3 planned 27), STATE `milestone: v2.2`, phase 26 dir with only CONTEXT (no plans), phase 27 listed in ROADMAP only | `audit-uat` same as supersede | `{test:12, decide:0, fix:1, blocked:5, phases_open:["20".."25"], not_built:["26"]}`; 19 and 27 absent |
| SC1 | Decide lane | `decide-lane`: root debug with 5 awaiting (one via F2 recovery) + 1 resolved via recovery, `workstreams/a/` present; HANDOFF stale + 1 human action | `audit-open` debug | `inbox --json --lane decide` (CLI test) lists exactly 5 debug groups + 1 handoff group, `summary.decide == 6`; no workstream duplicates |
| D-16 | Human mapping names | `verification-human-mapping`: mapping with `test/expected/why_human`, scalar, mapping without `test` | `audit-uat` tuples + **name equality** for this fixture | JSON `name == "Weekly …"` (neutral text), no `expected:`/`why_human:` substring, `expected`/`why_human` separate; plain `- [ ]` line = test text |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OWN-10 | Layer A debug/CONTEXT = upstream | oracle | `rtk bypass cargo test --test owner_open_conformance -- --nocapture` | ❌ Wave 0 (new file) |
| OWN-10 | Layer B lanes, normalization, recovery, RESOLVED filter | unit | `rtk bypass cargo test --lib owner_items::` | ✅ module tests exist; new cases ❌ |
| OWN-10 | One fixture per row | integration | `rtk bypass cargo test --test owner_digest` | ✅ file; new tests ❌ |
| OWN-08 | `closes_after` totals and lists | integration | `rtk bypass cargo test --test owner_digest closes_after` | ❌ |
| OWN-13 ext | additive inbox keys, README pins, goldens | integration | `rtk bypass cargo test --test inbox_cli --test owner_digest` | ✅ (append-only edits) |
| OWN-15 ext | additive `owner` keys, skill + README pins | integration | `rtk bypass cargo test --test status_cli` | ✅ (append-only edits) |
| D-16 | names, oracle name parity | oracle + integration | `rtk bypass cargo test --test owner_audit_conformance --test inbox_cli human_mapping` | ❌ |
| SC1/SC2 live | real projects, read-only | manual-automatable | `target/debug/gsd-meta-manager inbox --json --lane decide /home/blk/projects/flutter/sentriq /home/blk/projects/rust/usbee \| jq '[.projects[] \| {alias: .path, g: [.groups[] \| {kind, phase_name, stale}]}]'` and `target/debug/gsd-meta-manager inbox --json /home/blk/projects/flutter/wordoclock \| jq '.projects[0].milestone.closes_after'` | n/a |

Expected live results after the phase: sentriq Decide includes 5 `debug` groups (baseline-purge-reruns-on-reprof, crank-missed-in-standby, faults-disconnected-live-dash, polling-rpm-skips-hung-stop, towing-drive-splits-18min) and no Fix debug group; usbee one `handoff` group, `stale: true`, 6 items; wordoclock `closes_after = {test:12, decide:0, fix:1, blocked:5, phases_open:["20","21","22","23","24","25"], not_built:[]}` and `summary.decide == 2` (two awaiting debug sessions, root-scoped, not in `closes_after`).

### Sampling Rate
- **Per task commit:** `rtk bypass cargo test --lib owner_items` + the touched integration binary.
- **Per wave merge:** `rtk bypass cargo test --test owner_audit_conformance --test owner_open_conformance --test owner_digest --test inbox_cli --test status_cli --test async_blocking_guard`.
- **Phase gate:** full suite green + clippy (three targets) + the two live commands above.

### Wave 0 Gaps
- [ ] `tests/owner_open_conformance.rs` — audit-open oracle harness (copy the resolve/opt-out/`@file:`/zero-comparison shape of `owner_audit_conformance.rs`; extend the `src/` scan to forbid `audit-open`)
- [ ] Fixtures listed above (10 new dirs) with `oracle.expect` and `audit-open.expect`
- [ ] `tests/inbox_cli.rs` `generate_project` (cold 20-project timing) gains debug + CONTEXT + HANDOFF files so the < 1 s bound covers the new reads

## Security Domain

`security_enforcement` is absent from `.planning/config.json` (treated as enabled).

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | read-only CLI |
| V5 Input Validation | yes | All project files are untrusted: `open_checked` (NOFOLLOW, regular file, 1 MiB), FAILSAFE YAML with anchor refusal, `terminal_capped` on every emitted string, `is_command_token`/`is_workstream_name` before any command string |
| V6 Cryptography | no (sha256 is an identity digest for upstream parity, not a security control) | `sha2` |
| V12 Files and Resources | yes | containment to the canonical planning tree / phase dir; debug entries skipped unless lstat says regular file |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal escape / bidi in `trigger`, `action`, question text | Tampering | `terminal_capped`; extend `hostile_strings_are_escaped_in_json_and_text` with debug + handoff + CONTEXT strings |
| Symlink in `debug/` or `phases/` pointing outside | Information disclosure | lstat skip (debug), `contained()` (phase files), NOFOLLOW open |
| FIFO / huge file in `debug/` or HANDOFF.json | DoS | non-blocking open, `READ_CAP`, unreadable → Fix (never clean) |
| YAML alias bomb in debug/CONTEXT frontmatter | DoS | `refuses_anchors` before `serde_yml` |
| Hostile HANDOFF `phase` / workstream name reaching `--ws` | Tampering | command built only from validated tokens; else `command: null` |
| Forged `audit_acknowledged` hiding work | Repudiation | exact snapshot match; acknowledged groups still built and counted in `hidden.acknowledged_files` |

## Sources

### Primary (HIGH confidence)
- `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/audit.cjs` (1.21.0) lines 55-340, 553-626, 740-1236 — read; `query audit-open --json` executed on scratch fixtures and on copies of mailbot, sentriq, wordoclock, usbee `.planning/`
- `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/{audit-command-router,planning-workspace,security,frontmatter}.cjs` — read/executed
- `$CLAUDE_CONFIG_DIR/gsd-core/workflows/{pause-work,resume-project}.md`, `skills/gsd-debug/SKILL.md` — read
- Worktree code read this session: `src/state_reader/owner_items/{mod,rules,scan,verification,frontmatter,markdown}.rs`, `src/state_reader/mod.rs`, `src/inbox.rs`, `src/inbox_cli.rs`, `src/status_cli.rs`, `src/oneoff/todos.rs`, `tests/{owner_audit_conformance,owner_digest,inbox_cli,status_cli,async_blocking_guard}.rs`, `README.md` inbox section, `.planning/research/OWNER-INBOX-UX.md`, Phase 34 VERIFICATION/REVIEW
- Live `target/debug/gsd-meta-manager status --json` / `inbox --json` on real projects (read-only)

### Secondary / Tertiary
- None (no web sources were needed).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all in-tree, read this session.
- Upstream behavior: HIGH — read and executed against gsd-core 1.21.0.
- Architecture: HIGH for the port shape; MEDIUM for module split (discretion).
- Pitfalls / findings F1-F3: HIGH (measured); the proposed fixes are MEDIUM (owner rules needing planner acceptance, logged A1-A3).

**Research date:** 2026-11-14
**Valid until:** until gsd-core moves past 1.21.0 (oracle pin) or the real projects' files change (live numbers).
