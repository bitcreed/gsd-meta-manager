---
created: 2026-08-22T22:19:41.629Z
title: Phases panel marker/grey disagrees with the disk-inferred stage
area: ui
severity: major
source: user observation on the Detail > Phases panel, 2026-08-22
files:
  - src/ui/screens/detail.rs:2604-2652
  - src/state_reader/roadmap_md.rs:150-250
  - src/state_reader/roadmap_md.rs:360-421
  - src/state_reader/mod.rs:200-216
  - src/state_reader/disk_status.rs:667-684
  - .planning/ROADMAP.md:93-95
  - .planning/ROADMAP.md:583-597
---

# Phases panel: the `+`/`o` marker and the `[stage]` badge report two sources that disagree

Inspected at commit `fbdfd6d176a09ddb6c6593a0dd3a6b59e23d034b`
(`test(21-17): red arm — two identities that render identically both resolve`).
A `/gsd:execute-phase` run for phase 21 was mid-flight in the same tree while this
was investigated, so line numbers are quoted against that pinned SHA, not the
working tree.

## Observation

```
  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count
  + P14: UI Fixes  4/4 plans [Complete]
  + P15: Transport Foundation  8/8 plans [Complete]
  + P16: Run Journal & State Substrate  6/6 plans [Complete]
  + P17: Supervisor  8/8 plans [Complete]
  + P18: Driver Tab, Live Watch & Durable Injection  11/11 plans [Complete]
  o P19: GITSAFE — Git & Blast-Radius Envelope  8/8 plans [Executed]
  o P20: Deterministic Decision Router & Run Bounds  5/5 plans [Complete]
  o P21: LLM Goal Layer & Prompt-Injection Hardening  16/18 plans [Executing 16/18]
  o P22: Container Execution Target  0/? plans [Not started
```

P14–P18 render dark grey (done). P20 renders un-greyed with the future marker `o`
while its own disk-inferred badge says `[Complete]`. No line carries the `*`
current marker at all.

## What drives each signal

**Marker + grey** — `src/ui/screens/detail.rs:2607` and `:2639`:

```rust
let (icon, is_current) = if phase.completed {
    ("+", false)
} else if phase.number == current_phase_num {
    ("*", true)
} else {
    ("o", false)
};
```

`phase.completed` comes only from the ROADMAP.md summary checklist checkbox —
`parse_roadmap_phases` sets `completed: &caps[1] != " "` from `- [x]` /  `- [ ]`
(`src/state_reader/roadmap_md.rs`, checklist regex + `merge_duplicate_phases`,
which ORs duplicate sightings). Nothing else can set it.

**`[stage]` badge** — `disk_suffix_spans` (`src/ui/screens/detail.rs:453-518`)
renders `DiskInference.status` from `src/state_reader/disk_status.rs`. Its
`Complete` arm is the conjunction (`disk_status.rs:669-675`):

```rust
let implementation_complete = summary_count >= plan_count && plan_count > 0;
let status = if implementation_complete && verification_status.is_passed() {
    DiskStatus::Complete
} else if implementation_complete {
    DiskStatus::Executed
```

The two are computed independently and never reconciled: `badge_spans` is
appended to whichever styled span the marker branch already chose.

## Root cause: stale data, not a rendering bug — plus a systematic close-out gap

The renderer is faithfully reporting two sources that disagree.

`.planning/ROADMAP.md:93-95` still reads:

```
- [ ] **Phase 19: GITSAFE — Git & Blast-Radius Envelope** - ...
- [ ] **Phase 20: Deterministic Decision Router & Run Bounds** - ...
- [ ] **Phase 21: LLM Goal Layer & Prompt-Injection Hardening** - ...
```

and the `## Progress` table (`.planning/ROADMAP.md:592-594`) has phases 19, 20
and 21 all as `In Progress`.

`git log -S'Phase 20: Deterministic Decision Router' -- .planning/ROADMAP.md`
returns exactly one commit: `cb85ed2 docs: create milestone v2.0 roadmap`. The
phase-20 checkbox has **never** been touched since the roadmap was written. Same
for phase 19. Meanwhile `20-VERIFICATION.md` landed `status: passed` at
`7657bc8` (2026-08-19 18:40) and the very next commit is phase-21 work.

