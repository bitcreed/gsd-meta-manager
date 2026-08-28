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

## AMENDMENT — 2026-08-28 (`21-34` T3, round 11): the declaration above still holds

**Append-only. No line above this heading was changed, shortened or re-derived.**
The round-10 declaration was read in full and checked against round 11's actual
diff before this paragraph was written, and it comes out **byte-identical**:
`rtk proxy git diff -- COVERAGE.md` for this task shows **additions only**.

**Round 11 of phase 21 (`21-31`, `21-32`, `21-33`, `21-34`) likewise integrates
no external API, SDK or service.** Its entire diff changes:

- **one argv element's shape** — the untrusted `/proc`-scraped session id fused
  into a single `--resume=<id>` element instead of trailing a bare `--resume`
  (`21-31`, gaps[0], CWE-88);
- **two census algorithms and the claims that rest on them** — the interpreter
  census's join budget stops being spent by comment lines, and the composition
  census's verdict moves from the joined logical unit to the innermost call
  (`21-32`, gaps[1] and gaps[2]);
- **one sort comparator clause** — `.then_with(|| a.cmp(b))` appended to
  `backlog_number_ordering`, making it total over ELEMENTS rather than only over
  keys (`21-33`, WR-05);
- **a set of controls, fixtures and doc corrections** — a hostile-id corpus
  widened 18 → 28 with an option-lookalike block, a two-sided join-window
  boundary control, a laundering fixture, a local autoref-specialization trait
  probe, an input-echo render spot-check, a census rename with its reach pinned
  as a checked per-file equality, and the narrowed claims that go with each;
- **three documentation records** — `deferred-items.md`, this file, and a
  comment-only module note in `tests/driver_reattach.rs`.

**Not an API integration among them.** Nothing above adds a client, a base URL,
an auth exchange, a response schema or a retry policy, because round 11 adds no
interface at all — it changes how this tree hands an argument to a program it
already spawned, and what its own censuses are permitted to claim.

**The no-dependency claim, MEASURED rather than asserted.** Both of these were
run under `rtk proxy` over the whole round and both produced **no output**:

```
rtk proxy git diff 2c13fcf HEAD -- Cargo.toml Cargo.lock     (empty)
rtk proxy git diff 343c408 HEAD -- Cargo.toml Cargo.lock     (empty)
```

`2c13fcf` is round 11's base (the last commit before the first executor commit)
and `343c408` is the pre-round-11 gate baseline. An empty manifest diff over both
is a stronger check than naming any single crate, and it is what the round's own
no-new-dependency prohibition requires. **The `## Package Legitimacy Audit` gate
does not fire: no `cargo add`, no new dependency, no `Cargo.toml` or `Cargo.lock`
change in any of round 11's four plans.**

**The `claude` CLI subprocess seam is STILL deliberately not a matrix row**, and
round 11 sharpens rather than weakens the reason. `21-31` measured the receiving
binary's option parser directly — six probes against `claude 2.1.248 (Claude
Code)` — precisely because it is a **program whose argv this phase hardens**, not
an interface this round integrates. What changed on that path is *how an
untrusted value is bound to an option name*; no protocol, endpoint or schema
exists to write rows about. Writing them would be inventing coverage, which is
the manufactured-evidence failure phase 21 exists to end.

That measurement did, however, expose a real obligation that a coverage matrix
would never have caught: the parser's behaviour belongs to the CLI's **version**,
and nothing in this tree goes red when it changes. It is recorded as a standing
staleness obligation in `deferred-items.md`, in the shape of the ratatui one,
carrying the six probe commands and the measured version. **Direction:
under-detection, SILENT.**

**Criterion 4 is unchanged and is not claimed here**, for the **fourth**
consecutive round: the ten `#[ignore]`d `tests/driver_injection_corpus.rs` arms
need a human with an authenticated Claude subscription. Measured on the merged
tree — 13 passed / 0 failed / 10 ignored, file empty in the round diff. It is
**permanently agent-unclosable by construction, and 4/5 is the expected and
correct ceiling for this phase, not a failure.**

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Written: 2026-08-27 (`21-30` T3)*
*Amended: 2026-08-28 (`21-34` T3), append-only*
