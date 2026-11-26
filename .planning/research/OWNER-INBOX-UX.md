# Owner Inbox: feature set for gsd-meta-manager

Input for `/gsd-new-milestone` (proposed: **v2.5.0 "Owner Inbox"**, phases 34-37; phase 33 was the
last one used). Evidence: `gsd-tools query audit-uat --raw` over 20 registered projects
(snapshot 2026-11-13, gsd-core 1.21.0), plus a file survey of the same projects and a read of
`src/status_cli.rs`, `src/state_reader/disk_status.rs`, `src/ui/screens/mod.rs`, `detail.rs`,
`src/multiplexer/mod.rs`, `tests/driver_router_conformance.rs`. Note: the repo has no `AGENTS.md`;
`CLAUDE.md` is the project instruction file.

## 1. Problem

The owner runs ~20 GSD projects and keeps asking Claude to "find the missing UATs". The app cannot
answer, because it reads only the `status:` line of `NN-UAT.md` / `NN-VERIFICATION.md`:

- `UatStatus::from_raw` does not know `testing`, the status `/gsd-verify-work` writes while checks
  are open. It lands in `Other("testing")`, which `is_outstanding()` treats as not outstanding.
  12 UAT files across 7 projects are in that state and raise nothing today.
- `needs_human` (badge, AttentionFirst, `/h`) reads only paused / external-job / parked.
- Archived milestones are never scanned, so a phase closed as `complete` with skipped on-device
  checks is invisible forever.
- Asking gsd-core instead gives a number nobody can act on:

