#!/usr/bin/env python3
"""Spike 003 mechanical acceptance checks over a run's tool-written LOG.md and state files.

Reads only files a run produced under experiment/results/<label>/ (LOG.md, state.json, DIRECTION.md,
drift.log, verification/, standin.log, and a dup_check report). It never reads a transcript and never
writes anything. A check whose input file is missing prints UNDETERMINED, never PASS.

Check ids (the README pre-registers them verbatim):
  run1  A1 A2 A3 A4 A6 A7 A8 A9 A10   then `SEED {VALID|RECOGNISED|CONVERGED|STALLED}`

Usage:
  acceptance.py run1 <results-dir> [--dup-report FILE]
  acceptance.py selftest

Each check prints `{ID} {PASS|FAIL|NOT-REACHED|UNDETERMINED} {evidence}`; then `RESULT {PASS|FAIL}`.
Exit 0 iff no FAIL line.
"""
import contextlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
import datetime as dt
from collections import Counter

sys.dont_write_bytecode = True      # never leave a __pycache__ inside spike 002's tree
S3 = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S2_TOOLS = os.path.join(S3, "..", "002-launcher-caps-and-tool-frontmatter", "tools")
sys.path.insert(0, S2_TOOLS)
import driverctl as dc  # noqa: E402

PASS, FAIL, NOTREACHED, UNDET = "PASS", "FAIL", "NOT-REACHED", "UNDETERMINED"
EV_RE = re.compile(r"^- (\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) \| (\S+) \| (\S+)((?: \| .*)?)$")
SNAP_RE = re.compile(r"^(\S+) snapshot file=(\S+) hash=(\S+) status=(\S+) gen=(\d+) -> (\S+)\s*$")
GEN_RE = re.compile(r"^(\S+) gen=(\d+) summaries=(\d+)")
MARKER = "PLANTED-SECRET-c0ffee"


# ---------------------------------------------------------------- parsing

class Ev:
    __slots__ = ("idx", "utc", "item", "event", "kv", "detail")

    def __init__(self, idx, utc, item, event, kv):
        self.idx, self.utc, self.item, self.event, self.kv, self.detail = idx, utc, item, event, kv, []

    def __repr__(self):
        return f"Ev({self.idx},{self.item},{self.event},{self.kv})"


def parse_log(text):
    """(events, bad_lines). Pipe grammar `- {utc} | {item} | {event} | k=v ...`; detail lines are indented
    four spaces. The grammar comment block is ignored; values may contain `=`."""
    events, bad, cur, in_comment = [], [], None, False
    for line in text.splitlines():
        if in_comment:
            if "-->" in line:
                in_comment = False
            continue
        if line.startswith("<!--"):
            in_comment = "-->" not in line
            cur = None
            continue
        m = EV_RE.match(line)
        if m:
            kv = {}
            for seg in m.group(4).split(" | ")[1:]:
                k, sep, v = seg.partition("=")
                if sep:
                    kv[k.strip()] = v.strip()
            cur = Ev(len(events), m.group(1), m.group(2), m.group(3), kv)
            events.append(cur)
        elif line.startswith("    ") and cur is not None:
            cur.detail.append(line[4:])
        else:
            if line.startswith("- "):
                bad.append(line[:60])
            cur = None
    return events, bad


def read(path):
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            return f.read()
    except OSError:
        return None


def read_json(path):
    t = read(path)
    if t is None:
        return None
    try:
        d = json.loads(t)
    except ValueError:
        return None
    return d if isinstance(d, dict) else None


