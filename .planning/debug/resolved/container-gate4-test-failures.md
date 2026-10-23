---
status: resolved
trigger: "Release gate `./scripts/pre-tag-check.sh --container vX.Y.Z` (builds scripts/pre-tag-check.Dockerfile, an ubuntu-24.04 lookalike of the publish runner in .github/workflows/release.yml) FAILED gate 4 (cargo test) with 3 tests that pass locally: driverctl_cli wr08 full-backlog EMFILE, driver_run read_log_delta FIFO not skipped, app launcher_new_run footer 'listing never reported' (also flaky locally under load). Make them correct and robust on the GitHub ubuntu-24.04 runner and in the container; fix root cause in test or product code."
created: 2026-10-23
updated: 2026-10-23
---

# Debug: three tests fail only in the pre-tag container (gate 4)

## Symptoms

- **Expected:** `./scripts/pre-tag-check.sh --container v1.15.0` reports GATE 4 (cargo test
  --no-fail-fast) PASS, as on the local machine (where only the known envelope::policy
  git-version witness fails, which is expected locally and green in container/CI).
- **Actual:** three tests fail inside the container:
  1. `driverctl_cli::tests::wr08_connect_to_a_full_backlog_times_out_instead_of_blocking`
     (bin target) panicked at src/driverctl_cli.rs:586:21 —
     `connect: Too many open files (os error 24)`. Likely container nofile ulimit, but the test
     must not depend on how high the fd limit is.
  2. `ui::screens::driver_run::tests::read_log_delta_skips_a_fifo_log_without_blocking` (lib)
     panicked at src/ui/screens/driver_run.rs:5941:9 with
     `LogRead { run_id: "20261020-log", from_offset: 36, cursor: TailCursor { offset: 36 }, ident: Some((93, 3015384)), lines: [], reset: false, missing: false }`.
     Need to find WHY the FIFO behaves differently in the container: overlayfs, running as root,
     filesystem type, or something else. Possibly a real product bug in FIFO handling.
  3. `app::tests::launcher_new_run_the_footer_names_launch_or_relaunch_for_the_selected_row`
     (lib) panicked at src/app.rs:6271:25 — `the launcher listing never reported`. Also flakes
     locally under heavy parallel load -> probably a timing/timeout assumption.
- **Errors:** as above. Full log:
  /tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/12f30172-218e-48ae-8121-ca7a11744d59/scratchpad/pretag.log
  (failures near lines 3405, 3873, 4141-4185; gate headers at 1049/1056/1099/1125).
- **Timeline:** surfaced on the first container dry run of the v1.15.0 release.
- **Reproduction:** build the image from scripts/pre-tag-check.Dockerfile (see how
  scripts/pre-tag-check.sh builds/runs it: mounts, user, target dir /cargo-target) and run the
  individual tests inside it, e.g. `cargo test --bin gsd-meta-manager wr08_connect_to_a_full`,
  `cargo test --lib read_log_delta_skips_a_fifo`, and stress-loop the launcher test
  (`cargo test --lib launcher_new_run_the_footer -- --test-threads=N`, repeated, ideally with
  the whole lib suite in parallel to reproduce load).

## Constraints (from the coordinator; human unavailable)

- Branch master; DO NOT push (origin is public). No tags. Do not touch Cargo.toml version
  (stays 1.14.0) and leave `git stash` stash@{0} (the 1.15.0 bump) alone.
- Fix root cause in test or product code; don't paper over a product bug. Change the
  Dockerfile only if it genuinely diverges from the GitHub ubuntu-24.04 runner.
- Inferred decisions marked `[INFERRED — audit]`.
- Verification: in-container targeted tests (stress the flaky one); local
  `rtk proxy cargo test --no-fail-fast` (only envelope::policy git-version failure expected),
  `cargo clippy --all-targets` 0 warnings, `cargo fmt --check`; finally
  `./scripts/pre-tag-check.sh --container v1.15.0` must show GATE 4 PASS (gate 1 fails on the
  tag/version mismatch — expected).
