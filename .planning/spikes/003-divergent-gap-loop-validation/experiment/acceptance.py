#!/usr/bin/env python3
"""Spike 003 mechanical acceptance checks over a run's tool-written LOG.md and state files.

Reads only files a run produced under experiment/results/<label>/ (LOG.md, state.json, DIRECTION.md,
drift.log, verification/, standin.log, stop-guard.log, and a dup_check report). It never reads a transcript
and never writes anything. A check whose input file is missing prints UNDETERMINED, never PASS.

Modes and check ids (the README pre-registers them verbatim):
  l7    L7-1 L7-2 L7-3 L7-4 L7-5 L7-6                      scripted misbehaving driver (gap-round cap)
  l8    L8-1 L8-2 L8-3 L8-4                                scripted early-worker probe (Stop guard, SendMessage)
  run1  A1 A2 A3 A4 A6 A7 A8 A9 A10   then `SEED {VALID|RECOGNISED|CONVERGED|STALLED}`
  run2  A5-deny A5-kill A11 A6 A7 A8 A9 A10   then `SEED ...` (prompt divergence rule disabled)

Usage:
  acceptance.py l7|l8|run1|run2 <results-dir> [--dup-report FILE]
  acceptance.py rubric-check [--sha S] [--block 26.1]
  acceptance.py selftest

Each check prints `{ID} {PASS|FAIL|NOT-REACHED|UNDETERMINED} {evidence}`; then `RESULT {PASS|FAIL|INCONCLUSIVE}`.
  RESULT FAIL          exit 1   some check printed FAIL
  RESULT INCONCLUSIVE  exit 2   no FAIL, but a check printed UNDETERMINED or no check printed PASS
                                (a missing or empty results dir lands here: closes T-26-11)
  RESULT PASS          exit 0   no FAIL, no UNDETERMINED and at least one PASS (NOT-REACHED never blocks PASS)

`rubric-check` compares the README's pre-registered block with the one at the prereg sha:
  (no --block)   `<!-- rubric:start -->`..`<!-- rubric:end -->` against results/prereg.txt (the Phase 26 rubric)
  --block 26.1   `<!-- rubric-26.1:start -->`..`<!-- rubric-26.1:end -->` against results/prereg-26.1.txt;
                 fail-closed: RUBRIC-MISSING (exit 2) while the prereg file or either block is absent
  RUBRIC-UNCHANGED exit 0, RUBRIC-CHANGED exit 1, RUBRIC-MISSING exit 2.
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
sys.path.insert(0, os.path.join(S3, "tools"))      # `python3 -I` drops the script dir; gapcount is the one round definition
import driverctl as dc  # noqa: E402
import gapcount  # noqa: E402  (coalesce: the same 45 s rule the live launcher gates on)

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
        self.guard = read(os.path.join(results, "stop-guard.log"))

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


# ---------------------------------------------------------------- l7 checks (scripted misbehaving driver)

def _need(ctx, **inputs):
    gone = [n for n, v in inputs.items() if v is None]
    return missing(*gone) if gone else None


def check_l7_1(ctx):
    m = _need(ctx, **{"LOG.md": ctx.events, "drift.log": ctx.drift_lines})
    if m:
        return m
    for e in ctx.of("verified"):
        f = re.search(r"feed-lines before=(\d+) after=(\d+)", e.kv.get("evidence", ""))
        if f:
            before, after = int(f.group(1)), int(f.group(2))
            gen1 = any((g := GEN_RE.match(l)) and g.group(2) == "1" for l in ctx.drift_lines)
            ok = after > before and gen1
            return (PASS if ok else FAIL), f"feed-lines before={before} after={after} drift.log gen=1 {'seen' if gen1 else 'absent'}"
    return FAIL, "no `feed-lines before= after=` record in a verified event"


def check_l7_2(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    counts = ctx.gap_counts()
    return (PASS if counts == [1, 2, 3, 4] else FAIL), f"phase 1 gap-observed counts={counts}"


def check_l7_3(ctx):
    m = _need(ctx, **{"LOG.md": ctx.events, "state.json": ctx.state})
    if m:
        return m
    c3 = count_event(ctx, 3)
    hook = ctx.state.get("hook_dispatches")
    early = [e for e in ctx.of("dispatch-denied") if c3 is None or e.idx < c3.idx]
    ok = c3 is not None and isinstance(hook, int) and hook >= 1 and not early
    return (PASS if ok else FAIL), f"hook_dispatches={hook} denies_before_count3={len(early)} count3={'seen' if c3 else 'absent'}"


def check_l7_4(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    c3, denies = count_event(ctx, 3), ctx.of("dispatch-denied")
    if not denies or c3 is None:
        return FAIL, f"dispatch-denied={len(denies)} count3={'seen' if c3 else 'absent'}"
    d = denies[0]
    ok = d.idx > c3.idx and "failed verification 3 times" in d.kv.get("reason", "")
    return (PASS if ok else FAIL), f"first deny {'after' if d.idx > c3.idx else 'before'} count=3, reason matches={'failed verification 3 times' in d.kv.get('reason', '')}"


def _kill(ctx):
    return [e for e in ctx.of("cap-tripped") if e.kv.get("cap") == "gap_rounds" and e.kv.get("used") == "4" and e.kv.get("limit") == "2"]


def check_l7_5(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    c4, kills = count_event(ctx, 4), _kill(ctx)
    early = [e for e in ctx.of("cap-tripped") if c4 is None or e.idx < c4.idx]
    ok = bool(kills) and c4 is not None and kills[0].idx > c4.idx and not early
    return (PASS if ok else FAIL), f"cap-tripped gap_rounds used=4 limit=2: {len(kills)}; before count=4: {len(early)}"


def check_l7_6(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    ends = ctx.of("launch-end")
    ok = bool(ends) and ends[-1].kv.get("tripped") == "gap_rounds" and ends[-1].kv.get("status") == "stopped"
    return (PASS if ok else FAIL), f"launch-end {ends[-1].kv if ends else 'absent'}".replace("'", "")


# ---------------------------------------------------------------- l8 checks (scripted early-worker probe)

def dup_line_fields(ctx):
    """{name: int} of the dup report's `dup_check:` line (line 1), or None without a usable report."""
    if ctx.dup is None:
        return None
    m = re.search(r"^dup_check: .*$", ctx.dup, re.M)
    if not m:
        return None
    return {k: int(v) for k, v in re.findall(r"\b([A-Za-z_]+)=(\d+)", m.group(0))}


def check_l8_1(ctx):
    """guard fired: stop-guard.log has a `block item=I-01` line."""
    if ctx.guard is None:
        return missing("stop-guard.log")
    blocks = [l for l in ctx.guard.splitlines() if re.search(r"\bblock item=I-01\b", l)]
    return (PASS if blocks else FAIL), f"stop-guard block item=I-01 lines={len(blocks)}"


def check_l8_2(ctx):
    """resumed by SendMessage: the dup report's SENDMESSAGE= is at least 1."""
    f = dup_line_fields(ctx)
    if f is None or "SENDMESSAGE" not in f:
        return UNDET, "no usable dup_check report (--dup-report)"
    return (PASS if f["SENDMESSAGE"] >= 1 else FAIL), f"SENDMESSAGE={f['SENDMESSAGE']}"


def check_l8_3(ctx):
    """no respawn: DUP=0 and workers= equals the number of LOG dispatched lines."""
    f = dup_line_fields(ctx)
    if f is None or "DUP" not in f or "workers" not in f:
        return UNDET, "no usable dup_check report (--dup-report)"
    if ctx.events is None:
        return missing("LOG.md")
    n = len(ctx.dispatches())
    ok = f["DUP"] == 0 and f["workers"] == n
    return (PASS if ok else FAIL), f"DUP={f['DUP']} workers={f['workers']} LOG dispatched={n}"


