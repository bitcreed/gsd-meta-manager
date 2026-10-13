#!/usr/bin/env python3
"""Editor-race stress test for the tool-written frontmatter on DIRECTION.md.

A "human" process saves DIRECTION.md N times (each save carries a counter in the body) while a
"tool" process regenerates the frontmatter in a tight loop - thousands of times more often than a
real run (events every few seconds, heartbeats coalesced to 2 min). After each save the human reads
the file back: if, 50 ms later, the body still shows an older counter, the tool silently reverted
that save (a LOST EDIT). Run every strategy x save-style pair.

Usage: race_test.py <scratch-dir> [seconds-per-case]
"""
import multiprocessing as mp
import os
import random
import re
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
import driverctl as dc  # noqa: E402

BODY = "# Driver direction: race\n\n## Direction\n\n- control: run\n- note: human edit {k}\n\n## Items\n\n### I-01 x\n\n- budget: dispatches=1\n"


def human(path, style, secs, q):
    k, reverts, transient, saves = 0, 0, 0, 0
    end = time.time() + secs
    while time.time() < end:
        k += 1
        with open(path) as f:
            fm, _ = dc.split_frontmatter(f.read())   # editor buffer keeps whatever header it loaded
        data = (fm or "") + BODY.format(k=k)
        if style == "rename":                       # vim/emacs/most editors: write tmp, rename
            tmp = path + ".human"
            with open(tmp, "w") as f:
                f.write(data)
            os.replace(tmp, path)
        else:                                       # in-place: open(O_TRUNC) + write (VS Code, nano, echo >)
            with open(path, "w") as f:
                f.write(data)
        saves += 1
        time.sleep(random.uniform(0, 0.002))
        seen = latest(path)
        if seen != k:
            time.sleep(0.05)
            if latest(path) != k:
                reverts += 1
            else:
                transient += 1
    q.put(("human", saves, reverts, transient))


def latest(path):
    try:
        with open(path) as f:
            m = re.search(r"human edit (\d+)", f.read())
        return int(m.group(1)) if m else None
    except FileNotFoundError:
        return None


def tool(path, strategy, secs, q):
    n, res = 0, {}
    end = time.time() + secs
    while time.time() < end:
        n += 1
        fm = f"---\n{dc.FM_MARKER} race\nn: {n}\n---\n"
        r = dc.splice_frontmatter(path, fm, strategy)
        res[r] = res.get(r, 0) + 1
    q.put(("tool", n, res))


def case(d, strategy, style, secs):
    path = os.path.join(d, f"DIRECTION-{strategy}-{style}.md")
    with open(path, "w") as f:
        f.write(BODY.format(k=0))
    q = mp.Queue()
    ps = [mp.Process(target=human, args=(path, style, secs, q)),
          mp.Process(target=tool, args=(path, strategy, secs, q))]
    for p in ps:
        p.start()
    out = dict((r[0], r[1:]) for r in (q.get() for _ in ps))
    for p in ps:
        p.join()
    saves, reverts, transient = out["human"]
    regens, res = out["tool"]
    with open(path) as f:
        final_ok = dc.body_sane(dc.split_frontmatter(f.read())[1])
    print(f"{strategy:8} {style:7} saves={saves:6} regens={regens:7} LOST={reverts:5} "
          f"transient={transient:4} final_body_sane={final_ok} tool={res}")


if __name__ == "__main__":
    d = sys.argv[1]
    secs = float(sys.argv[2]) if len(sys.argv) > 2 else 5
    os.makedirs(d, exist_ok=True)
    for strategy in ("naive", "reread", "exchange"):
        for style in ("rename", "inplace"):
            case(d, strategy, style, secs)
