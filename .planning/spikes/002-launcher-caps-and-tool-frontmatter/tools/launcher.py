#!/usr/bin/env python3
"""Spike 002 launcher: owns everything the model must not be trusted with.

Runs on the HOST, outside the sandbox, for the whole life of one driver launch:

* the run lock (flock on RUN_DIR/launcher.lock - no pids, so no pid-namespace reuse bug);
* the driverctl server (single writer of state.json, LOG.md and DIRECTION.md's frontmatter);
* a per-run CLAUDE_CONFIG_DIR (host ~/.claude stays read-only and its secrets are masked);
* auth by access token in the environment (no refresh token ever enters the sandbox);
* hard caps counted from tool-written state, not from the driver's say-so:
    - dispatches: driver-level Agent calls seen by the PreToolUse hook (denied at the limit);
    - wall clock: active minutes across launches;
    - gap rounds: distinct `status: gaps_found` VERIFICATION.md writes per phase, observed on disk;
  a tripped cap denies further dispatches and then kills the systemd scope.

    launcher.py init --project P --run-id R --direction DIRECTION.md
    launcher.py run  --project P --run-id R [--model opus] [--loop-mode loop]
                     [--cap-dispatches 10] [--cap-minutes 60] [--cap-gap-rounds 2]
                     [--max-budget-usd 10] [--sandbox bwrap|none] [--state-dir D]
"""
from __future__ import annotations

import argparse
import fcntl
import glob
import hashlib
import json
import os
import re
import shutil
import signal
import socket
import subprocess
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import driverctl as dc  # noqa: E402

SPIKE = os.path.dirname(HERE)
HOME = os.path.expanduser("~")
HOST_CLAUDE = os.path.join(HOME, ".claude")
LINK_FROM_HOST = ("agents", "commands", "skills", "gsd-core", "get-shit-done", "hooks")
STRIP_ENV = ("CLAUDECODE", "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_BRIDGE_SESSION_ID",
             "CLAUDE_CODE_ENTRYPOINT", "CLAUDE_CODE_CHILD_SESSION", "CLAUDE_CODE_MESSAGING_SOCKET",
             "CLAUDE_CODE_MESSAGING_TOKEN", "CLAUDE_PID", "SSH_AUTH_SOCK", "GH_TOKEN", "GITHUB_TOKEN",
             "CLAUDE_CONFIG_DIR", "ANTHROPIC_API_KEY", "CLAUDE_ENV_FILE")


def run_dir_of(project, run_id):
    return os.path.join(project, ".planning", "meta-manager", "drivers", run_id)


# ---------------------------------------------------------------- init

def cmd_init(a):
    rd = run_dir_of(a.project, a.run_id)
    os.makedirs(rd, exist_ok=True)
    gi = os.path.join(os.path.dirname(rd), ".gitignore")
    if not os.path.exists(gi):
        with open(gi, "w") as f:
            f.write("*\n")
    shutil.copy(a.direction, os.path.join(rd, "DIRECTION.md"))
    with open(os.path.join(SPIKE, "templates", "LOG.md")) as f:
        tmpl = f.read()
    grammar = tmpl[tmpl.index("<!--"): tmpl.index("-->") + 3]
    with open(os.path.join(rd, "LOG.md"), "w") as f:
        f.write(f"# Driver log: {a.run_id}\n\n{grammar}\n\n")
    store = dc.RunStore(rd, a.project)
    store.save()
    print(rd)


# ---------------------------------------------------------------- per-run Claude config dir

