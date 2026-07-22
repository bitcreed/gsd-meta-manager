# A GSD Project Manager

A TUI command center to manage multiple GSD-run projects at once.
Provides general preferences to GSD and keeps track of project phases.

## Use Cases

### New Project

#### Create new project

Option to create a new project straight off the TUI
* Create directory based on name (propose one based on path and preview that so user can edit or confirm)
* Create git repo
* Import global settings (see below)
* Have the user enter their ideas of the project
* Kick of a research phase to complete the idea and then preseed CONCEPT.md
* Kick of /gsd:new-project with CONCEPT.md

#### Import and Extend Global Settings
Project A and B are both independent GSD projects that the user drives, now he wants to add a third project C.
Use general settings as base information for that project like "run as
autonomous as possible", how coarse the default project planning is and whether
to "Commit GSD's .planning to git"

This is GSD output detailing my choices for a standard project. This hardly ever changes.
```
● User answered Claude's questions:
  ⎿  · How do you want to work? → YOLO (Recommended)
     · How finely should scope be sliced into phases? → Coarse
     · Run plans in parallel? → Parallel (Recommended)
     · Commit planning docs to git? → Yes (Recommended)

───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────

←  ☒ Research  ☒ Plan Check  ☒ Verifier  ☒ AI Models  ✔ Submit  →

Review your answers

 ● Research before planning each phase? (adds tokens/time)
   → Yes (Recommended)
 ● Verify plans will achieve their goals? (adds tokens/time)
   → Yes (Recommended)
 ● Verify work satisfies requirements after each phase? (adds tokens/time)
   → Yes (Recommended)
 ● Which AI models for planning agents?
   → Quality
```

The researchers then runs

```
● ◆ Spawning 4 researchers in parallel...
    → Stack research
    → Features research
    → Architecture research
    → Pitfalls research
```

That each store to a markdown file in .planning

```
.planning/research/ARCHITECTURE.md
.planning/research/PITFALLS.md
```

So apparently gsd uses a tool to record its state (at least when called)
```
● Bash(node "$HOME/.claude/get-shit-done/bin/gsd-tools.cjs" state record-session --stopped-at "Phase 02 UAT round 2 + hotfixes complete" --resume-file
      ".planning/phase…)
  ⎿  {
       "recorded": true,
       "updated": [
     … +5 lines (ctrl+o to expand)

● Bash(node "$HOME/.claude/get-shit-done/bin/gsd-tools.cjs" commit "docs(state): record phase 02 UAT completion" --files .planning/STATE.md)
  ⎿  {
       "committed": true,
       "hash": "161ab21",
     … +2 lines (ctrl+o to expand)

● Phase 2 closed. Here's where things stand:
...
```

And then .planning/STATE.md has a tripple-dash gated toml entry at the top:
```
---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: planning
stopped_at: Phase 1 UI-SPEC approved
last_updated: "2026-03-25T04:22:49.224Z"
last_activity: 2026-03-24 — Roadmap created
progress:
  total_phases: 4
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# The rest is Markdown
```
and at the very end of the file
```
## Session Continuity

Last session: 2026-03-25T04:22:49.221Z
Stopped at: Phase 1 UI-SPEC approved
Resume file: .planning/phases/01-core-infrastructure/01-UI-SPEC.md
```

