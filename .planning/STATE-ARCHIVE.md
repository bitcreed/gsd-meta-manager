# STATE Archive

Pruned entries from STATE.md. Recoverable but no longer loaded into agent context.

## Pruned 2026-10-28 (phases 1-25, kept recent 3)

### Decisions

- [Phase 11]: HANDOFF file detection with non-empty content validation for pause badges
- [Phase 12]: Archive module in separate src/archive.rs file (not in detail.rs) for testability
- [Phase 12]: Abbreviated tab labels (5:Pipe, 7:Sess) to fit 8 tabs within 80 columns
- [Phase 13]: Strategy A (per-item isolated execution) recommended for v1.3 queue execution
- [Phase 13]: LLM-agnostic Executor trait interface for queue execution backends
- [Phase 19]: T-19-60 closed by resolving the effective program STRUCTURALLY: consume leading NAME=VALUE assignment words by the shell grammar, then find the first token whose BASENAME is in a closed GOVERNED_PROGRAMS set. No wrapper-name list is added anywhere — one is DELETED (NESTED_SHELLS) (19-11) — The set of things that can precede a program is open and unlistable (env, timeout, nohup, stdbuf, setsid, doas, busybox env, ...), so a list plus one row per name is green on the day it lands and silent on the seventh wrapper. The set of programs the envelope GOVERNS is closed and already defined by classify_git and pr_command_label (D-08). made-up-wrapper-9000 resolving identically to env is the proof the fix is over the class rather than over a list.
- [Phase 19]: GIT_CONFIG_COUNT=0 git push --force parks under hook_bypass_blocked, not force_push_blocked: the assignment is refused on its own account before resolution reaches the push (19-11) — One token does two things: it hides the command from layer 2 AND it stops core.hooksPath being injected, so no pre-push hook runs (D-09). Parking it under the reason --no-verify and core.hooksPath already use is where a later reader greps for a disarmed enforcement layer. ENVELOPE_ENV_KEYS is drift-pinned against cred::build_env_in, and the pin found GIT_SSH_COMMAND on its first run — a key the plan list omitted and whose reassignment would put the user ssh agent back within the run reach.
- [Phase 19]: The alphabet-absence control asserts wrapper names absent from PRODUCTION CODE (comments and the #[cfg(test)] module stripped), not from raw file bytes (19-12) — 8 of the 12 designated names appear in policy.rs raw text: 7 in the doc comments of GOVERNED_PROGRAMS and resolve_program, where they are listed precisely to record that the set of things which can precede a program is open and must NOT be enumerated, and made-up-wrapper-9000 4x in the unit-test module as 19-11's own class-level fixture. A literal raw-bytes reading would be RED today for the opposite of the reason the control exists. Stripping is the same fact 19-11's audit measured with grep -v '^\s*//'. Strengthened rather than relaxed: the 4 raw-absent names (unshare, firejail, torsocks, catchsegv) are additionally asserted against unprocessed bytes.
- [Phase 19]: Every absence control needs a POSITIVE control beside it: include_str! fails the build on a MISSING path but not on a WRONG-BUT-EXISTING one (19-12) — Point include_str! at a different source file, or at the same file twice, and every absence assertion passes having certified nothing. An absence assertion cannot distinguish "the name is not in this file" from "this is not the file I think it is". Each included file is now asserted to contain an anchor unique to it (fn resolve_program for policy.rs, fn classify_segments( for hooks.rs), absent from the other, present after stripping, ordered BEFORE the absence assertions. Proved fail-first by repointing HOOKS_SOURCE at policy.rs.
- [Phase 19]: X=git; env $X push --force is PERMITTED and is pinned as such: resolve_program step 7 closes the SAME-command-line binding, and a semicolon makes it a different command line (19-12) — The plan required the semicolon form to be REFUSED; measured exit 0. The plan's own rationale and resolve_program's doc both say step 7 refuses a binding "in the same command line", so the semicolon form is the T-19-74 residual by definition rather than a bound on it. Bound 2 is pinned as X=git env $X push --force (no semicolon, refused under envelope_assertion_failed) AND the semicolon form is pinned permitted in a dedicated boundary test, so the residual's edge cannot move outward OR inward unnoticed.
- [Phase 19]: Any phase-19 suite number must come from cargo test --no-fail-fast (19-12) — cargo test stops at the first failing test BINARY. driver_reattach fails (documented pre-existing pair) and envelope_* sorts after driver_*, so a plain run never executes any envelope test and reports 1245/2/13 no matter what was added. With --no-fail-fast the true total after 19-12 is 1470 passed / 2 failed / 13 ignored.
- [Phase 19]: T-19-60 is closed for the wrapper-operand sub-class only; T-19-86 registered, pinned and deferred
- [Phase 19]: /gsd-secure-phase 19 is NOT cleared by plan 19-13 alone — T-19-86 and T-19-87 remain open
- [Phase 19]: Rule A: the classifier decision region — every index reported by the scan the classifier itself runs, never a second scan
- [Phase 19]: T-19-87's two severed-prefix rows left RED on purpose, so Rule A and Rule B are each shown separately load-bearing
- [Phase 19]: T-19-91 measured, pinned and registered OPEN rather than closed — round discipline, and reflog/symbolic-ref have no second carrier
- [Phase 19]: Rule B is POSITIONAL — the substring/prefix/suffix/length formulation against ENVELOPE_ENV_KEYS stays withdrawn on two measurements (evadable by moving the split point; refuses every uppercase assignment)
- [Phase 19]: The word-splitting OPENER is excluded, which is what keeps command substitutions classified and ordinary shell running
- [Phase 19]: split_segments is DEFINED OVER split_segments_with_heads so a decision region comes from the same scan the classifier runs, never a second scan
- [Phase 19]: The false T-19-87 test was DELETED with a tombstone rather than re-worded; its two rows re-homed with corrected reason identifiers
- [Phase 19]: The bare-brace Rule B row is spelled without its binding, because the bound spelling is already refused under hook_bypass_blocked and would be a control that could not fail on its class
- [Phase 19]: 19-16: comma-list brace expansions produce N words, so only range spellings of the concatenated splice class assemble a real force push; comma spellings are corpus COVERAGE, labelled as such
- [Phase 19]: 19-16: git push {--force,origin} main is cwd-dependent — exit 0 in the envelope namespace, exit 2 outside it under an unrelated arm — so it is pinned with the repository passed explicitly
- [Phase 19]: The rule is INVERTED: a decision word must be provably LITERAL, so anything not provably literal is unresolvable and refuses
- [Phase 19]: Clause 2(b) reads what a splice PRODUCES over the WHOLE WORD, quote-removed, never what its alternatives are called
- [Phase 19]: A nested { inside an alternative is a fail-closed UNENUMERABLE trigger, not a recursion
- [Phase 19]: T-19-93 is closed in the tokenizer's literal-brace branch with neither forge scan changed
- [Phase 19]: The four FORGE rows assert a RESTORED COUNT, not a refusal — the SAFE-06 cap is bypassed UNCOUNTED rather than exceeded (19-18)
- [Phase 19]: Three rows whose post-fix verdict 19-18 cannot derive are RECORDED and never asserted: >$F, >*.log and {v}> (19-18)
- [Phase 19]: T-19-17r's bookkeeping gap recorded OUTSTANDING; no AR-19-13 row added and no acceptance made (19-18)
- [Phase 19]: 19-20: the callee's grammar is a THIRD corpus axis, not more entries in either shell axis — both existing axes are axes of bash and audit 6 verified that boundary closed
- [Phase 19]: 19-20: assert the POST-fix verdict wherever derivable so the row is red now and green after — no pin for the next plan to move, and no replacement exception needed
- [Phase 19]: 19-20: the anti-vacuity ratio floor is REPLACED by absolute per-file byte floors (40k/20k), not lowered — 428 bytes of comment headroom made it a documentation budget
- [Phase 19]: 19-21: an unestablished git verb slot is a refusal, not the next non-`-` word — the fail-closed treatment resolve_program's wrapper axis already had
- [Phase 19]: 19-21: git's option grammar is pinned against the installed git binary in a TEST, never consulted from the guard's path
- [Phase 19]: 19-22: the corpus for git's CONFIG RESOLUTION is written and RED before either rule exists — 9 failing names are 19-23's handoff contract
- [Phase 19]: 19-22: T-19-103 reproducers must sit on LAYER-2-PERMITTED bases; the five --force compositions are controls, fenced mechanically
- [Phase 19]: 19-22: a dotless -c key stays CONFINED — refusing it would turn round 7's whole generative property red behind CALLEE_KNOWN_LEADING_PREFIX
- [Phase 19]: 19-22: CONFIG_RESOLUTION_CLASSES is a FOURTH axis beside three byte-identical command-line axes — what the verb runs under, not the command line
- [Phase 19]: 19-22: GIT_CONFIG_NOSYSTEM recorded as a measured DEFEAT with an INERT harm — never a new threat ID, never a bypass
- [Phase 19]: A SIXTH class on the EXISTING config-resolution axis, not a fifth axis — the region moved, not the axis
- [Phase 19]: The shell-alias carve-out is git's own one-byte rule, fenced mechanically rather than by care
- [Phase 19]: T-19-108 will close only AS SCOPED; audit 7's !-bodied destructive pair is T-19-86 and stays open at high
- [Phase 19]: Option (b) adopted for T-19-108: option (a) rejected on three MEASURED grounds (second tokenizer, depth-3 recursion, carriers with no readable body)
- [Phase 19]: The re-parse value test reads the CARRIER as well as the first byte — measured, --config-env=alias.q='!EVIL' resolves /INCLUDE_WINS
- [Phase 19]: Region 2 gates on !is_read plus a VALUE WORD because is_write is blind to a dash-leading value
- [Phase 19]: T-19-110 registered OPEN and NOT fixed: widening is_write moves verdicts with no corpus able to fail on them
- [Phase 19]: 19-26: CONTROL_CARRIER_CLASSES is a genuine FIFTH axis — no existing axis property can be satisfied by a command reaching no governed program
- [Phase 19]: 19-26: C-15 (~/.config/gsd-meta-manager/config.json) is a DISTINCT route to SAFE-06 and gets control (e) — no rule; pr_cap_* deliberately NOT clamped
- [Phase 19]: 19-26: T-19-111 is registered SEPARATELY from T-19-86 and must be moved OUT of it — five mis-attribution sites enumerated
- [Phase 19]: 19-27: the carrier-operand rule — a segment naming an ABSOLUTE LITERAL path under envelope_dir_in(root, alias) is refused, raised ONCE before the resolution match; it fails OPEN in four named directions handed to no control
- [Phase 19]: 19-27: T-19-111 moved OUT of T-19-86 at all five sites, with no rule and no acceptance written for it
- [Phase 19]: 19-27: SECTION_ENVELOPE's Guaranteed cap clause repaired to a location fact making no completeness claim — C-15 is a fifth route to SAFE-06 and the deferred option (b) a sixth
- [Phase 19]: The redirection-target channel is a SEGMENT-borne fact, never a token in the stream — pinned falsifiable by segment-count assertions (19-28)
- [Phase 19]: The binary carrier C-10 is an EXACT-PATH boundary, not a directory prefix, because its directory is shared (19-28)
- [Phase 19]: The ledger size bound is DEADLINE-derived and must not be cap-derived, because the caps are unclamped (C-15) (19-28)
- [Phase 19]: T-19-115's tilde/glob/brace spellings get NO rule; the residue's arithmetic is corrected from four directions to seven (19-28)
- [Phase 19]: 19-29: the redirection target rides on the Segment, never as a token in the stream — proved by the SEGMENT-COUNT pins rather than argued
- [Phase 19]: 19-29: two protected paths, two boundary KINDS — the envelope directory as a PREFIX, the guard's own binary as an EXACT PATH
- [Phase 19]: 19-29: the credential control's criterion is 'git credential fill' failing closed, never 'git config --get-all', which still lists the helper
- [Phase 19]: 19-29: the ledger size bound is DEADLINE-derived and fails CLOSED; a cap-derived bound would be one the agent steers
- [Phase 19]: 19-30: the option-attachment boundary is /-anchored substrings of a LITERAL word, never a list of attachment characters (D-08 one level over)
- [Phase 19]: 19-30: containment is of the NORMALISED STRING, not the index — lexical_absolute_components strips leading ./ before testing for /
- [Phase 19]: The interior-path boundary is /-anchored substrings of a LITERAL word — not =, not a character list; containment is of the NORMALISED STRING, not of the index
- [Phase 19]: The residue is a CONDITION with NO count, handed to no pin, schedule or version witness
- [Phase 19]: credential.helper is denied by SECTION + FINAL COMPONENT, never a substring test
- [Phase 19]: The ledger KIND check goes inside the existing Ok arm; the link-following stat is kept
- [Phase 19]: cred.rs names the rule that acts rather than claiming a layer governs — prefer a claim that stays true to one that must be maintained
- [Phase 19]: T-19-122 NARROWED by a THIRD word class: a SECOND Segment field carrying LITERAL NON-PATHNAME redirection targets, chained at the ONE existing reading site. Design decided by which fenced pins each option turns RED.
- [Phase 19]: T-19-123 NARROWED by an ANCESTOR clause beside the PREFIX one, bounded at the envelope ROOT on ownership grounds. Grows the protected set by exactly one path; the residue above the root is registered, disclosed and UNACCEPTED.
- [Phase 19]: T-19-124 NARROWED: the candidate scan's work is now LINEAR in the word's length via suffix folds with structure sharing, with a fail-closed CANDIDATE_SCAN_WORK_CEILING a test DRIVES. Task 3 was NOT severed.
- [Phase 19]: Both authorised cross-fence pin moves performed with each reasoning REWRITTEN under WR-02 rather than deleted, and a STOP row added beside each.
- [Phase 19]: Audit 12's finish-line sentence is deliberately NOT written by plan 19-33; that is audit 13's judgement. /gsd-secure-phase 19 is NOT cleared.
- [Phase 24]: ROADMAP `Build phase` headings live in ProjectState.planned_phases, never phases — router/frontier stay GSD-conformant
- [Phase 24]: Roadmap list is a pure model (git-log lanes, transitive reduction with (implied via N), milestone bands with folds); cursor targets are keys, never row indices; Parallel = same wave
- [Phase 24]: RoadmapView splits side-by-side at >=100 cols, stacked below; stacked detail pane shrinks 9->6 rows and shortens the goal before edge lines
- [Phase 24]: Final tabs 1:Roadmap 2:Phases 3:Backlog 4:Git 5:Queue 6:Sess 7:Cfg 8:Docs + D:Drive; Archive is Docs › Milestones (`m`), switch_to_sub_view is the one arrival rule [INFERRED — audit]
- [Phase 24]: Roadmap footer left at 91/87 cols — trimming hints is a scope change, deferred
- [Phase 25]: 25-01: agent git counts live in worktrees::worktree_counts so every src/agents git call stays in worktrees.rs [inferred]
- [Phase 25]: 25-01: no-linked-worktree prefilter reports main_worktree=project_root, base_sha=None [inferred]
- [Phase 25]: 25-01: non-z porcelain fallback also decodes octal byte escapes (non-ASCII paths) [inferred]
- [Phase 25]: 25-02: an unparseable Claude meta is no data for its row; a mistyped field loses only itself [inferred]
- [Phase 25]: 25-02: read_meta_capped refuses symlinks and non-regular files (transcript redirect, FIFO hang) [inferred]
- [Phase 25]: 25-02: the MAX_AGENT_AGE_SECS session bound covers the whole second pass; the per-id worktree lookup is unbounded [inferred]
- [Phase 25]: 25-02: a live meta whose worktreePath names an already-enriched worktree attaches as a child [inferred]
- [Phase 25]: 25-03: commit-scope attribution routes through worktrees::commit_subjects so every src/agents git call stays in worktrees.rs [inferred]
- [Phase 25]: 25-03: worktree SUMMARY check excludes FIX/GAPCLOSURE summaries, matching main's Pass 2 pairing [inferred]
- [Phase 25]: 25-03: ladder drops P13 first (RESEARCH A3, deviates from D-C14 drop-from-right); Unknown-only plans read queued [inferred]
- [Phase 25]: 25-04: agent_views is pruned by per-scan wholesale replacement plus the registered-alias filter, not by prune_driver_maps [inferred]
- [Phase 25]: 25-04: DASHBOARD_HIGHLIGHT_SYMBOL is shared by dashboard_table and status_column_cells so the fit check cannot drift [inferred]
- [Phase 25]: 25-05: Sessions › Agents shares Sessions' tab index; m toggles through switch_to_sub_view; Enter/n inert on Agents [inferred details in SUMMARY]
- [Phase 25]: 25-05: agent_list_len is the one line count both render and scroll keys clamp against; ages floor-rounded from scanned_at
- [Phase 25]: 25-05: Docs and Sessions strips share two_sub_tab_strip; Agents footer adds only [m] sessions (prefix already has [j/k]) [inferred]
- [Phase 25]: 25-06: fixer phase from description 'phase NN' else own fix(NN) scope; denominator findings.total verbatim (A7, audit) [inferred]
- [Phase 25]: 25-07: orphans aged past MAX_AGENT_AGE_SECS (a day of inactivity) read Ended, not a new variant [inferred]
- [Phase 25]: 25-07: only Live/Idle (AgentLiveness::is_running) switch the Status-cell summary, the fixer estimate and the executor ladder on; Finished counts only inside a running run [inferred, CR-01]
- [Phase 25]: 25-07: active-phase vote is tiered - running rows first, Finished/Stalled fallback (WR-01 folded in) [inferred]

### Performance Metrics

| 14 | 4 | - | - |
| 15 | 8 | - | - |
| 16 | 6 | - | - |
| 17 | 8 | - | - |
| 18 | 11 | - | - |
| 24 | 7 | - | - |
| 25 | 7 | - | - |

## Session history archived 2026-10-28 (cleanup)

Moved verbatim from STATE.md by the 2026-10-28 project cleanup so STATE.md stays lean. Headings are demoted one level.
Historical lines saying "Phase 19's gap-closure loop is halted by user decision" (and the same for Phase 21) are
superseded: on 2026-10-28 Phases 19, 21, 22 and 23 were closed by supersession, and their loops will not be resumed.
The old frontmatter (milestone v1.15.0 / Workstream Visibility, a shipped tag) is preserved below too.

### Old frontmatter

```yaml
gsd_state_version: "1.0"
milestone: v1.15.0
current_phase: "28.1"
current_phase_name: Sandbox Egress & Agent-Channel Isolation
status: verifying
stopped_at: Phase 31 complete, ready to verify Phase 28.1
last_updated: "2026-10-28T21:28:22.018Z"
last_activity: 2026-10-28
last_activity_desc: Phase 31 complete, transitioned to Phase 28.1
state_head: 29034171f8dc1a30139e8910c3fe45559422c1ab
progress:
  total_phases: 10
  completed_phases: 15
  total_plans: 9
  completed_plans: 112
milestone_name: Workstream Visibility
```

### Current Position

Status: Phase complete — ready for verification
  Phase 30 (Driver Tab on DIRECTION.md) complete 2026-10-24: 9/9 plans, re-verified 6/6 after quick
  261023-v1s staled the report (direction.rs give-back exchange); UAT 2/2, SECURITY threats_open 0,
  regression gate green apart from the known local git-version witness. Re-review WR-10/WR-08/WR-09
  left open (advisory; PROJECT.md Active item). DTAB-01..04 complete. Current phase stays 28.1.
  Next: `/gsd-plan-phase 28.1` (or discuss first).

Earlier (2026-10-24):
  Phase 28 (Sandboxed Driver Launcher) complete 2026-10-24: 9/9 plans, verification passed 6/6,
  UAT 2/2 on agent evidence (auth setup; the macOS degraded-mode item was run on a real macOS 26.6.2
  host, the local docker-osx VM, with HEAD built natively) `[inferred — audit]`. LNCH-01..08 and the
  DRIVE-01 launcher half complete; SECURITY threats_open 0. The 2026-10-24 isolation audit (open
  network, $HOME denylist, abstract sockets, planted files) is owned by Phase 28.1.
  Next: `/gsd-plan-phase 28.1` (or discuss first), and `/gsd-verify-work 30` is still pending.

Earlier (2026-10-23):
Status: v1.15.0 shipped (interim release; v2.0 milestone still open)
  GSD workstreams (`.planning/workstreams/<dir>/`) surfaced across the TUI: debug session
  workstreams-not-surfaced (dashboard lines per workstream, lock-aware active phase, Detail Roadmap
  and Phases tabs) and quick batch 261023-05f (05g Roadmap header, 05h Docs tab, 05i agent/worktree
  details, 05j dashboard aggregate status, 05k `/` filter, 05l empty-roadmap band, 05m stale-lock
  fallback); quick 261022-wtw removed the `v` box view. Also shipped since v1.14.0: Phases 26, 26.1,
  27 and 29 complete, Phases 28 and 30 executed (verification pending), and quick tasks
  261019-foc/guw/hqu, 261020-f80/vxg, 261021-0jg/101/1id/1if.
  Next: `/gsd-verify-work 28`, then `/gsd-verify-work 30` `[inferred — audit]`.
  Re-tag: the first v1.15.0 publish (tag at 450713f) failed `cargo publish` verify because the
  `assets/` exclude dropped src/**/assets; nothing reached crates.io. Fixed by quick 261023-pxh
  (plus 261023-qs8 flaky race test, 261023-p51 container storage cap); v1.15.0 re-tagged on the
  commit after this note and force-pushed with operator approval. Container dry run: 6/6 gates PASS.
  Known flake left open: `editor_race_smoke` (tests/driverctl_race.rs:348), 11/200 in container.

Earlier (2026-10-21):
Status: Phase complete — ready for verification
  Phase 29 (Master Prompt & Run Templates) complete 2026-10-20: 8/8 plans, re-verification passed
  5/5 after the code-review fixes (01eb8c8), UAT 1/1 (operator sign-off accepting o1 ADJUST x2 and
  h2 H2-5 as a rubric setup flaw; no re-run). PRMT-01..05, DRIVE-03 and DRIVE-01 complete. Review
  ledger 0/12 open; SECURITY threats_open 0. Backlog 999.7 holds the next prompt bump
  (reconcile-first refusal routing + IN-01).
  Phase 28 (Sandboxed Driver Launcher) still reads 28-UAT.md `testing` and 28-VERIFICATION.md
  `human_needed`. Next: `/gsd-verify-work 28` `[inferred — audit]`.

Earlier (2026-10-19):
Status: Phase complete — ready for verification
  Phase 27 (driverctl & Driver Frontmatter) complete 2026-10-19: 6/6 plans, verification passed 4/4,
  security SECURED (0 open threats). DCTL-01..04 complete. The `stalled` status value carried from
  26.1 shipped (RunStatus::is_terminal covers stopped|done|crashed|stalled|parked).
  Open advisories: 27-REVIEW-DISPOSITION.md has CR-01 (critical: gap scan reads driver-plantable
  files with no type/size guard, under the store mutex), WR-01..09 and IN-01..03 all `open`.
  Progress: 146/150 plans (progress.bar).
  Next: `/gsd-discuss-phase 28` (no 28-CONTEXT.md yet). Consider `/gsd-code-review 27 --fix` for
  CR-01 first, since Phase 28's launcher runs `driverctl serve` `[inferred — audit]`.

Earlier (2026-10-18):
Status: Ready to plan
  Phase 26.1 complete 2026-10-18 (11/11 plans; verification passed after operator sign-off on the
  human item, 26.1-UAT.md test 1). VALID-01 proven on an organic Opus driver. VALID-02 accepted by the
  operator as ADJUST (final): launcher deny/kill is model-independent launcher code proven by the
  scripted Haiku run r261-l7 (L7-1..L7-6 PASS); the Opus driver self-parked at gap count 2 before
  round 3 (results/r261-run2b/LOG.md:100-101), so deny/kill were NOT-REACHED on Opus.
  Carried risk -> Phases 28/29: they must exercise launcher deny/kill under the real driver (recorded
  in ROADMAP.md on both phases).
  Follow-up for Phase 27 (from 26.1 planning): the driver status list/enum (SC3 frontmatter states)
  needs a `stalled` value.
  Open advisories (deferred, not blocking): 26.1 review warnings WR-10 and WR-11 remain `open`
  (26.1-REVIEW-DISPOSITION.md); IN-04 deferred.
  Next: `/gsd-discuss-phase 27` (or `/gsd-plan-phase 27`).

Earlier (2026-10-17):
Status: Ready to plan
  revised spike 003 as decimal Phase 26.1 BEFORE Phase 27 (option (a), redesign + re-run). Scope:
  (1) count gap rounds per verifier run so one verifier re-write cannot inflate the gap count or trip
  the cap; (2) keep the headless driver alive until its dispatched worker reports, or make the
  launcher tell that case apart from a crash; (3) close T-26-11 (acceptance.py PASS when every check
  is undetermined) and cheap 26-REVIEW warnings; (4) re-run Opus Run 1 (and Run 2 if the rubric
  allows) to prove VALID-01 / VALID-02. Spend cap `[inferred — audit]`: $52.19, the unspent remainder
  of spike 003's $60 ceiling. 26.1 gates Phases 27-31. Next: `/gsd-execute-phase 26.1`.
  Planner `[inferred — audit]` items (in the 26.1 plans): Run 1 retry counter restarts for 26.1;
  NO-GO (d)/(e) evaluated only on a VALID-seed Run 1; prompt v0.4 (worker brief + Loop step 4 only);
  per-verifier-run counting approximated by a 45 s mtime coalescing window; crashed/stalled runs
  count as SEED STALLED; Run 2 skipped if Run 1 already settles NO-GO. Forward note: Phase 27's
  driver status enum needs a `stalled` value.

Earlier (2026-10-16):
Status: Ready to plan Phase 27, but BLOCKED. Phase 26 completed 2026-10-16 (3/3 plans; verification
  passed against the plan contract). Spike 003 verdict: INCONCLUSIVE. Both Run 1 attempts were SEED
  STALLED, Run 2 was skipped, and $7.81 of $60 was spent. The driver is NOT validated: VALID-01
  Unproven, VALID-02 Partial (REQUIREMENTS.md). Phase 27 stays held until spike 003's seed and
  instruments are redesigned or Phases 27-31 are re-planned. See Blockers.
  Progress: 129/133 plans across v2.0 (roadmap.analyze).

Earlier (v1.14.0 release, 2026-10-16):
Status: Ready to plan
  gsd-core v1.17.0 sync: quick batch 261016-dx2 (261016-dx3 baseline constants 1.17.0 / v1.17.0 /
  1.17.0, sync doc, README note; 261016-dx4 `/gsd-<name>` command form in suggestions, hints,
  backlog enqueue and docs, colon form rewritten at the Claude executor edge, smart-entry pinned to
  GSD_RUNTIME=claude; 261016-dx5 awaiting-verification wording and `verifying` classify as Idle),
  plus quick 261014-0jv (worktree-held phase plans browsable in the TUI). `cargo update`: deranged,
  equivalent, kasuari, libc, pest*, tokio patch bumps. `./scripts/pre-tag-check.sh --container
  v1.14.0` exit **0**, all five gates PASS, git banner MATCH, oracle @opengsd/gsd-core@1.17.0,
  60 suites / **2991 passed / 0 failed** / 16 ignored. Next: `/gsd-plan-phase 26`.

Earlier (v2.0 re-plan, 2026-10-14):
Status: v2.0 re-planned 2026-10-14. The milestone was restructured in place rather than closed. No
tag was created, Cargo.toml was not bumped, and nothing was pushed `[inferred]`.

- **Superseded:** Phases 19 (GITSAFE), 21 (LLM goal layer), 22 (containers) and 23 (gate policy),
  replaced by spikes 001/002 (`.planning/spikes/`).
  - Safety now comes from capability removal, with a bwrap sandbox under `systemd-run --user
    --scope`, no push credentials and a read-only run dir.
  - Launcher hard caps are counted from tool-written state.
  - An Opus master driver steers through a human-owned DIRECTION.md, with frontmatter written by
    driverctl.
- **Code:** the existing Phase 19/21 code stays in place `[inferred]`. Phase 31 retires or rewires
  it.
- **Phase 21:** its gap loop must NOT be resumed. 4/5 is the hard ceiling.
- **Phase 20:** marked complete (`20-VERIFICATION.md` passed 5/5) `[inferred]`. It is kept as the
  advisory router hint.
- **Building blocks:** Phases 15-18 are kept as infrastructure.
- **New phases:**
  - 26 Driver Validation Spike 003
  - 27 driverctl & Driver Frontmatter
  - 28 Sandboxed Driver Launcher
  - 29 Master Prompt & Run Templates
  - 30 Driver Tab on DIRECTION.md
  - 31 Retire Superseded Envelope & Goal-Layer Code
- **Open human calls**, recorded with `[inferred]` defaults and not blocking:
  - OQ-R1: stop rules outrank a human `answer:` (default yes).
  - OQ-R2: driven runs do not see the global CLAUDE.md (default no).
- **Next:** `/gsd-plan-phase 26`.

Earlier (v1.13.0 release, 2026-10-12):
Status: v1.13.0 shipped 2026-10-12 (interim release; v2.0 milestone still open)
  gsd-core v1.16.0 sync: quick batch 261012-jqa (261012-jqb config key delta — six gates.* keys
  retired, gates.confirm_milestone_scope + audit.enabled modelled, DEFAULTS_OPTION_COUNT 135;
  261012-jqc GSD_HOME-aware global defaults path) and 261012-kee (baseline constants 1.16.0 /
  v1.16.0 / 1.16.0, sync doc, README note, $GSD_HOME key-file help); plus fast task 0c525f6
  (auto-register an unregistered GSD project on `gsd-meta-manager <path>`). All deps bumped: MSRV
  raised 1.88 -> 1.89 to take uuid 1.28.0 (CI msrv job + container image moved with it).
  `./scripts/pre-tag-check.sh --container v1.13.0` exit **0**, all five gates PASS, git banner
  MATCH, 59 suites / **2959 passed / 0 failed** / 15 ignored. Router's next is Phase 19, whose
  loop is halted by user decision — ask first.
Earlier (v1.12.0 release, 2026-10-06):
  Quick tasks 261004-gsd-expansion, 261004-readme-accuracy, 261006-cfg, 261006-cfr, 261006-cpp,
  261006-git-lanes and 261006-ujx, plus worktree-only roadmap phases, modal Yes/No confirm popups,
  manager-preferences editor and $EDITOR cache reloads. Release gate also cleared four clippy lints
  and pinned uuid at 1.26.1 (1.27.0 needs rustc 1.89 > MSRV 1.88). `./scripts/pre-tag-check.sh
  --container v1.12.0` exit **0**, all five gates PASS, git banner MATCH, 58 suites / **2947 passed
  / 0 failed** / 15 ignored. Router's next is Phase 19, whose loop is halted by user decision — ask first.
Earlier (v1.11.0 release, 2026-09-30):
  Quick tasks 260929-szq (completed-but-unmerged worktree work surfaced: dashboard ◐N, Waves pane /
  Agents sub-view unmerged state, Roadmap unmerged status + lane glyph, Work State help legend) and
  260930-vvk (`gsd-meta-manager <alias|path>` opens straight into that project's view; path targets
  resolve to the nearest registered root; Esc returns to the overview; subcommands keep priority;
  unknown target exits 1 before terminal setup). `./scripts/pre-tag-check.sh --container v1.11.0`
  exit **0**, all five gates PASS, git banner MATCH, 58 suites / **2882 passed / 0 failed** / 15
  ignored. Router's next is Phase 19, whose loop is halted by user decision — ask first.
Earlier (v1.10.0 release, 2026-09-29):
  Quick tasks 260927-gi9 (Roadmap lane gutter over visible rows only), 260927-t3s (7:Cfg `g`
  project/global scope toggle with visible, clickable scope strip) and 260929-g9u (arrow-key focus
  and ←/→ switching on the scope strip). `./scripts/pre-tag-check.sh --container v1.10.0` exit
  **0**, all five gates PASS, git banner MATCH, 56 suites / **2825 passed / 0 failed** / 15 ignored.
  Router's next is Phase 19, whose loop is halted by user decision — ask first.
Earlier (v1.9.0 release, 2026-09-26):
  Phase 25 (Running Agents & Live Wave View) with WR-02..WR-06 fixed; quick tasks 260925-x0v,
  260926-06g/0u3/16t/1t1/2l4/dyf/kes/fcp/fi9/gd5/j0a/jnf and the gsd-core 1.15.0 sync batch
  260926-gtj (gtk/gtl/gtm/gtn). Config surface synced to gsd-core release/1.15.0
  (v1.14.0-111-gec81d0d10); oracle still pinned to published 1.14.0 — bump GSD_CORE_SYNCED_VERSION
  once 1.15.0 is on npm. `./scripts/pre-tag-check.sh --container v1.9.0` exit **0**, all five gates
  PASS, git banner MATCH, 56 suites / **2798 passed / 0 failed** / 15 ignored.
  Router's next is Phase 19, whose loop is halted by user decision — ask first.
Earlier (v1.8.0 release):
  Phase 24, quick tasks 260922-hdh/hdi/hdj, 260923-lr8/lr9/lra/md1, 260924-drx and five debug
  fixes; README gained screenshots and a Codex section. `./scripts/pre-tag-check.sh --container
  v1.8.0` exit **0**, all five gates PASS, git banner MATCH, 49 suites / **2367 passed / 0 failed**
  / 15 ignored; `cargo publish --dry-run --locked` clean. Next: Phase 19 (halted — ask first).

Previous (v1.7.2):
  `--container` gate.** It supersedes and carries the contents of v1.7.0 and v1.7.1, NEITHER OF
  WHICH EVER PUBLISHED: for anyone installing from crates.io this is the jump from **1.6.0**.
  `Cargo.toml` moved 1.7.1 -> 1.7.2; `cargo update` relocked only this crate's own entry, with
  `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0 (latest 0.2.2) still held by
  upstream `=` pins. What made this cut different is quick task 260917-r4c: the publish job's
  `Test` step now runs `--no-fail-fast` (so a red run enumerates every failure instead of one per
  tag), provisions the GSD conformance oracle via `actions/setup-node@v4` +
  `scripts/install-conformance-oracle.sh`, and `scripts/pre-tag-check.sh` gained `--container`,
  which re-runs every gate inside an `ubuntu-latest` lookalike. Gate on this tree:
  `./scripts/pre-tag-check.sh --container v1.7.2` exit **0**, all five gates PASS, git banner
  MATCH at 2.55.0, 48 suites / **2121 passed / 0 failed** / 15 ignored. The bare local run is
  unchanged at exit 1 with gates 1/2/3/5 PASS and gate 4 failing on exactly one test, the
  environmental git-version witness (local git 2.53.0 vs constants derived against 2.55.0), which
  the container proves green on the runner's git.

Release history preserved (do not rewrite — this is the record):
  **v1.7.0 was tagged and pushed but NEVER PUBLISHED** — its publish job died at `cargo test` on
  the git-version witness. **v1.7.1 was tagged and pushed but NEVER PUBLISHED EITHER** — its
  publish job died at `cargo test` too, but on a DIFFERENT test,
  `tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`,
  which fails BY DESIGN when its oracle is absent and the `ubuntu-latest` runner had no GSD
  install. Fail-fast is what made each tag reveal exactly one failure; that is the cycle v1.7.2
  closes at the source.

Superseded detail from the v1.7.1 entry, kept verbatim:

Status: **v1.7.1 tagged and pushed (Release Gate Repair) — BUT IT DID NOT PUBLISH EITHER.**
  crates.io is still at **1.6.0**; neither 1.7.0 nor 1.7.1 exists there. v1.7.0's publish job died
  at `cargo test`; v1.7.1's publish job died at `cargo test` too, but on a DIFFERENT test —
  `tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`,
  which fails BY DESIGN when its oracle (`~/.claude/gsd-core/bin/gsd-tools.cjs` or a `gsd-tools` on
  PATH) is absent, and the `ubuntu-latest` runner has no GSD install. The escape hatch the test
  names itself is `GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1`. **`scripts/pre-tag-check.sh` structurally
  cannot catch this**: the oracle IS installed on the developer machine, so the test passes locally
  and the gate is green on precisely the thing CI is red on. Next release must address that gap.
  The five fixes below all landed and the git-version witness DID go green on the runner
  (lib suite 1346 passed / 0 failed), so v1.7.1 is a strictly better tree than v1.7.0 —
  it is just not on crates.io. Nothing in non-test `src/` changed between the two — v1.7.1 is test, tooling and
  documentation only, which is what makes it a patch. Five quick tasks make it up: 260917-ii4
  (config-section constants re-derived against git 2.55.0, the runner's version — neither
  `INDIRECTION_SECTIONS` nor `REPARSED_COMMAND_SECTIONS` changed, and a git 2.54
  `alias.<name>.command` spelling falsified a doc justification, repaired in place), 260917-jdi
  (`scripts/pre-tag-check.sh`, a local dry run of the publish gate, with CLAUDE.md release step 3
  routed through it), 260917-k6y (`tests/driver_reattach.rs`: two spawn/write races closed by
  waiting on the artifact rather than on process liveness), 260917-lkg
  (`tests/envelope_tracer.rs`: the ETXTBSY exec race closed with a bounded ETXTBSY-only retry) and
  260917-nhc (`src/driver/run.rs`'s `proc_parse` test race closed with a bounded seqlock re-read).
  Pre-tag gate on the tagged tree: gates 1/2/3/5 PASS, gate 4 failing on exactly one test — the
  environmental git-version witness (local git 2.53.0 against constants derived against 2.55.0),
  green on the runner. 48 suites / 2120 passed / 1 failed / 15 ignored.

Carried forward from v1.7.0 (unchanged by this patch):
  The entire v2.0 driver stack (Phases 15-21: duplex stream-json transport, run journal, supervisor
  with detach/kill-switch/dry-run/opt-in gate, Driver tab with live watch and durable injection, the
  GITSAFE git/blast-radius envelope, the deterministic decision router with run bounds, and the LLM
  goal layer with prompt-injection hardening) is IN this release but is **hidden by default** behind
  the `GSDMM_EXPERIMENTAL_FEATURES` startup flag (quick 260917-fko) and the `drive` subcommand is
  hidden from `--help` (quick 260917-hc3). Nothing in that stack is reachable by a default install,
  which is why the release ships despite Phases 19/20/21 still reading "In Progress" on the ROADMAP
  (all plans executed; `/gsd-secure-phase 19` has not cleared, T-19-86/T-19-91 open at high).
  MSRV is 1.88 in this release — consumer-visible.

Active milestone (superseded by the 2026-10-14 re-plan above; resume there):
Milestone: v2.0 Autonomous Orchestration (Phases 14-31). Phases 19/21/22/23 are superseded.
Phase: 28.1 — Sandbox Egress & Agent-Channel Isolation
Plan: 14
  19-11 closed T-19-60 structurally (resolve the effective program by finding the first token
  whose BASENAME is in a closed GOVERNED_PROGRAMS set; NO wrapper-name list added, one DELETED).
  19-12 certifies that closure at the CLASS level rather than at the instance level:
  tests/envelope_wrapper_class.rs, one created file, 1181 lines, 11 tests, 0.2s.
  A hand-rolled fixed-seed LCG (seed 0x1912_C0DE_5EED_0060, no crate — T-19-SC holds) composes
  30 wrapper spellings chained 0-3 deep x 4 assignment prefixes x basename/absolute-path forms
  x 3 quoting styles x an optional sh -c / bash -lc payload layer, and asserts
  verdict(wrap(base)) == verdict(base) on exit code AND D-24 reason identifier, with the
  right-hand side MEASURED from the unwrapped command in the same run.
  MEASURED: 1680 refused cases / 773 distinct wrapper chains / 3 distinct D-24 reasons, and a
  paired allow corpus of 1080 permitted cases / 533 chains, each exit 0 with EMPTY stdout (D-32).
  12 wrapper names proved absent from the PRODUCTION LOGIC of policy.rs and hooks.rs (4 of them
  from the raw bytes too), plus an orchestrator-required POSITIVE control proving each
  include_str! file is the file the scanner thinks it is — both proved fail-first.
  T-19-74 pinned on both sides, including the command-line boundary where the residual begins.
  Gate: cargo test --no-fail-fast 1470 passed / 2 failed / 13 ignored; clippy -D warnings exit 0;
  --all-targets lint count unchanged. USE --no-fail-fast: a plain `cargo test` stops at
  driver_reattach and never reaches any envelope_* binary, reporting 1245/2/13 regardless of
  what was added. The 2 failures are the pre-existing driver_reattach pair (deferred-items.md).
Status: Ready to execute
  21-35 closes a producer/consumer wire-format mismatch: the producer emits the fused
  `--resume=<id>` (one argv element) while the consumer still parses only the split
  `["--resume", "<id>"]`, so TUI-resumed sessions became silently undetectable in /proc.
  The consumer learns BOTH forms (both are legitimate on the wire — sessions started
  outside the TUI still use split); the producer is NOT un-fused (round 11 measured that a
  `--` end-of-options separator deletes the resume feature outright).
  The round also adds the producer<->consumer round-trip control that spans the pair — no
  single-sided test could have caught this — and corrects, in place, the `read_session_id`
  doc comment that argued against a control there. That argument is right about the SECURITY
  property (it is a property of the sink) and is precisely what hid the FUNCTIONAL coupling.
  criterion 4 remains PRESENT_BEHAVIOUR_UNVERIFIED and permanently agent-unclosable — it needs
  a human with a live Claude subscription for the 10 #[ignore]d arms. 4/5 is the correct ceiling.
  Phase 19 human-judgement UAT items were deferred by explicit user decision on 2026-08-19, not resolved.
Last activity: 2026-10-28 — Phase 31 complete, transitioned to Phase 28.1
Note (260908-uqq): the D-06 "a capability refusal costs zero tokens and zero quota" guarantee no
longer holds unconditionally. It holds on the **eager arm** only (CLI announced inside the grace).
On the **late arm** — 2.1.266's measured shape — the grace releases the prompt first, so a refusal
aborts a turn that has already begun. Both arms still refuse. Do not re-broaden that claim.

### Deferred Verification

| Phase | State | Resume |
|-------|-------|--------|
| 19 | verification_deferred_human | /gsd-verify-work 19 |

Phase 19's `19-VERIFICATION.md` remains `status: human_needed` — 5/5 automated must-haves are
verified, and the four outstanding items are all reading judgements (does the pinned honesty
statement read as candour; are 19-08's composed proofs faithful decompositions; do the
residual-exposure disclosures read as admissions) plus the `driver_lock` one-off risk call.
Deferred by user request on 2026-08-19 so Phase 20 could start; **not** marked passed.

#### Phase 25 gap planning (25-07) — decision-coverage gate override (autonomous run — audit this)

`/gsd-plan-phase 25 --gaps` step-13a gate returned `{passed: false, reason: "could-not-parse",
total: 30}` and was **overridden** [inferred], human unavailable. Parser mismatch, not a coverage
gap: `25-CONTEXT.md` uses letter-grouped IDs (`D-A01`, `D-C08`), which the `D-NN`/`D4-NN` grammar
rejects. Phase-wide decision coverage was already planned by 25-01..25-06; 25-07 is gap-only
(CR-01 + WR-01) and cites D-A03/A08/B01/C08/C10/C12/C13/C14/C16/C17. CONTEXT.md not edited.

#### Phase 22 planning — decision-coverage gate override (autonomous run — audit this)

`/gsd-plan-phase 22`'s step-13a decision-coverage gate returned
`{passed: false, reason: "could-not-parse", total: 16, covered: 0, uncovered: []}` and was
**overridden** by the orchestrator, with the human unavailable. It is a parser mismatch, not a
coverage gap: the gate's ID grammar reads `D-NN` / `D4-NN`, and `22-CONTEXT.md` uses the
three-segment `D-22-NN` form, so three bullets (D-22-08, D-22-13, D-22-15) were reported
unparseable and coverage counting was abandoned before it ran. Verified by direct grep instead:
all **19** decisions D-22-01…D-22-19 are cited in `22-0*-PLAN.md`, each at least twice
(D-22-19 sixteen times, D-22-18 twelve). No decision was dropped. Nothing in `22-CONTEXT.md` was
edited to satisfy the parser — it is a committed, locked discuss artifact. Re-surface at
verify-phase if the gate is fixed.

#### Phase 17 gap-closure notes (autonomous run — review these)

- **The code review (`17-REVIEW.md`, `issues_found`) found SIX BLOCKERS after all 7 plans had
  executed and the gate was green.** All six were in the kill switch and its liveness probe —
  the phase's own safety contract — and each made the goal's word "stoppable" false on a
  reachable path. Plan **17-08** closed them: CR-01 (SIGTERM buffered during
  `Executor::start()`, orphaning the agent group), CR-02 (the stop signalled an agent-writable
  `pgid` out of `run.json`), CR-03 (the detached spawn dropped the TUI's `--config`, so a
  driver could run in a different project than the user selected), CR-04 (a run whose
  `--run-id` was not on argv read as dead and could not be stopped), CR-05 (`/proc`-only
  liveness answering `false` off Linux, consumed as "already gone"), CR-06 (unregistering a
  project abandoned its live agent with no stop path).

- **Seventeen WARNINGS (WR-02..WR-17) are deliberately DEFERRED**, with a one-line reason each
  in `17-08-PLAN.md`'s deferral table. WR-01 was folded in because CR-02's kernel-vs-record
  agreement check is meaningless while the record's `pgid` is assumed rather than read. The
  highest-value carry-forwards for a later phase: **WR-02** (`run_id` is joined into a path with
  no component validation — the run directory can escape the project, reproduced), **WR-10**
  (blocking `flock`/fs/git inside `async fn`, which already caused an observed deadlock in
  `tests/driver_lock.rs`), **WR-15** (`DriverStopped` drops the observed run even when nothing
  was stopped) and **WR-16** (the hidden `--claude-program` flag ships in release builds).

- **`ObservedRun.live: bool` became `liveness: Liveness` + `is_live()`** so the third state
  CR-05 requires can be carried. Phase 18 consumes `ObservedRun`; it must read the tri-state
  rather than reintroducing a boolean.

- **`--run-id` is now REQUIRED for a real `drive`** (a preview still works without one). The
  "generated when absent" mode `cli.rs` and `driver/mod.rs` used to document is gone, because a
  generated id never reaches argv and therefore made the run invisible to liveness.

#### Phase 15 planning notes (autonomous run — review these)

- **Structural finding no research document anticipated:** `type:"result"` is a **turn** boundary,
  not a **run** terminator. A run receiving a second stdin message emits two `system/init` and two
  `result` envelopes in one process. An executor returning on the first `result` would truncate
  every steered run while reporting success. Recorded as amendments **D-29..D-32** in
  `15-CONTEXT.md` so the decision-coverage gate forces the plans to handle it.

- **Decision-coverage gate is live again and PASSES 32/32.** `15-CONTEXT.md` uses the
  `- **D-NN:** …` bullet form, which fixes the `could-not-parse` failure recorded for Phase 14
  below. Note for future phases: the gate matches `\bD-NN\b` **only inside designated sections**
  (plan frontmatter `must_haves`/`truths`/`objective`, designated headings, XML tag bodies), so
  plans must cite the ids inline, not merely implement the decisions.

- **ROADMAP gained an authoritative `**UI hint**: no` for Phase 15.** The blocking `ui.plan-gate`
  fired because `checkUiPresence` token-sniffed the bare word `ui` out of a risk bullet that refers
  to *Phase 18's* injection UI. Phase 15 ships no visual surface, so the author-declaration form was
  used rather than a transient `--skip-ui`, making the record durable for verify-work and progress.

- **`--research-phase` was interpreted as `--research`.** ROADMAP's Phase 15 entry says
  `Research: yes — /gsd-plan-phase --research-phase`, but that flag is research-**only** mode and
  exits before the planner runs, producing no plans. Ran the full research→plan→verify flow instead.
  The same wording appears on Phases 20 and 22 and will need the same reading.

- **Seven raw spike transcripts staged, gitignored.** `.planning/phases/15-transport-foundation/transcripts-raw/`
  holds the captured NDJSON (clean success, budget-exhausted, tool-use success, the hook hang, the
  two-turn queued injection, interrupt-during-streaming, and the interrupt race). They carry absolute
  host paths, so a `.gitignore` entry prevents accidental commit; plan 15-01 redacts and promotes
  them into `tests/fixtures/` and plan 15-02 deletes the staging directory. Scanned for
  credential-shaped strings — none found.

#### Phase 14 planning notes (autonomous run — review these)

- **Decision-coverage gate overridden.** `check.decision-coverage-plan` returned
  `passed: false, reason: could-not-parse, total: 0, uncovered: []` — `14-CONTEXT.md` writes its
  decisions as `### UIFIX-0N — title` headings rather than the `- **D-NN:** …` bullets the parser
  requires, so zero decisions were extracted and none could be reported uncovered. Proceeded
  deliberately: retrofitting `D-NN` ids would make the gate report every id as uncovered (the plans
  cite `UIFIX-NN`), converting a cosmetic parse failure into a real block. The substantive check was
  performed instead by `gsd-plan-checker`, which returned VERIFICATION PASSED and confirmed each
  CONTEXT.md decision (verify-first, fix-at-source, key-routing, clamp-at-handler, testing
  convention, scope fences) is honored. **Follow-up:** consider normalising CONTEXT.md decision
  format to `D-NN` bullets for future phases so this gate is live.

- **Plan-checker's clippy warning was a false positive.** It reported the "5 pre-existing
  `--all-targets` lints" in `14-CONTEXT.md` as stale (claiming 0 today). Re-measured directly:
  `cargo clippy --all-targets -- -D warnings` still fails with exactly **5** pre-existing lints —
  3× `assert_eq!` with a literal bool (`browser.rs`), 1× owned-instance-for-comparison
  (`project_creator.rs`), 1× items-after-test-module (`state_reader/mod.rs`). The checker measured
  without `-D warnings`. `14-CONTEXT.md` and `14-PATTERNS.md` are **correct as written** and were
  deliberately NOT "corrected". The lib-target project gate `cargo clippy -- -D warnings` passes clean.

- **UI-SPEC gate honored, not skipped.** ROADMAP marks Phase 14 `UI hint: yes` and the blocking
  `ui.plan-gate` fired; `14-UI-SPEC.md` was generated and approved 6/6 by `gsd-ui-checker`.
  Research was skipped per ROADMAP (`Research: skip`) — no RESEARCH.md exists for this phase.

### Pending Todos (list as of 2026-10-28, before cleanup)

- [2026-07-29] [ui] Driver tab renders blank pipeline row at terminal heights 8-13 — [todo file](.planning/todos/pending/2026-07-29-driver-tab-layout-at-medium-terminal-heights.md)
- [2026-08-19] [driver] The verify-work gate policy must be configurable — skip | defer | auto-validate — [todo file](.planning/todos/pending/2026-08-19-verify-work-gate-policy-configurable.md)
- [2026-09-15] [ui] Make queue input truly multi-line — [todo file](.planning/todos/pending/2026-09-15-make-queue-input-truly-multi-line.md) — Needs TBD — this todo's deliverable is a **UX proposal**, not a fix. Whoever picks.
- [2026-09-22] [driver] "Codex runtime: remainder after the MVP slice (260922-hdj) [AUDIT]" — [todo file](.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md)
- [2026-10-18] [state_reader] Dashboard renders quoted VERIFICATION status as Unknown while driverctl counts it as a gap round — [todo file](.planning/todos/pending/2026-10-18-dashboard-verification-status-quote-divergence.md)
- [2026-10-21] [ui] Borders and typography still draw 2 cells under ambiguous-wide settings; DarkGray remains outside the Driver tab — [todo file](.planning/todos/pending/2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md)
- [2026-10-24] [session_detector] Session detector binds the successor of -r/--resume although claude treats the value as optional — [todo file](.planning/todos/pending/2026-10-24-session-detector-resume-optional-arity.md)
- [2026-10-25] [envelope] Two unpinned properties in surviving envelope code (scan Absent never permits, CredentialSource drift) — [todo file](.planning/todos/pending/2026-10-25-phase-19-surviving-pin-gaps-scan-and-credential-source.md)
- [2026-10-25] [session_detector] Round-trip certificate for session ids omits the --session-id spellings the executor emits — [todo file](.planning/todos/pending/2026-10-25-session-detector-wire-forms-omit-session-id.md)
- [2026-10-27] [ui] Enable TUI driver launch and steering on macOS (close 28-REVIEW WR-05) — [todo file](.planning/todos/pending/2026-10-27-enable-tui-driver-launch-and-steering-on-macos.md)
- [2026-10-27] [ui] Add TUI badge for per-repo release maps — [todo file](.planning/todos/pending/2026-10-27-tui-badge-for-per-repo-release-maps.md)

### Blockers/Concerns

- [Phase 26.1] **Phase 27 blocker CLEARED 2026-10-18.** 26.1 verification passed; operator accepted
  VALID-02 = ADJUST. Phases 27-31 may proceed.
- [Phase 26.1 -> 28/29] **Carried risk (operator-accepted):** launcher gap-cap deny at limit+1 / kill
  at limit+2 was never reached under a real Opus driver (scripted Haiku r261-l7 only). Phases 28 and 29
  must exercise launcher deny/kill under the real driver, with the prompt divergence rule disabled.
  **RESOLVED by Phase 28 (2026-10-24):** acceptance d33 ran a real Opus driver to
  `cap-tripped gap_rounds kill-scope` (28-VERIFICATION.md, DRIVE-01 row) `[inferred — audit]`.
- [Phase 27] Advisory, not blocking: 27-REVIEW.md CR-01 (critical — gap scan has no type/size guard on
  driver-plantable files, a FIFO can hang the store mutex) and WR-01..09 open (27-REVIEW-DISPOSITION.md).
  Phase 28 runs `driverctl serve` under the launcher, so fix or defer before relying on it
  (`/gsd-code-review 27 --fix`) `[inferred — audit]`.
- [Phase 26.1] Advisory review warnings WR-10 and WR-11 remain open/deferred (26.1-REVIEW-DISPOSITION.md).

- [Re-plan 2026-10-14] The T-19-* rows below and the open gaps from Phase 21 round 13 no longer
  block the milestone. Those phases are superseded. Phase 31 closes their ids, each with a pointer to
  the mechanism that superseded it. Do not resume either gap loop.
- [Phase 26] **RESOLVED 2026-10-17 → Phase 26.1** (re-run revised spike 003 before Phase 27; 26.1 now carries the go/no-go gate for 27-31). Original entry: several risks are still unproven, and each needs a go verdict before Phase 27:
  - the gap-closure loop and divergence detection have never been observed on a real GSD phase;
  - the async-Agent "wait, never re-spawn" rule is untested, and in spike 002 it doubled the Opus
    planner cost;
  - a host OAuth refresh revokes the env token a sandboxed run is using. The fix, a
    `claude setup-token` long-lived token, is in Phase 28.

- `--bare` is slated to become the `-p` default and is incompatible with subscription
  auth; Phase 15 ships a version gate and a regression guard against it

- Phase 13's queue-execution design self-dated "valid until 2026-04-30"; re-verify GSD's
  autonomous-mode / checkpoint contract during Phase 20 research

- T-19-86 (governed program's own operand names a governed command) and T-19-87 (${VAR} fragments a command past the splitter) are open, pinned and deferred to a round-4 plan; /gsd-secure-phase 19 cannot clear until they are closed
- T-19-97/T-19-98/T-19-99 open — corpus RED at 1605 (passed+failed); 19-19 writes the rule. /gsd-secure-phase 19 NOT cleared: T-19-86 and T-19-91 remain open at high
- 19-21: tests/envelope_callee_grammar.rs pins `git --super-prefix x push --force origin main` at force_push_blocked while pinning `git --super-prefix x status` at envelope_assertion_failed — identical leading tokens, so no rule obeying 19-21's prohibitions satisfies both. Row left RED. Both refuse at exit 2; only the identifier differs. **RESOLVED 2026-09-04 (989f21a)** — the 19-21 executor's analysis was verified by measurement against the built binary (fresh envelope root per row, root walked after) and HELD: both spellings refuse at exit 2 with an empty walk, `--super-prefix` is absent from GIT_GLOBAL_VALUE_OPTS after 19-21, and git 2.43.0 rejects it bare, separate and attached. `force_push_blocked` was a PRE-fix observation mislabelled as post-fix. Row 937 corrected to envelope_assertion_failed; exit code and empty walk still asserted; no `src/` change and no second reading site. All thirteen envelope_* binaries green; passed+failed = 1639, 0 failures. **This does NOT clear `/gsd-secure-phase 19`** — T-19-86 and T-19-91 remain OPEN at high, T-19-17r stays OUTSTANDING (no AR-19-13, not accepted), and only the WRAPPER-OPERAND sub-class of T-19-60 is closed.
- T-19-110 (NEW, high): scan_config under-reads a dash-leading config value, so git config core.hooksPath -c and -- exit 0 while /dev/null and - are refused. Found by 19-25, deliberately not fixed — needs a RED corpus first.
- 19-29 must add tests/envelope_control_carrier.rs to its files_modified: its direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted pins three rows PERMITTED that 19-29's redirection rule refuses (19-28 finding)

- [Phase 24] Advisory, not blocking: 24-REVIEW.md has 5 warnings / 6 info open — WR-01 colliding BandKeys deadlock j/k, WR-02 `declared_phase_count` u32 overflow on third-party input (roadmap_md.rs:898, debug-build panic), WR-03 build-phase range includes its own id (spurious cycles), WR-04 width measured in chars (wide glyphs overflow columns), WR-05 Roadmap cursor does not write back to Phases selection. Fix via `/gsd-code-review 24 --fix` or a quick task.
- [Phase 24] Roadmap footer (91/87 cols) clips `[?]help` at 80 cols.
- [Phase 25] Deferred review warnings from 25-REVIEW.md (advisory, not blocking; fix via `/gsd-code-review 25 --fix` or a quick task):
  - WR-02: a hung git call (no timeout in `git_read_raw`) leaves `agents_scan_in_flight` set forever — scan never re-runs and the frozen view still reads live (`src/app.rs:1212-1234`, `src/state_reader/git_ops.rs:202-217`); needs a watchdog or per-call timeout
  - WR-03: fixer estimate picks the first `*-REVIEW.md`, so `NN-EVAL-REVIEW.md`/`NN-UI-REVIEW.md` can supply the wrong `findings.total` (`src/agents/fixers.rs:152-171`)
  - WR-04: earlier review runs inflate `fixed` (unbounded id union, can exceed total, unclamped) and any leftover `*-REVIEW-FIX.md` hides the count (`src/agents/fixers.rs:163-164,266-285`)
  - WR-05: long-path (>200-unit) prefix fallback can adopt another project's Claude dir and list its live subagents as this project's (`src/agents/adapters/claude.rs:288-311`)
  - WR-06: Finished orphan fixers still count in `N fixers`, outvote the running fixer's phase and inflate `fixed` — the CR-01 pattern left in `src/agents/fixers.rs:137-285`
  - Info IN-01..IN-07 also open (redraw every scan for aged-out rows, tests reading real `~/.claude`, relative `gitdir:` ignored, `lock_released` premise, children not counted as running, `plan_state` re-spells `is_running`, gate-ordering test) — see 25-REVIEW.md
- [Phase 25] deferred-items.md: `.planning/WINDOWS.md` ledger table disagrees with its fenced JSON (row 19), so `gsd-tools windows append` refuses to write; `cargo clippy --all-targets` has pre-existing warnings (browser.rs, project_creator.rs, several `tests/envelope_*`); repo is not `cargo fmt --check` clean
- 31-05 SC4: container dry run exit 1, gate 4 red on two pre-existing 28.1 launcher tests (planted.rs a_truncation_the_baseline_had_too..., launcher_sandbox no_flag_names_a_credential_directory...); identical on pre-plan 235a912; needs a 28.1 follow-up then a container re-run

### Session Continuity

Last session: 2026-10-28T05:05:15.832Z
Stopped at: Phase 31 complete, ready to verify Phase 28.1
Resume file: None

Next: `/gsd-plan-phase 28.1` (or `/gsd-discuss-phase 28.1`). Phase 30 verified and complete. v1.15.0 is an interim release; the v2.0
milestone stays open.

Earlier (2026-10-21): Next: `/gsd-verify-work 28` (Sandboxed Driver Launcher: 28-UAT.md is `testing`, 28-VERIFICATION.md
`human_needed`). Phases 30-31 follow.

Earlier (2026-10-17): Next: `/gsd-execute-phase 26.1`. 26.1 must return a go verdict before Phase 27 starts.

Earlier (2026-10-16): Phase 26 complete (spike 003 INCONCLUSIVE). The blocker decision recorded then:

Next: a human decides between (a) redesigning spike 003 and re-running it, and (b) re-planning
Phases 27-31 with `/gsd-phase`. Either way, use `/gsd-progress` to route. Do NOT run
`/gsd-discuss-phase 27` or `/gsd-plan-phase 27` while the Phase 26 blocker stands.

Earlier (2026-10-14): v2.0 re-planned around the capability-removal driver from spikes 001 and
002. Phases 19, 21, 22 and 23 are superseded, and Phases 26-31 were added.

Next: `/gsd-plan-phase 26`. Phase 26 is the validation spike. It must give a go verdict before
Phase 27 starts.

Still unproven, and owned by Phase 26:

- gap rounds 1-2 on a real phase;
- divergence detection;
- the async-agent no-respawn rule.

macOS is unproven too, and is owned by Phase 28's degraded mode.

Open human calls, each with an `[inferred]` default:

- OQ-R1: stop rules outrank `answer:` (default yes).
- OQ-R2: driven runs do not see the global CLAUDE.md (default no).

The Phase 19 and Phase 21 threat and blocker rows below are moot once Phase 31 closes them as
superseded.
Resume file: None

Previous session (v1.13.0 release, 2026-10-12):
Last session: 2026-10-12
Stopped at: v1.13.0 released (gsd-core 1.16.0 Sync)
Release: `Cargo.toml` 1.12.0 -> 1.13.0; MSRV `rust-version` 1.88 -> 1.89 (uuid 1.26.1 -> 1.28.0,
user decision). `cargo update` moved cc, indoc, litrs, mac_address (drops nix), pest*, serde_json,
sha2, smallvec, syn, unicode-ident; adds stacker/psm. Still held by upstream `=` pins: psm 0.1.24
(0.1.32, pest), generic-array 0.14.7 (0.14.9), unicode-width 0.2.0 (0.2.2). Container pre-tag
check green. Phase 19's gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.12.0 release, 2026-10-06):
Stopped at: v1.12.0 released (Lanes, Popups and Config Polish)
Release: `Cargo.toml` 1.11.0 -> 1.12.0; `cargo update` moved cc, either, lazy_static, libc, mio,
powerfmt, tokio; uuid held at 1.26.1 (1.27.0 needs rustc 1.89, MSRV is 1.88). Still held by upstream
`=` pins: generic-array 0.14.7 (0.14.9), unicode-width 0.2.0 (0.2.2). Container pre-tag check green.
Phase 19's gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.11.0 release, 2026-09-30):
Stopped at: v1.11.0 released (Direct Project Launch)
Release: `Cargo.toml` 1.10.0 -> 1.11.0; `cargo update` moved mac_address 1.1.8 -> 1.2.0 (adds nix
0.30.1) and yoke-derive 0.8.3 -> 0.8.4; still held by upstream `=` pins: generic-array 0.14.7
(0.14.9), unicode-width 0.2.0 (0.2.2). Container pre-tag check green (see Current Position). Open
follow-ups unchanged from v1.9.0.
Phase 19's gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.10.0 release, 2026-09-29):
Stopped at: v1.10.0 released (Config Scope Navigation)
Release: `Cargo.toml` 1.9.0 -> 1.10.0; `cargo update` relocked only this crate's own entry; still
held by upstream `=` pins: generic-array 0.14.7 (0.14.9), unicode-width 0.2.0 (0.2.2). Container
pre-tag check green (see Current Position). Open follow-ups unchanged from v1.9.0.
Phase 19's gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.9.0 release, 2026-09-26):
Stopped at: v1.9.0 released (Running Agents & GSD 1.15.0 Sync)
Release: `Cargo.toml` 1.8.0 -> 1.9.0; `cargo update` moved cc 1.5.1, find-msvc-tools 0.1.14,
js-sys 0.3.106, siphasher 1.0.4, smallvec 1.16.2, wasm-bindgen* 0.2.129; still held by upstream `=`
pins: generic-array 0.14.7 (0.14.9), unicode-width 0.2.0 (0.2.2). Container pre-tag check green
(see Current Position). Open follow-ups: GSD_CORE_SYNCED_VERSION -> 1.15.0 once published on npm;
help/footer vs keymap mismatches F1-F8 (quick 260926-gd5 SUMMARY); Phase 25 IN-01..IN-07.
Phase 19's gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (2026-09-25): Phase 25 verified passed and closed; PROJECT.md evolved
(AGENT-01..07 validated, four Phase 25 key decisions).

Previous session (v1.8.0 release, text truncated by earlier state tooling):
user-facing features); `cargo update` moved cc, find-msvc-tools, finl_unicode, instability, libredox,
lru, pest*, process-wrap, thiserror*; still held by upstream `=` pins: generic-array 0.14.7 (0.14.9),
unicode-width 0.2.0 (0.2.2). `assets/` excluded from the crate package. Router names Phase 19 next,
but its gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.7.2 release):
Release: `Cargo.toml` at 1.7.2. `cargo update` relocked **only this crate's own entry**
(1.7.1 -> 1.7.2); no dependency moved. Still behind latest, unchanged from v1.7.0/v1.7.1 and not
bumpable from this manifest: `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0
(latest 0.2.2), both held by upstream `=` pins. `Cargo.lock` is committed — CI publishes with
`cargo publish --locked`.
Verification ran through **both** arms of CLAUDE.md release step 3:
  `./scripts/pre-tag-check.sh --container v1.7.2` — **exit 0**, gates 1 (tag/version),
  2 (MSRV 1.88), 3 (`cargo build --release`), 4 (`cargo test --no-fail-fast`) and 5
  (`cargo clippy -- -D warnings`) ALL PASS; git banner **MATCH** (container git 2.55.0 against the
  constant's 2.55.0); 48 suites / 2121 passed / **0 failed** / 15 ignored. This is the arm that
  corresponds to CI, and it is the first time it has been green.
  `./scripts/pre-tag-check.sh v1.7.2` (bare, dev machine) — exit 1, gates 1/2/3/5 PASS, gate 4
  failing on **exactly one** test: the environmental git-version witness
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
  (local git 2.53.0 vs constants derived against 2.55.0). 48 suites / 2120 passed / 1 failed /
  15 ignored. That single failure is the state a correctly-prepared release is supposed to be in,
  and the container run is the proof it goes green on the runner.
Deliberately-open items carried, NOT fixed in this release: the `run_stub` BrokenPipe race in
`tests/envelope_tracer.rs` (~1 in 800 contended runs) and the second BrokenPipe at
`tests/envelope_interior_path.rs:1068`. Neither appeared in either gate run.
`master` pushed, `dev` fast-forwarded to `master` and pushed (never rebased — planning docs cite
shas), then the `v1.7.2` tag pushed last, triggering `.github/workflows/release.yml`.

Previous session (2026-09-17T23:30:00.000Z) — Tagged and pushed v1.7.1, which DID NOT PUBLISH:
Release: `Cargo.toml` at 1.7.1, `Cargo.lock` refreshed by `cargo update` — **only this crate's own
version entry moved** (1.7.0 -> 1.7.1); no dependency relocked, because v1.7.0's refresh was a day
earlier. `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0 (latest 0.2.2) remain
behind latest under upstream `=` pins — the same two carried over from v1.7.0, deferred, not
bumpable from this manifest. Verification ran through `./scripts/pre-tag-check.sh v1.7.1`
(CLAUDE.md release step 3): gates 1 (tag/version), 2 (MSRV 1.88 `cargo check --locked`),
3 (`cargo build --release`) and 5 (`cargo clippy -- -D warnings`) PASS, no gate NOT REACHED, and
gate 4 (`cargo test --no-fail-fast`, 48 suites / 2120 passed / 1 failed / 15 ignored) failing on
exactly one test — `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
which is environmental (local git 2.53.0 against constants deliberately re-derived against the
runner's 2.55.0) and is the state a correctly-prepared v1.7.1 is supposed to be in.
`master` pushed, `dev` fast-forwarded to `master` and pushed (never rebased — planning docs cite
shas), then the `v1.7.1` tag pushed last, which triggered `.github/workflows/release.yml`
(run 35290670227).
**RESULT: msrv job success, publish job FAILURE at the `Test` step — nothing published.**
crates.io remains at **1.6.0**; neither the v1.7.0 nor the v1.7.1 tag names an installable version.
The failing test is
`tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`
at `tests/driver_router_conformance.rs:377`, panicking because the GSD conformance oracle is not
present on the runner. It was NOT re-run and NOT fixed — that is the next task, and the honest
options are to install GSD/Node in the publish job or to set
`GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1` there and accept that CI verifies none of the
transcription. Whichever is chosen, `scripts/pre-tag-check.sh` needs a way to reproduce the
runner's oracle-absent condition, because a developer machine always has the oracle and the
script was green on this exact tree.
(That repair was done by quick task 260917-r4c and v1.7.2 was cut on top of it — see above.)
Next action: v2.0 resumes (no release has closed the milestone): execute Phase 22, and
re-run `/gsd-secure-phase 19` so 19/20/21 can move off "In Progress".
Prior-session note, still live:
`21-20-PLAN.md` and both PASSED after revision (3b0ef4d, addc3cc); the ROADMAP now carries its
"Gap closure, round 7" block. Next action: execute round 7 (wave 1 = 21-19, wave 2 = 21-20).
Resume file: None
(both retained deliberately — the plan-checker step runs as a separate agent and needs them;
delete only after round 7 executes).
Note: `.planning/phases/19-gitsafe-git-blast-radius-envelope/.continue-here.md` is a stale
mid-execution checkpoint (claims task 3 of 8); all 12 plans have summaries. Ignore it.
Note (19-12): the next action for Phase 19 is **re-run `/gsd-secure-phase 19`** — `19-SECURITY.md`
still carries `threats_open: 1` / `status: blocked` and a Sign-Off block predating 19-11. Neither
19-11 nor 19-12 edited it, deliberately (D-25): re-running the audit is what should flip it, not
the plans that closed the finding writing their own verdict into it. `T-19-60` is closed and now
certified at the class level; the `T-19-74` pin the register names as 19-12's job is done.
Note (19-12): use `cargo test --no-fail-fast` for any phase-19 suite number. A plain `cargo test`
stops at the failing `driver_reattach` binary and never reaches any `envelope_*` binary.
Note: `.planning/ROADMAP.md`'s "Gap closure, round 7" block is now written (unexecuted `- [ ]`
boxes for 21-19/21-20); the outstanding-edit note that stood here is resolved.
