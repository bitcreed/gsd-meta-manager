# Driver log: {run-id}

<!--
TOOL-WRITTEN, APPEND-ONLY. Only driverctl appends here (one O_APPEND write per event); the run dir is
read-only inside the sandbox, so neither the driver nor a worker can edit this file directly. The
driver records events with `driverctl log <event> ...`, which validates every field first. This file
starts with no events: this comment is its grammar.

One event = one header line, then optional indented detail lines (the worker report).
Header grammar (pipe-separated, first three fields fixed; the line starts with a dash and a UTC stamp):
  {ISO-8601 UTC} | {item-id or RUN} | {event} | key=value | key=value ...
Values are one line with no pipe. A key whose value is empty is omitted.

Written on the driver's request (`driverctl log ...`):
  RUN   start           mode={interactive|headless} loop_mode={loop|fresh-per-item} session={id}
  RUN   reconcile       found={n}
  {id}  dispatched      step={...} base={sha: tool-computed} round={n} dispatch={k: tool-counted}/{budget} phase={N}
  {id}  report          status={done|partial|blocked|failed} truncated=yes   (+ the <=15-line report, verbatim)
  {id}  verified        done-when={met|not-met} evidence={short}
  {id}  status          state={pending|running|done|parked|failed} rule={budget|gap-rounds|divergence|no-progress|needs-human|scope|run-budget|launcher-cap} outcome={one line}
  {id}  needs-human     question={one line}
  {id}  recovered       action={done|resume|reset|park} reason={short}
  RUN   end             reason={all-done|all-parked|stop|pause|run-budget|fresh-exit}

Written by the launcher and the hooks, never by the model:
  RUN   launch          n={k} mode={standalone|...} model=... prompt={driver-prompt/X.Y.Z|custom:hash}
  RUN   connect         session={claude session id} source=startup                (SessionStart hook)
  RUN   disconnect      session=... reason=...                                    (SessionEnd hook)
  RUN   gap-observed    phase={N} count={k} file=... source=driverctl-scan
  RUN   gap-coalesced   phase={N} count={k} file=...
  RUN   dispatch-denied reason=...                                                (PreToolUse gate)
  RUN   cap-tripped     cap={dispatches|wall_clock_min|gap_rounds|operator-signal} used=.. limit=.. action=kill-scope
  RUN   crash-detected  previous_launch={k} evidence=...
  RUN   launch-end      rc=.. status={running|paused|done|parked|stopped|stalled|crashed|...} reason=..
  RUN   auth-relaunch   attempt={k} source=host
  RUN   push            dest=refs/heads/gsd-auto/... sha=... outcome=...
  RUN   pr              url=...
  RUN   push-refused    reason=... dest=... findings=...
  RUN   push-failed     dest=... detail=...
  RUN   pr-refused      reason=...
  RUN   pr-failed       detail=...
  RUN   pr-skipped      reason=not-github
  RUN   direction-lock-timeout                                                    (DIRECTION.lock not taken in time)
  RUN   skipped-insane-body                                                       (DIRECTION.md body lacks a required section)
  RUN   splice-gave-up                                                            (status block not written: a human save won the race)

Current item status = the last `status` event for that id (absent = pending).
-->