`gsd-core` does own this write: `roadmap.cjs:828-831` (`update-plan-progress`)
and `phase.complete` both flip the checkbox, gated on
`isPhaseComplete` / verification `passed` (`roadmap.cjs:675-706`). It did not run
for phase 20. Notably `cefb2bd docs(phase-21): begin phase` bumped **STATE.md**
`current_phase` 20 → 21 and `completed_phases` 6 → 7 in a one-file commit that
never touched ROADMAP.md — so the two tracking files were advanced by different
hands and diverged. The phase-20 run was an autonomous run
(`autonomous.md:401` invokes `execute-phase --no-transition`, and only runs
`transition.md` — the thing that calls `phase.complete` — after verification);
the transition step evidently never fired.

## P19 vs P20 — not the same case

- **P20 is genuinely wrong.** `20-VERIFICATION.md` frontmatter is
  `status: passed`. Disk correctly infers `Complete`. The unchecked `[ ]` and the
  `In Progress` Progress row are stale. The user is right that P20 should be grey
  with a `+`.
- **P19 is correct as rendered.** `19-VERIFICATION.md` frontmatter is
  `status: human_needed`, and `.planning/STATE.md` "Deferred Verification"
  records phase 19 as `verification_deferred_human`, deferred by explicit user
  decision on 2026-08-19. Disk infers `Executed`, which by design is *not*
  `Complete` (`disk_status.rs:60-66`: "Verification passed. The **only** value
  that admits `DiskStatus::Complete`"). An unchecked box is the honest state.
  Nothing to fix for P19 until `/gsd-verify-work 19` is run.

## Secondary defect found in the same block — the `*` marker can never fire here

`src/ui/screens/detail.rs:2604`:

```rust
let current_phase_num = (state.completed_phases + 1).to_string();
```

This compares an **ordinal count** against a phase **number**. For milestone v2.0
the phases are numbered 14–22, and `state.completed_phases` is overwritten by the
ROADMAP `## Progress` table (`src/state_reader/mod.rs:207-215`,
`roadmap_md::roadmap_progress` counts rows whose Status cell is `Complete`) — it
is 5. So `current_phase_num == "6"`, which matches no phase, and no line ever
gets `*` or the status colour. STATE.md's own `current_phase: 21` is parsed into
`state.current_phase` but is not consulted here. The same expression appears at
`detail.rs:2778` (roadmap viz), so that surface has the bug too. This is a real
rendering bug independent of the ROADMAP staleness, and it survives any milestone
that does not start at phase 1.

## Proposed fix — three separable pieces

1. **Data (do this first, it is the actual defect the user saw).** Tick phase 20:
   `- [x] **Phase 20: …** (completed 2026-08-19)` and flip its `## Progress` row
   to `Complete | 2026-08-19`. Prefer running the owning tool rather than hand
   editing — `gsd_run query roadmap.update-plan-progress 20` (or
   `phase.complete 20`), which is verification-gated and will refuse if it
   disagrees. Leave phase 19 alone.

2. **Process (this is why it will recur).** The autonomous / `--no-transition`
   path advanced STATE.md without running the ROADMAP half of the close-out. Add
   a close-out assertion: after a phase's verification reaches `passed`, the
   ROADMAP checkbox and Progress row must agree, or the run stops and says so.
   Without this, the same silent divergence recurs on every autonomously executed
   phase.

3. **Renderer (defence in depth, optional).** Do **not** make the marker prefer
   the disk-inferred stage outright — that would erase the deliberate
   `Executed` vs `Complete` distinction phase 20 built, and P19 would wrongly
   turn grey the moment its plans finished. The safe version is narrower: grey
   (and `+`) when `phase.completed || disk status == DiskStatus::Complete`,
   since `DiskStatus::Complete` already requires a passing verification. That is
   a two-line change at `detail.rs:2607` / `:2639`. Separately fix
   `current_phase_num` at `detail.rs:2604` and `:2778` to use the parsed
   `current_phase` number rather than `completed_phases + 1`.

Recommendation: do 1 and 2. Treat 3 as a follow-up — it makes the panel
self-correcting when tracking files go stale, which is a genuine improvement for
a tool whose whole job is reading other projects' planning state, but it must
keep `Executed` un-greyed.

## Will phase 21 hit this?

Yes, on the current evidence. P21's checkbox is `[ ]` and its Progress row is
`In Progress`; it is being executed by the same autonomous/execute-phase path
that skipped the ROADMAP write for P20. Unless fix 2 lands (or the operator runs
`phase.complete 21` explicitly at close), P21 will finish verification and render
`o … [Complete]` un-greyed exactly like P20 does now. Phase 19 will additionally
stay unchecked until its deferred human verification is actually run — that one
is correct and should not be "fixed" by ticking the box.
