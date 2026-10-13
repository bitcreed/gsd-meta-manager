#!/usr/bin/env python3
"""driverctl - the single writer of a driver run's state, log and frontmatter (spike 002 prototype).

Two halves in one file:

* **Server** (`RunStore`, `serve()`): runs OUTSIDE the sandbox, inside the launcher process. It owns
  `state.json`, appends to `LOG.md`, and regenerates the tool-written frontmatter at the top of
  `DIRECTION.md`. Nothing else writes those (the run dir is bind-mounted read-only into the sandbox).
* **Client** (`main()`): what the driver (and the Claude hooks) run INSIDE the sandbox. It validates
  arguments, sends one JSON request over a unix socket, and prints the reply. It never touches files.

    driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 [--phase 1]
    driverctl log report     --item I-01 --status done   < report.txt     (report body on stdin)
    driverctl log verified   --item I-01 --done-when not-met --evidence "VERIFICATION status=gaps_found"
    driverctl log status     --item I-01 --state parked --rule gap-rounds --outcome "..."
    driverctl log needs-human --item I-01 --question "..."
    driverctl log recovered  --item I-01 --action reset --reason "..."
    driverctl log reconcile  --found 0
    driverctl log start      --mode headless --loop-mode loop
    driverctl log end        --reason all-parked
    driverctl show           (prints the current frontmatter; read-only)
    driverctl hook           (Claude hook entry point; hook JSON on stdin)

Python 3.10+, stdlib only.
"""
from __future__ import annotations

import argparse
import ctypes
import datetime as dt
import hashlib
import json
import os
import re
import socket
import subprocess
import sys
import threading
import time

VERSION = "0.2-spike"
SCHEMA_VERSION = 1
STATE_ENUM = ("pending", "running", "done", "parked", "failed")
RUN_STATUS_ENUM = ("starting", "running", "paused", "parked", "stopped", "done", "crashed")
RULE_ENUM = ("budget", "gap-rounds", "divergence", "no-progress", "needs-human", "scope",
             "run-budget", "launcher-cap")
END_REASONS = {"all-done": "done", "all-parked": "parked", "pause": "paused", "stop": "stopped",
               "run-budget": "stopped", "fresh-exit": "running"}
HEARTBEAT_STALE_AFTER_S = 300      # readers: running + heartbeat older than this = stale
HEARTBEAT_REGEN_MIN_S = 120        # heartbeat-only frontmatter rewrites are coalesced to this
MAX_REPORT_LINES = 15
ITEM_RE = re.compile(r"^### (I-\d+)\b", re.M)
FM_MARKER = "generated_by: driverctl"


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_ts(s: str) -> float:
    return dt.datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=dt.timezone.utc).timestamp()


class Invalid(Exception):
    pass


def one_line(name: str, v, limit=200) -> str:
    if v is None or str(v).strip() == "":
        raise Invalid(f"--{name} is required")
    v = str(v)
    if "\n" in v or "|" in v or len(v) > limit:
        raise Invalid(f"--{name} must be one line, no '|', <= {limit} chars")
    return v.strip()


# ---------------------------------------------------------------- DIRECTION.md parsing (read-only)

def split_frontmatter(text: str):
    """(frontmatter_block_or_None, body). Only a leading block carrying our marker counts as ours."""
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1 and FM_MARKER in text[:end]:
            return text[: end + 5], text[end + 5:]
    return None, text


def parse_direction(body: str) -> dict:
    out = {"control": None, "items": {}}
    m = re.search(r"^- control:\s*(\w+)", body, re.M)
    out["control"] = m.group(1) if m else None
    for m in ITEM_RE.finditer(body):
        iid = m.group(1)
        seg = body[m.end(): (ITEM_RE.search(body, m.end()) or re.search(r"\Z", body)).start()]
        b = re.search(r"^- budget:\s*(.+)$", seg, re.M)
        budget = {}
        if b:
            for kv in b.group(1).split(","):
                if "=" in kv:
                    k, v = kv.split("=", 1)
                    try:
                        budget[k.strip()] = int(v.strip())
                    except ValueError:
                        pass
        out["items"][iid] = {"budget": budget}
    return out


# ---------------------------------------------------------------- atomic writes

_libc = ctypes.CDLL(None, use_errno=True)
AT_FDCWD = -100
RENAME_EXCHANGE = 2


def exchange(a: str, b: str) -> None:
    if _libc.renameat2(AT_FDCWD, a.encode(), AT_FDCWD, b.encode(), RENAME_EXCHANGE) != 0:
        e = ctypes.get_errno()
        raise OSError(e, os.strerror(e), b)


