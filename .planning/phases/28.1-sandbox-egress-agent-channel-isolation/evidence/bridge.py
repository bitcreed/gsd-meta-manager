# In-sandbox TCP 127.0.0.1:PORT -> unix socket bridge.
import asyncio, sys
PORT, SOCK = int(sys.argv[1]), sys.argv[2]
async def pipe(r, w):
    try:
        while (b := await r.read(65536)): w.write(b); await w.drain()
    except Exception: pass
    finally:
        try: w.close()
        except Exception: pass
async def handle(r, w):
    ur, uw = await asyncio.open_unix_connection(SOCK)
    await asyncio.gather(pipe(r, uw), pipe(ur, w))
async def main():
    srv = await asyncio.start_server(handle, "127.0.0.1", PORT)
    async with srv: await srv.serve_forever()
asyncio.run(main())
