#!/usr/bin/env python3
"""Spike 003 (Phase 26.1) D-07: measured spike-003 spend against the campaign ceiling.

The $60 spike-003 ceiling used to be a README procedure. run.sh calls this tool before every launch
and before every in-script auth relaunch (any mode) and refuses with exit 8 when the measured total plus
the launch's `--max-budget-usd` would exceed the ceiling.

Cost of one results subdirectory (first rule that applies):
  1. a dir holding only SKIPPED.txt costs 0 (a documented skipped run);
  2. `stage-cost.txt` with a `TOTAL ... total=X` line (X is the last TOTAL line), whose header
     `launches=N` covers every launcher-result (WR-04): the larger of X and the priced launcher-result sum.
     When the header covers fewer launches than there are `launcher-result-*.json` files (a relaunch landed
     after stage-cost.txt was written) the stage total is stale: the dir is charged the larger of X and
     (priced launcher-result sum + each unpriced launcher-result at the dir's `MAX_BUDGET_USD`);
  3. else the sum of `cost.total_cost_usd` over `launcher-result-*.json`; a launcher-result without a
     numeric cost (a launch killed before its `result` event) is charged the dir's `MAX_BUDGET_USD`;
  4. else (a launch with neither) its `MAX_BUDGET_USD` from run-env.txt.
Rules 2 (stale part), 3 (unpriced part) and 4 are printed as `ESTIMATED label=<l> usd=<c> reason=<why>`:
the worst case, never a guess below it. Haiku launches count toward SPENT_ALL; only `MODE=run1|run2` launches count toward
SPENT_REAL (the figure the original rubric tracked).

Usage:
  campaign_budget.py [--results DIR] [--ceiling USD] [--next USD]
  campaign_budget.py selftest

Defaults: --results is this spike's experiment/results, --ceiling 60, --next 0. Output: one `COST` line
per results subdirectory, the `ESTIMATED` lines, then the last line
  SPENT_ALL=<4dp> SPENT_REAL=<4dp> NEXT=<n> CEILING=<c> OK|OVER
(OVER when SPENT_ALL + NEXT > CEILING; exactly equal is OK).
Exit codes: 0 OK; 1 OVER; 2 unreadable results, unpriceable directory or bad arguments (fail closed).
"""
import argparse
import contextlib
import glob
import io
import json
import math
import os
import re
import sys
import tempfile

sys.dont_write_bytecode = True

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_RESULTS = os.path.normpath(os.path.join(HERE, "..", "experiment", "results"))
REAL_MODES = ("run1", "run2")
TOTAL_RE = re.compile(r"^TOTAL\b.*\btotal=([0-9]+(?:\.[0-9]+)?)\b")
LAUNCHES_RE = re.compile(r"\blaunches=([0-9]+)\b")
MICRO = 1_000_000


class Unreadable(Exception):
    pass


def micro(x):
    """Dollars to integer micro-dollars so the OVER comparison is exact at the boundary."""
    return int(round(x * MICRO))


def read_env(path):
    env = {}
    try:
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                k, sep, v = line.rstrip("\n").partition("=")
                if sep:
                    env[k] = v
    except FileNotFoundError:
        return None
    except OSError as e:
        raise Unreadable("%s: %s" % (path, e))
    return env


def stage_total(path):
    """(last TOTAL, launches covered by the header) or None. A missing `launches=` field counts as 0."""
    try:
        with open(path, encoding="utf-8") as fh:
            lines = fh.read().splitlines()
    except FileNotFoundError:
        return None
    except OSError as e:
        raise Unreadable("%s: %s" % (path, e))
    total = None
    for line in lines:
        m = TOTAL_RE.match(line)
        if m:
            total = float(m.group(1))
    if total is None:
        return None
    m = LAUNCHES_RE.search(lines[0]) if lines else None
    return total, (int(m.group(1)) if m else 0)


def launcher_costs(d):
    """List of per-launcher-result costs; None entries are results with no numeric cost."""
    out = []
    for p in sorted(glob.glob(os.path.join(d, "launcher-result-*.json"))):
        try:
            with open(p, encoding="utf-8") as fh:
                data = json.load(fh)
        except (OSError, ValueError) as e:
            raise Unreadable("%s: %s" % (p, e))
        cost = data.get("cost") if isinstance(data, dict) else None
        v = cost.get("total_cost_usd") if isinstance(cost, dict) else None
        out.append(float(v) if isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) and v >= 0 else None)
    return out