def atomic_write(path: str, data: str) -> None:
    tmp = f"{path}.tmp.{os.getpid()}.{threading.get_ident()}"
    with open(tmp, "w") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, path)


def body_sane(body: str) -> bool:
    # An in-place (truncate-then-write) editor save can be observed half done. A DIRECTION body
    # without its two section headers is never written back.
    return "## Direction" in body and "## Items" in body


def splice_frontmatter(path: str, fm: str, strategy: str = "exchange", tries: int = 5) -> str:
    """Replace only the leading tool block of `path`, preserving the body byte-for-byte.

    strategy=exchange (default): renameat2(RENAME_EXCHANGE) puts our file in place AND hands back the
    inode we displaced. If its body is not the one we built on, a human save landed in the race
    window (a rename-save replaced the file, or an in-place save truncated/rewrote it): we exchange
    BACK, which restores the human's inode - including any in-place write still in flight into it -
    and retry. A human's bytes are never unlinked unseen.
    strategy=reread: re-read just before a plain rename (small window, can lose an edit).
    strategy=naive: read once, rename (the obvious bug, kept for the race test).
    Returns "ok" | "skipped-insane-body" | "gave-up".
    """
    for attempt in range(tries):
        if attempt:
            time.sleep(0.005 * attempt)
        with open(path) as f:
            _, body = split_frontmatter(f.read())
        if not body_sane(body):
            return "skipped-insane-body"
        tmp = f"{path}.fm.{os.getpid()}.{threading.get_ident()}"
        with open(tmp, "w") as f:
            f.write(fm + body)
        try:
            os.chmod(tmp, os.stat(path).st_mode & 0o7777)
        except OSError:
            pass
        if strategy == "naive":
            os.replace(tmp, path)
            return "ok"
        if strategy == "reread":
            with open(path) as f:
                _, body2 = split_frontmatter(f.read())
            if body2 != body:
                os.unlink(tmp)
                continue
            os.replace(tmp, path)
            return "ok"
        exchange(tmp, path)                      # tmp now holds the inode that was at `path`
        with open(tmp) as f:
            _, displaced = split_frontmatter(f.read())
        if displaced == body:
            os.unlink(tmp)
            return "ok"
        exchange(tmp, path)                      # a human save raced us: give their inode back
        with open(tmp) as f:
            ours = f.read() == fm + body
        if ours:
            os.unlink(tmp)                       # our stale file: drop it and try again
        else:                                    # an even newer rename-save replaced ours in the
            os.replace(tmp, path)                # meantime: newest human bytes win, then retry
    return "gave-up"


# ---------------------------------------------------------------- YAML emission (no PyYAML needed)

def y(v) -> str:
    if v is None:
        return "null"
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    return json.dumps(str(v))   # a JSON string is a valid YAML double-quoted scalar


def flow(d: dict) -> str:
    return "{" + ", ".join(f"{k}: {y(v)}" for k, v in d.items()) + "}"


# ---------------------------------------------------------------- the store (server side)

