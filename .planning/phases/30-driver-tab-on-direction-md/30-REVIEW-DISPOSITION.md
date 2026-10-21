---
phase: 30
review: 30-REVIEW.md
titles: json
findings:
  - id: WR-02
    severity: warning
    disposition: open
    title: "Run-directory files are opened without a file-type guard, although the phase hardens one reader against exactly that"
  - id: WR-07
    severity: warning
    disposition: open
    title: "The model rule is narrower than `claude --model`, so the form refuses `opus[1m]`-style names that the CLI accepts"
  - id: IN-04
    severity: info
    disposition: open
    title: "A lint popup whose findings get zero rows is forced into the clipped, unconfirmable state"
  - id: IN-05
    severity: info
    disposition: deferred
    title: "Body-relative `DIRECTION.md:<line>` in the TUI now differs from `driver check` and the launcher's own refusal"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "The launch form accepts a model name that the detached launch then rejects"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "`steer_run` and `apply_edit` rewrite DIRECTION.md even when they changed nothing, then say \"Nothing was written\""
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "Two line-number bases are both labelled `DIRECTION.md:<line>`"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "The launch form puts the spend and \"keeps running\" disclosure last, where short terminals clip it first"
  - id: WR-06
    severity: warning
    disposition: fixed
    title: "`$VISUAL` / `$EDITOR` with arguments cannot be launched, and Phase 30 now depends on this path for every edit"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "`set_item_field` mishandles a heading on an unterminated last line"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "The slug input stays \"pending\" when the request is refused before it starts"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "The launch form and the launcher read the `control` word with different readers"
open: 3
total: 12
recorded: 2026-10-21T05:15:32.882Z
---

# Phase 30: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-02 | warning | open | - |
| WR-07 | warning | open | - |
| IN-04 | info | open | - |
| IN-05 | info | deferred | [inferred — audit] only visible if DIRECTION.md changes between preflight and launch; body-relative numbering is the D-17/D-18 editor contract; relabel or re-base in a later cleanup |
| WR-01 | warning | fixed | fixed by 30-09 9ad47bd (model_name_ok in commit_edit); re-review e05ad64 confirms (not in the current review) |
| WR-03 | warning | fixed | fixed by 30-07 4df4dd1 (Plan::Keep); re-review e05ad64 confirms (not in the current review) |
| WR-04 | warning | fixed | fixed by 30-09 9ad47bd for TUI paths; cross-tool remainder tracked as IN-05 (not in the current review) |
| WR-05 | warning | fixed | fixed by 30-09 65d04da (essential_lines first, fitted_body_lines); re-review e05ad64 confirms (not in the current review) |
| WR-06 | warning | fixed | fixed by 30-08 168d03e/8d6a763 (editor_command sh -c wrapper); re-review e05ad64 confirms (not in the current review) |
| IN-01 | info | fixed | fixed by 30-07 4df4dd1 (terminated from byte before segment); re-review e05ad64 confirms (not in the current review) |
| IN-02 | info | fixed | fixed by 30-09 7d8fd67 (refusal routes through finish_new_run); re-review e05ad64 confirms (not in the current review) |
| IN-03 | info | fixed | fixed by 30-09 9ad47bd (launch_preflight uses control_line); re-review e05ad64 confirms (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
