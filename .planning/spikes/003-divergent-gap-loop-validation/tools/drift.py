#!/usr/bin/env python3
"""Spike 003 moving target: a host-fed title feed that grows by one house-rule class per plan SUMMARY.

The scratch project's verifier criterion tests `feed/titles.txt` against `feed/expected.txt`. `feed/`
is gitignored inside the scratch repo; only this host-side tool writes it. Two triggers move it:

  --trigger summary   (default) generation = number of `*-SUMMARY.md` files in the scratch repo's HEAD
                      (capped at MAX_GEN). The executor's own SUMMARY commit is not its last action, so
                      an executor that stays alive can still see the new rule and absorb it (r261-run2).
  --trigger verifier  (seed v2, D-06) generation = number of distinct gsd-verifier spawns in the run's
                      transcripts under --claude-config (capped at MAX_GEN). A spawn is an `Agent` or
                      `Task` tool_use block whose `input.subagent_type` is `gsd-verifier`, de-duplicated by
                      tool_use id. The feed therefore moves after the executor has ended and before the
                      verifier reads it; SUMMARY commits do not move it. A watcher restart recounts the
                      spawns from the transcripts and logs no duplicate advance.

Either way the feed advances between "execute" and "verify" of every GSD round, and each verification
lists cases the previous one did not.

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

Privacy: from transcripts this tool takes only a tool_use block's `id`, `name`, `input.subagent_type` and
the line `timestamp`. It never writes message text to drift.log, stdout or a fixture.

Usage:
  drift.py emit N --project P
  drift.py watch --project P --log FILE --snapshots DIR [--interval 0.4] [--once]
                 [--trigger summary|verifier] [--claude-config DIR]    (verifier needs --claude-config)
  drift.py extract-spawns CLAUDE_CONFIG       (one skeleton line per Agent/Task tool_use, by timestamp)
  drift.py selftest
"""
from __future__ import annotations

import argparse
import datetime as dt
import glob
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:               # `python3 -I` drops the script directory from sys.path
    sys.path.insert(0, HERE)
import gapcount  # noqa: E402  (WR-03: the one status definition, shared with the fork launcher)

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


# --------------------------------------------------------------------------- verifier spawns (seed v2)

SPAWN_TOOLS = ("Agent", "Task")
VERIFIER_TYPE = "gsd-verifier"
_TS_RE = re.compile(r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,9})?Z$")
_ID_RE = re.compile(r"^[A-Za-z0-9_-]{1,100}$")
_TYPE_RE = re.compile(r"^[A-Za-z0-9._-]{1,80}$")


def _spawn_blocks(lines):
    """(timestamp, tool_use id, tool name, subagent_type) for every Agent/Task tool_use block in `lines`
    (str or bytes JSON lines). Only those four fields are read, each checked against a strict pattern so a
    hostile or odd value can neither reach a log line nor carry message text. Anything else is skipped."""
    for line in lines:
        try:
            d = json.loads(line)
        except ValueError:                                   # includes UnicodeDecodeError on bad bytes
            continue
        if not isinstance(d, dict):
            continue
        ts, msg = d.get("timestamp"), d.get("message")
        content = msg.get("content") if isinstance(msg, dict) else None
        if not isinstance(ts, str) or not _TS_RE.match(ts) or not isinstance(content, list):
            continue
        for b in content:
            if not isinstance(b, dict) or b.get("type") != "tool_use" or b.get("name") not in SPAWN_TOOLS:
                continue
            tid, inp = b.get("id"), b.get("input")
            if not isinstance(tid, str) or not _ID_RE.match(tid) or not isinstance(inp, dict):
                continue
            st = inp.get("subagent_type")
            if isinstance(st, str) and _TYPE_RE.match(st):
                yield ts, tid, b["name"], st


def verifier_spawns(lines):
    """[(timestamp, tool_use_id)] for every gsd-verifier spawn in the transcript lines, in line order."""
    return [(ts, tid) for ts, tid, _, st in _spawn_blocks(lines) if st == VERIFIER_TYPE]


def _transcript_files(cfg: str):
    """Parent session transcripts and every subagent transcript under a claude-config dir (dup_check's layout)."""
    found = set()
    for pat in (os.path.join(cfg, "projects", "*", "*.jsonl"),
                os.path.join(cfg, "projects", "*", "*", "subagents", "**", "agent-*.jsonl")):
        found.update(glob.glob(pat, recursive=True))
    return sorted(found)


