# API Coverage — Claude Code CLI (driver runtime and auth) and GitHub CLI (`gh`, push-step PR)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Phase 28 calls no HTTP API itself: it drives two host CLIs. The OAuth token is passed through as
> `CLAUDE_CODE_OAUTH_TOKEN`; the launcher never calls an OAuth endpoint.

| capability | decision | reason |
|---|---|---|
| claude-cli:print-stream-json | INTEGRATE | |
| claude-cli:model-budget-permission-flags | INTEGRATE | |
| claude-cli:setting-sources-user | INTEGRATE | |
| claude-cli:hooks-with-onfailure-block | INTEGRATE | |
| claude-cli:setup-token | INTEGRATE | |
| claude-cli:version | INTEGRATE | |
| claude-cli:oauth-token-env | INTEGRATE | |
| claude-cli:rate-limit-and-result-events | INTEGRATE | |
| claude-cli:interactive-tty-attach | OPT-OUT | explicitly out of scope — LNCH-08 makes attach optional; CONTEXT Deferred Ideas |
| claude-cli:session-resume | OPT-OUT | not needed — D-05 relaunches fresh per launch under one lock |
| claude-cli:oauth-refresh-flow | OPT-OUT | explicitly out of scope — D-22 forbids refresh-token material in the sandbox |
| claude-cli:mcp-servers | OPT-OUT | not needed — the driver uses the user's GSD skills and hooks only |
| claude-cli:api-key-auth | OPT-OUT | explicitly out of scope — API keys and apiKeyHelper are stripped so they cannot outrank the run token (RESEARCH §3) |
| gh:pr-create | INTEGRATE | |
| gh:pr-view-list-merge-close-comment | OPT-OUT | not needed — D-26 scopes the push step to opening at most one PR per run |
| gh:auth-token | OPT-OUT | explicitly out of scope — the host push uses the user's own git credentials; nothing is read from gh |
| gh:issues-releases-checks-actions | OPT-OUT | not needed — no phase requirement touches them |
