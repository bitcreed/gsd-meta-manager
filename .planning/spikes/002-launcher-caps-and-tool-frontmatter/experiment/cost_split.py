#!/usr/bin/env python3
"""Split one launch's cost into driver vs workers.

The `result` event's modelUsage is per MODEL, and under the `balanced` profile the GSD planner runs
on Opus like the driver, so per-model totals cannot separate them. Instead: sum token usage per
transcript file (main session = driver; every subagent transcript = worker side), then price it
with a per-model rate solved from modelUsage, assuming Anthropic's standard price ratios
(output 5x input, cache read 0.1x, cache write 1.25x).

Usage: cost_split.py <claude-config-dir> <launch-N.jsonl>
"""
import glob
import json
import os
import sys
from collections import defaultdict

W = {"in": 1.0, "out": 5.0, "cr": 0.1, "cw": 1.25}


def weighted(u):
    return sum(W[k] * u.get(k, 0) for k in W)


def usage_by_model(path):
    acc = defaultdict(lambda: defaultdict(int))
    last = {}                      # message id -> (model, usage); the LAST line carries final usage
    for line in open(path):
        try:
            d = json.loads(line)
        except ValueError:
            continue
        if d.get("type") != "assistant":
            continue
        m = d.get("message") or {}
        last[m.get("id")] = (m.get("model", "?"), m.get("usage") or {})
    for model, u in last.values():
        a = acc[model]
        a["in"] += u.get("input_tokens", 0)
        a["out"] += u.get("output_tokens", 0)
        a["cr"] += u.get("cache_read_input_tokens", 0)
        a["cw"] += u.get("cache_creation_input_tokens", 0)
    return acc


def main(cfg, stream):
    result = None
    for line in open(stream):
        if line.startswith("{"):
            d = json.loads(line)
            if d.get("type") == "result":
                result = d
    if not result:
        print("no result event (killed launch?) - cost unknown from stream")
        return
    sid = result["session_id"]
    main_tx = glob.glob(os.path.join(cfg, "projects", "*", f"{sid}.jsonl"))[0]
    subs = glob.glob(os.path.join(cfg, "projects", "*", sid, "subagents", "**", "*.jsonl"), recursive=True)
    rate = {}
    for model, u in result["modelUsage"].items():
        wt = weighted({"in": u["inputTokens"], "out": u["outputTokens"], "cr": u["cacheReadInputTokens"],
                       "cw": u["cacheCreationInputTokens"]})
        rate[model] = u["costUSD"] / wt if wt else 0
    rows = []
    for label, files in (("driver", [main_tx]), ("workers", subs)):
        tot = defaultdict(lambda: defaultdict(int))
        for f in files:
            for m, u in usage_by_model(f).items():
                for k, v in u.items():
                    tot[m][k] += v
        for m, u in tot.items():
            rm = next((r for k, r in rate.items() if k.startswith(m) or m.startswith(k)), 0)
            rows.append((label, m, dict(u), round(rm * weighted(u), 3)))
    print(f"session {sid}  total_cost_usd={result['total_cost_usd']:.3f}  turns={result.get('num_turns')}  "
          f"duration_s={round(result.get('duration_ms', 0) / 1000)}  subagent_transcripts={len(subs)}")
    for label in ("driver", "workers"):
        sub = [r for r in rows if r[0] == label]
        print(f"  {label:8} est ${sum(r[3] for r in sub):.3f}")
        for _, m, u, c in sub:
            print(f"     {m:28} ${c:7.3f}  in={u['in']} out={u['out']} cache_read={u['cr']} cache_write={u['cw']}")


if __name__ == "__main__":
    main(*sys.argv[1:3])
