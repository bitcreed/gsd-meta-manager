# Phase 22: Container Execution Target - Research

**Researched:** 2026-09-16
**Domain:** container runtime integration (rootless podman / rootful docker), Claude CLI auth and session storage, git-envelope parity across a process boundary
**Confidence:** HIGH for the five routed questions (all five answered by measurement on this host), MEDIUM for the image recipe (not built)

> **Unattended run.** The human operator was unavailable. Nothing below relitigates a locked
> decision. Where a measurement contradicts one, it is raised in
> `## Conflicts with locked decisions` rather than planned around silently.

---

## Summary

Every one of the five questions CONTEXT.md routed here was answerable **by measurement on
this machine**, and four of the five came back with a different answer than the artifacts
anticipated. The headline results: `claude setup-token` is **not** disqualified by the Phase 15
gate (`apiKeySource` is `"none"` on the OAuth-token path, measured); `--network=slirp4netns`
**is** sufficient for the CLI's egress on rootless podman 5.7.0 (real TLS handshake against
`api.anthropic.com`, measured) so `passt` is a preference, not a prerequisite; bollard costs
**25 locked crates and ~5 s of clean release build** and its feature flags buy nothing; and an
egress allowlist is **enforceable on rootful docker and not enforceable on rootless podman**
without the privileged flags the spike's fence forbids — which is the single finding that
should change how the plan words D-22-14.

