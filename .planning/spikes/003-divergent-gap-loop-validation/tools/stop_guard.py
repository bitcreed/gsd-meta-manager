#!/usr/bin/env python3
"""Spike 003 (Phase 26.1) D-03: Stop-hook keep-alive guard for the headless driver.

Phase 26 Run 1b: the driver dispatched one worker, the worker started its own background agent and ended
its turn, and the driver (told "the worker is running, I'll wait") ended ITS turn too. `claude -p` then
exited rc=0 with the worker's dispatch still unreported. This hook is a second `Stop` entry seeded by the
fork launcher (after the driverctl heartbeat). It blocks a premature end of the driver's turn only:

  stdin JSON (Stop hook payload)  ->  stdout: nothing (allow) or {"decision": "block", "reason": "..."}

Decision order (first match wins; every decision appends one line to $CLAUDE_CONFIG_DIR/stop-guard.log):
  unparseable stdin / no --run-dir        allow bad-input
  not a Stop event, or agent_id present   allow not-top-level   (only the top-level driver is guarded)
  background_tasks non-empty              allow background      (the session is waiting for background work)
  `RUN | end` in <run dir>/LOG.md         allow run-end
  no outstanding dispatch in the LOG      allow no-outstanding  (same definition as exitclass.outstanding_item)
  CLAUDE_CONFIG_DIR unset                 allow no-config-dir
  otherwise bump $CLAUDE_CONFIG_DIR/stop-guard.count: attempt <= GUARD_LIMIT -> block, else allow limit.

The guard only READS the run dir (read-only inside the sandbox) and writes only under CLAUDE_CONFIG_DIR
(read-write bind). It always exits 0: a broken guard must never wedge the driver.

Usage:
  stop_guard.py --run-dir RD        (Claude Code Stop hook)
  stop_guard.py selftest

Exit codes: hook mode always 0; selftest 0 ok, 1 failed.
"""
import json
import os
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:               # `python3 -I` drops the script directory from sys.path
    sys.path.insert(0, HERE)
from dup_check import parse_log  # noqa: E402
from exitclass import outstanding_item  # noqa: E402

# Claude Code itself overrides a block after eight consecutive hook continuations; stay well below it.
GUARD_LIMIT = 3


def _utc():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _reason(item):
    return (f"The dispatch for {item} has no report in LOG.md and no agent is running, so the worker ended "
            "early. Do NOT dispatch a new worker: that would be a duplicate. Use SendMessage to the SAME "
            "worker agentId from its 'Async agent launched' result and ask it to finish the step, commit, "
            "and reply with its STATUS report. If it cannot, follow Startup step 3 (recovered).")


def _bump(path):
    try:
        with open(path, encoding="utf-8") as f:
            n = int(f.read().strip() or 0)
    except (OSError, ValueError):
        n = 0
    n += 1
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(f"{n}\n")
    os.replace(tmp, path)
    return n


def guard(stdin_text, run_dir, cfg_dir):
    """Returns (stdout_text, decision_line_or_None). Appends the decision line under cfg_dir when set."""
    decision = None
    out = ""
    try:
        p = json.loads(stdin_text)
        if not isinstance(p, dict):
            raise ValueError("payload is not an object")
    except ValueError:
        p = None
        decision = "allow reason=bad-input"
    if decision is None and not run_dir:
        decision = "allow reason=bad-input"
    if decision is None and (p.get("hook_event_name") != "Stop" or p.get("agent_id")):
        decision = "allow reason=not-top-level"
    if decision is None and p.get("background_tasks"):
        decision = "allow reason=background"
    if decision is None:
        events = parse_log(os.path.join(run_dir, "LOG.md"))
        if any(i == "RUN" and e == "end" for _, i, e, _ in (events or [])):
            decision = "allow reason=run-end"
        else:
            item = outstanding_item(events)
            if item is None:
                decision = "allow reason=no-outstanding"
            elif not cfg_dir:
                decision = "allow reason=no-config-dir"
            else:
                n = _bump(os.path.join(cfg_dir, "stop-guard.count"))
                if n <= GUARD_LIMIT:
                    decision = f"block item={item} attempt={n}"
                    out = json.dumps({"decision": "block", "reason": _reason(item)})
                else:
                    decision = "allow reason=limit"
    line = f"{_utc()} {decision}"
    if cfg_dir:
        with open(os.path.join(cfg_dir, "stop-guard.log"), "a", encoding="utf-8") as f:
            f.write(line + "\n")
    return out, line


def hook_main(argv):
    run_dir = None
    if "--run-dir" in argv and argv.index("--run-dir") + 1 < len(argv):
        run_dir = argv[argv.index("--run-dir") + 1]
    try:
        text = sys.stdin.read()
    except (OSError, ValueError):
        text = ""
    cfg = os.environ.get("CLAUDE_CONFIG_DIR") or None
    try:
        out, _ = guard(text, run_dir, cfg)
    except Exception:  # noqa: BLE001 - fail open: a broken guard must never wedge the driver
        return 0
    if out:
        print(out)
    return 0


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["selftest"]:
        return selftest()
    return hook_main(argv)


# --------------------------------------------------------------------------- selftest

def _snapshot(d):
    out = {}
    for root, _, files in os.walk(d):
        for fn in files:
            p = os.path.join(root, fn)
            st = os.stat(p)
            out[os.path.relpath(p, d)] = (st.st_mtime_ns, st.st_size)
    return out


