# Phase 34: Owner items reader + inbox CLI - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

Gathered unattended (`--auto`). Every decision follows `.planning/research/OWNER-INBOX-UX.md`
(adopted as written by REQUIREMENTS.md) unless marked **[inferred]**, which means the orchestrator
decided it from the report, the requirements and a scout of the code and of gsd-core 1.21.0.

<domain>
## Phase Boundary

A native, process-free reader of per-item UAT/VERIFICATION data (Layer A, equal to gsd-core's
`audit-uat`) plus the owner's rules on top (Layer B: lanes, R2 dedup, R3 gap merge, R4 acknowledged,
R5 archived, R6 deferred, R7 closeable-by-testing, R9 age, R10 commands), exposed as
`gsd-meta-manager inbox [--json]` and as additive `owner` counts in `status --json`. Includes the
`status: testing` fix (OWN-09), safety (OWN-11) and cost (OWN-12).

Requirements: OWN-01..07, OWN-09, OWN-11..15.

Not in this phase: debug / handoff / CONTEXT-question / todo-age signals and the Decide lane content
(OWN-10, Phase 35), `closes_after` (OWN-08, Phase 35), any TUI surface (Phases 36-37), the
acknowledge launcher (Phase 37).

</domain>

<decisions>
## Implementation Decisions

### `status: testing` and what "outstanding" means (OWN-09)
- **D-01:** `UatStatus::from_raw` gains a `testing` arm (new variant `UatStatus::Testing`). 18 real
  UAT files carry it today and all land in `Other("testing")`, so nothing fires for them.
- **D-02:** Split the predicate **[inferred]**. The driver router's G7 gate (`src/driver/router.rs`
  ~634) is a transcription of gsd-core's `BLOCKING_UAT_FM_STATUSES`, which omits `testing`, and is
  pinned by `tests/driver_router_conformance.rs`. So: keep a predicate equal to the upstream set for
  the router (rename to e.g. `is_upstream_blocking()`, router unchanged in behavior), and make the
  owner-facing predicate (used by `status_cli`'s `uat_outstanding` and by the owner reader) =
  upstream set ∪ `{testing}` ∪ "status not in `{complete, resolved, passed, deferred}` and the file
  has ≥ 1 open Test or Fix item". Router divergence from upstream is recorded as an upstream issue
  (report §8 item 6), not fixed here.
- **D-03:** The existing negative test (`passed`, `deferred`, `complete` do not gate) stays green;
  `deferred` remains a human's explicit decision for the attention item (items inside the file
  still follow Layer A/B). `tests/status_cli.rs` key paths are unchanged.
- **D-04 (folded todo):** `UatStatus::from_raw` and `VerificationStatus::from_raw` accept an
  optionally single- or double-quoted value (`status: "gaps_found"`), matching gsd-core's
  `extractFrontmatter`; add a quoted fixture to the disk_status tests and to the owner fixtures
  (todo `2026-10-18-dashboard-verification-status-quote-divergence.md`; driverctl's `fm_status`
  port is not touched).

### Layer A parity with gsd-core (OWN-01, OWN-02)
- **D-05:** New module `src/state_reader/owner_items.rs` (report §4 types `OwnerItem`, `OwnerGroup`,
  `OwnerDigest`, `Lane`, `Kind`, `Category`). It is a faithful port of gsd-core 1.21.0
  `uat.cjs` (`cmdAuditUat`, `parseUatItemsWithStats`, `parseVerificationItems`, `categorizeItem`),
  `audit.cjs` (`isAuditItemAcknowledged`) and `phase-locator.cjs` (`listMilestonePhaseDirs`,
  `getAllArchivedPhaseDirs`). Nothing in it spawns a process.