The envelope question (Q2) resolves against the cheaper option. Option (b) — "the egress
allowlist plus the absence of a git credential makes the push boundary unreachable" — is
**false on two independent grounds**, both measured: the allowlist is not enforceable on one of
the two required runtimes, and a `git push` to a *local path remote* needs neither network nor
credential (every one of this research's four git-hook probes pushed exactly that way). Option
(a) is therefore the only true resolution, and it is **smaller than CONTEXT.md feared**: the
host binary can be bind-mounted read-only at its identical absolute path and runs unmodified
inside a glibc container under `--userns=keep-id` (measured, prints its own `--version`). That
is one plan's work, not a phase's, and it should stay **in scope for Phase 22**.

The one genuinely new hazard this research found and no artifact names: the driven CLI's
**session history lives inside `CLAUDE_CONFIG_DIR`** (`<dir>/projects/<slug-of-absolute-path>/`,
measured), and the slug **is the project's absolute path with `/` replaced by `-`**. That is
direct mechanical evidence for D-22-08's identity path map, and it settles the design fork the
planner was asked to settle: the named credential volume carries session history *for free*,
because it is the same directory. No second volume.

**Primary recommendation:** Build the container target as an argv prefix composed in a new
`ExecutionTarget::Container` arm of `build_argv`, carrying a verified-runtime capability token;
mount the project root, the alias's envelope directory *and the host `gsd-meta-manager` binary*
at identical absolute paths; put `CLAUDE_CONFIG_DIR` on a named volume seeded by a one-shot
`claude auth login --claudeai`; gate the run behind a pre-flight that names `claude auth status`,
the runtime identity, the network backend and the binary-runs-in-image probe by name; and state
the egress allowlist's real posture (enforced on docker, advisory on rootless podman) rather
than promising one the runtime cannot deliver.

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-22-01: Runtime identity is read from the response, never inferred from which
  constructor was called.** `bollard::connect_with_podman_defaults()` was *observed*
  returning a healthy handle to a **docker** daemon with no error and no warning
  (`22-SPIKE-OQ5.md` § F5, Trap 2). Identity comes from `version().components[].name` —
  `"Podman Engine"` vs `"Engine"` — or equivalently the `Libpod-Api-Version` header on
  `/_ping`. A run whose runtime identity cannot be read **refuses to start** rather than
  guessing. — *Reversibility: costly.*
- **D-22-02: `podman info`'s `RemoteSocket.exists` is never trusted.** Probe by connecting,
  not by asking.
- **D-22-03: Auto-detect means "no user-facing configuration", not "one argv for both".**
  Success criterion 1 is satisfied by the driver branching internally on a **verified**
  runtime, and the plan must not chase a single portable invocation.
- **D-22-04: The driven run stays a child process spawned through an argv prefix**
  (`docker run -i …` / `podman run -i …` in front of the existing `claude` argv). —
  *Reversibility: costly.*
- **D-22-05: Detection, inspect and lifecycle (stop/remove) go through the Docker-compat
  REST API via bollard.** The `ps --format json` normalization the ROADMAP budgeted for
  **is not written at all** — a deliberate deletion of planned work.
- **D-22-06: The API is also how a containerized run is actually stopped.** The container id
  is recorded on the run so a reattaching driver or the TUI can reach it.
- **D-22-07: The mount argv branches on the verified runtime, using the spike's measured
  recipe** (§ F2): rootless podman gets `--userns=keep-id`; rootful docker gets
  `--user $UID:$GID`.
- **D-22-08: The project root is mounted at the *identical absolute path* it has on the
  host.** "Path map swap" in the ROADMAP is therefore an identity map.
- **D-22-09: The host `~/.claude` is never bind-mounted.** Credentials live in a **named
  volume** owned by this tool. — *Reversibility: costly.*
- **D-22-10: `CLAUDE_CONFIG_DIR` is set to the same path the volume is mounted at.** One
  directory, one volume, one variable.
- **D-22-11: Proof that criterion 2 holds is inherited, not newly asserted.** Phase 15's
  capability gate already **refuses** a run whose `apiKeySource` is anything but `"none"`
  (`src/executor/gate.rs`, D-08).
- **D-22-12: First-time authentication is an explicit, separate, interactive one-shot** —
  never something a driven run performs. A driven run that finds the volume unauthenticated
  **refuses with a named remedy**.
- **D-22-13: Host prerequisites are checked and reported *by name*, up front**, in the same
  shape as Phase 15's capability gate — `pasta`/`passt` and `podman.socket` specifically.
- **D-22-14: The image pins the CLI version, disables the auto-updater, and runs as a
  non-root user, with egress restricted by an allowlist** (ROADMAP § Phase 22 risks, adopted
  verbatim).
- **D-22-15: `ExecutionTarget::Container` carries the verified runtime identity as a type
  that cannot be constructed without a successful probe** — the `DrivableProject` idiom. —
  *Reversibility: costly.*
- **D-22-16: The journal's `target` string stops being a `Debug` rendering.** Note the
  existing disagreement: fixtures at `src/journal/mod.rs:2989,3215,3261,3568` and
  `src/envelope/mod.rs:537` say `"host"` lowercase while the producer emits `"Host"`. —
  *Reversibility: one-way.*
- **D-22-17: A containerized run whose Phase 19 git envelope cannot be shown live inside the
  container must refuse to start.** The *mechanism* is left to research; the fail-closed
  posture is locked.
- **D-22-18: Target is a per-project setting with a per-run argv override, and the override
  wins.**
- **D-22-19: The TUI gains no second spawn path.** `tests/spawn_seam_guard.rs` must stay
  green unmodified; if it needs weakening, the design is wrong.

### Claude's Discretion

- Exact naming of the runtime-identity type, the label strings in D-22-16, the volume name,
  and the argv flag spelling for D-22-18.
- Whether bollard is added as a dependency or the API is spoken directly over the socket —
  though the spike drove bollard 0.21.1 against both runtimes unmodified (§ F4), so bollard is
  the evidenced default and the graph cost should be *measured* and recorded in `Cargo.toml` in
  this repository's established idiom (see the `sha2` and `rustix` comment blocks).
- Test decomposition, file layout, and wave/plan splitting.

### Deferred Ideas (OUT OF SCOPE)

- **Remote/CI execution targets.** The `ExecutionTarget` enum will accept a third variant; this
  phase adds exactly one.
- **A second `Executor` implementor.** Explicitly rejected by `src/executor/mod.rs:78`.
- **Publishing the container image as a distributed release artifact.**
- **`ps --format json` normalization.** Not deferred — **deleted** by D-22-05.
- All 14 todo matches reviewed, **zero folded**.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CTNR-01 | A user can run a driven project inside a container instead of on the host | § *The `ExecutionTarget::Container` shape*; § *Container argv recipe*; measured stdio-duplex proof that `podman run -i` / `docker run -i` carry the stream-json duplex intact |
| CTNR-02 | The container runtime is auto-detected; both docker and podman work without configuration | § *Runtime identity via bollard*; `connect_with_podman_defaults` fall-through **confirmed from bollard source**, not just observed; § *Question 3* names the network pre-flight; § *podman API socket activation* names a systemd-free path the spike did not find |
| CTNR-03 | Claude subscription authentication works inside the container without bind-mounting the host credential store | § *Question 1* — measured file layout under `CLAUDE_CONFIG_DIR`, measured `apiKeySource` for three auth channels, measured `claude auth status` detector |
| CTNR-04 | A user can start, stop, and resume containerized sessions from the TUI | § *Stop through the API*; `--name <run-id>` makes the container id knowable before spawn, so `run.json`'s exactly-twice write needs no third write |
| CTNR-05 | Container and host execution behave identically from the driver's perspective, including session resume | § *Question 2* (envelope parity is the real content of "identically"); § *Resume and the identity path map* — measured slug derivation proving D-22-08 |

</phase_requirements>

---

## Project Constraints (from CLAUDE.md)

| Directive | Where it binds this phase |
|-----------|---------------------------|
| **GSD workflow enforcement** — no direct repo edits outside a GSD command | Research wrote no source files; all experiments ran in the session scratchpad and `git status` is byte-identical to the pre-existing three entries |
| **Release process, step 2: `cargo update` at each milestone** | bollard's 25 new crates join that sweep; the `Cargo.toml` comment should say so, matching the `icu_properties` block's "refreshes under the release process's existing `cargo update`" sentence |
| **MSRV floor is measured, not chosen** (`rust-version = "1.88"`) | bollard 0.21.1's graph must be re-measured against the floor command in `Cargo.toml:13-14` after the add; if bollard or `icu_normalizer 2.3.0` declares a higher `rust-version`, the floor moves and the `msrv` CI job catches it |
| **Rust + ratatui stack; `tokio::sync::Mutex` in async code; no blocking in the render loop** | bollard is async/tokio-native; every API call lands in the driver process, never the TUI's render path |
| **"What NOT to use": no C FFI, no platform-specific deps** | bollard is pure Rust over a unix socket; `hyper-named-pipe` (Windows) rides in via the `pipe` feature but is inert under this repo's `#[cfg(unix)]` gating |
| **Technology stack table records version + why** | the Standard Stack table below follows that shape |

No `.claude/skills/` or `.agents/skills/` directory exists in this repository — checked.

---

# The Five Questions

## Question 1 — Which first-login mechanism satisfies CTNR-03, and what does it write?

**Verdict: `claude auth login --claudeai`, run once interactively in a one-shot container with
the named volume mounted at `CLAUDE_CONFIG_DIR`. `claude setup-token` is a viable fallback and
is NOT disqualified by `src/executor/gate.rs` — but it is the wrong default, for a reason that
is about this repository's own credential posture rather than about the gate.**

**Confidence: HIGH.** Every claim below except where marked was measured on this host against
`claude` **2.1.274**.

### 1a. `CLAUDE_CONFIG_DIR` relocates BOTH files — the two-file trap is closed by one variable

The ROADMAP's "two-file credential trap" (`~/.claude.json` lives outside `~/.claude`) is real
on a default install, and D-22-10's fix is correct. Measured directly:

```
$ env CLAUDE_CONFIG_DIR=<scratch>/cfgtest HOME=<scratch>/fakehome claude auth status
{ "loggedIn": false, "authMethod": "none", "apiProvider": "firstParty",
  "projectsDirectory": "<scratch>/cfgtest/projects",
  "configDirectory":   "<scratch>/cfgtest" }

$ find <scratch>/cfgtest
<scratch>/cfgtest/.claude.json                     <-- the "outside" file is INSIDE
<scratch>/cfgtest/backups/.claude.json.backup.…
$ find <scratch>/fakehome        # nothing at all
```

`[VERIFIED: measured 2026-09-16, claude 2.1.274]` — with `CLAUDE_CONFIG_DIR` set, `.claude.json`
is written **inside** that directory and `HOME` is not touched. One volume at one path covers
both files. D-22-09 + D-22-10 as written are sufficient; no second mount is needed for config.

### 1b. Session history ALSO lives inside `CLAUDE_CONFIG_DIR` — this settles the design fork

`auth status` reports `projectsDirectory = <CLAUDE_CONFIG_DIR>/projects`. On the authenticated
host config that is `/home/blk/.claude/projects`, and its entries are named by **the project's
absolute path with every `/` replaced by `-`**:

```
$ ls ~/.claude/projects/ | head
-home-blk/
-home-blk-projects-aconta-aiFlowAgent/
-home-blk-projects-rust-gsd-meta-manager/      <-- this repo
$ ls ~/.claude/projects/-home-blk-projects-rust-gsd-meta-manager/ | wc -l
34    # 34 session directories, plus 18 *.jsonl transcripts
```

`[VERIFIED: measured 2026-09-16]`

Two consequences the planner must carry:

1. **The named credential volume carries session history for free.** It is the same directory.
   The fork ("does the credential volume also need to carry session history for criterion 4's
   resume parity?") is settled: **yes, and no extra work is required** — mounting one volume at
   `CLAUDE_CONFIG_DIR` gets both. Do **not** add a second volume.
2. **This is the mechanical evidence for D-22-08.** The session lookup key is literally derived
   from the project's absolute path. Mount the project at a different path inside the container
   and `--resume` looks in a different directory that does not exist. The identity path map is
   not a preference; it is what makes the slug agree.

### 1c. The gate does NOT disqualify `setup-token` — measured, and this was the deciding question

`src/executor/gate.rs:81` pins `SUBSCRIPTION_API_KEY_SOURCE = "none"` and
`check_auth_path` (`:251-260`) refuses anything else. The question CONTEXT.md flagged as
deciding the answer is whether an OAuth-token-shaped auth reports something other than `"none"`.
Measured, three channels, same binary, same host, `system/init` captured off the real
stream-json wire:

| # | Environment | `apiKeySource` at `system/init` | Phase 15 gate |
|---|-------------|--------------------------------|---------------|
| A | real subscription (`.credentials.json`), no extra env | `"none"` | **passes** |
| B | `ANTHROPIC_API_KEY=<garbage>` | `"ANTHROPIC_API_KEY"` | **refuses** (`AuthPathChanged`) |
| C | `CLAUDE_CODE_OAUTH_TOKEN=<garbage>` (the `setup-token` channel) | `"none"` | **passes** |
| D | isolated, unauthenticated `CLAUDE_CONFIG_DIR` | `"none"` | **passes** ⚠ |

`[VERIFIED: measured 2026-09-16, claude 2.1.274]`

Corroborated from the CLI's own resolver, recovered from the installed binary:

```js
function uy(e={}){ … if(a.ANTHROPIC_API_KEY) return {key:…,source:"ANTHROPIC_API_KEY"};
                   if(dy())               return {key:…,source:"apiKeyHelper"};
                   … return {key:null,source:"none"}; }
```
`[VERIFIED: strings ~/.local/share/claude/versions/2.1.274]` — the `apiKeySource` enum is
`"ANTHROPIC_API_KEY" | "apiKeyHelper" | "none"`. `CLAUDE_CODE_OAUTH_TOKEN` appears in that
function only as an existence guard against throwing; it is an **auth token**, not an API key,
and never becomes a `source`. So `setup-token` is **not** disqualified. D-22-12's open question
is answered on that axis: the gate is not what decides it.

### 1d. Row D is the finding that actually matters — the gate is blind to an unseeded volume

An **unauthenticated** `CLAUDE_CONFIG_DIR` emits a perfectly normal `system/init` with
`apiKeySource: "none"`, sails through `validate_first_init`, and only then fails:

```
init  apiKeySource = 'none'
result is_error = True | "Not logged in · Please run /login"
```
`[VERIFIED: measured 2026-09-16]`

**D-22-11's inheritance is therefore correct but incomplete, and the plan must not lean on it
for the unseeded case.** The gate proves *"no API key snuck in"* (criterion 2's real content).
It does **not** prove *"the volume is authenticated"*. D-22-12's separate pre-flight refusal is
load-bearing, not belt-and-braces.

### 1e. The detector: `claude auth status`, which is JSON by default

```
$ claude auth status --help
Options:  --json  Output as JSON (default)   --text  Output as human-readable text
```

Authenticated (real config) vs unauthenticated (fresh dir), both measured:

```json
{ "loggedIn": true,  "authMethod": "claude.ai", "apiProvider": "firstParty",
  "configDirectory": "/home/blk/.claude", "projectsDirectory": "/home/blk/.claude/projects",
  "email": "…", "orgId": "…", "orgName": "…", "subscriptionType": "max" }

{ "loggedIn": false, "authMethod": "none", "apiProvider": "firstParty",
  "configDirectory": "<fresh>", "projectsDirectory": "<fresh>/projects" }
```
`[VERIFIED: measured 2026-09-16]`

**The pre-flight predicate:** run `claude auth status` *inside the container, with the volume
mounted and `CLAUDE_CONFIG_DIR` set*, and require `loggedIn == true`. Treat any other value —
including a body that will not parse — as a refusal. `authMethod == "claude.ai"` is the
subscription path and is worth asserting alongside; the full value domain of `authMethod` was
not measured (only `"claude.ai"` and `"none"` were observed), so a plan should refuse on
`loggedIn` and *report* `authMethod` rather than enumerate it. `[ASSUMED]` that other
`authMethod` values exist (e.g. for console/API-key logins).

### 1f. What the one-shot writes, and what the run detects

Measured key shapes of `<CLAUDE_CONFIG_DIR>/.credentials.json` (mode `0600`; **no secret values
are reproduced — key names and string lengths only**):

```
{ "claudeAiOauth": { "accessToken": str(108), "refreshToken": str(108),
                     "expiresAt": number, "refreshTokenExpiresAt": number,
                     "scopes": [6 entries], "subscriptionType": str, "rateLimitTier": str },
  "mcpOAuth": { … per-MCP-server entries, irrelevant here … } }
```
`[VERIFIED: measured 2026-09-16, shape only]`

And `<CLAUDE_CONFIG_DIR>/.claude.json` — 89 top-level keys on this host, of which the
auth-relevant ones are `oauthAccount` (an object carrying `accountUuid`, `emailAddress`,
`organizationUuid`, `subscriptionCreatedAt`, …), `userID`, `hasCompletedOnboarding: true`,
and `projects` (per-project state). `[VERIFIED: measured, key names only]`

**Do not detect authentication by sniffing for these files.** `auth status` is a supported,
machine-readable predicate that already accounts for token expiry; a file-existence check would
report "authenticated" for a volume whose refresh token expired months ago.

### 1g. Recommendation, and why `auth login` beats `setup-token` here

| | `claude auth login --claudeai` (one-shot container) | `claude setup-token` |
|---|---|---|
| Where the credential ends up | **inside the named volume** (`.credentials.json`) | a token string the human must then store and inject |
| Survives an image rebuild (criterion 2, half 2) | **yes** — it is volume state, not image state | only if the operator re-injects it every run |
| Reaches the run | already there; nothing passed | must ride `CLAUDE_CODE_OAUTH_TOKEN` — a long-lived secret **in the process table and in `-e` argv** |
| Consistency with this repo's own posture | matches D-17 (`src/envelope/cred.rs:700-709`): *"never an argv element, which is world-readable through the process table"* | **directly contradicts it** |
| Gate compatibility | passes (row A) | passes (row C) |
| Needs a browser on the host | yes | yes (the OAuth flow is the same) |

The gate does not decide this; **D-17 does**. `setup-token` puts a long-lived subscription
credential on the container-create argv of every run, which is precisely the channel
`src/envelope/cred.rs` spent a phase removing for the git token. Recommend `auth login` as the
documented one-shot and `setup-token` only as a named fallback for a host with no browser,
with the token supplied through a file descriptor or a mounted secret rather than `-e`.

**The exact one-shot command a human runs, once** (spelling is Claude's discretion; the shape
is not):

```bash
# podman (rootless) — interactive, TTY attached, the volume is created by this command
podman run --rm -it \
  --userns=keep-id \
  --network=slirp4netns \
  -v gsd-mm-claude-config:/claude-config \
  -e CLAUDE_CONFIG_DIR=/claude-config \
  <image> claude auth login --claudeai

# docker (rootful)
docker run --rm -it \
  --user "$(id -u):$(id -g)" \
  -v gsd-mm-claude-config:/claude-config \
  -e CLAUDE_CONFIG_DIR=/claude-config \
  <image> claude auth login --claudeai
```

`[ASSUMED]` that `auth login` completes inside a container without a host browser by printing a
URL for the human to paste — this was **not** exercised (completing it would mint a real
credential into a scratch volume). The planner should treat the one-shot's *interaction shape*
as the one open item from Q1 and verify it during execution; everything else here is measured.

---

## Question 2 — How the Phase 19 envelope is made live inside the container (D-22-17)

**Verdict: option (b) is FALSE and must not be used. Option (a) is the only true resolution, and
it is IN SCOPE for Phase 22 as approximately one plan — materially smaller than CONTEXT.md
feared, because the binary is bind-mounted rather than baked into the image.**

**Confidence: HIGH.** The mechanism was measured end to end with real `git` 2.53.0 and a real
container.

### 2a. What the envelope actually hands the child, and through which channel

Traced from source, this session:

| Artifact | Path (host) | Delivery channel | Survives `docker run` by default? |
|---|---|---|---|
| `pre-push`, `pre-commit` stubs | `<data_local>/gsd-meta-manager/envelope/<alias>/hooks/` | `core.hooksPath` via `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` **env** (`cred.rs:500-505`, `:751-753`) | **no** — env is not inherited by a container |
| generated `gitconfig` (identity only, no helper) | `<envelope>/gitconfig` | `GIT_CONFIG_GLOBAL` + `GIT_CONFIG_SYSTEM` **env** (`cred.rs:692-699`) | **no** |
| `askpass` responder stub | `<envelope>/askpass` | `GIT_ASKPASS` **env** (`cred.rs:706-709`) | **no** |
| `gh` config dir | `<envelope>/gh/` | `GH_CONFIG_DIR` **env** (`cred.rs:723-726`) | **no** |
| run-journal locator | project root | `GSD_MM_ENVELOPE_PROJECT_ROOT` **env** (`cred.rs:742-745`) | **no** |
| settings file (layer 2, the `PreToolUse` guard) | `<envelope>/settings.json` | `--settings <path>` on **argv** (`claude.rs:290-293`) | argv yes, **file no** |
| tool denylist (layer 1) | — | `--disallowedTools` on **argv** (`claude.rs:286-289`) | **yes** |
| `.git/info/exclude` block | inside the project | written on the host before spawn | **yes** (project is mounted) |

Every stub names `std::env::current_exe()` — a **host** path — baked in at generation time
(`hooks.rs:92-101`, `:196-204`; `cred.rs:626-628`, `:789-797`). `envelope_root()` is
`dirs::data_local_dir()/gsd-meta-manager/envelope/` (`envelope/mod.rs:12-14`), i.e.
`$XDG_DATA_HOME` or `$HOME/.local/share` — **resolved again inside the container** when the
binary re-enters as a hook, which is a second constraint nobody has named.

### 2b. What git actually does — four cases, measured

Real `git 2.53.0`, a real bare remote, `core.hooksPath` injected exactly the way
`cred::hooks_path_env` injects it:

| # | Configuration | Container analogue | Push result | Posture |
|---|---|---|---|---|
| 1 | `core.hooksPath` → directory that does not exist | envelope dir not mounted, env forwarded | **`* [new branch] HEAD -> c1`, exit 0, no warning** | **FAIL-OPEN, SILENT** |
| 2 | hooksPath dir exists, stub present, stub's binary missing | envelope dir mounted, binary absent from image | `exec: …/gsd-meta-manager: not found` → `error: failed to push some refs` | fail-closed, but every push dies |
| 3 | no `GIT_CONFIG_*` at all | **the default `docker run` / `podman run`** | **`* [new branch] HEAD -> c3`, exit 0** | **FAIL-OPEN, SILENT** |
| 4 | stub + binary present, hook exits 1 | correct in-container envelope | `error: failed to push some refs` | fail-closed, policy verdict |

`[VERIFIED: measured 2026-09-16, git 2.53.0]`

**Case 3 is the default.** Neither runtime propagates the client's environment into the
container. So a container target that does nothing special reaches case 3: `git push` inside the
container is completely unpoliced and git says nothing at all. D-22-17's hazard is real.

### 2c. One thing does fail loudly, and it narrows the silent window

`--settings` is argv, so it *is* forwarded — pointing at a host path that does not exist inside
the container. Measured at claude 2.1.274:

```
$ claude -p … --settings /nonexistent/x.json  < msg.json
Error: Settings file not found: /nonexistent/x.json
exit=1 ; stdout bytes: 0
```
`[VERIFIED: measured 2026-09-16]`

And the *other* failure mode, which is the one `write_settings_in`'s round-trip check exists for:

```
$ claude -p … --settings <file that exists but is invalid JSON>  < msg.json
exit=0 ; stderr empty ; run proceeds normally
```
`[VERIFIED: measured 2026-09-16]`

So **a missing settings file is loud (exit 1, nothing on stdout) while an invalid one is
silent.** This is good news for D-22-17: the naive container run does not silently lose the
envelope, it refuses to start — for a confusing reason, but it refuses. The genuinely silent
window is narrower and specific: **mount the envelope directory without shipping the binary,
and layer 2's guard hook execs a missing program while layer 3 lands in case 2.**

### 2d. Why option (b) is false — two independent grounds

1. **The egress allowlist premise does not hold on one of the two required runtimes.** Question
   5 measures that an enforced allowlist is unavailable on rootless podman without the
   privileged flags the spike's fence forbids. An argument that rests on it is an argument that
   is true on docker and false on podman, which is exactly the asymmetry criterion 1 forbids.
2. **A `git push` needs neither network nor credential.** Every one of the four probes above
   pushed to `../remote.git` — a **local path remote** — with no network, no credential helper,
   no ssh key and no token. `git push /some/path`, `git push ../worktree`, and a remote added
   inside the mounted project root are all reachable from inside a network-isolated container,
   and all four are pushes the `pre-push` namespace policy
   (`hooks.rs::classify_refs` → `policy::classify_push_ref`) exists to judge. The absence of a
   credential bounds *remote* pushes; it does not bound pushes.

Either ground alone kills (b). Both hold. **Do not take option (b).**

### 2e. Option (a), sized — and the cheap version of it

CONTEXT.md's option (a) says "ship the same-version `gsd-meta-manager` binary in the image".
**Bind-mount the host binary instead.** Measured, rootless podman, the binary mounted read-only
at its own absolute path:

| Base image | libc | Result |
|---|---|---|
| `alpine:3` | musl | `exec /home/blk/.cargo/bin/gsd-meta-manager: no such file or directory` — the missing-ELF-interpreter failure |
| `php:8.2-cli` (Debian trixie, glibc 2.41) | glibc | **ran**: `gsd-meta-manager 1.5.0` |

```bash
podman run --rm --network=none --userns=keep-id -e HOME=/tmp \
  -v /home/blk/.cargo/bin/gsd-meta-manager:/home/blk/.cargo/bin/gsd-meta-manager:ro \
  php:8.2-cli /home/blk/.cargo/bin/gsd-meta-manager --version
→ gsd-meta-manager 1.5.0
```
`[VERIFIED: measured 2026-09-16; host glibc 2.43, image glibc 2.41]`

Bind-mounting is strictly better than baking:

- **Version skew becomes unrepresentable.** `assert_provenance_in` (`hooks.rs:242-294`) compares
  *canonicalised paths*, not versions — a baked binary one release behind would certify itself
  happily while enforcing a different policy. A mount is the same inode.
- It removes the image from the release-coupling story entirely, which keeps the deferred item
  ("publishing the image as a distributed artifact") deferred.

Two measured constraints it imposes:

- **The base image must be glibc-based** (Debian/Ubuntu family), or the project must ship a
  `x86_64-unknown-linux-musl` static build. This is a real image-choice constraint and belongs
  in the plan, not in a comment.
- **The binary needs a writable `HOME`.** Without it, measured:
  `panicked at tracing-appender-0.2.5/src/rolling.rs:156: initializing rolling file appender
  failed: … PermissionDenied`. On the hook critical path that is a panic on every push — exit
  non-zero, so fail-closed, but an unusable envelope. `[VERIFIED: measured]`

### 2f. The concrete work, itemised

1. Mount `envelope_dir_in(root, alias)` read-write at the **identical absolute path**
   (the ledger is appended and the guard's protected-carrier rule assumes the run's uid can
   write there).
2. Mount the host `gsd-meta-manager` binary read-only at the **identical absolute path**
   `std::env::current_exe()` reports.
3. Pass `HOME` and `XDG_DATA_HOME` so `dirs::data_local_dir()` **inside** the container resolves
   to the same `<envelope>` parent the host used — otherwise the re-entered binary's
   `envelope_root()` disagrees with the path baked into the stub it is running as, and
   `assert_provenance` refuses every hook.
4. Translate the existing `EnvelopeEnv` value into `-e KEY=VALUE` flags **from the same value**
   the spawn closure applies, so `cred::build_env` stays the single producer (see
   § *The criterion-4 seam* below). The `None` (removal) entries are free: a container inherits
   nothing.
5. A **pre-flight probe that makes D-22-17 mechanical**: run
   `<runtime> run --rm <mounts> <image> <binary-path> --version` and require the output to equal
   the host binary's own `--version` before the real run is created. That single probe covers
   the musl case, the missing-mount case and the writable-`HOME` case at once, and it is the
   fail-closed check D-22-17 demands. Its refusal names the remedy: *"the image cannot execute
   the envelope binary; rebuild the image on a glibc base, or build a musl target"*.

**Scope call: IN SCOPE for Phase 22, one plan, roughly one wave.** No new module, no new spawn
seam, no envelope-policy change. It is two mounts, an env translation that reuses an existing
value, and one probe. The alternative — splitting it out — would ship a Phase 22 whose success
criterion 4 ("the driver's code path is identical to the host path apart from target selection")
is knowingly false, with the difference being a security boundary. That is the outcome the
inferred-decision audit flagged, and the evidence says close it here.

---

## Question 3 — Is `--network=slirp4netns` sufficient, or is `passt` a hard requirement?

**Verdict: `slirp4netns` is sufficient. `passt` is NOT a hard requirement for the driven run's
egress. The pre-flight should probe both by name and choose, with a documented fallback order —
`pasta` when present (podman's own default), `slirp4netns` explicitly when it is not, refusal
naming both when neither is.**

**Confidence: HIGH.** Measured against the real `api.anthropic.com` on this host.

### 3a. The host's state, and F1 reproduced

```
podman 5.7.0   docker 29.1.3
pasta        MISSING
passt        MISSING
slirp4netns  /usr/bin/slirp4netns   (version 1.3.3, libslirp 4.9.1)

$ podman run --rm alpine:3 true
Error: could not find pasta, the network namespace can't be configured:
       exec: "pasta": executable file not found in $PATH
```
`[VERIFIED: measured 2026-09-16 — reproduces 22-SPIKE-OQ5.md § F1 exactly]`

### 3b. `slirp4netns` works — DNS and real TLS egress

```
$ podman run --rm --network=slirp4netns alpine:3 sh -c 'cat /etc/resolv.conf; getent hosts api.anthropic.com'
search lan
nameserver 10.0.2.3          <-- slirp4netns' built-in DNS forwarder
nameserver 192.168.1.1
2607:6bc0::10   api.anthropic.com

$ podman run --rm --network=slirp4netns alpine:3 wget -q -O- https://api.anthropic.com/v1/messages
wget: server returned error: HTTP/1.1 405 Method Not Allowed
```
`[VERIFIED: measured 2026-09-16]` — a `405` on a `GET` to `/v1/messages` is the correct answer
from the real API, so the TLS handshake and the full request path completed. **This is the arm
the spike never exercised** (§ F1: *every probe ran `--network=none`).

### 3c. Not deprecated, not removed, no warning

`man podman-run` on 5.7.0 documents `slirp4netns[:OPTIONS,…]` as a first-class mode with its
own option set (`allow_host_loopback` — **default false**, `cidr`, `enable_ipv6`, `mtu`,
`outbound_addr`, `port_handler`). No deprecation text appears anywhere in the section, and
`podman run --network=slirp4netns … true` printed **nothing** on stderr.
`[VERIFIED: man podman-run, podman 5.7.0, measured 2026-09-16]`

`podman info` reports the backend availability directly and does not lie about it the way
`RemoteSocket.exists` does:

```
$ podman info --format '{{json .Host.Slirp4NetNS}}'
{"executable":"/usr/bin/slirp4netns","package":"slirp4netns_1.3.3-1_amd64","version":"…"}
$ podman info --format '{{json .Host.Pasta}}'
{"executable":"","package":"","version":""}
```
`[VERIFIED: measured]` — an **empty `executable`** is a truthful absence signal. Unlike Trap 1,
this field agrees with the filesystem.

### 3d. The cost of choosing `slirp4netns`, stated honestly

- **User-defined networks become unavailable.** Measured: `podman run --network=<custom>` fails
  with `setting up Pasta: could not find pasta` *and* warns `aardvark-dns binary not found,
  container dns will not be enabled` — rootless user-defined networks need **both** `passt` and
  `aardvark-dns`. This is what kills the `--internal`-network route to an enforced egress
  allowlist on podman (Question 5). `[VERIFIED: measured]`
- `slirp4netns` is the slower of the two and does not preserve source IP with the default
  `rootlesskit` port handler — irrelevant here, since a driven run publishes no ports.
- `allow_host_loopback` defaults to **false**, which is the posture this phase wants anyway: the
  container cannot reach services on the host's loopback.

### 3e. Recommendation for the pre-flight (D-22-13)

```
if runtime == Podman:
    pasta  = podman info → .Host.Pasta.executable         (non-empty?)
    slirp  = podman info → .Host.Slirp4NetNS.executable   (non-empty?)
    if pasta:  network flag = <omit; podman's own default is pasta>
    elif slirp: network flag = --network=slirp4netns      (+ report which was chosen)
    else: REFUSE — "no rootless network backend: install `passt` (preferred) or `slirp4netns`"
```

Do **not** hard-require `passt`. It is absent on this very host, the driven run works without
it, and a pre-flight that refuses a working configuration is the shape of control that gets
switched off. Do report which backend was chosen on the run record, because it is the one
observable that distinguishes "egress may be constrained" from "egress is wide open" (Q5).

---

## Question 4 — bollard's exact measured dependency-graph cost

**Verdict: 25 newly locked packages (354 → 379), ~+5 s on a clean release build of the
dependency set. Feature trimming buys essentially nothing — the unix-socket transport requires
the `pipe` feature and TLS is already off by default. Take the default features.**

**Confidence: HIGH.** Measured on a scratch copy of this repo's exact `Cargo.toml`/`Cargo.lock`
under the session scratchpad; the repository's tracked files were never modified
(`git status` verified identical before and after).

### 4a. Lockfile delta, by feature set

| Feature set | Locked packages added | Total (from 354) | `connect_with_socket` compiles? |
|---|---|---|---|
| `bollard = "0.21.1"` (default = `["http","pipe"]`) | **25** | 379 | **yes** |
| `default-features = false, features = ["pipe"]` | **25** | 379 | **yes** |
| `default-features = false, features = ["hyperlocal"]` | 24 | 378 | **no** — `error[E0599]: no associated function … connect_with_socket` |
| `default-features = false` | 21 | 375 | no (no unix transport at all) |

`[VERIFIED: measured 2026-09-16 via `cargo add` on a scratch copy + `grep -c '^\[\[package\]\]' Cargo.lock`]`

The reason trimming fails: bollard 0.21.1's `[features]` declares
`pipe = ["hyper-util", "hyperlocal", "hyper-named-pipe"]`, and the unix-socket constructors are
gated behind it. Asking for `hyperlocal` alone pulls the *dependency* without enabling the
*code*. Dropping `http` (leaving only `pipe`) changes the lock count by zero because `http`
resolves to `hyper-util`, which `pipe` already requires.
`[VERIFIED: bollard-0.21.1/Cargo.toml [features], read this session]`

**TLS is already absent.** `ssl`, `ssl_providerless`, `aws-lc-rs`, `buildkit`, `ssh` and
`websocket` are all non-default; `rustls`/`hyper-rustls` never enter the graph. There is nothing
to switch off. `[VERIFIED: bollard-0.21.1/Cargo.toml]`

### 4b. The 25 crates, named

```
atomic-waker 1.1.2        bollard 0.21.1            bollard-stubs 1.53.1-rc.29.3.1
form_urlencoded 1.2.2     http 1.5.0                http-body 1.1.0
http-body-util 0.1.5      httparse 1.10.1           httpdate 1.0.3
hyper 1.11.1              hyper-named-pipe 0.1.1    hyper-util 0.1.20
hyperlocal 0.9.1          icu_normalizer 2.3.0      icu_normalizer_data 2.3.0
idna 1.1.0                idna_adapter 1.2.2        percent-encoding 2.3.2
serde_repr 0.1.21         serde_urlencoded 0.7.1    tokio-util 0.7.19
tower-service 0.3.3       try-lock 0.2.5            url 2.5.8
want 0.3.1
```
`[VERIFIED: measured `cargo add` output, 2026-09-16]`

Two notes the `Cargo.toml` comment should carry, in the honest register the `sha2` block uses:

- **`tokio-util` enters the graph here.** `Cargo.toml:47-48` currently records a deliberate
  decision *not* to depend on it ("no line-framing crate is added … tokio-util is deliberately
  not a dependency"). bollard pulls it transitively. That comment is now **stale as written** and
  must be corrected rather than left to mislead: the *decision* (NDJSON framing does not use
  tokio-util) still stands, but the *fact* (tokio-util is not in the graph) does not. This is
  exactly the WR-02 failure mode `hooks.rs:156-168` is written about.
- **`icu_normalizer` 2.3.0 joins the `icu_*` family already reached through
  `icu_properties`.** Per `Cargo.toml:5-15` the MSRV floor is *derived* from the graph, and the
  `icu_* 2.3.x` crates are named as co-owners of the current 1.88 floor. Re-run the floor
  command after the add; if it moves, the manifest moves with it.

### 4c. Clean release build time

Measured on this host (16 cores, warm cargo registry), a stub crate carrying this repo's exact
dependency set, `rm -rf target` between runs:

| | run 1 | run 2 |
|---|---|---|
| without bollard (354 pkgs) | 18 s | 19 s |
| with bollard, default features (379 pkgs) | 24 s | 24 s |

**~+5 s, ~+27 %.** `[VERIFIED: measured 2026-09-16]` — the `rm -rf target` raced once in each
group, which can only *understate* the with-bollard figure, so treat +5 s as a floor and
approximate. This is a dependency-set build, not the full crate build; the repository's own
compile time is unaffected beyond the link step.

### 4d. The declined alternative (the idiom requires naming it)

**Declined: speaking the Docker REST API directly over the unix socket by hand** — either a
hand-rolled HTTP/1.1 client, or `hyper` + `hyperlocal` wired up in this tree. Declined for three
reasons, in the register the `sha2` block uses:

1. `hyper` + `hyperlocal` alone already cost most of the graph. The measurement above shows the
   transport crates (`hyper`, `hyper-util`, `hyperlocal`, `http`, `http-body`, `http-body-util`,
   `httparse`, `want`, `try-lock`, `atomic-waker`, `tower-service`) are 11 of the 25. bollard's
   own marginal cost over doing it by hand is roughly `bollard` + `bollard-stubs` +
   `serde_repr` + `serde_urlencoded` + `form_urlencoded` + `url` + `percent-encoding` + the two
   `idna` crates + the two `icu_normalizer` crates — and most of *that* is `url`, which any
   hand-rolled client would want anyway.
2. `bollard-stubs` is the generated Docker API model. Hand-rolling means hand-writing the
   `SystemVersion` / `ContainerSummary` / `ContainerInspectResponse` shapes and re-deriving them
   every time either runtime's API moves — for a project whose whole premise (§ F4) is that the
   two runtimes return *identical Docker-shaped payloads*. Hand-rolling would put this repository
   back in the business the spike just proved unnecessary.
3. **The spike drove bollard 0.21.1 against both runtimes unmodified** (§ F4). A hand-rolled
   client would be a re-derivation of a result that is already measured, with the measurement
   thrown away.

### 4e. The `Cargo.toml` entry, in this repository's idiom

```toml
# The Docker-compat REST API client, for runtime detection and container
# lifecycle (D-22-05). Not for the run itself: the driven agent is still a child
# process behind an argv prefix (D-22-04), which is what keeps Phase 15's
# framing, gate and interjection code untouched.
#
# Why a crate at all. `22-SPIKE-OQ5.md` § F4 measured this exact version driving
# a rootless podman socket and a rootful docker socket UNMODIFIED, returning
# identical Docker-shaped payloads from both — which is what deletes the
# `ps --format json` normalization layer (§ F3) rather than merely deferring it.
# The declined alternative was speaking the API by hand over `hyper` +
# `hyperlocal`. Declined because those two are 11 of the 25 crates below anyway,
# and the remainder is mostly `bollard-stubs` — the generated API model whose
# hand-written equivalent would have to be re-derived every time either runtime
# moves, for a boundary the spike proved is already identical.
#
# Graph cost, measured rather than borrowed: 25 newly locked packages, 354 -> 379
# (atomic-waker, bollard, bollard-stubs, form_urlencoded, http, http-body,
# http-body-util, httparse, httpdate, hyper, hyper-named-pipe, hyper-util,
# hyperlocal, icu_normalizer, icu_normalizer_data, idna, idna_adapter,
# percent-encoding, serde_repr, serde_urlencoded, tokio-util, tower-service,
# try-lock, url, want). Clean release build of the dependency set: ~19 s -> ~24 s
# on a 16-core host with a warm registry.
#
# DEFAULT FEATURES, DELIBERATELY. `default = ["http", "pipe"]`, and the unix
# socket transport lives behind `pipe` — measured: `default-features = false,
# features = ["hyperlocal"]` drops one crate and then fails to compile
# `Docker::connect_with_socket` at all. TLS is already out: `ssl`,
# `ssl_providerless`, `aws-lc-rs`, `buildkit`, `ssh` and `websocket` are all
# non-default, so no rustls enters the graph and there is nothing to switch off.
# `hyper-named-pipe` (Windows) rides in with `pipe` and is inert under this
# tree's `#[cfg(unix)]` gating.
#
# NOTE for the `tokio-util` comment above: bollard pulls `tokio-util 0.7.19`
# transitively. The DECISION recorded there still holds — NDJSON framing uses
# `tokio::io::BufReader::lines()` and an explicit byte bound, not tokio-util —
# but the sentence "tokio-util is deliberately not a dependency" became false
# with this add and is corrected rather than left standing.
bollard = "0.21.1"
```

---

## Question 5 — Is an egress allowlist enforceable on both runtimes without privileged flags?

**Verdict: NO. It is enforceable on rootful docker and NOT enforceable on rootless podman with
only `slirp4netns`. The honest posture is: enforced on docker, advisory on podman — and the
plan must say which one a given run got, on the run record.**

**Confidence: HIGH.** Both halves measured on this host, inside the spike's fence (no
`--privileged`, no `--cap-add`, no `sudo`).

### 5a. What is NOT achievable, measured

| Mechanism | Rootless podman (this host) | Why |
|---|---|---|
| `slirp4netns` destination filtering | **unavailable** | the mode's entire option set is `allow_host_loopback`, `cidr`, `enable_ipv6`, `mtu`, `outbound_addr`, `port_handler` — nothing filters a destination. `[VERIFIED: man podman-run 5.7.0]` |
| `pasta` destination filtering | **unavailable** | pasta's port options govern **inbound** forwarding; there is no outbound allowlist. `[ASSUMED]` — pasta is not installed here and was not exercised |
| `--internal` user-defined network | **unavailable** | measured: `Error: setting up Pasta: could not find pasta` *plus* `aardvark-dns binary not found` — rootless user-defined networks need `passt` **and** `aardvark-dns`, neither of which is installed. `[VERIFIED: measured]` |
| in-container `iptables`/`nftables` | **unavailable** | measured `CapEff: 00000000800405fb` inside a default rootless container — `CAP_NET_ADMIN` (bit 12, `0x1000`) is **not** set, and `iptables` is absent from the image. Granting it needs `--cap-add`, which the fence forbids. `[VERIFIED: measured]` |
| `--dns <resolver>` restriction | **advisory only** | a process that dials a literal IP never consults the resolver |
| `HTTPS_PROXY` alone | **advisory only** | under `slirp4netns` the container retains full direct outbound; nothing compels a process to use the proxy |

### 5b. What IS achievable on rootful docker, measured

```bash
docker network create --internal gsdq5-internal
# on the internal network:
docker run --rm --network=gsdq5-internal … 'getent hosts api.anthropic.com; wget https://api.anthropic.com/…'
→ DNS_FAIL
→ wget: bad address 'api.anthropic.com'
# control, default bridge:
docker run --rm … 'wget https://api.anthropic.com/v1/messages'
→ wget: server returned error: HTTP/1.1 405 Method Not Allowed
```
`[VERIFIED: measured 2026-09-16; network removed afterwards, `docker network ls` restored]`

`docker network create --internal` installs the blocking rules **in the daemon**, and the run
itself needs no `--privileged`, no `--cap-add` and no `sudo` — only docker-group membership,
which is the same privilege `docker run` already requires. A dual-homed egress proxy container
(internal + bridge) with `HTTPS_PROXY`/`NO_PROXY` pointed at it then gives a genuine
CONNECT-host allowlist. The CLI honours those variables: `HTTPS_PROXY` (85 occurrences),
`HTTP_PROXY`, `NO_PROXY`, `NODE_EXTRA_CA_CERTS`, `SSL_CERT_FILE` all appear in the installed
binary. `[VERIFIED: strings, claude 2.1.274]`

### 5c. The honest posture, and what the plan should promise

D-22-14 adopted "restrict egress with an allowlist" verbatim from the ROADMAP's risk list, and
the measurement says that promise is **not deliverable symmetrically**. Three options, with a
recommendation:

| Option | Assessment |
|---|---|
| Require `passt` + `aardvark-dns` so podman can do `--internal` too | pushes a host-package prerequisite onto every podman user for a control the *docker* arm gets free; and `--internal` under netavark was **not measured** here (pasta is absent), so it would be a promise made on `[ASSUMED]` evidence |
| Implement the proxy sidecar on docker and skip it on podman | criterion 1's "no configuration change" survives (the user configures nothing either way) but the *security posture* differs silently between hosts — the worst outcome |
| **Recommended: state the posture, make it observable, do not build the proxy in Phase 22** | the image ships with `HTTPS_PROXY`/`NO_PROXY` *honoured* and an allowlist documented as **advisory**, the pre-flight reports the network backend by name, and the run record carries `egress: "unrestricted(slirp4netns)"` / `"unrestricted(pasta)"` / `"proxied(<endpoint>)"`. An enforced allowlist becomes its own deferred item with the docker recipe above already measured. |

The recommendation follows this repository's own rule against unearned assurance: an allowlist
that half the hosts silently do not enforce is a control that reads as present and is not.
**This is the one place where research contradicts a locked decision, and it is raised in the
section below rather than planned around.**

Note the feedback into Question 2: this is *also* why option (b) fails. An argument from the
allowlist is an argument that is true on one runtime.

---

## Conflicts with locked decisions

Four items where measurement disagrees with an artifact. None is a request to reopen a decision;
each is a fact the plan needs so it does not encode something false.

### C-1 — D-22-14's egress allowlist is not unprivileged-enforceable on rootless podman

**Conflict: substantive.** See Question 5. `--internal` needs `passt` + `aardvark-dns`;
`CAP_NET_ADMIN` is not in the default capability set; `slirp4netns` has no destination filter.
The locked decision adopted the ROADMAP's wording verbatim, and the wording overpromises for one
of the two required runtimes. **Recommendation:** keep the decision's *intent* (egress is
restricted where it can be, the allowlist is written down, the proxy variables are honoured) and
change its *claim* from "restricted by an allowlist" to "restricted by an allowlist where the
runtime can enforce one, and the run records which it got". An enforced-on-both version is a
separate, sized piece of work with a measured docker recipe already in hand.

### C-2 — D-22-01's `Libpod-Api-Version` alternative is not reachable through bollard

**Conflict: mechanical, not substantive.** `Docker::ping()` in bollard 0.21.1 returns
`Result<String, Error>` — **the response body only, no headers**
(`bollard-0.21.1/src/system.rs:85`). The `Libpod-Api-Version` header *is* present on the wire
(measured: `Libpod-Api-Version: 5.7.0` on `GET /v1.41/_ping`), but bollard does not surface it.
`version().components[].name` is therefore the **only** identity route through the chosen
dependency, and the decision's "or equivalently" clause names a mechanism that would require
bypassing bollard. Note also that `SystemVersion.components` is
`Option<Vec<SystemVersionComponents>>` — an **absent** `Components` array must be a refusal, not
a fall-through to "assume docker".

### C-3 — the compile-error tripwire is ONE site, not two

CONTEXT.md (§ *Reusable Assets*, § D-22-15) says the enum "is matched exhaustively with no
wildcard at `src/executor/claude.rs:244` **and at the spawn closure**". Measured: the only match
on `options.target` in the whole tree is at `claude.rs:244`
(`grep -rn "options.target" src/` → `claude.rs:244`, `driver/run.rs:954`). The exhaustive match
*inside* the spawn closure (`claude.rs:511`) is on `options.profile`, not on the target.
**Consequence for the plan:** adding the `Container` variant produces exactly **one** compile
error. The mount/userns/`-e` work in the spawn closure at `:480-569` is **not** forced by the
compiler and must be routed deliberately, or it will be silently skipped. Consider adding a
second exhaustive `match &target` inside the closure in the same commit, purely to restore the
tripwire the artifacts believed was already there.

### C-4 — D-22-13's "there is no API socket until `systemctl --user start podman.socket`"

**Conflict: the spike's Trap-1 conclusion is too strong.** The spike observed
`podman system service unix:///run/user/1000/podman/podman.sock` failing with
`bind: no such file or directory` and concluded podman "will not create it". The real cause is
the missing **parent directory**, which is an ordinary user-writable path under
`$XDG_RUNTIME_DIR`. Measured this session:

```
$ mkdir -p /run/user/1000/podman
$ podman system service --time=10 unix:///run/user/1000/podman/podman.sock &
$ curl -s --unix-socket /run/user/1000/podman/podman.sock -D - http://d/v1.41/_ping
HTTP/1.1 200 OK
Api-Version: 1.41
Libpod-Api-Version: 5.7.0
$ curl -s --unix-socket … http://d/v1.41/version
Version 5.7.0  Api 1.41  Components ['Podman Engine', 'Conmon', 'OCI Runtime (runc)']
```
`[VERIFIED: measured 2026-09-16; directory removed afterwards, `podman.socket` left inactive/disabled]`

So a socket can be brought up with **no systemd, no sudo, no privilege**. The decision's posture
(report the prerequisite by name) is still right — see the recommendation in
§ *podman API socket activation* below — but the fact that only systemd can do it is false.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Runtime detection + identity verification | driver process (`executor::container`, new) | — | must happen before any container is created; the result is the capability token |
| Container create + the driven run | driver process, through the **existing** `CommandWrap` spawn at `claude.rs:480` | container runtime | D-22-04; argv prefix keeps the stream-json duplex on `ChildStdin`/`ChildStdout` |
| Container stop / inspect | driver process + TUI, through bollard over the runtime socket | — | D-22-06; a process-group kill reaches the client, not the container |
| Credential storage | **container runtime named volume** | — | D-22-09; never the host `~/.claude` and never the image |
| Session history (`--resume`) | **the same named volume** | — | measured: `<CLAUDE_CONFIG_DIR>/projects/<slug>/`; not a separate tier |
| Envelope policy enforcement | **inside the container**, via the host binary bind-mounted at its own path | host driver (generation) | Q2; generation stays on the host, enforcement must be in the container |
| Target selection (per-project + per-run override) | `config.rs` registry entry + `cli.rs`/`driver::DriveArgs` | TUI | D-22-18/D-22-19; the TUI reaches it through the one existing spawn seam |
| Egress restriction | container runtime (docker) / **not enforceable** (rootless podman) | the CLI's `HTTPS_PROXY` handling | Q5 |
| Journal `target` rendering | `journal::RunStarted` + `RunRecord` producers | — | D-22-16; a durable value, so one explicit producer |

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---|---|---|---|
| `bollard` | 0.21.1 | Docker-compat REST API: runtime identity, inspect, stop/remove | measured driving **both** runtimes unmodified (`22-SPIKE-OQ5.md` § F4); the spike's own version, so no re-derivation |

### Supporting — already in the tree, no new dependency

| Library | Version | Purpose in this phase |
|---|---|---|
| `process-wrap` | 10.0.0 (`tokio1`) | unchanged — the `docker`/`podman` client becomes the process-group leader instead of `claude` |
| `serde` / `serde_json` | 1 | the container id, the rendered target label, the pre-flight report |
| `tokio` | 1 (full) | bollard is async and lands in the driver's existing runtime |
| `tempfile`, `rustix` | — | unchanged |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|---|---|---|
| bollard | hand-rolled HTTP over `hyperlocal` | 11 of the 25 crates are the transport anyway; loses the generated API model. Declined — see § 4d |
| bollard | shelling out to `docker`/`podman` CLI | re-introduces § F3's field-name divergence that § F4 deletes; adds a second process-spawn site the seam guard would have to allowlist |
| `--network=slirp4netns` | require `passt` | `passt` is absent on this host and the run works without it; a pre-flight that refuses a working config is a control that gets switched off |
| bind-mounting the host binary | baking it into the image | `assert_provenance` compares paths, not versions — a baked binary one release behind certifies itself. See § 2e |
| `claude auth login` one-shot | `claude setup-token` | `setup-token`'s token must ride `-e` on every run, directly against `cred.rs`'s D-17 posture. See § 1g |

**Installation:**
```bash
cargo add bollard@0.21.1
```

### Version verification

```
$ cargo search bollard    # and the vendored manifest read this session
bollard = "0.21.1"        # resolved and locked on a scratch copy, 2026-09-16
```
`[VERIFIED: cargo add resolved 0.21.1 from the crates.io index, 2026-09-16]`

---

## Package Legitimacy Audit

Only one package is added by this phase.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---|---|---|---|---|---|---|
| `bollard` 0.21.1 | crates.io | mature (0.x series since 2018) | high (a top-tier Docker client for Rust) | `github.com/fussybeaver/bollard` | **OK** | Approved |

Evidence for the verdict, in preference order:

1. **Exercised against a real runtime in this project's own spike.** `22-SPIKE-OQ5.md` § F4
   records bollard 0.21.1 driving both a podman socket and a docker socket, returning
   `Version="5.7.0"`/`Version="29.1.3"`. This is stronger than any registry signal: the artifact
   was *run*. `[VERIFIED: 22-SPIKE-OQ5.md § F4]`
2. **Source read this session.** `~/.cargo/registry/src/…/bollard-0.21.1/src/` — 21 modules,
   `docker.rs` 1000+ lines with documented constructors, a generated `bollard-stubs` model
   crate. Not a stub or a squat. `[VERIFIED: source read 2026-09-16]`
3. Resolved and locked by `cargo add` from the crates.io index on a scratch copy, compiling
   cleanly against this repo's exact dependency set. `[VERIFIED: measured]`

**Packages removed due to [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

Two package *names* this phase depends on outside Cargo, which are **not** Rust packages and
must not be conflated with one:

- `passt` (Debian/Ubuntu package name; the binary is `pasta`) — `[VERIFIED: podman 5.7.0's own
  error message names it, and `podman info .Host.Pasta.package` is the field that would report
  it]`. Named in a refusal, never installed by this tool.
- `slirp4netns` — `[VERIFIED: present on this host at /usr/bin/slirp4netns, version 1.3.3,
  package `slirp4netns_1.3.3-1_amd64` per `podman info`]`.

---

## Architecture Patterns

### System Architecture Diagram

```
                          ┌──────────────────────── HOST ────────────────────────┐
  user (TUI)              │                                                       │
      │                   │  App::start_driver_run ──► driver::spawn::spawn_detached
      │ Action::Driver…   │            (the ONE spawn seam — unchanged, D-22-19)   │
      ▼                   │                          │                            │
 ┌─────────┐              │                          ▼                            │
 │  TUI    │              │              gsd-meta-manager drive <alias>            │
 │ process │              │                          │                            │
 └────┬────┘              │                          ▼                            │
      │ reads only        │              driver::run::execute_run                  │
      │ journal.jsonl     │                  │              │                      │
      │ (D-03)            │   establish_envelope      target selection             │
      ▼                   │   (host-side, writes       (registry entry             │
 .planning/meta-manager/  │    stubs + settings        ∥ --target argv,            │
   runs/<id>/*.json       │    under <envelope>)       argv wins — D-22-18)        │
      ▲                   │           │                       │                    │
      │                   │           │            ┌──────────┴──────────┐         │
      │                   │           │            ▼                     ▼         │
      │                   │           │       Host target          Container target │
      │                   │           │       (unchanged)                 │         │
      │                   │           │                                   ▼         │
      │                   │           │                    ┌──── PRE-FLIGHT ────┐   │
      │                   │           │                    │ 1 connect socket   │   │
      │                   │           │                    │ 2 version()        │───┼──► runtime
      │                   │           │                    │   components[].name│   │    socket
      │                   │           │                    │ 3 network backend  │   │  (bollard)
      │                   │           │                    │ 4 binary-in-image  │   │
      │                   │           │                    │ 5 auth status      │   │
      │                   │           │                    └─────────┬──────────┘   │
      │                   │           │                              │ VerifiedRuntime
      │                   │           │                              ▼               │
      │                   │           └────────────► build_argv(target) ────────────┐│
      │                   │                                         │               ││
      │                   │                    CommandWrap::with_new(program, …)    ││
      │                   │                    program = "podman" | "docker"        ││
      │                   │                    leading = [run -i --rm --name <id> …]││
      │                   │                              │ stdin/stdout piped       ││
      └───────────────────┼──────────────────────────────┼──────────────────────────┘│
                          └──────────────────────────────┼───────────────────────────┘
                                                         ▼
       ┌─────────────────────────── CONTAINER ───────────────────────────────┐
       │  claude -p --input-format stream-json --output-format stream-json …  │
       │        ▲ stdin        │ stdout (NDJSON)                              │
       │        │              ▼                                              │
       │  mounts, all at IDENTICAL absolute paths:                            │
       │   • <project root>        rw   ← --resume slug + journal/inbox agree  │
       │   • <envelope dir>        rw   ← hooks, settings, ledger, gitconfig   │
       │   • <host binary>         ro   ← what every stub execs (Q2)           │
       │  volume:                                                             │
       │   • <named volume> → CLAUDE_CONFIG_DIR  ← creds AND session history   │
       │  env: the EnvelopeEnv value, translated to -e from the SAME producer  │
       │  egress: pasta | slirp4netns  (podman) · bridge | --internal (docker) │
       └──────────────────────────────────────────────────────────────────────┘
```

### Recommended structure

```
src/executor/
├── mod.rs           # ExecutionTarget gains Container { … }  (the 1-line variant)
├── claude.rs        # build_argv gains the Container arm; spawn closure gains -e translation
├── gate.rs          # unchanged — D-22-11's apiKeySource refusal is inherited
└── container/       # NEW
    ├── mod.rs       # VerifiedRuntime (the D-22-15 capability token) + its ONE constructor
    ├── detect.rs    # bollard connect-by-path, version().components, the refusals
    ├── argv.rs      # the F2 mount/userns recipe, the -e translation, --name <run-id>
    ├── preflight.rs # the five named checks, in the GateOutcome shape
    └── lifecycle.rs # stop_container / remove_container by name (D-22-06)
```

### Pattern 1: `VerifiedRuntime` — the D-22-15 capability token

Modelled exactly on `DrivableProject` (`src/executor/mod.rs:134-257`): private fields, one
production constructor, one `#[doc(hidden)]` test hatch, and `tests/spawn_seam_guard.rs`'s
`drivable_project_has_exactly_two_constructors_and_private_fields` extended to cover it.

```rust
/// Proof that a container runtime was identified from its own response.
///
/// The D-22-15 capability token, and it is a *type* on purpose: the compiler,
/// not a code review, is what makes an unverified `Container` target
/// unrepresentable. `22-SPIKE-OQ5.md` § F5 Trap 2 measured
/// `bollard::connect_with_podman_defaults()` returning a healthy handle to a
/// **docker** daemon with no error and no warning; a design that infers the
/// runtime from which constructor was called emits `--userns=keep-id` at docker
/// (rejected, spike row H) or writes root-owned files into the user's project
/// tree (spike row F).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRuntime {
    kind: RuntimeKind,     // Podman | Docker
    socket: PathBuf,       // the path that was CONNECTED to, never the one info claimed
    server_version: String,
    api_version: String,
}

impl VerifiedRuntime {
    /// The **only production constructor**. Connects, asks, and refuses.
    pub async fn probe(socket: &Path) -> Result<VerifiedRuntime, RuntimeProbeError> {
        // `connect_with_socket` checks the file exists and returns
        // `SocketNotFoundError` — which is how D-22-02 is satisfied without ever
        // reading `podman info`'s RemoteSocket.exists.
        let docker = bollard::Docker::connect_with_socket(
            socket.to_str().ok_or(RuntimeProbeError::SocketPathNotUtf8)?,
            PROBE_TIMEOUT_SECS,
            bollard::API_DEFAULT_VERSION,
        )?;
        let version = docker.version().await?;

        // An ABSENT Components array is a refusal, not a fall-through to Docker.
        // `SystemVersion.components` is `Option<Vec<SystemVersionComponents>>`.
        let components = version.components.ok_or(RuntimeProbeError::IdentityUnreadable {
            socket: socket.to_path_buf(),
        })?;
        let kind = match components.iter().map(|c| c.name.as_str()).find(|name| {
            *name == PODMAN_ENGINE || *name == DOCKER_ENGINE
        }) {
            Some(PODMAN_ENGINE) => RuntimeKind::Podman,
            Some(DOCKER_ENGINE) => RuntimeKind::Docker,
            _ => return Err(RuntimeProbeError::UnknownEngine {
                observed: components.iter().map(|c| c.name.clone()).collect(),
            }),
        };
        …
    }
}

/// Measured on this host, 2026-09-16:
///   podman 5.7.0  → ["Podman Engine", "Conmon", "OCI Runtime (runc)"]
///   docker 29.1.3 → ["Engine", "containerd", "runc", "docker-init"]
const PODMAN_ENGINE: &str = "Podman Engine";
const DOCKER_ENGINE: &str = "Engine";
```

**Do not call `connect_with_podman_defaults()` at all.** Its fall-through is not merely observed
— it is in the source:

```rust
// bollard-0.21.1/src/docker.rs:1000-1022, read 2026-09-16
#[cfg(unix)]
pub fn connect_with_podman_defaults() -> Result<Docker, Error> {
    if let Some(host) = env::var("DOCKER_HOST").ok().filter(|p| p.starts_with("unix://")) { … }
    if let Some(sock) = Self::podman_rootless_socket_path() { … }
    if let Some(sock) = Self::podman_system_socket_path()   { … }
    Docker::connect_with_unix(DEFAULT_SOCKET, DEFAULT_TIMEOUT, API_DEFAULT_VERSION)  // ← docker
}
```
`[VERIFIED: bollard-0.21.1 source, read 2026-09-16]` — enumerate the candidate socket paths in
*this* codebase and call `connect_with_socket` on each, so the path that answered is a recorded
fact rather than a library's private preference.

### Pattern 2: the container argv recipe (spike § F2, cited by row)

| Runtime | Mount/uid flags | Spike evidence |
|---|---|---|
| rootless podman | `--userns=keep-id` | § F2 row **D** — keep-id alone remaps the image's non-root default user to host uid 1000 and files land `1000:1000`. `--user` is redundant beside it (row C) |
| rootless podman | **never** `--user $UID:$GID` alone | § F2 row **B** — `Permission denied` writing to a `1000:1000` mount |
| rootful docker | `--user "$(id -u):$(id -g)"` | § F2 row **G**; without it row **F** writes **root-owned** files into the user's project tree |
| rootful docker | **never** `--userns=keep-id` | § F2 row **H** — `docker: --userns: invalid USER mode` |

The full prefix the planner should name (flag list; exact spelling is discretion):

```
<runtime> run
  -i                                   # stdin open — the stream-json duplex (measured: works on both)
  --rm
  --name <run-id>                      # so the container id is known BEFORE spawn (see below)
  --init                               # reap zombies; claude spawns tool subprocesses
  -w <project-root-absolute>           # identical to the host cwd (D-22-08)
  -v <project-root>:<project-root>     # identical absolute path
  -v <envelope-dir>:<envelope-dir>     # Q2
  -v <host-binary>:<host-binary>:ro    # Q2
  -v <named-volume>:<config-dir>       # D-22-09 / D-22-10
  -e CLAUDE_CONFIG_DIR=<config-dir>
  -e HOME=<writable>  -e XDG_DATA_HOME=<so data_local_dir agrees>   # Q2 step 3
  -e <each EnvelopeEnv Some(..) entry>                              # Q2 step 4
  --userns=keep-id            (podman)   |   --user $UID:$GID   (docker)
  --network=slirp4netns       (podman, only when pasta is absent — Q3)
  <image>
  claude                                 # then build_argv(options) is appended by existing code
```

**Deliberately NOT in the list:** `-t` (there is no TTY; the detached driver's stdio is null and
a pty would corrupt the NDJSON framing), `--privileged`, any `--cap-add`, `--network=host`
(it would undo every isolation claim), and `:z`/`:Z` SELinux relabels (this host is AppArmor;
the spike did not exercise them — `[ASSUMED]` that an SELinux host needs `:Z`, and the plan
should surface that as a documented limitation rather than guess a flag into the recipe).

**`--name <run-id>` is the load-bearing small choice.** `RunRecord` is written *exactly twice*
(`driver/run.rs:931-933`), so a container id discovered after `docker run` returns would need a
third write. Naming the container after the run id makes it deterministic and known before
spawn: `stop_container("<run-id>")` works from the TUI, from a reattaching driver, and from
`driver::reconcile`, with no extra state. The run id is already validated as a plain path
component, so it is a safe container name.

### Pattern 3: the criterion-4 seam — where "identical code path" gets subtle

`claude.rs:480-569` carries an explicit claim: *"This closure is the ONE place in the tree that
builds the child's environment."* For a container target that sentence quietly changes meaning —
the closure builds the **client's** environment, and the client does not pass it on.

Recommended resolution, so the claim stays true rather than being weakened:

- **Leave the closure exactly as it is.** Setting `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`,
  scrubbing `CLAUDE*`, and applying `EnvelopeEnv` to the `docker`/`podman` client is harmless
  and keeps the host and container paths textually identical.
- **Derive the `-e` flags from the same `EnvelopeEnv` value** in the prefix builder — one
  producer, two consumers, which is the shape `cred.rs:748-753` already uses for the
  `GIT_CONFIG_COUNT` derivation ("adding a key here cannot silently drop the whole injection the
  way a hardcoded count would").
- **Amend the closure's doc comment in the same commit**, naming the container case explicitly.
  A comment that became false while the code stayed correct is `hooks.rs:156-168`'s recorded
  failure mode, and this phase should not add a second instance of it.
- The `None` (removal) entries — `SSH_AUTH_SOCK`, `SSH_AGENT_PID` — need no translation. A
  container inherits nothing, which makes the removal free and stronger than it is on the host.

### Pattern 4: where the container prefix attaches

`start_run` already composes exactly the right shape (`claude.rs:469-470`):

```rust
let mut argv = self.leading_args.clone();
argv.extend(build_argv(&options));
```

with `program` and `leading_args` as fields of `ClaudeExecutor`. So the container prefix is
`program = "podman"` (or `"docker"`, a **literal** chosen by the verified `RuntimeKind`, never a
user-supplied string) and `leading_args = [run, -i, …, <image>, claude]`.

**But do not reach for `ClaudeExecutor::with_program`.** Its doc (`claude.rs:377-390`) says it
exists so the transcript-replaying stand-in stays "out of the production path entirely — no
Cargo feature, no fixture branch in `main`". Using it in production makes that sentence false.
Add a named production constructor instead — e.g. `ClaudeExecutor::for_target(&ExecutionTarget)`
— so the two intents stay distinguishable in the source.

Note what the seam guard does and does not police: `AGENT_OVERRIDE_FIELDS` is
`["claude_program:", "claude_args:"]` and `OVERRIDE_PARSER_HOME` is `src/cli.rs`
(`tests/spawn_seam_guard.rs:150-157`). A new `target:` or `container_image:` field is **not**
matched, so `the_agent_program_override_fields_are_debug_only` will not fire. That is a gap
worth closing deliberately: the reason those fields are debug-gated is
*"a release build accepts a flag that makes the driver exec an arbitrary program"*, and an image
name is one `--entrypoint`-shaped mistake away from the same hazard. Recommend that the image
name be validated (non-empty, no leading `-`, no path traversal) at
`driver::DriveArgs::from_argv` — the repository's stated home for payload validation
(`src/cli.rs:74-85`) — rather than in a clap `value_parser`.

### Pattern 5: podman API socket activation and the pre-flight probe (D-22-02, D-22-13)

Candidate socket paths, probed **by connecting**, in order:

1. `$DOCKER_HOST` when it starts with `unix://`
2. `$XDG_RUNTIME_DIR/podman/podman.sock`
3. `/run/user/<uid>/podman/podman.sock`
4. `/run/podman/podman.sock`
5. `/var/run/docker.sock`

For each: `Docker::connect_with_socket(path, …)` (which returns `SocketNotFoundError` when the
file is absent — no `podman info` involved, satisfying D-22-02) then `version()`, then read
`components[].name`. **Record which path answered**, because with a docker socket at position 5
a "podman" run and a "docker" run are the same probe with different answers.

Measured facts for the refusal text:

```
$ podman info --format '{{json .Host.RemoteSocket}}'
{"path":"/run/user/1000/podman/podman.sock","exists":true}        ← claims true
$ ls /run/user/1000/podman/podman.sock
No such file or directory                                         ← and the PARENT is missing too
$ systemctl --user is-active podman.socket  →  inactive
$ systemctl --user is-enabled podman.socket →  disabled
```
`[VERIFIED: measured 2026-09-16 — Trap 1 reproduced]`

**Remedy to name, in order of preference:**

1. `systemctl --user start podman.socket` — the supported path; systemd creates the parent
   directory and manages the lifetime. *(Also worth naming `systemctl --user enable --now
   podman.socket` for a user who wants it to persist.)*
2. Documented fallback for a host with no systemd user session:
   `mkdir -p "$XDG_RUNTIME_DIR/podman" && podman system service --time=0 unix://$XDG_RUNTIME_DIR/podman/podman.sock &`
   — measured working, unprivileged (C-4). **Recommend the plan NAME this and decline to
   automate it**: spawning a daemon would add a process-spawn site that
   `every_process_spawn_site_in_src_is_on_the_allowlist` would have to be widened for, and
   widening a spawn allowlist to start a background service is exactly the kind of quiet
   meaning-change that test exists to prevent.

### Pattern 6: resume, and why the identity path map is not negotiable

`--resume <session-id>` scopes its lookup to the project directory. Measured mechanism:
`<CLAUDE_CONFIG_DIR>/projects/<absolute-path-with-slashes-as-dashes>/` (§ 1b). Therefore:

- project mounted at `/home/blk/projects/rust/gsd-meta-manager` → slug
  `-home-blk-projects-rust-gsd-meta-manager` → sessions found.
- project mounted at `/workspace` → slug `-workspace` → **a different, empty directory**. Resume
  silently starts a new session instead of resuming.

That failure is silent, which is why D-22-08 is a decision and not an optimisation. The same
identity also makes the journal, inbox and lock paths the host driver writes agree with what the
container-side agent sees — the D-03 filesystem boundary survives the new process boundary
unchanged, with no mapping table.

`build_argv` at `claude.rs:267-272` already pushes `--resume <value>` with an explicit value
("a bare `-r` opens an interactive picker, which under `-p` with no TTY is at best an error").
No change is needed there for the container target — which is itself evidence for criterion 4.

### Pattern 7: the image

| Concern | Recommendation | Evidence |
|---|---|---|
| Base image | **glibc** — `debian:trixie-slim` or `ubuntu:24.04` | measured: the host binary runs on Debian trixie glibc 2.41 and **fails on alpine/musl** with `no such file or directory` (§ 2e). Alpine is disqualified unless the project ships a musl-static build |
| Pin the CLI version | install a named version, never `latest`. Two supported channels: `claude install <version>` (native build → `~/.local/share/claude/versions/<version>`), or the npm package pinned exactly | `[VERIFIED: claude install --help — "Use [target] to specify version (stable, latest, or specific version)"]` |
| Which version | ≥ `MINIMUM_CLAUDE_VERSION` = **2.1.214** (`gate.rs:68`). Above `TESTED_MAXIMUM_CLAUDE_VERSION` = **2.1.220** (`gate.rs:75`) the gate *warns and proceeds*. This host runs 2.1.274 and every measurement above is from it | `[VERIFIED: gate.rs read; claude --version measured]` |
| Disable the auto-updater | **both carriers**, matching this repo's own D-07 "never the sole carrier" rule: env `DISABLE_AUTOUPDATER=1` in the image, **and** `autoUpdates: false` seeded in `<CLAUDE_CONFIG_DIR>/.claude.json` | `[VERIFIED: both names present in the 2.1.274 binary; its own text reads "`autoUpdates` is `false` in `~/.claude.json` or `DISABLE_AUTOUPDAT[ER]`"]` |
| Non-root user | a fixed non-root uid in the image. This is **why** `--userns=keep-id` matters: spike § F2 row D shows keep-id remaps a non-root image user to host uid 1000 so mounted writes stay host-owned, while rootless podman's *default* (row A) is correct only for a root-default image | `[VERIFIED: 22-SPIKE-OQ5.md § F2 rows A and D]` |
| Writable `HOME` | the image's non-root user needs a writable home, or the bind-mounted `gsd-meta-manager` panics in `tracing-appender` on the hook critical path | `[VERIFIED: measured, § 2e]` |
| Non-essential traffic | consider `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` to narrow what the allowlist has to permit | `[VERIFIED: name present in the binary]`; its exact effect `[ASSUMED]` |

### Anti-Patterns to Avoid

- **Inferring the runtime from `connect_with_podman_defaults()` succeeding.** It falls through to
  the docker socket, in source (§ Pattern 1).
- **Reading `podman info .Host.RemoteSocket.exists`.** It said `true` while both the socket and
  its parent directory were absent (measured).
- **Translating the project path.** Silently breaks `--resume` via the slug (§ Pattern 6).
- **`--network=host`.** Undoes every isolation claim the phase makes and would put the container
  on the host's loopback.
- **Mounting `~/.claude`.** Criterion 2 forbids it outright (D-22-09).
- **Detecting authentication by looking for `.credentials.json`.** The file is present and the
  token can be expired; `claude auth status` already answers the real question.
- **Mounting the envelope directory without the binary.** The one genuinely silent state — layer
  2's guard execs a missing program while layer 3 lands in git case 2.
- **Promising an enforced egress allowlist on both runtimes.** Not deliverable on rootless
  podman (Q5/C-1).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---|---|---|---|
| Talking to the runtime | an HTTP/1.1-over-unix-socket client | `bollard` 0.21.1 | measured driving both runtimes unmodified (§ F4); 11 of its 25 crates are the transport you would need anyway |
| Normalising `docker ps` vs `podman ps` | a field-name/type shim (`Id` vs `ID`, array vs string) | **nothing — the API erases it** (§ F4, D-22-05) | this is planned work that gets *deleted*; § F3 documents it precisely so nobody re-adds it |
| Deciding "is this volume authenticated" | parsing `.credentials.json` and checking `expiresAt` | `claude auth status` (JSON by default) | the CLI already owns token refresh and expiry; a hand-rolled check reports "authenticated" for an expired refresh token |
| Deciding "which auth path is live" | a second `apiKeySource` check | `src/executor/gate.rs::check_auth_path` (D-22-11) | it already exists and refuses; a parallel check drifts |
| Mapping a project path to its session directory | a slug function | mount at the identical path so no mapping exists | the slug is the CLI's private detail and can change; identity is version-independent |
| Enforcing the push boundary in the container | a container-local hook script | the same binary, bind-mounted, through the same stubs | `hooks.rs:1-18`: *"A shell script full of `case` statements is precisely the artifact an agent rewrites"* |
| Bringing up the podman API socket | spawning `podman system service` from the driver | name `systemctl --user start podman.socket` as the remedy | a new spawn site would widen a security allowlist for a convenience |

**Key insight:** this phase's biggest "don't hand-roll" is not a library choice — it is
**deleting the `ps --format json` normalization the ROADMAP budgeted for**. § F3 measured it as
real work, § F4 measured it away. The plan should carry § F3's table only as the explanation for
why the code does not exist.

---

## Runtime State Inventory

This phase changes a **durable on-disk record** (D-22-16) and introduces new runtime state, so
the inventory applies.

| Category | Items Found | Action Required |
|---|---|---|
| **Stored data** | `run.json` `target: String` and `journal.jsonl` `RunStarted.target` — currently `format!("{:?}", …)` producing `"Host"`. Records already on disk carry `"Host"`; `driver::reconcile` reads them back. **Five in-tree sites disagree with the producer and say `"host"`:** `src/journal/mod.rs:2989`, `:3215`, `:3261`, `:3568`, `src/envelope/mod.rs:537` | **Code edit + a read-side decision.** Pick one label spelling, change the producer at `driver/run.rs:954`, and change all five sites in the same commit. Because `journal.jsonl` is durable and tailed, the reader must accept **both** spellings for records already written, or the plan must state that pre-Phase-22 runs render differently. `[VERIFIED: grep of src/, 2026-09-16]` |
| **Stored data (new)** | the container id / name on the run record | **Code edit.** New `#[serde(default)] pub container_id: Option<String>` on `RunRecord`. Note `run_record_fields_all_declare_their_scope` (`src/journal/mod.rs:3147`) parses the struct's own source and **fails unless the new field's doc comment contains "run-scoped" or "iteration-scoped"** |
| **Live service config** | the **named volume** holding `CLAUDE_CONFIG_DIR`. It is runtime state in the container runtime's store, not in git, and it is **user state**: once a human has authenticated into it, renaming it or changing its internal layout means they must log in again (D-22-09's own reversibility note) | **Choose the name once and pin it in a test.** A rename is a user-visible migration |
| **Live service config** | `podman.socket` unit state — currently `inactive` **and** `disabled` on this host | **None from this tool.** Report by name; never enable a unit on the user's behalf |
| **OS-registered state** | none. This phase registers no systemd unit, no task, no launchd plist | **None — verified**: the tool only *reads* `systemctl --user is-active` in the pre-flight and never writes |
| **Secrets / env vars** | `CLAUDE_CODE_OAUTH_TOKEN` **if** `setup-token` is chosen (§ 1g recommends against it). The envelope's `GIT_ASKPASS` channel carries no secret at rest and needs no change | **None if `auth login` is chosen.** If `setup-token` is ever adopted, the token must not ride `-e` |
| **Build artifacts** | the **container image**. It is not in git, it is not versioned by `cargo`, and a stale one is indistinguishable from a fresh one at run time | **A version probe, not a rebuild.** The § 2f pre-flight (`<binary> --version` inside the image, compared to the host's) is what makes a stale image a refusal rather than a silent behaviour difference |
| **Build artifacts** | `target/` — the running host binary. `hooks.rs:245-251` already refuses when `current_exe()` no longer exists ("a running-but-deleted executable reports its path as `… (deleted)`"), and a `cargo build` **replaces the inode the container has bind-mounted** | **Document it.** A rebuild mid-run changes what the container's stubs exec. Not new — it is the existing hazard crossing a mount — but the mount makes it easier to hit |

---

## Common Pitfalls

### Pitfall 1: the silent envelope gap

**What goes wrong:** the container's `git push` is unpoliced and git says nothing.
**Why it happens:** neither runtime propagates the client's environment, so `core.hooksPath` is
simply absent (git case 3, measured), and git treats a missing hook as "no hook".
**How to avoid:** mount the envelope directory *and* the binary at identical paths; make the
§ 2f version probe a hard pre-flight.
**Warning signs:** a container run that pushes successfully to a ref outside the reserved
namespace; `journal.jsonl` with no `envelope` park record on a run that pushed.

### Pitfall 2: the Docker habit is backwards on podman

**What goes wrong:** `--user $(id -u):$(id -g)` fails with `Permission denied` on rootless
podman (§ F2 row B); omitting it on rootful docker writes **root-owned files into the user's
project tree** (row F).
**Why it happens:** rootless podman maps container uid 0 → host uid 1000 and container uid 1+ →
host 165536+.
**How to avoid:** branch on the *verified* runtime, per § Pattern 2.
**Warning signs:** `Permission denied` on the first write; `ls -l` in the project showing
`root root` on new files.

### Pitfall 3: `connect_with_podman_defaults()` returns docker

**What goes wrong:** `--userns=keep-id` is emitted at a docker daemon, which rejects it
(`invalid USER mode`, row H) — or worse, the run succeeds against docker and produces root-owned
files.
**Why it happens:** the function's last line is the docker socket (source, § Pattern 1).
**How to avoid:** never call it; enumerate paths and read `version().components`.
**Warning signs:** a run that reports "podman" on a host where `podman.socket` is inactive.

### Pitfall 4: the settings file that is found but wrong

**What goes wrong:** layer 2's guard is absent with no error.
**Why it happens:** measured — `--settings <invalid JSON that exists>` → **exit 0, no stderr,
run proceeds**; only a *missing* file is loud. `write_settings_in`'s round-trip check exists for
exactly this.
**How to avoid:** nothing new is needed on the host path; on the container path, ensure the
mounted file is the one the host round-tripped (identical path ⇒ identical bytes).
**Warning signs:** a container run whose `PreToolUse` guard never appears in the journal.

### Pitfall 5: `--resume` silently starts a new session

**What goes wrong:** resume "works" but the conversation is empty.
**Why it happens:** the session directory slug is derived from the absolute path (§ 1b).
**How to avoid:** D-22-08's identity mount.
**Warning signs:** a resumed run whose first `system/init` reports zero prior turns; a
`<CLAUDE_CONFIG_DIR>/projects/` containing two slugs for one project.

### Pitfall 6: the two-file trap, half-solved

**What goes wrong:** the container is authenticated for one run and signed out the next.
**Why it happens:** a volume mounted at `~/.claude` with `CLAUDE_CONFIG_DIR` **unset** leaves
`~/.claude.json` outside the volume. Measured: setting the variable moves it *inside*.
**How to avoid:** D-22-10 — one directory, one volume, one variable. Assert it: the pre-flight's
`auth status` output includes `configDirectory`, so the plan can require it to equal the mount
point rather than trust the env var was applied.

### Pitfall 7: the `PreToolUse` hang, re-created at a new boundary

**What goes wrong:** the agent hangs for minutes per tool call.
**Why it happens:** `src/executor/mod.rs:225-239` records a **reproduced** 180-240 s hang from
unbounded `PreToolUse` hooks. In a container, the guard hook execs a bind-mounted binary whose
first act is to initialise a file logger — measured to **panic** on an unwritable `HOME`.
**How to avoid:** `GUARD_TIMEOUT_SECS = 5` is already registered in the generated settings
(`hooks.rs:1484`) and caps the damage; the § 2f probe catches the unwritable-`HOME` case before
any run starts.
**Warning signs:** a container run where every Bash tool call takes ~5 s.

### Pitfall 8: a stale image is invisible

**What goes wrong:** a run silently uses an old CLI or an old policy binary.
**Why it happens:** images have no build-time relationship to `cargo`, and
`assert_provenance_in` compares **paths, not versions**.
**How to avoid:** bind-mount the binary (removes the skew entirely) and pin + probe the CLI
version.
**Warning signs:** `claude_code_version` on the run record differing between a host run and a
container run of the same project.

---

## Code Examples

### The auth pre-flight (D-22-12's refusal, with the remedy named)

```rust
/// Refuse a containerized run whose credential volume was never seeded.
///
/// **This does NOT duplicate `gate::check_auth_path`, and the distinction is the
/// point.** Measured 2026-09-16 against claude 2.1.274: an unauthenticated
/// `CLAUDE_CONFIG_DIR` emits a perfectly ordinary `system/init` with
/// `apiKeySource: "none"`, which the Phase 15 gate ACCEPTS — and only then does
/// the run fail with `result: "Not logged in · Please run /login"`. D-22-11's
/// inheritance proves "no API key snuck in"; it does not prove "the volume is
/// authenticated". Both checks are needed and neither subsumes the other.
async fn assert_volume_authenticated(
    runtime: &VerifiedRuntime,
    volume: &str,
    config_dir: &Path,
    image: &str,
) -> Result<(), PreflightError> {
    let out = run_oneshot(runtime, image, volume, config_dir, &["claude", "auth", "status"]).await?;
    let status: AuthStatus = serde_json::from_slice(&out.stdout)
        .map_err(|_| PreflightError::AuthStatusUnreadable)?;   // unparseable is a refusal

    if !status.logged_in {
        return Err(PreflightError::VolumeUnauthenticated {
            volume: volume.to_string(),
            auth_method: status.auth_method,
            // The remedy, BY NAME — `<specifics>`: never a generic
            // "container runtime unavailable".
            remedy: format!(
                "run once, interactively:  {} run --rm -it {} -v {volume}:{cfg} \
                 -e CLAUDE_CONFIG_DIR={cfg} {image} claude auth login --claudeai",
                runtime.program(), runtime.identity_flags(), cfg = config_dir.display(),
            ),
        });
    }
    // Measured: `configDirectory` echoes back what the CLI actually resolved, so
    // this catches a volume mounted somewhere CLAUDE_CONFIG_DIR does not name —
    // the two-file trap's half-solved form.
    if Path::new(&status.config_directory) != config_dir { … }
    Ok(())
}

/// Measured shape, claude 2.1.274:
///   {"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty",
///    "configDirectory":"/home/blk/.claude","projectsDirectory":"/home/blk/.claude/projects",
///    "email":"…","orgId":"…","orgName":"…","subscriptionType":"max"}
///   {"loggedIn":false,"authMethod":"none","apiProvider":"firstParty", …}
/// Tolerant deserialisation, per `stream_json`'s posture: unknown fields are kept,
/// never rejected — the CLI adds fields between releases.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthStatus {
    logged_in: bool,
    auth_method: String,
    config_directory: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}
```

### The envelope-parity probe (D-22-17, made mechanical)

```rust
/// Refuse unless the image can execute THIS binary — the fail-closed check
/// D-22-17 locks the posture for.
///
/// One probe covers three measured failure modes at once:
///   * musl base image  → `exec …: no such file or directory` (alpine, measured)
///   * mount forgotten  → the same error, for a different reason
///   * unwritable HOME  → `panicked at tracing-appender … PermissionDenied` (measured)
///
/// It must run BEFORE the real container is created. If it is skipped, git's own
/// behaviour is the fallback, and git's behaviour is FAIL-OPEN AND SILENT:
/// measured, `core.hooksPath` pointing at a directory that does not exist pushes
/// with exit 0 and no warning.
async fn assert_envelope_binary_runs_in_image(
    runtime: &VerifiedRuntime,
    image: &str,
    binary: &Path,
) -> Result<(), PreflightError> {
    let host_version = env!("CARGO_PKG_VERSION");
    let out = run_oneshot_with_binary_mount(runtime, image, binary,
                                            &[binary.to_str().unwrap(), "--version"]).await?;
    let observed = String::from_utf8_lossy(&out.stdout);
    if !observed.contains(host_version) {
        return Err(PreflightError::EnvelopeBinaryUnrunnable {
            image: image.to_string(),
            binary: binary.to_path_buf(),
            observed: observed.trim().to_string(),
            remedy: "the image cannot execute the envelope binary. Rebuild the image on a \
                     glibc base (debian/ubuntu) — a musl base such as alpine cannot run a \
                     glibc-linked binary — and give its non-root user a writable HOME."
                     .to_string(),
        });
    }
    Ok(())
}
```

### Runtime identity, with the refusals ordered

```rust
// Measured `version()` responses, 2026-09-16:
//   podman 5.7.0  Version="5.7.0"  ApiVersion="1.41"  Components=["Podman Engine","Conmon","OCI Runtime (runc)"]
//   docker 29.1.3 Version="29.1.3" ApiVersion="1.52"  Components=["Engine","containerd","runc","docker-init"]
//
// `SystemVersion.components` is `Option<Vec<..>>`. An ABSENT array is
// `IdentityUnreadable` — a refusal — and never a fall-through to Docker, because
// the whole point of D-22-01 is that guessing wrong emits `--userns=keep-id` at
// docker (rejected) or root-owns the user's project tree.
match identify(&version) {
    Ok(RuntimeKind::Podman) => …,   // --userns=keep-id      (spike § F2 row D)
    Ok(RuntimeKind::Docker) => …,   // --user $UID:$GID      (spike § F2 row G)
    Err(e) => return Err(e),        // refuse; never default
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| CLI parsing (`docker ps --format json`) with a normalization shim | the Docker-compat REST API, identical payloads from both runtimes | measured in `22-SPIKE-OQ5.md` § F4 | § F3's normalization layer is **deleted**, not deferred (D-22-05) |
| `slirp4netns` as podman's rootless default | `pasta` (the `passt` package) is podman 5.x's default | podman 5.x | `slirp4netns` stays fully supported and documented in `man podman-run` 5.7.0 — it is now a **choice**, not a legacy |
| `~/.claude` + `~/.claude.json` as two locations | `CLAUDE_CONFIG_DIR` relocates both into one directory | measured at 2.1.274 | the ROADMAP's "two-file trap" is closed by one variable (D-22-10) |
| `/login` inside an interactive session | `claude auth login` / `auth logout` / `auth status` as first-class subcommands, JSON by default | present at 2.1.274 | `auth status` is a supported machine-readable predicate — no file sniffing |

**Deprecated / outdated:**

- **Nothing about `slirp4netns`.** No deprecation notice in podman 5.7.0's man page; no runtime
  warning observed. Treat any claim that it is removed as stale.
- `bollard::Docker::connect_with_podman_defaults()` is not deprecated but is **unfit for
  detection** by construction (§ Pattern 1).
- `Cargo.toml:47-48`'s sentence "tokio-util is deliberately not a dependency" becomes false the
  moment bollard is added (§ 4b).

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| `podman` | CTNR-02 podman arm | ✓ | 5.7.0 (rootless, netavark, runc) | — |
| `docker` | CTNR-02 docker arm | ✓ | 29.1.3 (**rootful**, user in `docker` group) | — |
| `pasta` / `passt` | podman 5.x default networking | ✗ | — | **`--network=slirp4netns`, measured working** (Q3) |
| `slirp4netns` | podman rootless egress fallback | ✓ | 1.3.3 (libslirp 4.9.1) | — |
| `aardvark-dns` | podman rootless user-defined-network DNS | ✗ | — | none — user-defined networks are unavailable (Q5) |
| `podman.socket` (systemd user unit) | bollard access to podman | ✗ (`inactive`, `disabled`) | — | `mkdir -p $XDG_RUNTIME_DIR/podman && podman system service …`, measured (C-4) |
| `/var/run/docker.sock` | bollard access to docker | ✓ | mode 660, user in `docker` group | — |
| `claude` CLI (host) | research measurement; not required at run time on the container path | ✓ | 2.1.274 | — |
| `git` | envelope hooks | ✓ | 2.53.0 | — |
| glibc base image | running the bind-mounted binary | ✓ (`php:8.2-cli`, Debian trixie glibc 2.41, present locally) | — | a musl-static build of `gsd-meta-manager` |
| the container image itself | every criterion | ✗ — **does not exist yet** | — | none: building it is Phase 22 work |
| `cargo` / rustc | build | ✓ | MSRV floor 1.88 | — |

**Missing dependencies with no fallback:**
- **The container image.** It is the phase's own deliverable, not a host prerequisite.
- `aardvark-dns` + `passt` together — their absence is what makes an enforced egress allowlist
  unavailable on this host's podman (Q5). No fallback exists inside the spike's fence.

**Missing dependencies with fallback:**
- `pasta`/`passt` → `--network=slirp4netns` (measured; DNS and real TLS egress both work).
- `podman.socket` → either `systemctl --user start podman.socket` (named remedy, preferred) or
  the manual `podman system service` path (measured, but recommended NOT to automate).

**CI note:** a CI runner will have **neither** runtime. See § *Testability* — the argv, the
identity parsing, the refusal paths and the label rendering are all testable with no runtime
present, and only the end-to-end arms need one.

---

## Testability

| What | Needs a runtime? | How the repo already does this |
|---|---|---|
| `build_argv` output for `Container` (the F2 flag list, per runtime) | **no** | pure-function argv tests; `tests/spawn_seam_guard.rs` already asserts argv shape |
| `VerifiedRuntime` construction refusals (absent `Components`, unknown engine, socket missing) | **no** | feed a fixture `SystemVersion` to the identify function; keep it a pure function taking `&SystemVersion` so the bollard call is the only untested line |
| `--name <run-id>` derivation and validation | **no** | plain unit test |
| The five pre-flight refusals' **messages naming the remedy** | **no** | mirrors `gate.rs`'s `CapabilityError` shape, already tested there |
| `RunRecord.target` label + the five `"host"`/`"Host"` sites | **no** | `run_record_fields_all_declare_their_scope` (`journal/mod.rs:3147`) already parses the struct's source; add a label-spelling assertion beside it |
| `tests/spawn_seam_guard.rs` staying green **unmodified** (D-22-19) | **no** | run it; the seam guard is the assertion |
| Envelope parity — git's fail-open on a missing `hooksPath` | **no** | reproducible with a scratch repo and no container at all; the four cases in § 2b are exactly such a test, and the **case-1/case-3 fail-open arms are the ones worth committing** as the regression that proves the probe is load-bearing |
| End-to-end container run, both runtimes | **yes** | `#[ignore]`, the established idiom — `tests/driver_injection_corpus.rs:68` (*"they are `#[ignore]`d … it needs a binary and an authenticated subscription"*), with the **ignore-count census** at `:1411` that asserts the exact number of `#[ignore]` attributes in the file |
| A test that skips itself when a runtime is absent | **yes, softly** | `tests/state_reader_test.rs:924`'s `let Ok(path) = std::env::var("GSD_META_MANAGER_REAL_STATE_MD") else { return }` — an env-gated self-skipping test |

**Recommendation:** put every container end-to-end arm in one new file with its own
`#[ignore]`-count census, copying `driver_injection_corpus.rs`'s shape verbatim. That file's own
comment states the rule this phase should inherit: *"`#[ignore]` is for a test that needs a real
binary and an authenticated subscription"* — a container runtime is the same class of
requirement. Do **not** make the container tests env-gated-and-silent: a silently skipped
security test is the failure this repository's conventions are written against.

---

## Security Domain

`security_enforcement` is not set in `.planning/config.json`, so it is enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---|---|---|
| V2 Authentication | **yes** | subscription OAuth in a named volume; `claude auth status` as the predicate; the run **never** performs a login (D-22-12) |
| V3 Session Management | **yes** | session history is volume state under `CLAUDE_CONFIG_DIR`; resume identity is the absolute-path slug |
| V4 Access Control | **yes** | `DrivableProject` + the new `VerifiedRuntime` capability tokens; uid mapping per § F2 so the container cannot write files the user cannot own |
| V5 Input Validation | **yes** | the image name and target flag validated at `driver::DriveArgs::from_argv` (the repo's stated home), never a clap `value_parser`; `crate::text::Untrusted` for anything echoed back |
| V6 Cryptography | no (none hand-rolled) | TLS is the CLI's and the runtime's; bollard speaks a **unix socket**, so no TLS enters this tree (§ 4a) |
| V12 Files & Resources | **yes** | bind mounts are the attack surface: every one read-only where possible, the binary strictly `:ro`, and nothing under the host `$HOME` mounted except the two named paths |
| V13 API & Web Service | **yes** | the runtime socket is a full-control API — reaching it is equivalent to the user's own docker/podman privilege; the tool must never widen it |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---|---|---|
| Container runtime misidentified → `--userns=keep-id` at docker, or root-owned files in the project tree | Tampering / EoP | `VerifiedRuntime` read from `version().components` (D-22-01/D-22-15), refusal on absence |
| Push boundary silently absent inside the container | Repudiation / Tampering | envelope dir + binary mounted at identical paths; § 2f probe; measured git fail-open is the thing being prevented |
| Credential exfiltration from the container | Information Disclosure | host `~/.claude` never mounted (D-22-09); `SSH_AUTH_SOCK` absent by construction; no git credential in the image; `auth login`, not a token on `-e` (§ 1g) |
| Runtime socket mounted into the container | EoP | **never mount `/var/run/docker.sock` or the podman socket into the driven container.** The container would then control the runtime that isolates it. Not currently proposed — recorded so it is not later added as a convenience |
| Unrestricted egress from a driven agent | Information Disclosure | **enforced on docker only** (Q5/C-1). State it; do not promise it on podman |
| Stale image running an old policy binary | Tampering | binary is bind-mounted (skew unrepresentable); CLI version pinned and probed |
| Image name supplied by an untrusted path becoming an arbitrary exec | EoP | validate at `DriveArgs::from_argv`; the runtime *program* is a literal chosen by `RuntimeKind`, never a user string |
| `PreToolUse` guard hung on the agent's critical path | DoS | `GUARD_TIMEOUT_SECS = 5` already registered (`hooks.rs:1484`); the reproduced 180-240 s incident is recorded at `executor/mod.rs:225-239` |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | `claude auth login --claudeai` completes inside a container by printing a URL for the human to paste (the OAuth flow was **not** exercised — completing it would mint a real credential) | Q1 § 1g | the recommended one-shot needs a different shape (e.g. `--console`, or a host-side login copied in). Cheap to falsify at execution time; it does not change any decision |
| A2 | `authMethod` has values beyond the measured `"claude.ai"` and `"none"` | Q1 § 1e | only matters if the plan enumerates the field; the recommendation is to refuse on `loggedIn` and *report* `authMethod`, which is robust either way |
| A3 | `pasta` offers no outbound destination filtering | Q5 § 5a | `passt` is not installed here and was not exercised. If pasta *does* filter, the podman arm of Q5 improves — it cannot get worse |
| A4 | podman `--internal` under netavark genuinely blocks egress (as docker's does) | Q5 § 5c | unmeasurable here (pasta absent). Named only as the option that would require a host-package prerequisite, and that option is **not** recommended |
| A5 | An SELinux host needs `:z`/`:Z` on the bind mounts | § Pattern 2 | this host is AppArmor and the spike explicitly did not exercise relabeling (§ F2 closing line). A plan that guesses a flag in would be guessing; surface it as a documented limitation |
| A6 | `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` narrows outbound endpoints | § Pattern 7 | the *name* is verified present in the binary; the *effect* is not. Only affects how wide an advisory allowlist has to be |
| A7 | The clean-release-build delta is ~+5 s; `rm -rf target` raced once per group, which can only understate the with-bollard figure | Q4 § 4c | the figure is directional. The crate count (25) is exact and is the number the `Cargo.toml` comment leans on |
| A8 | Naming the container `<run-id>` is safe for both runtimes' name grammar | § Pattern 2 | run ids are already validated as plain path components; a runtime with a narrower name grammar would need a prefix. Trivial to falsify at execution time |

---

## Open Questions

1. **What exactly does `claude auth login` look like inside a one-shot container?**
   - What we know: the subcommand exists with `--claudeai` (default), `--console`, `--sso`,
     `--email`; `auth status` reads back `loggedIn`/`authMethod`; the credential lands in
     `<CLAUDE_CONFIG_DIR>/.credentials.json` under `claudeAiOauth`.
   - What's unclear: whether the device/URL flow is usable with no browser *in* the container.
   - Recommendation: first task of whichever plan owns CTNR-03 — it is one command and one
     observation, and it does not gate planning.

2. **Which label strings does D-22-16 settle on, and does the reader accept both?**
   - What we know: the producer emits `"Host"`; five in-tree sites say `"host"`; `journal.jsonl`
     is durable and re-read by `driver::reconcile`.
   - What's unclear: whether pre-Phase-22 records must keep rendering correctly.
   - Recommendation: choose lowercase (it matches the five existing fixtures and every other
     string this codebase puts on disk), make the reader accept both, and pin the tolerance with
     a test — the durable-record rule (D-22-16, *one-way*) makes silence here expensive.

3. **Where does the named volume's name live, and how is a rename handled?**
   - What we know: D-22-09 rates a rename as *costly* — it is user state.
   - Recommendation: a single named constant with a comment saying a change is a user migration,
     asserted by a test, in the register `SUBSCRIPTION_API_KEY_SOURCE` uses at `gate.rs:77-81`.

4. **Does the egress-allowlist posture change ROADMAP § Phase 22's risk list?**
   - What we know: C-1 — the verbatim-adopted wording is not deliverable on rootless podman.
   - Recommendation: the plan should state the real posture and record the measured docker
     recipe as a deferred, sized follow-up rather than quietly dropping the risk line.

---

## Sources

### Primary (HIGH confidence — measured on this host, 2026-09-16)

- `claude` CLI **2.1.274** — `--help`, `auth status`/`auth login`/`setup-token`/`install` help,
  `doctor`; three `apiKeySource` measurements off the real stream-json wire; `--settings`
  missing-vs-invalid behaviour; `CLAUDE_CONFIG_DIR` file relocation; `~/.claude/projects/` slug
  derivation; `.credentials.json` / `.claude.json` key shapes (names only, no values);
  `strings` over `~/.local/share/claude/versions/2.1.274` for the `apiKeySource` resolver and
  the env-var name inventory.
- `git` **2.53.0** — four `core.hooksPath` cases against a real bare remote (fail-open on a
  missing directory and on absent env; fail-closed on a missing binary and on a non-zero hook).
- `podman` **5.7.0** rootless — `pasta` absence reproduced; `--network=slirp4netns` DNS + real
  HTTPS egress to `api.anthropic.com` (405); `man podman-run` network section; `podman info`
  `.Host.Pasta` / `.Host.Slirp4NetNS` / `.Host.RemoteSocket`; `--internal` network failure;
  container `CapEff`; `mkdir` + `podman system service` socket activation with `/_ping` and
  `/version` responses; host-binary bind-mount execution on musl vs glibc; `run -i` stdio duplex.
- `docker` **29.1.3** rootful — `network create --internal` egress blocking with a default-bridge
  control; `run -i --user` stdio duplex.
- `cargo` — bollard lockfile deltas across four feature sets on a scratch copy of this repo's
  exact `Cargo.toml`/`Cargo.lock`; compile success/failure per feature set; clean release build
  timings (2 runs each).
- **This repository's source**, read this session: `src/executor/mod.rs`, `claude.rs`, `gate.rs`;
  `src/envelope/hooks.rs`, `cred.rs`, `mod.rs`; `src/driver/run.rs`, `kill.rs`; `src/config.rs`,
  `src/cli.rs`; `src/journal/mod.rs`; `tests/spawn_seam_guard.rs`; `Cargo.toml`.
- **`bollard` 0.21.1 source**, read from the local registry: `src/docker.rs` (`connect_with_socket`,
  `connect_with_podman_defaults`), `src/system.rs` (`version`, `ping`, `info`),
  `src/container.rs` (`list_containers`, `stop_container`, `remove_container`,
  `inspect_container`), `bollard-stubs` `SystemVersion` / `SystemVersionComponents`,
  `Cargo.toml` `[features]`.

### Binding project artifacts (cited, not re-derived)

- `.planning/phases/22-container-execution-target/22-SPIKE-OQ5.md` — F1 (pasta missing), F2 (the
  uid/mount matrix, rows A–H), F3 (`ps --format json` divergence), F4 (the API erases it),
  F5 (the two auto-detect traps).
- `.planning/phases/22-container-execution-target/22-CONTEXT.md` — D-22-01 … D-22-19.
- `.planning/ROADMAP.md` § *Phase 22*; `.planning/REQUIREMENTS.md` § *Containerized Sessions*.
- `.planning/codebase/ARCHITECTURE.md` § *Drive-Start Path*, § *Key Abstractions*,
  § *Architectural Constraints*.
- `CLAUDE.md` — stack table, release process, MSRV provenance.

### Secondary / Tertiary

None. Every claim in this document is either measured on this host, read from source in this
session, cited from a binding project artifact, or explicitly tagged `[ASSUMED]` in the
Assumptions Log. **No web search was used and none was needed.**

---

## Metadata

**Confidence breakdown:**

| Area | Level | Reason |
|---|---|---|
| Question 1 (auth mechanism) | **HIGH** | three `apiKeySource` channels, the unauthenticated row, the `auth status` detector and the `CLAUDE_CONFIG_DIR` relocation all measured; only the interactive login *shape* is `[ASSUMED]` (A1) |
| Question 2 (envelope parity) | **HIGH** | four git cases, two `--settings` cases, and the binary-in-container execution all measured end to end |
| Question 3 (slirp4netns) | **HIGH** | real TLS round trip to `api.anthropic.com`; man page checked for deprecation; F1 reproduced |
| Question 4 (bollard cost) | **HIGH** for the crate count (exact), **MEDIUM** for the build-time delta (A7) |
| Question 5 (egress allowlist) | **HIGH** for what is unavailable on podman and what works on docker; **MEDIUM** overall because pasta's own capabilities are `[ASSUMED]` (A3, A4) |
| Standard stack | **HIGH** | one dependency, already exercised against both runtimes by this project's own spike |
| Architecture / the `Container` shape | **HIGH** | every integration point read in source this session, including the C-3 correction to CONTEXT.md's tripwire claim |
| Image recipe | **MEDIUM** | the base-image and updater constraints are measured; the image itself was not built |
| Pitfalls | **HIGH** | every one is a measurement or a source-recorded incident in this repository |

**Research date:** 2026-09-16
**Valid until:** 2026-10-16 for the runtime and git findings (stable). **~7 days** for the
`claude` CLI findings — 2.1.274 auto-updates, this host moved to it on 2026-09-17, and the gate's
`TESTED_MAXIMUM_CLAUDE_VERSION` is 2.1.220. Re-measure `apiKeySource` and `auth status` before
pinning a CLI version into the image.

**Experiment residue:** none. Verified after teardown — `podman ps -a` empty,
`podman volume ls` empty, `podman network ls` shows only the default `podman`, both pre-existing
podman images untouched and nothing pulled; `docker network ls` restored (the `gsdq5-internal`
network removed); `podman.socket` `inactive`/`disabled` and `/run/user/1000/podman` absent, as
found; `git status --porcelain` shows exactly the three pre-existing entries and nothing else.
All scratch work lives under the session scratchpad.
