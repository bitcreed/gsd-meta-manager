---
phase: quick-260926-gtn
plan: 01
type: execute
wave: 4
depends_on: ["260926-gtm"]
files_modified:
  - src/state_reader/state_md.rs
  - src/state_reader/disk_status.rs
  - src/driver/router.rs
  - tests/driver_router_conformance.rs
  - src/ui/screens/detail.rs
  - src/agents/fixers.rs
  - README.md
  - docs/GETTING-STARTED.md
autonomous: true
requirements: [QUICK-260926-gtn]

must_haves:
  truths:
    - "Some phases have a *-VERIFICATION.md whose frontmatter block is closed but is not parseable YAML (the real-world shape is `re_verification: true` followed by indented keys). They read VerificationStatus::Unparseable and DiskStatus::Executed, never Complete. The router parks them with `gate_verification_unparseable` rather than GoalMet. This matches GSD 1.14.0 and 1.15.0 init.manager, which both emit action `verify` for them."
    - "A *-VERIFICATION.md whose leading `---` block is never closed reads VerificationStatus::Missing with has_verification true, which gives gate_stale_check_indeterminate. It never reads Passed. This matches upstream extractFrontmatter, which returns {} for an unterminated block, giving `missing`."
    - "A literal `status: unparseable` reads VerificationStatus::Unparseable, because 1.15.0 accepts it as a routing-table key."
    - "The Phases tab shows one `Verify:` line under the stage block whenever the selected phase's verification is present and not passed. A stale report names `/gsd:execute-phase N` and never names verify-work. An unparseable report says to fix the YAML in the VERIFICATION.md and names no `/gsd:` command. human_needed, gaps_found, unknown and a status-less artifact each carry the command GSD 1.15.0 assigns them. A phase with no verification artifact, or one that passed, draws no such line."
    - "`NN-REVIEW-DISPOSITION.md` is never classified as a code review. On its own it leaves has_review false, and agents::fixers::is_code_review_name rejects it. Its table rows yield open/fixed/skipped/deferred counts. When a REVIEW.md is present, those counts render beside `✓Code Review` on the Phases tab Checks line as `(N open, M deferred)`."
    - "Across every *-VERIFICATION.md under ~/projects/**/.planning (337 files when this plan was written), the set of phase dirs the app reads as Unparseable equals the set the 1.15.0 oracle's `frontmatter get` flags. It was exactly .planning/milestones/v1.1-phases/06-read-only-views when measured."
    - "The router conformance test passes against the local 1.14.0 oracle AND the 1.15.0 oracle. `rtk proxy cargo test --no-fail-fast` fails on nothing except the src/envelope/policy.rs git-version witness. `rtk proxy cargo clippy --all-targets -- -D warnings` is clean."
  artifacts:
    - path: "src/state_reader/state_md.rs"
      provides: "pub(crate) frontmatter_block_parses: the single serde_yml parse site, raw parse then the existing one-shot repair_frontmatter_block reparse"
      contains: "fn frontmatter_block_parses"
    - path: "src/state_reader/disk_status.rs"
      provides: "VerificationStatus::Unparseable; closed/unterminated-aware read_verification_status; ReviewDisposition + review_disposition field + ledger reader; ledger filename arm ahead of the REVIEW.md arm"
      contains: "REVIEW-DISPOSITION.md"
    - path: "src/driver/router.rs"
      provides: "RouterReason::GateVerificationUnparseable / REASON_GATE_VERIFICATION_UNPARSEABLE and its verification_gate arm"
      contains: "gate_verification_unparseable"
    - path: "tests/driver_router_conformance.rs"
      provides: "fixtures executed_unparseable and executed_unterminated declared Upstream::VerifyGate; name-to-state derivation extended"
      contains: "executed_unparseable"
    - path: "src/ui/screens/detail.rs"
      provides: "verification hint line in render_pipeline_tab head; ledger counts on stage_block_lines' Checks line"
      contains: "execute-phase"
    - path: "src/agents/fixers.rs"
      provides: "negative case: 12-REVIEW-DISPOSITION.md is not the phase's code review"
      contains: "REVIEW-DISPOSITION"
    - path: "docs/GETTING-STARTED.md"
      provides: "Phases-tab text names the Verify line and the ledger counts"
      contains: "REVIEW-DISPOSITION"
  key_links:
    - from: "disk_status::read_verification_status"
      to: "state_md::frontmatter_block_parses"
      via: "closed leading block text -> parse check -> Unparseable"
      pattern: "frontmatter_block_parses"
    - from: "driver::router::verification_gate"
      to: "RouterReason::GateVerificationUnparseable"
      via: "exhaustive match on VerificationStatus"
      pattern: "VerificationStatus::Unparseable"
    - from: "detail.rs render_pipeline_tab head"
      to: "DiskInference.verification_status / has_verification"
      via: "pure verification-hint builder, fit to cols, phase number through shown()"
      pattern: "Verify:"
    - from: "detail.rs stage_block_lines Checks line"
      to: "DiskInference.review_disposition"
      via: "counts span appended after the Code Review token"
      pattern: "review_disposition"
