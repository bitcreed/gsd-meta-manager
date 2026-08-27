# Phase 21 — API Coverage Declaration

**Status: NO EXTERNAL API INTEGRATION.** This is a reasoned declaration in place
of a coverage matrix, and the reason below is mandatory exactly as an `OPT-OUT`
reason is — a declaration without one is an un-decided hole, not a decision.

## Declaration

**Round 10 of phase 21 (`21-27`, `21-28`, `21-29`, `21-30`) integrates no
external API, SDK or service.** Its entire diff changes:

- subprocess **argument construction** — an argv vector replacing a program
  string handed to a command interpreter (`21-27`, CR-01);
- **render-time escape composition** — which of two in-crate character classes is
  applied where a value reaches a terminal cell (`21-27`…`21-30`);
- an **edit buffer's type** — `String` → `ui::screens::EditBuffer` over
  `crate::text::Untrusted` (`21-30`);
- a **trait's return type** — `&'static str` → the crate-private
  `ui::screens::RenderDisposition` enum (`21-30`);
- a **sort comparator** — `partial_cmp(..).unwrap_or(Equal)` → `f64::total_cmp`
  over a finite-filtered key (`21-29`);
- and a set of **source censuses and probe fixtures**, which are test code.

No new dependency of any kind was added. `git diff Cargo.toml Cargo.lock` over
round 10 is **empty** — a stronger check than naming any single crate, and what
`21-30`'s prohibition 4 actually requires. In particular `unicode-width` was NOT
added; see the IN-02 residual in `deferred-items.md`.

## Why the `api-coverage` detector fired, and why both signals are false positives

The detector returned `detected: true` over the phase scope. Both of its signals
were inspected:

1. One matches a **prior plan's own no-integration declaration** — the detector
   matching the words of a previous statement that there is no integration.
2. The other matches the phrase **"wraps the real API"**, which in this tree
   refers to an internal Rust API (a carrier type wrapping a `String`), not to a
   network or vendor API.

## The `claude` CLI subprocess seam — deliberately NOT a matrix row

The one place this phase touches something external is the `claude` binary the
driver spawns. **It is a spawned program this phase HARDENS, not an API this
round INTEGRATES**, and the distinction is the reason this file carries no
fabricated rows for it:

- It is invoked as a **subprocess with an argv vector**, not called over a
  protocol. There is no client, no base URL, no auth token exchanged by this
  code, no response schema to version against, and no vendor SDK in
  `Cargo.toml`.
- The integration itself predates round 10 and is unchanged by it. What round 10
  changed on that path is how untrusted text is kept OUT of a program string
  (`21-27`) and how the model's own returned prose is escaped before it reaches
  a terminal cell (`21-28`).
- Writing matrix rows for endpoints, rate limits, error codes and retry policy
  here would be inventing coverage for an interface this round does not have.
  That is precisely the manufactured-evidence failure phase 21 exists to end.

Its behavioural half is tracked where it belongs and is **not claimed here**:
ROADMAP success criterion 4, whose ten `#[ignore]`d
`tests/driver_injection_corpus.rs` arms need a human with an authenticated
Claude subscription. It is re-surfaced verbatim and unchanged in
`deferred-items.md` for the third consecutive round, with no work claimed
against it, and it is **permanently agent-unclosable by construction**.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Written: 2026-08-27 (`21-30` T3)*
