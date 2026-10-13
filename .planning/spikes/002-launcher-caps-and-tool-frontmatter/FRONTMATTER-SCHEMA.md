# Driver-run frontmatter schema v1 (tool-written)

Lives at the top of `.planning/meta-manager/drivers/<run-id>/DIRECTION.md`, between the first two
`---` lines. **Written only by driverctl** (the launcher-side server) - never by a model, never by
hand. It is a *derived view*: regenerated from `state.json` (tool-owned) plus DIRECTION.md's body
(for `control` and the item list) on every run event. Heartbeat-only changes are coalesced to one
rewrite per 120 s; `state.json` always has the fresh value.

A block counts as the tool's only if it carries the `generated_by: driverctl` marker; any other
leading block is treated as body and preserved.

| Field | Type | Source (who decides) | Meaning |
|---|---|---|---|
| `generated_by` | string | tool | `driverctl <version>` + do-not-edit comment. Marker for the splice |
| `driver_frontmatter_version` | int | tool | Schema version (1). Readers reject unknown majors |
| `run_id` | string | launcher | Run dir name, stable across launches |
| `status` | enum | launcher + `driverctl log end` | `starting` `running` `paused` `parked` `stopped` `done` `crashed` |
| `status_reason` | string/null | same | `driver:<end-reason>`, `launcher-cap:<cap>`, `driver exited rc=N without RUN end` |
| `control` | string/null | DIRECTION.md body | Echo of `- control:` (run/pause/stop) at the last regeneration |
| `current_item` / `current_phase` / `current_step` | string/null | `log dispatched` | What the last dispatch was for |
| `items` | map | state.json + DIRECTION item list | `total` + count per item state (`pending running done parked failed`) |
| `caps.dispatches` | `{used, limit}` | PreToolUse hook (driver-level Agent calls, no `agent_id`) | Counted by the tool, not by the driver's own log |
| `caps.wall_clock_min` | `{used, limit}` | launcher | Active minutes summed over launches |
| `caps.gap_rounds` | `{used, limit, phase}` | launcher scan of `*-VERIFICATION.md` | Distinct `status: gaps_found` writes for the worst phase |
| `dispatch_audit` | `{logged, observed, denied}` | tool | `logged` = `driverctl log dispatched` calls; `observed` = hook count; `denied` = gate refusals. `observed > logged` means the driver dispatched without recording it |
| `launches` | int | launcher | Number of launches of this run |
| `connected[]` | list of flow maps | launcher + hooks | One entry per process: `role` (launcher/driver), `state` (connected/disconnected/lost), `session_id` (SessionStart hook), `model`, `mode` (bwrap/none for the launcher, headless/interactive for the driver), `pid` (host pid), `scope_unit`, `host`, `started_at`, `last_heartbeat` |
| `heartbeat_stale_after_s` | int | tool | Reader threshold (300) |
| `last_event` | `{at, item, event, summary}` | tool | Last LOG.md event header, summarised |
| `updated_at` | string | tool | ISO-8601 UTC of this regeneration |

## Liveness rules for readers (gsd-mm)

1. `status` not `running`/`starting` -> trust it.
2. `status: running` and `launcher.lock` in the run dir is **not** flock-held (probe with a
   non-blocking `flock`) -> **crashed** (the launcher is gone; die-with-parent took the scope with
   it). This check needs no clock and no pid, so it is immune to pid-namespace reuse.
3. Lock held and newest `connected[].last_heartbeat` older than `heartbeat_stale_after_s` ->
   **stale** (alive but not making tool calls: a long API call, a hung hook, a rate-limit wait).
   For a fresh value read `state.json` rather than the coalesced frontmatter.

## Parsing in gsd-mm

`state_reader::state_md::extract_frontmatter` (`src/state_reader/state_md.rs:325`) plus
`serde_yml::from_str` into a `#[derive(Deserialize)]` struct parses it unchanged - verified in a
throwaway crate against five generated files (see README). Top-level scalars (`status`, `run_id`,
`control`, `current_item`, `updated_at`) are also readable with the existing
`disk_status::leading_frontmatter_value`. Emitter rules that keep it parseable: strings are JSON
double-quoted (a valid YAML double-quoted scalar), nested maps are single-line flow maps, no value
ever contains a newline, and the closing `---` is always on its own line.