class RunStore:
    def __init__(self, run_dir: str, project_root: str, fm_strategy: str = "exchange"):
        self.run_dir = run_dir
        self.project_root = project_root
        self.lock = threading.RLock()
        self.state_path = os.path.join(run_dir, "state.json")
        self.log_path = os.path.join(run_dir, "LOG.md")
        self.dir_path = os.path.join(run_dir, "DIRECTION.md")
        self.fm_strategy = fm_strategy
        self.last_regen = 0.0
        self.gate = None          # callable(payload) -> (allow, reason); set by the launcher
        if os.path.exists(self.state_path):
            with open(self.state_path) as f:
                self.s = json.load(f)
        else:
            self.s = {
                "schema": SCHEMA_VERSION, "run_id": os.path.basename(run_dir.rstrip("/")),
                "project_root": project_root, "status": "starting", "status_reason": None,
                "created_at": now(), "updated_at": now(), "launches": 0,
                "current": {"item": None, "phase": None, "step": None},
                "items": {}, "connections": [],
                "caps": {"dispatches": {"used": 0, "limit": None},
                         "wall_clock_min": {"used": 0, "limit": None},
                         "gap_rounds": {"limit": None, "per_phase": {}, "seen": {}}},
                "hook_dispatches": 0, "denied_dispatches": 0, "last_event": None, "cost": [],
            }

    # -- persistence
    def save(self, regen=True, heartbeat_only=False):
        self.s["updated_at"] = now()
        atomic_write(self.state_path, json.dumps(self.s, indent=1))
        if regen:
            if heartbeat_only and time.time() - self.last_regen < HEARTBEAT_REGEN_MIN_S:
                return
            self.regen()

    def direction(self):
        with open(self.dir_path) as f:
            _, body = split_frontmatter(f.read())
        return parse_direction(body)

    def append_log(self, item: str, event: str, kv: dict, detail: list[str] | None = None):
        head = f"- {now()} | {item} | {event}"
        for k, v in kv.items():
            if v is not None:
                head += f" | {k}={v}"
        data = head + "\n" + "".join(f"    {l}\n" for l in (detail or []))
        fd = os.open(self.log_path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
        try:
            os.write(fd, data.encode())   # one O_APPEND write: readers never see a torn event
        finally:
            os.close(fd)
        summary = " ".join(f"{k}={v}" for k, v in kv.items() if v is not None)[:160]
        self.s["last_event"] = {"at": now(), "item": item, "event": event, "summary": summary}

    def item(self, iid):
        return self.s["items"].setdefault(iid, {"state": "pending", "outcome": None, "dispatches": 0,
                                                "first_dispatch_at": None, "findings": []})

    # -- frontmatter
    def frontmatter(self) -> str:
        s = self.s
        try:
            d = self.direction()
        except OSError:
            d = {"control": None, "items": {}}
        ids = list(d["items"].keys()) or list(s["items"].keys())
        counts = {k: 0 for k in STATE_ENUM}
        for iid in ids:
            counts[s["items"].get(iid, {}).get("state", "pending")] += 1
        caps = s["caps"]
        gr = caps["gap_rounds"]
        worst_phase, worst = None, 0
        for ph, n in gr["per_phase"].items():
            if n >= worst:
                worst_phase, worst = ph, n
        lines = [
            "---",
            f"{FM_MARKER} {VERSION}   # TOOL-WRITTEN. Do not edit: regenerated on every run event.",
            f"driver_frontmatter_version: {SCHEMA_VERSION}",
            f"run_id: {y(s['run_id'])}",
            f"status: {s['status']}",
            f"status_reason: {y(s['status_reason'])}",
            f"control: {y(d['control'])}",
            f"current_item: {y(s['current']['item'])}",
            f"current_phase: {y(s['current']['phase'])}",
            f"current_step: {y(s['current']['step'])}",
            "items: " + flow({"total": len(ids), **counts}),
            "caps:",
            "  dispatches: " + flow(caps["dispatches"]),
            "  wall_clock_min: " + flow(caps["wall_clock_min"]),
            "  gap_rounds: " + flow({"used": worst, "limit": gr["limit"], "phase": worst_phase}),
            f"dispatch_audit: " + flow({"logged": sum(i.get('dispatches', 0) for i in s['items'].values()),
                                        "observed": s["hook_dispatches"], "denied": s["denied_dispatches"]}),
            f"launches: {s['launches']}",
            "connected:" + ("" if s["connections"] else " []"),
        ]
        for c in s["connections"]:
            lines.append("  - " + flow({k: c.get(k) for k in (
                "role", "state", "session_id", "model", "mode", "pid", "scope_unit", "host",
                "started_at", "last_heartbeat")}))
        le = s["last_event"] or {}
        lines += [
            f"heartbeat_stale_after_s: {HEARTBEAT_STALE_AFTER_S}",
            "last_event: " + flow({"at": le.get("at"), "item": le.get("item"), "event": le.get("event"),
                                   "summary": le.get("summary")}),
            f"updated_at: {y(s['updated_at'])}",
            "---",
            "",
        ]
        return "\n".join(lines)

    def regen(self):
        if not os.path.exists(self.dir_path):
            return "no-direction"
        r = splice_frontmatter(self.dir_path, self.frontmatter(), self.fm_strategy)
        self.last_regen = time.time()
        return r

    # -- request handling
    def handle(self, req: dict) -> dict:
        with self.lock:
            op = req.get("op")
            if op == "log":
                return self.op_log(req)
            if op == "hook":
                return self.op_hook(req.get("payload") or {})
            if op == "show":
                return {"ok": True, "frontmatter": self.frontmatter()}
            raise Invalid(f"unknown op {op!r}")

    def known_item(self, iid):
        if iid == "RUN":
            return iid
        if not iid or iid not in self.direction()["items"]:
            raise Invalid(f"--item {iid!r} is not an item in DIRECTION.md")
        return iid

    def op_log(self, r: dict) -> dict:
        ev = r.get("event")
        a = r.get("args") or {}
        if ev == "start":
            mode = a.get("mode")
            if mode not in ("interactive", "headless"):
                raise Invalid("--mode must be interactive|headless")
            self.append_log("RUN", "start", {"mode": mode, "loop_mode": a.get("loop_mode"),
                                             "session": a.get("session")})
        elif ev == "reconcile":
            self.append_log("RUN", "reconcile", {"found": int(a.get("found", 0))})
        elif ev == "dispatched":
            iid = self.known_item(a.get("item"))
            step = one_line("step", a.get("step"))
            rnd = int(a.get("round", 0))
            it = self.item(iid)
            it["dispatches"] += 1
            it["first_dispatch_at"] = it["first_dispatch_at"] or now()
            if it["state"] == "pending":
                it["state"] = "running"
            budget = self.direction()["items"][iid]["budget"].get("dispatches")
            base = subprocess.run(["git", "-C", self.project_root, "rev-parse", "--short", "HEAD"],
                                  capture_output=True, text=True).stdout.strip() or None
            self.s["current"] = {"item": iid, "phase": a.get("phase"), "step": step}
            self.append_log(iid, "dispatched", {"step": step, "base": base, "round": rnd,
                                                "dispatch": f"{it['dispatches']}/{budget}",
                                                "phase": a.get("phase")})
        elif ev == "report":
            iid = self.known_item(a.get("item"))
            st = a.get("status")
            if st not in ("done", "partial", "blocked", "failed"):
                raise Invalid("--status must be done|partial|blocked|failed")
            body = (a.get("body") or "").rstrip("\n").split("\n")
            if not body or body == [""]:
                raise Invalid("report body (stdin) is empty; pipe the worker report verbatim")
            trunc = len(body) > MAX_REPORT_LINES
            body = body[:MAX_REPORT_LINES]
            m = next((re.match(r"FINDINGS:\s*(\d+)", l) for l in body if l.startswith("FINDINGS:")), None)
            if m:
                self.item(iid)["findings"].append(int(m.group(1)))
            self.append_log(iid, "report", {"status": st, "truncated": "yes" if trunc else None}, body)
        elif ev == "verified":
            iid = self.known_item(a.get("item"))
            dw = a.get("done_when")
            if dw not in ("met", "not-met"):
                raise Invalid("--done-when must be met|not-met")
            self.append_log(iid, "verified", {"done-when": dw, "evidence": one_line("evidence", a.get("evidence"))})
        elif ev == "status":
            iid = self.known_item(a.get("item"))
            state = a.get("state")
            if state not in STATE_ENUM:
                raise Invalid(f"--state must be one of {'|'.join(STATE_ENUM)}")
            rule = a.get("rule")
            if state == "parked" and rule not in RULE_ENUM:
                raise Invalid(f"--rule is required for parked and must be one of {'|'.join(RULE_ENUM)}")
            outcome = one_line("outcome", a.get("outcome")) if state != "running" else a.get("outcome")
            it = self.item(iid)
            it["state"], it["outcome"] = state, outcome
            self.append_log(iid, "status", {"state": state, "rule": rule, "outcome": outcome})
        elif ev == "needs-human":
            iid = self.known_item(a.get("item"))
            self.append_log(iid, "needs-human", {"question": one_line("question", a.get("question"), 300)})
        elif ev == "recovered":
            iid = self.known_item(a.get("item"))
            act = a.get("action")
            if act not in ("done", "resume", "reset", "park"):
                raise Invalid("--action must be done|resume|reset|park")
            self.append_log(iid, "recovered", {"action": act, "reason": one_line("reason", a.get("reason"))})
        elif ev == "end":
            reason = a.get("reason")
            if reason not in END_REASONS:
                raise Invalid(f"--reason must be one of {'|'.join(END_REASONS)}")
            self.s["status"], self.s["status_reason"] = END_REASONS[reason], f"driver:{reason}"
            self.s["ended_by_driver"] = True
            self.append_log("RUN", "end", {"reason": reason})
        else:
            raise Invalid(f"unknown event {ev!r}")
        self.save()
        return {"ok": True}

    def driver_conn(self):
        for c in reversed(self.s["connections"]):
            if c.get("role") == "driver" and c.get("state") == "connected":
                return c
        return None

    def op_hook(self, p: dict) -> dict:
        ev = p.get("hook_event_name")
        c = self.driver_conn()
        sub = p.get("agent_id") is not None       # nested (subagent) context
        if c is not None:
            c["last_heartbeat"] = now()
        if ev == "SessionStart" and c is not None:
            c["session_id"] = p.get("session_id")
            self.append_log("RUN", "connect", {"session": p.get("session_id"), "source": p.get("source")})
            self.save()
            return {"ok": True}
        if ev == "SessionEnd":
            if c is not None:
                c["state"] = "disconnected"
            self.append_log("RUN", "disconnect", {"session": p.get("session_id"), "reason": p.get("reason")})
            self.save()
            return {"ok": True}
        if ev == "PreToolUse" and p.get("tool_name") in ("Agent", "Task") and not sub:
            allow, reason = self.gate(p) if self.gate else (True, None)
            if allow:
                self.s["hook_dispatches"] += 1
                self.s["caps"]["dispatches"]["used"] = self.s["hook_dispatches"]
                self.save()
                return {"ok": True}
            self.s["denied_dispatches"] += 1
            self.append_log("RUN", "dispatch-denied", {"reason": reason})
            self.save()
            return {"ok": True, "deny": reason}
        self.save(heartbeat_only=True)
        return {"ok": True}


def serve(store: RunStore, sock_path: str, stop: threading.Event):
    if os.path.exists(sock_path):
        os.unlink(sock_path)
    srv = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    srv.bind(sock_path)
    os.chmod(sock_path, 0o600)
    srv.listen(16)
    srv.settimeout(0.5)
    while not stop.is_set():
        try:
            conn, _ = srv.accept()
        except socket.timeout:
            continue
        with conn:
            conn.settimeout(10)
            buf = b""
            while not buf.endswith(b"\n"):
                chunk = conn.recv(65536)
                if not chunk:
                    break
                buf += chunk
            try:
                resp = store.handle(json.loads(buf))
            except Invalid as e:
                resp = {"ok": False, "error": str(e)}
            except Exception as e:  # noqa: BLE001 - report, never kill the server
                resp = {"ok": False, "error": f"internal: {e!r}"}
            conn.sendall((json.dumps(resp) + "\n").encode())
    srv.close()
    try:
        os.unlink(sock_path)
    except OSError:
        pass


# ---------------------------------------------------------------- client side

def request(req: dict) -> dict:
    path = os.environ.get("DRIVERCTL_SOCK")
    if not path:
        raise SystemExit("driverctl: DRIVERCTL_SOCK is not set (are you running under the launcher?)")
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    s.settimeout(15)
    s.connect(path)
    s.sendall((json.dumps(req) + "\n").encode())
    buf = b""
    while not buf.endswith(b"\n"):
        chunk = s.recv(65536)
        if not chunk:
            break
        buf += chunk
    return json.loads(buf)


def hook_main() -> int:
    raw = sys.stdin.read()
    try:
        p = json.loads(raw)
    except ValueError:
        return 0
    is_gate = p.get("hook_event_name") == "PreToolUse" and p.get("tool_name") in ("Agent", "Task") \
        and p.get("agent_id") is None
    try:
        r = request({"op": "hook", "payload": {k: p.get(k) for k in (
            "hook_event_name", "session_id", "agent_id", "agent_type", "tool_name", "source", "reason")}
            | {"prompt": ((p.get("tool_input") or {}).get("prompt") or "")[:4000]}})
    except OSError as e:
        if is_gate:   # fail closed: no launcher, no dispatch
            r = {"deny": f"driverctl unreachable ({e}); launcher-side caps cannot be checked"}
        else:
            return 0
    if r.get("deny"):
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                                 "permissionDecision": "deny",
                                                 "permissionDecisionReason": "LAUNCHER CAP: " + r["deny"]}}))
    return 0


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(prog="driverctl")
    sp = ap.add_subparsers(dest="cmd", required=True)
    lg = sp.add_parser("log")
    lg.add_argument("event")
    for f in ("item", "step", "round", "phase", "status", "done-when", "evidence", "state", "rule",
              "outcome", "question", "action", "reason", "found", "mode", "loop-mode", "session"):
        lg.add_argument(f"--{f}")
    sp.add_parser("show")
    sp.add_parser("hook")
    a = ap.parse_args(argv)
    if a.cmd == "hook":
        return hook_main()
    if a.cmd == "show":
        print(request({"op": "show"})["frontmatter"], end="")
        return 0
    args = {k: v for k, v in vars(a).items() if k not in ("cmd", "event") and v is not None}
    if a.event == "report":
        if sys.stdin.isatty():
            print("driverctl: pipe the worker report on stdin", file=sys.stderr)
            return 2
        args["body"] = sys.stdin.read()
    if a.event == "start":
        args.setdefault("session", os.environ.get("CLAUDE_CODE_SESSION_ID"))
    r = request({"op": "log", "event": a.event, "args": args})
    if not r.get("ok"):
        print(f"driverctl: REJECTED: {r.get('error')}", file=sys.stderr)
        return 2
    print("ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