def dir_cost(d):
    """(label, mode, cost_usd, source, estimated_reason|None) for one results subdirectory."""
    label = os.path.basename(d)
    env = read_env(os.path.join(d, "run-env.txt"))
    names = os.listdir(d)
    if env is None and names == ["SKIPPED.txt"]:
        return label, "", 0.0, "skipped", None
    mode = (env or {}).get("MODE", "")

    def budget():
        raw = (env or {}).get("MAX_BUDGET_USD", "")
        try:
            b = float(raw)
        except ValueError:
            raise Unreadable("%s: no priced result and no readable MAX_BUDGET_USD" % d)
        if not math.isfinite(b) or b < 0:
            raise Unreadable("%s: MAX_BUDGET_USD %r" % (d, raw))
        return b

    stage = stage_total(os.path.join(d, "stage-cost.txt"))
    lc = launcher_costs(d)
    if stage is not None:
        st, covered = stage
        priced = sum(c for c in lc if c is not None)
        if len(lc) <= covered:
            if priced > st:
                return label, mode, priced, "launcher-result", None
            return label, mode, st, "stage-cost", None
        # WR-04: stage-cost.txt predates a relaunch; its total is stale. Unpriced results are charged the budget.
        unpriced = sum(1 for c in lc if c is None)
        cost = priced + (unpriced * budget() if unpriced else 0.0)
        est = "stage-cost covers %d of %d launches" % (covered, len(lc)) if unpriced else None
        return label, mode, max(st, cost), "max(stage-cost,launcher-result)", est
    if lc:
        priced = sum(c for c in lc if c is not None)
        unpriced = sum(1 for c in lc if c is None)
        if unpriced:
            return label, mode, priced + unpriced * budget(), "launcher-result", "%d launcher-result without cost" % unpriced
        return label, mode, priced, "launcher-result", None
    return label, mode, budget(), "estimated", "no stage-cost.txt and no launcher-result"


def measure(results):
    if not os.path.isdir(results):
        raise Unreadable("%s: not a directory" % results)
    rows = []
    for name in sorted(os.listdir(results)):
        d = os.path.join(results, name)
        if os.path.isdir(d):
            rows.append(dir_cost(d))
    all_ = sum(micro(r[2]) for r in rows) / MICRO
    real = sum(micro(r[2]) for r in rows if r[1] in REAL_MODES) / MICRO
    return rows, all_, real


def fmt_n(x):
    return "%g" % x


def report(results, ceiling, nxt, out):
    rows, spent_all, spent_real = measure(results)
    for label, mode, cost, source, est in rows:
        out.write("COST label=%s mode=%s usd=%.4f source=%s\n" % (label, mode or "-", cost, source))
        if est:
            out.write("ESTIMATED label=%s usd=%.4f reason=%s\n" % (label, cost, est))
    over = micro(spent_all) + micro(nxt) > micro(ceiling)
    out.write("SPENT_ALL=%.4f SPENT_REAL=%.4f NEXT=%s CEILING=%s %s\n"
              % (spent_all, spent_real, fmt_n(nxt), fmt_n(ceiling), "OVER" if over else "OK"))
    return 1 if over else 0


def run_cli(argv, out):
    ap = argparse.ArgumentParser(prog="campaign_budget.py", add_help=True)
    ap.add_argument("--results", default=DEFAULT_RESULTS)
    ap.add_argument("--ceiling", type=float, default=60.0)
    ap.add_argument("--next", dest="nxt", type=float, default=0.0)
    a = ap.parse_args(argv)
    for name, v in (("ceiling", a.ceiling), ("next", a.nxt)):
        if not math.isfinite(v) or v < 0:
            sys.stderr.write("campaign_budget: --%s must be a finite number >= 0\n" % name)
            return 2
    try:
        return report(a.results, a.ceiling, a.nxt, out)
    except Unreadable as e:
        sys.stderr.write("campaign_budget: unreadable: %s\n" % e)
        return 2