- Every commit carries trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Note: rtk filters cargo output; use `rtk proxy` for any check that greps raw output, and
  remember a piped grep downstream of proxy can still be truncated by rtk.

## Current Focus

- bug_class: Bohrbug for tests 1 and 2 (deterministic given the environment); test 3 is
  load-dependent (Mandelbug-ish) and was handled by stress loop + probe.
- status: RESOLVED. All three root causes confirmed by measurement and fixed; container gate 1-5 PASS.
- next_action: none (archived).

```yaml
reasoning_checkpoint:   # written after the fix was applied, before verification (see note below)
  hypothesis: "Three tests encode an environment assumption that holds on the dev host and not in the
    container/runner: (1) wr08 needs RLIMIT_NOFILE > net.core.somaxconn, (2) the FIFO test builds a
    'replaced log' by unlink+create and assumes the inode number changes, (3) the new-run footer test
    waits for a launcher listing that its own helper may already have discarded."
  confirming_evidence:
    - "(1) container soft nofile 1024 vs somaxconn 4096; test binary FAILS at ulimit -n 1024/4000 and PASSES at 8192/65536 (same binary)"
    - "(2) first.ident == replaced.ident == (93, 2910379) on overlayfs, 4/4 FAIL; tmpfs gives (170,6)->(170,7), PASS; unlink+create reuses the inode 20/20 on overlay/ext4, 0/20 on tmpfs and zfs; stage+rename 0/20 everywhere"
    - "(3) 9/9 failures coincide with a LauncherRunsRefreshed being discarded by next_matching, 0/81 passes did"
  falsification_test: "(1) after the fix the test must pass at ulimit -n 32; (2) after the fix it must pass 50/50 on overlay alone and under load; (3) after the fix 0 failures in >=100 loaded iterations where the old code failed ~12-23%. A product mutant in connect_within (blocking connect) must still be caught by (1)."
  fix_rationale: "All three are defects in the TESTS' construction of their scenario, not in product behaviour: the product contracts they pin (connect_within times out on a full backlog; a replaced LOG.md with a new inode resets the tail; the launcher listing reports) are correct and are exercised unchanged. Each fix removes the environment assumption instead of widening a margin."
  blind_spots: "The GitHub runner's RLIMIT_NOFILE/somaxconn and /tmp filesystem could not be read from public sources; the fixes are independent of them by construction, but the runner itself was not run. delete+recreate of a log under a reused inode is still invisible to the (dev, ino)+length identity (documented limitation, out of scope)."
  candidate_causes:
    - "(1) code: test holds one fd per queued connection / environment: RLIMIT_NOFILE 1024, somaxconn 4096"
    - "(2) code: unlink+create used to fake a replacement / environment: ext4+overlayfs reuse freed inode numbers, tmpfs does not / data: same-length content makes the length check blind"
    - "(3) code: next_matching drops an action the caller later awaits / environment: scheduling order of two spawn_blocking tasks under 4-cpu parallel load"
  and_gate: "yes for all three: each fails only when a test-design condition (code) AND an environment condition co-occur; both recorded in root_cause. Removing the code half makes each independent of the environment half."
```

NOTE (honesty): the checkpoint block above was committed to the file after the edits were applied, not before.
The evidence it cites was all gathered before the edits; no edit was made on a guess.

## Evidence

- timestamp: 2026-10-23
  checked: container (`docker run` exactly as pre-tag-check.sh does it; helper at scratchpad/inc.sh)
  found: uid=1000, `ulimit -n` soft 1024 / hard 524288, /proc/sys/net/core/somaxconn=4096, /tmp is
  overlayfs (on ext4), /work is zfs bind mount, /cargo-target is ext4 volume. HOST: soft=hard=524288,
  /tmp is tmpfs.
  implication: host has ~500x the fd headroom the container does; /tmp differs in fs type.
