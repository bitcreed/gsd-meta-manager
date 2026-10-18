#!/usr/bin/env python3
"""Spike 003 VALID-03: find duplicate planner / executor runs in a driven session's worker tree.

Spike 002's cost_split.py counts subagent transcripts but never reads the agent type, so it cannot show
a second planner. This tool rebuilds each session's agent tree from `subagents/agent-*.meta.json`
(`agentType`, `parentAgentId`, `description`) and flags, per top-level worker (no `parentAgentId`):

  DUP              more than one gsd-planner, more than one gsd-verifier, or more than one gsd-executor
                   sharing one description, among the worker's descendants; plus one DUP per top-level
                   worker beyond (LOG dispatched - LOG dispatch-denied), i.e. a re-spawn the driver did
                   not log (skipped when the LOG has no I-item `dispatched` line at all: a scripted
                   cap-test driver such as L7 never logs dispatches)
  EXECUTOR_MISSING a worker with a gsd-verifier but no gsd-executor (it ran the plan inline; reported,
                   not a duplicate)

SendMessage nudges and Agent/Task calls in the main transcript are reported, not counted as duplicates.

Privacy: only agent metadata and tool_use block NAMES are read. No transcript message text is ever
printed or stored.

Usage:
  dup_check.py <claude-config> <LOG.md> [--state <state.json>]
  dup_check.py selftest

Output: line 1 `dup_check: sessions=.. workers=.. DUP=.. EXECUTOR_MISSING=.. DRIVER_AGENT_CALLS=..
LOG_DISPATCHED=.. LOG_DENIED=.. SENDMESSAGE=..` (+ ` driver_check=skipped`), line 2 `types: ...`, then one
`worker ...` line each. Exit 0 data and DUP=0; 1 DUP > 0; 2 no data or bad input.
"""
import contextlib
import glob
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict

EV_RE = re.compile(r"^- (\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) \| (\S+) \| (\S+)((?: \| .*)?)$")
AGENT_TOOLS = ("Agent", "Task")


def parse_log(path):
    """[(utc, item, event, {k: v})] from the pipe grammar, or None when unreadable."""
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines = f.read().splitlines()
    except OSError:
        return None
    out = []
    for line in lines:
        m = EV_RE.match(line)
        if not m:
            continue
        kv = {}
        for seg in m.group(4).split(" | ")[1:]:
            k, sep, v = seg.partition("=")
            if sep:
                kv[k.strip()] = v.strip()
        out.append((m.group(1), m.group(2), m.group(3), kv))
    return out


def load_nodes(cfg, sid):
    """{agentId: node} from every agent-*.meta.json of one session (recursive, guarded)."""
    nodes = {}
    for d in glob.glob(os.path.join(cfg, "projects", "*", sid)):
        pat = os.path.join(d, "subagents", "**", "agent-*.meta.json")
        for p in sorted(glob.glob(pat, recursive=True)):
            base = os.path.basename(p)
            aid = base[len("agent-"):-len(".meta.json")]
            try:
                with open(p, encoding="utf-8") as f:
                    meta = json.load(f)
            except (OSError, ValueError):
                continue
            if not isinstance(meta, dict):
                continue
            nodes[aid] = {
                "agentId": aid,
                "agentType": str(meta.get("agentType") or "unknown"),
                "parentAgentId": meta.get("parentAgentId") or None,
                "description": str(meta.get("description") or ""),
                "toolUseId": meta.get("toolUseId"),
                "spawnDepth": meta.get("spawnDepth"),
                "finished": meta.get("finished"),
            }
    return nodes


def driver_calls(cfg, sid):
    """(transcript_found, agent_call_ids, sendmessage_count): tool_use block names only."""
    found, ids, send = False, set(), 0
    for p in glob.glob(os.path.join(cfg, "projects", "*", sid + ".jsonl")):
        found = True
        try:
            f = open(p, encoding="utf-8", errors="replace")
        except OSError:
            continue
        with f:
            for n, line in enumerate(f):
                try:
                    d = json.loads(line)
                except ValueError:
                    continue
                if not isinstance(d, dict) or d.get("type") != "assistant":
                    continue
                content = (d.get("message") or {}).get("content")
                if not isinstance(content, list):
                    continue
                for b in content:
                    if not isinstance(b, dict) or b.get("type") != "tool_use":
                        continue
                    name = b.get("name")
                    key = b.get("id") or f"{p}:{n}:{name}"
                    if name in AGENT_TOOLS:
                        ids.add(key)
                    elif name == "SendMessage":
                        send += 1
    return found, ids, send


