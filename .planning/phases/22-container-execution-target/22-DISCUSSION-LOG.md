# Phase 22: Container Execution Target - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-16
**Phase:** 22-container-execution-target
**Mode:** unattended (`--auto` semantics — the human operator was unavailable; every
selection below was made by the agent from ROADMAP.md, REQUIREMENTS.md, 15-CONTEXT.md,
`22-SPIKE-OQ5.md` and the codebase, with no AskUserQuestion)
**Areas discussed:** Runtime detection, Transport split, Mount & path parity, Credentials,
Pre-flight prerequisites, Target shape & durable record, Envelope parity, Target selection & TUI

---

## Runtime detection and identity

| Option | Description | Selected |
|--------|-------------|----------|
| Trust `connect_with_podman_defaults()` | Assume podman if the podman constructor succeeds | |
| Trust `podman info`'s `RemoteSocket.exists` | Ask the CLI whether a socket is there | |
| Verify identity from `version().components[]` | Read `"Podman Engine"` vs `"Engine"` off the response; refuse if unreadable | ✓ |

**Choice:** verify from the response (D-22-01, D-22-02).
**Notes:** Not a judgement call — the spike observed both rejected options lying in the same
direction ("podman is present" when it is not reachable). § F5 Trap 2 recorded the podman
constructor returning docker 29.1.3 with no error.

---

## Transport split — how the container is actually driven

| Option | Description | Selected |
|--------|-------------|----------|
| All-argv | `docker run` prefix for everything, parse `ps --format json` for lifecycle | |
| All-API | bollard `create` + `attach` for the run itself, replacing the child-process stdio | |
| Hybrid | argv prefix for the run; API for detection, inspect and stop | ✓ |

