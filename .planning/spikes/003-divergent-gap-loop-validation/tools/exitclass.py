#!/usr/bin/env python3
"""Spike 003 (Phase 26.1) D-03: tell a stalled driver from a crashed one.

Phase 26 Run 1b ended with `launch-end | rc=0 | status=crashed`: the headless driver finished a turn with
`result subtype=success` while its worker was still running, so the process exited cleanly with no
`RUN end`. Spike 002's launcher has one non-crash path (`ended_by_driver`) and labelled everything else
`crashed` (`launcher.py:362-363`). This module is the single definition of the distinction, shared by the
fork launcher (`classify_exit`) and the Stop guard (`outstanding_dispatch`, so both agree on what "a dispatch
with no report" means).

  outstanding_dispatch(events)  (item, event index) of the latest open dispatch, or None. A dispatch is open
                            until a later `report`, `recovered` or `status` done|parked|failed for its item;
                            a `RUN | dispatch-denied` cancels only the immediately preceding dispatch (WR-01)
  outstanding_item(events)  the item of outstanding_dispatch
  classify_exit(...)        None when the existing status stands (cap tripped / driver logged `RUN end`);
                            ("stalled", reason) when rc == 0, the stream ended in `result success` and a
                            dispatch is outstanding; otherwise ("crashed", reason)

Usage:
  exitclass.py selftest
  exitclass.py replay --results DIR [--stream SKELETON]
  exitclass.py extract-skeleton STREAM

Exit codes: 0 ok, 1 check failed, 2 no data or bad input.

`replay` prints `EXIT_CLASS <status>`: the recorded status when the LOG has `RUN end` or the launcher result
has a tripped cap, otherwise the classification, followed by `OUTSTANDING item=<id|none>` and
`RESULT_OK yes|no`. `extract-skeleton` prints a sanitized JSONL skeleton of a `claude -p` stream-json run:
only allowlisted structural keys (ids, types, subtypes, statuses, tool names, booleans, text lengths); no
message text, prompt, command line or path is ever emitted.

`stalled` is not in driverctl's RUN_STATUS_ENUM; driverctl does not validate it (26.1-RESEARCH A2).
Forward note for Phase 27: the typed driver-status enum needs the `stalled` variant.
"""
import contextlib
import glob
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:               # `python3 -I` drops the script directory from sys.path
    sys.path.insert(0, HERE)
from dup_check import parse_log  # noqa: E402  (one LOG parser; no fourth EV_RE copy)

DONE_STATES = ("done", "parked", "failed")
SAFE_VALUE = re.compile(r"^[A-Za-z0-9._:-]{1,80}$")
SECRET_RE = re.compile(r"sk-ant|oat01|accessToken|refreshToken|eyJ")
SKELETON_KEYS = ("type", "subtype", "task_id", "status", "spawn_depth", "parent_task_id", "tool_use_id",
                 "tasks", "blocks", "parent_tool_use_id_set")


def outstanding_dispatch(events):
    """The latest open dispatch as (item, event index), or None (WR-01).

    Every item-level `dispatched` opens an entry, and a `report`, `recovered` or terminal `status` closes all
    open entries of that item. A `RUN | dispatch-denied` cancels exactly one entry: the most recent open one,
    and only when no item-level event (any event whose item is not RUN) lies between that dispatch and the
    denial. RUN-level launcher lines (gap-observed, connect, launch, ...) never count as "between", so they
    cannot shield a denied attempt. A denial therefore cannot hide a different, genuinely unreported dispatch.
    """
    open_entries = []            # [(event index, item)] in dispatch order
    last_item_event = -1         # index of the latest item-level event seen so far
    for n, (_, item, ev, kv) in enumerate(events or []):
        if item != "RUN":
            if ev == "dispatched":
                open_entries.append((n, item))
            elif ev in ("report", "recovered") or (ev == "status" and kv.get("state") in DONE_STATES):
                open_entries = [e for e in open_entries if e[1] != item]
            last_item_event = n
        elif ev == "dispatch-denied":
            if open_entries and last_item_event == open_entries[-1][0]:
                open_entries.pop()
    if not open_entries:
        return None
    n, item = max(open_entries)
    return item, n


