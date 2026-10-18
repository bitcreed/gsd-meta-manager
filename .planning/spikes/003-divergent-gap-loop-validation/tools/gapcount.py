#!/usr/bin/env python3
"""Spike 003 (Phase 26.1) D-01: the ONE definition of a gap round.

Phase 26 counted every hash-distinct `status: gaps_found` VERIFICATION.md write as a gap round
(spike 002 `launcher.py:119-140`). One gsd-verifier run wrote the file and then patched it with `sed -i`
6.5 s later, so a single re-verification counted as two rounds and Run 1 reached count 3. This module
replaces that rule with a sliding-window coalescing rule on the file mtime, shared by the fork launcher
(live gate), the replay below and the acceptance scorer (offline), so there is no second definition.

Rule (strict, float seconds, no rounding or truncation):
  * a digest already seen is `duplicate`: it moves neither the count nor the window anchor;
  * otherwise, if the previous gaps_found write of the same phase is less than COALESCE_S earlier,
    the write is `coalesced` (the anchor slides to it, so three writes 20 s apart are one round);
  * otherwise it is `new` and the phase count goes up by one.
The state lives in the `caps.gap_rounds` dict (`seen`, `per_phase`, `last_write`, `coalesced`) so a
relaunch that reloads state.json keeps the window. spike 002's gate and trip read only `per_phase`.

Usage:
  gapcount.py selftest
  gapcount.py window
  gapcount.py replay --results DIR [--writes FILE] [--gap-limit 2]
  gapcount.py extract-writes TRANSCRIPT

Exit codes: 0 ok, 1 check failed, 2 no data or bad input.

Output of `replay`: `OLD counts=[..]` (the logged hash-distinct counts), `NEW counts=[..]` (the count of
each round the coalescing rule opens; coalesced observations add no entry), `NEW coalesced=N`,
`GATE round-2 dispatch ALLOWED|DENIED` (spike 002's unchanged deny predicate `n >= gl + 1`),
`CAP tripped=yes|no` (its unchanged trip predicate `worst > gl + 1`) and, with --writes,
`WRITES agents=A writes=W rounds=R`.

Privacy: `extract-writes` reads tool names and file basenames only; it never prints command text, file
content or message text.
"""
import contextlib
import datetime as dt
import io
import json
import os
import re
import subprocess
import sys
import tempfile

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:               # `python3 -I` drops the script directory from sys.path
    sys.path.insert(0, HERE)
from dup_check import parse_log  # noqa: E402  (one LOG parser; no fourth EV_RE copy)

# [inferred - audit] measured verifier re-write gap 6.5 s (transcript) / 8 s (LOG); shortest legitimate gap
# between two gap rounds 534 s (26.1-RESEARCH A1). 45 s sits well inside that interval on both sides.
COALESCE_S = 45

SAFE_NAME = re.compile(r"^[A-Za-z0-9._-]{1,80}$")
VERIF_PATH = re.compile(r"[^\s'\"]*-VERIFICATION\.md")
WRITE_TOOLS = ("Write", "Edit")


def epoch(utc):
    """ISO-8601 UTC ('2026-10-17T02:57:04Z' or with fractional seconds) -> float epoch seconds."""
    return dt.datetime.fromisoformat(utc.replace("Z", "+00:00")).timestamp()


class GapCounter:
    """Per phase: a gaps_found write at least `window` seconds after the previous write starts a new round."""

    def __init__(self, window=COALESCE_S, state=None):
        self.window = window
        self.state = state if state is not None else {}
        for key in ("seen", "per_phase", "last_write", "coalesced"):
            self.state.setdefault(key, {})

    def observe(self, phase, ts, digest, file=None):
        """-> ("new"|"coalesced"|"duplicate", k) where k is the phase's round count after this write."""
        st = self.state
        phase = str(phase)
        if digest in st["seen"]:
            return "duplicate", st["per_phase"].get(phase, 0)
        st["seen"][digest] = {"phase": phase, "file": file}
        prev = st["last_write"].get(phase)
        count = st["per_phase"].get(phase, 0)
        if prev is not None and count >= 1 and abs(ts - prev) < self.window:
            st["last_write"][phase] = max(prev, ts)       # sliding anchor, never moves backwards
            st["coalesced"][phase] = st["coalesced"].get(phase, 0) + 1
            return "coalesced", count
        st["last_write"][phase] = ts if prev is None else max(prev, ts)
        st["per_phase"][phase] = count + 1
        return "new", count + 1


def coalesce(times, window=COALESCE_S):
    """Group indices of `times` into bursts with the same sliding rule: [[i, ...], ...] in time order."""
    order = sorted(range(len(times)), key=lambda i: times[i])
    bursts = []
    anchor = None
    for i in order:
        if anchor is not None and times[i] - anchor < window:
            bursts[-1].append(i)
        else:
            bursts.append([i])
        anchor = times[i]
    return bursts