# ----------------------------------------------------------------------------- selftest
def _w(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write(text)


def _env(mode, budget):
    return "MODE=%s\nMAX_BUDGET_USD=%s\n" % (mode, budget)


def selftest():
    def run(root, *args):
        buf = io.StringIO()
        with contextlib.redirect_stderr(io.StringIO()):
            rc = run_cli(["--results", root] + list(args), buf)
        return rc, buf.getvalue()

    with tempfile.TemporaryDirectory() as td:
        # stage-cost-only real run (also a second TOTAL line: the last one wins)
        _w(os.path.join(td, "a", "run-env.txt"), _env("run1", 15))
        _w(os.path.join(td, "a", "stage-cost.txt"), "TOTAL driver=1 workers=1 total=9.9999 x\nTOTAL driver=0.5 workers=1.0 total=1.5000 driver_share=1%\n")
        # launcher-result-only Haiku run (two launches summed)
        _w(os.path.join(td, "b", "run-env.txt"), _env("smoke1", 1))
        _w(os.path.join(td, "b", "launcher-result-1.json"), json.dumps({"cost": {"total_cost_usd": 0.25}}))
        _w(os.path.join(td, "b", "launcher-result-2.json"), json.dumps({"cost": {"total_cost_usd": 0.25}}))
        # a killed launch with neither: charged its budget, flagged ESTIMATED
        _w(os.path.join(td, "c", "run-env.txt"), _env("run2", 15))
        # SKIPPED-only dir costs 0
        _w(os.path.join(td, "d", "SKIPPED.txt"), "skipped\n")
        # a loose file in the results root is ignored (prereg.txt)
        _w(os.path.join(td, "prereg.txt"), "PREREG_SHA=x\n")
        rc, out = run(td, "--next", "0")
        last = out.strip().splitlines()[-1]
        assert rc == 0 and last == "SPENT_ALL=17.0000 SPENT_REAL=16.5000 NEXT=0 CEILING=60 OK", (rc, last)
        assert "ESTIMATED label=c usd=15.0000" in out and out.count("ESTIMATED") == 1, out
        assert "COST label=d mode=- usd=0.0000 source=skipped" in out, out
        # boundary: SPENT_ALL + NEXT exactly equal to the ceiling is OK, one micro-dollar more is OVER
        rc, out = run(td, "--ceiling", "20", "--next", "3")
        assert rc == 0 and out.strip().endswith("NEXT=3 CEILING=20 OK"), (rc, out)
        rc, out = run(td, "--ceiling", "20", "--next", "3.000001")
        assert rc == 1 and out.strip().endswith("OVER"), (rc, out)
        rc, out = run(td, "--ceiling", "8", "--next", "15")
        assert rc == 1 and out.strip().endswith("CEILING=8 OVER"), (rc, out)
        # a launcher-result without a cost next to a priced one: the unpriced one is charged the budget
        _w(os.path.join(td, "e", "run-env.txt"), _env("run1", 7))
        _w(os.path.join(td, "e", "launcher-result-1.json"), json.dumps({"cost": {"total_cost_usd": 1.0}}))
        _w(os.path.join(td, "e", "launcher-result-2.json"), json.dumps({"rc": 1, "cost": None}))
        rc, out = run(td)
        assert out.strip().splitlines()[-1].startswith("SPENT_ALL=25.0000 SPENT_REAL=24.5000 "), out
        assert "ESTIMATED label=e usd=8.0000" in out, out
        # unreadable inputs fail closed (exit 2)
        _w(os.path.join(td, "f", "launcher-result-1.json"), "{not json")
        rc, _ = run(td)
        assert rc == 2, rc
        os.remove(os.path.join(td, "f", "launcher-result-1.json"))
        os.rmdir(os.path.join(td, "f"))
        os.makedirs(os.path.join(td, "g"))                     # empty dir: no run-env, nothing to price
        rc, _ = run(td)
        assert rc == 2, rc
        os.rmdir(os.path.join(td, "g"))
        rc, _ = run(os.path.join(td, "missing"))
        assert rc == 2, rc
        rc, _ = run(td, "--next", "-1")
        assert rc == 2, rc
        rc, _ = run(td, "--ceiling", "nan")
        assert rc == 2, rc
    # WR-04: a stage-cost.txt is trusted only when its `launches=` count covers every launcher-result
    with tempfile.TemporaryDirectory() as td:
        hdr = "stage_cost: sessions=1 launches=%d priced=yes rates_from=x\n"
        tot = "TOTAL driver=0 workers=0 total=%s driver_share=0%%\n"

        def lr(d, n, usd):
            _w(os.path.join(td, d, "launcher-result-%d.json" % n),
               json.dumps({"cost": {"total_cost_usd": usd} if usd is not None else None}))

        def one(d):
            rc, out = run(td)
            row = [l for l in out.splitlines() if l.startswith("COST label=%s " % d)]
            return rc, row[0] if row else "", out

        # stale and priced: launches=1 covers one of two results, stage 2.0 vs launcher sum 5.0 (old rule: 2.0)
        _w(os.path.join(td, "s1", "run-env.txt"), _env("run2", 15))
        _w(os.path.join(td, "s1", "stage-cost.txt"), hdr % 1 + tot % "2.0000")
        lr("s1", 1, 2.0)
        lr("s1", 2, 3.0)
        rc, row, out = one("s1")
        assert rc == 0 and row == "COST label=s1 mode=run2 usd=5.0000 source=max(stage-cost,launcher-result)", (rc, row)
        assert "ESTIMATED label=s1" not in out, out
        # stale with an unpriced second result: priced 2.0 + budget 15 = 17.0, ESTIMATED
        _w(os.path.join(td, "s2", "run-env.txt"), _env("run2", 15))
        _w(os.path.join(td, "s2", "stage-cost.txt"), hdr % 1 + tot % "2.0000")
        lr("s2", 1, 2.0)
        lr("s2", 2, None)
        rc, row, out = one("s2")
        assert row == "COST label=s2 mode=run2 usd=17.0000 source=max(stage-cost,launcher-result)", row
        assert "ESTIMATED label=s2 usd=17.0000 reason=stage-cost covers 1 of 2 launches" in out, out
        # the l7 shape: launches=1, one unpriced launcher-result, covered by the header: the stage total stands
        _w(os.path.join(td, "s3", "run-env.txt"), _env("l7", 1))
        _w(os.path.join(td, "s3", "stage-cost.txt"), hdr % 1 + tot % "0.0122")
        lr("s3", 1, None)
        rc, row, out = one("s3")
        assert row == "COST label=s3 mode=l7 usd=0.0122 source=stage-cost", row
        assert "ESTIMATED label=s3" not in out, out
        # covered, but the launcher results sum above the stage total: the larger figure is charged
        _w(os.path.join(td, "s4", "run-env.txt"), _env("run1", 15))
        _w(os.path.join(td, "s4", "stage-cost.txt"), hdr % 2 + tot % "1.0000")
        lr("s4", 1, 1.0)
        lr("s4", 2, 4.0)
        rc, row, out = one("s4")
        assert row == "COST label=s4 mode=run1 usd=5.0000 source=launcher-result", row
        # a stage-cost.txt with no `launches=` field covers 0 launches
        _w(os.path.join(td, "s5", "run-env.txt"), _env("run1", 15))
        _w(os.path.join(td, "s5", "stage-cost.txt"), "stage_cost: sessions=1\n" + tot % "1.0000")
        lr("s5", 1, 3.0)
        rc, row, out = one("s5")
        assert row == "COST label=s5 mode=run1 usd=3.0000 source=max(stage-cost,launcher-result)", row
        # an in-flight dir (run-env.txt and one copied priced launcher-result, no stage-cost yet) costs that price
        _w(os.path.join(td, "s6", "run-env.txt"), _env("run2", 15))
        lr("s6", 1, 3.0)
        rc, row, out = one("s6")
        assert row == "COST label=s6 mode=run2 usd=3.0000 source=launcher-result", row
        assert "ESTIMATED label=s6" not in out, out
    print("selftest OK")
    return 0


def main(argv):
    if argv and argv[0] == "selftest":
        return selftest()
    return run_cli(argv, sys.stdout)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
