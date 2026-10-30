# API Coverage — Anthropic API behind the launcher egress proxy, and Claude Code CLI network/settings surface

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Phase 28.1 does not call the Anthropic API for its own features: it forwards the sandboxed driver's
> requests through a token-injecting reverse proxy (D-04) whose whole purpose is to let exactly the
> inference surface through (D-05). Every opt-out below is a security decision from CONTEXT D-05/D-06/D-13
> and RESEARCH Q1, not a gap. The D-11 reach probe (28.1-07) calls four account endpoints read-only in an
> opt-in test to measure a token's server-side scope; that is a measurement, not an integration.

| capability | decision | reason |
|---|---|---|
| anthropic:messages-create (`POST /v1/messages`, any query) | INTEGRATE | |
| anthropic:messages-count-tokens (`POST /v1/messages/count_tokens`) | INTEGRATE | |
| anthropic:messages-streaming (SSE responses) | INTEGRATE | |
| anthropic:messages-server-tool-web-search (`web_search_*`) | INTEGRATE | |
| anthropic:messages-client-tools (no `type` / `custom`) | INTEGRATE | |
| anthropic:oauth-bearer-auth (Bearer + `oauth-2025-04-20` beta) | INTEGRATE | |
| anthropic:messages-server-tools-other (web_fetch, code_execution, advisor, ...) | OPT-OUT | explicitly out of scope — server-side reach on the driver's behalf; refused by the body policy (D-05, RESEARCH Q1: web_fetch, code_execution, advisor, tool_search, mcp_toolset, computer) |
| anthropic:messages-mcp-connector (`mcp_servers`) | OPT-OUT | explicitly out of scope — Anthropic-side MCP reach; refused (D-05) |
| anthropic:messages-url-and-file-sources (`source.type` url/file) | OPT-OUT | explicitly out of scope — server-side fetch channel; refused recursively (RESEARCH Pitfall 1) |
| anthropic:messages-container-skills (`container`) | OPT-OUT | explicitly out of scope — runs code in an Anthropic container; refused (RESEARCH Q1) |
| anthropic:api-key-auth (`x-api-key`) | OPT-OUT | explicitly out of scope — inbound credentials are dropped; the launcher injects only the run's OAuth token |
| anthropic:models-list | OPT-OUT | not needed — never observed in Claude Code 2.1.310 traffic; add only from evidence in egress.log (RESEARCH Open Question 4) |
| anthropic:files | OPT-OUT | explicitly out of scope — org file access is account reach (D-05 path allowlist) |
| anthropic:message-batches | OPT-OUT | not needed — Claude Code does not use batches |
| anthropic:code-sessions (`/v1/code/sessions`) | OPT-OUT | explicitly out of scope — account-scoped Remote Control surface; 403 at the proxy (D-05); probed read-only by the D-11 test |
| anthropic:code-triggers (`/v1/code/triggers`) | OPT-OUT | explicitly out of scope — routines/RemoteTrigger reach; 403 at the proxy (D-05); probed read-only by the D-11 test |
| anthropic:mcp-servers-listing (`/v1/mcp_servers`) | OPT-OUT | explicitly out of scope — claude.ai connector reach; 403 at the proxy (D-05); probed read-only by the D-11 test |
| anthropic:environments-bridge (`/v1/environments/bridge`) | OPT-OUT | explicitly out of scope — Remote Control bridge; 403 at the proxy (D-05) |
| anthropic:remote-control-eligibility-profile | OPT-OUT | explicitly out of scope — 403 at the proxy; probed read-only by the D-11 test |
| anthropic:oauth-token-refresh | OPT-OUT | explicitly out of scope — no refresh token is handled; the host token is re-read from the file the host's own claude refreshes (RESEARCH Pitfall 4) |
| anthropic:admin-organization-apis | OPT-OUT | explicitly out of scope — no admin key, no account management from a driver run |
| claude-cli:managed-settings-file (`/etc/claude-code/managed-settings.json`) | INTEGRATE | |
| claude-cli:permissions-deny | INTEGRATE | |
| claude-cli:anthropic-base-url | INTEGRATE | |
| claude-cli:https-proxy-env (HTTP CONNECT) | INTEGRATE | |
| claude-cli:nonessential-traffic-switches (telemetry, autoupdate, preflight) | INTEGRATE | |
| claude-cli:socks-proxy | OPT-OUT | not needed — Claude Code supports HTTP CONNECT only (docs) |
| claude-cli:remote-server-managed-settings | OPT-OUT | explicitly out of scope — not fetched behind a non-Anthropic base URL; documented residual risk (RESEARCH Open Question 3) |
| claude-cli:upstream-proxy-chaining | OPT-OUT | not needed yet — hosts behind a corporate HTTPS proxy are unsupported, warned at launch and documented (RESEARCH Open Question 2, `[inferred — audit]`) |