class SpawnScanner:
    """Incremental reader of the transcripts: a byte offset per file, only complete lines processed (a partial
    last line waits for its newline), and the set of distinct gsd-verifier tool_use ids seen so far."""

    def __init__(self, cfg: str):
        self.cfg = cfg
        self.offsets = {}
        self.spawns = {}                                      # tool_use id -> timestamp

    def scan(self):
        """-> [(timestamp, id)] of verifier spawns first seen by this call, in timestamp order."""
        new = []
        for path in _transcript_files(self.cfg):
            off = self.offsets.get(path, 0)
            try:
                if os.path.getsize(path) < off:
                    off = 0                                   # truncated or replaced: reread; ids de-duplicate
                with open(path, "rb") as f:
                    f.seek(off)
                    data = f.read()
            except OSError:
                continue
            end = data.rfind(b"\n")
            if end < 0:
                self.offsets[path] = off
                continue
            self.offsets[path] = off + end + 1
            for ts, tid in verifier_spawns(data[:end + 1].split(b"\n")):
                if tid not in self.spawns:
                    self.spawns[tid] = ts
                    new.append((ts, tid))
        return sorted(new)


def extract_spawns(cfg: str):
    """Skeleton lines (strings) for every Agent/Task tool_use block of any subagent type, sorted by timestamp."""
    seen, rows = set(), []
    for path in _transcript_files(cfg):
        try:
            with open(path, "rb") as f:
                lines = f.read().split(b"\n")
        except OSError:
            continue
        for ts, tid, name, st in _spawn_blocks(lines):
            if tid not in seen:
                seen.add(tid)
                rows.append((ts, tid, name, st))
    rows.sort()
    return [json.dumps({"type": "assistant", "timestamp": ts, "message": {"content": [
        {"type": "tool_use", "id": tid, "name": name, "input": {"subagent_type": st}}]}}, separators=(",", ":"))
        for ts, tid, name, st in rows]


class Watcher:
    def __init__(self, project: str, log: str, snapshots: str, trigger: str = "summary", claude_config=None):
        if trigger not in ("summary", "verifier"):
            raise ValueError(f"unknown trigger {trigger!r}")
        if trigger == "verifier" and not claude_config:
            raise ValueError("--trigger verifier requires --claude-config")
        self.project = os.path.abspath(project)
        self.log = log
        self.snapshots = snapshots
        self.trigger = trigger
        self.scanner = SpawnScanner(claude_config) if trigger == "verifier" else None
        self.count = None
        self.gen = 0
        self.seen_hashes = set()

    def _log(self, line: str) -> None:
        with open(self.log, "a") as f:
            f.write(f"{utc()} {line}\n")

    def start(self) -> None:
        os.makedirs(os.path.dirname(os.path.abspath(self.log)), exist_ok=True)
        os.makedirs(self.snapshots, exist_ok=True)
        if self.trigger == "verifier":
            self.scanner.scan()                              # a missing claude-config dir is zero spawns
            self.gen = min(len(self.scanner.spawns), MAX_GEN)
            write_feed(self.project, self.gen)
            self._log(f"start gen={self.gen} trigger=verifier")
            return
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

    def _poll_verifier(self) -> None:
        """Seed v2: each new gsd-verifier spawn raises the generation by one (capped); SUMMARY commits do not."""
        for ts, tid in self.scanner.scan():
            if self.gen >= MAX_GEN:
                self._log(f"exhausted spawns={len(self.scanner.spawns)}")
                continue
            self.gen += 1
            titles = write_feed(self.project, self.gen)
            head = (_git(self.project, "rev-parse", "--short=7", "HEAD") or "").strip() or "none"
            self._log(f"gen={self.gen} summaries={count_summaries(self.project) or 0} head={head} "
                      f"titles={titles} trigger=verifier spawn={tid[-8:]} spawn_at={ts}")

    def poll(self) -> None:
        if self.trigger == "verifier":
            self._poll_verifier()
            self._snapshot_verifications()
            return
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
                status = gapcount.fm_status(raw) or "unknown"      # WR-03: one definition (quote-tolerant)
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
    w = Watcher(a.project, a.log, a.snapshots, a.trigger, a.claude_config)
    w.start()
    w.poll_safe()                                           # IN-05: the first poll is guarded like every later one
    if a.once:
        return 0
    try:
        watch_loop(w, a.interval)
    except KeyboardInterrupt:
        pass
    return 0


# --------------------------------------------------------------------------- selftest

