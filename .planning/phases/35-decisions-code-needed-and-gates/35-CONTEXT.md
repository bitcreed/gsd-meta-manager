# Phase 35: Decisions, code-needed and gates - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

Gathered unattended (`--auto`). Every decision follows `.planning/research/OWNER-INBOX-UX.md`
(adopted as written by REQUIREMENTS.md) and the Phase 34 decisions unless marked **[inferred]**,
which means the orchestrator decided it from the report, the requirements, Phase 34's
CONTEXT/SUMMARY/VERIFICATION and a scout of the Phase 34 code and of gsd-core 1.21.0. One item was
added to scope by the orchestrator from live `inbox` output (D-14..D-16).

<domain>
## Phase Boundary

The owner reader (Phase 34, `src/state_reader/owner_items/`) learns the non-UAT signals of report §4
"Other signals" and the milestone gate, and both CLIs expose them additively:

- **Decide lane:** paused handoff (`HANDOFF.json` `status`, `next_action`), human actions pending
  (`human_actions_pending[]`, `blockers[]`), phase checkpoints (`phases/<p>/.continue-here.md`),
  debug sessions awaiting confirmation (D2), CONTEXT open questions minus RESOLVED/ANSWERED/CLOSED
  bullets (D11).
- **Fix lane:** open debug investigations (any other non-terminal debug status) and `gaps_found`
  verifications.
- **Counts:** pending-todo count and oldest age.
- **Gate:** `closes_after` per project (R8): lane totals and open phases for the current milestone,
  phases with no plans named "not built".
- **Added scope:** owner-facing names of VERIFICATION `human_verification` items show the test text,
  not gsd-core's flattened YAML (D-14..D-16).

Requirements: OWN-08, OWN-10 (extends OWN-13 and OWN-15 delivered in Phase 34).

Not in this phase: any TUI surface (dashboard `You` column, detail Owner block: Phase 36; inbox
screen, `v`/`A`/`y`: Phase 37); the acknowledge launcher (Phase 37); todo triage beyond a count;
STATE.md Blockers/Concerns parsing; heuristic "this UAT row is really a ruling" detection (report §6
deferred list, OWN-F01..).

</domain>

<decisions>
## Implementation Decisions

### Carried forward from Phase 34 (not re-decided)
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

### Signal model (OWN-10)
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

### Counts and the milestone gate (OWN-08, extends OWN-13/15)
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

### Owner-facing names of VERIFICATION items (added scope, orchestrator observation)
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

### Conformance (Layer A of the new signals)
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

### Musk pass
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

### Folded Todos
None.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and contract
- `.planning/research/OWNER-INBOX-UX.md` — §3.2 closes-after header, §3.3 DECIDE/FIX rows, §3.5
  schema (`closes_after`, `decide`), §4 "Other signals" table, R8, R10, §7 D1/D2/D11, §8 items 8-9
- `.planning/REQUIREMENTS.md` — OWN-08, OWN-10 (and OWN-13/15 for the additive rule)
- `.planning/ROADMAP.md` §"Phase 35" — goal and success criteria
- `README.md` §"Scripting and agents" and the `inbox` section — schema contract, additive-key rule

### Phase 34 (what this extends)
- `.planning/phases/34-owner-items-reader-inbox-cli/34-CONTEXT.md` — D-01..D-22
- `.planning/phases/34-owner-items-reader-inbox-cli/34-VERIFICATION.md` — live numbers (wordoclock
  test 12 / fix 1 / blocked 5), open info items IN-02, IN-04, IN-05
- `.planning/phases/34-owner-items-reader-inbox-cli/34-0*-SUMMARY.md`, `34-REVIEW.md`

### Upstream behavior to port (gsd-core 1.21.0, resolve via `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/`)
- `audit.cjs` — `scanDebugSessions` (~278, `DEBUG_KNOWLEDGE_BASE_FILENAME`), `deriveOpenQuestions`
  (~197), `deriveOpenQuestionsDigest` (~259), `scanContextQuestions` (~943),
  `isAuditItemAcknowledged`, the `audit-open` entry point
- `planning-workspace.cjs` — `debugDirFrom` / `debugDir` (~357)
- `uat.cjs` — `parseHumanVerificationItems`, `normalizeHumanVerificationEntry` (D-14 Layer A stays as is)

### Code to extend
- `src/state_reader/owner_items/mod.rs` — `Kind`, `Lane`, `OwnerItem`, `OwnerDigest`, `Hidden`,
  `read_owner_digest`, parse cache
