# Musk milestone pass — v2.3.0 Multiplexer Backends

Run 2026-11-10 (auto, from discuss-phase 33 discuss:pre; scope_cuts=defer). User unavailable; verdicts are [inferred].

| item | owner | verdict | M-id | reason |
|---|---|---|---|---|
| Phase 33 | user request 2026-11-10 (REQUIREMENTS.md Source) | keep | — | sole phase of the milestone |
| MUX-01 | user: "properly designed trait ... reused for other frameworks" | keep | — | core ask |
| MUX-02 | user: herdr nested in/around tmux; owned dependency of MUX-04/05 | keep | — | without it herdr-in-tmux picks the wrong backend |
| MUX-03 | user: tmux migrated onto seam; security mitigation (T-W0D-*) | keep | — | never-cut: security mitigation + tests guarding remaining behaviour |
| MUX-04 | user: herdr support (focus) | keep | — | core ask |
| MUX-05 | user: herdr support (launch); no-interpreter invariant | keep | — | core ask; refusal clause is a security mitigation |
| MUX-06 | owned dependency: existing fallback behaviour must remain | keep | — | guards existing behaviour |
| MUX-07 | user: tmux migrated onto seam | keep | — | defines "migrated" |
| Criteria 1-4 | trace to MUX-01..07 | keep | — | each maps to an owned requirement |

**Totals:** 9 items reviewed, 0 cut, 0 narrowed, 0 merged, 0 to backlog.

Run gsd-tools musk stats for the estimated savings.