def selftest():
    spike = os.path.dirname(HERE)
    src = os.path.join(spike, "experiment", "results", "run1b", "LOG.md")
    stop = json.dumps({"hook_event_name": "Stop", "session_id": "s", "stop_hook_active": False,
                       "background_tasks": [], "last_assistant_message": "PLANTED-SECRET-c0ffee"})
    with tempfile.TemporaryDirectory() as td:
        rd = os.path.join(td, "rundir")
        cfg = os.path.join(td, "cfg")
        os.makedirs(rd)
        os.makedirs(cfg)
        with open(src, encoding="utf-8") as f, open(os.path.join(rd, "LOG.md"), "w", encoding="utf-8") as g:
            g.write(f.read())
        before = _snapshot(rd)
        log = os.path.join(cfg, "stop-guard.log")

        def lines():
            with open(log, encoding="utf-8") as f:
                return f.read().splitlines()

        def call(payload, expect_block, n_lines, run_dir=rd, cfg_dir=cfg):
            out, line = guard(payload, run_dir, cfg_dir)
            assert bool(out) == expect_block, (payload, out, line)
            if cfg_dir:
                assert len(lines()) == n_lines, lines()
            return out, line

        # the run1b shape: top-level Stop, no background work, unreported dispatch -> block attempts 1..3
        for attempt in (1, 2, 3):
            out, line = call(stop, True, attempt)
            d = json.loads(out)
            assert set(d) == {"decision", "reason"} and d["decision"] == "block", d
            assert "SendMessage" in d["reason"] and "SAME" in d["reason"] and "I-01" in d["reason"], d
            assert line.endswith(f"block item=I-01 attempt={attempt}"), line
        with open(os.path.join(cfg, "stop-guard.count"), encoding="utf-8") as f:
            assert f.read().strip() == "3"
        out, line = call(stop, False, 4)                     # fourth attempt: the limit lets the driver stop
        assert line.endswith("allow reason=limit"), line
        assert _snapshot(rd) == before, "stop_guard wrote under the run dir"
        with open(os.path.join(cfg, "stop-guard.count"), encoding="utf-8") as f:
            assert f.read().strip() == "4"
        # allow paths (fresh counter so the limit is not what allows them)
        os.remove(os.path.join(cfg, "stop-guard.count"))
        n = 4
        for label, payload, reason in (
                ("background", dict(json.loads(stop), background_tasks=[{"task_id": "a1"}]), "background"),
                ("subagent", dict(json.loads(stop), agent_id="a1"), "not-top-level"),
                ("other-event", dict(json.loads(stop), hook_event_name="SubagentStop"), "not-top-level")):
            n += 1
            out, line = call(json.dumps(payload), False, n)
            assert line.endswith(f"allow reason={reason}"), (label, line)
        for text in ("{not json", "", "[1, 2]"):
            n += 1
            out, line = call(text, False, n)
            assert line.endswith("allow reason=bad-input"), line
        assert not os.path.exists(os.path.join(cfg, "stop-guard.count")), "allow paths must not bump the counter"
        n += 1
        out, line = call(stop, False, n, run_dir="")
        assert line.endswith("allow reason=bad-input"), line
        # no config dir: allowed, nothing written anywhere
        out, line = guard(stop, rd, None)
        assert out == "" and line.endswith("allow reason=no-config-dir"), (out, line)
        assert len(lines()) == n
        # no LOG at all / report present -> no outstanding dispatch
        n += 1
        out, line = call(stop, False, n, run_dir=os.path.join(td, "nolog"))
        assert line.endswith("allow reason=no-outstanding"), line
        with open(os.path.join(rd, "LOG.md"), "a", encoding="utf-8") as f:
            f.write("- 2026-10-17T03:06:00Z | I-01 | report | status=done\n")
        before = _snapshot(rd)                               # snapshot after the test's own append
        n += 1
        out, line = call(stop, False, n)
        assert line.endswith("allow reason=no-outstanding"), line
        # RUN end -> allow even with the dispatch unreported
        with open(src, encoding="utf-8") as f, open(os.path.join(rd, "LOG.md"), "w", encoding="utf-8") as g:
            g.write(f.read() + "- 2026-10-17T03:08:00Z | RUN | end | reason=stop\n")
        before = _snapshot(rd)
        n += 1
        out, line = call(stop, False, n)
        assert line.endswith("allow reason=run-end"), line
        # the guard never writes under the run dir
        assert _snapshot(rd) == before, "stop_guard wrote under the run dir"
        # CLI under -I: block JSON on stdout, exit 0; invalid JSON exits 0 with no output
        with open(src, encoding="utf-8") as f, open(os.path.join(rd, "LOG.md"), "w", encoding="utf-8") as g:
            g.write(f.read())
        cfg2 = os.path.join(td, "cfg2")
        os.makedirs(cfg2)
        env = dict(os.environ, CLAUDE_CONFIG_DIR=cfg2)
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "--run-dir", rd], input=stop,
                           capture_output=True, text=True, env=env)
        assert r.returncode == 0 and json.loads(r.stdout)["decision"] == "block", r
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "--run-dir", rd], input="{oops",
                           capture_output=True, text=True, env=env)
        assert r.returncode == 0 and r.stdout == "", r
        with open(os.path.join(cfg2, "stop-guard.log"), encoding="utf-8") as f:
            assert len(f.read().splitlines()) == 2
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