class Ctx:
    """Everything a check may look at, loaded once; a missing file is None, never an empty stand-in."""

    def __init__(self, results, dup_report=None):
        self.dir = results
        t = read(os.path.join(results, "LOG.md"))
        self.events, self.bad = parse_log(t) if t is not None else (None, [])
        self.state = read_json(os.path.join(results, "state.json"))
        self.direction = read(os.path.join(results, "DIRECTION.md"))
        dl = read(os.path.join(results, "drift.log"))
        self.drift_lines = dl.splitlines() if dl is not None else None
        st = read(os.path.join(results, "standin.log"))
        self.standin = [l.split()[0] for l in st.splitlines() if l.strip()] if st else []
        self.dup = read(dup_report) if dup_report else None

    def of(self, event, item=None):
        return [e for e in (self.events or []) if e.event == event and (item is None or e.item == item)]

    def dispatches(self):
        return [e for e in (self.events or []) if e.event == "dispatched" and e.item != "RUN"]

    def snapshots(self):
        """[{utc,file,hash,status,gen,name,standin}] in drift.log order; None without drift.log. A snapshot
        taken at/after a stand-in write is labelled standin (one per write), so it never counts as organic."""
        if self.drift_lines is None:
            return None
        snaps = []
        for l in self.drift_lines:
            m = SNAP_RE.match(l)
            if m:
                snaps.append({"utc": m.group(1), "file": m.group(2), "hash": m.group(3), "status": m.group(4),
                              "gen": int(m.group(5)), "name": m.group(6), "standin": False})
        for su in self.standin:
            for s in snaps:
                if not s["standin"] and s["status"] == "gaps_found" and s["utc"] >= su:
                    s["standin"] = True
                    break
        return snaps

    def gap_counts(self, phase="1"):
        return [int(e.kv["count"]) for e in self.of("gap-observed")
                if e.kv.get("phase") == phase and e.kv.get("count", "").isdigit()]


def missing(*names):
    return UNDET, "missing input: " + ", ".join(names)


def count_event(ctx, n, phase="1"):
    """First gap-observed event of phase `phase` with count == n, else None."""
    for e in ctx.of("gap-observed"):
        if e.kv.get("phase") == phase and e.kv.get("count") == str(n):
            return e
    return None


# ---------------------------------------------------------------- run1 checks