def seed_config(cfg: str, hook_cmd: str):
    os.makedirs(os.path.join(cfg, "bin"), exist_ok=True)
    for name in LINK_FROM_HOST:
        src, dst = os.path.join(HOST_CLAUDE, name), os.path.join(cfg, name)
        if os.path.exists(src) and not os.path.lexists(dst):
            os.symlink(src, dst)                       # resolves to a read-only path in the sandbox
    with open(os.path.join(HOST_CLAUDE, "settings.json")) as f:
        s = json.load(f)
    for k in ("statusLine", "enabledPlugins", "remoteControlAtStartup", "agentPushNotifEnabled"):
        s.pop(k, None)
    hooks = s.setdefault("hooks", {})
    mine = lambda m=None: {**({"matcher": m} if m else {}),            # noqa: E731
                           "hooks": [{"type": "command", "command": hook_cmd, "timeout": 20}]}
    for ev in ("SessionStart", "SessionEnd", "Stop"):
        hooks.setdefault(ev, []).append(mine())
    hooks.setdefault("PreToolUse", []).insert(0, mine("Agent|Task"))
    hooks.setdefault("PostToolUse", []).append(mine("*"))
    with open(os.path.join(cfg, "settings.json"), "w") as f:
        json.dump(s, f, indent=1)
    gj = os.path.join(cfg, ".claude.json")
    if not os.path.exists(gj):
        with open(gj, "w") as f:
            json.dump({"hasCompletedOnboarding": True}, f)
    wrapper = os.path.join(cfg, "bin", "driverctl")
    with open(wrapper, "w") as f:
        f.write(f'#!/bin/sh\nexec python3 -I {os.path.join(HERE, "driverctl.py")} "$@"\n')
    os.chmod(wrapper, 0o755)
    return wrapper


def host_access_token():
    with open(os.path.join(HOST_CLAUDE, ".credentials.json")) as f:
        c = json.load(f)["claudeAiOauth"]
    return c["accessToken"], c["expiresAt"] / 1000.0


# ---------------------------------------------------------------- gap-round observation

FM_STATUS = re.compile(r"^status:\s*([A-Za-z_]+)", re.M)


def scan_gaps(project, seen: dict, per_phase: dict):
    """Count distinct gaps_found VERIFICATION.md writes per phase. Returns newly seen (phase, file)."""
    new = []
    for p in sorted(glob.glob(os.path.join(project, ".planning", "phases", "*", "*-VERIFICATION.md"))):
        try:
            with open(p, "rb") as f:
                raw = f.read()
        except OSError:
            continue
        text = raw.decode(errors="replace")
        fm = text.split("\n---", 1)[0] if text.startswith("---") else ""
        m = FM_STATUS.search(fm)
        if not m or m.group(1) != "gaps_found":
            continue
        key = hashlib.sha256(raw).hexdigest()[:16]
        if key in seen:
            continue
        phase = re.match(r"0*(\d+(?:\.\d+)?)", os.path.basename(os.path.dirname(p))).group(1)
        seen[key] = {"phase": phase, "file": os.path.relpath(p, project)}
        per_phase[phase] = per_phase.get(phase, 0) + 1
        new.append((phase, os.path.relpath(p, project), per_phase[phase]))
    return new


def mentions_phase(prompt: str, phase: str) -> bool:
    ph = re.escape(phase)
    return bool(re.search(rf"(?i)(phase[\s_-]*0*{ph}\b|-phase\s+0*{ph}\b|\b0*{ph}\s+--gaps)", prompt))


# ---------------------------------------------------------------- run