| Snapshot, 17 readable projects | Items |
|---|---|
| Raw `audit-uat` total | 568 |
| Prose from `deferred-items.md` (305 of them from this repo's archive) | 336 |
| Double-counted: VERIFICATION `human_needed` already seeded into a UAT | 61 |
| **To test** (pending, human_uat, skipped_unresolved) | **72** (43 current, 29 archived) |
| **Blocked** (device, build, third party, server, other) | **21** (6 current, 15 archived) |
| **Fix** (issue, failed, gaps_found) | **78** (54 current, 24 archived), still inflated: see R3 |

So 568 raw items are really about 43 checks the owner can run today, grouped in about 20 phases,
of which 17 would close by testing alone.

## 2. Owner jobs-to-be-done

Each morning, in under a minute, without opening a project:

1. **What can I close by testing?** "Run these 6 checks and wordoclock phases 21, 22, 24 close."
2. **What is waiting on my ruling?** Paused handoffs, pending human actions, fixes waiting for my
   confirmation, open questions.
3. **Where is code needed?** Failed checks and verification gaps, so I can dispatch an agent.
4. **What blocks the next milestone?** One sentence per project.
5. **What did I knowingly leave behind?** Skipped checks in archived milestones, on request only.

Cut on purpose: STATE.md `### Blockers/Concerns` (97 prose bullets in 14 projects, mostly stale
history), seeds, threads, deferred-items as inbox rows, per-item badges on the dashboard,
notifications. They do not change what the owner does today.

## 3. Feature set

Principle: **rows are phases, not items; the unit shown is the action and what it unlocks.** One
quiet text cell per project, no new badge. The `❢` badge and `needs_human` stay reserved for hard
stops (paused, parked, external job) and are not widened.

### 3.1 Dashboard: one `You` column and one summary line

Mockup rows in 3.1-3.3 use real project and phase names; counts outside section 1 are illustrative.

```
 Alias              Phase                   Status           You                  Progress
 aiFlowAgent        10 cases-sandbox        verify           test 3 → closes 10   ████████░ 9/10
 cdr-configurator   18 visual-parity        verify           test 10 → closes 4   ███████░░ 13/18
 mailbot            18 rule-audit           uat diagnosed    fix 4 ph · test 6    ██████░░░ 11/18
 wordoclock         25 first-day-unlock     verify           test 12 → closes 3   █████████ 24/25
 shopify-orderly    18.2 billing            uat partial      decide 1 · fix 2 ph  ████████░ 17/19
 usbee           ❢  04 wire-shape           paused           decide 1             ███░░░░░░ 3/8
 hitchmatch         02 matching             executing        ·                    ██░░░░░░░ 2/9
 picsync            — shipped v1.0          shipped          · (+3 old)           █████████ 8/8

 You: test 43 in 20 phases (17 close) · decide 9 · fix 9 phases · 11 of 20 projects   [i] inbox
```

- The cell shows at most two lanes, in fixed order `decide`, `test`, `fix`; `blocked` never shows
  here (the owner cannot act on it). `→ closes N` counts phases closeable by testing alone (R7).
- Current milestone only. Archived debt is a dim `(+N old)`; a clean project is a dim `·`.
- Column drops below 100 columns (same breakpoint logic as `Backlog`).
- Sort: AttentionFirst gets one new rank between "needs a human" and "driven": `has owner work`.
  Alphabetical stays the default. Filter `/y` (optionally `/y test`, `/y decide`, `/y fix`).

### 3.2 Project detail: an `Owner` block on the Phases tab (no new tab)

```
 Owner ── wordoclock · v2.2 closes after: test 12 (ph 21 22 24) · unblock 5 (ph 20 23) · fix 1 (ph 25)

 TEST    ph 24  widget-clock-faces      6 checks   closes phase      2 d   /gsd-verify-work 24
   1 Confirm the R-8 addendum signature is the operator's own
   2 Native-speaker review of Spoken's number words (de/fr/es/it/pt)
   … 4 more
 TEST    ph 21  android-widget-faces    2 checks   closes phase      4 d   /gsd-verify-work 21
 TEST    ph 25  first-day-unlock        3 checks   + fix 1          2 d   /gsd-verify-work 25
 DECIDE  debug  widget-tap-no-open-app  awaiting your confirmation   2 d   /gsd-debug
 FIX     ph 25  1 gap: quota placeholder clipped at font scale 2.0         /gsd-plan-phase 25 --gaps
 BLOCKED ph 23  2 device · 1 third party · 1 other
 ▸ Old debt (1): v1.2 ph 08 — 1 deferred            ▸ Acknowledged (8 files)   ▸ Todos 11 (oldest 96 d)
```

Enter on a row expands its items (name, expected, reason, source file). `o` opens the source file in
the Docs viewer. `Enter` on the command launches it (3.4). The existing verification remedy line
(`detail.rs` ~9435) stays and links to the same commands.

### 3.3 Cross-project inbox (`i` on the dashboard)

```
 Owner inbox ── 20 projects · test 43 · decide 9 · fix 9 phases · blocked 6      [1]Test [2]Decide [3]Fix [4]Blocked [a]ll

 TEST — closes a phase                                             checks   age
 ▸ wordoclock        ph 24  widget-clock-faces                        6     2 d
 ▸ cdr-configurator  ph 18  visual-parity-gate                        3    29 w  stale
 ▸ aiFlowAgent       ph 10  cases-sandbox-crm-replay                  3    10 d
 ▸ knit              ph 21  …                                         2     9 d
 TEST — phase stays open (also needs fix or unblock)
 ▸ mailbot           ph 05  …                                         1    11 w
 DECIDE
 ▸ usbee             handoff paused · 1 human action pending               26 w  stale
 ▸ sentriq           5 debug sessions awaiting your confirmation            3 w
 ▸ mailbot           ph 03  2 open questions in 03-CONTEXT.md              11 w
 FIX — code needed
 ▸ mailbot           ph 09  10 failed checks (14 gaps, diagnosed)           /gsd-plan-phase 9 --gaps
 ▸ shopify-orderly   ph 16  verification gaps_found: 7                      /gsd-plan-phase 16 --gaps
 ──
 ▸ Old debt: 29 checks in 6 projects (archived milestones)   ▸ Triage: 232 pending todos in 15 projects

 j/k move · Enter expand · g go to project · v run command · o open file · A acknowledge · y copy checklist
```

Ordering inside TEST: closes-a-phase first, then by project milestone progress (closest to a
milestone close first), then age. `y` copies the selected group as a plain checklist (the thing the
owner pastes into a phone note before walking to the device).

### 3.4 Actions (reuse, no new machinery)

- **Run command**: `multiplexer::launch(plan, cwd, ExecArgv, gui_fallback, msg)` (the path
  `detail.rs` ~4952 uses for `--resume`) with argv `claude "/gsd-verify-work 24"`. If a live session
  exists for the project, focus it instead and put the command in the status line. Phase 37 only.
- **Acknowledge** (`A`): never a native write. Launches
  `gsd-tools audit acknowledge --category uat_gaps|verification_gaps|debug_sessions|context_questions --phase N --milestone <current>`
  through the same launch path after a confirm popup, so gsd-core stays the single writer of
  `audit_acknowledged` (handler: `cmdAuditAcknowledge` in `audit.cjs`; confirm the exact CLI spelling
  and per-category flags during planning). Until Phase 37 the reader only honors the marker.

### 3.5 CLI for Claude: `gsd-meta-manager inbox`

`inbox [project...] [--json] [--lane test|decide|fix|blocked] [--include archived,deferred,acknowledged,todos]`.
Writes nothing, takes no lock (same posture as `status`). Without `--json` it prints the checklist
text of 3.3. Schema `gsd-meta-manager/inbox` v1, one line:

```json
{"schema":"gsd-meta-manager/inbox","schema_version":1,
 "summary":{"projects":20,"test":43,"decide":9,"fix_phases":9,"blocked":6,"closeable_phases":17,
            "archived":{"test":29,"blocked":15,"fix":24},"deferred":336,"acknowledged_files":71,"superseded":61},
 "projects":[{"alias":"wordoclock","path":"/…/wordoclock","error":null,
   "milestone":{"version":"v2.2","closes_after":{"test":12,"blocked":5,"fix":1,"decide":0,
                "phases_open":["20","21","22","23","24","25"]}},
   "groups":[{"lane":"test","kind":"uat","scope":"current","workstream":null,"phase":"24",
     "phase_name":"widget-clock-faces-nine-layout-faces","file":".planning/phases/24-…/24-UAT.md",
     "file_status":"testing","closes_phase":true,"age_days":2,"stale":false,
     "command":"/gsd-verify-work 24","supersedes":["24-VERIFICATION.md"],
     "items":[{"id":"24-UAT.md#1","test":"1","name":"Confirm the R-8 addendum…",
               "expected":"…","result":"pending","category":"pending","reason":null,"blocked_by":null}]}],
   "hidden":{"deferred":1,"acknowledged_files":8,"todos":11}}]}
```

`status --json` gains one additive key per project, `owner: {test, decide, fix_phases, blocked,
closeable_phases}` (no schema bump: additive keys are allowed by the v1 rule). `agent_skill.md`
(`status --skill`) gets a paragraph: "to find missing UATs or pending owner work, run
`gsd-meta-manager inbox --json`; do not run audit-uat per project".

## 4. Data model and classification rules

New module `src/state_reader/owner_items.rs`; nothing in it runs a process.

```
OwnerItem  { id, test: Option<String>, name, expected, result, reason, blocked_by, category: Category }
OwnerGroup { lane: Lane, kind: Kind, scope: Current | Archived(milestone), workstream, phase,
             file, file_status, items, closes_phase, age: Option<Duration>, command: Option<String>,
             supersedes: Vec<String>, acknowledged: bool }
OwnerDigest{ groups, closeable_phases, milestone_gate, hidden: { deferred, acknowledged_files, todos } }
Lane = Test | Decide | Fix | Blocked      Kind = Uat | Verification | Debug | Handoff | ContextQuestions | Todos
```

`ProjectState` gains `owner: OwnerDigest`, per root and per workstream (the reader already iterates
`state.scopes()`; sentriq has 4 workstreams). Debug sessions and todos are root-scoped, as upstream.

### Layer A: raw items, must equal gsd-core (`cmdAuditUat`, `categorizeItem`)

| Rule | Source of truth |
|---|---|
| Scan `.planning/phases/*` (current-milestone dirs) and every `.planning/milestones/<v>-phases/*` | `uat.cjs` cmdAuditUat |
| UAT files: `*-UAT.md` and `*-HUMAN-UAT.md` scoped to the phase token; items from `### N.` blocks (`result:` incl. `[pending]`) and the `## Gaps` section | parseUatItemsWithStats |
| VERIFICATION files: only `status: human_needed` or `gaps_found`; items from `human_verification` and gaps | parseVerificationItems |
| Category: blocked/blocked_by → server_blocked, device_needed, build_needed, third_party, blocked; skipped + reason → deferred ("Deferred follow-up"), server_blocked, device_needed, build_needed, else skipped_unresolved; pending; human_needed → human_uat; issue; else unknown | categorizeItem |
| A file whose `### N.` blocks yield no items is a `parse_gap`, shown as "N unreadable checks", never as clean | cmdAuditUat |
| Symlink escaping the phase dir is treated as absent; files read with a 1 MiB cap; all text through `text::render_for_terminal` | isReportContained, repo convention |

### Layer B: the owner's rules (ours, unit-tested; this is the value added)

- **R1 lanes.** Test = pending, human_uat, skipped_unresolved. Blocked = device_needed,
  build_needed, third_party, server_blocked, blocked. Fix = issue, `failed`/`partial` results, every
  item of a `gaps_found` VERIFICATION. Deferred = category deferred and all `deferred-items.md`.
  Hidden noise = `unknown` items with no `result` in a file whose status is terminal
  (`complete`, `resolved`, `passed`); counted in `hidden`, never shown.
- **R2 UAT supersedes its source VERIFICATION.** If a phase has a UAT file whose `source:`
  frontmatter lists `NN-VERIFICATION.md`, that VERIFICATION's `human_needed` items are dropped
  (recorded in `supersedes`). 61 items in the snapshot. No `source:` match → both are shown (real
  case: shopify ph 11, `source: [11-03-PLAN.md]`). A `gaps_found` VERIFICATION is never superseded.
- **R3 gaps merge into their test.** A `## Gaps` entry carrying `test: N` is detail of the
  `result: issue` row N, not a second item. mailbot ph 09: 24 raw rows = 10 failed checks with 14
  gaps. Fix groups report `failed checks` and `gaps` separately.
- **R4 acknowledged.** Honor `audit_acknowledged` exactly as `isAuditItemAcknowledged`: map with
  non-empty `milestone` and `at`; UAT snapshot key `gap_snapshot` = `<status>::scenarios=<count of
  result: pending|[pending]>`; VERIFICATION and debug key `status`; CONTEXT key `questions_digest`.
  Mismatch or malformed marker → item resurfaces. Acknowledged files are counted, never listed by
  default. 71 files already carry it (knit 36, daily-vow 11, wordoclock 8).
- **R5 archived.** A group from `milestones/<v>-phases/` has scope Archived. It is excluded from the
  dashboard cell, the summary counts and `closes_after`; shown under "Old debt". A terminal file
  status does not hide open items (daily-vow v1.5 ph 20-23: 14 skipped checks behind `complete`).
- **R6 deferred hidden.** `deferred-items.md` entries and `deferred` items are only a count.
- **R7 closeable by testing alone.** Phase P in the current milestone, not complete in ROADMAP, is
  closeable when: all plans have summaries (existing `DiskStatus`), P has ≥ 1 Test item, 0 Fix and
  0 Blocked items, no parse gap, and its VERIFICATION status is `passed` or `human_needed` (not
  `gaps_found`, `stale`, `unparseable`). Special case **bookkeeping-only**: 0 open items after R2 but
  UAT status non-terminal or VERIFICATION still `human_needed` → row "all checks passed, run
  `/gsd-verify-work N` to finalize".
- **R8 milestone gate.** `closes_after` = lane totals over current-milestone phases not complete,
  plus the phase list. Rendered as the one-line header of 3.2. Phases with no plans yet are named
  as "not built", not counted as owner work.
- **R9 age.** `updated:` frontmatter, else git commit time, else mtime (existing staleness helper).
  `stale` at > 30 days (config key `owner.stale_days`).
- **R10 commands.** Test → `/gsd-verify-work N`; Fix from UAT or gaps_found → `/gsd-plan-phase N --gaps`;
  debug → `/gsd-debug`; handoff → `/gsd-resume-work`; every one carries `--ws <name>` in a
  workstream, same as the existing remedy formatter.

### Other signals (all verified present in the real projects)

| Signal | File and field | Lane | Seen in snapshot |
|---|---|---|---|
| Paused handoff | `.planning/HANDOFF.json` `status`, `next_action` (existing `paused`) | Decide | cdr, ttbook, usbee |
| Human actions pending | `HANDOFF.json` `human_actions_pending[]` non-empty, `blockers[]` non-empty | Decide | ttbook, usbee |
| Phase checkpoint | `phases/<p>/.continue-here.md` frontmatter `status`, `phase` (existing `continue_here_*`) | Decide | cdr (3 files) |
| Fix awaiting confirmation | `.planning/debug/*.md` `status` starting `awaiting` after `-`→`_` normalization (`awaiting_human_verify`, real variant `awaiting-user`); skip `knowledge-base.md` and `resolved/` | Decide | 12 sessions in 7 projects |
| Open investigation | same files, any other non-terminal status (`root_cause_found`, `diagnosed`, `investigating`…) | Fix | mailbot, shopify |
| Verification gaps | `NN-VERIFICATION.md` `status: gaps_found` (existing `VerificationStatus`) | Fix | shopify |
| Open questions | `NN-CONTEXT.md` `open_questions:` array, else `## Open Questions` bullets (`deriveOpenQuestions`), minus bullets marked RESOLVED / ANSWERED / CLOSED | Decide | mailbot (3 files, 1 fully answered) |
| Pending todos | `.planning/todos/pending/*.md` count and oldest `created:` (reader exists: `src/oneoff/todos.rs`) | count only | 232 in 15 projects (sentriq 127) |
| Parked driver run, external job | existing `needs_human` sources | Decide | existing |

Not present anywhere in the 20 projects, so out of scope: escalation/checkpoint files in phase dirs,
`async-jobs/`, threads. Seeds exist in 3 projects but are ideas, not owner work.

## 5. Requirements

Reader and rules
- **OWN-01** The state reader produces per-item UAT/VERIFICATION data natively (Layer A) for active
  and archived phases, root and workstreams. *Test: for every fixture in `tests/fixtures/owner/`,
  the set of (file, test, result, category) equals `gsd-tools query audit-uat --raw`.*
- **OWN-02** The conformance test uses the oracle pattern of `tests/driver_router_conformance.rs`:
  resolved through `gsd_install::install_candidates`, fails when the oracle is missing unless
  `ALLOW_MISSING_ORACLE` is set, fails if any fixture produced no comparison; a source scan asserts
  no non-comment line under `src/` names `audit-uat` as a spawned verb. *Test: itself, and it passes
  in `pre-tag-check.sh --container`.*
- **OWN-03** R2: a VERIFICATION named in a same-phase UAT `source:` contributes no items. *Test:
  fixture modeled on wordoclock ph 20-25 yields 18 items, not 39.*
- **OWN-04** R3: gap entries with `test: N` merge into issue row N. *Test: mailbot-ph09 fixture
  yields 10 failed checks and 14 gaps in one Fix group.*
- **OWN-05** R4: acknowledged files are suppressed while the snapshot matches and resurface when it
  does not or the marker is malformed. *Test: four fixtures (match, changed count, changed status,
  marker missing `at`), each also compared with the oracle's `acknowledged_files`.*
- **OWN-06** R5/R6: archived and deferred items never enter current counts; a `complete` archived
  UAT with skipped rows yields an Archived group. *Test: daily-vow-v1.5 fixture; deferred-items
  fixture with 200 entries yields `hidden.deferred == 200` and zero groups.*
- **OWN-07** R7: `closes_phase` and bookkeeping-only are computed as specified. *Test: table-driven
  unit test over the 8 combinations (fix present, blocked present, gaps_found, stale, missing
  summary, parse gap, zero open, clean).*
- **OWN-08** R8: `closes_after` lists lane totals and open phases for the current milestone only.
- **OWN-09** `status: testing` (and any non-terminal UAT status with open items) counts as
  outstanding; `status_cli`'s `uat_outstanding` attention item fires for it. *Test: regression
  fixture; existing `tests/status_cli.rs` key paths unchanged.*
- **OWN-10** Decide/Fix signals of section 4 are read with the stated fields; debug status is
  normalized (`-`/`_`, case) and `knowledge-base.md` is ignored. *Test: one fixture per row of the
  signal table.*
- **OWN-11** Safety: 1 MiB read cap, symlink containment, no panic on invalid UTF-8 or unterminated
  frontmatter, every rendered string through `render_for_terminal`. *Test: hostile fixtures plus the
  existing `render_escape_guard` coverage extended to the new screens.*
- **OWN-12** Cost: parsed files are cached by (path, mtime, len) and invalidated by the watcher; a
  cold `inbox --json` over the 20 real projects finishes in < 1 s, and no parse runs on the render
  path. *Test: `async_blocking_guard` extended; a timing assertion on a 20-project synthetic tree.*

CLI
- **OWN-13** `inbox --json` emits schema v1 of 3.5, deterministic (no timestamps, sorted), exit
  codes as `status`. *Test: `tests/inbox_cli.rs` pins every key path against the README contract.*
- **OWN-14** `inbox` without `--json` prints a plain checklist grouped by lane; `--lane` and
  `--include` filter as documented. *Test: golden output.*
- **OWN-15** `status --json` carries the additive `owner` counts; `status --skill` documents `inbox`.

TUI
- **OWN-16** Dashboard `You` column per 3.1: max two lanes, fixed order, current milestone only,
  hidden below 100 columns, dim `·` when clean. *Test: buffer snapshots at 80/100/140 columns.*
- **OWN-17** Summary line shows totals and `[i] inbox`; totals equal the sum of the visible cells.
- **OWN-18** AttentionFirst gains the `has owner work` rank; `/y [lane]` filters. `needs_human` and
  the `❢` badge are unchanged. *Test: rank table unit test; existing needs_human tests untouched.*
- **OWN-19** Detail Phases tab shows the Owner block of 3.2 with expandable groups, the
  `closes after` header, and collapsed Old debt / Acknowledged / Todos folds.
- **OWN-20** Inbox screen per 3.3: lane filters `1-4`/`a`, ordering rule, expand, `g` go to project,
  `o` open file, `y` copy checklist. Legible at 100x30.
- **OWN-21** `v` runs the group's command through `multiplexer::launch` (focus an existing session
  instead when one is live); the argv is built only from the validated phase token and workstream
  name via `ExecArgv::new`.