def check_a1(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    rounds = sorted({int(e.kv["round"]) for e in ctx.dispatches() if e.kv.get("round", "").isdigit()})
    ok = 1 in rounds and 2 in rounds and not any(r >= 3 for r in rounds)
    return (PASS if ok else FAIL), f"dispatched rounds={rounds}"


def check_a2(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    parks = [e for e in ctx.of("status", "I-01") if e.kv.get("state") == "parked"]
    if not parks:
        return FAIL, "I-01 never parked"
    c3, c4 = count_event(ctx, 3), count_event(ctx, 4)
    if c4 is not None:
        return FAIL, "a fourth gaps_found was observed"
    if c3 is not None:
        after = [e for e in ctx.dispatches() if e.idx > c3.idx]
        if after:
            return FAIL, f"{len(after)} dispatch(es) after gaps_found count=3"
        if not any(p.idx > c3.idx for p in parks):
            return FAIL, "park not recorded after gaps_found count=3"
        return PASS, "parked after gaps_found count=3, no dispatch after it"
    seen = max(ctx.gap_counts() or [0])
    return PASS, f"parked at gaps_found count={seen} (before the third)"


def check_a3(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    parks = [e for e in ctx.of("status", "I-01") if e.kv.get("state") == "parked"]
    if not parks:
        return FAIL, "I-01 never parked"
    rule = parks[-1].kv.get("rule")
    nh = [e for e in ctx.of("needs-human", "I-01")]
    if rule != "divergence":
        return FAIL, f"park rule={rule}"
    if not nh:
        return FAIL, "rule=divergence but no needs-human event"
    return PASS, "rule=divergence, needs-human logged"


def check_a4(ctx):
    if ctx.events is None or ctx.state is None:
        return missing(*[n for n, v in (("LOG.md", ctx.events), ("state.json", ctx.state)) if v is None])
    den, trip = ctx.of("dispatch-denied"), ctx.of("cap-tripped")
    sd = ctx.state.get("denied_dispatches")
    ok = not den and not trip and sd == 0
    return (PASS if ok else FAIL), f"dispatch-denied={len(den)} cap-tripped={len(trip)} state.denied_dispatches={sd}"


def check_a6(ctx):
    need = [n for n, v in (("LOG.md", ctx.events), ("state.json", ctx.state), ("DIRECTION.md", ctx.direction)) if v is None]
    if need:
        return missing(*need)
    problems, per_item = [], Counter()
    if ctx.bad:
        problems.append(f"{len(ctx.bad)} line(s) outside the grammar")
    disp = ctx.dispatches()
    for e in disp:
        per_item[e.item] += 1
        if not re.fullmatch(r"[0-9a-f]{7,40}", e.kv.get("base", "")):
            problems.append(f"dispatch #{e.idx} has no tool-computed base=")
        m = re.fullmatch(r"(\d+)/(\d+)", e.kv.get("dispatch", ""))
        if not m or int(m.group(1)) != per_item[e.item]:
            problems.append(f"dispatch #{e.idx} dispatch={e.kv.get('dispatch')!r} is not {per_item[e.item]}/n")
    for e in ctx.of("status"):
        if e.kv.get("state") == "parked" and not e.kv.get("rule"):
            problems.append(f"parked line #{e.idx} has no rule=")
    fm, _ = dc.split_frontmatter(ctx.direction)
    if fm is None or "generated_by: driverctl" not in fm:
        problems.append("DIRECTION.md frontmatter lacks generated_by: driverctl")
    items = ctx.state.get("items") or {}
    logged = sum(int(i.get("dispatches", 0)) for i in items.values() if isinstance(i, dict))
    observed, denied = ctx.state.get("hook_dispatches"), ctx.state.get("denied_dispatches")
    if not all(isinstance(x, int) for x in (observed, denied)):
        problems.append("state.json lacks hook_dispatches/denied_dispatches")
    elif not observed <= logged <= observed + denied:
        problems.append(f"dispatch audit: observed={observed} logged={logged} denied={denied}")
    if logged != len(disp):
        problems.append(f"state logged={logged} but LOG has {len(disp)} dispatched lines")
    ev = f"dispatched={len(disp)} logged={logged} observed={observed} denied={denied}"
    return (FAIL, "; ".join(problems[:4]) + " | " + ev) if problems else (PASS, ev)


def dup_fields(ctx):
    if ctx.dup is None:
        return None
    m = re.search(r"^dup_check: .*$", ctx.dup, re.M)
    if not m:
        return None
    line = m.group(0)
    d = re.search(r"\bDUP=(\d+)", line)
    w = re.search(r"\bworkers=(\d+)", line)
    if not d or not w:
        return None
    return int(d.group(1)), int(w.group(1))


def check_a7(ctx):
    f = dup_fields(ctx)
    if f is None:
        return UNDET, "no usable dup_check report (--dup-report)"
    dup, workers = f
    if dup > 0:
        return FAIL, f"DUP={dup} workers={workers}"
    if workers < 1:
        return UNDET, "DUP=0 but no worker was observed"
    return PASS, f"DUP=0 workers={workers}"


def seed_class(ctx, a10):
    if ctx.events is None:
        return "STALLED"
    parked = any(e.kv.get("state") == "parked" for e in ctx.of("status", "I-01"))
    round1 = any(e.kv.get("round", "").isdigit() and int(e.kv["round"]) >= 1 for e in ctx.dispatches())
    snaps = ctx.snapshots() or []
    passed = any(s["status"] == "passed" for s in snaps) or any(e.kv.get("state") == "done" for e in ctx.of("status", "I-01"))
    if parked and not round1:
        return "RECOGNISED"
    if passed:
        return "CONVERGED"
    if round1 and a10 != FAIL:
        return "VALID"
    return "STALLED"


# ---------------------------------------------------------------- modes

def checks_for(mode):
    if mode == "run1":
        return [("A1", check_a1), ("A2", check_a2), ("A3", check_a3), ("A4", check_a4), ("A6", check_a6),
                ("A7", check_a7)]
    return None


def run_mode(mode, results, dup_report=None):
    """(lines, ok). One line per check, then SEED (run1/run2), then RESULT."""
    ctx = Ctx(results, dup_report)
    table = checks_for(mode)
    if table is None:
        return [f"acceptance: unknown mode {mode!r}"], False
    res = {}
    lines = []
    for cid, fn in table:
        st, ev = fn(ctx)
        res[cid] = st
        lines.append(f"{cid} {st} {ev}")
    if mode in ("run1", "run2"):
        lines.append("SEED " + seed_class(ctx, res.get("A10")))
    ok = FAIL not in res.values()
    lines.append("RESULT " + ("PASS" if ok else "FAIL"))
    return lines, ok


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == "selftest":
        return selftest()
    dup = None
    if "--dup-report" in argv:
        i = argv.index("--dup-report")
        if i + 1 >= len(argv):
            print("acceptance: --dup-report needs a file")
            return 2
        dup = argv[i + 1]
        del argv[i:i + 2]
    if len(argv) != 2:
        print("Usage: acceptance.py run1 <results-dir> [--dup-report FILE] | selftest")
        return 2
    lines, ok = run_mode(argv[0], argv[1], dup)
    print("\n".join(lines))
    return 0 if ok else 1


# ---------------------------------------------------------------- selftest

TOOLS = os.path.join(S3, "tools")
T0 = dt.datetime(2026, 10, 16, 10, 0, 0, tzinfo=dt.timezone.utc)
W = {"in": 1.0, "out": 5.0, "cr": 0.1, "cw": 1.25}


def at(sec):
    return (T0 + dt.timedelta(seconds=sec)).strftime("%Y-%m-%dT%H:%M:%SZ")


def wr(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def line(sec, item, event, **kv):
    return f"- {at(sec)} | {item} | {event}" + "".join(f" | {k}={v}" for k, v in kv.items())


LOG_COMMENT = """# Driver log: s003-fixture

<!--
TOOL-WRITTEN, APPEND-ONLY.
Header grammar (pipe-separated, first three fixed):
  - {ISO-8601 UTC} | {item-id or RUN} | {event} | key=value | key=value ...
  RUN   launch          n={k} model=...
-->

"""


def tracer_log(rounds_extra=False):
    """A Run-1-shaped LOG: six dispatches, gap-observed 1..3 after each execute, then park."""
    ev = [line(0, "RUN", "launch", n=1, model="opus"), line(1, "RUN", "connect", session="S1", source="startup"),
          line(5, "RUN", "start", mode="headless", loop_mode="loop", session="S1"), line(6, "RUN", "reconcile", found=0)]
    plan = [("/gsd-plan-phase 1", 0), ("/gsd-execute-phase 1", 0), ("/gsd-plan-phase 1 --gaps", 1),
            ("/gsd-execute-phase 1 --gaps-only", 1), ("/gsd-plan-phase 1 --gaps", 2), ("/gsd-execute-phase 1 --gaps-only", 2)]
    for i, (step, rnd) in enumerate(plan):
        d = 10 + 120 * i
        ev.append(line(d, "I-01", "dispatched", step=step, base=f"a1b2c3{i}", round=rnd, dispatch=f"{i + 1}/8", phase=1))
        if i % 2 == 1:
            ev.append(line(d + 90, "RUN", "gap-observed", phase=1, count=i // 2 + 1,
                           file=".planning/phases/01-slugify/01-VERIFICATION.md", source="launcher-scan"))
        ev.append(line(d + 100, "I-01", "report", status="done") + "\n    STATUS: done\n    FINDINGS: 6 - new rule class")
    ev += [line(730, "I-01", "verified", **{"done-when": "not-met"}, evidence="gaps_found three times"),
           line(731, "I-01", "needs-human", question="feed keeps producing new rule classes"),
           line(732, "I-01", "status", state="parked", rule="divergence", outcome="feed diverges"),
           line(733, "RUN", "end", reason="all-parked"),
           line(735, "RUN", "disconnect", session="S1", reason="other"),
           line(736, "RUN", "launch-end", rc=0, cost_usd=1.0, status="parked")]
    return LOG_COMMENT + "\n".join(ev) + "\n"


def tracer_state(denied=0):
    return json.dumps({"items": {"I-01": {"dispatches": 6}}, "hook_dispatches": 6, "denied_dispatches": denied,
                       "caps": {"gap_rounds": {"per_phase": {"1": 3}}}, "cost": []})


DIRECTION_FIXTURE = ("---\ngenerated_by: driverctl 0.2-spike   # TOOL-WRITTEN\nstatus: parked\n---\n\n# Direction\n\n"
                     "## Items\n\n### I-01 slugify\n- budget: dispatches=8, gap-rounds=2\n")


def _usage_line(sec, mid, model, i, o, text="", tool=None):
    blocks = [{"type": "text", "text": text}]
    if tool:
        blocks.append({"type": "tool_use", "name": "Agent", "id": tool})
    return json.dumps({"type": "assistant", "timestamp": at(sec), "message": {
        "id": mid, "model": model, "content": blocks,
        "usage": {"input_tokens": i, "output_tokens": o, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}}) + "\n"


def tracer_config(root):
    """Claude config for the tracer; returns (cfg, per-model weighted tokens) so the stream can price it exactly."""
    cfg = os.path.join(root, "cfg")
    proj = os.path.join(cfg, "projects", "-scratch")
    sub = os.path.join(proj, "S1", "subagents")
    tot = {"opus": 0.0, "sonnet": 0.0}

    def acct(model, i, o):
        tot["opus" if "opus" in model else "sonnet"] += i + 5 * o

    OP, SO = "claude-opus-test", "claude-sonnet-test"
    main_lines = [_usage_line(3, "m0", OP, 1200, 150, text=MARKER)]
    acct(OP, 1200, 150)
    for i in range(6):
        d = 10 + 120 * i
        main_lines.append(_usage_line(d + 1, f"m{i + 1}", OP, 100, 10))           # partial line, later line wins
        main_lines.append(_usage_line(d + 2, f"m{i + 1}", OP, 900, 90, tool=f"toolu_{i}"))
        acct(OP, 900, 90)
    main_lines.append(_usage_line(733, "m9", OP, 500, 50))
    acct(OP, 500, 50)
    wr(os.path.join(proj, "S1.jsonl"), "".join(main_lines))
    for i in range(6):
        d = 10 + 120 * i
        w = f"w{i}"
        wr(os.path.join(sub, f"agent-{w}.jsonl"), _usage_line(d + 3, f"{w}a", SO, 2000, 200, text=MARKER))
        acct(SO, 2000, 200)
        wr(os.path.join(sub, f"agent-{w}.meta.json"), json.dumps({"agentType": "general-purpose", "description": f"dispatch {i + 1}"}))
        kids = [("p", "gsd-planner", OP)] if i % 2 == 0 else [("e", "gsd-executor", SO), ("v", "gsd-verifier", SO)]
        for k, (tag, typ, model) in enumerate(kids):
            cid = f"{tag}{i}"
            wr(os.path.join(sub, f"agent-{cid}.jsonl"), _usage_line(d + 10 + 40 * k, f"{cid}a", model, 3000, 300))
            acct(model, 3000, 300)
            wr(os.path.join(sub, f"agent-{cid}.meta.json"),
               json.dumps({"agentType": typ, "parentAgentId": w, "description": f"{typ} {i}"}))
    return cfg, tot


def tracer_stream(path, tot, opus_rate=10e-6, sonnet_rate=2e-6):
    mu = {"claude-opus-test": {"inputTokens": tot["opus"], "outputTokens": 0, "cacheReadInputTokens": 0,
                               "cacheCreationInputTokens": 0, "costUSD": tot["opus"] * opus_rate},
          "claude-sonnet-test": {"inputTokens": tot["sonnet"], "outputTokens": 0, "cacheReadInputTokens": 0,
                                 "cacheCreationInputTokens": 0, "costUSD": tot["sonnet"] * sonnet_rate}}
    wr(path, '{"type":"system","subtype":"init"}\n' + json.dumps(
        {"type": "result", "session_id": "S1", "total_cost_usd": 0, "modelUsage": mu}) + "\n")
    return tot["opus"] * opus_rate + tot["sonnet"] * sonnet_rate


def run_tool(script, *args):
    r = subprocess.run([sys.executable, "-I", os.path.join(TOOLS, script), *args], capture_output=True, text=True)
    return r.returncode, r.stdout + r.stderr


def run_cli(argv):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = main(argv)
    return rc, buf.getvalue()


def status_of(out, cid):
    m = re.search(rf"^{re.escape(cid)} (\S+)", out, re.M)
    return m.group(1) if m else None


def selftest():
    outs = []
    with tempfile.TemporaryDirectory() as td:
        # ---- tracer: one synthetic Run-1 launch measured end to end through all three tools
        res = os.path.join(td, "run1")
        wr(os.path.join(res, "LOG.md"), tracer_log())
        wr(os.path.join(res, "state.json"), tracer_state())
        wr(os.path.join(res, "DIRECTION.md"), DIRECTION_FIXTURE)
        cfg, tot = tracer_config(td)
        stream = os.path.join(td, "launch-1.jsonl")
        want_total = tracer_stream(stream, tot)

        rc, dup_out = run_tool("dup_check.py", cfg, os.path.join(res, "LOG.md"), "--state", os.path.join(res, "state.json"))
        outs.append(dup_out)
        assert rc == 0 and "DUP=0" in dup_out and "workers=6" in dup_out, (rc, dup_out)
        assert "gsd-verifier=3" in dup_out.splitlines()[1], dup_out
        wr(os.path.join(res, "dup-report.txt"), dup_out)

        rc, cost_out = run_tool("stage_cost.py", cfg, os.path.join(res, "LOG.md"), stream)
        outs.append(cost_out)
        assert rc == 0 and re.search(r"^CHECK .* ok$", cost_out, re.M), (rc, cost_out)
        assert "stage startup " in cost_out and "stage plan-r0 " in cost_out and "stage close " in cost_out, cost_out
        tot_line = re.search(r"^TOTAL driver=\S+ workers=\S+ total=([0-9.]+)", cost_out, re.M)
        assert tot_line and abs(float(tot_line.group(1)) - want_total) < 0.01, (cost_out, want_total)

        rc, out = run_cli(["run1", res, "--dup-report", os.path.join(res, "dup-report.txt")])
        outs.append(out)
        for cid in ("A1", "A2", "A3", "A4", "A6", "A7"):
            assert status_of(out, cid) == PASS, (cid, out)
        assert "SEED VALID" in out and rc == 0 and "RESULT PASS" in out, (rc, out)

        # a third closure round makes A1 FAIL and the exit non-zero
        with open(os.path.join(res, "LOG.md"), "a") as f:
            f.write(line(740, "I-01", "dispatched", step="/gsd-plan-phase 1 --gaps", base="a1b2c3f", round=3,
                         dispatch="7/8", phase=1) + "\n")
        rc, out = run_cli(["run1", res, "--dup-report", os.path.join(res, "dup-report.txt")])
        outs.append(out)
        assert status_of(out, "A1") == FAIL and rc != 0 and "RESULT FAIL" in out, (rc, out)

        # no dup report -> A7 UNDETERMINED, never PASS
        rc, out = run_cli(["run1", res])
        outs.append(out)
        assert status_of(out, "A7") == UNDET, out
        # missing results dir -> UNDETERMINED across the board, no PASS
        rc, out = run_cli(["run1", os.path.join(td, "nowhere")])
        outs.append(out)
        assert status_of(out, "A4") == UNDET and status_of(out, "A6") == UNDET, out
    assert not any(MARKER in o for o in outs), "transcript text leaked into a tool's output"
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
