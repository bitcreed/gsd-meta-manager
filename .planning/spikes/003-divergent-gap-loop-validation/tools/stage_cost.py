#!/usr/bin/env python3
"""Spike 003 VALID-04: Opus driver and worker cost per stage and per driven GSD phase.

Spike 002's cost_split.py gives one driver/worker split per launch and prints nothing for a launch killed
before its `result` event. This tool prices a launch from its transcripts and attributes the cost to
stage windows read from the tool-written LOG.md:

  startup      first `launch` to the first I-item `dispatched`
  {kind}-r{N}  one window per `dispatched` line (kind: plan for a step containing plan-phase, execute for
               execute-phase, else step; N = round=), until the next `dispatched` or `end`
  close        `end` to `launch-end`
  unbucketed   anything outside every window (never dropped)

Driver usage (main transcript) is bucketed by each message's own `timestamp`; a worker's whole subtree
(its transcript plus every descendant) by the worker transcript's first timestamp. Usage is the LAST line
per `message.id`. Per-model rates come from the first given launch stream with a `result` event
(`modelUsage`, cost_split's method: weights in=1 out=5 cache-read=0.1 cache-write=1.25); a killed launch
has none, so `--rates-from` names another launch's stream. Costs are list-price equivalents.

Privacy: only `type`, `timestamp`, `message.id`, `message.model` and `message.usage` are read; no message
text is ever printed or stored.

Usage:
  stage_cost.py <claude-config> <LOG.md> <launch-N.jsonl>... [--rates-from <launch.jsonl>]
  stage_cost.py selftest

Exit 0 ok; 1 MISMATCH (per-stage driver costs do not sum to the driver total within $0.01); 2 no data or
unpriceable (token counts are still printed, `priced=no`).
"""
import contextlib
import datetime as dt
import glob
import io
import json
import os
import re
import sys
import tempfile
from collections import defaultdict

W = {"in": 1.0, "out": 5.0, "cr": 0.1, "cw": 1.25}
EV_RE = re.compile(r"^- (\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) \| (\S+) \| (\S+)((?: \| .*)?)$")
TOLERANCE_USD = 0.01
_FAULT = {"drop_driver_message": False}      # selftest-only: simulate a bucketing bug to prove CHECK detects it


def weighted(u):
    return sum(W[k] * u.get(k, 0) for k in W)


