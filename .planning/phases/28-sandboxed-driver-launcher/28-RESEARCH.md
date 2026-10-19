# Phase 28: Sandboxed Driver Launcher — Research

**Source:** gsd-phase-researcher (Opus) run, 2026-10-19. The researcher's own write of this file was
interrupted twice by a tooling safety stop. The orchestrator transcribed this file from the researcher's
returned summary, without adding findings. Probe procedures are omitted on purpose; claims are
stated at the level of "channel reachable / closed by".

## Summary

Phase 28 is buildable with no new crates. Three findings change the design that CONTEXT.md
assumed:

1. The spike sandbox profile needs to be hardened.
2. Phase 27 lacks several store and serve hooks that the launcher needs.
3. The push step has to run from a launcher-owned mirror.

## 1. Sandbox profile (affects D-19, SC3, SC5)

- **Finding:** under the spike-002 bwrap profile that D-19 ports, host services in the user's
  `$XDG_RUNTIME_DIR` stay reachable from inside the sandbox. That includes the SSH agent socket
  and the user systemd manager. Probe evidence confirmed both channels on this host. SC3 and SC5
  ("push inside the sandbox fails for lack of credentials") therefore do NOT hold under the spike
  profile.
- **Mitigation, verified by probe:** mount a tmpfs over `$XDG_RUNTIME_DIR` inside the sandbox, then
  re-bind only the driverctl socket at its original path. This closed both channels.
  **Treat it as part of D-19, not as an option.** `[inferred — audit]`
- Mask paths that do not exist make bwrap abort, so masks must be conditional on the path
  existing.
- A copied bwrap binary fails under Ubuntu's AppArmor userns rule. Availability detection must
  actually execute bwrap with a trivial command, not just check `which`.

## 2. systemd-run and scope behaviour (host-confirmed)

- On this host `systemctl --user is-system-running` prints `degraded` and exits 1. Detection must
  parse the output: `running` and `degraded` both count as usable. The exit code alone is not
  enough.
- `RuntimeMaxSec` on a `--user --scope` works; it timed out at 3 s in a probe. That makes it a
  valid wall-clock backstop.
- Without `--collect`, failed scopes stay loaded. Always pass `--collect`.
- `systemctl --user stop <scope>` killed a `setsid` grandchild inside bwrap, so stopping the scope
  kills all descendants (SC1).

## 3. Claude Code configuration inside the sandbox

- Claude Code reloads project settings mid-session. Left alone, the driver could edit project
  settings and switch off its own gate hooks. Pass `--setting-sources user` so that only the
  per-run `CLAUDE_CONFIG_DIR` user settings apply. `[inferred — audit]`
- Auth precedence: `ANTHROPIC_AUTH_TOKEN` and `apiKeyHelper` both win over
  `CLAUDE_CODE_OAUTH_TOKEN`. Strip `ANTHROPIC_AUTH_TOKEN` from the env and make sure the per-run
  settings contain no `apiKeyHelper`, along with `ANTHROPIC_API_KEY`, `GH_TOKEN` and
  `SSH_AUTH_SOCK` (SC3/SC4).
- The hook field `onFailure: "block"` needs Claude Code >= 2.1.295 (this host has 2.1.303).
  Gate on the version, or document the minimum.

## 4. Push step (SC5, LNCH push requirement)

- The driver can rewrite `.git/config` and hooks in the checkout it drives. A read-only bind of
  `.git/config` does not help, because the driver can rename `.git`. **The host-side push must
  run from a launcher-owned mirror (fetch from the driven checkout into a mirror gsd-mm owns, then
  push with a fixed refspec into the reserved namespace). Never push from the driven checkout
  itself.** Reuse Phase 19's secret scan, protected-branch refusal and PR cap on the mirror's refs.
  `[inferred — audit]`

## 5. Phase 27 gaps the launcher needs (land after the parallel CR-01 fix)

