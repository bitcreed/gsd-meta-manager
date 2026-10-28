---
phase: 30
review: 30-REVIEW.md
titles: json
findings:
  - id: WR-08
    severity: warning
    disposition: fixed
    title: "`DIRECTION.lock` is opened by path with symlink-following and truncating-write semantics, the one run-directory open the hardening still skips"
  - id: WR-09
    severity: warning
    disposition: fixed
    title: "`run_loop`'s guarded read has no size cap, although every other run-directory reader enforces 1 MiB"
  - id: WR-10
    severity: warning
    disposition: fixed
    title: "A give-back `ENOENT` strands the human's newest save at a hidden temp name, where the next launch sweeps it"
  - id: IN-06
    severity: info
    disposition: fixed
    title: "macOS exchange failure on a volume without `RENAME_SWAP` surfaces under the Linux syscall name"
  - id: IN-08
    severity: info
    disposition: fixed
    title: "`contended > 0` is satisfied by any busy tool thread, so it does not show the read-to-exchange window was raced"
  - id: IN-09
    severity: info
    disposition: fixed
    title: "Exhausting the give-back leaves the newest save at the temp name and an older one at `DIRECTION.md`"
  - id: IN-05
    severity: info
    disposition: deferred
    title: "Body-relative `DIRECTION.md:<line>` in the TUI differs from `driver check` and the launcher's own refusal (carried, unchanged)"
  - id: IN-07
    severity: info
    disposition: skipped
    title: "At a zero row budget both lint popups drop every finding and leave a header that ends in a colon"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Run-directory files are opened without a file-type guard, although the phase hardens one reader against exactly that"
  - id: WR-07
    severity: warning
    disposition: fixed
    title: "The model rule is narrower than `claude --model`, so the form refuses `opus[1m]`-style names that the CLI accepts"
  - id: IN-04
    severity: info
    disposition: fixed
    title: "A lint popup whose findings get zero rows is forced into the clipped, unconfirmable state"
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
open: 0
total: 19
recorded: 2026-10-24T18:41:08.771Z
---

# Phase 30: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-08 | warning | fixed | fixed by quick 261028-n0z 670e099 (acquire_at opens with O_NOFOLLOW plus O_NONBLOCK and refuses a non-regular or multiply-linked file before flock; launcher.lock too) |
| WR-09 | warning | fixed | fixed by quick 261028-n0z 0f85617 (read_run_file capped at 1 MiB via take; run_loop refuses a write past the cap) |
| WR-10 | warning | fixed | fixed by quick 261028-n0z 44eef32 (a give-back ENOENT restores by no-replace rename; EEXIST runs another exchange round) |
| IN-06 | info | fixed | fixed by quick 261028-n12 886acc8 (exchange failure reports a portable atomic rename exchange label, with a filesystem hint for NOTSUP and INVAL) |
| IN-08 | info | fixed | fixed by quick 261028-n12 886acc8 (the contended gloss, sample comment and assertion message in driverctl_race.rs say what the counter measures; editor_race_naive_positive_control is the non-vacuity proof) |
| IN-09 | info | fixed | fixed by quick 261028-n12 886acc8 (the exhaustion error says the newest save is the one left at the temp, that the next launch's stale-temp sweep deletes it, and to copy it out; message only, no extra exchange adopted because eight racing saves within microseconds is not a realistic input) [inferred — cleanup 2026-10-28] |
| IN-05 | info | deferred | [INFERRED — audit] carried from the 2026-10-21 ledger: same finding re-reported 2026-10-23 with a "(carried, unchanged)" title suffix, which the ledger script read as an id reuse; only visible if DIRECTION.md changes between preflight and launch; body-relative numbering is the D-17/D-18 editor contract; relabel or re-base in a later cleanup (not in the current review) |
| IN-07 | info | skipped | thin disclosure, and only at the one or two heights where the list gets zero rows. The count stays in the header, which the review says is not a safety hole. Making y inert on the override would bring back the unconfirmable popup that quick 261021-101 (IN-04) closed [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-02 | warning | fixed | fixed by quick 261021-101 64f73e7 (run_loop opens DIRECTION.md and the exchanged temp through open_run_file) (not in the current review) |
| WR-07 | warning | fixed | fixed by quick 261021-101 a3201ba (model_name_ok admits [ and ]; relaunch_prefill shares the predicate) (not in the current review) |
| IN-04 | info | fixed | fixed by quick 261021-101 ce352c0 (both lint popups skip fit_list at a zero row budget) (not in the current review) |
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