PLANTED = "PLANTED-SECRET-c0ffee"
FIXTURE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "experiment", "replay",
                       "run2-spawn-skeleton.jsonl")
ADV_RE = re.compile(r"^(\S+) gen=(\d+) summaries=(\d+)")     # acceptance.py GEN_RE


def _selftest_verifier(tmp: str, me: str) -> None:
    """Seed v2 end to end on a synthetic claude-config tree, then r261-run2's real spawn timeline."""
    p = os.path.join(tmp, "pv")
    os.makedirs(os.path.join(p, ".planning"))
    sh = lambda *c: subprocess.run(c, cwd=p, check=True, capture_output=True)  # noqa: E731
    sh("git", "init", "-q", "-b", "main")
    sh("git", "config", "user.email", "t@example.invalid")
    sh("git", "config", "user.name", "t")
    with open(os.path.join(p, ".gitignore"), "w") as f:
        f.write("feed/\n")
    sh("git", "add", ".gitignore")
    sh("git", "commit", "-qm", "init")
    cfg = os.path.join(tmp, "cfg")                          # does not exist yet: gen 0, no error
    log, snaps = os.path.join(tmp, "vdrift.log"), os.path.join(tmp, "vsnaps")
    w = Watcher(p, log, snaps, "verifier", cfg)
    w.start()
    w.poll_safe()
    feed = lambda: open(os.path.join(p, "feed", "titles.txt")).read().splitlines()  # noqa: E731
    readlog = lambda: open(log).read()  # noqa: E731
    assert w.gen == 0 and feed() == emit(0)[0] and "start gen=0 trigger=verifier" in readlog()
    assert "poll-error" not in readlog()
    sub = os.path.join(cfg, "projects", "-proj", "sess1", "subagents")
    os.makedirs(sub)
    tp = os.path.join(sub, "agent-aaa.jsonl")

    def spawn(ts, tid, st, name="Agent", prompt=None):
        inp = {"subagent_type": st}
        if prompt:
            inp["prompt"] = prompt
        return json.dumps({"type": "assistant", "timestamp": ts, "message": {"content": [
            {"type": "text", "text": PLANTED}, {"type": "tool_use", "id": tid, "name": name, "input": inp}]}})

    def append(path, text):
        with open(path, "a") as f:
            f.write(text)
    # an executor spawn does not advance
    append(tp, spawn("2026-10-18T02:31:05.029Z", "toolu_exec00000001", "gsd-executor") + "\n")
    w.poll_safe()
    assert w.gen == 0 and feed() == emit(0)[0], "an executor spawn must not advance the feed"
    # a verifier spawn line without its newline waits
    line = spawn("2026-10-18T02:33:31.810Z", "toolu_veri00000001", "gsd-verifier", prompt=PLANTED)
    append(tp, line)
    w.poll_safe()
    assert w.gen == 0 and " trigger=verifier spawn=" not in readlog(), "a partial last line must wait"
    append(tp, "\n")
    w.poll_safe()
    assert w.gen == 1 and feed() == emit(1)[0], "the verifier spawn must advance the feed to gen 1"
    assert " trigger=verifier spawn=00000001 spawn_at=2026-10-18T02:33:31.810Z" in readlog(), readlog()
    # the next VERIFICATION write is snapshotted with gen=1
    vd = os.path.join(p, ".planning", "phases", "01-x")
    os.makedirs(vd)
    with open(os.path.join(vd, "01-VERIFICATION.md"), "w") as f:
        f.write("---\nphase: 01-x\nstatus: gaps_found\n---\nbody\n")
    w.poll_safe()
    assert "status=gaps_found gen=1" in readlog(), readlog()
    # the same tool_use id again (here in a second transcript) is ignored
    tp2 = os.path.join(sub, "agent-bbb.jsonl")
    append(tp2, line + "\n")
    w.poll_safe()
    assert w.gen == 1 and readlog().count(" trigger=verifier spawn=") == 1
    # a SUMMARY commit does not advance in verifier mode
    with open(os.path.join(p, "x-SUMMARY.md"), "w") as f:
        f.write("s\n")
    sh("git", "add", "x-SUMMARY.md")
    sh("git", "commit", "-qm", "summary")
    w.poll_safe()
    assert w.gen == 1 and feed() == emit(1)[0] and readlog().count(" trigger=verifier spawn=") == 1
    # a second verifier spawn: gen 2
    append(tp, spawn("2026-10-18T02:40:00.000Z", "toolu_veri00000002", "gsd-verifier", name="Task") + "\n")
    w.poll_safe()
    assert w.gen == 2 and feed() == emit(2)[0] and "summaries=1 " in readlog()
    # restart on the same tree: start gen=2, no new advance line, no second snapshot
    r = subprocess.run([sys.executable, "-I", me, "watch", "--project", p, "--log", log, "--snapshots", snaps,
                        "--once", "--trigger", "verifier", "--claude-config", cfg], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    lg = readlog()
    assert "start gen=2 trigger=verifier" in lg and lg.count(" trigger=verifier spawn=") == 2, lg
    assert lg.count("snapshot ") == 1, lg
    # past MAX_GEN the feed stops and `exhausted spawns=` is logged
    append(tp, "".join(spawn("2026-10-18T03:00:%02d.000Z" % i, "toolu_veri%08d" % (i + 3), "gsd-verifier") + "\n"
                       for i in range(8)))
    w.poll_safe()
    assert w.gen == MAX_GEN and feed() == emit(MAX_GEN)[0] and "exhausted spawns=10" in readlog()
    # every advance line (summary mode in the first test, verifier mode here) matches acceptance's GEN_RE
    adv = [ln for ln in readlog().splitlines() if " gen=" in ln and " summaries=" in ln]
    assert len(adv) == MAX_GEN and all(ADV_RE.match(ln) for ln in adv), adv
    # privacy: planted message text reaches neither the log, the snapshots nor extract-spawns
    r = subprocess.run([sys.executable, "-I", me, "extract-spawns", cfg], capture_output=True, text=True)
    assert r.returncode == 0 and PLANTED not in r.stdout and PLANTED not in r.stderr and PLANTED not in readlog()
    out = [json.loads(x) for x in r.stdout.splitlines()]
    assert len(out) == 11 and all(set(x) == {"type", "timestamp", "message"} for x in out), out
    assert [x["timestamp"] for x in out] == sorted(x["timestamp"] for x in out)
    for n in os.listdir(snaps):
        assert PLANTED not in open(os.path.join(snaps, n)).read()
    # bad values never reach the log: a hostile id / timestamp is skipped
    assert verifier_spawns([spawn("2026-10-18T02:33:31.810Z", "bad id\nINJECT", "gsd-verifier"),
                            spawn("not a time", "toolu_okokokok", "gsd-verifier"), "not json " + PLANTED,
                            "[]", b"\xff\xfe"]) == []
    # a summary-mode watcher on the same tree ignores the transcripts entirely
    ws = Watcher(p, os.path.join(tmp, "s.log"), os.path.join(tmp, "ssn"))
    ws.start()
    assert ws.gen == 1 and ws.scanner is None
    # r261-run2's real spawn timeline: exactly one verifier spawn, after the executor spawn
    with open(FIXTURE, encoding="utf-8") as f:
        fx = f.read().splitlines()
    assert verifier_spawns(fx) == [("2026-10-18T02:33:31.810Z", "toolu_015GDTpWSNpjHL7mEX31oR1E")], verifier_spawns(fx)
    execs = [ts for ts, _, _, st in _spawn_blocks(fx) if st == "gsd-executor"]
    assert execs == ["2026-10-18T02:31:05.029Z"] and execs[0] < "2026-10-18T02:33:31.810Z", execs
    assert PLANTED not in "".join(fx)


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
        # summary-mode advance lines match acceptance's GEN_RE too, and the start line carries no trigger field
        adv = [ln for ln in lg.splitlines() if " gen=" in ln and " summaries=" in ln]
        assert adv and all(ADV_RE.match(ln) for ln in adv) and "trigger=" not in lg, adv
        _selftest_verifier(tmp, me)
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
    w.add_argument("--trigger", choices=("summary", "verifier"), default="summary")
    w.add_argument("--claude-config")
    x = sp.add_parser("extract-spawns")
    x.add_argument("claude_config")
    sp.add_parser("selftest")
    a = ap.parse_args()
    if a.cmd == "watch" and a.trigger == "verifier" and not a.claude_config:
        ap.error("--trigger verifier requires --claude-config")
    if a.cmd == "emit":
        n = write_feed(a.project, a.n)
        print(f"emitted gen={min(a.n, MAX_GEN)} titles={n}")
        return 0
    if a.cmd == "watch":
        return cmd_watch(a)
    if a.cmd == "extract-spawns":
        for ln in extract_spawns(a.claude_config):
            print(ln)
        return 0
    return selftest()


if __name__ == "__main__":
    sys.exit(main())
