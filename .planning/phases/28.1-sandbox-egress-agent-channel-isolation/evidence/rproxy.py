# Host-side: plain-HTTP reverse proxy for api.anthropic.com on a unix socket.
# Strips inbound credentials, injects host token, optional path allowlist, logs METHOD PATH STATUS.
import asyncio, os, sys, time, ssl, json, re
SOCK, LOG = sys.argv[1], sys.argv[2]
FILTER = os.environ.get("FILTER", "0") == "1"
ALLOWED = [re.compile(p) for p in os.environ.get("PATHS", "").split(",") if p]
TOKEN = json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]["accessToken"]
UP = "api.anthropic.com"; CTX = ssl.create_default_context()
def log(m):
    with open(LOG, "a") as f: f.write(f"{time.strftime('%H:%M:%S')} {m}\n")
async def pipe(r, w):
    try:
        while (b := await r.read(65536)): w.write(b); await w.drain()
    except Exception: pass
async def handle(r, w):
    try:
        head = await r.readuntil(b"\r\n\r\n")
        lines = head.decode("latin-1").split("\r\n")
        method, target, ver = lines[0].split(" ", 2)
        if method == "CONNECT":
            log(f"CONNECT {target} DENY"); w.write(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n"); await w.drain(); return
        path = re.sub(r"^https?://[^/]+", "", target)
        hdrs, clen, chunked, sawauth = [], 0, False, []
        for l in lines[1:]:
            if not l: continue
            k, _, v = l.partition(":"); kl = k.strip().lower()
            if kl in ("authorization", "x-api-key"): sawauth.append(kl); continue
            if kl in ("host", "connection", "proxy-connection", "keep-alive"): continue
            if kl == "content-length": clen = int(v)
            if kl == "transfer-encoding" and "chunked" in v.lower(): chunked = True
            hdrs.append(l)
        p0 = path.split("?")[0]
        ok = (not FILTER) or any(rx.fullmatch(p0) for rx in ALLOWED)
        if not ok:
            log(f"{method} {path} DENY(path) auth_in={sawauth}")
            w.write(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"); await w.drain(); return
        if chunked: log(f"  NOTE chunked request body for {path}")
        body = await r.readexactly(clen) if clen else b""
        ur, uw = await asyncio.open_connection(UP, 443, ssl=CTX, server_hostname=UP)
        out = f"{method} {path} {ver}\r\nHost: {UP}\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n" + "".join(h + "\r\n" for h in hdrs) + "\r\n"
        uw.write(out.encode("latin-1") + body); await uw.drain()
        if chunked: asyncio.ensure_future(pipe(r, uw))
        rhead = await ur.readuntil(b"\r\n\r\n")
        status = rhead.split(b"\r\n")[0].decode(errors="replace")
        log(f"{method} {path} -> {status.split(' ',2)[1]} auth_in={sawauth}")
        rhead = re.sub(rb"(?im)^connection:[^\r]*\r\n", b"", rhead)
        w.write(rhead[:-2] + b"Connection: close\r\n\r\n"); await w.drain()
        await pipe(ur, w)
    except Exception as e:
        log(f"  ERR {type(e).__name__}: {e}")
    finally:
        try: w.close()
        except Exception: pass
async def main():
    if os.path.exists(SOCK): os.unlink(SOCK)
    srv = await asyncio.start_unix_server(handle, SOCK); os.chmod(SOCK, 0o600)
    async with srv: await srv.serve_forever()
asyncio.run(main())