For `/gsd:resume` we get:
```
❯ /gsd:resume-work

  Searched for 1 pattern, read 4 files (ctrl+o to expand)

● Bash(cat .planning/HANDOFF.json 2>/dev/null; ls .planning/phases/*/.continue-here*.md 2>/dev/null; for plan in
      .planning/phases/*/*-PLAN.md; do summary="${plan/PLAN/…)
  ⎿  Incomplete: .planning/phases/08-signing-build-foundation/08-03-PLAN.md
     Incomplete: .planning/phases/09-crashlytics-privacy-compliance/09-03-PLAN.md
     Incomplete: .planning/phases/10-visual-assets-release-verification/10-01-PLAN.md
     … +2 lines (ctrl+o to expand)

● Let me check the phase 10 plans that were executed and the recent git history.

● Bash(git log --oneline -15)
  ⎿  344c317 docs(state): pause phase 10 at wave 1 checkpoint
     26cd126 Merge branch 'worktree-agent-a0bfdd93'
     fb8605b feat(10-02): clean up icon foreground and regenerate all platform icons
     … +12 lines (ctrl+o to expand)

  Read 2 files (ctrl+o to expand)

● ╔══════════════════════════════════════════════════════════════╗
  ║  PROJECT STATUS                                             ║
  ╠══════════════════════════════════════════════════════════════╣
  ║  Building: OEM towing spec database app (Ram 2000-2026)     ║
  ║                                                             ║
  ║  Milestone: v1.1 — App Store Submission                     ║
  ║  Phase: 10 of 11 — Visual Assets & Release Verification     ║
  ║  Progress: [██████████░░░░░░░░░░] ~44% (4/9 plans done)    ║
  ║                                                             ║
  ║  Last activity: 2026-03-25 — Paused at Wave 1 checkpoint    ║
  ╚══════════════════════════════════════════════════════════════╝

  ⚠️   Incomplete work detected:
    - Phase 8 Plan 03 — iOS distribution signing (needs Apple Dev license)
    - Phase 9 Plan 03 — Firebase project setup (manual checkpoint)
    - Phase 10 Plans 01, 02, 03 — Current phase, in progress

  📍 Where we left off:
    Phase 10 Wave 1 executed (plans 10-01 and 10-02 committed):
    - **10-01**: ProGuard/R8 rules added, release build enabled
    - **10-02**: Icon foreground cleaned up, all platform icons regenerated

    Both are marked `autonomous: false` — **awaiting device verification**
    before proceeding to Wave 2 (10-03: screenshot automation).

  ⚠️   Carried concerns:
    - iOS signing: Apple Developer Portal distribution cert status unknown
    - flutter_inappwebview v6.1.5: Apple Guideline 2.5.1 rejection risk

  What would you like to do?

  1. Verify Wave 1 on device — test release build + icon rendering, then mark 10-01/10-02 complete
  2. Skip verification, proceed to 10-03 — screenshot automation & feature graphic (/gsd:execute-phase 10)
  3. Review Phase 10 plans in detail
  4. Something else

✻ Worked for 49s
```

One setting I'd like to be able to auto-apply is namespacing for apps it should be us.bitcreed.${appCodeName}

### Plugins

Not just a TUI but also an extensible command center. E.g. a Telegram bridge
(not as part of this project's v1, but as a possible addition later)

## Quick Pane

Show which projects are active and in what stage, which require input/assistance, and maybe also which are dormant.
Add a way to archive dormant projects (check if cleanup tasks need to be done
first: commit changes, remove wip branches, tag, push etc and then evict them
from being tracked here)

## Workflow Tracking

This is the main element.
Visual (ASCII) flow-chart of each project. So scroll through projects and quickly see where each is at.

A cached view of not just /gsd:progress but a whole ASCII rendering of the project

## Visualize Upcoming Work

An option to add or edit future milestones, quick tasks, postpone manual UI
validation/verification, in a way that doesn't interfere with a running GSD
instance on a current milestone. It's a topic to research.

## Enqueue Work

While work is still in progress, already enqueue the next work items and track
what's been done so it can be checked/verified/tested.

Enqueuing work should not only be for tasks (quick tasks) but also for next phases.

This is the output of GSD at the end of an execution (implementation) phase:
```
● ✓ Phase 2: Admin Enhancements Complete

  Plans: 3/3 | Tests: 496 passing (48 new) | Regressions: None

  What was built

  - Company navigation — click company row → orders tab filtered, expandable rows with hover-cards, hybrid bulk actions
  - Maintenance bypass — admins skip overlay, see sticky amber warning banner
  - ShowPrices email — EmailJS notification to company admin on toggle ON, with gender salutation
  - Registration — 14-field form, VAT validation, createNewUserAndCompany CF, 3 emails

  Human verification needed

  6 items require live Firebase/EmailJS testing (maintenance overlay, email delivery, end-to-end registration, company nav, bulk actions).

  ---
  /gsd:progress — see updated roadmap
  /gsd:discuss-phase 3 — discuss next phase before planning
  /gsd:plan-phase 3 — plan next phase
  /gsd:verify-work 02 — run human acceptance testing

✻ Sautéed for 30m 53s
```

If I can already specify ahead of time that this specific phase or execution
phases in general with UI components need manual verification then it can
already prepare that by running `/gsd:verify-work 02`

## Research

### Hook into Claude Session

Check if we can find a good way to hook straight into a claude terminal session
running the project. Could be `screen`, could be `ssh`, launch it inside the
TUI?, get creative...

### Manage Separate States

How can we consistently manage the remote project state (as in different folders, not necessarily on different machines)
without blowing through unnecessarily many tokens by continuously running `/gsd:progress`
E.g. can we extract it straight from a project folder or should we hook into gsd somehow to output state info to a centralized folder?

## Tech Stack

### Best suited language

Python or Rust would be my favorite. What do you recommend?