class Launch:
    def __init__(self, a):
        self.a = a
        self.project = os.path.abspath(a.project)
        self.rd = run_dir_of(self.project, a.run_id)
        self.tripped = None
        self.unit = f"gsdmm-{a.run_id}-{int(time.time())}"
        # Outside the project on purpose: the project is writable inside the sandbox, the state dir is not
        # needed there except for claude-config (bound separately).
        self.state_dir = os.path.abspath(a.state_dir or os.path.join(
            os.environ.get("XDG_STATE_HOME", os.path.join(HOME, ".local", "state")),
            "gsd-meta-manager", "driver-runs", a.run_id))

    def gate(self, p):
        s = self.store.s
        caps = s["caps"]
        lim = caps["dispatches"]["limit"]
        if self.tripped:
            return False, f"run stopped by launcher cap {self.tripped}"
        gl = caps["gap_rounds"]["limit"]
        if gl is not None:
            for ph, n in caps["gap_rounds"]["per_phase"].items():
                if n >= gl + 1 and mentions_phase(p.get("prompt", ""), ph):
                    return False, (f"phase {ph} has failed verification {n} times (gap-round cap {gl}); "
                                   f"no further dispatch for phase {ph}. Park the item with --rule gap-rounds.")
        if lim is not None and s["hook_dispatches"] >= lim:
            return False, f"dispatch cap reached ({s['hook_dispatches']}/{lim}). Park the item with --rule launcher-cap and end the run."
        return True, None

    def trip(self, cap, used, limit):
        if self.tripped:
            return
        self.tripped = cap
        with self.store.lock:
            self.store.append_log("RUN", "cap-tripped", {"cap": cap, "used": used, "limit": limit,
                                                         "action": "kill-scope"})
            self.store.s["status"], self.store.s["status_reason"] = "stopped", f"launcher-cap:{cap}"
            self.store.save()
        subprocess.run(["systemctl", "--user", "kill", "--signal=SIGTERM", f"{self.unit}.scope"],
                       capture_output=True)
        t = time.time()
        while self.proc.poll() is None and time.time() - t < 15:
            time.sleep(0.5)
        subprocess.run(["systemctl", "--user", "kill", "--signal=SIGKILL", f"{self.unit}.scope"],
                       capture_output=True)

    def run(self):
        a = self.a
        os.makedirs(self.state_dir, exist_ok=True)
        lockf = open(os.path.join(self.rd, "launcher.lock"), "a+")
        try:
            fcntl.flock(lockf, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            print("launcher: run is locked by another launcher (flock held) - refusing", file=sys.stderr)
            return 3
        lockf.seek(0), lockf.truncate(), lockf.write(f"pid={os.getpid()} host={socket.gethostname()}\n"), lockf.flush()

        self.store = st = dc.RunStore(self.rd, self.project)
        st.gate = self.gate
        s = st.s
        with st.lock:
            if s["status"] in ("running", "starting") and s["launches"] > 0 and not s.get("ended_by_driver"):
                st.append_log("RUN", "crash-detected", {"previous_launch": s["launches"],
                                                        "evidence": "status=running and no RUN end; lock was free"})
            for c in s["connections"]:
                if c.get("state") == "connected":
                    c["state"] = "lost"
            s["connections"] = [c for c in s["connections"] if c.get("role") != "launcher"][-4:]
            s["launches"] += 1
            s["ended_by_driver"] = False
            s["status"], s["status_reason"] = "running", None
            caps = s["caps"]
            caps["dispatches"]["limit"] = a.cap_dispatches
            caps["wall_clock_min"]["limit"] = a.cap_minutes
            caps["gap_rounds"]["limit"] = a.cap_gap_rounds
            prior_active = s.get("active_seconds", 0)
            t0 = time.time()
            launcher_conn = {"role": "launcher", "state": "connected", "pid": os.getpid(), "host": socket.gethostname(),
                             "started_at": dc.now(), "last_heartbeat": dc.now(), "mode": a.sandbox,
                             "scope_unit": f"{self.unit}.scope"}
            driver_conn = {"role": "driver", "state": "connected", "session_id": None, "model": a.model,
                           "mode": "headless", "pid": None, "scope_unit": f"{self.unit}.scope",
                           "host": socket.gethostname(), "started_at": dc.now(), "last_heartbeat": None}
            s["connections"] += [launcher_conn, driver_conn]
            st.append_log("RUN", "launch", {"n": s["launches"], "model": a.model, "loop_mode": a.loop_mode,
                                            "sandbox": a.sandbox, "unit": f"{self.unit}.scope",
                                            "caps": f"dispatches={a.cap_dispatches},minutes={a.cap_minutes},gap_rounds={a.cap_gap_rounds}"})
            st.save()

        sock_dir = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "gsdmm-driverctl")
        os.makedirs(sock_dir, mode=0o700, exist_ok=True)
        sock = os.path.join(sock_dir, f"{a.run_id}.sock")
        stop = threading.Event()
        srv = threading.Thread(target=dc.serve, args=(st, sock, stop), daemon=True)
        srv.start()

        cfg = os.path.join(self.state_dir, "claude-config")
        inner_sock = "/tmp/driverctl.sock" if a.sandbox == "bwrap" else sock
        seed_config(cfg, os.path.join(cfg, "bin", "driverctl") + " hook")
        token, exp = host_access_token()
        left_min = (exp - time.time()) / 60
        with st.lock:
            s["auth"] = {"mode": "env-access-token", "expires_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(exp)),
                         "minutes_left_at_launch": round(left_min, 1)}
            st.save(regen=False)
        if left_min < 5:
            print("launcher: host access token expires in <5 min; run any host claude command to refresh first",
                  file=sys.stderr)
            return 4

        with open(a.prompt_file or os.path.join(SPIKE, "MASTER-PROMPT.md")) as f:
            prompt = (f.read().replace("{RUN_DIR}", self.rd).replace("{PROJECT_ROOT}", self.project)
                      .replace("{MODE}", a.loop_mode))
        prompt += f"\n\nBegin now: run Startup, then the Loop, for RUN_DIR={self.rd}."
        env = {k: v for k, v in os.environ.items() if k not in STRIP_ENV}
        env.update({"CLAUDE_CONFIG_DIR": cfg, "DRIVERCTL_SOCK": inner_sock, "DRIVER_LOCK_HELD": "1",
                    "PATH": os.path.join(cfg, "bin") + ":" + env.get("PATH", ""),
                    "CLAUDE_CODE_OAUTH_TOKEN": token})
        claude = ["claude", "-p", prompt, "--model", a.model, "--output-format", "stream-json", "--verbose",
                  "--permission-mode", "bypassPermissions", "--max-budget-usd", str(a.max_budget_usd)]
        if a.sandbox == "bwrap":
            mask = []
            for d in (".ssh", ".gnupg", ".config/gh", ".aws", ".docker"):
                if os.path.isdir(os.path.join(HOME, d)):
                    mask += ["--tmpfs", os.path.join(HOME, d)]
            for fpath in (os.path.join(HOST_CLAUDE, ".credentials.json"), os.path.join(HOME, ".claude.json"),
                          os.path.join(HOME, ".git-credentials")):
                if os.path.exists(fpath):
                    mask += ["--ro-bind", "/dev/null", fpath]
            cmd = ["systemd-run", "--user", "--scope", "--quiet", f"--unit={self.unit}",
                   "-p", "MemoryMax=6G", "-p", "TasksMax=1024",
                   "-p", f"RuntimeMaxSec={a.cap_minutes * 60 + 300}",
                   "bwrap", "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp",
                   *mask,
                   "--bind", self.project, self.project,
                   "--ro-bind", self.rd, self.rd,             # run dir: read-only to everything inside
                   "--bind", cfg, cfg,
                   "--bind", sock, inner_sock,
                   "--unshare-pid", "--die-with-parent", "--chdir", self.project, *claude]
        else:
            cmd = claude
        out_path = os.path.join(self.state_dir, f"launch-{s['launches']}.jsonl")
        out = open(out_path, "w")
        self.proc = subprocess.Popen(cmd, env=env, stdin=subprocess.DEVNULL, stdout=out, stderr=subprocess.STDOUT,
                                     cwd=self.project, start_new_session=True)
        del token, env
        with st.lock:
            driver_conn["pid"] = self.proc.pid
            st.save()

        def on_signal(sig, _f):
            self.trip("operator-signal", sig, None)
        signal.signal(signal.SIGTERM, on_signal)
        signal.signal(signal.SIGINT, on_signal)

        last_hb = 0
        while self.proc.poll() is None:
            time.sleep(2)
            el = time.time() - t0
            with st.lock:
                caps = s["caps"]
                new = scan_gaps(self.project, caps["gap_rounds"]["seen"], caps["gap_rounds"]["per_phase"])
                for ph, fpath, cnt in new:
                    st.append_log("RUN", "gap-observed", {"phase": ph, "count": cnt,
                                                          "file": fpath, "source": "launcher-scan"})
                caps["wall_clock_min"]["used"] = round((prior_active + el) / 60, 1)
                hb = time.time() - last_hb > 15
                if hb:
                    launcher_conn["last_heartbeat"] = dc.now()
                    last_hb = time.time()
                if new:
                    st.save()
                elif hb:
                    st.save(heartbeat_only=True)
                gl = caps["gap_rounds"]["limit"]
                worst = max(caps["gap_rounds"]["per_phase"].values(), default=0)
            if a.cap_minutes and (prior_active + el) / 60 > a.cap_minutes:
                self.trip("wall_clock_min", round((prior_active + el) / 60, 1), a.cap_minutes)
            elif gl is not None and worst > gl + 1:
                self.trip("gap_rounds", worst, gl)
            elif a.cap_dispatches is not None and s["hook_dispatches"] > a.cap_dispatches:
                self.trip("dispatches", s["hook_dispatches"], a.cap_dispatches)
        rc = self.proc.wait()
        out.close()

        result = None
        with open(out_path) as f:
            for line in f:
                if line.startswith("{"):
                    try:
                        d = json.loads(line)
                    except ValueError:
                        continue
                    if d.get("type") == "result":
                        result = d
        with st.lock:
            s["active_seconds"] = prior_active + (time.time() - t0)
            s["caps"]["wall_clock_min"]["used"] = round(s["active_seconds"] / 60, 1)
            for c in s["connections"]:
                if c.get("state") == "connected":
                    c["state"] = "disconnected"
            cost = None
            if result:
                cost = {"launch": s["launches"], "total_cost_usd": result.get("total_cost_usd"),
                        "subtype": result.get("subtype"), "num_turns": result.get("num_turns"),
                        "duration_s": round((result.get("duration_ms") or 0) / 1000),
                        "session_id": result.get("session_id"),
                        "by_model": {m: {"cost_usd": u.get("costUSD"), "in": u.get("inputTokens"),
                                         "out": u.get("outputTokens"), "cache_read": u.get("cacheReadInputTokens"),
                                         "cache_write": u.get("cacheCreationInputTokens")}
                                     for m, u in (result.get("modelUsage") or {}).items()}}
                s["cost"].append(cost)
            if not self.tripped and not s.get("ended_by_driver"):
                s["status"], s["status_reason"] = "crashed", f"driver exited rc={rc} without RUN end"
            st.append_log("RUN", "launch-end", {"rc": rc, "tripped": self.tripped,
                                                "cost_usd": cost and cost["total_cost_usd"],
                                                "status": s["status"]})
            st.save()
        stop.set()
        srv.join(timeout=3)
        fcntl.flock(lockf, fcntl.LOCK_UN)
        lockf.close()
        subprocess.run(["systemctl", "--user", "stop", f"{self.unit}.scope"], capture_output=True)
        print(json.dumps({"rc": rc, "status": s["status"], "reason": s["status_reason"], "tripped": self.tripped,
                          "cost": cost, "stream": out_path}, indent=1))
        return 0


def main():
    ap = argparse.ArgumentParser()
    sp = ap.add_subparsers(dest="cmd", required=True)
    i = sp.add_parser("init")
    i.add_argument("--project", required=True)
    i.add_argument("--run-id", required=True)
    i.add_argument("--direction", required=True)
    r = sp.add_parser("run")
    r.add_argument("--project", required=True)
    r.add_argument("--run-id", required=True)
    r.add_argument("--model", default="opus")
    r.add_argument("--loop-mode", default="loop", choices=("loop", "fresh-per-item"))
    r.add_argument("--cap-dispatches", type=int, default=10)
    r.add_argument("--cap-minutes", type=int, default=60)
    r.add_argument("--cap-gap-rounds", type=int, default=2)
    r.add_argument("--max-budget-usd", type=float, default=10)
    r.add_argument("--sandbox", default="bwrap", choices=("bwrap", "none"))
    r.add_argument("--state-dir")
    r.add_argument("--prompt-file", help="override MASTER-PROMPT.md (cap tests use a misbehaving driver)")
    a = ap.parse_args()
    if a.cmd == "init":
        a.project = os.path.abspath(a.project)
        return cmd_init(a)
    return Launch(a).run()


if __name__ == "__main__":
    sys.exit(main() or 0)
