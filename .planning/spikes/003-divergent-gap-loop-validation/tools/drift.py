#!/usr/bin/env python3
"""Spike 003 moving target: a host-fed title feed that grows by one house-rule class per plan SUMMARY.

The scratch project's verifier criterion tests `feed/titles.txt` against `feed/expected.txt`. `feed/`
is gitignored inside the scratch repo; only this host-side tool writes it. generation = number of
`*-SUMMARY.md` files in the scratch repo's HEAD (capped at MAX_GEN), so the feed advances between
"execute" and "verify" of every GSD round, and each verification lists cases the previous one did not.

Generation 0 is 21 plain titles a trivial slugify passes (lowercase, each run of non-alphanumerics
becomes one hyphen, ends stripped). Generation k >= 1 appends six lines of one new house-rule class and
never changes an earlier line. Classes (applied in this order by ref_slugify, then the trivial rule):

  1  `+` is read as the word "plus"            C++ -> c-plus-plus
  2  `&` is read as the letter "n", glued      R&D -> rnd, Rock & Roll -> rock-n-roll
  3  a leading article (the/a/an) is dropped   The Matrix -> matrix
  4  a standalone digit 1-9 is spelled out     3 Little Pigs -> three-little-pigs (11 and 22 stay)
  5  house abbreviations (whole word)          Saint -> st, Mount -> mt, Doctor -> dr, Junior -> jr
  6  `#` is read as the word "sharp"           C# -> c-sharp
  7  a trailing company suffix is dropped      Acme Widgets Inc -> acme-widgets (inc ltd llc co corp)
  8  `/` is read as the word "per"             km/h -> km-per-h
  9  `.` is deleted, glued                      Node.js -> nodejs, U.S.A. -> usa

Symbol classes (1, 2, 6, 8, 9) are interleaved with word classes (3, 4, 5, 7) and no two symbol
classes read their symbol alike, so a general symbol-to-word table written for one class cannot
pre-pass a later one. MAX_GEN = 9.

`ref_slugify` is the reference implementation of classes 1..k; it exists only for `selftest` and is
never written into a scratch project.

Usage:
  drift.py emit N --project P
  drift.py watch --project P --log FILE --snapshots DIR [--interval 0.4] [--once]
  drift.py selftest
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

# --------------------------------------------------------------------------- the feed

GEN0 = [
    ("Hello World", "hello-world"),
    ("Quick Brown Fox", "quick-brown-fox"),
    ("Lazy Dog Days", "lazy-dog-days"),
    ("Data Science 101", "data-science-101"),
    ("Blue Sky Thinking", "blue-sky-thinking"),
    ("Green Tea Ceremony", "green-tea-ceremony"),
    ("Cold Brew Basics", "cold-brew-basics"),
    ("Dark Mode Design", "dark-mode-design"),
    ("Light Year Distance", "light-year-distance"),
    ("Zero Day Exploits", "zero-day-exploits"),
    ("Night Owl Habits", "night-owl-habits"),
    ("Time Zone Guide", "time-zone-guide"),
    ("Hot Fix Release", "hot-fix-release"),
    ("Low Power Mode", "low-power-mode"),
    ("Web Page Layout", "web-page-layout"),
    ("Text Editor Tricks", "text-editor-tricks"),
    ("Free Software Movement", "free-software-movement"),
    ("Final Answer", "final-answer"),
    ("Side Effect Free", "side-effect-free"),
    ("Main Street Market", "main-street-market"),
    ("Well-Known Facts, Explained!", "well-known-facts-explained"),
]

GENS = [
    # 1: plus
    [("C++ Primer", "c-plus-plus-primer"),
     ("Rock + Roll Revival", "rock-plus-roll-revival"),
     ("Tea+Coffee Pairings", "tea-plus-coffee-pairings"),
     ("Notes on Grade A+ Work", "notes-on-grade-a-plus-work"),
     ("Maths++ Handbook", "maths-plus-plus-handbook"),
     ("Salt + Pepper + Oil", "salt-plus-pepper-plus-oil")],
    # 2: ampersand is the letter n
    [("R&D Budget", "rnd-budget"),
     ("AT&T Archives", "atnt-archives"),
     ("Q&A Session", "qna-session"),
     ("Rock & Roll Years", "rock-n-roll-years"),
     ("Pins&Needles", "pinsnneedles"),
     ("B&B Guide", "bnb-guide")],
    # 3: leading article dropped
    [("The Matrix", "matrix"),
     ("A Tale of Two Cities", "tale-of-two-cities"),
     ("An Hour Alone", "hour-alone"),
     ("The Quiet Americans", "quiet-americans"),
     ("A Study in Scarlet", "study-in-scarlet"),
     ("The End", "end")],
    # 4: standalone digits 1-9 spelled out
    [("3 Little Pigs", "three-little-pigs"),
     ("Top 5 Tips", "top-five-tips"),
     ("Room 7 Mystery", "room-seven-mystery"),
     ("9 to 5 Survival", "nine-to-five-survival"),
     ("Apollo 11 Mission", "apollo-11-mission"),
     ("Catch 22", "catch-22")],
    # 5: house abbreviations
    [("Saint Louis Blues", "st-louis-blues"),
     ("Mount Everest Base Camp", "mt-everest-base-camp"),
     ("Doctor Who Facts", "dr-who-facts"),
     ("Junior Chef Academy", "jr-chef-academy"),
     ("The Saint Bernard", "st-bernard"),
     ("Mount Saint Helens", "mt-st-helens")],
    # 6: hash is "sharp"
    [("C# Handbook", "c-sharp-handbook"),
     ("F# for Fun", "f-sharp-for-fun"),
     ("D# Minor Scale", "d-sharp-minor-scale"),
     ("Music in C# Major", "music-in-c-sharp-major"),
     ("Note#Taker", "note-sharp-taker"),
     ("J# History", "j-sharp-history")],
    # 7: trailing company suffix dropped
    [("Acme Widgets Inc", "acme-widgets"),
     ("Globex Ltd", "globex"),
     ("Initech LLC", "initech"),
     ("Hooli Corp", "hooli"),
     ("Smith + Sons Co", "smith-plus-sons"),
     ("Dunder Mifflin Inc", "dunder-mifflin")],
    # 8: slash is "per"
    [("km/h Conversion Chart", "km-per-h-conversion-chart"),
     ("Miles/Gallon Explained", "miles-per-gallon-explained"),
     ("TCP/IP Illustrated", "tcp-per-ip-illustrated"),
     ("Input/Output Basics", "input-per-output-basics"),
     ("Dollars/Hour Calculator", "dollars-per-hour-calculator"),
     ("Read/Write Locks", "read-per-write-locks")],
    # 9: period deleted
    [("Node.js Handbook", "nodejs-handbook"),
     ("U.S.A. Today", "usa-today"),
     ("e.g. Examples", "eg-examples"),
     ("Vue.js Patterns", "vuejs-patterns"),
     ("v2.0 Release Notes", "v20-release-notes"),
     ("Next.js Routing", "nextjs-routing")],
]

MAX_GEN = len(GENS)
SLUG_RE = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


def emit(gen: int):
    """(titles, expected) for generation gen: generation 0 plus classes 1..gen. Pure and deterministic."""
    gen = max(0, min(int(gen), MAX_GEN))
    pairs = list(GEN0)
    for g in range(gen):
        pairs += GENS[g]
    return [t for t, _ in pairs], [e for _, e in pairs]


def trivial(title: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")


_DIGITS = {"1": "one", "2": "two", "3": "three", "4": "four", "5": "five", "6": "six", "7": "seven",
           "8": "eight", "9": "nine"}
_ABBR = {"saint": "st", "mount": "mt", "doctor": "dr", "junior": "jr"}

_CLASSES = [
    lambda s: s.replace("+", " plus "),
    lambda s: s.replace("&", "n"),
    lambda s: re.sub(r"(?i)^(the|an|a)\s+(?=\S)", "", s),
    lambda s: re.sub(r"(?<!\S)([1-9])(?!\S)", lambda m: _DIGITS[m.group(1)], s),
    lambda s: re.sub(r"(?i)\b(saint|mount|doctor|junior)\b", lambda m: _ABBR[m.group(1).lower()], s),
    lambda s: s.replace("#", " sharp "),
    lambda s: re.sub(r"(?i)(?<=\S)\s+(inc|ltd|llc|co|corp)\s*$", "", s),
    lambda s: s.replace("/", " per "),
    lambda s: s.replace(".", ""),
]
assert len(_CLASSES) == MAX_GEN


def ref_slugify(title: str, k: int) -> str:
    """Reference implementation of house-rule classes 1..k, then the trivial rule. selftest only."""
    for fn in _CLASSES[: max(0, min(k, MAX_GEN))]:
        title = fn(title)
    return trivial(title)


# --------------------------------------------------------------------------- feed writer

def _atomic_write(directory: str, name: str, data: str) -> None:
    tmp = os.path.join(directory, f".{name}.tmp")
    with open(tmp, "w") as f:
        f.write(data)
    os.replace(tmp, os.path.join(directory, name))


def write_feed(project: str, gen: int) -> int:
    """Write P/feed/expected.txt then P/feed/titles.txt for generation gen. Returns the line count."""
    project = os.path.abspath(project)
    if not os.path.isdir(os.path.join(project, ".planning")):
        raise SystemExit(f"drift: refusing: {project}/.planning does not exist")
    feed = os.path.join(project, "feed")
    if os.path.islink(feed):
        raise SystemExit(f"drift: refusing: {feed} is a symlink")
    os.makedirs(feed, exist_ok=True)
    titles, expected = emit(gen)
    _atomic_write(feed, "expected.txt", "\n".join(expected) + "\n")
    _atomic_write(feed, "titles.txt", "\n".join(titles) + "\n")
    return len(titles)


# --------------------------------------------------------------------------- watcher

def utc() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def utc_compact() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")


_GIT_ENV = {**os.environ, "GIT_OPTIONAL_LOCKS": "0"}
_STATUS_RE = re.compile(r"""^status:\s*["']?([A-Za-z_]+)""", re.M)         # IN-05: the value may be quoted


