# Outside-sandbox HTTP CONNECT proxy on a unix socket, with host allowlist + log.
import asyncio, os, sys, time, fnmatch
SOCK, LOG = sys.argv[1], sys.argv[2]
ALLOW = [a for a in os.environ.get("ALLOW", "*").split(",") if a]
def allowed(h): return any(fnmatch.fnmatch(h, p) for p in ALLOW)
def log(msg):
    with open(LOG, "a") as f: f.write(f"{time.strftime('%H:%M:%S')} {msg}\n")
async def pipe(r, w):
    try:
        while (b := await r.read(65536)): w.write(b); await w.drain()
    except Exception: pass
    finally:
        try: w.close()
        except Exception: pass
async def handle(r, w):
    try:
        head = await r.readuntil(b"\r\n\r\n")
    except Exception: w.close(); return
    line = head.split(b"\r\n")[0].decode(errors="replace")
    parts = line.split()
    if len(parts) < 2: w.close(); return
    method, target = parts[0], parts[1]
    if method != "CONNECT":
        log(f"NONCONNECT {method} {target} DENY"); w.write(b"HTTP/1.1 403 Forbidden\r\n\r\n"); await w.drain(); w.close(); return
    host, _, port = target.rpartition(":")
    ok = allowed(host) and port == "443"
    log(f"CONNECT {target} {'ALLOW' if ok else 'DENY'}")
    if not ok:
        w.write(b"HTTP/1.1 403 Forbidden\r\n\r\n"); await w.drain(); w.close(); return
    try: ur, uw = await asyncio.open_connection(host, int(port))
    except Exception as e:
        log(f"  upstream fail {target}: {e}"); w.write(b"HTTP/1.1 502 Bad Gateway\r\n\r\n"); await w.drain(); w.close(); return
    w.write(b"HTTP/1.1 200 Connection Established\r\n\r\n"); await w.drain()
    await asyncio.gather(pipe(r, uw), pipe(ur, w))
async def main():
    if os.path.exists(SOCK): os.unlink(SOCK)
    srv = await asyncio.start_unix_server(handle, SOCK)
    os.chmod(SOCK, 0o600)
    async with srv: await srv.serve_forever()
asyncio.run(main())