---

<objective>
Surface the gsd-core 1.15.0 review and verification state in the manager for batch item C (quick id 260926-gtn), and keep the router conformant with GSD's own router while doing it.

Purpose: 1.15.0 adds three artifact-level signals the manager currently cannot see or misreads.
(1) `status: unparseable` (upstream #4806, src/verification.cts:131-139 and :1040-1049) is a report the verifier wrote but whose YAML is broken. Today this reader line-scans such a file. If a `status: passed` line survives the breakage, the reader reports Complete/GoalMet, while upstream (1.14.0 AND 1.15.0) reports executed with action `verify`. That is a fail-open divergence on the DRIVE-05 gate. It was proven by oracle probe (see context).
(2) A stale report's remedy moved from verify-work to execute-phase (upstream #4682, verification.cts:113-123 and :1140-1147).
(3) The per-finding code-review disposition ledger `{PADDED}-REVIEW-DISPOSITION.md` (execute-phase/steps/code-review-disposition.md:145-146) records which review findings are still open or were deferred.

Output: reader, router arm, conformance fixtures, Phases-tab display, README/GETTING-STARTED text, and a SUMMARY with every INFERRED decision listed for audit.

Routing answer the orchestrator asked for (verified, record it in SUMMARY): 1.15.0 init.manager (src/init.cts ~3216) still emits action `verify` with `command = verification_next_command` for every `executed` phase. Only the command changed: stale moved to execute-phase, and unparseable has an empty command. This router never auto-selects a verify command (the declared Upstream::VerifyGate divergence), so neither change moves a routing decision. No 1.15.0 router, init or progress code reads the disposition ledger (git grep: no consumer outside the workflow step), so it is display-only. The one router-visible change is closing the pre-existing fail-open above. Conformance fixtures prove it.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

Execution order is mandated sequential on the primary checkout. The order is gtk, then gtl, then gtm, then THIS item. There are no worktrees and no agent branches. Never push.
260926-gtk edited src/ui/screens/detail.rs (Defaults-tab rows). 260926-gtm edited tests/driver_router_conformance.rs (Oracle::resolve Codex candidates, GSD_RUNTIME=claude on the oracle child).
Edit by SYMBOL (function or const names below), never by the line numbers quoted here.
The human is unavailable. Decide from these artifacts and list each decision as `INFERRED:` in the SUMMARY.

Upstream reads (never check out a branch there; use `git -C ~/projects/node/gsd-core show upstream/release-1.15.0:<path>`):
- src/verification.cts: VERIFICATION_ROUTING_TABLE (~:91-146), readVerificationStatus unparseable branch (~:1036-1049), stale projection to execute-phase (~:1140-1147).
- src/frontmatter.cts: frontmatterRegion/extractFrontmatter (~:690-740). An unterminated block gives {}, and a YAML failure gives the FRONTMATTER_UNPARSEABLE marker. Also repairAmbiguousColonValues (~:511-523) and refuseAnchorsAndAliases (~:204).
- gsd-core/workflows/execute-phase/steps/code-review-disposition.md: path construction (:145-146), prior-row regex (:390), and the ledger render (:974-999).
- src/init.cts: the recommended_actions loop (~:3211-3223). `executed` produces action `verify`.

Oracles:
- Local 1.14.0: ~/.claude/gsd-core/bin/gsd-tools.cjs (what `cargo test` resolves; CI pins 1.14.0 via scripts/install-conformance-oracle.sh).
- 1.15.0 build: /tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/9e776af9-6a4a-4326-9fd5-76d0bedeccfe/scratchpad/gsd115/gsd-core/bin/gsd-tools.cjs, with fake HOME .../scratchpad/fakehome (its .claude/gsd-core links to the 1.15.0 build).
  - Run the conformance test against it with `HOME=<fakehome> CARGO_HOME=/home/blk/.cargo RUSTUP_HOME=/home/blk/.rustup rtk proxy cargo test --test driver_router_conformance`. The command was measured working with 3/3 passing before this item.
  - Per-file frontmatter oracle: `HOME=<fakehome> node <gsd115>/gsd-core/bin/gsd-tools.cjs frontmatter get <file> --field status`. It prints a JSON `error` containing "not parseable" for an unparseable block.

Probe results measured at planning time (1.14.0 = local, 1.15.0 = fakehome; tree = ROADMAP with Phase 01, one PLAN + one SUMMARY, one 01-VERIFICATION.md):

| VERIFICATION.md shape | 1.14.0 init.manager | 1.15.0 init.manager | this reader today |
|---|---|---|---|
| `status: passed` + `score: [unclosed` | executed / missing / verify | executed / unparseable / verify (command "") | Passed -> Complete (DIVERGES) |
| `status: passed` + indented `  bad: indent` line | executed / missing / verify | executed / unparseable / verify | Passed -> Complete (DIVERGES) |
| `status: passed` + tab-indented line | n/a | executed / unparseable / verify | Passed -> Complete (DIVERGES) |
| unterminated block, `status: passed` | n/a | executed / missing / verify | Passed -> Complete (DIVERGES) |
| literal `status: unparseable` | executed / unknown / verify | executed / unparseable / verify | Unknown -> parks |
| literal `status: stale` | executed / unknown / verify | executed / unknown / verify | Stale -> parks |
| `score: 5/5: all verified` (ambiguous colon) | complete / passed | complete / passed | Passed (must STAY passed) |
| duplicate `status:` key | n/a | complete / passed | Passed (must STAY passed) |
| anchor `&x` / alias `*x` | n/a | executed / unparseable | serde_yml accepts (named residual) |

serde_yml 0.0.13 (a re-export shim over noyalib 0.0.5) was probed into `serde_yml::Value`:
- Errors on: `[unclosed`, bad indent, tab indent, and `5/5: all verified` (so the repair pass is REQUIRED).
- Accepts: duplicate keys, anchors, lists, and an empty block.
- A billion-laughs block ends in "alias expansion limit exceeded" after ~2.4 s in release. It runs on the refresh `spawn_blocking` thread (src/app.rs schedule_reparse), which is the same exposure STATE.md parsing already has.

Across ~/projects, 337 *-VERIFICATION.md files exist. The 1.15.0 per-file oracle flags exactly ONE as unparseable: /home/blk/projects/rust/gsd-meta-manager/.planning/milestones/v1.1-phases/06-read-only-views/06-VERIFICATION.md (a `re_verification: true` line followed by indented child keys). No REVIEW-DISPOSITION.md exists anywhere locally yet, so ledger tests use the synthetic shape below.

Out of scope, recorded for the SUMMARY as a follow-up and not built here: upstream's fingerprint/mtime staleness scan (verification.cts `covered_files`/`covered_digest`, SUMMARY-mtime fallback) predates 1.15.0 and is not modelled by this reader. On this repo both oracles read phases 19/24/25 as `stale`, while the app reads their literal statuses. That is a pre-existing divergence, and a conformance fixture declaring a mismatched `covered_digest` would expose it. The Stale hint therefore fires only for a literal `status: stale` until that scan is ported.

<interfaces>
Ledger shape (rendered by code-review-disposition.md:979-999; the table is the human-edit surface, so a hand-set `deferred` makes the frontmatter `open:` stale; count from TABLE rows):

    ---
    phase: 05
    review: 05-REVIEW.md
    titles: json
    findings:
      - id: CR-01
        severity: critical
        disposition: open
        title: "Parser: loses data"
    open: 2
    total: 4
    recorded: 2026-09-26T10:00:00.000Z
    ---

    # Phase 05: Code Review Disposition

    | Finding | Severity | Disposition | Source |
    |---------|----------|-------------|--------|
    | CR-01 | critical | open | - |
    | WR-02 | warning | fixed | 05-REVIEW-FIX.md |
    | WR-03 | warning | deferred | waiting on team A \| team B |
    | IN-01 | info | skipped | 05-REVIEW-FIX.md (not in the current review) |

    Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.

The expected counts for that sample are open 1, fixed 1, skipped 1, deferred 1, total 4. The frontmatter says `open: 2`, and the table wins.

Upstream prior-row regex (:390), which is the row grammar to mirror:
`^\|\s*((?:CR|BL|WR|IN)-\d+)\s*\|\s*([^|]*?)\s*\|\s*(open|fixed|skipped|deferred)\s*\|\s*(.*?)\s*\|?\s*$`
The enum is case-sensitive. A row whose id cell matches but whose disposition cell is outside the enum falls back to `open`, which is upstream's safe default.

Existing helpers to reuse (read, do not re-implement):
- disk_status.rs: leading_frontmatter_value (byte-zero `---`, column-zero keys), split_markdown_row, is_alignment_row, the sorted-first tie-break idiom in read_verification_status/read_uat_status, and the `name == "X.md" || name.ends_with("-X.md")` arm style in infer_disk_status.
- state_md.rs: repair_frontmatter_block (private, one conservative quote-in-place pass that mirrors upstream repairAmbiguousColonValues) and the read_frontmatter serde_yml call shape.
- router.rs: RouterReason (closed taxonomy, REASON_* constants, as_str, ALL), verification_gate, test helpers executed_with / state_with.
- detail.rs: shown() (the terminal escape for third-party text), fit_spans, fit_cells, render_pipeline_tab head assembly, stage_block_lines, substage_table, and the tab's hint spelling `/gsd:plan-phase {}` in render_waves_pane.
- fixers.rs: is_code_review_name, and the test only_the_phases_own_code_review_is_the_review.
</interfaces>
</context>

<tasks>

<task type="tracer">
  <name>Task 1: Unparseable/unterminated VERIFICATION.md end-to-end: reader, router gate, conformance, Phases-tab Verify line</name>
  <files>src/state_reader/state_md.rs, src/state_reader/disk_status.rs, src/driver/router.rs, tests/driver_router_conformance.rs, src/ui/screens/detail.rs</files>
  <precondition>The 1.15.0 oracle file .../scratchpad/gsd115/gsd-core/bin/gsd-tools.cjs exists. If it is absent, run every 1.15.0 check below against the local 1.14.0 oracle only, and mark the 1.15.0 half UNVERIFIED in the SUMMARY. Do not check out branches in ~/projects/node/gsd-core to rebuild it.</precondition>
  <action>
RED FIRST (conformance is the evidence that the router change is required and not a preference):
- In tests/driver_router_conformance.rs, add two entries to the FIXTURES const, both declared `Upstream::VerifyGate`. Each builds skeleton + plan "01" + summary "01" + a hand-written 01-VERIFICATION.md.
  - `executed_unparseable`: a closed block carrying `phase: 01`, `status: passed`, then `re_verification: true` followed by two indented child keys. This is the real-world shape of v1.1-phases/06's 06-VERIFICATION.md.
  - `executed_unterminated`: an opening `---`, `phase: 01`, `status: passed`, then a blank line and a `# Verification` body, with no closing `---`.
- In every_fixture_reaches_the_state_it_is_named_for, extend the fixture-name-to-state derivation the same way it already strips `_gaps` / `_human`, so both new names expect `executed`.
- Run `rtk proxy cargo test --test driver_router_conformance` and confirm both new fixtures FAIL today, in the oracle-free test and the oracle test alike. Record the failure lines in the SUMMARY.

READER (state_md.rs, disk_status.rs):
- In state_md.rs add `pub(crate) fn frontmatter_block_parses(yaml: &str) -> bool`, the ONE serde_yml parse site that disk_status uses.
  - It returns true when `serde_yml::from_str::<serde_yml::Value>(yaml)` succeeds, or when repair_frontmatter_block(yaml) yields a rewrite that then parses as a Value. Any Value counts, including Null for an empty block, because upstream treats a non-mapping root as an empty mapping, not as unparseable.
  - Same single repair attempt as read_frontmatter, not a loop.
  - Doc-comment the upstream mirror (loadWithAmbiguousColonRepair) and the two named residuals. First, anchors/aliases and the U+E000 sentinel are refused upstream but accepted here. Second, a YAML bomb is bounded by serde_yml's "alias expansion limit exceeded" (~2.4 s release, measured) on the refresh spawn_blocking thread, the same exposure STATE.md already has.
  - Do not add a crate. Do not change read_frontmatter's behaviour.
- In disk_status.rs:
  - Add `VerificationStatus::Unparseable`, inserted after Stale. from_raw maps the case-insensitive literal `unparseable` to it (1.15.0 accepts the literal as a table key; probe row 5). as_str returns "unparseable". is_passed stays Passed-only.
  - Rewrite the enum doc to seven values matching upstream 1.15.0 VERIFICATION_ROUTING_TABLE (cite #4806).
  - Correct the Stale variant's doc. It is read only from a literal `status: stale`; upstream's fingerprint/mtime staleness scan is not modelled (see context).
- Add a private helper that classifies a document's leading block using exactly leading_frontmatter_value's byte-zero rule. The first line trimmed must be `---`, and the block ends at the first later line whose trimmed form is `---`. It returns absent, unterminated, or closed together with the text of the lines strictly between the fences joined by newline. It must handle CRLF (str::lines already strips `\r`).
- Rework read_verification_status:
  - Keep the same sorted-first file choice.
  - An unreadable file, absent block, or unterminated block gives Missing. Unterminated means upstream's extractFrontmatter returns {}, i.e. `missing`, so a body `status:` line can no longer be read (the DEFECT.FRONTMATTER-SCALAR-BROAD-GREP class).
  - A closed block where frontmatter_block_parses is false gives Unparseable.
  - Otherwise, the existing leading_frontmatter_value "status" scan through from_raw.
- Leave leading_frontmatter_value itself and every other caller (UAT, plans, summaries) unchanged.
- Unit tests in disk_status.rs's test module, each a closed-block content string or a tempdir phase dir via infer_disk_status:
  - The `[unclosed`, bad-indent, tab-indent and `re_verification: true` + indented-children shapes give Unparseable, with status Executed when plan+summary are present, never Complete.
  - Unterminated with `status: passed` gives Missing with has_verification true.
  - The literal `unparseable` gives Unparseable.
  - The ambiguous-colon `score: 5/5: all verified` shape and the duplicate-status-key shape STAY Passed. Include a comment naming the probe table.
  - A closed empty block gives Missing and not Unparseable.
  - A body `status: passed` below a correctly closed block with no status key still gives Missing (existing anchor behaviour preserved).

ROUTER (router.rs):
- Add `RouterReason::GateVerificationUnparseable` with `REASON_GATE_VERIFICATION_UNPARSEABLE = "gate_verification_unparseable"`. Place it after GateVerificationUnknown in the enum, in as_str and in ALL.
- Its doc says it mirrors upstream 1.15.0's `unparseable` table key 1:1, the way G1-G5 mirror theirs.
- INFERRED: a named arm and not a fold into G6. After this change G6 means "artifact present, block valid or unterminated, no status", which is upstream's `missing`, and unparseable is upstream's own distinct key. The journal should say which one fired.
- Add the verification_gate arm `VerificationStatus::Unparseable => Some(RouterReason::GateVerificationUnparseable)`.
- Update the verification_gate doc paragraph that explains the Missing split so it names the new split.
- Add Unparseable to the status lists in every_verification_status_outside_passed_yields_its_own_gate_reason, no_verification_status_other_than_passed_can_produce_goal_met and every_gate_reason_is_reachable_from_some_observable_state.

UI (detail.rs):
- Add a pure builder that renders nothing for Passed, or for Missing without an artifact. For every other state it returns one styled line: "  Verify: " (DarkGray), then the status word (Yellow), then a DarkGray remedy.
  - The remedy text is transcribed from upstream 1.15.0's VERIFICATION_ROUTING_TABLE next_command, spelled in this tab's existing `/gsd:` convention (INFERRED: matches the neighbouring `No plans yet — /gsd:plan-phase N` hint) with the phase number passed through shown().
  - Take each command from the 1.15.0 oracle, NOT from memory. Build one probe tree per status under the scratchpad (same shape as the conformance skeleton), run `query init.manager` with HOME=fakehome, and copy `phases[0].verification_next_command`. Record the table you measured in the SUMMARY.
  - Expected shape:
    - Stale: `/gsd:execute-phase N`, with a note that it re-runs the verifier and verify-work cannot refresh a stale report.
    - Unparseable: "fix the YAML frontmatter in the phase's VERIFICATION.md — re-running execute-phase cannot fix it". No `/gsd:` command.
    - HumanNeeded: `/gsd:verify-work N`.
    - GapsFound: `/gsd:plan-phase N --gaps`.
    - Unknown(v): the value v through shown(), then `/gsd:execute-phase N`.
    - Missing with an artifact present: "no status", then `/gsd:execute-phase N`.
- Push the line, cut with fit_spans to cols, into render_pipeline_tab's `head`. It goes after stage_block_lines and before the external-job line, so the Waves pane simply starts one row lower when the line is present.
- Do NOT change derive_all_stage_statuses or build_pipeline_line. INFERRED: the V ladder keeps meaning "a verification artifact concluded something". Unparseable counts as concluded because the verify step ran (upstream #4806's own reading), and the new line carries the failure. The Driver tab reuses the ladder unchanged.
- Tests:
  - Builder unit tests per status: the Stale text contains `execute-phase` and does not contain `verify-work`, and the Unparseable text contains `unparseable` and `YAML` and no `/gsd:`.
  - A render test at 100x32 modelled on waves_pane_tracer_is_visible_at_100x32 with an Unparseable phase: the Verify line sits at name_row + 4 and the Waves pane top at name_row + 5.
  - The existing tracer test (no verification) stays unchanged.

GREEN:
- Re-run the conformance test against 1.14.0 (plain) and 1.15.0 (HOME=fakehome form from context). Both new fixtures must now land in the VerifyGate arm with a `gate_` reason.

AGREEMENT SWEEP (throwaway, never committed):
- List every phase dir holding a *-VERIFICATION.md with `find /home/blk/projects -path '*/node_modules' -prune -o -path '*/.planning/*' -name '*VERIFICATION.md' -print`, taking the dirnames sorted unique, and write the list to a scratchpad file.
- Add a temporary tests/zz_gtn_sweep.rs that reads that list from an env var, calls gsd_meta_manager::state_reader::disk_status::infer_disk_status per dir, and prints the dirs whose verification_status is Unparseable. Adjust the path if the module is not pub, or use whatever public entry reaches infer_disk_status.
- Run it once, then DELETE the file.
- The printed set must equal the 1.15.0 per-file oracle's set, measured at planning as exactly .planning/milestones/v1.1-phases/06-read-only-views of this repo. Run the oracle loop again yourself and record both sets and the total file count in the SUMMARY.
- Any disagreement blocks the task. Fix the reader rather than the expectation, unless the only disagreement is the anchor/alias residual, which must then be named.
  </action>
  <verify>
    <automated>rtk proxy cargo test --test driver_router_conformance</automated>
    <automated>HOME=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/9e776af9-6a4a-4326-9fd5-76d0bedeccfe/scratchpad/fakehome CARGO_HOME=/home/blk/.cargo RUSTUP_HOME=/home/blk/.rustup rtk proxy cargo test --test driver_router_conformance</automated>
    <automated>rtk proxy cargo test --lib disk_status</automated>
    <automated>rtk proxy cargo test --lib router</automated>
    <automated>rtk proxy cargo test --lib detail</automated>
  </verify>
  <done>
- The two new fixtures failed before the reader change and pass after it, under both oracles. Their decisions are Park with a `gate_` reason: gate_verification_unparseable for executed_unparseable and gate_stale_check_indeterminate for executed_unterminated.
- The ambiguous-colon and duplicate-key shapes still read Passed.
- The Phases tab draws the Verify line for an Unparseable phase and for a Stale phase naming /gsd:execute-phase.
- The 337-file sweep agrees with the 1.15.0 oracle.
- tests/zz_gtn_sweep.rs no longer exists.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: REVIEW-DISPOSITION ledger: reader, never a REVIEW.md, open/deferred counts on the Checks line</name>
  <files>src/state_reader/disk_status.rs, src/ui/screens/detail.rs, src/agents/fixers.rs</files>
  <behavior>
    - A dir holding only 05-REVIEW-DISPOSITION.md gives has_review false and review_disposition Some. The ledger is not counted as a plan or summary and sets no other has_* flag.
    - 05-REVIEW.md + 05-REVIEW-DISPOSITION.md gives has_review true, and the counts are taken from the ledger TABLE.
    - The interfaces sample gives open 1, fixed 1, skipped 1, deferred 1, total 4. The frontmatter `open: 2` is ignored.
    - A `| WR-07 | warning | Deferred | x |` row (disposition outside the case-sensitive enum) counts as open.
    - A duplicate id row counts once, first occurrence wins.
    - Header, alignment and prose rows, and rows inside a fenced block, are ignored. A `BL-01` id counts.
    - A ledger with zero finding rows, an unreadable ledger, or none at all gives review_disposition None.
    - A ledger larger than the read cap is read only up to the cap and still yields the rows inside it, with no panic and no full read.
    - fixers::is_code_review_name("12-REVIEW-DISPOSITION.md", phase 12) is false.
    - With has_review and a ledger of open 2, deferred 1, the Checks line contains `✓Code Review (2 open, 1 deferred)`. Without a ledger it is exactly as today. With a ledger but no REVIEW.md, no counts are drawn.
  </behavior>
  <action>
disk_status.rs:
- Add `pub struct ReviewDisposition { open, fixed, skipped, deferred: u32 }` deriving Debug, Clone, Copy, Default, PartialEq and Eq, with a `total()` method.
- Add `pub review_disposition: Option<ReviewDisposition>` to DiskInference next to has_review. Its doc explains three things. The file is upstream 1.15.0's per-finding ledger, a sibling of REVIEW.md written by execute-phase's code_review_gate and code-review-fix's record_disposition. The TABLE is authoritative because it is the surface a human edits, and the frontmatter `open:` goes stale on a hand-set deferral. It is display-only: no router or init consumer exists upstream.
- In infer_disk_status, add an arm matching `REVIEW-DISPOSITION.md` or a `-REVIEW-DISPOSITION.md` suffix. It collects the name and `continue`s, and it is placed BEFORE the UI-REVIEW / EVAL-REVIEW / REVIEW arms, with a comment that it must never set has_review.
- After the scan, read the sorted-first ledger (the existing tie-break idiom). Use a bounded read, reusing a 256 KiB cap constant; mirror fixers::REVIEW_READ_CAP by `take`, and use from_utf8_lossy.
- Parse with a private fn:
  - Toggle a fence flag on lines whose trimmed form starts with three backticks or three tildes, and skip lines inside a fence.
  - For each line starting with `|`, split with split_markdown_row.
  - A row counts when cell 0 trimmed matches `(CR|BL|WR|IN)-\d+` exactly (regex crate, OnceLock, static pattern).
  - The disposition is cell 2 trimmed, compared case-sensitively against open/fixed/skipped/deferred, and anything else is open (upstream :390 safe default).
  - Dedupe by id with first occurrence winning. A result with zero counted rows is None.
- Add the Task-1-style unit tests from <behavior>, plus the naming test beside test_eval_review_detected.

fixers.rs:
- Add "12-REVIEW-DISPOSITION.md" to the negative list in only_the_phases_own_code_review_is_the_review. No production change is needed: strip_suffix("-REVIEW.md") already rejects it, and the test pins that.
- INFERRED: the fixer estimate (`N fixers · ~x/y fixed`) is NOT fed from the ledger. It is a live-run numerator from `fix(NN)` commit subjects over the current REVIEW.md, while the ledger is written AFTER a fix pass (code-review-fix record_disposition), so mixing them would put a previous pass's record into a live estimate. The ladder widths are also pinned by tests. State this in the SUMMARY.

detail.rs, in stage_block_lines where the ✓ tokens are pushed:
- Immediately after the `✓Code Review` token, when inf.review_disposition is Some, push a span ` ({open} open, {deferred} deferred)`. Always print both numbers. The span is Yellow when open > 0, else DarkGray. Integers only, no ledger text reaches the terminal.
- Nothing is drawn when has_review is false. INFERRED: upstream writes the ledger only after reading REVIEW.md (code-review-disposition.md:145-170, :276-282), so a ledger without its review is not a state upstream produces.
- substage_table keeps its shape.
- Add a stage_block_lines unit test for the with-ledger, without-ledger and ledger-without-review cases.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib disk_status</automated>
    <automated>rtk proxy cargo test --lib fixers</automated>
    <automated>rtk proxy cargo test --lib detail</automated>
  </verify>
  <done>
- The ledger never sets has_review and is never a code review for fixers.
- Counts come from table rows with upstream's enum and safe default.
- The Phases tab Checks line shows `(N open, M deferred)` beside ✓Code Review when a ledger is present, and is byte-identical to today otherwise.
- All the behavior cases above are pinned by tests.
  </done>
</task>

<task type="auto">
  <name>Task 3: User-visible docs + full gates</name>
  <files>README.md, docs/GETTING-STARTED.md</files>
  <action>
docs/GETTING-STARTED.md, "Reading the Waves pane" section, first paragraph:
- Extend the sentence about the stage summary. When the phase's verification is present but not passed, one `Verify:` line names the state and what to run next. A stale report means `/gsd:execute-phase N`, which re-runs the verifier. An unparseable VERIFICATION.md means fixing its YAML.
- The `Checks` line shows `(N open, M deferred)` beside `✓Code Review` when gsd-core 1.15's `NN-REVIEW-DISPOSITION.md` findings ledger is present.

README.md:
- In the Features bullet describing the Phases tab ("ladder and stage summary above a focusable Waves pane"), add a short clause: the stage summary flags non-passed or unparseable verification with the command to run, and shows the code-review ledger's open/deferred counts.
- Keep the surrounding wording and wrapping style.
- Do not touch the dashboard "Verification state is shown separately" sentence except to finish it with a period if it still lacks one.

Then run the whole-item gates on the primary checkout.
- Redirect each run's output to a scratchpad file and inspect that file with the Read tool. Do NOT pipe rtk-proxied output into grep (rtk filters downstream of proxy; see MEMORY).
- Run `rtk proxy cargo test --no-fail-fast`. The ONLY permitted failure is the src/envelope/policy.rs git-version witness.
- Run `rtk proxy cargo clippy --all-targets -- -D warnings`, which must exit 0.
- Run both conformance runs (1.14.0 plain and 1.15.0 via HOME=fakehome).
- Record suite counts (passed/failed/ignored) and the conformance line "N command comparisons, N declared divergences, N upstream-silent states, over N fixtures" (captured with --nocapture) in the SUMMARY.
- The SUMMARY also lists every INFERRED decision from Tasks 1-2, the measured command table, the sweep sets, the routing answer from <objective>, the named residuals (anchors/aliases, U+E000, BOM-prefixed files read as Missing while upstream strips the BOM, YAML-bomb latency) and the staleness follow-up.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings</automated>
    <automated>rtk proxy cargo test --test driver_router_conformance -- --nocapture</automated>
  </verify>
  <done>
- README and GETTING-STARTED describe the Verify line and the ledger counts.
- The full suite fails on nothing but the git-version witness, and clippy is clean.
- Conformance passes under both oracles with the two new fixtures counted as declared divergences.
- The SUMMARY carries the audit list.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| foreign project files -> reader | *-VERIFICATION.md and *-REVIEW-DISPOSITION.md come from arbitrary registered (possibly cloned, possibly agent-written) repositories |
| reader -> router | VerificationStatus drives the DRIVE-05 gate set, and a wrong Passed is a false GoalMet |
| reader -> terminal | status values and ledger content could carry escape sequences |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-gtn-01 | Elevation / Spoofing | disk_status::read_verification_status -> router::verification_gate | high | mitigate | Broken-YAML and unterminated blocks can no longer line-scan `status: passed`. They read Unparseable/Missing and park (gate_verification_unparseable / gate_stale_check_indeterminate). Pinned by the executed_unparseable / executed_unterminated conformance fixtures under both oracles, plus unit tests. |
| T-gtn-02 | Denial of Service | state_md::frontmatter_block_parses (YAML alias bomb) | low | accept | serde_yml's alias expansion limit ends it (~2.4 s release, measured). It runs on the refresh spawn_blocking thread, never the render loop, which is the same accepted exposure as STATE.md parsing. Named in the doc comment and the SUMMARY. |
| T-gtn-03 | Denial of Service | ledger read | low | mitigate | Bounded read (256 KiB cap via take) and a line-linear parser. A test covers the over-cap file. |
| T-gtn-04 | Tampering (terminal injection) | detail.rs Verify line / Checks counts | medium | mitigate | An Unknown status value and the phase number go through shown(). The ledger contributes integers only, and no id, severity or source text is rendered. |
| T-gtn-05 | Spoofing | anchors/aliases in VERIFICATION frontmatter | low | accept | Upstream refuses them (unparseable) while serde_yml accepts them, so an anchor-bearing `status: passed` reads Passed. There is no security delta: a writer able to plant that file can write a clean `status: passed`. Named residual. |
| T-gtn-06 | Tampering | ledger path selection | low | mitigate | Only fixed-suffix entry names from the phase dir's own read_dir, sorted-first. No path is built from file content. |
</threat_model>

<verification>
- Conformance (1.14.0 local): `rtk proxy cargo test --test driver_router_conformance`. It passes, and the two new fixtures are counted as declared divergences.
- Conformance (1.15.0): the HOME=fakehome form from context. It passes.
- Full suite: `rtk proxy cargo test --no-fail-fast`. The only failure permitted is src/envelope/policy.rs's git-version witness. Read raw result lines from a redirected file, never through a pipe.
- Lint: `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- Agreement sweep (Task 1): the app's Unparseable dir set equals the 1.15.0 per-file oracle's set, and the throwaway test file is deleted.
- Docs: README.md and docs/GETTING-STARTED.md mention the Verify line and the ledger counts.
</verification>

<success_criteria>
- The fail-open is closed: broken or unterminated VERIFICATION frontmatter can no longer produce Complete or GoalMet, as proven by conformance under both oracles.
- `unparseable` is modelled as upstream 1.15.0's seventh status, with its own named router reason.
- The Phases tab tells the user what to run for each non-passed verification state. Stale names execute-phase, and unparseable says fix the YAML.
- The REVIEW-DISPOSITION ledger is read, never mistaken for REVIEW.md, and its open/deferred counts are visible beside Code Review.
- No new crates, no router command-selection changes, and all gates are green except the known witness.
</success_criteria>

<output>
Create `.planning/quick/260926-gtn-surface-gsd-core-1-15-0-review-verification-state-read-revie/260926-gtn-SUMMARY.md` when done. It lists every INFERRED decision, the measured oracle command table, the sweep result, the routing answer, the named residuals and the staleness follow-up.
</output>