- `src/state_reader/owner_items/verification.rs` — `human_verification_items`, `gap_items`,
  `normalize_entry` (D-14)
- `src/state_reader/owner_items/frontmatter.rs` — `fm_list`, `flatten_object_list_item`, ack marker
- `src/state_reader/owner_items/rules.rs` — `owner_item`, item ids, lanes, R7, counts
- `src/state_reader/owner_items/scan.rs` — phase targets, current-milestone filter, archives
- `src/state_reader/owner_items/markdown.rs` — sectionizer for `## Open Questions`
- `src/state_reader/mod.rs` — `ProjectState` handoff fields (`paused`, `pause_context`,
  `stale_handoff`, `continue_here_present`), `scopes()`, `in_current_milestone`
- `src/state_reader/disk_status.rs` — `DiskStatus` (plans present → not built), `VerificationStatus`
- `src/oneoff/todos.rs` — `list_pending`
- `src/inbox.rs` — `item_json`, `group_json`, `project_json`, `build_document`, plain checklist
- `src/inbox_cli.rs`, `src/status_cli.rs` (`owner` object), `src/agent_skill.md`, `README.md`
- `tests/owner_audit_conformance.rs`, `tests/owner_digest.rs`, `tests/inbox_cli.rs`,
  `tests/status_cli.rs`, `tests/fixtures/owner/`, `tests/fixtures/owner_golden/`

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Phase 34 owner reader: capped contained reads, FAILSAFE frontmatter (`frontmatter.rs`), markdown
  sectionizer (`markdown.rs`), acknowledgment matcher, parse cache, `terminal_capped`.
- `ProjectState` already detects HANDOFF (paused, stale) and root `.continue-here.md`.
- `oneoff::todos::list_pending` for todo files and titles; `sha2` already a dependency.
- Oracle harness of `tests/owner_audit_conformance.rs` (install candidates, opt-out env, TempDir copy).

### Established Patterns
- Layer A port compared with the oracle; Layer B rules unit-tested; declared divergences asserted
  against the oracle's specific output.
- Additive JSON keys, key paths pinned against README by `tests/inbox_cli.rs`.
- No process spawned under `src/` for gsd-core verbs (source scan).

### Integration Points
- `read_owner_digest` gains the new signals and `closes_after`; `inbox.rs` and `status_cli.rs`
  consume the digest (no second read path).
- `summary.decide` placeholder (`inbox.rs` `build_document`) and `owner.decide` become real.

</code_context>

<specifics>
## Specific Ideas

- SC1: `inbox --json --lane decide` lists sentriq's 5 awaiting debug sessions and usbee's paused
  handoff.
- SC2: wordoclock `closes_after` = test 12, blocked 5, fix 1, phases 20-25, current milestone only.
- Observed bad name (live output): `test: Weekly summary live send (PRICE-14) …, expected: …,
  why_human: …` must read `Weekly summary live send (PRICE-14) …` with `expected` and `why_human`
  as separate JSON fields.

</specifics>

<deferred>
## Deferred Ideas

- Report §6 deferred list (OWN-F01..): STATE.md Blockers/Concerns parsing, heuristic ruling
  detection, todo triage UI beyond a count, seeds and threads, native acknowledge writer,
  notifications, daily digest.
- Upstream issues to report (not fixed here): `deriveOpenQuestions` counts answered questions (§8
  item 8), debug status drift `awaiting-user` vs `awaiting_human_verify` (§8 item 9).
- 34 VERIFICATION follow-ups not in scope: `App::update(FileChanged)` eviction test (Phase 36 when
  the TUI consumes the cache), IN-02 id collision after truncation, IN-04 byte-bounded cache.

### Reviewed Todos (not folded)
`todo.match-phase 35` returned the same 17 keyword-only matches Phase 34 reviewed (score 0.2-0.9:
TUI badge for release maps, help popup at 80x24, unregister guard, registry glyphs, launcher-marker
in-flight flag, executor remnants, capture-pump redaction, run-lock hard link, managed-policy shapes,
run-file hardening, unsupervised child, multi-line queue input, ambiguous-wide typography, driverctl
`fm_status` IN-06, driverctl LOG.md redaction, D-17 recovery block, sibling-run tmpfs). None touches
the owner reader or its CLIs, so none is folded despite the ≥ 0.4 auto-fold rule; folding them would
be scope creep **[inferred]**.

</deferred>

---

*Phase: 35-decisions-code-needed-and-gates*
*Context gathered: 2026-11-14*
