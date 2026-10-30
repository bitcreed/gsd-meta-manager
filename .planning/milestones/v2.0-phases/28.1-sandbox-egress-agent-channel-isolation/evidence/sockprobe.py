import socket,sys
for p in open(sys.argv[1]).read().split():
    s=socket.socket(socket.AF_UNIX); s.settimeout(1)
    try: s.connect(p); r="CONNECT-OK"
    except Exception as e: r=type(e).__name__
    s.close(); print(r, p)
