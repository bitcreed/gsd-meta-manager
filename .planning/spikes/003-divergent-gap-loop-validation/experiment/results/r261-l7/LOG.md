# Driver log: s003-r261-l7

<!--
TOOL-WRITTEN, APPEND-ONLY. Only driverctl appends here (one O_APPEND write per event); the run dir is
read-only inside the sandbox, so neither the driver nor a worker can edit this file directly. The
driver records events with `driverctl log <event> ...`, which validates every field first.

One event = one header line, then optional indented detail lines (the worker report).
Header grammar (pipe-separated, first three fields fixed):
  - {ISO-8601 UTC} | {item-id or RUN} | {event} | key=value | key=value ...

Written on the driver's request (`driverctl log ...`):
  RUN   start        mode={interactive|headless} loop_mode={loop|fresh-per-item} session={id}
  RUN   reconcile    found={n}
  {id}  dispatched   step={...} base={sha: tool-computed} round={n} dispatch={k: tool-counted}/{budget} phase={N}
  {id}  report       status={done|partial|blocked|failed}   (+ the <=15-line report, verbatim)
  {id}  verified     done-when={met|not-met} evidence={short}
  {id}  status       state={pending|running|done|parked|failed} rule={budget|gap-rounds|divergence|no-progress|needs-human|scope|run-budget|launcher-cap} outcome={one line}
  {id}  needs-human  question={one line}
  {id}  recovered    action={done|resume|reset|park} reason={short}
  RUN   end          reason={all-done|all-parked|stop|pause|run-budget|fresh-exit}

Written by the launcher / hooks, never by the model:
  RUN   launch          n={k} model=... loop_mode=... sandbox=... unit=... caps=...
  RUN   connect         session={claude session id} source=startup      (SessionStart hook)
  RUN   disconnect      session=... reason=...                          (SessionEnd hook)
  RUN   gap-observed    phase={N} count={k} file=... source=launcher-scan
  RUN   dispatch-denied reason=...                                      (PreToolUse gate)
  RUN   cap-tripped     cap={dispatches|wall_clock_min|gap_rounds|operator-signal} used=.. limit=.. action=kill-scope
  RUN   crash-detected  previous_launch={k} evidence=...
  RUN   launch-end      rc=.. tripped=.. cost_usd=.. status=..

Current item status = the last `status` event for that id (absent = pending).
-->

- 2026-10-18T01:46:27Z | RUN | launch | n=1 | model=haiku | loop_mode=loop | sandbox=bwrap | unit=gsdmm-s003-r261-l7-1792287987.scope | caps=dispatches=5,minutes=10,gap_rounds=2
- 2026-10-18T01:46:27Z | RUN | connect | session=f775d45f-0e96-4e50-96c9-7e63b6957dd9 | source=startup
- 2026-10-18T01:46:31Z | RUN | start | mode=headless | loop_mode=loop | session=f775d45f-0e96-4e50-96c9-7e63b6957dd9
- 2026-10-18T01:46:40Z | I-01 | verified | done-when=not-met | evidence=feed-lines before=21 after=27
- 2026-10-18T01:46:43Z | RUN | gap-observed | phase=1 | count=1 | file=.planning/phases/01-slugify/01-VERIFICATION.md | source=launcher-scan
- 2026-10-18T01:47:33Z | RUN | gap-observed | phase=1 | count=2 | file=.planning/phases/01-slugify/01-VERIFICATION.md | source=launcher-scan
- 2026-10-18T01:48:31Z | RUN | gap-observed | phase=1 | count=3 | file=.planning/phases/01-slugify/01-VERIFICATION.md | source=launcher-scan
- 2026-10-18T01:49:20Z | RUN | dispatch-denied | reason=phase 1 has failed verification 3 times (gap-round cap 2); no further dispatch for phase 1. Park the item with --rule gap-rounds.
- 2026-10-18T01:49:25Z | RUN | gap-observed | phase=1 | count=4 | file=.planning/phases/01-slugify/01-VERIFICATION.md | source=launcher-scan
- 2026-10-18T01:49:25Z | RUN | cap-tripped | cap=gap_rounds | used=4 | limit=2 | action=kill-scope
- 2026-10-18T01:49:25Z | RUN | launch-end | rc=-15 | tripped=gap_rounds | cost_usd=0.007907299999999999 | status=stopped
