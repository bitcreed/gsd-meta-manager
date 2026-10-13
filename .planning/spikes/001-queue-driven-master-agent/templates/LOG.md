# Driver log: {run-id}

<!--
DRIVER-OWNED, APPEND-ONLY. Humans and gsd-mm read it and never edit it.
The driver appends with a shell heredoc (`cat >> LOG.md <<'EOF'`, a single O_APPEND write) and
never rewrites the file with Write/Edit, so a reader can never see a torn or reordered history.

One event = one header line, then optional indented detail lines (the subagent report goes here).
Header grammar (pipe-separated, first four fields fixed):
  - {ISO-8601 UTC} | {item-id or RUN} | {event} | key=value | key=value ...
Events:
  RUN   start      driver=pid:{pid} session={claude-session-id} mode={interactive|headless|fresh-per-item}
  RUN   reconcile  found={n} items in dispatched state
  {id}  dispatched step={what was asked} base={git sha} round={n} dispatch={n}/{budget}
  {id}  report     status={done|partial|blocked|failed}   (followed by the indented <=15-line report)
  {id}  verified   done-when={met|not-met} evidence={short}
  {id}  status     state={pending|running|done|parked|failed} outcome={one line}
  {id}  needs-human question={one line}   (paired with status state=parked)
  {id}  recovered  action={resume|reset|park} reason={short}
  RUN   end        reason={all-done|all-parked|stop|pause|run-budget|fresh-exit}

Current item status = the last `status` event for that id (absent = pending).
needs-human is open when an item's last status is parked and DIRECTION.md's `answer:` is empty.
-->

- 2026-10-13T09:00:00Z | RUN | start | driver=pid:4242 session=0f1e... mode=headless
- 2026-10-13T09:00:02Z | RUN | reconcile | found=0
- 2026-10-13T09:00:05Z | I-01 | status | state=running
- 2026-10-13T09:00:05Z | I-01 | dispatched | step=/gsd-quick "add README usage line" base=3b5167b round=0 dispatch=1/4
- 2026-10-13T09:03:40Z | I-01 | report | status=done
    STATUS: done
    ITEM: I-01
    STEP: /gsd-quick add README usage line
    COMMITS: 9a1b2fa
    WHERE: branch=master worktree=none
    CHANGED: 1 file (README.md)
    CHECKS: grep -q "Usage" README.md -> ok
    DONE_WHEN: met - README.md line 3
    FINDINGS: 0 new
    NEEDS_HUMAN: none
    SUGGESTED_NEXT: none (advisory only)
- 2026-10-13T09:03:45Z | I-01 | verified | done-when=met evidence=grep-ok,commit=9a1b2fa
- 2026-10-13T09:03:45Z | I-01 | status | state=done outcome=README usage line committed in 9a1b2fa
- 2026-10-13T09:03:50Z | RUN | end | reason=all-done
