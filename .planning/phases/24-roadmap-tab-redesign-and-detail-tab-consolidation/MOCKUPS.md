# Phase 24: Reference Mockups

Copied verbatim from `/tmp/rmux/{A,B,C}.txt` on 2026-09-23. These three designs were verified by the user.
They show the **pre-consolidation tab bar** (`1:Phases … 0:Docs`); the tab bar in the shipped phase follows CONTEXT.md D-B01 instead.
Mockup A uses invented demo data (`bookly`); B is daily-vow; C is sentriq at 80 columns (stacked detail pane).

## Mockup A

```text
 1:Phases │[2:Roadmap]│ 3:Backlog │ 4:Git │ 5:Pipe │ 6:Queue │ 7:Sess │ 8:Arch │ 9:Cfg │ 0:Docs                         
 bookly  ·  milestone M3 Live booking  ·  phase 10 in progress  ·  2 of 11 phases done                                  
┌─ Roadmap ────────────────────────────────────────────────── v list ┐┌─ Phase 12 ─────────────────────────────────────┐
│ Start now: ◉10 Slot picker UI (active) ║ ○11 Calendar sync         ││ Checkout & payments                            │
│  lanes   #  Phase                                       plans  wave││ M3 Live booking · wave 4 of 7                  │
│         M3 Live booking  2/8 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ││ ◌ blocked · 0/3 plans                          │
│ ●          8  Booking data model                          3/3   W1 ││                                                │
│ ●          9  Availability API                            4/4   W2 ││ Goal                                           │
│ ├─┐                                                                ││ A held slot can be paid for end to end, with   │
│ ◉ │    ↑  10  Slot picker UI                              2/3   W3 ││ refunds and an emailed receipt, before the     │
│ │ ○    ↑  11  Calendar sync                               0/2   W3 ││ hold expires.                                  │
│ ├─┘                                                                ││                                                │
│ ◌      ▶  12  Checkout & payments                         0/3   W4 ││ Needs     ◉ 10 Slot picker UI       2/3        │
│ ├─┐                                                                ││           ○ 11 Calendar sync        0/2        │
│ ◌ │    ↓  13  Confirmation flow                             —   W5 ││ Unblocks  ◌ 13 Confirmation flow               │
│ ├─┼─┐                                                              ││           ◌ 18 Web booking portal   M5         │
│ ◌ │ │     14  Reminders & notifications                     —   W6 ││ Parallel  none in wave 4                       │
│ ◌ │ │     15  Live booking launch                           —   W7 ││                                                │
│   │ │   M4 Support chat  0/2 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ││ ⏎ open in Phases   h/l follow edge             │
│   │ ◌     16  Chat backend                                  —   W6 ││                                                │
│   │ ◌     17  Chat UI                                       —   W7 ││                                                │
│   │     M5 Web  0/1 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ││                                                │
│   ◌    ↓  18  Web booking portal                            —   W5 ││                                                │
└────────────────────────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
```

## Mockup B

```text
 1:Phases │[2:Roadmap]│ 3:Backlog │ 4:Git │ 5:Pipe │ 6:Queue │ 7:Sess │ 8:Arch │ 9:Cfg │ 0:Docs                         
 daily-vow  ·  milestone v1.5 Closing the Loop  ·  phase 23 planning  ·  5 of 6 phases done                             
┌─ Roadmap ────────────────────────────────────────────────── v list ┐┌─ Phase 23 ─────────────────────────────────────┐
│ Start now: ◉23 Transparency, Reset & History (active)              ││ Transparency, Reset & History                  │
│  lanes   #  Phase                                       plans  wave││ v1.5 Closing the Loop · wave 5 of 5            │
│ ▸ v1.0 … v1.4   5 milestones · 17 phases shipped                   ││ ◉ active · planning · plans TBD                │
│         ▾ v1.5 Closing the Loop  5/6 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ││                                                │
│ ●         18  Calibration Foundation                      8/8   W1 ││ Goal                                           │
│ ●         19  Effective Profile Resolution                6/6   W2 ││ The user can see what the app has learned      │
│ ●      ·  20  Notification Scheduling Extraction & Det…   5/5   W3 ││ about the spouse, check its reasoning, undo    │
│ ●      ↑  21  Probes End-to-End                           5/5   W4 ││ it, and be reminded when the underlying        │
│ ├─┐                                                                ││ assessment has gone stale.                     │
│ ● │       22  Response-Weighted Nudge Selection           5/5   W5 ││                                                │
│   ◉    ▶  23  Transparency, Reset & History               0/?   W5 ││ Needs     ● 21 Probes End-to-End     5/5       │
│                                                                    ││           ● 20 Notification Sched…   5/5       │
│                                                                    ││             (implied via 21)                   │
│                                                                    ││ Unblocks  nothing (last in v1.5)               │
│                                                                    ││ Parallel  ● 22 Response-Weighted…  done        │
│                                                                    ││                                                │
│                                                                    ││ ⏎ open in Phases   h/l follow edge             │
└────────────────────────────────────────────────────────────────────┘└────────────────────────────────────────────────┘
```

## Mockup C

```text
 1:Ph │[2:Rd]│ 3:Bk │ 4:Gt │ 5:Pp │ 6:Qu │ 7:Ss │ 8:Ar │ 9:Cf │ 0:Dc            
 sentriq · v0.12 Actuation Routines · phase 9 planning · 0/4 done               
┌─ Roadmap ──────────────────────────────────────────────────────────── v list ┐
│ Start now: ◉9 Routine Event Logging (active) ║ ○12 Two-Truck Hardware Valid… │
│  lanes   #  Phase                                                 plans  wave│
│ ▸ earlier   pre-GSD 1–3 · v0.11 4–7 · TASK-111 (quick)                       │
│         ▾ v0.12 Actuation Routines  0/4 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ ◉          9  Routine Event Logging (Schema v18)                    0/2   W1 │
│ ◌         10  The Air Box Test on a Fake Transport                    —   W2 │
│ ◌         11  First Supervised On-Vehicle Run                         —   W3 │
│   ○    ▶  12  Two-Truck Hardware Validation                           —   W1 │
└──────────────────────────────────────────────────────────────────────────────┘
┌─ Phase 12 ───────────────────────────────────────────────────────────────────┐
│ Two-Truck Hardware Validation   v0.12 · wave 1 · ○ ready · plans TBD         │
│ Goal  On real hardware, both trucks work from one phone with separately      │
│       scoped data, and a first real F350 drive produces a logged session     │
│       and a completed DTC scan.                                              │
│ Needs     nothing declared   Unblocks  nothing                               │
│ Parallel  ◉ 9  ◌ 10  ◌ 11   (no edge either way; can run any time)           │
│ ⏎ open in Phases   h/l follow edge   j/k move                                │
└──────────────────────────────────────────────────────────────────────────────┘
```