- **D-06:** Scan targets mirror upstream exactly: active `.planning/phases/*` filtered to the
  current milestone (ROADMAP window, sentinel dirs such as `999.x` excluded, using our existing
  ROADMAP parser), plus every archived `.planning/milestones/<v>-phases/*` **unfiltered**, across
  root and workstreams (`state.scopes()`; upstream's cross-workstream enumeration, #3804).
- **D-07:** A project with `.planning/` but no phases dir and no archive yields empty groups and
  `error: null` (upstream hard-errors, report §8 item 7). The conformance test treats the oracle's
  "No phases directory found" on such a fixture as the expected empty result, not as a missing
  comparison.
- **D-08:** Conformance test is a new `tests/owner_audit_conformance.rs` copying the oracle pattern
  of `tests/driver_router_conformance.rs`: oracle resolved via `gsd_install::install_candidates`,
  runs `gsd-tools query audit-uat --raw` per fixture copied into a `TempDir`, fails when the oracle is
  missing unless the **same** `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` is set **[inferred]**, and fails
  if any fixture produced no comparison. Compared: the set of (file, test, result, category), plus
  `acknowledged_files` and `parse_gap_files` counts. Layer B results are never compared with the
  oracle (they deliberately differ); they are unit-tested.
- **D-09:** Source scan **[inferred]**: no non-comment line of any `src/**/*.rs` contains the verb
  `audit-uat` (string literal or argv). `src/agent_skill.md` is prose and must say "do not run
  audit-uat per project", so markdown files are excluded from the scan.
- **D-10:** Fixtures under `tests/fixtures/owner/<name>/.planning/…` are **synthetic** **[inferred]**:
  they reproduce the structure, statuses and counts of the real cases (wordoclock ph 20-25 → 18
  items not 39; mailbot ph 09 → 10 failed checks + 14 gaps; daily-vow v1.5 archived `complete` with
  skipped rows; shopify ph 11 `source:` mismatch; 200-entry `deferred-items.md`; the four
  acknowledged variants; a `testing` UAT; a no-phases project; a workstream project) with neutral
  invented text. The crate is published on crates.io, so no prose from the owner's private
  projects is copied. Real projects under `~/projects` and the stale snapshot in the scratchpad audit
  directory are read-only sanity checks during development, never committed.

### Layer B rules (OWN-03..07)
- **D-11:** R1-R10 exactly as report §4 Layer B, with D3 strict (no `source:` match → show both),
  D8 (`unknown` rows without a result in a terminal-status file are hidden and counted;
  `failed`/`partial` results go to Fix) and D9 (stale > 30 days).
- **D-12:** R7 uses the existing `DiskStatus` for "all plans have summaries" and the existing
  `VerificationStatus` for the verification condition. Bookkeeping-only groups render as "all
  checks passed, run `/gsd-verify-work N` to finalize". OWN-07's 8-combination table test lives
  next to the module.
- **D-13:** R9 age **[inferred]**: `updated:` frontmatter, else file mtime. Per-file git commit time
  is used only if an existing helper provides it without one spawn per file; otherwise it is dropped,
  because the < 1 s cold budget (OWN-12) wins. `stale_days` lives in the app config as
  `owner.stale_days` (default 30), read with the existing config loader (read-only for the CLI).
- **D-14:** All computation takes `now` as a parameter so age/stale are reproducible in golden and
  JSON tests **[inferred]**.

### Placement and cost (OWN-12)
- **D-15:** The digest is computed by a separate entry point (e.g.
  `owner_items::read_owner_digest(planning_dir, &ProjectState, now)`), not inside
  `parse_project_state` **[inferred]**. Reason: `parse_project_state` also runs in the driver loop,
  `next` and the router, which must not pay an archive scan. `ProjectState` gets an
  `owner: Option<OwnerDigest>` (or equivalent) that the TUI's background reload fills in Phase 36.
  This narrows report §4 ("`ProjectState` gains `owner`"). — **Reversibility:** reversible
- **D-16:** Per-file parse cache keyed by (path, mtime, len) in a process-wide map inside
  `owner_items` (std `Mutex` is fine: never held across `.await`); a changed key re-parses, so the
  watcher's existing reload invalidates implicitly, and a watcher event for a project also evicts
  that project's entries. No parse on the render path; `tests/async_blocking_guard.rs` extended.
  Timing test: cold `inbox --json` over a synthetic 20-project tree < 1 s.

### Safety (OWN-11)
- **D-17:** Reads capped at 1 MiB by generalizing the existing capped reader
  (`driver_frontmatter::read_run_file_capped` pattern); an oversize file is reported as a parse gap
  ("unreadable checks", never clean) **[inferred]**. Symlinks escaping the phase dir are absent
  (upstream `isReportContained`). Invalid UTF-8 is decoded lossily (as Node does); an unterminated
  frontmatter reads as `{}` via the existing `state_md::read_frontmatter`. Hostile fixtures are kept
  out of the oracle comparison. Every emitted project-authored string goes through
  `text::render_for_terminal` and the `status` cap (`MAX_TEXT_CHARS`).

### `inbox` CLI (OWN-13, OWN-14)
- **D-18:** New `Commands::Inbox { projects, json, lane, include }` in `src/cli.rs`, new
  `src/inbox_cli.rs`, dispatched in `main` before `init_tracing` like `status` (writes nothing, no
  lock, no log). Target resolution, slot handling and exit codes (0 / 1 with one `inbox: ` stderr
  line / 2 clap) are shared with `status_cli` by factoring its `resolve_target`/`Slot` helpers out
  rather than copying them **[inferred]**.
- **D-19:** `--json` prints schema `gsd-meta-manager/inbox` v1 on one line, exactly report §3.5.
  Phase 34 keys whose data arrives in Phase 35 are emitted now with zero/empty values (`decide: 0`)
  so Phase 35 is purely additive; `milestone.closes_after` is omitted until Phase 35 (additive rule)
  **[inferred]**. `hidden.todos` is the pending-todo count from the existing
  `oneoff::todos::list_pending` (oldest age is Phase 35). — **Reversibility:** costly — key names are
  a published README contract; renames need a `schema_version` bump, additions do not.
- **D-20:** Determinism: projects in the same order as `status`; groups per project sorted by scope
  (current, then archived by milestone), lane (`test`, `decide`, `fix`, `blocked`), phase number
  (numeric), file path; items by test number. No timestamps. `--lane` and `--include` take
  comma-separated values (clap `value_delimiter`) **[inferred]**; `--include` admits `archived`,
  `deferred`, `acknowledged`, `todos`; without it archived groups, deferred items and acknowledged
  files are counted in `summary`/`hidden` but not listed.
- **D-21:** Plain `inbox` prints the report §3.3 content as a terminal-safe checklist without TUI
  chrome: a summary line, then per lane a heading (`TEST — closes a phase`, `TEST — phase stays
  open`, `FIX — code needed`, `BLOCKED`), one line per group (alias, phase, name, checks, age, the
  command), its items as `- [ ] <name>`; folds as one-line counts (old debt, acknowledged, todos).
  Cross-project order inside TEST: closes-a-phase first, then milestone progress (closest to close
  first), then age (oldest first), ties by alias, phase, file **[inferred: oldest-first]**. No
  color. Golden-output test with a fixed `now`.

### `status --json` and the skill (OWN-15)
- **D-22:** Each readable project entry gains `owner: {test, decide, fix_phases, blocked,
  closeable_phases}` (current milestone only, after Layer B; `decide` is 0 until Phase 35); no
  schema bump. README's "Scripting and agents" contract and `tests/status_cli.rs` gain the key
  paths; `src/agent_skill.md` gets the paragraph from report §3.5 ("to find missing UATs or pending
  owner work, run `gsd-meta-manager inbox --json`; do not run audit-uat per project"). README also
  documents the `inbox` command and its schema (the TUI part of OWN-23 stays in Phases 36-37).

### Claude's Discretion
- Exact Rust type layout, module split inside `owner_items` (e.g. `uat_parse`, `rules`), helper
  names, and how the shared CLI target code is factored.
- Number and grouping of plans (coarse granularity).
- Whether the per-file cache stores parsed items or whole groups.

### Folded Todos
- **Dashboard renders quoted VERIFICATION status as Unknown**
  (`.planning/todos/pending/2026-10-18-dashboard-verification-status-quote-divergence.md`): the
  owner reader reads the same `status:` values and must match upstream's quote-stripping; folded as
  D-04. Move the todo to done when the phase ships.

### Musk pass
- Milestone pass (`.planning/musk/MILESTONE-v2.5.0.md`) and phase pass: no cuts. Every Phase 34
  requirement traces to the owner's request and the report adopted as written; items the user asked
  for are never cut **[inferred]**. Lean lane not used: 13 requirements across reader, CLI and tests.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and contract
- `.planning/research/OWNER-INBOX-UX.md` — source of truth: §3.5 `inbox` JSON schema, §4 data model,
  Layer A/B rules and signal table, §6 success criteria, §7 defaults D1-D12, §8 upstream issues
- `.planning/REQUIREMENTS.md` — OWN-01..07, 09, 11..15 with their tests
- `.planning/ROADMAP.md` §"Phase 34" — goal and success criteria
- `README.md` §"Scripting and agents" — `status` schema v1 contract and the additive-key rule

### Upstream behavior to port (gsd-core 1.21.0, resolve via `$CLAUDE_CONFIG_DIR/gsd-core/bin/lib/`)
- `uat.cjs` — `cmdAuditUat` (scan targets, parse gaps), `parseUatItemsWithStats`,
  `parseVerificationItems`, `categorizeItem`
- `audit.cjs` — `isAuditItemAcknowledged` and the snapshot keys (`gap_snapshot`, `status`)
- `phase-locator.cjs` — `listMilestonePhaseDirs`, `getAllArchivedPhaseDirs`
- `frontmatter.cjs` — `extractFrontmatter` (quote stripping, unterminated block → `{}`)
- `uat-predicate.cjs` — `BLOCKING_UAT_FM_STATUSES` (the router's set; omits `testing`)

### Code to extend or mirror
- `src/state_reader/disk_status.rs` — `UatStatus`, `VerificationStatus`, `DiskStatus`, UAT file
  discovery (~1138), negative gate test (~2342)
- `src/driver/router.rs` ~634 — G7 gate that must keep upstream parity (D-02)
- `src/status_cli.rs` — target resolution, exit codes, escaping, `attention_json` `uat_outstanding`
- `src/cli.rs` `Commands::Status` — CLI shape and the `spawn_seam_guard` naming note
- `src/state_reader/mod.rs` — `ProjectState`, `scopes()`, `parse_project_state`
- `src/state_reader/gsd_install.rs` — `install_candidates`
- `src/state_reader/driver_frontmatter.rs` — `read_run_file_capped` (1 MiB cap pattern)
- `src/state_reader/state_md.rs` — `read_frontmatter`
- `src/oneoff/todos.rs` — `list_pending`
- `src/text.rs` — `render_for_terminal`
- `src/agent_skill.md` — `status --skill` text
- `tests/driver_router_conformance.rs` — oracle pattern; `tests/status_cli.rs` — key-path pinning;
  `tests/async_blocking_guard.rs` — no blocking parse on async/render paths

### Sanity data (read only, never committed)
- `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/3c5b5553-2c5b-4685-8e9a-1e01e7d70b16/scratchpad/audit/*.json`
  — per-project `audit-uat --raw` snapshot from 2026-11-13 (`results`, `summary`,
  `acknowledged_files`); `*.err` holds the "No phases directory found" cases. A day stale; may be
  gone after a reboot. Real projects under `~/projects` may be read, never modified.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `UatStatus` / `VerificationStatus` / `DiskStatus` (`disk_status.rs`): file-level status and
  "all plans have summaries" for R7.
- `ProjectState::scopes()`: root plus workstreams, already used by `status_cli::attention_json`.
- `status_cli` target resolution (`resolve_target`, `Slot`, `canon_or_raw`, `owning_root`), `fail`,
  `out_line`, `cap`/`text` helpers: factor out for `inbox`.
- `gsd_install::install_candidates` + the conformance harness of `driver_router_conformance.rs`.
- `state_md::read_frontmatter`, `driver_frontmatter::read_run_file_capped`,
  `text::render_for_terminal`, `oneoff::todos::list_pending`.

### Established Patterns
- Read-only CLIs (`status`, `next`) dispatch before `init_tracing`, emit one compact JSON line,
  no timestamps, `schema` + `schema_version`, additive keys without a bump.
- Oracles run only in tests; a source scan proves the verb never appears under `src/`.
- Missing-oracle runs fail unless `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` is set (non-empty).
- Fail-safe parsing: unreadable artifacts mean "nothing observed", never "passed".

### Integration Points
- `src/cli.rs` (new `Inbox` variant), `src/main.rs` (dispatch), new `src/inbox_cli.rs`.
- `src/state_reader/mod.rs` (export `owner_items`, optional `owner` on `ProjectState`).
- `src/status_cli.rs` (`owner` key, owner-facing `uat_outstanding`), `src/driver/router.rs`
  (predicate rename only).
- `src/config.rs` (`owner.stale_days`), `README.md`, `src/agent_skill.md`.
- `scripts/pre-tag-check.sh --container` must stay green with the new oracle test (the container
  provisions the oracle through `scripts/install-conformance-oracle.sh`).

</code_context>

<specifics>
## Specific Ideas

- Success anchor: `gsd-meta-manager inbox` on the owner's machine answers "find the missing UATs" for
  all registered projects in one call; wordoclock shows 18 open checks (not 39 or 41); bitcreed.us,
  starlink-rv-tracker and usbeehive return empty groups.
- Fix groups report `failed checks` and `gaps` separately (mailbot ph 09: 10 and 14).
- `deferred-items.md` with 200 entries → `hidden.deferred == 200`, zero groups.

</specifics>

<deferred>
## Deferred Ideas

- Router G7 adopting `testing`: only if gsd-core adds it to `BLOCKING_UAT_FM_STATUSES` (report §8
  item 6); the router conformance test will flag the change.
- Per-file git commit time for R9 age, if a batched helper appears later (D-13).
- Filling `ProjectState.owner` in the TUI reload: Phase 36.

### Reviewed Todos (not folded)
Matched by keyword only (score 0.6-0.9) and outside this phase's reader/CLI scope **[inferred]**:
queue input multi-line, ambiguous-wide typography/DarkGray, TUI badge for release maps, help popup
at 80x24, launcher-marker in-flight flag, unregister guard run locks, registry glyph overflow (UI);
D-17 recovery block, driverctl `fm_status` IN-06 shapes, executor remnants, sibling-run tmpfs
symlink, driverctl LOG.md redaction, capture-pump redaction, run-lock hard link, managed-policy
shapes, run-file open hardening, unsupervised child (driver/launcher).

</deferred>

---

*Phase: 34-owner-items-reader-inbox-cli*
*Context gathered: 2026-11-14*