# --------------------------------------------------------------------------- replay

def replay(results, writes=None, gap_limit=2):
    """Returns (rc, lines)."""
    log = os.path.join(results, "LOG.md")
    events = parse_log(log)
    if events is None:
        return 2, [f"gapcount: ERROR cannot read {os.path.basename(log)} in the results dir"]
    obs = [(utc, kv) for utc, item, ev, kv in events
           if item == "RUN" and ev == "gap-observed" and kv.get("phase") == "1"]
    if not obs:
        return 2, ["gapcount: ERROR no phase-1 `gap-observed` line in LOG.md"]
    old = []
    for _, kv in obs:
        try:
            old.append(int(kv.get("count", "")))
        except ValueError:
            return 2, ["gapcount: ERROR gap-observed line without a numeric count"]
    gc = GapCounter()
    new, coalesced = [], 0
    for n, (utc, kv) in enumerate(obs):
        kind, k = gc.observe("1", epoch(utc), f"obs{n}", kv.get("file"))
        if kind == "new":
            new.append(k)
        elif kind == "coalesced":
            coalesced += 1
    top = max(new)
    lines = [f"OLD counts={old}", f"NEW counts={new}", f"NEW coalesced={coalesced}",
             "GATE round-2 dispatch " + ("ALLOWED" if top < gap_limit + 1 else "DENIED"),
             "CAP tripped=" + ("yes" if top > gap_limit + 1 else "no")]
    if writes:
        try:
            with open(writes, encoding="utf-8") as f:
                ws = json.load(f)
            times = [epoch(w["ts"]) for w in ws]
            agents = len({w["agent_id"] for w in ws})
        except (OSError, ValueError, KeyError, TypeError):
            return 2, lines + ["gapcount: ERROR cannot read the verifier-writes fixture"]
        lines.append(f"WRITES agents={agents} writes={len(ws)} rounds={len(coalesce(times))}")
    return 0, lines


# --------------------------------------------------------------------------- extract-writes

def extract_writes(transcript):
    """Sanitized write list for one agent transcript. Only allowlisted structural keys are emitted."""
    m = re.match(r"agent-([A-Za-z0-9]+)\.jsonl$", os.path.basename(transcript))
    agent_id = m.group(1) if m else None
    agent_type = None
    meta = os.path.join(os.path.dirname(transcript), f"agent-{agent_id}.meta.json") if agent_id else None
    if meta and os.path.isfile(meta):
        try:
            with open(meta, encoding="utf-8") as f:
                md = json.load(f)
            t = md.get("agentType") or md.get("subagent_type")
            agent_type = t if isinstance(t, str) and SAFE_NAME.match(t) else None
        except (OSError, ValueError, AttributeError):
            agent_type = None
    out = []
    with open(transcript, encoding="utf-8", errors="replace") as f:
        for line in f:
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
                name, inp = b.get("name"), b.get("input")
                if not isinstance(inp, dict):
                    continue
                target = None
                if name in WRITE_TOOLS:
                    fp = inp.get("file_path")
                    if isinstance(fp, str) and fp.endswith("-VERIFICATION.md"):
                        target = os.path.basename(fp)
                elif name == "Bash":
                    cmd = inp.get("command")
                    if isinstance(cmd, str) and "sed -i" in cmd:
                        mm = VERIF_PATH.search(cmd)
                        if mm:
                            target = os.path.basename(mm.group(0))
                if target is None or not SAFE_NAME.match(target):
                    continue
                ts = d.get("timestamp")
                if not isinstance(ts, str):
                    continue
                out.append({"ts": ts, "agent_id": agent_id, "agent_type": agent_type,
                            "tool": name, "target": target})
    return out


# --------------------------------------------------------------------------- CLI

def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__.split("Usage:")[1].split("Output of")[0].rstrip())
        return 2
    cmd, rest = argv[0], argv[1:]
    if cmd == "selftest":
        return selftest()
    if cmd == "window":
        print(COALESCE_S)
        return 0
    if cmd == "replay":
        opts = {"--results": None, "--writes": None, "--gap-limit": "2"}
        i = 0
        while i < len(rest):
            if rest[i] in opts and i + 1 < len(rest):
                opts[rest[i]] = rest[i + 1]
                i += 2
            else:
                print(f"gapcount: ERROR bad argument {rest[i]!r}")
                return 2
        try:
            gl = int(opts["--gap-limit"])
        except ValueError:
            print("gapcount: ERROR --gap-limit must be an integer")
            return 2
        if not opts["--results"]:
            print("gapcount: ERROR --results is required")
            return 2
        rc, lines = replay(opts["--results"], opts["--writes"], gl)
        print("\n".join(lines))
        return rc
    if cmd == "extract-writes":
        if len(rest) != 1 or not os.path.isfile(rest[0]):
            print("gapcount: ERROR extract-writes needs one transcript path", file=sys.stderr)
            return 2
        print(json.dumps(extract_writes(rest[0]), indent=1))
        return 0
    print(f"gapcount: ERROR unknown command {cmd!r}")
    return 2