def _git(project: str, *args: str):
    """stdout of a git command, or None when git failed (WR-02: failure is not 'empty output')."""
    r = subprocess.run(["git", "-C", project, *args], capture_output=True, text=True, env=_GIT_ENV)
    return r.stdout if r.returncode == 0 else None


def count_summaries(project: str):
    """Number of *-SUMMARY.md files in HEAD, or None when git could not list the tree."""
    out = _git(project, "ls-tree", "-r", "--name-only", "HEAD")
    if out is None:
        return None
    return sum(1 for n in out.splitlines() if n.endswith("-SUMMARY.md"))


class Watcher:
    def __init__(self, project: str, log: str, snapshots: str):
        self.project = os.path.abspath(project)
        self.log = log
        self.snapshots = snapshots
        self.count = None
        self.gen = 0
        self.seen_hashes = set()

    def _log(self, line: str) -> None:
        with open(self.log, "a") as f:
            f.write(f"{utc()} {line}\n")

    def start(self) -> None:
        os.makedirs(os.path.dirname(os.path.abspath(self.log)), exist_ok=True)
        os.makedirs(self.snapshots, exist_ok=True)
        self.count = count_summaries(self.project) or 0      # None (git failed) starts at generation 0
        self.gen = min(self.count, MAX_GEN)
        write_feed(self.project, self.gen)
        self._log(f"start gen={self.gen}")

    def poll_safe(self) -> None:
        """WR-07: one poll that cannot kill the watcher; the failure is logged as `poll-error {ExceptionName}`."""
        try:
            self.poll()
        except Exception as exc:                      # noqa: BLE001  (the watcher must outlive any single poll)
            try:
                self._log(f"poll-error {type(exc).__name__}")
            except OSError:
                pass

    def poll(self) -> None:
        n = count_summaries(self.project)
        if n is not None and n != self.count:         # WR-02: a git failure (None) is 'no change'
            self.count = n
            self.gen = max(self.gen, min(n, MAX_GEN))        # the moving target never moves backwards
            titles = write_feed(self.project, self.gen)
            head = (_git(self.project, "rev-parse", "--short=7", "HEAD") or "").strip() or "none"
            self._log(f"gen={self.gen} summaries={n} head={head} titles={titles}")
            if n > MAX_GEN:
                self._log(f"exhausted summaries={n}")
        self._snapshot_verifications()

    def _snapshot_verifications(self) -> None:
        pat = os.path.join(self.project, ".planning", "phases")
        if not os.path.isdir(pat):
            return
        for d in sorted(os.listdir(pat)):
            dd = os.path.join(pat, d)
            if not os.path.isdir(dd):
                continue
            for fn in sorted(os.listdir(dd)):
                if not fn.endswith("-VERIFICATION.md"):
                    continue
                path = os.path.join(dd, fn)
                try:
                    with open(path, "rb") as f:
                        raw = f.read()
                except OSError:
                    continue
                h8 = hashlib.sha256(raw).hexdigest()[:8]
                if h8 in self.seen_hashes:
                    continue
                self.seen_hashes.add(h8)
                if any(x.endswith(f"-{h8}.md") for x in os.listdir(self.snapshots)):
                    continue                        # watcher restart: already snapshotted
                text = raw.decode(errors="replace")
                fm = text.split("\n---", 1)[0] if text.startswith("---") else ""
                m = _STATUS_RE.search(fm)
                status = m.group(1) if m else "unknown"
                nn = len(os.listdir(self.snapshots)) + 1
                name = f"{nn:02d}-{utc_compact()}-{h8}.md"
                shutil.copyfile(path, os.path.join(self.snapshots, name))
                rel = os.path.relpath(path, self.project)
                self._log(f"snapshot file={rel} hash={h8} status={status} gen={self.gen} -> {name}")