- `TerminalStatus` is only `Crashed | Stalled | Stopped` (`src/driverctl/store.rs:246-250`). It
  needs a `Parked` variant (or an equivalent) for the rate-limit, auth and secret-scan parks.
- No store method writes `cap-tripped`, records a launch end without a status change, updates the
  wall-clock `used`, or sets launcher-owned keys. All four must be added.
- The accept loop and ticker in `serve.rs` are private, so the launcher cannot embed or drive
  them. They need a pub(crate) entry point.
- No TUI code reads Phase 27 run dirs, so D-11 ("stop a selected driver run" from the TUI) has
  nothing to select yet. A minimal TUI listing or selection of driver runs is needed.
- The gap count comes from the Phase 27 store's `caps.gap_rounds.per_phase`, via its cap hook
  point. Phase 28 owns only the gate, trip and kill.

## 6. Repo guards that plans must satisfy

- `tests/spawn_seam_guard.rs`: every file containing `Command::new` must be on its allowlist. Put
  all launcher process spawns (bwrap, systemd-run, systemctl, git for the mirror and push, claude
  setup-token) in ONE module file, which then needs only one allowlist entry.
- The frontmatter goldens are byte-exact. Emit the D-30 `sandbox` key only when it is set, so the
  existing goldens stay unchanged.
- CI runs `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings`. Every test
  that needs live bwrap, systemd or claude must be `#[ignore]`, with a documented command for
  running it.
- Use `rtk proxy cargo ...` for verify commands (see the project memory on rtk filtering).

## 7. Acceptance harness timing (SC2, D-33 carried risk)

- In the L3/L4 cases, gap writes must be spaced more than 45 s apart, because `COALESCE_S = 45` in
  the Phase 27 gap store. Writes closer together coalesce into a single round.
- D-33 (real Opus driver, divergence rule disabled): the kill half is likely to be missed. Opus
  parks within seconds of the deny, so a `gaps_found` injected later arrives after the scope is
  gone. **Fix: inject the next `gaps_found` right at the deny, with a forward mtime**, so that
  limit+2 is reached before the driver parks. `[inferred — audit]`

## 8. Dependencies

- No new crates. rustix 1.1.5 (already a dependency) provides `setsid`, `getpgid`,
  `set_child_subreaper` and pdeathsig. Degraded-mode (unsandboxed) process supervision can use
  these: a process group plus a subreaper, with kill-on-cap.

## 9. Validation architecture (summary)

- Unit: arg/profile builders (the bwrap argv, systemd-run argv, env scrub list) are pure
  functions, tested with golden argv assertions.
- Integration (non-ignored): fake-binary shims on PATH for bwrap, systemd-run and systemctl
  record argv. They prove refusal without bwrap/systemd (SC6), the flock refusal and the
  `crash-detected` relaunch (SC1), and that the posture is recorded.
- Live (`#[ignore]`): the real bwrap/systemd isolation assertions (SC3 checklist, including the
  `$XDG_RUNTIME_DIR` channel closure), the scope kill of descendants, and RuntimeMaxSec. The
  spike-002 L1-L5 cases come in two forms, a scripted stand-in and a real-Opus D-33 run.

## 10. Suggested decomposition (for the planner, non-binding)

1. Phase 27 store/serve extensions: Parked, cap-tripped, launch-end, wall-clock used, launcher
   keys, pub serve entry, and the optional `sandbox` key.
2. Host detection plus the sandbox/scope argv builders and the spawn module (hardened profile,
   env scrub, per-run CLAUDE_CONFIG_DIR, `--setting-sources user`).
3. The launch/stop CLI, the flock, crash-detected, the cap gate/trip/kill wiring, and the
   rate-limit park.
4. Auth onboarding (setup-token storage) and the revoked-token fallback relaunch.
5. The push-from-mirror step that reuses Phase 19.
6. Degraded mode, posture recording and display in the TUI, and the TUI run selection and stop.
7. The acceptance harness: L1-L5 and the D-33 real-driver run.