def outstanding_item(events):
    """The item of the latest open dispatch (see outstanding_dispatch), or None."""
    found = outstanding_dispatch(events)
    return found[0] if found else None


def classify_exit(rc, tripped, ended_by_driver, result_ok, outstanding):
    """None keeps the existing status; otherwise (status, reason)."""
    if tripped or ended_by_driver:
        return None
    if rc == 0 and result_ok and outstanding:
        return ("stalled", f"driver ended its turn with a worker outstanding (dispatch {outstanding} has no "
                           "report in LOG.md); the process exited cleanly, so this is not a crash")
    return ("crashed", f"driver exited rc={rc} without RUN end")


# --------------------------------------------------------------------------- replay

def _last_result(results):
    paths = sorted(glob.glob(os.path.join(results, "launcher-result-*.json")),
                   key=lambda p: int(re.search(r"-(\d+)\.json$", p).group(1))
                   if re.search(r"-(\d+)\.json$", p) else 0)
    if not paths:
        return None
    try:
        with open(paths[-1], encoding="utf-8") as f:
            d = json.load(f)
        return d if isinstance(d, dict) else None
    except (OSError, ValueError):
        return None


def _skeleton_result_ok(path):
    """True/False from the last `result` line of a skeleton, None when there is none or it is unreadable."""
    last = None
    try:
        with open(path, encoding="utf-8") as f:
            for line in f:
                try:
                    d = json.loads(line)
                except ValueError:
                    continue
                if isinstance(d, dict) and d.get("type") == "result":
                    last = d.get("subtype")
    except OSError:
        return None
    return None if last is None else last == "success"


def replay(results, stream=None):
    """Returns (rc, lines)."""
    events = parse_log(os.path.join(results, "LOG.md"))
    res = _last_result(results)
    if events is None or res is None:
        return 2, ["exitclass: ERROR need LOG.md and a launcher-result-*.json in the results dir"]
    recorded = res.get("status")
    if any(i == "RUN" and e == "end" for _, i, e, _ in events) or res.get("tripped"):
        return 0, [f"EXIT_CLASS {recorded}"]
    result_ok = _skeleton_result_ok(stream) if stream else None
    if result_ok is None:
        result_ok = ((res.get("cost") or {}).get("subtype") == "success")
    item = outstanding_item(events)
    verdict = classify_exit(res.get("rc"), res.get("tripped"), False, result_ok, item)
    status = verdict[0] if verdict else recorded
    return 0, [f"EXIT_CLASS {status}", f"OUTSTANDING item={item or 'none'}",
               f"RESULT_OK {'yes' if result_ok else 'no'}"]


# --------------------------------------------------------------------------- extract-skeleton

def _safe(v):
    return v if isinstance(v, str) and SAFE_VALUE.match(v) and not SECRET_RE.search(v) else None


def _block(b):
    if not isinstance(b, dict):
        return None
    t = _safe(b.get("type"))
    if t is None:
        return None
    out = {"type": t}
    if t == "tool_use":
        name = _safe(b.get("name"))
        if name:
            out["name"] = name
        inp = b.get("input")
        out["run_in_background"] = bool(isinstance(inp, dict) and inp.get("run_in_background"))
    elif t == "tool_result":
        tid = _safe(b.get("tool_use_id"))
        if tid:
            out["tool_use_id"] = tid
    elif t == "text":
        txt = b.get("text")
        out["text_len"] = len(txt) if isinstance(txt, str) else 0
    return out


