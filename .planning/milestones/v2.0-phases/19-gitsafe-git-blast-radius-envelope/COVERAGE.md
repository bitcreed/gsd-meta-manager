# API Coverage — GitHub REST (via `gh`), repository protection & pull-request surface

> Full coverage by default. Opt-outs are explicit, reasoned decisions.

The detector fired on this phase because D-26 performs a **read-only** probe of the remote's
ruleset / branch-protection state through `gh api`, and because SAFE-06 rate-caps
`gh pr create` / `gh api POST …/pulls`. The matrix below is the subtraction record for that
surface. Every opt-out reason is carried from `19-CONTEXT.md` and is a scope decision, not an
oversight: D-18 withholds `Administration` scope from the run credential by design, so every
protection-*mutating* capability is unreachable on purpose.

| capability | decision | reason |
|---|---|---|
| `GET repos/{owner}/{repo}/rulesets` | INTEGRATE | |
| `GET repos/{owner}/{repo}/branches/{branch}/protection` | INTEGRATE | fallback path when rulesets returns nothing or is unavailable (D-26) |
| `GET repos/{owner}/{repo}` (default branch resolution) | INTEGRATE | needed to know which branch to probe |
| `POST repos/{owner}/{repo}/rulesets` | OPT-OUT | applying protection needs `Administration` scope, which D-18 withholds from the run credential by design; deferred as an explicitly-consented separate credential (Deferred Ideas) |
| `PUT repos/{owner}/{repo}/rulesets/{id}` | OPT-OUT | same `Administration` scope withholding (D-18); the envelope detects and reports, never mutates |
| `DELETE repos/{owner}/{repo}/rulesets/{id}` | OPT-OUT | same `Administration` scope withholding (D-18) |
| `PUT repos/{owner}/{repo}/branches/{branch}/protection` | OPT-OUT | same `Administration` scope withholding (D-18) |
| `DELETE repos/{owner}/{repo}/branches/{branch}/protection` | OPT-OUT | same `Administration` scope withholding (D-18); removing protection is the inverse of this phase's purpose |
| `POST repos/{owner}/{repo}/pulls` (`gh pr create`) | OPT-OUT | Phase 19 **caps and records** PR creation, it never issues one; the driven agent is the caller and SAFE-06 is the gate (D-19, D-20) |
| `GET repos/{owner}/{repo}/pulls` | OPT-OUT | the ledger is the local, out-of-repo source of truth for the cap (D-19); a remote list read would add a network call to the `PreToolUse` critical path, which `<specifics>` forbids outright |
| `PATCH repos/{owner}/{repo}/pulls/{n}` | OPT-OUT | not needed — Phase 19 bounds PR *creation* rate; PR editing is not a blast-radius control |
| `POST repos/{owner}/{repo}/pulls/{n}/merge` | OPT-OUT | explicitly out of scope — merging is a human decision and no requirement in this phase names it |
| `gh auth token` | INTEGRATE | one of the `CredentialSource` command forms (D-18); read-only, never a literal token in `config.json` |
| GitLab `glab mr create` | OPT-OUT | classified and capped by the guard (D-19) but never issued; the ledger is platform-agnostic, the probe is GitHub-only and reports `unknown(non-GitHub remote)` elsewhere (D-26) |

## Non-integration note

Phase 19 is a **gate over** this API surface far more than a consumer of it. The only capability
the tool itself calls is the read-only protection probe; every write capability is opted out with
the same reason — the run credential deliberately lacks `Administration` scope (D-18), and
`unknown` is never rendered as `protected` (D-26).