def clean(s, limit=80):
    s = re.sub(r"\s+", " ", s).replace('"', "'").strip()
    return s[:limit]


def analyse(cfg, log_path, state_path=None):
    """Returns (rc, lines)."""
    events = parse_log(log_path)
    if events is None:
        return 2, [f"dup_check: ERROR cannot read LOG {os.path.basename(log_path)}"]
    sessions = []
    for _, item, ev, kv in events:
        if item == "RUN" and ev == "connect" and kv.get("session") and kv["session"] not in sessions:
            sessions.append(kv["session"])
    if not sessions:
        return 2, ["dup_check: ERROR no `connect session=` line in LOG"]
    log_dispatched = sum(1 for _, i, e, _ in events if i != "RUN" and e == "dispatched")
    log_denied = sum(1 for _, _, e, _ in events if e == "dispatch-denied")

    all_nodes, workers = {}, []
    calls_total, send_total, errors = 0, 0, []
    any_data = False
    for sid in sessions:
        nodes = load_nodes(cfg, sid)
        found, ids, send = driver_calls(cfg, sid)
        any_data = any_data or found or bool(nodes)
        calls_total += len(ids)
        send_total += send
        if ids and not nodes:
            errors.append(f"session {sid[:8]} dispatched {len(ids)} worker(s) but has no agent metadata")
        for aid, n in nodes.items():
            all_nodes[(sid, aid)] = n
            # WR-10: a node whose parent is missing from the session's node set is a top-level worker (the rule
            # stage_cost.py already uses), so an orphan subtree is neither dropped from workers= nor hidden from DUP.
            if not n["parentAgentId"] or n["parentAgentId"] not in nodes:
                workers.append((sid, aid))
    if not any_data:
        return 2, ["dup_check: ERROR no transcript and no agent metadata for any session"]
    allowed = max(0, log_dispatched - log_denied)
    if allowed >= 1 and not all_nodes and not errors:
        errors.append("LOG shows an allowed dispatch but no agent metadata exists")

    children = defaultdict(list)
    for (sid, aid), n in all_nodes.items():
        if n["parentAgentId"]:
            children[(sid, n["parentAgentId"])].append((sid, aid))

    type_total = Counter(n["agentType"] for n in all_nodes.values())
    dup, exec_missing, wlines = 0, 0, []
    for sid, aid in workers:
        desc, stack, seen = [], list(children.get((sid, aid), [])), set()
        while stack:
            k = stack.pop()
            if k in seen:
                continue
            seen.add(k)
            desc.append(all_nodes[k])
            stack.extend(children.get(k, []))
        types = Counter(n["agentType"] for n in desc)
        reasons = []
        if types["gsd-planner"] > 1:
            reasons.append(f"{types['gsd-planner']} gsd-planner")
        if types["gsd-verifier"] > 1:
            reasons.append(f"{types['gsd-verifier']} gsd-verifier")
        by_desc = Counter(n["description"] for n in desc if n["agentType"] == "gsd-executor")
        for d, c in sorted(by_desc.items()):
            if c > 1:
                reasons.append(f"{c} gsd-executor with description \"{clean(d, 40)}\"")
        missing = types["gsd-verifier"] >= 1 and types["gsd-executor"] == 0
        w = all_nodes[(sid, aid)]
        tl = ",".join(f"{t}={c}" for t, c in sorted(types.items())) or "-"
        line = f'worker {aid} desc="{clean(w["description"])}" {tl}'
        for r in reasons:
            dup += 1
            line += f" DUP: {r}"
        if missing:
            exec_missing += 1
            line += " EXECUTOR_MISSING"
        wlines.append(line)

    skipped = log_dispatched == 0
    if not skipped and len(workers) > allowed:
        extra = len(workers) - allowed
        dup += extra
        wlines.append(f"DUP: {extra} top-level worker(s) beyond logged dispatches "
                      f"({log_dispatched} dispatched - {log_denied} denied)")

    head = (f"dup_check: sessions={len(sessions)} workers={len(workers)} DUP={dup} "
            f"EXECUTOR_MISSING={exec_missing} DRIVER_AGENT_CALLS={calls_total} "
            f"LOG_DISPATCHED={log_dispatched} LOG_DENIED={log_denied} SENDMESSAGE={send_total}")
    if skipped:
        head += " driver_check=skipped"
    out = [head, "types: " + (" ".join(f"{t}={c}" for t, c in sorted(type_total.items())) or "-")]
    out += wlines
    if state_path:
        try:
            with open(state_path, encoding="utf-8") as f:
                hook = json.load(f).get("hook_dispatches")
            if hook is not None and hook != len(workers):
                out.append(f"warning: workers={len(workers)} but state.json hook_dispatches={hook}")
        except (OSError, ValueError, AttributeError):
            out.append("warning: state.json unreadable; hook_dispatches cross-check skipped")
    if errors:
        out += [f"ERROR: {e}" for e in errors]
        return 2, out
    return (1 if dup else 0), out


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["selftest"]:
        return selftest()
    state = None
    if "--state" in argv:
        i = argv.index("--state")
        if i + 1 >= len(argv):
            print("dup_check: ERROR --state needs a path")
            return 2
        state = argv[i + 1]
        del argv[i:i + 2]
    if len(argv) != 2:
        print(__doc__.split("Usage:")[1].split("Output:")[0].rstrip())
        return 2
    rc, lines = analyse(argv[0], argv[1], state)
    print("\n".join(lines))
    return rc