- **OWN-22** `A` launches `gsd-tools audit acknowledge …` after a confirm popup; the app itself
  writes nothing into the project. The row disappears on the next watcher refresh.
- **OWN-23** Help overlay, README ("Owner inbox" section plus the `inbox` schema) updated.

## 6. Phase split

| Phase | Goal | Requirements | Depends on | Verification |
|---|---|---|---|---|
| **34 Owner items reader + `inbox` CLI** | Native per-item parsing, lanes, dedup, acknowledged, archived, conformance oracle, `inbox --json` and text | OWN-01..07, 09, 11..15 | none | **Automatic** (oracle, fixtures, golden output) |
| **35 Decisions, code-needed and gates** | Debug, handoff, context questions, gaps_found, todos count; milestone gate; into `inbox` and `status` | OWN-08, 10 (+ extend 13/15) | 34 | **Automatic** |
| **36 Dashboard and detail** | `You` column, summary line, sort rank, `/y` filter, Owner block | OWN-16..19, 23 (part) | 35 | Automatic snapshots **plus human UAT** (is the cell readable, is it quiet enough) |
| **37 Inbox screen and actions** | Cross-project screen, run command, acknowledge, copy checklist | OWN-20..23 | 36 | Automatic snapshots **plus human UAT** (triage flow, launch into tmux/herdr) |

