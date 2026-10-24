# Capture reverse proxy: like evidence/rproxy.py, but logs request SHAPE (header names, selected
# header values, JSON body top-level keys, tool type/name pairs). Never logs credential values.
import asyncio, os, sys, time, ssl, json, re
SOCK, LOG = sys.argv[1], sys.argv[2]
TOKEN = json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]["accessToken"]
UP = "api.anthropic.com"; CTX = ssl.create_default_context()
SHOW = ("anthropic-beta", "anthropic-version", "user-agent", "x-app", "content-type", "accept",
        "anthropic-dangerous-direct-browser-access", "transfer-encoding", "content-encoding")
def log(m):
    with open(LOG, "a") as f: f.write(f"{time.strftime('%H:%M:%S')} {m}\n")
async def pipe(r, w):
    try:
        while (b := await r.read(65536)): w.write(b); await w.drain()
    except Exception: pass
async def read_chunked(r):
    out = b""
    while True:
        line = await r.readuntil(b"\r\n"); n = int(line.split(b";")[0].strip(), 16)
        if n == 0:
            await r.readuntil(b"\r\n"); return out
        out += await r.readexactly(n); await r.readexactly(2)
def summarize(body):
    try: j = json.loads(body)
    except Exception as e: return f"NONJSON({type(e).__name__}) len={len(body)}"
    if not isinstance(j, dict): return f"JSON-{type(j).__name__}"
    s = {"keys": sorted(j.keys()), "model": j.get("model"), "stream": j.get("stream"),
         "max_tokens": j.get("max_tokens"),
         "tools": [(t.get("type", "-"), t.get("name")) for t in j.get("tools", []) if isinstance(t, dict)],
         "tool_keys": sorted({k for t in j.get("tools", []) if isinstance(t, dict) for k in t}),
         "metadata_keys": sorted((j.get("metadata") or {}).keys()),
         "thinking": j.get("thinking"), "output_config": j.get("output_config"),
         "context_management": j.get("context_management"), "tool_choice": j.get("tool_choice"),
         "mcp_servers": j.get("mcp_servers"), "container": j.get("container"),
         "system_type": type(j.get("system")).__name__,
         "msg_block_types": sorted({c.get("type") for m in j.get("messages", []) if isinstance(m.get("content"), list) for c in m["content"] if isinstance(c, dict)})}
    return json.dumps(s)
async def handle(r, w):
    try:
        head = await r.readuntil(b"\r\n\r\n")
        lines = head.decode("latin-1").split("\r\n")
        method, target, ver = lines[0].split(" ", 2)
        if method == "CONNECT":
            log(f"CONNECT {target} DENY"); w.write(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n"); await w.drain(); return
        path = re.sub(r"^https?://[^/]+", "", target)
        hdrs, clen, chunked, names, shown = [], 0, False, [], {}
        for l in lines[1:]:
            if not l: continue
            k, _, v = l.partition(":"); kl = k.strip().lower(); names.append(kl)
            if kl in SHOW: shown[kl] = v.strip()
            if kl in ("authorization", "x-api-key"): continue
            if kl in ("host", "connection", "proxy-connection", "keep-alive", "content-length", "transfer-encoding"): continue
            if kl == "content-length": pass
            hdrs.append(l)
        for l in lines[1:]:
            k, _, v = l.partition(":"); kl = k.strip().lower()
            if kl == "content-length": clen = int(v)
            if kl == "transfer-encoding" and "chunked" in v.lower(): chunked = True
        body = await read_chunked(r) if chunked else (await r.readexactly(clen) if clen else b"")
        log(f"REQ {method} {path} chunked={chunked} len={len(body)} hdr_names={sorted(names)}")
        log(f"    hdr_vals={json.dumps(shown)}")
        if body: log(f"    body={summarize(body)}")
        ur, uw = await asyncio.open_connection(UP, 443, ssl=CTX, server_hostname=UP)
        out = (f"{method} {path} HTTP/1.1\r\nHost: {UP}\r\nAuthorization: Bearer {TOKEN}\r\nConnection: close\r\n"
               f"Content-Length: {len(body)}\r\n" + "".join(h + "\r\n" for h in hdrs) + "\r\n")
        uw.write(out.encode("latin-1") + body); await uw.drain()
        rhead = await ur.readuntil(b"\r\n\r\n")
        status = rhead.split(b"\r\n")[0].decode(errors="replace")
        log(f"    -> {status}")
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