# --------------------------------------------------------------------------- selftest

MARKER = "PLANTED-SECRET-c0ffee"


def _w(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def _fixture(td, name, log_lines, agents, main_agent_calls=1, sid="S1"):
    """agents: [(id, type, parent, description)]. Returns (cfg, log_path)."""
    cfg = os.path.join(td, name, "cfg")
    proj = os.path.join(cfg, "projects", "-p")
    head = ["- 2026-10-16T10:00:00Z | RUN | launch | n=1", f"- 2026-10-16T10:00:01Z | RUN | connect | session={sid} | source=startup"]
    _w(os.path.join(td, name, "LOG.md"), "# t\n\n<!--\n  - {ISO} | {item} | x\n-->\n\n" + "\n".join(head + log_lines) + "\n")
    blocks = [{"type": "text", "text": MARKER}]
    blocks += [{"type": "tool_use", "name": "Agent", "id": f"toolu_{i}"} for i in range(main_agent_calls)]
    blocks.append({"type": "tool_use", "name": "SendMessage", "id": "toolu_s"})
    _w(os.path.join(proj, sid + ".jsonl"),
       json.dumps({"type": "assistant", "timestamp": "2026-10-16T10:00:02Z",
                   "message": {"id": "m1", "content": blocks}}) + "\n")
    for aid, typ, parent, desc in agents:
        meta = {"agentType": typ, "description": desc, "toolUseId": "t"}
        if parent:
            meta["parentAgentId"] = parent
        _w(os.path.join(proj, sid, "subagents", f"agent-{aid}.meta.json"), json.dumps(meta))
        _w(os.path.join(proj, sid, "subagents", f"agent-{aid}.jsonl"), "{}\n")
    return cfg, os.path.join(td, name, "LOG.md")


def _run(cfg, log, extra=()):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = main([cfg, log, *extra])
    return rc, buf.getvalue()


def selftest():
    D = "- 2026-10-16T10:00:10Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=abc1234 | round=0 | dispatch=1/8 | phase=1"
    DEN = "- 2026-10-16T10:00:20Z | RUN | dispatch-denied | reason=phase 1 has failed verification 3 times"
    outs = []
    with tempfile.TemporaryDirectory() as td:
        # clean: one logged dispatch, one worker with one planner -> exit 0
        cfg, log = _fixture(td, "clean", [D], [("w0", "general-purpose", None, "plan"), ("p0", "gsd-planner", "w0", "planner")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 0 and "DUP=0" in out and "workers=1" in out and "driver_check" not in out, (rc, out)
        assert out.splitlines()[1] == "types: general-purpose=1 gsd-planner=1", out
        assert "SENDMESSAGE=1" in out and "DRIVER_AGENT_CALLS=1" in out, out
        # duplicate planner under one worker -> exit 1
        cfg, log = _fixture(td, "dupplan", [D], [("w0", "general-purpose", None, "plan"), ("p0", "gsd-planner", "w0", "a"),
                                                   ("p1", "gsd-planner", "w0", "b")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and "DUP=1" in out and "DUP: 2 gsd-planner" in out, (rc, out)
        # second verifier; same-description executors dup, different descriptions fine
        ag = [("w0", "general-purpose", None, "exec"), ("e0", "gsd-executor", "w0", "plan 01"),
              ("e1", "gsd-executor", "w0", "plan 02"), ("v0", "gsd-verifier", "w0", "v"), ("v1", "gsd-verifier", "w0", "v")]
        cfg, log = _fixture(td, "dupver", [D], ag)
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and "DUP: 2 gsd-verifier" in out and "gsd-executor with" not in out, (rc, out)
        ag = [("w0", "general-purpose", None, "exec"), ("e0", "gsd-executor", "w0", "plan 01"),
              ("e1", "gsd-executor", "w0", "plan 01"), ("v0", "gsd-verifier", "w0", "v")]
        cfg, log = _fixture(td, "dupexec", [D], ag)
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and 'DUP: 2 gsd-executor with description "plan 01"' in out, (rc, out)
        # nested one level deeper still counts toward the top-level worker
        ag = [("w0", "general-purpose", None, "x"), ("m0", "general-purpose", "w0", "mid"),
              ("p0", "gsd-planner", "m0", "a"), ("p1", "gsd-planner", "w0", "b")]
        cfg, log = _fixture(td, "nested", [D], ag)
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and "DUP: 2 gsd-planner" in out, (rc, out)
        # WR-10: a worker whose parent has no meta.json is still a top-level worker, and duplicates under it are flagged
        ag = [("w0", "general-purpose", "ghost", "plan"), ("p0", "gsd-planner", "w0", "a"), ("p1", "gsd-planner", "w0", "b")]
        cfg, log = _fixture(td, "orphandup", [D], ag)
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and "workers=1" in out.splitlines()[0] and "DUP=1" in out.splitlines()[0], (rc, out)
        assert "DUP: 2 gsd-planner" in out and "ghost" not in out, (rc, out)
        cfg, log = _fixture(td, "orphanok", [D], [("w0", "general-purpose", "ghost", "plan"), ("p0", "gsd-planner", "w0", "a")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 0 and "workers=1" in out.splitlines()[0] and "DUP=0" in out.splitlines()[0], (rc, out)
        # verifier without executor: reported, not a duplicate
        cfg, log = _fixture(td, "inline", [D], [("w0", "general-purpose", None, "x"), ("v0", "gsd-verifier", "w0", "v")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 0 and "EXECUTOR_MISSING=1" in out and "DUP=0" in out, (rc, out)
        # dispatched but no metadata at all -> exit 2, never 0
        cfg, log = _fixture(td, "nometa", [D], [])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 2 and "DUP=0" in out and "ERROR" in out, (rc, out)
        # L7 shape: no `dispatched` line, one worker -> exit 0 with driver_check=skipped
        cfg, log = _fixture(td, "l7", [], [("w0", "general-purpose", None, "x")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 0 and "driver_check=skipped" in out.splitlines()[0], (rc, out)
        # one dispatched line, two top-level workers -> DUP=1
        cfg, log = _fixture(td, "respawn", [D], [("w0", "general-purpose", None, "x"), ("w1", "general-purpose", None, "y")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 1 and "DUP=1" in out.splitlines()[0], (rc, out)
        # a denied dispatch does not make a worker a re-spawn
        D2 = D.replace("10:00:10Z", "10:00:15Z").replace("dispatch=1/8", "dispatch=2/8")
        cfg, log = _fixture(td, "denied", [D, D2, DEN], [("w0", "general-purpose", None, "x")])
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 0 and "LOG_DENIED=1" in out, (rc, out)
        # no connect session -> exit 2
        os.makedirs(os.path.join(td, "noconn"))
        _w(os.path.join(td, "noconn", "LOG.md"), "- 2026-10-16T10:00:00Z | RUN | launch | n=1\n")
        rc, out = _run(os.path.join(td, "noconn"), os.path.join(td, "noconn", "LOG.md"))
        outs.append(out)
        assert rc == 2 and "DUP=0" not in out, (rc, out)
        # unreadable LOG, empty config -> exit 2
        rc, out = _run(td, os.path.join(td, "missing.md"))
        outs.append(out)
        assert rc == 2, (rc, out)
        cfg, log = _fixture(td, "empty", [], [])
        for p in glob.glob(os.path.join(cfg, "projects", "*", "*.jsonl")):
            os.remove(p)
        rc, out = _run(cfg, log)
        outs.append(out)
        assert rc == 2, (rc, out)
        # --state cross-check warns on mismatch
        cfg, log = _fixture(td, "state", [D], [("w0", "general-purpose", None, "x")])
        _w(os.path.join(td, "state", "state.json"), json.dumps({"hook_dispatches": 3}))
        rc, out = _run(cfg, log, ["--state", os.path.join(td, "state", "state.json")])
        outs.append(out)
        assert rc == 0 and "warning: workers=1 but state.json hook_dispatches=3" in out, (rc, out)
        # CLI under -I, same contract
        cfg, log = _fixture(td, "cli", [D], [("w0", "general-purpose", None, "x")])
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), cfg, log], capture_output=True, text=True)
        outs.append(r.stdout + r.stderr)
        assert r.returncode == 0 and r.stdout.startswith("dup_check: sessions=1 workers=1 DUP=0"), r
    assert not any(MARKER in o for o in outs), "transcript text leaked into output"
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