def skeleton_line(d):
    """One sanitized object for one stream event, or None when the event carries nothing structural."""
    if not isinstance(d, dict):
        return None
    out = {}
    for k in ("type", "subtype", "task_id", "status", "parent_task_id", "tool_use_id"):
        v = _safe(d.get(k))
        if v:
            out[k] = v
    if isinstance(d.get("spawn_depth"), int) and not isinstance(d.get("spawn_depth"), bool):
        out["spawn_depth"] = d["spawn_depth"]
    if d.get("subtype") == "background_tasks_changed" and isinstance(d.get("tasks"), list):
        ids = []
        for t in d["tasks"]:
            tid = _safe(t.get("task_id") if isinstance(t, dict) else t)
            if tid:
                ids.append(tid)
        out["tasks"] = ids
    msg = d.get("message")
    if d.get("type") in ("assistant", "user") and isinstance(msg, dict) and isinstance(msg.get("content"), list):
        blocks = [x for x in (_block(b) for b in msg["content"]) if x]
        out["blocks"] = blocks
    if "parent_tool_use_id" in d:
        out["parent_tool_use_id_set"] = bool(d.get("parent_tool_use_id"))
    return out or None


def extract_skeleton(stream):
    out = []
    with open(stream, encoding="utf-8", errors="replace") as f:
        for line in f:
            if not line.startswith("{"):
                continue
            try:
                d = json.loads(line)
            except ValueError:
                continue
            s = skeleton_line(d)
            if s:
                out.append(s)
    return out


# --------------------------------------------------------------------------- CLI

def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__.split("Usage:")[1].split("Exit codes:")[0].rstrip())
        return 2
    cmd, rest = argv[0], argv[1:]
    if cmd == "selftest":
        return selftest()
    if cmd == "replay":
        opts = {"--results": None, "--stream": None}
        i = 0
        while i < len(rest):
            if rest[i] in opts and i + 1 < len(rest):
                opts[rest[i]] = rest[i + 1]
                i += 2
            else:
                print(f"exitclass: ERROR bad argument {rest[i]!r}")
                return 2
        if not opts["--results"]:
            print("exitclass: ERROR --results is required")
            return 2
        rc, lines = replay(opts["--results"], opts["--stream"])
        print("\n".join(lines))
        return rc
    if cmd == "extract-skeleton":
        if len(rest) != 1 or not os.path.isfile(rest[0]):
            print("exitclass: ERROR extract-skeleton needs one stream path", file=sys.stderr)
            return 2
        for s in extract_skeleton(rest[0]):
            print(json.dumps(s, separators=(",", ":")))
        return 0
    print(f"exitclass: ERROR unknown command {cmd!r}")
    return 2


# --------------------------------------------------------------------------- selftest

MARKER = "PLANTED-SECRET-c0ffee"
TOKEN = "sk-ant-oat01-PLANTEDTOKEN0123456789"


def _ev(utc, item, ev, **kv):
    return (utc, item, ev, {k: str(v) for k, v in kv.items()})


