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
  no outstanding dispatch in the LOG      allow no-outstanding  (same definition as exitclass.outstanding_dispatch)
  CLAUDE_CONFIG_DIR unset                 allow no-config-dir
  otherwise bump the per-dispatch counter in $CLAUDE_CONFIG_DIR/stop-guard.count: attempt <= GUARD_LIMIT ->
  block, else allow limit.

The block budget is GUARD_LIMIT per outstanding dispatch, not per run (WR-02). `stop-guard.count` holds JSON
`{"key": "<item>@<event index>", "n": <blocks so far>}`. When the outstanding dispatch changes the guard
appends `reset from=<old key or -> to=<new key>` to stop-guard.log and counts from 1; when nothing is
outstanding any more it clears the counter and appends `reset from=<old key> to=-` before the allow line.

IN-01 trust boundary: `settings.json` and `stop-guard.count` live in the model-writable CLAUDE_CONFIG_DIR
(spike 002's trust boundary). A counter file that is unreadable, not JSON, a legacy bare integer, or holds a
missing key or a negative / non-integer `n` is read as `{"key": "", "n": 0}`. That clamp bounds a forged
counter to at most GUARD_LIMIT blocks per dispatch (a forged file can only restart the count, never extend
it); the 90-minute wall-clock cap is the backstop.

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
from exitclass import outstanding_dispatch  # noqa: E402

# Claude Code itself overrides a block after eight consecutive hook continuations; stay well below it.
GUARD_LIMIT = 3


def _utc():
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _reason(item):
    return (f"The dispatch for {item} has no report in LOG.md and no agent is running, so the worker ended "
            "early. Do NOT dispatch a new worker: that would be a duplicate. Use SendMessage to the SAME "
            "worker agentId from its 'Async agent launched' result and ask it to finish the step, commit, "
            "and reply with its STATUS report. If it cannot, follow Startup step 3 (recovered).")


def _read_count(path):
    """(key, n) from the counter file; anything not exactly {"key": str, "n": int >= 0} reads as ("", 0)."""
    try:
        with open(path, encoding="utf-8") as f:
            d = json.load(f)
    except (OSError, ValueError):
        return "", 0
    if not isinstance(d, dict):                      # legacy bare integer, list, string, ...
        return "", 0
    key, n = d.get("key"), d.get("n")
    if not isinstance(key, str) or isinstance(n, bool) or not isinstance(n, int) or n < 0:
        return "", 0
    return key, n


def _write_count(path, key, n):
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump({"key": key, "n": n}, f)
        f.write("\n")
    os.replace(tmp, path)


def guard(stdin_text, run_dir, cfg_dir):
    """Returns (stdout_text, decision_line_or_None). Appends the decision line under cfg_dir when set."""
    decision = None
    out = ""
    pre = []                              # `reset` lines, logged before the decision line
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
            found = outstanding_dispatch(events)
            count_path = os.path.join(cfg_dir, "stop-guard.count") if cfg_dir else None
            if found is None:
                decision = "allow reason=no-outstanding"
                if count_path:
                    old, n0 = _read_count(count_path)
                    if n0 > 0:
                        pre.append(f"reset from={old or '-'} to=-")
                        try:
                            os.remove(count_path)
                        except OSError:
                            pass
            elif not cfg_dir:
                decision = "allow reason=no-config-dir"
            else:
                item = found[0]
                key = f"{item}@{found[1]}"
                old, n = _read_count(count_path)
                if old != key:
                    pre.append(f"reset from={old or '-'} to={key}")
                    n = 0
                n = min(n + 1, GUARD_LIMIT + 1)   # past the limit the value no longer matters
                _write_count(count_path, key, n)
                if n <= GUARD_LIMIT:
                    decision = f"block item={item} attempt={n}"
                    out = json.dumps({"decision": "block", "reason": _reason(item)})
                else:
                    decision = "allow reason=limit"
    line = f"{_utc()} {decision}"
    if cfg_dir:
        with open(os.path.join(cfg_dir, "stop-guard.log"), "a", encoding="utf-8") as f:
            for r in pre:
                f.write(f"{_utc()} {r}\n")
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


def _selftest_budget(td, src, stop):
    """WR-02 / IN-01: per-dispatch budget, reset lines, and the tamper clamp."""
    rd = os.path.join(td, "rd-budget")
    os.makedirs(rd)
    logp = os.path.join(rd, "LOG.md")
    with open(src, encoding="utf-8") as f:
        base = f.read()

    def put(extra=""):
        with open(logp, "w", encoding="utf-8") as f:
            f.write(base + extra)

    def fresh(name):
        c = os.path.join(td, name)
        os.makedirs(c)
        return c

    def tail(c, k):
        with open(os.path.join(c, "stop-guard.log"), encoding="utf-8") as f:
            return f.read().splitlines()[-k:]

    def counter(c):
        with open(os.path.join(c, "stop-guard.count"), encoding="utf-8") as f:
            return json.load(f)

    rep_line = "- 2026-10-17T03:09:00Z | I-01 | report | status=done\n"
    redo = "- 2026-10-17T03:10:00Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=e1003bc | round=0 | dispatch=2/8 | phase=1\n"

    # three blocks, then limit, for one dispatch
    put()
    c = fresh("b1")
    kinds = []
    for _ in range(5):
        out, line = guard(stop, rd, c)
        kinds.append(("block" if out else "allow", line.split(" ", 1)[1]))
    assert [k for k, _ in kinds] == ["block"] * 3 + ["allow"] * 2, kinds
    assert [d for _, d in kinds][:3] == [f"block item=I-01 attempt={i}" for i in (1, 2, 3)], kinds
    assert kinds[3][1] == kinds[4][1] == "allow reason=limit", kinds
    key0 = counter(c)["key"]
    assert key0.startswith("I-01@") and counter(c)["n"] == 4, counter(c)

    # a later dispatch of the same item (new index): reset line, then attempt=1 again
    put(rep_line + redo)
    out, line = guard(stop, rd, c)
    assert out and line.endswith("block item=I-01 attempt=1"), line
    key1 = counter(c)["key"]
    assert key1 != key0 and counter(c)["n"] == 1, (key0, key1)
    assert tail(c, 2)[0].endswith(f"reset from={key0} to={key1}"), tail(c, 2)
    assert tail(c, 2)[1].endswith("block item=I-01 attempt=1"), tail(c, 2)

    # nothing outstanding any more: counter cleared with a `reset ... to=-` line before the allow line
    put(rep_line + redo + "- 2026-10-17T03:11:00Z | I-01 | report | status=done\n")
    out, line = guard(stop, rd, c)
    assert out == "" and line.endswith("allow reason=no-outstanding"), line
    assert tail(c, 2)[0].endswith(f"reset from={key1} to=-"), tail(c, 2)
    assert not os.path.exists(os.path.join(c, "stop-guard.count"))
    n_before = len(tail(c, 1000))
    guard(stop, rd, c)                                       # already clear: no second reset line
    assert len(tail(c, 1000)) == n_before + 1

    # IN-01 clamp: negative, garbage, legacy integer, wrong shapes -> attempt=1, never more than 3 blocks per key
    put()
    for label, content in (("neg", "-999\n"), ("garbage", "garbage\n"), ("legacy", "3\n"),
                           ("negjson", '{"key": "I-01@0", "n": -999}'), ("strn", '{"key": "I-01@0", "n": "1"}'),
                           ("boolean", '{"key": "I-01@0", "n": true}'), ("nokey", '{"n": 2}'),
                           ("list", "[1, 2]"), ("empty", "")):
        c = fresh("t-" + label)
        with open(os.path.join(c, "stop-guard.count"), "w", encoding="utf-8") as f:
            f.write(content)
        blocks = 0
        for i in range(6):
            out, line = guard(stop, rd, c)
            if out:
                blocks += 1
                if i == 0:
                    assert line.endswith("block item=I-01 attempt=1"), (label, line)
        assert blocks == 3, (label, blocks)
    # a forged count that names the live key and is within range is honoured only as far as the clamp allows
    c = fresh("t-forged-key")
    with open(os.path.join(c, "stop-guard.count"), "w", encoding="utf-8") as f:
        json.dump({"key": "I-01@%d" % outstanding_dispatch(parse_log(logp))[1], "n": 2}, f)
    out, line = guard(stop, rd, c)
    assert out and line.endswith("attempt=3"), line


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
            out, line = call(stop, True, attempt + 1)         # +1: the first block is preceded by a `reset` line
            d = json.loads(out)
            assert set(d) == {"decision", "reason"} and d["decision"] == "block", d
            assert "SendMessage" in d["reason"] and "SAME" in d["reason"] and "I-01" in d["reason"], d
            assert line.endswith(f"block item=I-01 attempt={attempt}"), line
        k0 = "I-01@%d" % outstanding_dispatch(parse_log(os.path.join(rd, "LOG.md")))[1]
        assert lines()[0].endswith(f"reset from=- to={k0}"), lines()
        with open(os.path.join(cfg, "stop-guard.count"), encoding="utf-8") as f:
            assert json.load(f) == {"key": k0, "n": 3}
        out, line = call(stop, False, 5)                     # fourth attempt: the limit lets the driver stop
        assert line.endswith("allow reason=limit"), line
        assert _snapshot(rd) == before, "stop_guard wrote under the run dir"
        with open(os.path.join(cfg, "stop-guard.count"), encoding="utf-8") as f:
            assert json.load(f) == {"key": k0, "n": 4}
        out, line = call(stop, False, 6)                     # stays at the limit; n does not run away
        assert line.endswith("allow reason=limit"), line
        with open(os.path.join(cfg, "stop-guard.count"), encoding="utf-8") as f:
            assert json.load(f) == {"key": k0, "n": 4}
        # allow paths (fresh counter so the limit is not what allows them)
        os.remove(os.path.join(cfg, "stop-guard.count"))
        n = 6
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
            assert len(f.read().splitlines()) == 3          # reset + block, then allow bad-input
        _selftest_budget(td, src, stop)
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