# --------------------------------------------------------------------------- selftest

MARKER = "PLANTED-SECRET-c0ffee"


def _kinds(gc, phase, times):
    return [gc.observe(phase, t, f"d{phase}-{i}-{t}") for i, t in enumerate(times)]


def selftest():
    # window constant and CLI
    assert COALESCE_S == 45
    # VALID-02 boundary edge: legitimate rounds 534 s apart stay separate -> counts 1, 2, 3
    gc = GapCounter()
    t = 1_000_000.0
    got = _kinds(gc, "1", [t, t + 534, t + 1068])
    assert got == [("new", 1), ("new", 2), ("new", 3)], got
    gl = 2                                           # S2 predicates: deny when n >= gl + 1, trip when n > gl + 1
    assert [(n >= gl + 1, n > gl + 1) for _, n in got] == [(False, False), (False, False), (True, False)]
    assert (4 >= gl + 1, 4 > gl + 1) == (True, True)  # kill only at L+2
    # precision edge: 44.9 s coalesced, exactly 45.0 s new (strict <, no rounding)
    gc = GapCounter()
    assert gc.observe("1", t, "a") == ("new", 1)
    assert gc.observe("1", t + 44.9, "b") == ("coalesced", 1)
    gc = GapCounter()
    assert gc.observe("1", t, "a") == ("new", 1)
    assert gc.observe("1", t + 45.0, "b") == ("new", 2)
    gc = GapCounter()
    assert gc.observe("1", t, "a") == ("new", 1)
    assert gc.observe("1", t + 44.999999, "b")[0] == "coalesced"      # no truncation to whole seconds
    # sliding anchor: three writes 20 s apart are one round (second is 40 s after the first, third 40 s after second anchor)
    gc = GapCounter()
    got = _kinds(gc, "1", [t, t + 20, t + 40, t + 60, t + 80])
    assert [k for k, _ in got] == ["new", "coalesced", "coalesced", "coalesced", "coalesced"], got
    assert gc.state["per_phase"] == {"1": 1} and gc.state["coalesced"] == {"1": 4}, gc.state
    # a write 45 s after the slid anchor opens a new round even though it is far from the first one
    assert gc.observe("1", t + 80 + 45, "z") == ("new", 2)
    # duplicate digest: no count change and the anchor does not move
    gc = GapCounter()
    gc.observe("1", t, "same")
    anchor = gc.state["last_write"]["1"]
    assert gc.observe("1", t + 100, "same") == ("duplicate", 1)
    assert gc.state["last_write"]["1"] == anchor and gc.state["per_phase"] == {"1": 1}
    assert gc.observe("1", t + 100, "other") == ("new", 2)
    # phases are independent
    gc = GapCounter()
    assert gc.observe("1", t, "a") == ("new", 1) and gc.observe("2", t + 1, "b") == ("new", 1)
    # JSON round trip keeps the window and the counts across a relaunch
    gc = GapCounter()
    gc.observe("1", t, "a", "f.md")
    state = json.loads(json.dumps(gc.state))
    gc2 = GapCounter(state=state)
    assert gc2.observe("1", t + 10, "b") == ("coalesced", 1)
    assert gc2.observe("1", t + 10 + 600, "c") == ("new", 2)
    assert gc2.observe("1", t + 10, "b") == ("duplicate", 2)
    # S2-shaped initial state (only seen / per_phase) is accepted
    gc3 = GapCounter(state={"limit": 2, "per_phase": {}, "seen": {}})
    assert gc3.observe("1", t, "a") == ("new", 1) and "last_write" in gc3.state
    # coalesce(): bursts by index, unsorted input, boundary
    assert coalesce([0, 20, 40]) == [[0, 1, 2]]
    assert coalesce([0, 534, 1068]) == [[0], [1], [2]]
    assert coalesce([0, 45.0]) == [[0], [1]] and coalesce([0, 44.9]) == [[0, 1]]
    assert coalesce([100, 0, 20]) == [[1, 2], [0]]
    assert coalesce([]) == []

    with tempfile.TemporaryDirectory() as td:
        # extract-writes: planted marker in message text and in the Bash command must not reach the output
        tdir = os.path.join(td, "sub")
        os.makedirs(tdir)
        tp = os.path.join(tdir, "agent-abc123.jsonl")
        meta = {"agentType": "gsd-verifier", "description": MARKER}
        with open(os.path.join(tdir, "agent-abc123.meta.json"), "w") as f:
            json.dump(meta, f)

        def entry(ts, blocks):
            return json.dumps({"type": "assistant", "timestamp": ts, "message": {"content": blocks}})
        rows = [
            entry("2026-10-17T02:57:00.000Z", [{"type": "text", "text": MARKER}]),
            entry("2026-10-17T02:57:04.625Z", [{"type": "tool_use", "name": "Write", "input": {
                "file_path": "/x/.planning/phases/01-s/01-VERIFICATION.md", "content": MARKER}}]),
            entry("2026-10-17T02:57:08.000Z", [{"type": "tool_use", "name": "Write", "input": {
                "file_path": "/x/notes.md", "content": MARKER}}]),
            entry("2026-10-17T02:57:11.149Z", [{"type": "tool_use", "name": "Bash", "input": {
                "command": f"echo {MARKER}; sed -i 's/{MARKER}/b/' /x/.planning/phases/01-s/01-VERIFICATION.md"}}]),
            entry("2026-10-17T02:57:12.000Z", [{"type": "tool_use", "name": "Bash", "input": {
                "command": f"cat /x/01-VERIFICATION.md {MARKER}"}}]),
            "not json " + MARKER,
        ]
        with open(tp, "w") as f:
            f.write("\n".join(rows) + "\n")
        ws = extract_writes(tp)
        assert len(ws) == 2 and ws[0]["tool"] == "Write" and ws[1]["tool"] == "Bash", ws
        assert all(set(w) == {"ts", "agent_id", "agent_type", "tool", "target"} for w in ws), ws
        assert all(w["agent_id"] == "abc123" and w["agent_type"] == "gsd-verifier"
                   and w["target"] == "01-VERIFICATION.md" for w in ws), ws
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            assert main(["extract-writes", tp]) == 0
        assert MARKER not in buf.getvalue() and json.loads(buf.getvalue()) == ws
        wp = os.path.join(td, "writes.json")
        with open(wp, "w") as f:
            json.dump(ws, f)

        # replay on a temp results dir shaped like Phase 26 Run 1
        res = os.path.join(td, "res")
        os.makedirs(res)
        g = "- {} | RUN | gap-observed | phase=1 | count={} | file=.planning/phases/01-s/01-VERIFICATION.md | source=launcher-scan"
        with open(os.path.join(res, "LOG.md"), "w") as f:
            f.write("# t\n\n" + "\n".join([
                g.format("2026-10-17T02:48:10Z", 1), g.format("2026-10-17T02:57:04Z", 2),
                g.format("2026-10-17T02:57:12Z", 3)]) + "\n")
        rc, lines = replay(res, wp, 2)
        assert rc == 0 and lines == ["OLD counts=[1, 2, 3]", "NEW counts=[1, 2]", "NEW coalesced=1",
                                     "GATE round-2 dispatch ALLOWED", "CAP tripped=no",
                                     "WRITES agents=1 writes=2 rounds=1"], lines
        # negative control: three genuine rounds 534 s apart are NOT merged -> count 3 denies, no kill
        with open(os.path.join(res, "LOG.md"), "w") as f:
            f.write("\n".join([g.format("2026-10-17T02:00:00Z", 1), g.format("2026-10-17T02:08:54Z", 2),
                               g.format("2026-10-17T02:17:48Z", 3)]) + "\n")
        rc, lines = replay(res, None, 2)
        assert lines[1] == "NEW counts=[1, 2, 3]" and lines[3] == "GATE round-2 dispatch DENIED" \
            and lines[4] == "CAP tripped=no", lines
        # a fourth genuine round trips the cap
        with open(os.path.join(res, "LOG.md"), "a") as f:
            f.write(g.format("2026-10-17T02:26:42Z", 4) + "\n")
        rc, lines = replay(res, None, 2)
        assert lines[1] == "NEW counts=[1, 2, 3, 4]" and lines[4] == "CAP tripped=yes", lines
        # no data -> exit 2
        with open(os.path.join(res, "LOG.md"), "w") as f:
            f.write("# nothing\n")
        assert replay(res)[0] == 2
        assert replay(os.path.join(td, "missing"))[0] == 2
        # CLI under -I
        with open(os.path.join(res, "LOG.md"), "w") as f:
            f.write(g.format("2026-10-17T02:48:10Z", 1) + "\n" + g.format("2026-10-17T02:48:18Z", 2) + "\n")
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "replay", "--results", res],
                           capture_output=True, text=True)
        assert r.returncode == 0 and "NEW counts=[1]" in r.stdout and "OLD counts=[1, 2]" in r.stdout, r
        r = subprocess.run([sys.executable, "-I", os.path.abspath(__file__), "window"], capture_output=True, text=True)
        assert r.stdout == "45\n", r
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