- timestamp: 2026-10-23
  checked: test 1 reproduced in container (`cargo test --bin gsd-meta-manager wr08_connect_to_a_full`),
  then the built test binary re-run under `ulimit -n` 1024 / 4000 / 8192 / 65536.
  found: FAIL at 1024 and 4000 (src/driverctl_cli.rs:586, `connect: Too many open files (os error 24)`),
  PASS at 8192 and 65536.
  implication: CONFIRMED. The test holds every successful client stream in `held` and needs more than
  somaxconn(4096) fds before the backlog fills. Note line 586 is the `assert!(detail.contains("no
  connection within"))`: EMFILE comes back from connect(2) as a Transport error, so the assert, not the
  connect, is what panics. The test depends on `RLIMIT_NOFILE > somaxconn` — two environment knobs.
  The GitHub runner's nofile could not be established from public sources (not needed: test must be
  independent of it).
- timestamp: 2026-10-23
  checked: panic site of test 2 (src/ui/screens/driver_run.rs:5941).
  found: line 5941 is `assert!(replaced.reset, ...)` — the SECOND half of the test (regular log tails
  incrementally, replaced log resets). The FIFO assertions at 5922-5926 sit above it and PASSED.
  The replacement is simulated with `remove_file(&log)` + `write(&log, ...)`; both versions of the
  log are exactly 36 bytes (same-length event header, "one" vs "two"), the cursor from the first read
  is at 36, and the failing LogRead shows the same ident-in-effect with from_offset 36 / lines [] /
  reset false.
  implication: FIFO handling is NOT the failure. The test needs the recreated file to get a NEW inode.
