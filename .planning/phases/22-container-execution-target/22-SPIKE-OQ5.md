# OQ5 Spike — Podman rootless UID mapping, volume permissions, and Docker-CLI/API parity

**Verdict:** RESOLVED

**Run date:** 2026-09-15
**podman:** 5.7.0 (rootless, runc, overlay, graphRoot `~/.local/share/containers/storage`)
**docker:** 29.1.3 (**rootful** daemon, `/var/lib/docker`, user in the `docker` group)
**bollard:** 0.21.1 (probed out-of-tree; not yet a project dependency)
**Host:** uid/gid 1000, subuid/subgid `blk:165536:65536`
**Spike owner:** Phase 22 MUST-SPIKE (ROADMAP § *Phase 22 — Phase risks*), deferred here by
Phase 15 (`15-CONTEXT.md` § *deferred*: "OQ5 (podman rootless uid mapping / volume
permissions) → Phase 22's spike, which requires actually installing podman")

Not a phase gate in the D-27 sense — OQ5 is a question that had to be answered with a real
podman before Phase 22's planning could finalize, not a pass/fail condition on the
milestone premise. It is now answered empirically on both runtimes.

---

## Scope

The roadmap flagged three LOW-confidence claims, all asserted from Docker/devcontainer-centric
docs and never exercised:

- **(a)** rootless podman UID/GID mapping and bind-mount write permissions — does a
  containerized run write host-owned files into a mounted project directory, and what does
  `--userns=keep-id` actually change?
- **(b)** `--format json` shape — docker NDJSON vs podman array.
- **(c)** general Docker-CLI/API compatibility, specifically
  `bollard::connect_with_podman_defaults()` end to end, since auto-detect is a user-locked
  hard requirement.

All three are answered below, each against both runtimes on the same host.

---

## Safety fences

| Fence | How it was honoured |
|-------|---------------------|
| Disposable scratch mount | a fresh `oq5/mnt` and `oq5/dmnt` under the session scratchpad; never the repo, never a registered project |
| No networked containers | every podman probe ran `--network=none` (see Finding F1) |
| Minimal image | `docker.io/library/alpine:3.20`, pulled for the spike and removed afterwards; the pre-existing podman `alpine:3` was left untouched |
| No privileged flags | no `--privileged`, no `--cap-add`, no `sudo` |
| Socket enabled transiently | `systemctl --user start podman.socket` for the API arm, stopped again afterwards — the unit was **not** `enable`d and is `inactive` again, exactly as found |
| Full teardown | no containers, images, volumes or sockets left behind — verified (see *Residue*) |

---

## Finding F1 — rootless podman on this host is broken out of the box: `pasta` is missing

The very first `podman run` failed before any mount question could be asked:

```
Error: could not find pasta, the network namespace can't be configured:
exec: "pasta": executable file not found in $PATH
```

podman 5.x defaults rootless networking to **pasta** (the `passt` package), which is not
installed here; only `slirp4netns 1.3.3` is. Every probe below therefore ran
`--network=none`.

**This is a hard prerequisite, not a footnote.** A containerized Claude CLI run needs
egress. On a rootless-podman host the phase must either require `passt`, or explicitly pass
`--network=slirp4netns`, and it must **detect and report this up front** — the error arrives
at container-create time with a message that has nothing to do with the driven run, and
would otherwise surface to the user as an opaque transport failure.

---

## Finding F2 — the UID-mapping matrix (question (a))

Rootless podman maps container uid 0 → host uid 1000, and container uid 1..65536 →
host 165536.. (`idMappings` confirmed from `podman info`):

```json
"uidmap": [{"container_id":0,"host_id":1000,"size":1},
           {"container_id":1,"host_id":165536,"size":65536}]
```

Mount: a host directory owned `1000:1000`, mode `775`, at `/work`.

| # | Runtime | Flags | In-container `id` | Write to `/work` | Host owner of new file |
|---|---------|-------|-------------------|------------------|------------------------|
| A | podman rootless | *(default)* | `uid=0(root)` | **ok** | **1000:1000** ✅ |
| B | podman rootless | `--user 1000:1000` | `uid=1000` | **`Permission denied`** ❌ | — |
| C | podman rootless | `--userns=keep-id --user 1000:1000` | `uid=1000(blk)` | **ok** | **1000:1000** ✅ |
| D | podman rootless | `--userns=keep-id` *(no `--user`)* | `uid=1000(blk)` | **ok** | **1000:1000** ✅ |
| F | docker (rootful) | *(default)* | `uid=0` | ok | **0:0** ❌ root-owned |
| G | docker (rootful) | `--user 1000:1000` | `uid=1000` | ok | **1000:1000** ✅ |
| H | docker (rootful) | `--userns=keep-id` | — | — | **`docker: --userns: invalid USER mode`** ❌ |

Four things fall out of this table, and they are the whole answer to (a):