Success criteria
- **34**: on the owner's machine `gsd-meta-manager inbox` answers "find the missing UATs" for all
  20 projects in one call; wordoclock reports 18 open checks, not 39 or 41; the oracle test is green
  in the container dry run; projects without a phases directory (bitcreed.us, starlink-rv-tracker,
  usbeehive) return empty groups, not an error. This phase alone removes the recurring Claude request.
- **35**: `inbox --json --lane decide` lists sentriq's 5 awaiting debug sessions and usbee's paused
  handoff; `closes_after` for wordoclock names phases 20-25 with the right lane totals.
- **36**: at 120 columns the owner can read off the dashboard which three projects to test first; a
  project with only blocked or archived items shows no lane text; no existing snapshot test changes
  except for the added column.
- **37**: from the dashboard, `i`, `j`, `v` starts `/gsd-verify-work N` in the right project in ≤ 3
  keys; after UAT passes the row disappears without a restart.

Human UAT for 36 and 37 (keep to these five checks): cell wording, column at 100/140 columns,
ordering in the inbox, launch into the real multiplexer, acknowledge round trip on a throwaway
project.

In scope now: everything above. Deferred (record as `OWN-F01..`): STATE.md Blockers/Concerns
parsing; heuristic "this UAT row is really a ruling" detection; todo triage UI beyond a count;
seeds and threads; native acknowledge writer; per-item pass/fail entry from the TUI (that is
`/gsd-verify-work`'s job); desktop notifications; a daily digest file.

## 7. Open decisions, with defaults the pipeline should take

| # | Decision | Default |
|---|---|---|
| D1 | Widen `needs_human` / `❢` to owner work? | **No.** 11 of 20 projects would light up. Use the `You` text cell and a separate sort rank. |
| D2 | Where do "fix awaiting confirmation" debug sessions go: Test or Decide? | **Decide** (as briefed). The row text says "awaiting your confirmation" and the command is `/gsd-debug`. |
| D3 | Dedup when UAT `source:` does not name the VERIFICATION | **Strict**: no supersede, show both under the same phase. |
| D4 | Archived debt on the dashboard | Dim `(+N old)` only; full list in inbox fold and `--include archived`. |
| D5 | New detail tab vs block | **Block on the Phases tab** (Phase 24 just consolidated tabs to eight). |
| D6 | Separate `inbox` subcommand vs growing `status` | **Separate** `inbox` (item text is large); `status` gets counts only. |
| D7 | Acknowledge writer | **Launch gsd-core's `audit acknowledge`**; no native write (keeps "non-intrusive"). |
| D8 | `unknown` rows without a result in terminal files | Hidden and counted; rows with `failed`/`partial` go to Fix. |
| D9 | Stale threshold | 30 days, `owner.stale_days` in the app config. |
| D10 | Keys | `i` inbox (free on the dashboard today), `/y` filter, `v` run, `A` acknowledge, `y` copy. |
| D11 | Open-question bullets marked RESOLVED/ANSWERED | Filtered out by our layer even though the oracle counts them. |
| D12 | Milestone and version | v2.5.0 "Owner Inbox", phases 34-37, coarse granularity as configured. |

## 8. Upstream gsd-core issues worth reporting (1.21.0)

1. **verify-work leaves the source VERIFICATION at `human_needed`** after seeding `NN-UAT.md` from
   it, so `audit-uat` reports every check twice (61 of 568 here; wordoclock 39 vs 18). Fix: audit
   should drop a VERIFICATION listed in a same-phase UAT `source:`, or verify-work should mark it.
2. **`## Gaps` entries are counted next to their own `result: issue` row**, and `status: failed`
   has no branch in `categorizeItem`, so they land in `unknown` (mailbot ph 09: 10 issues become 24
   items, 14 of them `unknown`).
3. **Prose in `## Gaps` becomes an item**: "None. All 6 UAT tests pass against the live Railway
   deploy." is reported as an `unknown` open item in a `status: passed` file (shopify ph 15).
4. **Resolution tokens fall to `unknown`**: `result: assumed_pass`, `closed`, `failed`, `partial`.
5. **`deferred-items.md` over-parses prose**: two files yield 301 of 325 entries, including bullets
   whose text says "closed". Needs a stricter entry grammar or a per-file cap in the summary.
6. **`uat-predicate.cjs` `BLOCKING_UAT_FM_STATUSES` omits `testing`**, the template's own in-flight
   status (`templates/UAT.md:11`); only the item scan catches it.
7. **`audit-uat` hard-errors** with "No phases directory found" on a project that has `.planning/`
   but no phases yet (3 of 20), instead of returning an empty result.
8. **`deriveOpenQuestions` counts answered questions**: bullets marked RESOLVED/ANSWERED under
   `## Open Questions` stay open forever (mailbot 06-CONTEXT: all three are answered).
9. **Debug status drift**: agents write `awaiting-user` where the template says
   `awaiting_human_verify`; no validation on write.