**Choice:** hybrid (D-22-04, D-22-05, D-22-06).
**Notes:** All-argv means writing the F3 normalization layer (`Id`/`ID`, `Names` array vs
string, `Command` array vs string) that F4 shows is unnecessary. All-API replaces the
`ChildStdin`/`ChildStdout` duplex and the `process-wrap` process group that Phase 15's
framing, gate and interjection code sit on — which would make success criterion 4 ("the
driver's code path is identical to the host path apart from target selection") false.
The ROADMAP ("argv prefix plus path map swap") and the spike ("go through the API, not the
CLI") appear to conflict; the hybrid is the agent's synthesis and is flagged for audit.

---

## Mount, uid mapping and path parity

| Option | Description | Selected |
|--------|-------------|----------|
| One portable argv | `--user $(id -u):$(id -g)` on both runtimes | |
| Branch on verified runtime | `--userns=keep-id` on rootless podman, `--user $UID:$GID` on docker | ✓ |
| Translate paths | Mount at a canonical container path and map host↔container paths | |
| Identity path map | Mount the project root at its identical host absolute path | ✓ |

**Choice:** branch on runtime (D-22-07); identity path map (D-22-08).
**Notes:** The "portable" argv is the trap the ROADMAP suspected and the spike confirmed —
row B (`Permission denied` on rootless podman, because container uid 1000 maps to host
166535) and row H (`invalid USER mode` on docker). No single argv satisfies both; the spike
states this outright. Identity mapping is the agent's reading of "mount-path parity", chosen
because `--resume` scopes lookup to the project directory and any translation reintroduces
the resume failure the ROADMAP names as a risk.

---

## Credentials inside the container

| Option | Description | Selected |
|--------|-------------|----------|
| Bind-mount host `~/.claude` | Simplest; reuses the host login | |
| Anonymous volume | Container-local credential store | |
| Named volume + `CLAUDE_CONFIG_DIR` | Tool-owned volume covering both credential files, survives rebuild | ✓ |

**Choice:** named volume with `CLAUDE_CONFIG_DIR` pointed at it (D-22-09, D-22-10).
**Notes:** Bind-mount is what success criterion 2 explicitly forbids. Anonymous volume fails
criterion 2's "stays authenticated across a container rebuild". The `CLAUDE_CONFIG_DIR` half
is the ROADMAP's two-file trap: `~/.claude.json` sits outside `~/.claude`.
Additional: rather than add a new check that the host store was not mounted, the phase leans
on Phase 15's existing `apiKeySource != "none"` refusal in `src/executor/gate.rs` (D-22-11).
First login is an explicit one-shot; a driven run refuses rather than attempting it (D-22-12,
inferred — unattended runs cannot complete an interactive OAuth flow).

---

## Pre-flight prerequisites

| Option | Description | Selected |
|--------|-------------|----------|
| Let it fail at create time | Surface whatever docker/podman says | |
| Named pre-flight check, Phase 15 gate shape | Check and report `passt`/`pasta` and `podman.socket` by name before creating anything | ✓ |

**Choice:** named pre-flight (D-22-13).
**Notes:** The spike's very first `podman run` failed with `could not find pasta, the network
namespace can't be configured` — a message with nothing to do with the driven run. `pasta`
is absent on this host and `podman.socket` is inactive by default. Both would otherwise reach
the user as opaque transport failures.

---

## `ExecutionTarget::Container` shape and the durable record

| Option | Description | Selected |
|--------|-------------|----------|
| `Container { image, runtime }` as plain data | Runtime is a field anyone can set | |
| Runtime identity as a capability type | Private fields, one probe-backed constructor, unrepresentable without verification | ✓ |
| Keep `format!("{:?}", target)` in the journal | No change to `src/driver/run.rs:954` | |
| Explicit stable label | Replace the `Debug` rendering with a named, stable string | ✓ |

**Choice:** capability type (D-22-15); explicit label (D-22-16).
**Notes:** The capability type mirrors `DrivableProject`/D-23, this repository's own idiom for
making the compiler the enforcement. The journal change is a defect the agent found while
scouting: today `run.json`/`journal.jsonl` receive `"Host"` from a `Debug` impl while five
in-tree fixtures say `"host"`, and the moment `Container` gains fields the durable record
would receive `Container { image: "…", runtime: Podman }`. Journal records are one-way.

---

## Envelope parity inside the container

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to a later phase | Note the gap, ship the container target without it | |
| Fail closed | Refuse a containerized run whose git envelope cannot be shown live inside the container | ✓ |

**Choice:** fail closed (D-22-17); mechanism left to research.
**Notes:** `src/envelope/hooks.rs` generates its `pre-push` stub naming
`std::env::current_exe()` — a host path — and hands git `core.hooksPath` pointing at it.
Inside a container that stub names a binary that is not present, so the Phase 19 push boundary
would be silently absent while the run looks normal. Criterion 4 is false if the envelope is
one of the differences between host and container. **This is the agent-inferred decision most
likely to change the phase's size and is explicitly flagged for audit.**

---

## Target selection and the TUI

| Option | Description | Selected |
|--------|-------------|----------|
| Per-run argv only | No persistent setting | |
| Per-project config only | No per-run override | |
| Per-project setting + per-run argv override, override wins | Matches Phase 23's stated precedence shape | ✓ |
| New TUI spawn path for containers | Separate start/stop/resume plumbing | |
| Reuse the existing Drive-Start Path | Same `Action` → `start_driver_run` → `spawn_detached` seam | ✓ |

**Choice:** per-project + argv override (D-22-18); no second spawn path (D-22-19).
**Notes:** Precedence was unspecified anywhere; chosen to match Phase 23's own "settable
per-run on argv **and** per-project/session in config, with a documented precedence".
`RegisteredProject`'s `#[serde(flatten)] extra` means an older binary will preserve rather
than delete the new field on a downgrade. `tests/spawn_seam_guard.rs` must stay green
unmodified — if it needs weakening, the design is wrong.

---

## Claude's Discretion

Delegated to planning/research by default in an unattended run, and recorded as such in
CONTEXT.md: naming of the runtime-identity type and the journal label strings, the volume
name, the argv flag spelling, whether bollard is a dependency or the socket is spoken
directly, and all test/plan decomposition.

## Deferred Ideas

- Remote/CI execution targets — a third `ExecutionTarget` variant, not this phase.
- A second `Executor` implementor — rejected by `src/executor/mod.rs:78` and by the ROADMAP.
- Publishing the container image as a distributed release artifact.
- `ps --format json` normalization — **deleted**, not deferred (spike § F4).
- All 14 todo matches reviewed, **none folded**; the matcher scored on generic keywords and
  no match was container-related. One of them is Phase 23's CTRL-08 verbatim. Folding at the
  `--auto` threshold would have been scope creep, so the literal `--auto` rule was
  deliberately not applied — recorded in CONTEXT.md `<inferred_decisions>`.