- timestamp: 2026-10-23
  checked: inode reuse after unlink+create, measured with 20 trials per mount (scratchpad/inode.sh).
  found: overlayfs /tmp (container) 20/20 reused; ext4 /cargo-target 20/20; host ext4 /var/tmp 20/20;
  host tmpfs /tmp and /dev/shm 0/20; zfs /work 0/20. Stage-new-file-then-rename-over (old inode still
  alive while the new one is allocated) reused 0/20 on EVERY filesystem.
  implication: CONFIRMED test artefact. tmpfs hands out monotonically increasing inode numbers, so the
  host (tmp on tmpfs) passes; any ext4/overlay /tmp (container, and the GitHub runner's ext4 root) gets
  the same inode back, so `(dev, ino)` is unchanged and the 36-byte replacement is invisible to the
  identity check. The sibling test `launcher_log_a_replacement_is_detected_by_device_and_inode`
  (driver_run.rs:5647) already does it correctly (stage + rename).

- timestamp: 2026-10-23
  checked: test 2 run directly in the container with a temporary eprintln of `first.ident` /
  `replaced.ident` (reverted), on overlay /tmp and on a tmpfs TMPDIR; plus the same test looped.
  found: overlay: `first=Some((93, 2910379)) replaced=Some((93, 2910379))` 4/4 runs -> FAILED at
  driver_run.rs:5941:9 (same dev, same ino). tmpfs: `(170, 6)` -> `(170, 7)`, `(170, 11)` -> `(170, 12)` ->
  PASS. zfs TMPDIR (/work) also PASS. Looped 50x alone on overlay: 50/50 failures except a one-off first
  run in a fresh container that happened to pass (allocator state); under load 26/50 (concurrent tests
  steal the freed inode some of the time). A shell replay of the test's exact create/remove/create
  sequence (scratchpad/seq.sh) reused the inode 6/6 on overlay.
  implication: CONFIRMED beyond doubt. TEST ARTEFACT: the test builds "a replaced log" with
  remove_file + write, which does not guarantee a new inode. Not a FIFO problem and not a product bug:
  the product's contract is "a new file renamed over LOG.md has a new inode", which is exactly what the
  sibling test `launcher_log_a_replacement_is_detected_by_device_and_inode` exercises. The same-length
  header ("one"/"two") makes the length check blind too, so nothing else can flag the replacement.
  Not claimed: that dev+ino identity is perfect. delete+recreate under a reused inode is a documented
  limitation of the identity design, out of scope here [INFERRED — audit].
- timestamp: 2026-10-23
  checked: test 3 (src/app.rs:6271 is `None => panic!("the launcher listing never reported")` in
  `deliver_launcher_refresh`; `next_action` is a 10 s timeout). Read `create_a_run` (app.rs:10373),
  `finish_new_run` (app.rs:1640-1680), `schedule_launcher_refresh` (ui/screens/mod.rs:2128),
  `next_matching` (app.rs:9117).
  found: `app.update(DriverNewRunCreated)` runs `finish_new_run`, which starts TWO independent
  spawn_blocking tasks: the launcher listing (`schedule_launcher_refresh` -> LauncherRunsRefreshed) and
  `request_edit` (-> DriverEditReady). `create_a_run` then waits with `next_matching(DriverEditReady)`,
  which DISCARDS every other action. If the listing lands before DriverEditReady it is thrown away; the
  test then calls `deliver_launcher_refresh`, which waits for a listing nobody will send again, times
  out after 10 s and panics. Order of the two tasks depends on scheduling, hence load-dependent.
  implication: hypothesis (3) "fixed timeout" was close but wrong in kind: not too short a timeout, a
  lost message.
- timestamp: 2026-10-23
  checked: stress loop in the container (scratchpad/stress.sh, whole lib suite x3-4 in parallel as load,
  4 cpus) of the single test; then a temporary probe in `next_matching` (reverted) printing when a
  LauncherRunsRefreshed/LauncherRunRefreshed is discarded, correlated with the verdict.
  found: no load: 0/40 failures. Load x3: 6/30 failures. Probe runs: (load x3, 30 iters) discarded&FAIL=2,
  discarded&pass=0, notdiscarded&pass=28, notdiscarded&FAIL=0; (load x4, 60 iters) discarded&FAIL=7,
  discarded&pass=0, notdiscarded&pass=53, notdiscarded&FAIL=0.
  implication: CONFIRMED: 9/9 failures coincide with the discarded listing and 0/81 passes had one.
- timestamp: 2026-10-23
  checked: every other `deliver_launcher_refresh` call site (17) and every other `create_a_run` caller.
  found: all other sites schedule the listing and deliver it with nothing draining the channel in
  between; only app.rs:10769-10770 drains (via `create_a_run`'s final `next_matching`) between the
  scheduling and the delivery.
  implication: a single latent race site; no sweep needed.

- timestamp: 2026-10-23
  checked: first container gate run after the fix (scratchpad/pretag-v1.14.0.log).
  found: gate 4 and gate 5 FAILED with "No space left on device" (os error 28), rustc-LLVM ERROR and a
  linker Bus error: the `gsdmm-pretag-target` volume (ext4 /dev/zd0, shared with /var/lib/docker) was at
  100% (585 MB free). `/cargo-target/debug` was 115 GB: 90 GB of stale `deps/` test binaries (cargo
  never garbage-collects old hashes; 186 stale `gsd_meta_manager-*`, 88 `launcher_degraded-*` ...) plus
  25 GB `incremental/`. My own probe builds were the last straw, but the accumulation predates this
  session.
  implication: environmental, NOT a test failure. Pruned `incremental/` and every `deps/` file > 50 MB
  older than 1 day inside the volume (a pure build cache; costs a recompile, loses nothing) ->
  93 GB free. [INFERRED — audit]: deleting cache in a named volume owned by this script is reversible,
  so no escalation. Follow-up worth a quick task: nothing in pre-tag-check.sh ever prunes the volume.
- timestamp: 2026-10-23
  checked: `./scripts/pre-tag-check.sh --container v1.14.0` (scratchpad/pretag-v1.14.0-run2.log).
  found: gate 1 PASS, gate 2 (MSRV 1.89.0 check --all-targets --locked) PASS, gate 3 PASS, gate 4 PASS
  (4201 passed, 0 failed, 24 ignored across all suites; the git-version witness also passes in there:
  banner reads MATCH, git 2.55.0), gate 5 PASS, exit 0.
  `./scripts/pre-tag-check.sh --container v1.15.0` (pretag-v1.15.0.log): gate 1 FAILED
  (tag v1.15.0 != crate 1.14.0) and the script HARD-STOPS there, gates 2-5 NOT REACHED.
  implication: the literal v1.15.0 invocation cannot reach gate 4 while Cargo.toml is 1.14.0 (the 1.15.0
  bump lives in stash@{0}, which was left alone as instructed). So gate 4 was certified with the tag of
  the version actually in the manifest. [INFERRED — audit]. The earlier failing run (pretag.log) had
  Cargo.toml at 1.15.0, which is why it reached gate 4.

## Eliminated

- hypothesis: FIFO detection/skip differs on overlayfs or as non-host user (the original premise).
  evidence: the FIFO assertions (driver_run.rs:5922-5926) pass in the container; the failing assertion is
  the replaced-log reset at driver_run.rs:5941. uid 1000 in the container, so root is also out.
  timestamp: 2026-10-23
- hypothesis (3): a fixed timeout in the test is too short for a loaded machine.
  evidence: `next_action` waits 10 s and the listing takes milliseconds; failures coincide 9/9 with the
  listing having been delivered and discarded earlier, not with it being slow.
  timestamp: 2026-10-23

## Resolution

- root_cause: THREE independent TEST-construction defects, each an AND of a code condition and an
  environment condition that the dev host (tmpfs /tmp, nofile 524288, idle cpus) never satisfied
  together. None is a product bug.
  (1) wr08 full-backlog: the test held one client fd per queued connection and `bind` listens with the
  kernel cap (somaxconn 4096), so it needed RLIMIT_NOFILE > 4096; the container's soft limit is 1024 ->
  EMFILE came back from connect(2) as a Transport error and failed the `contains("no connection
  within")` assert (driverctl_cli.rs:586).
  (2) read_log_delta "FIFO" test: the FIFO half passes everywhere; the failing assert is the second half
  (driver_run.rs:5941). The test faked "a replaced log" with remove_file + write; ext4 and overlayfs
  hand the freed inode number straight back (first.ident == replaced.ident == (93, 2910379)), and with
  same-length content nothing else flags the change; tmpfs never reuses numbers, which is why it
  passed locally.
  (3) launcher new-run footer: `finish_new_run` starts two independent spawn_blocking tasks (launcher
  listing, editor prepare); `create_a_run`'s `next_matching(DriverEditReady)` discards every other
  action, so when the listing lands first it is thrown away and the test's following
  `deliver_launcher_refresh` waits 10 s for a listing nobody will resend ("the launcher listing never
  reported", app.rs:6271). Load-dependent ordering.
- fix: (1) src/driverctl_cli.rs: after `bind`, call `rustix::net::listen(&listener, 1)` so the queue is
  two connections regardless of somaxconn or RLIMIT_NOFILE; loop bound 10_000 -> 16. Cargo.toml: dev-only
  `rustix = { version = "1.1", features = ["net"] }` (no new crate, Cargo.lock untouched, `net` absent
  from the normal/release graph).
  (2) src/ui/screens/driver_run.rs: stage `LOG.md.new` and rename it over LOG.md (old inode alive while
  the new one is allocated => distinct on every filesystem), plus a precondition `assert_ne!` on
  (dev, ino) so the test says so if that ever stops holding. Mirrors the sibling
  `launcher_log_a_replacement_is_detected_by_device_and_inode`.
  (3) src/app.rs: the test schedules its own listing (`app.ctx.schedule_launcher_refresh`) after
  `create_a_run` instead of depending on the one the helper may have discarded; the run is on disk by
  then, so any listing shows it. Product code, Dockerfile and scripts unchanged.
- verification:
    target_test:
      result: pass
      detail: "container (uid 1000, soft nofile 1024, overlay /tmp, 4 cpus): wr08 passes at ulimit -n 32/64/1024
        (was FAIL at 1024 and 4000); FIFO test 0/50 failures alone and 0/50 under 3 parallel lib suites on
        overlay (was 50/50 and 26/50); launcher footer test 0/100 failures under 4 parallel lib suites
        (was 6/30 and 7/60). local: all three pass."
    mutation_check:
      result: pass
      detail: "Stryker n/a (Rust). Hand-seeded mutants at the product code each test pins, run with the new
        tests: connect_within `recv_timeout(limit)` -> `recv()` => wr08 hangs (killed by `timeout 20`, exit
        124, in container); read_log_delta `ident` mismatch reset disabled (`false && ...`) => FIFO test
        fails at the `replaced.reset` assert (driver_run.rs:5964, local). Test 3 pins no product mutant
        (its fix is in the harness); its before/after stress numbers are the evidence."
      mutant_killed: true
    no_op_deletion:
      result: pass
      deletion_justified_by_rca: false
      detail: "No behaviour or assertion removed: wr08 keeps the same timeout assertion and detail-text
        check; the FIFO test gains a precondition assert; the footer test keeps every assertion and only
        changes how it obtains its listing."
    adjacent_tests:
      result: pass
      suites_run: "local `rtk proxy cargo test --no-fail-fast`: 4200 passed, 1 failed (the expected
        envelope::policy git-version witness, local git != 2.55.0); container gate 4: 4201 passed, 0 failed
        (witness included, git MATCH)"
    revert_and_reconfirm:
      result: pass
      bug_returned_on_revert: true
      fixed_on_reapply: true
      detail: "Pre-fix measurements are the revert state (same tests, unmodified): wr08 fail at nofile 1024/4000,
        FIFO test 50/50 fail, footer test 6/30 and 7/60 fail; post-fix the same harness gives 0 failures.
        Mutation reverts verified by `git diff --stat` matching the intended diff."
    oracle_type: derived (the contracts are the product's own: connect_within bounded by its limit; a new
      inode resets the tail; the launcher listing reports). Boundary neighbours: wr08 now exercises
      backlog+1 (3rd connect blocks) deterministically; the replacement test keeps the same-length
      worst case that makes the inode the only discriminator.
    container_gate: "./scripts/pre-tag-check.sh --container v1.14.0 -> gates 1,2,3,4,5 all PASS, exit 0.
      --container v1.15.0 hard-stops at gate 1 (manifest is 1.14.0; bump is in stash@{0})."
    local_gates: "cargo clippy --all-targets: 0 warnings; cargo fmt --check: clean."
    guardrail_verdict: accepted
    human_verification: "[INFERRED — audit] the human is unavailable; the container gate (the exact check
      the human asked to pass) substitutes for the human-verify checkpoint. Session archived on that basis."
- files_changed: [src/driverctl_cli.rs, src/ui/screens/driver_run.rs, src/app.rs, Cargo.toml]
- not_changed_on_purpose: scripts/pre-tag-check.Dockerfile and scripts/pre-tag-check.sh. The container's
  soft nofile (1024) and /tmp (overlay) may or may not match the GitHub runner (could not be established
  from public sources), and the fixed tests no longer depend on either, so there is no measured divergence
  to correct. [INFERRED — audit]

## Prevention (blameless)

- Branching 5-Whys. Code branch: the tests built their scenario from a raw OS side effect (fill a backlog by
  holding fds; "replace" a file by unlink+create; wait for an action another helper may have consumed)
  instead of the construct that is portable across environments. Environment branch: the dev host is
  unusually forgiving on all three axes (524288 fds, tmpfs /tmp, idle cpus) and the only
  ubuntu-24.04 lookalike (the pre-tag container) had never been run on this tree before the v1.15.0
  release. Data branch: the same-length "one"/"two" log headers removed the length check as a second
  line of defence.
- Why not caught: the pre-tag container gate IS the gate that should catch this class, and it did, but
  it runs only at release time and release.yml runs only on a tag push, so there was no per-commit
  signal. Local `cargo test` was green by construction on tmpfs.
- Recurrence guard: the three fixed tests themselves, which now carry no environment assumption:
  src/driverctl_cli.rs:`wr08_connect_to_a_full_backlog_times_out_instead_of_blocking` (self-chosen
  backlog; passes at nofile 32), src/ui/screens/driver_run.rs:`read_log_delta_skips_a_fifo_log_without_
  blocking` (stage+rename and a precondition assert that fails loudly if the inode is shared),
  src/app.rs:`launcher_new_run_the_footer_names_launch_or_relaunch_for_the_selected_row` (explicit
  listing). Plus the KB entry below. Suggested but NOT done (scope): run the container gate periodically
  rather than only at tag time, and have pre-tag-check.sh prune the cargo-target volume.