def selftest():
    # classify_exit matrix
    assert classify_exit(0, None, False, True, "I-01")[0] == "stalled"
    assert "I-01" in classify_exit(0, None, False, True, "I-01")[1]
    assert classify_exit(1, None, False, True, "I-01")[0] == "crashed"          # rc != 0
    assert classify_exit(0, "gap_rounds", False, True, "I-01") is None          # tripped cap keeps its status
    assert classify_exit(0, None, True, True, None) is None                     # driver logged RUN end
    assert classify_exit(0, None, False, True, None)[0] == "crashed"            # nothing outstanding
    assert classify_exit(0, None, False, False, "I-01")[0] == "crashed"         # stream did not end in success
    assert classify_exit(-9, None, False, True, "I-01")[0] == "crashed"         # killed by signal
    # outstanding_item
    d = _ev("t1", "I-01", "dispatched", step="s")
    assert outstanding_item([d]) == "I-01"
    assert outstanding_item([d, _ev("t2", "I-01", "report", status="done")]) is None
    assert outstanding_item([d, _ev("t2", "I-01", "status", state="parked")]) is None
    assert outstanding_item([d, _ev("t2", "I-01", "status", state="failed")]) is None
    assert outstanding_item([d, _ev("t2", "I-01", "status", state="running")]) == "I-01"
    assert outstanding_item([d, _ev("t2", "I-01", "recovered", action="park")]) is None
    assert outstanding_item([d, _ev("t2", "RUN", "dispatch-denied", reason="x")]) is None
    assert outstanding_item([_ev("t0", "RUN", "dispatch-denied", reason="x"), d]) == "I-01"   # denial BEFORE the dispatch
    assert outstanding_item([d, _ev("t2", "I-01", "report", status="done"),
                             _ev("t3", "I-01", "dispatched", step="s2")]) == "I-01"          # re-dispatch reopens
    assert outstanding_item([d, _ev("t2", "I-02", "dispatched", step="s")]) == "I-02"        # latest dispatch wins
    assert outstanding_item([]) is None and outstanding_item(None) is None
    # WR-01: a denial cancels only its own immediately preceding dispatch
    den = _ev("t9", "RUN", "dispatch-denied", reason="x")
    d2 = _ev("t2", "I-01", "dispatched", step="s2")
    assert outstanding_dispatch([d, d2, den]) == ("I-01", 0)                                 # (a)
    assert outstanding_dispatch([d, den]) is None                                            # (b)
    assert outstanding_dispatch([d, _ev("t2", "I-02", "dispatched", step="s"), den]) == ("I-01", 0)   # (c)
    assert outstanding_dispatch([d, _ev("t2", "RUN", "gap-observed", n=1), den]) is None     # (d)
    assert outstanding_dispatch([d, _ev("t2", "I-01", "report", status="done")]) is None     # (e)
    assert outstanding_dispatch([d, _ev("t2", "I-01", "report", status="done"), d2, den,
                                 _ev("t5", "I-01", "dispatched", step="s3")]) == ("I-01", 4)  # (f)
    assert outstanding_dispatch([d, _ev("t2", "I-01", "status", state="running"), den]) == ("I-01", 0)  # item event between
    assert outstanding_dispatch([]) is None and outstanding_dispatch(None) is None

    spike = os.path.dirname(HERE)
    run1b = os.path.join(spike, "experiment", "results", "run1b")
    run1 = os.path.join(spike, "experiment", "results", "run1")
    skel = os.path.join(spike, "experiment", "replay", "run1b-stream-skeleton.jsonl")
    outs = []
    with tempfile.TemporaryDirectory() as td:
        # replay over temp copies of the committed Run 1b / Run 1 files
        for name, src, want in (("run1b", run1b, "stalled"), ("run1", run1, "parked")):
            dst = os.path.join(td, name)
            os.makedirs(dst)
            for fn in ("LOG.md", "launcher-result-1.json"):
                shutil.copy(os.path.join(src, fn), os.path.join(dst, fn))
            rc, lines = replay(dst)
            assert rc == 0 and lines[0] == f"EXIT_CLASS {want}", (name, rc, lines)
            if name == "run1b":
                assert lines[1] == "OUTSTANDING item=I-01" and lines[2] == "RESULT_OK yes", lines
                if os.path.isfile(skel):
                    rc, lines = replay(dst, skel)
                    assert rc == 0 and lines[0] == "EXIT_CLASS stalled" and lines[2] == "RESULT_OK yes", lines
        # synthetic negative controls built from the run1b shape: never `stalled`
        base = os.path.join(td, "run1b")
        with open(os.path.join(base, "launcher-result-1.json"), encoding="utf-8") as f:
            res = json.load(f)
        with open(os.path.join(base, "LOG.md"), encoding="utf-8") as f:
            log = f.read()

        def control(name, rc_, tripped=None, log_extra="", cost_subtype="success"):
            c = os.path.join(td, name)
            os.makedirs(c)
            r = dict(res, rc=rc_, tripped=tripped, cost=dict(res["cost"], subtype=cost_subtype))
            with open(os.path.join(c, "launcher-result-1.json"), "w", encoding="utf-8") as f:
                json.dump(r, f)
            with open(os.path.join(c, "LOG.md"), "w", encoding="utf-8") as f:
                f.write(log + log_extra)
            return replay(c)
        rc, lines = control("rc1", 1)
        assert lines[0] == "EXIT_CLASS crashed", lines
        rc, lines = control("tripped", 0, tripped="gap_rounds")
        assert lines[0] == f"EXIT_CLASS {res['status']}", lines            # recorded status kept, not stalled
        rc, lines = control("reported", 0, log_extra="- 2026-10-17T03:07:00Z | I-01 | report | status=done\n")
        assert lines[0] == "EXIT_CLASS crashed" and lines[1] == "OUTSTANDING item=none", lines
        rc, lines = control("denied", 0, log_extra="- 2026-10-17T03:07:00Z | RUN | dispatch-denied | reason=x\n")
        assert lines[0] == "EXIT_CLASS crashed", lines
        rc, lines = control("nosuccess", 0, cost_subtype="error_max_turns")
        assert lines[0] == "EXIT_CLASS crashed" and lines[2] == "RESULT_OK no", lines
        assert replay(os.path.join(td, "missing"))[0] == 2

        # extract-skeleton: planted marker / prompt / token must not reach the output
        stream = os.path.join(td, "stream.jsonl")
        rows = [
            {"type": "system", "subtype": "init", "cwd": f"/home/{MARKER}", "tools": [MARKER], "session_id": MARKER},
            {"type": "assistant", "parent_tool_use_id": None, "message": {"content": [
                {"type": "text", "text": MARKER + " " + TOKEN},
                {"type": "tool_use", "name": "Agent", "id": "toolu_1",
                 "input": {"prompt": MARKER, "description": TOKEN, "run_in_background": True}}]}},
            {"type": "user", "parent_tool_use_id": "toolu_1", "message": {"content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": MARKER}]}},
            {"type": "system", "subtype": "task_started", "task_id": "a1", "prompt": MARKER, "spawn_depth": 2,
             "parent_task_id": TOKEN, "tool_use_id": "toolu_1", "description": MARKER},
            {"type": "system", "subtype": "background_tasks_changed",
             "tasks": [{"task_id": "a1", "description": MARKER}, {"task_id": TOKEN}]},
            {"type": "system", "subtype": "background_tasks_changed", "tasks": []},
            {"type": "result", "subtype": "success", "result": MARKER + TOKEN, "total_cost_usd": 1.0},
        ]
        with open(stream, "w", encoding="utf-8") as f:
            f.write("\n".join(json.dumps(r) for r in rows) + "\nnot json " + MARKER + "\n")
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            assert main(["extract-skeleton", stream]) == 0
        out = buf.getvalue()
        outs.append(out)
        sk = [json.loads(x) for x in out.splitlines()]
        assert all(set(x) <= set(SKELETON_KEYS) for x in sk), sk
        assert not any(w in out for w in (MARKER, "sk-ant", "oat01", "/home")), out
        agent = [b for x in sk for b in x.get("blocks", []) if b.get("name") == "Agent"]
        assert agent and agent[0]["run_in_background"] is True, sk
        assert any(x.get("subtype") == "background_tasks_changed" and x.get("tasks") == [] for x in sk), sk
        assert [x for x in sk if x.get("subtype") == "background_tasks_changed"][0]["tasks"] == ["a1"], sk
        assert any(b.get("text_len") == len(MARKER + " " + TOKEN) for x in sk for b in x.get("blocks", [])), sk
        sk_path = os.path.join(td, "sk.jsonl")
        with open(sk_path, "w", encoding="utf-8") as f:
            f.write(out)
        assert _skeleton_result_ok(sk_path) is True
        # CLI under -I
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "replay", "--results", base],
                           capture_output=True, text=True)
        assert r.returncode == 0 and r.stdout.splitlines()[0] == "EXIT_CLASS stalled", r
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "replay", "--results", base,
                            "--stream", sk_path], capture_output=True, text=True)
        assert r.returncode == 0 and r.stdout.splitlines()[0] == "EXIT_CLASS stalled", r
    assert not any(MARKER in o for o in outs)
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
