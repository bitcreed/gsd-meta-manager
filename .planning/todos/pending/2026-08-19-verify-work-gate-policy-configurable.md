---
created: 2026-08-19T00:00:00.000Z
title: The verify-work gate policy must be configurable — skip | defer | auto-validate
area: driver
severity: major
files:
  - src/driver/router.rs
  - src/config.rs
---

## Problem

Phase 20's plan 20-04 made `/gsd-verify-work` a **hardcoded** exclusion from the router's safe
output alphabet. The reasoning was sound as far as it went — upstream GSD routes an `executed`
phase to `/gsd-verify-work`, and those verification statuses *are* the DRIVE-05 human-judgement
gate set, so auto-selecting the command would have the router answering the exact question the
gate exists to ask a human.

But "never auto-select" is one policy baked in as if it were a law. The user's decision
(2026-08-19) is that this must be a **user-configurable three-way policy**, because the right
answer depends on the project and the moment, not on the router:

| Mode | Meaning |
|------|---------|
| `skip` | Do not verify. Treat the phase as done for routing purposes and move on. The run does not stop here. |
| `defer` | Park the run with the gate named, for a human to pick up at a suitable time. **This is today's hardcoded behaviour** and should remain the default. |
| `auto` | Attempt auto-validation — the AI finds a way to act as the human would: controlled browser, app simulator, TUI driver, or whatever the project's surface requires. |

## Why this matters

- `defer`-always is what makes a fully autonomous run park at nearly every phase boundary.
  CONTEXT.md's OQ7 resolution accepted that consequence explicitly and asked for it to be
  *measured* rather than assumed (which is why `gate_verification_gaps_found` got its own
  greppable reason). This todo is the escape hatch that finding will need.
- `skip` is genuinely wanted for projects where verification is theatre, and genuinely dangerous
  where it is not — so it must be a recorded, visible choice, never a default and never implicit.
- `auto` is a real capability, not a flag: acting like a human against a TUI, a browser, or an
  app simulator is its own subsystem. It should NOT be smuggled in as a third enum arm that
  silently does something weaker than it claims.

## Proposed split

1. **Now (phase 20 follow-up plan, or phase 21):** introduce the config key and the
   `skip` / `defer` arms. `defer` stays the default so existing behaviour is unchanged. `auto`
   is accepted by the parser but **refused at runtime with a typed "not implemented" error that
   names the missing capability** — never silently downgraded to `defer`, which would be exactly
   the class of quiet lie phase 19 and 20 spent their effort eliminating.
2. **Its own phase:** implement `auto` — controlled browser / app simulator / TUI driver, plus
   the question of how an auto-validation result is recorded so it is distinguishable on disk
   from a human's verification. A machine-produced `status: passed` that is indistinguishable
   from a human-produced one would defeat the gate rather than automate it. Note the related
   open exposure already disclosed by 20-04: nothing mechanically stops a driven agent writing
   `status: passed` into a `*-VERIFICATION.md` today.

## Conformance-oracle consequence

20-04's conformance oracle records the verify-work exclusion as a **declared divergence from
upstream that fails in both directions** — including if the divergence ever stops diverging.
That test encodes the hardcoded policy. Whichever plan implements this todo must update the
oracle to assert the divergence *per configured mode*, or it will fail the moment `skip` is
selected. Do not weaken the oracle to accommodate the knob.

## Source

User decision, 2026-08-19, during the phase 20 autonomous run — in response to 20-04's report
that `/gsd-verify-work` had been excluded from the safe alphabet by fiat.