1. **The naive "portable" argv is exactly backwards on podman.** The Docker habit
   `--user $(id -u):$(id -g)` — which is *required* on rootful docker (row F writes
   root-owned files into the user's project tree) — **fails outright** on rootless podman
   (row B): uid 1000 inside the container maps to host uid 166535, which has no write
   access to a `1000:1000` directory. Row B is the trap the roadmap suspected, confirmed.

2. **`--userns=keep-id` is podman-only and docker rejects it** (row H, `invalid USER mode`).
   There is no single argv that satisfies both runtimes. The `ExecutionTarget::Container`
   argv prefix **must branch on the detected runtime**; this is a concrete requirement on
   whatever type carries the container target.

3. **`--userns=keep-id` without `--user` already does the right thing** (row D): keep-id
   remaps the container's default user to the host uid, so the process comes up as
   `uid=1000(blk)` *even though the image's default user is root*. `--user` is redundant
   alongside it, though harmless (row C).

4. **Rootless podman's default (row A) is already correct for a host-mounted project.**
   Container root → host uid 1000 → host-owned files. `keep-id` is needed only when the
   image runs a non-root user (the recommended posture for the CLI image, per the roadmap's
   "run as a non-root user" risk note) — which is precisely the case Phase 22 will be in.

**The recipe Phase 22 should carry:**

| Runtime | Mount argv |
|---------|-----------|
| rootless podman | `--userns=keep-id` (plus `--user $UID:$GID` only if the image default must be overridden) |
| rootful docker | `--user $UID:$GID` |

Both then produce host-uid-owned files in the mounted project directory, which is what
mount-path parity and `--resume` require.

No SELinux on this host (AppArmor); `:z`/`:Z` relabeling was not needed and was not
exercised.

---

## Finding F3 — `ps --format json` shapes genuinely diverge (question (b))

Confirmed, and worse than "array vs NDJSON" — the *field names and types* differ too.

**podman:** a pretty-printed JSON **array** (first byte `[`).
**docker:** **NDJSON**, one object per line (first byte `{`). `docker ps --format json` and
`docker ps --format '{{json .}}'` produce byte-identical output.

Key shape differences for the same running container:

| | podman | docker |
|---|--------|--------|
| id field | `Id` (full 64-hex) | `ID` (12-hex short) |
| `Names` | **array** `["oq5-podman-ps"]` | **string** `"oq5-docker-ps"` |
| `Command` | **array** `["sleep","60"]` | **string** `"\"sleep 60\""` |
| `Image` | `docker.io/library/alpine:3.20` (fully qualified) | `alpine:3.20` (as tagged) |
| podman-only | `AutoRemove`, `CIDFile`, `Created`, `ExitCode`, `Exited`, `ExitedAt`, `ExposedPorts`, `ImageID`, `IsInfra`, `Namespaces`, `Pid`, `Pod`, `PodName`, `Restarts`, `StartedAt` | — |
| docker-only | — | `ID`, `LocalVolumes`, `Platform`, `RunningFor` |

`State` (`"running"`) and `Status` (`"Up …"`) are the only two interesting fields that agree
in both name and type.

**Implication:** the `ps --format json` normalization the roadmap anticipated is real work —
but Finding F4 makes it unnecessary.

---

## Finding F4 — the Docker-compat REST API erases the CLI divergence

`systemctl --user start podman.socket` (the socket is **not** running by default here, see
F5) exposes `/run/user/1000/podman/podman.sock`, and it speaks Docker's API:

```
GET /v1.41/_ping  → HTTP/1.1 200 OK
   Api-Version: 1.41   Libpod-Api-Version: 5.7.0   Docker-Experimental: true
GET /v1.41/version → {"Version":"5.7.0","ApiVersion":"1.41","MinAPIVersion":"1.24",...}
```

`GET /v1.41/containers/json` returns a **JSON array of Docker-shaped objects** — `Id`,
`Names: ["/oq5-podman-ps"]` (array, leading slash, exactly like Docker), `State`, `Status`,
`NetworkSettings`, `Mounts`. Podman's CLI-only fields (`Pod`, `IsInfra`, `Restarts`, …) are
gone.

bollard 0.21.1 drove both sockets unmodified:

```
podman: version() Version="5.7.0" Api="1.41" Components=["Podman Engine","Conmon","OCI Runtime (runc)"]
        list_containers() n=1 names=["/oq5-podman-ps2"] state=RUNNING
        ping() OK
docker: version() Version="29.1.3" Api="1.52"
```

**Go through the API, not the CLI.** One bollard code path covers both runtimes for
inspect/list/lifecycle, and Finding F3's normalization layer never needs to be written. The
argv-level branch of Finding F2 remains — mount and userns flags are create-time
parameters, not a CLI-parsing concern.

---

## Finding F5 — two auto-detect traps, both silent

Auto-detect is a user-locked hard requirement, and both of these defeat a naive
implementation.

**Trap 1 — `podman info` claims a socket that does not exist.**

```
$ podman info --format '{{json .Host.RemoteSocket}}'
{"path":"/run/user/1000/podman/podman.sock","exists":true}
$ ls /run/user/1000/podman/podman.sock
ls: cannot access ...: No such file or directory
```

`exists: true` is asserted while the file is absent and `podman.socket` is `inactive`.
Worse, `/run/user/1000/podman/` does not exist either, and podman will not create it:

```
$ podman system service --time=5 unix:///run/user/1000/podman/podman.sock
Error: unable to create socket: listen unix /run/user/1000/podman/podman.sock:
       bind: no such file or directory
```

Only `systemctl --user start podman.socket` brings it up (systemd creates the directory).
So on a stock rootless podman host **there is no API socket until something starts it** —
Phase 22 must either start/require `podman.socket` or fall back to the CLI, and it must
never trust `RemoteSocket.exists`.

**Trap 2 — `connect_with_podman_defaults()` silently returns a *Docker* connection.**
This is the load-bearing finding for auto-detect. bollard's documented discovery order is:

```
1. $DOCKER_HOST (if unix://)   2. $XDG_RUNTIME_DIR/podman/podman.sock
3. /run/user/$UID/podman/podman.sock   4. /run/podman/podman.sock
5. Falls back to the default Docker socket (/var/run/docker.sock)
```

Observed, same binary, same host, socket up then down:

```
socket UP   → podman_defaults: OK Version="5.7.0"  Api="1.41" Components=["Podman Engine","Conmon","OCI Runtime (runc)"]
socket DOWN → podman_defaults: OK Version="29.1.3" Api="1.52" Components=["Engine","containerd","runc","docker-init"]
```

The function named `connect_with_podman_defaults` returned a healthy handle to **docker**,
with no error and no warning. Any auto-detect that infers "podman" from that call
succeeding will then emit `--userns=keep-id` at a rootful docker daemon, which rejects it
(row H) — or, on a host where docker *is* reachable, silently produce root-owned files in
the user's project tree (row F).

**The runtime must be identified from the response, never from which constructor was
called.** `version().components[].name` is decisive and free — `"Podman Engine"` vs
`"Engine"` — as is the `Libpod-Api-Version` response header on `/_ping`. Detect, then
choose the argv prefix from the detected runtime.

---

## What this means for Phase 22

OQ5's three LOW-confidence claims are now HIGH-confidence, and none of them landed where
the docs suggested:

1. **The mount recipe is runtime-specific and inverted between the two runtimes** (F2).
   `--userns=keep-id` on rootless podman, `--user $UID:$GID` on rootful docker; each flag is
   wrong or rejected on the other runtime. The `ExecutionTarget::Container` argv prefix must
   branch on the detected runtime — this is a real constraint on the "argv prefix plus path
   map swap" design, not a free parameter.
2. **Use the Docker-compat REST API via bollard for everything except create-time flags**
   (F4). The `ps --format json` normalization the roadmap budgeted for is not needed; the
   API returns identical Docker-shaped payloads from both runtimes.
3. **Auto-detect must verify runtime identity from `version().components`, and must not
   trust `connect_with_podman_defaults()` or `podman info`'s `RemoteSocket.exists`** (F5).
   Both lie in the direction of "podman is present" when it is not reachable.
4. **A rootless-podman host needs `passt`/`pasta` (or an explicit
   `--network=slirp4netns`), and needs `podman.socket` started** (F1, F5). Both are
   pre-flight conditions the driver should check and report by name, in the same shape as
   Phase 15's capability gate — a missing `pasta` currently surfaces as an opaque
   container-create failure.

Phase 22 planning may proceed. Success criterion 1 ("the same driven run works on a docker
host and on a rootless podman host with no configuration change") survives, but with its
meaning pinned down: *no user-facing* configuration change, achieved by the driver
branching internally on a **verified** runtime identity — not by a single argv that happens
to work on both. No such argv exists.

---

## Residue

**None.** Verified after teardown:

- `podman ps -a` — empty. `podman volume ls` — empty. `podman images` — only the
  pre-existing `docker.io/library/alpine:3`; the spike's `alpine:3.20` was removed.
- `docker ps -a` / `docker images` — only the host's pre-existing unrelated workloads
  (coolify, photoprism, …); every `oq5-*` container and every alpine tag pulled by this
  spike was removed.
- `systemctl --user is-active podman.socket` → `inactive`, and
  `/run/user/1000/podman/podman.sock` is gone — restored to exactly the state found.
- Scratch mounts and the out-of-tree bollard probe crate live only under the session
  scratchpad; nothing was added to this repo's `Cargo.toml` or `Cargo.lock`.
- The one root-owned file docker created (row F) was removed from inside a container, not
  with `sudo`.