def check_l8_4(ctx):
    """reported, not crashed: an I-01 report, a RUN end, and a last launch-end that is neither crashed nor stalled."""
    if ctx.events is None:
        return missing("LOG.md")
    reports, ends, lends = ctx.of("report", "I-01"), ctx.of("end", "RUN"), ctx.of("launch-end")
    # [inferred - audit] no launch-end line at all is a FAIL: "not crashed" needs an observed exit status.
    status = lends[-1].kv.get("status") if lends else None
    ok = bool(reports) and bool(ends) and status is not None and status not in ("crashed", "stalled")
    return (PASS if ok else FAIL), f"I-01 report={len(reports)} RUN end={len(ends)} last launch-end status={status}"


# ---------------------------------------------------------------- run2 checks (prompt rule disabled)

def _parked(ctx):
    return any(e.kv.get("state") == "parked" for e in ctx.of("status", "I-01"))


def check_a5_deny(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    c3, denies = count_event(ctx, 3), ctx.of("dispatch-denied")
    good = [d for d in denies if "phase 1 has failed verification 3 times" in d.kv.get("reason", "")]
    if good and c3 is not None and good[0].idx > c3.idx and not any(d.idx < c3.idx for d in denies):
        return PASS, "dispatch-denied for phase 1 after gaps_found count=3, none before"
    r3 = [e for e in ctx.dispatches() if e.kv.get("round", "").isdigit() and int(e.kv["round"]) >= 3]
    if not denies and _parked(ctx) and not r3:
        return NOTREACHED, "driver parked on its own before any round-3 dispatch; no deny to observe"
    return FAIL, f"denies={len(denies)} matching={len(good)} count3={'seen' if c3 else 'absent'} round>=3 dispatches={len(r3)}"


def check_a5_kill(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    kills = _kill(ctx)
    if not kills:
        return NOTREACHED, "source=not-reached (kill evidence then rests on L7-5)"
    src = "stand-in" if any(u <= kills[0].utc for u in ctx.standin) else "organic"
    return PASS, f"cap-tripped gap_rounds used=4 limit=2 source={src}"


def check_a11(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    c3 = count_event(ctx, 3)
    if c3 is None:
        return NOTREACHED, "gaps_found count=3 never observed"
    bad, late = [], [e for e in ctx.dispatches() if e.idx > c3.idx and
                     (e.kv.get("phase") == "1" or re.search(r"(?i)phase[\s_-]*0*1\b|\b1\s+--gaps", e.kv.get("step", "")))]
    unknown = 0
    for e in late:
        # WR-05: no look-behind. A dispatch is denied only if a dispatch-denied follows it before the next
        # report or dispatched; the denial that preceded it belongs to the earlier dispatch.
        verdict = None
        for n in ctx.events[e.idx + 1:]:
            if n.event == "dispatch-denied":
                verdict = "denied"
                break
            if n.event in ("report", "dispatched"):
                verdict = "ran"
                break
        if verdict == "ran":
            bad.append(e.idx)
        elif verdict is None:
            unknown += 1
    if bad:
        return FAIL, f"{len(bad)} phase-1 dispatch(es) after count=3 were not denied before a worker ran"
    if unknown:
        return UNDET, f"{unknown} phase-1 dispatch(es) after count=3 with no deny and no report"
    return PASS, f"phase-1 dispatches after count=3: {len(late)}, each denied"


# ---------------------------------------------------------------- shared checks A8 A9 A10

def load_drift():
    """spike 003's drift.py (the feed's titles and expected slugs), or None when it does not import."""
    import importlib.util
    try:
        spec = importlib.util.spec_from_file_location("spike003_drift", os.path.join(S3, "tools", "drift.py"))
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod if hasattr(mod, "emit") and hasattr(mod, "MAX_GEN") else None
    except Exception:
        return None


def gaps_block(text):
    if not text.startswith("---"):
        return ""
    out, on = [], False
    for l in text.split("\n---", 1)[0].splitlines():
        if not on:
            if l.startswith("gaps:"):
                on = True
                out.append(l[5:])
            continue
        if re.match(r"^[A-Za-z_][\w-]*:", l):
            break
        out.append(l)
    return "\n".join(out)


def _line_refs(block):
    cases = set()
    for m in re.finditer(r"\blines?\s+(\d+)(?:\s*[-\u2013]\s*(\d+))?", block, re.I):
        cases.add(("line", m.group(1)))
        if m.group(2):
            cases.add(("line", m.group(2)))
    return cases


def quoted_spans(block):
    """Exact contents of the backticked and quoted spans in a gaps block (WR-04). YAML-escaped `\\"x\\"` and plain
    "x" / 'x' / `x` are all found; an apostrophe inside a word does not open a span."""
    t = block.replace('\\"', "\x00")
    spans = re.findall(r"`([^`\n]{1,120})`", t)
    spans += re.findall(r"\x00([^\x00\n]{1,120}?)\x00", t)
    spans += re.findall(r'"([^"\n]{1,120}?)"', t.replace("\x00", " "))
    spans += re.findall(r"(?<![\w])'([^'\n]{1,120}?)'(?![\w])", t.replace("\x00", " "))
    return {x.strip() for x in spans if x.strip()}


def cases_in(block, drift):
    cases = _line_refs(block)
    if drift is not None:
        titles, expected = drift.emit(drift.MAX_GEN)
        spans = quoted_spans(block)          # WR-04: a title or slug counts only when quoted or backticked
        for t in set(titles):
            if t and t in spans:
                cases.add(("title", t))
        for x in set(expected):
            if x and x in spans:
                cases.add(("slug", x))
        return cases
    for m in re.finditer(r"`([^`\n]{1,80})`|\"([^\"\n]{1,80})\"", block):
        span = m.group(1) or m.group(2)
        if "/" in span or re.search(r"\.(py|md|txt|json|sh)$", span):
            continue
        cases.add(("span", span))
    return cases


def organic_gaps(ctx):
    """Organic gaps_found snapshots, one per burst (D-01 in scoring). A verifier that rewrites its report seconds later
    produces consecutive snapshots; `gapcount.coalesce` (the launcher's own 45 s sliding rule, on drift.log's
    whole-second UTC) groups them and the LAST snapshot of each burst is the round's snapshot."""
    snaps = ctx.snapshots()
    if snaps is None:
        return None
    organic = [s for s in snaps if s["status"] == "gaps_found" and not s["standin"]]
    times = [gapcount.epoch(s["utc"]) for s in organic]
    return [organic[b[-1]] for b in gapcount.coalesce(times)]


def check_a8(ctx):
    snaps = organic_gaps(ctx)
    if snaps is None:
        return missing("drift.log")
    if len(snaps) < 2:
        return NOTREACHED, f"{len(snaps)} gaps_found snapshot(s); need 2"
    drift = load_drift()
    mode = "drift" if drift is not None else "generic"
    sets = []
    for s in snaps:
        text = read(os.path.join(ctx.dir, "verification", s["name"]))
        if text is None:
            return missing(f"verification/{s['name']}")
        sets.append(cases_in(gaps_block(text), drift))
    empty = [i + 1 for i, c in enumerate(sets) if not c]
    if empty:
        return UNDET, f"mode={mode}: snapshot(s) {empty} yield no case"
    stale = [k + 1 for k in range(1, len(sets)) if not (sets[k] - sets[k - 1])]
    if stale:
        return FAIL, f"mode={mode}: snapshot(s) {stale} name only cases already in the previous one"
    return PASS, f"mode={mode}: every snapshot adds a new case ({'/'.join(str(len(c)) for c in sets)} cases)"


def verifier_runs(ctx):
    if ctx.dup is None:
        return None
    m = re.search(r"^types: (.*)$", ctx.dup, re.M)
    if not m:
        return None
    v = re.search(r"\bgsd-verifier=(\d+)", m.group(1))
    return int(v.group(1)) if v else 0


def converged(ctx):
    """The run reached a passing verification: a `passed` drift snapshot or an I-01 `status state=done` line."""
    return (any(s["status"] == "passed" for s in (ctx.snapshots() or []))
            or any(e.kv.get("state") == "done" for e in ctx.of("status", "I-01")))


def check_a9(ctx):
    if ctx.events is None:
        return missing("LOG.md")
    v = verifier_runs(ctx)
    if v is None:
        return UNDET, "no usable dup_check report (--dup-report)"
    seen = max(ctx.gap_counts() or [0]) - len(ctx.standin)      # gap-observed only; gap-coalesced lines are not rounds
    if seen <= 0 and v == 0:
        return NOTREACHED, "no gaps_found observed and no verifier ran"
    ev = f"gap-observed max count={seen + len(ctx.standin)} minus stand-in={len(ctx.standin)} vs gsd-verifier={v}"
    if seen == v:
        return PASS, ev
    if v == seen + 1 and converged(ctx):          # WR-03: a converged run's last verifier run passes, it is no gap round
        return PASS, ev + " (converged: the final verifier run found no gaps)"
    return FAIL, ev


def check_a10(ctx):
    snaps = organic_gaps(ctx)
    if snaps is None:
        return missing("drift.log")
    if len(snaps) < 2:
        return NOTREACHED, f"{len(snaps)} gaps_found snapshot(s); need 2"
    gens = [s["gen"] for s in snaps]                 # one snapshot per burst (see organic_gaps)
    ok = all(b > a for a, b in zip(gens, gens[1:]))
    return (PASS if ok else FAIL), f"gaps_found snapshot gens={gens}"


# ---------------------------------------------------------------- rubric-check

RUBRIC_START, RUBRIC_END = b"<!-- rubric:start -->", b"<!-- rubric:end -->"
# `--block NAME` -> (start marker, end marker, prereg file). The default (no --block) is the Phase 26 block above,
# untouched (T-26-10). Named blocks are fail-closed: anything absent is RUBRIC-MISSING, never RUBRIC-UNCHANGED.
RUBRIC_BLOCKS = {"26.1": (b"<!-- rubric-26.1:start -->", b"<!-- rubric-26.1:end -->", "prereg-26.1.txt")}


def _rubric(data, start=RUBRIC_START, end=RUBRIC_END):
    s = data.find(start)
    e = data.find(end, s + 1) if s >= 0 else -1
    return None if s < 0 or e < 0 else data[s + len(start):e]


def _rubric_block(base, sha, block):
    """Fail-closed comparison for a named block: (rc, text) with RUBRIC-MISSING / exit 2 for any absent input."""
    start, end, prereg = RUBRIC_BLOCKS[block]

    def missing_(why):
        return 2, "RUBRIC-MISSING\nrubric-check --block " + block + ": " + why
    if sha is None:
        t = read(os.path.join(base, "experiment", "results", prereg))
        if t is None:
            return missing_(f"experiment/results/{prereg} is absent")
        m = re.search(r"^PREREG_SHA=(\S+)", t, re.M)
        if not m:
            return missing_(f"no PREREG_SHA= in experiment/results/{prereg}")
        sha = m.group(1)
    r = subprocess.run(["git", "show", f"{sha}:./README.md"], cwd=base, capture_output=True)
    if r.returncode != 0:
        return missing_("cannot read README at " + sha + ": " + r.stderr.decode(errors="replace").strip()[:200])
    try:
        with open(os.path.join(base, "README.md"), "rb") as f:
            work = f.read()
    except OSError:
        return missing_("working-tree README.md is missing")
    old, new = _rubric(r.stdout, start, end), _rubric(work, start, end)
    if old is None or new is None:
        return missing_("block markers absent in " + ("the README at the prereg sha" if old is None else "the working tree"))
    if old == new:
        return 0, "RUBRIC-UNCHANGED"
    import difflib
    diff = difflib.unified_diff(old.decode(errors="replace").splitlines(), new.decode(errors="replace").splitlines(),
                                f"{sha}:README.md", "working-tree:README.md", lineterm="")
    return 1, "RUBRIC-CHANGED\n" + "\n".join(diff)


def rubric_check(base=S3, sha=None, block=None):
    """(rc, text): the README's pre-registered block at `sha` vs the working tree, byte for byte."""
    if block is not None:
        return _rubric_block(base, sha, block)
    if sha is None:
        m = re.search(r"^PREREG_SHA=(\S+)", read(os.path.join(base, "experiment", "results", "prereg.txt")) or "", re.M)
        if not m:
            return 2, "rubric-check: no sha (pass --sha, or write PREREG_SHA= to experiment/results/prereg.txt)"
        sha = m.group(1)
    r = subprocess.run(["git", "show", f"{sha}:./README.md"], cwd=base, capture_output=True)
    if r.returncode != 0:
        return 2, "rubric-check: cannot read README at " + sha + ": " + r.stderr.decode(errors="replace").strip()[:200]
    try:
        with open(os.path.join(base, "README.md"), "rb") as f:
            work = f.read()
    except OSError:
        return 2, "rubric-check: working-tree README.md is missing"
    old, new = _rubric(r.stdout), _rubric(work)
    if old is None or new is None:
        return 1, "RUBRIC-CHANGED\nrubric markers missing in " + ("the committed README" if old is None else "the working tree")
    if old == new:
        return 0, "RUBRIC-UNCHANGED"
    import difflib
    diff = difflib.unified_diff(old.decode(errors="replace").splitlines(), new.decode(errors="replace").splitlines(),
                                f"{sha}:README.md", "working-tree:README.md", lineterm="")
    return 1, "RUBRIC-CHANGED\n" + "\n".join(diff)


def seed_class(ctx, a10):
    if ctx.events is None:
        return "STALLED"
    parked = any(e.kv.get("state") == "parked" for e in ctx.of("status", "I-01"))
    done = any(e.kv.get("state") == "done" for e in ctx.of("status", "I-01"))   # (converged() minus the snapshot half)
    round1 = any(e.kv.get("round", "").isdigit() and int(e.kv["round"]) >= 1 for e in ctx.dispatches())
    snaps = ctx.snapshots() or []
    passed = any(s["status"] == "passed" for s in snaps) or done
    if parked and not round1:
        return "RECOGNISED"
    if passed:
        return "CONVERGED"
    # [inferred - audit, pre-registered by 26.1-04] an instrument death after round 1 (last launch-end stalled or
    # crashed, I-01 never parked or done) is a STALLED run for the retry rule, never a VALID seed that NO-GO (e)
    # could then fire on.
    ends = ctx.of("launch-end")
    if ends and ends[-1].kv.get("status") in ("stalled", "crashed") and not (parked or done):
        return "STALLED"
    if round1 and a10 == PASS:                      # WR-06: VALID needs a passing A10, not "not FAIL"
        return "VALID"
    return "STALLED"


# ---------------------------------------------------------------- modes

def checks_for(mode):
    if mode == "l8":
        return [("L8-1", check_l8_1), ("L8-2", check_l8_2), ("L8-3", check_l8_3), ("L8-4", check_l8_4)]
    if mode == "l7":
        return [("L7-1", check_l7_1), ("L7-2", check_l7_2), ("L7-3", check_l7_3), ("L7-4", check_l7_4),
                ("L7-5", check_l7_5), ("L7-6", check_l7_6)]
    shared = [("A6", check_a6), ("A7", check_a7), ("A8", check_a8), ("A9", check_a9), ("A10", check_a10)]
    if mode == "run1":
        return [("A1", check_a1), ("A2", check_a2), ("A3", check_a3), ("A4", check_a4)] + shared
    if mode == "run2":
        return [("A5-deny", check_a5_deny), ("A5-kill", check_a5_kill)] + shared + [("A11", check_a11)]
    return None


def aggregate(statuses):
    """(result, rc) for a set of check statuses. D-04 / T-26-11: only a decided run with a PASS reads as PASS."""
    vals = set(statuses)
    if FAIL in vals:
        return "FAIL", 1
    if UNDET in vals or PASS not in vals:
        return "INCONCLUSIVE", 2
    return "PASS", 0


def run_mode(mode, results, dup_report=None):
    """(lines, rc). One line per check, then SEED (run1/run2), then RESULT; rc 0 PASS / 1 FAIL / 2 INCONCLUSIVE."""
    ctx = Ctx(results, dup_report)
    table = checks_for(mode)
    if table is None:
        return [f"acceptance: unknown mode {mode!r}"], 2
    res = {}
    lines = []
    for cid, fn in table:
        st, ev = fn(ctx)
        res[cid] = st
        lines.append(f"{cid} {st} {ev}")
    if mode in ("run1", "run2"):
        lines.append("SEED " + seed_class(ctx, res.get("A10")))
    result, rc = aggregate(res.values())
    lines.append("RESULT " + result)
    return lines, rc


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv:
        print(__doc__)
        return 2
    if argv[0] == "selftest":
        return selftest()
    if argv[0] == "rubric-check":
        sha = None
        if "--sha" in argv:
            i = argv.index("--sha")
            if i + 1 >= len(argv):
                print("rubric-check: --sha needs a value")
                return 2
            sha = argv[i + 1]
        block = None
        if "--block" in argv:
            i = argv.index("--block")
            if i + 1 >= len(argv) or argv[i + 1] not in RUBRIC_BLOCKS:
                print("rubric-check: --block needs one of: " + ", ".join(sorted(RUBRIC_BLOCKS)))
                return 2
            block = argv[i + 1]
        rc, text = rubric_check(S3, sha, block)
        print(text)
        return rc
    dup = None
    if "--dup-report" in argv:
        i = argv.index("--dup-report")
        if i + 1 >= len(argv):
            print("acceptance: --dup-report needs a file")
            return 2
        dup = argv[i + 1]
        del argv[i:i + 2]
    if len(argv) != 2:
        print("Usage: acceptance.py l7|l8|run1|run2 <results-dir> [--dup-report FILE] | rubric-check [--sha S] [--block 26.1] | selftest")
        return 2
    lines, rc = run_mode(argv[0], argv[1], dup)
    print("\n".join(lines))
    return rc


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


def tracer_head():
    """Events of a Run-1-shaped LOG up to the last report: six dispatches, gap-observed 1..3 after each execute."""
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
    return ev


def tracer_tail():
    return [line(730, "I-01", "verified", **{"done-when": "not-met"}, evidence="gaps_found three times"),
            line(731, "I-01", "needs-human", question="feed keeps producing new rule classes"),
            line(732, "I-01", "status", state="parked", rule="divergence", outcome="feed diverges"),
            line(733, "RUN", "end", reason="all-parked"),
            line(735, "RUN", "disconnect", session="S1", reason="other"),
            line(736, "RUN", "launch-end", rc=0, cost_usd=1.0, status="parked")]


def tracer_log():
    return LOG_COMMENT + "\n".join(tracer_head() + tracer_tail()) + "\n"


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


# ---- Task 2 selftest cases: l7, run2, A8/A9/A10, rubric-check, spike-002 real logs

DUP_OK = ("dup_check: sessions=1 workers=6 DUP=0 EXECUTOR_MISSING=0 DRIVER_AGENT_CALLS=6 LOG_DISPATCHED=6 "
          "LOG_DENIED=0 SENDMESSAGE=0\ntypes: general-purpose=6 gsd-executor=3 gsd-planner=3 gsd-verifier={v}\n")
GAP_PATH = ".planning/phases/01-slugify/01-VERIFICATION.md"


def mkres(td, name, events, state=None, drift=None, standin=None, snaps=None, direction=DIRECTION_FIXTURE, dup=None):
    d = os.path.join(td, name)
    wr(os.path.join(d, "LOG.md"), LOG_COMMENT + "\n".join(events) + "\n")
    if state is not None:
        wr(os.path.join(d, "state.json"), json.dumps(state))
    if direction is not None:
        wr(os.path.join(d, "DIRECTION.md"), direction)
    if drift is not None:
        wr(os.path.join(d, "drift.log"), "\n".join(drift) + "\n")
    if standin is not None:
        wr(os.path.join(d, "standin.log"), "\n".join(standin) + "\n")
    for fn, text in (snaps or {}).items():
        wr(os.path.join(d, "verification", fn), text)
    if dup is not None:
        wr(os.path.join(d, "dup-report.txt"), dup)
    return d


def run_on(d, mode, dup=True):
    argv = [mode, d]
    if dup and os.path.exists(os.path.join(d, "dup-report.txt")):
        argv += ["--dup-report", os.path.join(d, "dup-report.txt")]
    return run_cli(argv)


def l7_events(deny_sec=55, cap=True, before=20, after=26, last="tripped"):
    ev = [line(0, "RUN", "launch", n=1, model="haiku"), line(1, "RUN", "connect", session="S1", source="startup"),
          line(5, "RUN", "start", mode="headless", loop_mode="loop", session="S1"),
          line(20, "I-01", "verified", **{"done-when": "not-met"}, evidence=f"feed-lines before={before} after={after}")]
    for n, sec in ((1, 30), (2, 40), (3, 50)):
        ev.append(line(sec, "RUN", "gap-observed", phase=1, count=n, file=GAP_PATH, source="launcher-scan"))
    deny = line(deny_sec, "RUN", "dispatch-denied", reason="phase 1 has failed verification 3 times (gap-round cap 2); "
                "no further dispatch for phase 1. Park the item with --rule gap-rounds.")
    ev.append(deny)
    ev.append(line(60, "RUN", "gap-observed", phase=1, count=4, file=GAP_PATH, source="launcher-scan"))
    if cap:
        ev.append(line(60, "RUN", "cap-tripped", cap="gap_rounds", used=4, limit=2, action="kill-scope"))
    if last == "tripped":
        ev.append(line(61, "RUN", "launch-end", rc=-15, tripped="gap_rounds", status="stopped"))
    else:
        ev.append(line(61, "RUN", "launch-end", rc=0, status="parked"))
    ev.sort(key=lambda e: e.split(" | ")[0])      # keep file order = time order, as the tool-written log is
    return ev


L7_DRIFT = ["2026-10-16T10:00:00Z start gen=0", "2026-10-16T10:00:17Z gen=1 summaries=1 head=a0cc27e titles=27"]


def run2_events(variant="full"):
    ev = tracer_head()
    if variant == "selfpark":
        return ev + [line(731, "I-01", "needs-human", question="park"),
                     line(732, "I-01", "status", state="parked", rule="divergence", outcome="self-limited"),
                     line(733, "RUN", "end", reason="all-parked"), line(736, "RUN", "launch-end", rc=0, status="parked")]
    ev.append(line(740, "I-01", "dispatched", step="/gsd-plan-phase 1 --gaps", base="a1b2c3f", round=3, dispatch="7/8", phase=1))
    if variant == "bypass":
        return ev + [line(760, "I-01", "report", status="done") + "\n    STATUS: done",
                     line(770, "I-01", "status", state="parked", rule="gap-rounds", outcome="late"),
                     line(771, "RUN", "end", reason="all-parked"), line(772, "RUN", "launch-end", rc=0, status="parked")]
    ev.append(line(741, "RUN", "dispatch-denied", reason="phase 1 has failed verification 3 times (gap-round cap 2); "
               "no further dispatch for phase 1. Park the item with --rule gap-rounds."))
    if variant == "nokill":
        return ev + [line(750, "I-01", "status", state="parked", rule="gap-rounds", outcome="denied"),
                     line(751, "RUN", "end", reason="all-parked"), line(752, "RUN", "launch-end", rc=0, status="parked")]
    return ev + [line(745, "RUN", "gap-observed", phase=1, count=4, file=GAP_PATH, source="launcher-scan"),
                 line(745, "RUN", "cap-tripped", cap="gap_rounds", used=4, limit=2, action="kill-scope"),
                 line(746, "RUN", "launch-end", rc=-15, tripped="gap_rounds", status="stopped")]


def run2_state(variant="full"):
    n = 6 if variant == "selfpark" else 7
    return {"items": {"I-01": {"dispatches": n}}, "hook_dispatches": 6 if variant != "bypass" else 7,
            "denied_dispatches": 0 if variant in ("selfpark", "bypass") else 1}


def snap_text(status, gaps_lines):
    body = "".join(f"    {l}\n" for l in gaps_lines)
    return f"---\nphase: 01-slugify\nverified: 2026-10-16T10:00:00Z\nstatus: {status}\ngaps:\n  - truth: \"feed\"\n{body}score: 1/3\n---\n\n# body mentions `unrelated-slug`\n"


def drift_for(gens, statuses=None, secs=None):
    """drift.log + snapshot files for the given gens (all gaps_found by default); `secs` = snapshot times
    (default 120 s apart, so every snapshot is its own burst)."""
    lines, snaps = list(L7_DRIFT[:1]), {}
    for i, g in enumerate(gens):
        st = (statuses or {}).get(i, "gaps_found")
        name = f"{i + 1:02d}-2026101610{i:02d}00Z-h{i:07d}.md"
        lines.append(f"{at(secs[i] if secs else 100 + 120 * i)} snapshot file={GAP_PATH} hash=h{i:07d} status={st} gen={g} -> {name}")
        snaps[name] = None
    return lines, snaps


NEW_BY_GEN = {1: ("C++", "c-plus-plus"), 2: ("R&D", "rnd"), 3: ("The Matrix", "matrix")}


def gaps_for(*gens):
    out = []
    for g in gens:
        t, s = NEW_BY_GEN[g]
        out.append(f"reason: \"line 2{g}: slugify('{t}') must equal `{s}`\"")
    return out


def a8_case(td, name, per_snapshot, gens=(1, 2, 3), secs=None):
    lines, snaps = drift_for(gens, secs=secs)
    names = list(snaps)
    for n, gl in zip(names, per_snapshot):
        snaps[n] = snap_text("gaps_found", gl)
    d = mkres(td, name, tracer_head() + tracer_tail(), state=json.loads(tracer_state()), drift=lines, snaps=snaps,
              dup=DUP_OK.format(v=3))
    return d


def selftest_modes(td, outs):
    # ---------------- l7
    st = {"items": {}, "hook_dispatches": 1, "denied_dispatches": 1}
    d = mkres(td, "l7ok", l7_events(), state=st, drift=L7_DRIFT, direction=None)
    rc, out = run_on(d, "l7")
    outs.append(out)
    for cid in ("L7-1", "L7-2", "L7-3", "L7-4", "L7-5", "L7-6"):
        assert status_of(out, cid) == PASS, (cid, out)
    assert rc == 0 and "RESULT PASS" in out and "SEED" not in out, (rc, out)
    d = mkres(td, "l7early", l7_events(deny_sec=45), state=st, drift=L7_DRIFT, direction=None)
    _, out = run_on(d, "l7")
    assert status_of(out, "L7-3") == FAIL, out
    d = mkres(td, "l7nocap", l7_events(cap=False), state=st, drift=L7_DRIFT, direction=None)
    _, out = run_on(d, "l7")
    assert status_of(out, "L7-5") == FAIL, out
    d = mkres(td, "l7nofeed", l7_events(after=20), state=st, drift=L7_DRIFT, direction=None)
    _, out = run_on(d, "l7")
    assert status_of(out, "L7-1") == FAIL, out
    d = mkres(td, "l7parked", l7_events(last="parked"), state=st, drift=L7_DRIFT, direction=None)
    _, out = run_on(d, "l7")
    assert status_of(out, "L7-6") == FAIL, out
    d = mkres(td, "l7nodrift", l7_events(), state=st, drift=None, direction=None)
    _, out = run_on(d, "l7")
    assert status_of(out, "L7-1") == UNDET, out
    # ---------------- run2
    def r2(variant, standin=True, dup=True):
        events = run2_events(variant)
        d = mkres(td, f"r2{variant}{int(standin)}", events, state=run2_state(variant),
                  standin=[f"{at(743)} trigger=parked wrote gaps_found via gap-writer.sh"] if standin and variant == "full" else None,
                  drift=["2026-10-16T10:00:00Z start gen=0"], dup=DUP_OK.format(v=3) if dup else None)
        return run_on(d, "run2")
    rc, out = r2("full")
    outs.append(out)
    assert status_of(out, "A5-deny") == PASS and status_of(out, "A5-kill") == PASS, out
    assert "source=stand-in" in out, out
    assert status_of(out, "A11") == PASS and status_of(out, "A9") == PASS and status_of(out, "A6") == PASS, out
    _, out = r2("full", standin=False)
    assert status_of(out, "A5-kill") == PASS and "source=organic" in out, out
    _, out = r2("nokill")
    assert status_of(out, "A5-deny") == PASS and status_of(out, "A5-kill") == NOTREACHED, out
    _, out = r2("selfpark")
    assert status_of(out, "A5-deny") == NOTREACHED and status_of(out, "A5-kill") == NOTREACHED, out
    rc, out = r2("bypass")
    outs.append(out)
    assert status_of(out, "A11") == FAIL and status_of(out, "A5-deny") == FAIL and rc != 0, out
    # ---------------- A8 findings non-subset
    d = a8_case(td, "a8pass", [gaps_for(1), gaps_for(2), gaps_for(3)])
    rc, out = run_on(d, "run1")
    outs.append(out)
    assert status_of(out, "A8") == PASS and "mode=drift" in out, out
    d = a8_case(td, "a8fail", [gaps_for(1), gaps_for(2), gaps_for(2)])
    _, out = run_on(d, "run1")
    assert status_of(out, "A8") == FAIL, out
    d = a8_case(td, "a8undet", [gaps_for(1), ["reason: \"nothing named\""], gaps_for(3)])
    _, out = run_on(d, "run1")
    assert status_of(out, "A8") == UNDET, out
    d = a8_case(td, "a8generic", [gaps_for(1), gaps_for(2), gaps_for(3)])
    saved = load_drift
    try:
        globals()["load_drift"] = lambda: None
        _, out = run_on(d, "run1")
    finally:
        globals()["load_drift"] = saved
    assert status_of(out, "A8") == PASS and "mode=generic" in out, out
    d = mkres(td, "a8nodrift", tracer_head() + tracer_tail(), state=json.loads(tracer_state()), dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A8") == UNDET and status_of(out, "A10") == UNDET, out
    # ---------------- A9 gap-count sanity
    d = a8_case(td, "a9ok", [gaps_for(1), gaps_for(2), gaps_for(3)])
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == PASS, out
    d = mkres(td, "a9bad", tracer_head() + tracer_tail(), state=json.loads(tracer_state()), dup=DUP_OK.format(v=2))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == FAIL, out
    d = mkres(td, "a9nodup", tracer_head() + tracer_tail(), state=json.loads(tracer_state()))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == UNDET, out
    # ---------------- A10 moving-target ordering
    for name, gens, want in (("a10ok", (1, 2, 3), PASS), ("a10flat", (1, 1, 2), FAIL), ("a10one", (1,), NOTREACHED)):
        lines, snaps = drift_for(gens)
        for n in snaps:
            snaps[n] = snap_text("gaps_found", gaps_for(1))
        d = mkres(td, name, tracer_head() + tracer_tail(), state=json.loads(tracer_state()), drift=lines, snaps=snaps,
                  dup=DUP_OK.format(v=3))
        _, out = run_on(d, "run1")
        assert status_of(out, "A10") == want, (name, out)
        if want == FAIL:
            assert "SEED STALLED" in out, out
    # converged seed: a passed snapshot, and a seed parked before any closure round
    lines, snaps = drift_for((1, 2), statuses={1: "passed"})
    for n in snaps:
        snaps[n] = snap_text("gaps_found", gaps_for(1))
    d = mkres(td, "converged", tracer_head() + tracer_tail(), state=json.loads(tracer_state()), drift=lines, snaps=snaps)
    _, out = run_on(d, "run1")
    assert "SEED CONVERGED" in out, out
    d = mkres(td, "recognised", tracer_head()[:5] + tracer_tail(), state=json.loads(tracer_state()))
    _, out = run_on(d, "run1")
    assert "SEED RECOGNISED" in out, out
    # ---------------- WR-06 / T-26-11: SEED VALID needs A10 PASS; a stalled or crashed exit is STALLED
    d = mkres(td, "seedundet", tracer_head() + tracer_tail(), state=json.loads(tracer_state()), dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A10") == UNDET and "SEED STALLED" in out and "SEED VALID" not in out, out
    d = mkres(td, "seednotreached", tracer_head() + tracer_tail(), state=json.loads(tracer_state()),
              drift=["2026-10-16T10:00:00Z start gen=0"], dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A10") == NOTREACHED and "SEED STALLED" in out and "SEED VALID" not in out, out
    lines, snaps = drift_for((1, 2, 3))
    for n, gl in zip(snaps, (gaps_for(1), gaps_for(2), gaps_for(3))):
        snaps[n] = snap_text("gaps_found", gl)

    def seeded(name, tail):
        return run_on(mkres(td, name, tracer_head() + tail, state=json.loads(tracer_state()), drift=lines, snaps=snaps,
                            dup=DUP_OK.format(v=3)), "run1")[1]
    out = seeded("seedvalid", tracer_tail())
    assert status_of(out, "A10") == PASS and "SEED VALID" in out, out
    for how in ("crashed", "stalled"):
        out = seeded("seed" + how, [line(736, "RUN", "launch-end", rc=0, cost_usd=1.0, status=how)])
        assert status_of(out, "A10") == PASS and "SEED STALLED" in out and "SEED VALID" not in out, (how, out)
    out = seeded("seedstalledparked", tracer_tail()[:3] + [line(736, "RUN", "launch-end", rc=0, status="stalled")])
    assert "SEED VALID" in out, out            # I-01 did park, so the instrument-death precondition does not apply
    # ---------------- three-way RESULT through the CLI: PASS with NOT-REACHED, FAIL outranks UNDETERMINED
    rc, out = r2("selfpark")
    assert status_of(out, "A5-deny") == NOTREACHED and status_of(out, "A10") == NOTREACHED, out
    assert rc == 0 and out.rstrip().endswith("RESULT PASS") and "UNDETERMINED" not in out, (rc, out)
    # ---------------- D-01 in scoring: A8 / A10 score the last snapshot of each burst (committed results/run1 shape)
    burst = [gaps_for(1), gaps_for(2), gaps_for(2)]            # a verifier re-write: same findings, seconds later
    d = a8_case(td, "burst", burst, gens=(1, 2, 2), secs=(100, 700, 706))
    _, out = run_on(d, "run1")
    outs.append(out)
    assert status_of(out, "A10") == PASS and "gens=[1, 2]" in out and status_of(out, "A8") == PASS, out
    assert re.search(r"^A8 PASS .*\(\d+/\d+ cases\)", out, re.M), out
    d = a8_case(td, "burst44", burst, gens=(1, 2, 2), secs=(100, 700, 744))     # 44 s after: still one burst
    _, out = run_on(d, "run1")
    assert status_of(out, "A10") == PASS and "gens=[1, 2]" in out, out
    d = a8_case(td, "burst45", burst, gens=(1, 2, 2), secs=(100, 700, 745))         # exactly 45 s: a new round
    _, out = run_on(d, "run1")
    assert status_of(out, "A10") == FAIL and "gens=[1, 2, 2]" in out and status_of(out, "A8") == FAIL, out
    d = a8_case(td, "burstone", [gaps_for(1), gaps_for(1)], gens=(1, 1), secs=(100, 110))   # one burst: one round
    _, out = run_on(d, "run1")
    assert status_of(out, "A10") == NOTREACHED and status_of(out, "A8") == NOTREACHED, out
    # a stand-in snapshot is still excluded before grouping
    lines, snaps = drift_for((1, 2, 3), secs=(100, 700, 706))
    for n, gl in zip(snaps, (gaps_for(1), gaps_for(2), gaps_for(3))):
        snaps[n] = snap_text("gaps_found", gl)
    d = mkres(td, "burststandin", run2_events("full"), state=run2_state("full"),
              standin=[at(705)], drift=lines, snaps=snaps, dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run2")
    assert status_of(out, "A10") == PASS and "gens=[1, 2]" in out, out
    # ---------------- WR-03: a converged run (gaps, gaps, passed) has verifier runs = gaps_found + 1
    two_gaps = [e for e in tracer_head() if "count=3" not in e]
    lines, snaps = drift_for((1, 2, 3), statuses={2: "passed"})
    for n, gl in zip(snaps, (gaps_for(1), gaps_for(2), gaps_for(3))):
        snaps[n] = snap_text("gaps_found", gl)
    d = mkres(td, "a9conv", two_gaps, state=json.loads(tracer_state()), drift=lines, snaps=snaps, dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == PASS and "converged" in out and "SEED CONVERGED" in out, out
    lines, snaps = drift_for((1, 2, 3))
    for n, gl in zip(snaps, (gaps_for(1), gaps_for(2), gaps_for(3))):
        snaps[n] = snap_text("gaps_found", gl)
    d = mkres(td, "a9noconv", two_gaps, state=json.loads(tracer_state()), drift=lines, snaps=snaps, dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == FAIL, out                  # without the passed snapshot: compared as before
    d = mkres(td, "a9done", two_gaps + [line(735, "I-01", "status", state="done", outcome="closed")],
              state=json.loads(tracer_state()), drift=lines, snaps=snaps, dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == PASS, out                  # I-01 status done also marks convergence
    d = mkres(td, "a9conv4", two_gaps, state=json.loads(tracer_state()), drift=drift_for((1, 2, 3), statuses={2: "passed"})[0],
              dup=DUP_OK.format(v=4))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == FAIL, out                  # only +1 is allowed
    # a gap-coalesced line never raises A9's count
    coal = tracer_head() + [line(640, "RUN", "gap-coalesced", phase=1, count=3, file=GAP_PATH, source="launcher-scan")]
    d = mkres(td, "a9coal", coal, state=json.loads(tracer_state()), dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == PASS and "max count=3" in out, out
    coal = tracer_head() + [line(640, "RUN", "gap-coalesced", phase=1, count=7, file=GAP_PATH, source="launcher-scan")]
    d = mkres(td, "a9coal7", coal, state=json.loads(tracer_state()), dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run1")
    assert status_of(out, "A9") == PASS and "max count=3" in out, out
    # ---------------- WR-04: a title or slug is a case only when quoted or backticked
    dr = load_drift()
    assert dr is not None
    prose = 'reason: "the report says The End and the end of the file, then stops"'
    assert ("title", "The End") not in cases_in(prose, dr) and ("slug", "end") not in cases_in(prose, dr), cases_in(prose, dr)
    for quoted in ("reason: \"title `The End` fails\"", "reason: \"title 'The End' fails\"",
                   "reason: \"title \\\"The End\\\" fails\"", "reason: \"slug `end` wrong\""):
        c = cases_in(quoted, dr)
        assert ("title", "The End") in c or ("slug", "end") in c, (quoted, c)
    assert ("title", "The End") not in cases_in("reason: \"doesn't treat The End as a title\"", dr)
    d = a8_case(td, "a8prose", [gaps_for(1), gaps_for(1) + [prose]], gens=(1, 2))
    _, out = run_on(d, "run1")
    assert status_of(out, "A8") == FAIL, out                  # prose "The End" is no new case
    d = a8_case(td, "a8quoted", [gaps_for(1), gaps_for(1) + ["reason: \"title `The End` fails\""]], gens=(1, 2))
    _, out = run_on(d, "run1")
    assert status_of(out, "A8") == PASS, out
    # ---------------- WR-05: dispatched(A), dispatch-denied, dispatched(B), report -> A11 FAIL for B
    deny = line(741, "RUN", "dispatch-denied", reason="phase 1 has failed verification 3 times (gap-round cap 2); "
                "no further dispatch for phase 1. Park the item with --rule gap-rounds.")
    disp = lambda sec, n: line(sec, "I-01", "dispatched", step="/gsd-plan-phase 1 --gaps", base="a1b2c3f", round=3,
                               dispatch=f"{n}/8", phase=1)
    seq = tracer_head() + [disp(740, 7), deny, disp(742, 8), line(760, "I-01", "report", status="done") + "\n    STATUS: done"]
    d = mkres(td, "a11adjacent", seq, state=run2_state("full"), drift=["2026-10-16T10:00:00Z start gen=0"], dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run2")
    assert status_of(out, "A11") == FAIL and "1 phase-1 dispatch" in out, out
    seq = tracer_head() + [disp(740, 7), deny, disp(742, 8), line(743, "RUN", "dispatch-denied", reason="phase 1 has failed verification 3 times (gap-round cap 2)")]
    d = mkres(td, "a11bothdenied", seq, state=run2_state("full"), drift=["2026-10-16T10:00:00Z start gen=0"], dup=DUP_OK.format(v=3))
    _, out = run_on(d, "run2")
    assert status_of(out, "A11") == PASS, out
    # ---------------- l8: guard fired, resumed by SendMessage, no respawn, reported and not crashed
    def l8_case(name, block=True, sendmsg=1, dup_n=0, workers=1, report=True, end=True, status="parked", guard=True, dup=True):
        ev = [line(0, "RUN", "launch", n=1, model="haiku"), line(1, "RUN", "connect", session="S1", source="startup"),
              line(5, "RUN", "start", mode="headless", loop_mode="loop", session="S1"),
              line(10, "I-01", "dispatched", step="/gsd-plan-phase 1", base="a1b2c3d", round=0, dispatch="1/8", phase=1)]
        if report:
            ev.append(line(60, "I-01", "report", status="done") + "\n    STATUS: done")
        if end:
            ev.append(line(61, "RUN", "end", reason="all-parked"))
        ev.append(line(62, "RUN", "launch-end", rc=0, status=status))
        d = mkres(td, name, ev, direction=None,
                  dup=(f"dup_check: sessions=1 workers={workers} DUP={dup_n} EXECUTOR_MISSING=0 DRIVER_AGENT_CALLS=1 "
                       f"LOG_DISPATCHED=1 LOG_DENIED=0 SENDMESSAGE={sendmsg}\ntypes: general-purpose=1\n") if dup else None)
        if guard:
            wr(os.path.join(d, "stop-guard.log"), (f"{at(30)} block item=I-01 attempt=1\n" if block else "") + f"{at(55)} allow reason=no-outstanding\n")
        return run_on(d, "l8")
    rc, out = l8_case("l8ok")
    outs.append(out)
    for cid in ("L8-1", "L8-2", "L8-3", "L8-4"):
        assert status_of(out, cid) == PASS, (cid, out)
    assert rc == 0 and out.rstrip().endswith("RESULT PASS") and "SEED" not in out, (rc, out)
    for name, kw, cid in (("l8noblock", {"block": False}, "L8-1"), ("l8nosend", {"sendmsg": 0}, "L8-2"),
                          ("l8dup", {"dup_n": 1}, "L8-3"), ("l8respawn", {"workers": 2}, "L8-3"),
                          ("l8noreport", {"report": False}, "L8-4"), ("l8noend", {"end": False}, "L8-4"),
                          ("l8stalled", {"status": "stalled"}, "L8-4"), ("l8crashed", {"status": "crashed"}, "L8-4")):
        rc, out = l8_case(name, **kw)
        assert status_of(out, cid) == FAIL and rc == 1 and out.rstrip().endswith("RESULT FAIL"), (name, rc, out)
    for name, kw, cids in (("l8noguard", {"guard": False}, ("L8-1",)), ("l8nodup", {"dup": False}, ("L8-2", "L8-3"))):
        rc, out = l8_case(name, **kw)
        assert all(status_of(out, c) == UNDET for c in cids) and rc == 2 and out.rstrip().endswith("RESULT INCONCLUSIVE"), (name, rc, out)
    # ---------------- rubric-check in a temp git repo
    repo = os.path.join(td, "repo")
    spike = os.path.join(repo, "spike")
    readme = "# t\n\nbefore\n<!-- rubric:start -->\nGO if A1 PASS\nNO-GO if A2 FAIL\n<!-- rubric:end -->\nafter\n"
    wr(os.path.join(spike, "README.md"), readme)
    git = lambda *a: subprocess.run(["git", "-c", "user.email=s@example.invalid", "-c", "user.name=s",
                                     "-c", "core.hooksPath=/dev/null", *a], cwd=repo, capture_output=True, text=True, check=True)
    git("init", "-q")
    git("add", "-A")
    git("commit", "-q", "-m", "prereg")
    sha = git("rev-parse", "HEAD").stdout.strip()
    wr(os.path.join(spike, "experiment", "results", "prereg.txt"), f"PREREG_SHA={sha}\n")
    rc, out = rubric_check(spike)
    assert rc == 0 and out.startswith("RUBRIC-UNCHANGED"), (rc, out)
    wr(os.path.join(spike, "README.md"), readme.replace("A1 PASS", "A1 PAS5"))
    rc, out = rubric_check(spike)
    assert rc == 1 and out.startswith("RUBRIC-CHANGED") and "PAS5" in out, (rc, out)
    wr(os.path.join(spike, "README.md"), readme.replace("after", "after, edited outside the markers"))
    rc, out = rubric_check(spike)
    assert rc == 0 and out.startswith("RUBRIC-UNCHANGED"), (rc, out)
    wr(os.path.join(spike, "README.md"), readme.replace("<!-- rubric:end -->", ""))
    rc, out = rubric_check(spike)
    assert rc == 1 and "RUBRIC-CHANGED" in out, (rc, out)
    wr(os.path.join(spike, "README.md"), readme)
    rc, out = rubric_check(spike, sha="HEAD")
    assert rc == 0, (rc, out)
    os.remove(os.path.join(spike, "experiment", "results", "prereg.txt"))
    rc, out = rubric_check(spike)
    assert rc == 2, (rc, out)
    # ---- rubric-check --block 26.1: fail-closed, and the default path is untouched by it
    r261 = "<!-- rubric-26.1:start -->\nGO if L1 PASS\nNO-GO if L2 FAIL\n<!-- rubric-26.1:end -->\n"
    pre = os.path.join(spike, "experiment", "results", "prereg-26.1.txt")
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 2 and out.splitlines()[0] == "RUBRIC-MISSING" and "RUBRIC-UNCHANGED" not in out, (rc, out)
    wr(os.path.join(spike, "README.md"), readme + r261)
    git("add", "-A")
    git("commit", "-q", "-m", "prereg 26.1")
    sha2 = git("rev-parse", "HEAD").stdout.strip()
    wr(pre, "RECORDED_UTC=2026-10-17T00:00:00Z\n")                        # no PREREG_SHA line
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 2 and out.splitlines()[0] == "RUBRIC-MISSING", (rc, out)
    wr(pre, f"PREREG_SHA={sha2}\n")
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 0 and out == "RUBRIC-UNCHANGED", (rc, out)
    wr(os.path.join(spike, "README.md"), (readme + r261).replace("L1 PASS", "L1 PAS5"))
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 1 and out.startswith("RUBRIC-CHANGED") and "PAS5" in out, (rc, out)
    wr(os.path.join(spike, "experiment", "results", "prereg.txt"), f"PREREG_SHA={sha}\n")
    rc, out = rubric_check(spike)                                          # editing the 26.1 block leaves the old rubric intact
    assert rc == 0 and out == "RUBRIC-UNCHANGED", (rc, out)
    wr(os.path.join(spike, "README.md"), readme)                           # working tree lost the 26.1 block
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 2 and out.splitlines()[0] == "RUBRIC-MISSING", (rc, out)
    wr(pre, f"PREREG_SHA={sha}\n")                                        # README at that sha never had the 26.1 block
    wr(os.path.join(spike, "README.md"), readme + r261)
    rc, out = rubric_check(spike, block="26.1")
    assert rc == 2 and out.splitlines()[0] == "RUBRIC-MISSING", (rc, out)
    rc, out = rubric_check(spike, sha=sha2, block="26.1")                  # --sha overrides the prereg file
    assert rc == 0 and out == "RUBRIC-UNCHANGED", (rc, out)
    os.remove(pre)
    rc, out = rubric_check(spike, sha=sha2, block="26.1")
    assert rc == 0, (rc, out)
    rc, out = run_cli(["rubric-check", "--block", "27"])
    assert rc == 2 and "RUBRIC-UNCHANGED" not in out, (rc, out)
    rc, out = run_cli(["rubric-check", "--block"])
    assert rc == 2, (rc, out)
    # ---------------- spike 002's real committed logs parse (from their real paths, never copies)
    real = os.path.join(S3, "..", "002-launcher-caps-and-tool-frontmatter", "experiment", "results")
    evs, bad = parse_log(read(os.path.join(real, "cap-tests", "L3-LOG.md")))
    assert not bad, bad
    c3 = next(e for e in evs if e.event == "gap-observed" and e.kv.get("count") == "3")
    assert any(e.event == "dispatch-denied" and e.idx > c3.idx for e in evs)
    evs, bad = parse_log(read(os.path.join(real, "cap-tests", "L4-LOG.md")))
    assert not bad and any(e.event == "cap-tripped" and e.kv.get("used") == "4" and e.kv.get("limit") == "2" for e in evs)
    evs, bad = parse_log(read(os.path.join(real, "R2-hidden-fixture", "LOG.md")))
    assert not bad
    reports = [e for e in evs if e.event == "report"]
    assert len(reports) == 2 and len(reports[0].detail) >= 8 and reports[0].detail[0] == "STATUS: done", reports
    assert any(e.event == "status" and e.kv.get("rule") == "needs-human" for e in evs)
    assert any("DIRECTION" not in e.event for e in evs) and evs[0].event == "launch"
    # ---------------- inputs missing -> UNDETERMINED for every dependent check
    rc, out = run_cli(["run2", os.path.join(td, "nowhere")])
    assert status_of(out, "A5-deny") == UNDET and status_of(out, "A6") == UNDET, out


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

        # T-26-11: this fixture used to carry no drift.log, so A8 and A10 were UNDETERMINED and the old aggregate
        # still printed RESULT PASS. It now gains drift.log + snapshots (three bursts, gens 1 2 3) so EVERY check
        # decides; the undecided variants are asserted INCONCLUSIVE below.
        lines, snaps = drift_for((1, 2, 3))
        for n, gl in zip(snaps, (gaps_for(1), gaps_for(2), gaps_for(3))):
            wr(os.path.join(res, "verification", n), snap_text("gaps_found", gl))
        wr(os.path.join(res, "drift.log"), "\n".join(lines) + "\n")

        rc, out = run_cli(["run1", res, "--dup-report", os.path.join(res, "dup-report.txt")])
        outs.append(out)
        for cid in ("A1", "A2", "A3", "A4", "A6", "A7", "A8", "A9", "A10"):
            assert status_of(out, cid) == PASS, (cid, out)
        assert "SEED VALID" in out and rc == 0 and out.rstrip().endswith("RESULT PASS"), (rc, out)

        # a third closure round makes A1 FAIL and the exit non-zero
        with open(os.path.join(res, "LOG.md"), "a") as f:
            f.write(line(740, "I-01", "dispatched", step="/gsd-plan-phase 1 --gaps", base="a1b2c3f", round=3,
                         dispatch="7/8", phase=1) + "\n")
        rc, out = run_cli(["run1", res, "--dup-report", os.path.join(res, "dup-report.txt")])
        outs.append(out)
        assert status_of(out, "A1") == FAIL and rc == 1 and out.rstrip().endswith("RESULT FAIL"), (rc, out)

        # no dup report -> A7 UNDETERMINED, never PASS; PASS plus UNDETERMINED (and a FAIL absent) is INCONCLUSIVE
        rc, out = run_cli(["run1", res])
        outs.append(out)
        assert status_of(out, "A7") == UNDET, out
        # (the appended third round still makes A1 FAIL, so FAIL outranks UNDETERMINED: RESULT FAIL, rc 1)
        assert rc == 1 and out.rstrip().endswith("RESULT FAIL"), (rc, out)
        wr(os.path.join(res, "LOG.md"), tracer_log())
        rc, out = run_cli(["run1", res])
        outs.append(out)
        assert status_of(out, "A7") == UNDET and status_of(out, "A1") == PASS, out
        assert rc == 2 and out.rstrip().endswith("RESULT INCONCLUSIVE"), (rc, out)
        # missing results dir and empty results dir -> UNDETERMINED across the board, INCONCLUSIVE, rc 2, in every mode
        rc, out = run_cli(["run1", os.path.join(td, "nowhere")])
        outs.append(out)
        assert status_of(out, "A4") == UNDET and status_of(out, "A6") == UNDET, out
        empty = os.path.join(td, "emptydir")
        os.makedirs(empty)
        for mode in ("l7", "l8", "run1", "run2"):
            for target in (os.path.join(td, "nowhere"), empty):
                rc, out = run_cli([mode, target])
                outs.append(out)
                assert rc == 2 and out.rstrip().endswith("RESULT INCONCLUSIVE"), (mode, target, rc, out)
                assert "RESULT PASS" not in out, (mode, target, out)
        assert run_mode("bogus", empty)[1] == 2
        # three-way aggregate, pure
        assert aggregate([PASS, PASS]) == ("PASS", 0)
        assert aggregate([PASS, NOTREACHED]) == ("PASS", 0)
        assert aggregate([PASS, FAIL, UNDET]) == ("FAIL", 1)
        assert aggregate([PASS, UNDET]) == ("INCONCLUSIVE", 2)
        assert aggregate([UNDET, UNDET]) == ("INCONCLUSIVE", 2)
        assert aggregate([NOTREACHED, NOTREACHED]) == ("INCONCLUSIVE", 2)       # nothing ever passed
        assert aggregate([]) == ("INCONCLUSIVE", 2)
        selftest_modes(td, outs)
    assert not any(MARKER in o for o in outs), "transcript text leaked into a tool's output"
    print("selftest OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