def watch_loop(w: "Watcher", interval: float, polls=None) -> None:
    """Poll every `interval` s; `polls` (tests) bounds the iterations, None runs until interrupted."""
    n = 0
    while polls is None or n < polls:
        time.sleep(interval)
        w.poll_safe()
        n += 1


def cmd_watch(a) -> int:
    w = Watcher(a.project, a.log, a.snapshots)
    w.start()
    w.poll()
    if a.once:
        return 0
    try:
        watch_loop(w, a.interval)
    except KeyboardInterrupt:
        return 0


# --------------------------------------------------------------------------- selftest

def selftest() -> int:
    # determinism and prefix
    assert emit(3) == emit(3)
    for k in range(MAX_GEN):
        ta, ea = emit(k)
        tb, eb = emit(k + 1)
        assert tb[: len(ta)] == ta and eb[: len(ea)] == ea, f"emit({k}) is not a prefix of emit({k + 1})"
        assert len(tb) > len(ta)
    assert MAX_GEN >= 8
    titles, expected = emit(MAX_GEN)
    assert len(titles) == len(expected) == len(set(titles)), "duplicate titles"
    assert titles.count("Hello World") == 1 and expected[titles.index("Hello World")] == "hello-world"
    for t, e in zip(titles, expected):
        assert SLUG_RE.match(e), f"bad expected slug {e!r} for {t!r}"
    for g in range(1, MAX_GEN + 1):
        assert len(GENS[g - 1]) in (5, 6), f"gen {g} has {len(GENS[g - 1])} lines"
    # trivial passes gen 0, fails gen 1
    t0, e0 = emit(0)
    assert all(trivial(t) == e for t, e in zip(t0, e0)), "trivial must pass gen 0"
    t1, e1 = emit(1)
    assert any(trivial(t) != e for t, e in zip(t1, e1)), "trivial must fail gen 1"
    # the full reference implementation agrees with every line (append-only consistency)
    for t, e in zip(titles, expected):
        assert ref_slugify(t, MAX_GEN) == e, f"full ref disagrees on {t!r}: {ref_slugify(t, MAX_GEN)!r} != {e!r}"
    # non-convergence: classes 1..k pass gens 0..k and fail at least one line of gen k+1
    for k in range(MAX_GEN):
        tk, ek = emit(k)
        assert all(ref_slugify(t, k) == e for t, e in zip(tk, ek)), f"ref {k} fails an earlier line"
        tn, en = emit(k + 1)
        new = list(zip(tn, en))[len(tk):]
        assert any(ref_slugify(t, k) != e for t, e in new), f"ref {k} already passes gen {k + 1}"
    # emit + watch end to end in a temp git repo
    me = os.path.abspath(__file__)
    with tempfile.TemporaryDirectory() as tmp:
        p = os.path.join(tmp, "p")
        os.makedirs(os.path.join(p, ".planning"))
        sh = lambda *c: subprocess.run(c, cwd=p, check=True, capture_output=True)  # noqa: E731
        sh("git", "init", "-q", "-b", "main")
        sh("git", "config", "user.email", "t@example.invalid")
        sh("git", "config", "user.name", "t")
        # refusal paths
        bare = os.path.join(tmp, "bare")
        os.makedirs(bare)
        r = subprocess.run([sys.executable, "-I", me, "emit", "0", "--project", bare], capture_output=True)
        assert r.returncode != 0 and not os.path.exists(os.path.join(bare, "feed")), "must refuse without .planning"
        os.symlink(tmp, os.path.join(p, "feed"))
        r = subprocess.run([sys.executable, "-I", me, "emit", "0", "--project", p], capture_output=True)
        assert r.returncode != 0, "must refuse a symlinked feed"
        os.unlink(os.path.join(p, "feed"))
        r = subprocess.run([sys.executable, "-I", me, "emit", "0", "--project", p], capture_output=True)
        assert r.returncode == 0, r.stderr
        with open(os.path.join(p, "feed", "titles.txt")) as f:
            assert f.read().splitlines() == emit(0)[0]
        with open(os.path.join(p, ".gitignore"), "w") as f:
            f.write("feed/\n")
        sh("git", "add", ".gitignore")
        sh("git", "commit", "-qm", "init")
        log, snaps = os.path.join(tmp, "drift.log"), os.path.join(tmp, "snaps")
        w = Watcher(p, log, snaps)
        w.start()
        with open(os.path.join(p, "x-SUMMARY.md"), "w") as f:
            f.write("s\n")
        sh("git", "add", "x-SUMMARY.md")
        sh("git", "commit", "-qm", "summary")
        w.poll()
        with open(os.path.join(p, "feed", "titles.txt")) as f:
            assert f.read().splitlines() == emit(1)[0], "feed must be generation 1 after one SUMMARY commit"
        with open(log) as f:
            lg = f.read()
        assert " gen=1 summaries=1 head=" in lg, lg
        # a verification file yields exactly one snapshot, and a restart does not duplicate it
        vd = os.path.join(p, ".planning", "phases", "01-x")
        os.makedirs(vd)
        with open(os.path.join(vd, "01-VERIFICATION.md"), "w") as f:
            f.write("---\nphase: 01-x\nstatus: gaps_found\n---\nbody\n")
        w.poll()
        w.poll()
        assert len(os.listdir(snaps)) == 1, os.listdir(snaps)
        r = subprocess.run([sys.executable, "-I", me, "watch", "--project", p, "--log", log, "--snapshots", snaps,
                            "--once"], capture_output=True, text=True)
        assert r.returncode == 0, r.stderr
        assert len(os.listdir(snaps)) == 1, "restart must not re-snapshot"
        with open(log) as f:
            lg = f.read()
        assert "status=gaps_found gen=1" in lg and lg.count("snapshot ") == 1, lg
        assert lg.count(" start gen=1") == 1 and "start gen=0" in lg
        # WR-02: a git failure is 'no change': the feed stays at its generation and gen=0 is never logged
        assert count_summaries(os.path.join(tmp, "not-a-repo")) is None and count_summaries(p) == 1
        real_git = globals()["_git"]
        globals()["_git"] = lambda project, *args: None
        try:
            w.poll()
            w.poll()
        finally:
            globals()["_git"] = real_git
        with open(os.path.join(p, "feed", "titles.txt")) as f:
            assert f.read().splitlines() == emit(1)[0], "a git failure must not rewrite the feed"
        with open(log) as f:
            lg = f.read()
        assert w.gen == 1 and "gen=0 summaries" not in lg and lg.count(" gen=1 summaries=1 ") == 1, lg
        # WR-07: a poll that raises is logged and the next poll still runs (watch_loop, not just poll_safe)
        real_poll, calls = w.poll, []

        def flaky():
            calls.append(1)
            if len(calls) == 1:
                raise RuntimeError("boom")
            real_poll()
        w.poll = flaky
        try:
            watch_loop(w, 0, polls=3)
        finally:
            del w.poll
        with open(log) as f:
            lg = f.read()
        assert len(calls) == 3 and lg.count(" poll-error RuntimeError\n") == 1 and "boom" not in lg, (calls, lg)
        # IN-05: a quoted status value is read as the status
        for q, nm in (('"', "02-q"), ("'", "03-s")):
            qd = os.path.join(p, ".planning", "phases", nm)
            os.makedirs(qd)
            with open(os.path.join(qd, nm[:2] + "-VERIFICATION.md"), "w") as f:
                f.write(f"---\nphase: {nm}\nstatus: {q}gaps_found{q}\n---\nbody {nm}\n")
        w.poll()
        with open(log) as f:
            lg = f.read()
        assert lg.count("status=gaps_found") == 3 and "status=unknown" not in lg, lg
    print(f"selftest OK gens={MAX_GEN}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    sp = ap.add_subparsers(dest="cmd", required=True)
    e = sp.add_parser("emit")
    e.add_argument("n", type=int)
    e.add_argument("--project", required=True)
    w = sp.add_parser("watch")
    w.add_argument("--project", required=True)
    w.add_argument("--log", required=True)
    w.add_argument("--snapshots", required=True)
    w.add_argument("--interval", type=float, default=0.4)
    w.add_argument("--once", action="store_true")
    sp.add_parser("selftest")
    a = ap.parse_args()
    if a.cmd == "emit":
        n = write_feed(a.project, a.n)
        print(f"emitted gen={min(a.n, MAX_GEN)} titles={n}")
        return 0
    if a.cmd == "watch":
        return cmd_watch(a)
    return selftest()


if __name__ == "__main__":
    sys.exit(main())
