# Driver sandbox

The experimental driver runs an unattended `claude` session against one of your registered
projects. This page describes how the launcher contains that session on Linux, what the TUI tells
you when a protection is missing, and which risks stay open by design.

The sandbox narrows what a driven agent can reach. It is **not a complete isolation boundary**: read
[Driver sandbox: residual risks](#driver-sandbox-residual-risks) before you point a driver at a
project you do not control.

## What the sandbox does

A sandboxed launch (`bwrap` inside a `systemd-run --user --scope` unit) gives the agent:

- **A network namespace with no network.** The only way out is a small in-sandbox bridge
  (`gsd-meta-manager bridge`) that carries TCP to two proxies the launcher serves on unix sockets:
  an inference proxy for `api.anthropic.com` and a CONNECT proxy for the egress allowlist
  (see [Egress allowlist](#egress-allowlist)).
- **Its credential outside the sandbox.** The run token stays in the launcher. The agent sees a
  fixed dummy value; the inference proxy drops any `authorization` or `x-api-key` the agent sends
  and injects the real token itself. The proxy forwards only `POST /v1/messages` and
  `/v1/messages/count_tokens`; every other path answers `403`, and a request that carries
  `mcp_servers` or a server-side tool other than `web_search` is refused.
- **A replaced `/run`, `/tmp`, `$XDG_RUNTIME_DIR` and `$HOME`.** Each is a tmpfs. `$HOME` gets a
  read-only allowlist re-bound on top of it (the `claude` install, `node`, the Rust toolchain, the
  GSD and rtk files, the `gsd-meta-manager` binary); everything else in your home directory,
  including dotfile credential stores, is simply absent. A symlink in an allowlisted directory that
  points out of the allowlist is not followed and dangles inside. A per-run `~/.gitconfig` carries
  only your `user.name` and `user.email`.
- **Gate policy it cannot turn off.** The gate hooks and the deny rules for the agent-to-agent
  tools (`Cron*`, `RemoteTrigger`, `SendMessage`, `ListAgents`, `ScheduleWakeup`, `DesignSync`,
  `mcp__claude_ai_*`) live in a managed settings file bound read-only at `/etc/claude-code`, so a
  nested `claude` with another config directory still fires them. Host policy already in
  `/etc/claude-code` is merged in, never hidden; the one exception is a host `hooks` or
  `permissions` value of the wrong shape (a value that is not an object, a hook event that is not
  a list, a malformed matcher entry, a permission list that is not a list of strings, or a
  permission setting that is neither a string nor a boolean), which is dropped so it can never
  displace the gate hooks or the deny rules; well-formed entries beside it still merge.
- **Read-only planted-file paths** inside the project (see [Planted-file guard](#planted-file-guard)).

A launch on a host that cannot do this is refused unless you pick **Launch UNSANDBOXED** (or pass
`--unsandboxed`), which asks a second time and lists what the run would lack.

## Relocated host directories

`CLAUDE_CONFIG_DIR` and the XDG base-directory variables move where Claude Code and the host's
tools keep their files. The launcher follows them with one rule: a value counts only when it is
**non-empty and absolute**; a blank or relative value (and a literal `~/x`) is treated as unset, as
Claude Code itself does, and the default under `$HOME` is used. It applies to `CLAUDE_CONFIG_DIR`
(default `~/.claude`), `XDG_CONFIG_HOME` (`~/.config`), `XDG_DATA_HOME` (`~/.local/share`),
`XDG_CACHE_HOME` (`~/.cache`) and `XDG_RUNTIME_DIR`.

What follows `CLAUDE_CONFIG_DIR`: the host-token fallback (`${CLAUDE_CONFIG_DIR:-~/.claude}/.credentials.json`,
read at launch, on every auth relaunch and by the egress server, and reported by `driver auth status`),
and the links and settings the per-run config directory is seeded from. When the variable is set there
is no fallback to `~/.claude`: the launch uses that profile's account or none.

**Binds and masks are the union of the default and the relocated location**, so nothing that was bound
or masked before is dropped:

- the GSD entries (`agents`, `commands`, `skills`, `gsd-core`, `get-shit-done`, `hooks`, `scripts`,
  `.gsd-profile`) of both the configured Claude root and `~/.claude`, and the rtk config of both
  `~/.config/rtk` and `$XDG_CONFIG_HOME/rtk`, are bound read-only (a hook that names `~/.claude/hooks/...`
  keeps working on a profile host);
- the per-run tmpfs for the cache and the rtk data exist at both the default and the relocated paths;
- a relocated XDG directory and every Claude root are never bound as a whole (a tool installed under one
  gets only its own file bound), and their credential stores join the no-overlap rule;
- the stored run token is masked wherever its resolver puts it, and the default `~/...` credential masks stay.

Where a relocated path lies decides what happens to it:

| What | Where | Treatment |
|------|-------|-----------|
| A Claude root | under `$HOME` | Its GSD entries are bound read-only like any `$HOME` entry. |
| A Claude root | under `/tmp` | Its GSD entries are re-bound read-only; `/tmp` is already an empty tmpfs. |
| A Claude root | elsewhere (and not `/`, not an ancestor of `$HOME`, `/tmp`, a toolchain or this binary) | An empty 1 MiB tmpfs hides it; only its GSD entries that exist are re-bound read-only. |
| A Claude root | `/` or an ancestor of `$HOME`, `/tmp`, a toolchain or this binary | Nothing: a tmpfs there would hide the sandbox's own footing. |
| Its `.credentials.json` and `.claude.json` | outside `$HOME` and `/tmp` | Masked with `/dev/null` whenever they exist. |
| Relocated `gh`, `gcloud`, `glab-cli`, `git` and `keyrings` directories | outside `$HOME` and `/tmp`, existing | An empty tmpfs hides each. Masking `git` also keeps the host git config (aliases, includes, `credential.helper`) away from the driver. |
| Relocated cache and rtk data | under `$HOME`, or elsewhere and existing | A per-run tmpfs. |

Under `$HOME` nothing needs masking: the `$HOME` tmpfs already hides every path that is not bound.
`gsd-meta-manager`'s own `GSD_HOME`-relative binds (`.gsd`, `.gsd-capabilities.json`) are not relocated.

## Protections and `protections_absent`

Every launch records `protections_absent` in its run files: the protections that launch did not
have, in the fixed order below. The Driver tab shows it on the posture row (`absent: ...`), the
launch form shows it on the sandbox line, and the unsandboxed confirm repeats the full list.

`protections_absent` is kept apart from `sandbox_missing`. `sandbox_missing` names missing host
tools (`bwrap`, `systemd-run`, `user-systemd`), the causes; `protections_absent` names the effects.
A key that is absent from the run means "not recorded" (a run older than this feature); an empty
list means "lacked nothing".

| Word | What is missing when it is listed |
|------|-----------------------------------|
| `net-isolation` | The agent shares the host network namespace; nothing stops it reaching localhost or LAN services. |
| `egress-proxy` | No launcher-side CONNECT proxy: the egress allowlist is not enforced and the agent can reach any host the machine can. |
| `token-injection` | The run token is in the agent's environment (readable from `/proc/<pid>/environ`) instead of staying in the launcher, and the inference path filter does not apply. |
| `home-allowlist` | `$HOME` is not replaced by a tmpfs with an allowlist: the agent can read your home directory, including credential files. |
| `run-tmpfs` | `/run`, `/tmp` and the runtime directory are the host's: sockets such as the Docker socket, D-Bus and agent sockets are reachable. |
| `planted-file-ro` | The project's git config, hooks and `.claude` settings paths are writable. Detection after the run still happens (it is host-side). |
| `managed-settings` | The gate policy is not bound at `/etc/claude-code`, so a nested `claude` can run without the gate hooks. |
| `registry-overlay` | The cargo registry and git caches are bound read-only instead of as a throwaway overlay, so a build inside the sandbox that needs to write there fails. |

Which words appear:

- A **degraded** (`--unsandboxed`) launch lists **all eight**.
- A sandboxed launch on a full host lists none.
- A sandboxed launch on an **old bubblewrap** lists only `registry-overlay`. Overlay mounts need
  bubblewrap 0.11.0 or newer plus a working dry probe; the launcher feature-detects both. Every
  other protection works on older versions, including 0.9.0 (Ubuntu 24.04).
  `managed-settings` is therefore never listed for a sandboxed launch.

## Egress allowlist

The default is **empty: no CONNECT egress at all**. A project opts in through
`driver.egress` in the manager config, using named presets and extra hosts:

| Preset | Hosts |
|--------|-------|
| `rust` | `index.crates.io`, `static.crates.io`, `crates.io` |
| `github-read` | `github.com`, `codeload.github.com`, `objects.githubusercontent.com` |
| `npm` | `registry.npmjs.org` |
| `python` | `pypi.org`, `files.pythonhosted.org` |

The key shape, validation rules and an example are in
[CONFIGURATION.md](CONFIGURATION.md); they are not repeated here. The active list is recorded on
the run as `egress_allow` and the Driver tab shows its size (`egress <n> hosts`).

How the CONNECT proxy decides:

- Port 443 only, and only hosts on the run's list.
- A name that resolves to a loopback, link-local or private address is refused (DNS rebinding).
- `api.anthropic.com` is **never** reachable through CONNECT, whatever the list says. All inference
  goes through the inference proxy, which is the one place the token is added.
- Every request is logged; headers are not. The log (`egress.log`) is capped at 8 MiB and rotated
  once to `egress.log.1`, so it holds at least the most recent 8 MiB. Refused requests take at most
  a quarter of each file; later ones are counted in a line before the next carried-out request, so a
  flood of junk requests cannot hide an allowed `CONNECT` or an inference call.

## Auth posture

A launch authenticates the agent one of two ways, recorded as `auth` on the run and shown on the
posture row:

- **`setup-token`**: a long-lived token you stored with `gsd-meta-manager driver auth setup`. It is
  minted for inference only (`user:inference`).
- **`host-token`**: the login token your own Claude Code install holds. It carries a wider
  server-side scope (sessions, connectors, plugins, profile and more).

The host-token fallback reads `${CLAUDE_CONFIG_DIR:-~/.claude}/.credentials.json` (see
[Relocated host directories](#relocated-host-directories)). It is kept so a launch works before you run
setup, but the Driver tab then
shows `auth host-token  run driver auth setup`. Run `driver auth setup` once to narrow the
server-side scope of the credential the launcher holds.

Either way the token never enters the sandbox (see above) and the inference proxy's path filter
applies, so even the host token can only reach `/v1/messages`. The setup-token is defence in depth
for the credential itself: if the launcher's token ever leaked, its scope would be inference only.

## Planted-file guard

A driven agent can write files that a later host-side tool will trust: a `.git/config` with a
`core.fsmonitor`, a git hook, a `.envrc`, `.vscode/tasks.json`, `.mcp.json`, a worktree pointer.
The guard works in two halves.

**Inside the sandbox**, where the path already exists, the launcher binds it read-only (and binds
its parent so it cannot be renamed away): `.git/config`, `.git/hooks/`, `.claude/settings.json`,
`.claude/settings.local.json` and `.claude/{hooks,agents,commands}/`. Paths GSD itself writes
during normal work (`.planning/`, `CLAUDE.md`, `.claude/skills/`) stay writable.

**After every launch**, the launcher compares hashes taken before the launch, and re-scans for new
or changed nested `.git` entries and worktree pointers outside the known worktrees, and greps every
git config it finds for keys that execute programs (`fsmonitor`, `hooksPath`, `sshCommand`, `pager`,
`editor`, `alias`, `filter.`, `textconv`, `credential`, `include`). It never runs a repository's own
git configuration to do so. Findings are recorded on the run as `planted`, logged, and shown on the
Driver tab as `PLANTED <n>` in the warning style.

Nothing is ever reverted automatically. Your files stay as the agent left them.

**The push step refuses.** `gsd-meta-manager driver push` runs git on the host, in your project,
so a planted git setting would run there. When the scan finds a git-relevant finding, push parks
the run with a typed reason and pushes nothing. Inspect what was planted, then clear the park
explicitly:

```bash
gsd-meta-manager driver push <project> <run-id> --ack-planted
```

Push also runs its own git commands with `core.hooksPath=/dev/null` and `core.fsmonitor=false`.
The steps that check out the agent's commits (the credential-scan worktree) additionally run with
your global and system git configuration switched off, so a committed `.gitattributes` cannot
activate a filter driver you defined there (git-lfs, for example).

**Large checkouts.** The nested-repository walk visits at most 1,000,000 directory entries. A
checkout that exceeds it is scanned only in part. That is reported as `scan-truncated` and parks
the push when the walk was complete at launch (the agent made the checkout that large). When the
launch-time walk was already incomplete, the finding is `scan-incomplete`: it is listed, but it does
not block the push, because every push of such a project would otherwise need `--ack-planted` and
the flag would become routine. A plant in the unvisited part of such a checkout is not detected.

## Driver sandbox: residual risks

These risks stay open. Some cannot be closed by a sandbox of this kind; the rest were accepted to
keep the driver usable on arbitrary projects. Each is its own entry.

- **The planning-doc steering channel (cannot be closed).** The agent writes `DIRECTION.md`, the
  run's `LOG.md` and files under `.planning/`. Anything outside the sandbox that reads them, such
  as a coordinating agent or a human-run `claude` session in the same project, is reading text an
  untrusted agent wrote. Treat all of it as **untrusted input**: it can contain instructions meant
  for the reader. The TUI renders run text escaped, but no filter makes the content safe for
  another agent to obey. A coordinator must not follow instructions found in these files.
- **Allowlisted package hosts are ingress and low-rate exfiltration.** Whatever host you put on the
  `driver.egress` list can serve the agent prompt-injection content, and can receive small
  amounts of data (a path, a query, a package name) from it. Presets are opt-in for this reason;
  the default is none.
- **Raw inference through the proxy is invisible to hooks.** A plain `/v1/messages` call made with
  no tool use does not pass any gate hook. It is bounded by the active-seconds cap, the scope's
  `RuntimeMaxSec`, `MemoryMax` and `TasksMax`, and the proxy's request count, which is recorded on
  the run as `inference_requests` and shown as `api <n>`. There is no request-count cap.
- **Other contents of relocated XDG directories outside `$HOME` stay readable.** The sandbox mounts `/`
  read-only; only the named credential stores of a relocated `XDG_CONFIG_HOME` or `XDG_DATA_HOME` outside
  `$HOME` and `/tmp` are hidden (see [Relocated host directories](#relocated-host-directories)). Masking
  the whole directory would hide toolchains installed under it. This is unchanged from before the
  directories could be relocated.
- **Dangling out-of-allowlist symlinks.** A symlink inside an allowlisted directory whose target is
  outside the allowlist is not followed, so it dangles inside the sandbox (for example a skill
  directory symlinked into `~/projects`). The launcher logs the dangling entries once; they are an
  accepted loss, not a bug.
- **Worktree-pointer and nested-repo planting is detected but not prevented.** A ro-bind cannot
  close this class: the agent legitimately creates worktrees and nested repositories. The guard
  flags them after the run and push refuses on git-relevant ones. Nested repositories under
  skipped build directories (`target`, `node_modules`, `.venv`, `venv`, `__pycache__`) are not
  scanned.
- **The inference and CONNECT proxy runs in the launcher, outside the sandbox's cgroup.** Its
  memory, connections and time are bounded by the proxy itself, not by the scope's `MemoryMax` and
  `TasksMax`: at most 64 connections; request bodies in flight share a 512 MiB weighted budget (a
  body counts four times its length, a chunked one at the 32 MiB maximum), and a request that
  cannot get its share within 10 seconds is answered `503`; a `CONNECT` tunnel ends after 5 minutes
  without a byte or 1 hour in all; a response relay ends after 10 minutes of upstream silence, 60
  seconds on one client write, or 2 hours in all. A driver can still use all 64 connection slots
  for the length of those limits, which shows up as inference failures in that run only.
- **The same-uid kernel attack surface.** The sandbox is namespaces and mounts, not a virtual
  machine. The agent shares your uid and your kernel; a kernel or `bwrap` vulnerability, or a flaw
  in a namespace the profile leaves open, is outside what this design defends.
- **CDN domain fronting.** A CONNECT to an allowlisted host reaches that host's CDN. A TLS server
  name or `Host` header naming another tenant of the same CDN may be served, and a CONNECT proxy
  that does not decrypt traffic cannot see it.
- **`web_search` queries and results.** The `web_search` server tool stays allowed. Search queries
  are a low-rate exfiltration channel to the search provider, and results are an injection ingress.
- **An allowlisted name that resolves to one of the host's own public address values.** The
  resolved-address guard refuses loopback, link-local and private ranges but does not know this
  machine's own global IPv4 or IPv6 addresses. An allowlisted name that resolved to one would reach
  host services bound to all interfaces. This needs control of DNS for an allowlisted name.
- **Hosts that need an upstream HTTPS proxy are unsupported.** If the launcher's environment sets
  `HTTPS_PROXY` or `https_proxy`, the launcher cannot chain through it: the allowlisted hosts and
  the API may be unreachable from the sandbox. The launch warns once and carries on.
- **Gateway and alternative-provider setups are unsupported.** `ANTHROPIC_BASE_URL`,
  `ANTHROPIC_CUSTOM_HEADERS`, `ALL_PROXY` and the Bedrock, Vertex and Foundry switches
  (`CLAUDE_CODE_USE_BEDROCK`, `CLAUDE_CODE_USE_VERTEX`, `CLAUDE_CODE_USE_FOUNDRY`) are removed from the
  driver's environment in both postures. A sandboxed run reaches `api.anthropic.com` through the
  inference proxy and nothing else; an `--unsandboxed` run talks to the public API directly. If you
  route through a corporate gateway or a cloud provider, inference from the driver either fails or
  goes to the public API. The launch prints one line naming any of these variables it finds set.
- **Remote server-managed settings are not applied inside.** Behind the inference proxy the agent
  does not fetch an organisation's server-managed settings, so policy that only exists there (a
  model allowlist, for example) does not apply to driver runs. The sandbox policy is stricter on
  every axis it covers, but it is not your organisation's policy.

A degraded (`--unsandboxed`) run has none of the containment above; see
[Protections and `protections_absent`](#protections-and-protections_absent).