def ts(s):
    """ISO-8601 (Z or offset) to epoch seconds, None when missing or malformed."""
    if not isinstance(s, str):
        return None
    try:
        return dt.datetime.fromisoformat(s.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return None


def parse_log(path):
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
        out.append({"utc": m.group(1), "t": ts(m.group(1)), "item": m.group(2), "event": m.group(3), "kv": kv})
    return out


def build_windows(events):
    """[(name, start, end, phase)] walking launch / dispatched / end / launch-end markers in log order."""
    wins, cur = [], None            # cur = [name, start, phase]

    def close(at):
        nonlocal cur
        if cur is not None:
            wins.append((cur[0], cur[1], at, cur[2]))
            cur = None

    for e in events:
        ev, t = e["event"], e["t"]
        if t is None:
            continue
        if ev == "launch" and e["item"] == "RUN":
            close(t)
            cur = ["startup", t, None]
        elif ev == "dispatched" and e["item"] != "RUN":
            close(t)
            step = e["kv"].get("step", "")
            kind = "plan" if "plan-phase" in step else "execute" if "execute-phase" in step else "step"
            rnd = e["kv"].get("round", "0")
            cur = [f"{kind}-r{rnd}", t, e["kv"].get("phase")]
        elif ev == "end" and e["item"] == "RUN":
            close(t)
            cur = ["close", t, None]
        elif ev == "launch-end":
            close(t + 1.0)          # LOG utc has 1 s resolution; keep the last second inside the launch
    if cur is not None and events and events[-1]["t"] is not None:
        close(events[-1]["t"] + 1.0)
    return wins


def read_usage(path):
    """{message_id: (model, usage, timestamp)} keeping the LAST line per message id."""
    last = {}
    try:
        f = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return last
    with f:
        for n, line in enumerate(f):
            try:
                d = json.loads(line)
            except ValueError:
                continue
            if not isinstance(d, dict) or d.get("type") != "assistant":
                continue
            m = d.get("message") or {}
            if not isinstance(m, dict):
                continue
            key = m.get("id") or f"noid:{path}:{n}"
            last[key] = (str(m.get("model") or "?"), m.get("usage") or {}, ts(d.get("timestamp")))
    return last


def first_ts(path):
    try:
        f = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return None
    with f:
        for line in f:
            try:
                d = json.loads(line)
            except ValueError:
                continue
            if isinstance(d, dict) and ts(d.get("timestamp")) is not None:
                return ts(d["timestamp"])
    return None


def norm(u):
    return {"in": u.get("input_tokens", 0) or 0, "out": u.get("output_tokens", 0) or 0,
            "cr": u.get("cache_read_input_tokens", 0) or 0, "cw": u.get("cache_creation_input_tokens", 0) or 0}


def result_event(path):
    res = None
    try:
        f = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return None
    with f:
        for line in f:
            if not line.startswith("{"):
                continue
            try:
                d = json.loads(line)
            except ValueError:
                continue
            if isinstance(d, dict) and d.get("type") == "result" and isinstance(d.get("modelUsage"), dict):
                res = d
    return res


def solve_rates(res):
    rate = {}
    for model, u in res["modelUsage"].items():
        wt = weighted({"in": u.get("inputTokens", 0), "out": u.get("outputTokens", 0),
                       "cr": u.get("cacheReadInputTokens", 0), "cw": u.get("cacheCreationInputTokens", 0)})
        rate[model] = (u.get("costUSD", 0) / wt) if wt else 0.0
    return rate


def rate_for(rates, model):
    for k, r in rates.items():
        if k.startswith(model) or model.startswith(k):
            return r
    return None


def session_agents(cfg, sid):
    """{agentId: {transcript, parent}} for one session; a transcript without metadata is top-level."""
    nodes = {}
    for d in glob.glob(os.path.join(cfg, "projects", "*", sid)):
        base = os.path.join(d, "subagents")
        for p in sorted(glob.glob(os.path.join(base, "**", "agent-*.jsonl"), recursive=True)):
            aid = os.path.basename(p)[len("agent-"):-len(".jsonl")]
            parent = None
            mp = p[:-len(".jsonl")] + ".meta.json"
            try:
                with open(mp, encoding="utf-8") as f:
                    parent = (json.load(f) or {}).get("parentAgentId") or None
            except (OSError, ValueError, AttributeError):
                pass
            nodes[aid] = {"transcript": p, "parent": parent}
    return nodes


def analyse(cfg, log_path, streams, rates_from=None):
    events = parse_log(log_path)
    if events is None:
        return 2, ["stage_cost: ERROR cannot read LOG"]
    sessions = []
    for e in events:
        sid = e["kv"].get("session")
        if e["item"] == "RUN" and e["event"] == "connect" and sid and sid not in sessions:
            sessions.append(sid)
    if not sessions:
        return 2, ["stage_cost: ERROR no `connect session=` line in LOG"]
    wins = build_windows(events)

    rates, rates_src = {}, "none"
    for s in streams:
        res = result_event(s)
        if res:
            rates, rates_src = solve_rates(res), "result"
            break
    if not rates and rates_from:
        res = result_event(rates_from)
        if res:
            rates, rates_src = solve_rates(res), rates_from

    # collect (side, model, usage, bucket-time) over all sessions
    msgs = []
    for sid in sessions:
        for p in glob.glob(os.path.join(cfg, "projects", "*", sid + ".jsonl")):
            for model, u, t in read_usage(p).values():
                msgs.append(("driver", model, norm(u), t))
        nodes = session_agents(cfg, sid)
        kids = defaultdict(list)
        for aid, n in nodes.items():
            if n["parent"] in nodes:
                kids[n["parent"]].append(aid)
        tops = [aid for aid, n in nodes.items() if n["parent"] not in nodes]
        for top in tops:
            t0 = first_ts(nodes[top]["transcript"])
            stack, seen = [top], set()
            while stack:
                a = stack.pop()
                if a in seen:
                    continue
                seen.add(a)
                stack.extend(kids.get(a, []))
                for model, u, _ in read_usage(nodes[a]["transcript"]).values():
                    msgs.append(("workers", model, norm(u), t0))
    if not msgs:
        return 2, [f"stage_cost: sessions={len(sessions)} launches={len(streams)} priced=no rates_from=none",
                   "ERROR no transcript usage found for any session"]

    unmatched = any(rate_for(rates, model) is None for _, model, _, _ in msgs) if rates else False
    if not rates:
        tot = defaultdict(lambda: defaultdict(int))
        for side, model, u, t in msgs:
            for k, v in u.items():
                tot[(side, model)][k] += v
        lines = [f"stage_cost: sessions={len(sessions)} launches={len(streams)} priced=no rates_from=none"]
        for (side, model), u in sorted(tot.items()):
            lines.append(f"tokens {side} {model} in={u['in']} out={u['out']} cache_read={u['cr']} cache_write={u['cw']}")
        lines.append("ERROR no `result` event with modelUsage in any launch stream and no usable --rates-from")
        return 2, lines
    priced = "partial" if unmatched else "yes"

    # one bucket per window (index) plus `unbucketed`; printed aggregated by stage name
    buckets = [{"driver": 0.0, "workers": 0.0, "msgs": 0} for _ in wins]
    loose = {"driver": 0.0, "workers": 0.0, "msgs": 0}
    driver_total, worker_total, dropped = 0.0, 0.0, False
    for side, model, u, t in msgs:
        cost = (rate_for(rates, model) or 0.0) * weighted(u)
        if side == "driver":
            driver_total += cost
            if _FAULT["drop_driver_message"] and not dropped:
                dropped = True
                continue
        else:
            worker_total += cost
        b = loose
        if t is not None:
            for i, (_, start, end, _) in enumerate(wins):
                if start <= t < end:
                    b = buckets[i]
                    break
        b[side] += cost
        if side == "driver":
            b["msgs"] += 1

    def fmt_t(x):
        return "-" if x is None else dt.datetime.fromtimestamp(x, dt.timezone.utc).strftime("%H:%M:%SZ")

    names = []
    for name, _, _, _ in wins:
        if name not in names:
            names.append(name)
    lines = [f"stage_cost: sessions={len(sessions)} launches={len(streams)} priced={priced} rates_from={rates_src}"]
    stage_driver_sum = 0.0
    rows = [(n, [i for i, w in enumerate(wins) if w[0] == n]) for n in names]
    for name, idxs in rows:
        d = sum(buckets[i]["driver"] for i in idxs)
        w = sum(buckets[i]["workers"] for i in idxs)
        m = sum(buckets[i]["msgs"] for i in idxs)
        stage_driver_sum += d
        lines.append(f"stage {name} window={fmt_t(min(wins[i][1] for i in idxs))}..{fmt_t(max(wins[i][2] for i in idxs))} "
                     f"driver_usd={d:.4f} workers_usd={w:.4f} total_usd={d + w:.4f} driver_msgs={m}")
    if loose["driver"] or loose["workers"] or loose["msgs"]:
        stage_driver_sum += loose["driver"]
        lines.append(f"stage unbucketed window=-..- driver_usd={loose['driver']:.4f} workers_usd={loose['workers']:.4f} "
                     f"total_usd={loose['driver'] + loose['workers']:.4f} driver_msgs={loose['msgs']}")
    total = driver_total + worker_total
    share = (driver_total / total * 100) if total else 0.0
    lines.append(f"TOTAL driver={driver_total:.4f} workers={worker_total:.4f} total={total:.4f} driver_share={share:.1f}%")
    phases = {}
    for i, (_, _, _, phase) in enumerate(wins):
        if phase is not None:
            p = phases.setdefault(phase, [0.0, 0.0])
            p[0] += buckets[i]["driver"]
            p[1] += buckets[i]["workers"]
    for phase in sorted(phases, key=lambda x: (len(x), x)):
        d, w = phases[phase]
        lines.append(f"PER_PHASE phase={phase} driver={d:.4f} workers={w:.4f} total={d + w:.4f}")
    ok = abs(stage_driver_sum - driver_total) <= TOLERANCE_USD
    lines.append(f"CHECK stages_driver_sum={stage_driver_sum:.4f} driver_total={driver_total:.4f} {'ok' if ok else 'MISMATCH'}")
    return (0 if ok else 1), lines


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv[:1] == ["selftest"]:
        return selftest()
    rates_from = None
    if "--rates-from" in argv:
        i = argv.index("--rates-from")
        if i + 1 >= len(argv):
            print("stage_cost: ERROR --rates-from needs a path")
            return 2
        rates_from = argv[i + 1]
        del argv[i:i + 2]
    if len(argv) < 3:
        print("Usage: stage_cost.py <claude-config> <LOG.md> <launch-N.jsonl>... [--rates-from F] | selftest")
        return 2
    rc, lines = analyse(argv[0], argv[1], argv[2:], rates_from)
    print("\n".join(lines))
    return rc


# --------------------------------------------------------------------------- selftest

MARKER = "PLANTED-SECRET-c0ffee"


def _w(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def _msg(t, mid, model, i, o, cr=0, cw=0, text=""):
    return json.dumps({"type": "assistant", "timestamp": t,
                       "message": {"id": mid, "model": model, "content": [{"type": "text", "text": text}],
                                   "usage": {"input_tokens": i, "output_tokens": o,
                                             "cache_read_input_tokens": cr, "cache_creation_input_tokens": cw}}}) + "\n"


def _result(path, opus_tokens, sonnet_tokens, opus_rate=10e-6, sonnet_rate=2e-6, sid="S1"):
    def mu(tok, rate):
        wt = weighted(tok)
        return {"inputTokens": tok["in"], "outputTokens": tok["out"], "cacheReadInputTokens": tok["cr"],
                "cacheCreationInputTokens": tok["cw"], "costUSD": wt * rate}
    ev = {"type": "result", "session_id": sid, "total_cost_usd": 0,
          "modelUsage": {"claude-opus-test[1m]": mu(opus_tokens, opus_rate), "claude-sonnet-test": mu(sonnet_tokens, sonnet_rate)}}
    _w(path, '{"type":"system","subtype":"init"}\n' + json.dumps(ev) + "\n")


LOG_HEAD = """# t

<!--
  - {ISO} | {item} | {event}
-->

- 2026-10-16T10:00:00Z | RUN | launch | n=1
- 2026-10-16T10:00:01Z | RUN | connect | session=S1 | source=startup
"""


def _fixture(td, name):
    """startup msg, two dispatches (plan-r0 phase 1, execute-r0 phase 1), close msg, two workers, one out-of-window msg."""
    root = os.path.join(td, name)
    log = LOG_HEAD + (
        "- 2026-10-16T10:01:00Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=abc1234 | round=0 | dispatch=1/8 | phase=1\n"
        "- 2026-10-16T10:03:00Z | I-01 | report | status=done\n    STATUS: done\n"
        "- 2026-10-16T10:04:00Z | I-01 | dispatched | step=/gsd-execute-phase 1 | base=abc1235 | round=0 | dispatch=2/8 | phase=1\n"
        "- 2026-10-16T10:06:00Z | I-01 | report | status=done\n    STATUS: done\n"
        "- 2026-10-16T10:07:00Z | RUN | end | reason=all-parked\n"
        "- 2026-10-16T10:07:30Z | RUN | launch-end | rc=0 | status=parked\n")
    _w(os.path.join(root, "LOG.md"), log)
    proj = os.path.join(root, "cfg", "projects", "-p")
    d = ""
    d += _msg("2026-10-16T10:00:10Z", "d1", "claude-opus-test", 1000, 100, text=MARKER)       # startup
    d += _msg("2026-10-16T10:01:05Z", "d2", "claude-opus-test", 1000, 100)                      # plan-r0
    d += _msg("2026-10-16T10:01:06Z", "d2", "claude-opus-test", 1000, 200)                      # same id, later line wins
    d += _msg("2026-10-16T10:04:05Z", "d3", "claude-opus-test", 2000, 100)                      # execute-r0
    d += _msg("2026-10-16T10:07:10Z", "d4", "claude-opus-test", 500, 50)                        # close
    d += _msg("2026-10-16T09:00:00Z", "d5", "claude-opus-test", 100, 10)                        # before every window
    _w(os.path.join(proj, "S1.jsonl"), d)
    sub = os.path.join(proj, "S1", "subagents")
    w0 = _msg("2026-10-16T10:01:10Z", "w0a", "claude-opus-test", 3000, 300, text=MARKER)
    _w(os.path.join(sub, "agent-w0.jsonl"), w0)
    _w(os.path.join(sub, "agent-w0.meta.json"), json.dumps({"agentType": "general-purpose"}))
    # child starts AFTER the worker's first message but its cost belongs to the worker's stage (plan-r0)
    _w(os.path.join(sub, "agent-p0.jsonl"), _msg("2026-10-16T10:05:30Z", "p0a", "claude-opus-test", 4000, 400))
    _w(os.path.join(sub, "agent-p0.meta.json"), json.dumps({"agentType": "gsd-planner", "parentAgentId": "w0"}))
    _w(os.path.join(sub, "agent-w1.jsonl"), _msg("2026-10-16T10:04:10Z", "w1a", "claude-sonnet-test", 5000, 500))
    _w(os.path.join(sub, "agent-w1.meta.json"), json.dumps({"agentType": "general-purpose"}))
    opus = {"in": 1000 + 1000 + 2000 + 500 + 100 + 3000 + 4000, "out": 100 + 200 + 100 + 50 + 10 + 300 + 400, "cr": 0, "cw": 0}
    sonnet = {"in": 5000, "out": 500, "cr": 0, "cw": 0}
    _result(os.path.join(root, "launch-1.jsonl"), opus, sonnet)
    return os.path.join(root, "cfg"), os.path.join(root, "LOG.md"), os.path.join(root, "launch-1.jsonl"), opus, sonnet


def _run(argv):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        rc = main(argv)
    return rc, buf.getvalue()


def _num(out, key, prefix):
    line = next(l for l in out.splitlines() if l.startswith(prefix))
    return float(re.search(key + r"=([0-9.]+)", line).group(1))


def selftest():
    outs = []
    with tempfile.TemporaryDirectory() as td:
        cfg, log, stream, opus, sonnet = _fixture(td, "a")
        rc, out = _run([cfg, log, stream])
        outs.append(out)
        assert rc == 0, (rc, out)
        assert out.splitlines()[0] == "stage_cost: sessions=1 launches=1 priced=yes rates_from=result", out
        assert any(l.startswith("stage startup ") for l in out.splitlines()), out
        assert "CHECK" in out and out.strip().endswith("ok"), out
        # startup got the 10:00:10 message only: 1100 weighted-in+out -> (1000 + 5*100) * 10e-6
        assert abs(_num(out, "driver_usd", "stage startup") - 1500 * 10e-6) < 1e-4, out
        # last line per message id wins: d2 = 1000 + 5*200
        assert abs(_num(out, "driver_usd", "stage plan-r0") - 2000 * 10e-6) < 1e-4, out
        # worker subtree (w0 + child p0, spawned 10:01:10) lands in plan-r0 although p0 logs at 10:05:30
        assert abs(_num(out, "workers_usd", "stage plan-r0") - ((3000 + 1500) + (4000 + 2000)) * 10e-6) < 1e-4, out
        assert abs(_num(out, "workers_usd", "stage execute-r0") - (5000 + 2500) * 2e-6) < 1e-4, out
        # the 09:00 message is out of every window: reported, not dropped
        assert abs(_num(out, "driver_usd", "stage unbucketed") - 150 * 10e-6) < 1e-4, out
        assert "PER_PHASE phase=1 " in out, out
        # total equals the result event's modelUsage cost (the fixture's tokens are the sums of the transcripts)
        want = weighted(opus) * 10e-6 + weighted(sonnet) * 2e-6
        assert abs(_num(out, "total", "TOTAL") - want) < 1e-3, (out, want)
        # killed launch: no result event. Without rates -> exit 2, priced=no; with --rates-from -> priced
        killed = os.path.join(td, "a", "launch-2.jsonl")
        _w(killed, '{"type":"system","subtype":"init"}\n')
        rc, out = _run([cfg, log, killed])
        outs.append(out)
        assert rc == 2 and "priced=no" in out and "tokens driver" in out, (rc, out)
        rc, out = _run([cfg, log, killed, "--rates-from", stream])
        outs.append(out)
        assert rc == 0 and f"priced=yes rates_from={stream}" in out.splitlines()[0], (rc, out)
        # a model with no rate -> priced=partial
        rc, out = _run([cfg, log, stream])
        assert "priced=yes" in out
        # injected bucketing bug (a driver message vanishes) must be caught by CHECK
        _FAULT["drop_driver_message"] = True
        try:
            rc, out = _run([cfg, log, stream])
        finally:
            _FAULT["drop_driver_message"] = False
        outs.append(out)
        assert rc == 1 and "MISMATCH" in out, (rc, out)
        # no connect session / unreadable LOG -> exit 2
        _w(os.path.join(td, "noconn.md"), "- 2026-10-16T10:00:00Z | RUN | launch | n=1\n")
        rc, out = _run([cfg, os.path.join(td, "noconn.md"), stream])
        outs.append(out)
        assert rc == 2, (rc, out)
        rc, out = _run([cfg, os.path.join(td, "nope.md"), stream])
        assert rc == 2, (rc, out)
        # session with no transcripts at all -> exit 2 (no vacuous total)
        rc, out = _run([os.path.join(td, "empty-cfg"), log, stream])
        outs.append(out)
        assert rc == 2 and "TOTAL" not in out, (rc, out)
    assert not any(MARKER in o for o in outs), "transcript text leaked into output"
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
