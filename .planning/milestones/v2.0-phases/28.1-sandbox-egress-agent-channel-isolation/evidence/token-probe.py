# usage: probe.py <label> <settings.json or -> <prompt> [ENV=VAL ...]
# Reads host access token from ~/.claude/.credentials.json into child env; never prints it.
import json,os,sys,subprocess,tempfile,shutil
label,settings,prompt=sys.argv[1:4]; extra=dict(a.split("=",1) for a in sys.argv[4:])
D=os.path.dirname(os.path.abspath(__file__))
tok=json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]["accessToken"]
cfg=tempfile.mkdtemp(prefix="cfg-",dir=D)
if settings!="-": shutil.copy(settings,os.path.join(cfg,"settings.json"))
env={k:v for k,v in os.environ.items() if not (k.startswith("CLAUDE") or k.startswith("ANTHROPIC"))}
env.update(CLAUDE_CONFIG_DIR=cfg,CLAUDE_CODE_OAUTH_TOKEN=tok,**extra)
argv=["/home/blk/.local/bin/claude","-p","--setting-sources","user","--model","haiku","--output-format","stream-json","--verbose","--permission-mode","bypassPermissions","--max-budget-usd","0.05",*([a for a in os.environ.get("XARGS","").split() if a]),"--",prompt]
wd=tempfile.mkdtemp(prefix="wd-",dir=D)
p=subprocess.run(argv,env=env,cwd=wd,capture_output=True,text=True,timeout=300)
out=p.stdout.replace(tok,"<REDACTED>"); err=p.stderr.replace(tok,"<REDACTED>")
open(os.path.join(D,label+".jsonl"),"w").write(out); open(os.path.join(D,label+".err"),"w").write(err)
for line in out.splitlines():
    try: e=json.loads(line)
    except: continue
    if e.get("type")=="system" and e.get("subtype")=="init":
        print("TOOLS:",sorted(e.get("tools",[])))
        print("MCP:",e.get("mcp_servers")); print("apiKeySource:",e.get("apiKeySource"))
    if e.get("type")=="user":
        for c in e["message"].get("content",[]):
            if isinstance(c,dict) and c.get("type")=="tool_result": print("TOOL_RESULT:",str(c.get("content"))[:300])
    if e.get("type")=="assistant":
        for c in e["message"].get("content",[]):
            if c.get("type")=="tool_use": print("TOOL_USE:",c["name"],json.dumps(c["input"])[:200])
            if c.get("type")=="text": print("TEXT:",c["text"][:300])
    if e.get("type")=="result": print("RESULT:",e.get("subtype"),"cost",e.get("total_cost_usd"),"denials",e.get("permission_denials"))
print("rc",p.returncode,"stderr:",err[:300])
