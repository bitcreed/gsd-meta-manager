#!/usr/bin/env python3
"""Spike 003 (Phase 26.1) D-07: measured spike-003 spend against the campaign ceiling.

The $60 spike-003 ceiling used to be a README procedure. run.sh calls this tool before every launch
(any mode, any relaunch) and refuses with exit 8 when the measured total plus the launch's
`--max-budget-usd` would exceed the ceiling.

Cost of one results subdirectory (first rule that applies):
  1. a dir holding only SKIPPED.txt costs 0 (a documented skipped run);
  2. `stage-cost.txt` with a `TOTAL ... total=X` line: X (the last TOTAL line);
  3. else the sum of `cost.total_cost_usd` over `launcher-result-*.json`; a launcher-result without a
     numeric cost (a launch killed before its `result` event) is charged the dir's `MAX_BUDGET_USD`;
  4. else (a launch with neither) its `MAX_BUDGET_USD` from run-env.txt.
Rules 3 (unpriced part) and 4 are printed as `ESTIMATED label=<l> usd=<c> reason=<why>`: the worst case,
never a guess below it. Haiku launches count toward SPENT_ALL; only `MODE=run1|run2` launches count toward
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
    return total


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

    st = stage_total(os.path.join(d, "stage-cost.txt"))
    if st is not None:
        return label, mode, st, "stage-cost", None
    lc = launcher_costs(d)
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
    print("selftest OK")
    return 0


def main(argv):
    if argv and argv[0] == "selftest":
        return selftest()
    return run_cli(argv, sys.stdout)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
