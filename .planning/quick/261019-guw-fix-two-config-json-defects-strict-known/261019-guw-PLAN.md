---
quick_id: 261019-guw
mode: quick
phase: quick-261019-guw
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_write.rs
  - src/state_reader/mod.rs
  - src/ui/screens/detail.rs
  - src/state_reader/config_json.rs
  - docs/GSD-CORE-SYNC.md
autonomous: true
requirements: [QUICK-261019-guw]

estimate:
  tokens: 130000
  raw_tokens: 130000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Every TUI save of `.planning/config.json` goes through `state_reader::config_write`. That covers all five `persist_active_config` call sites in src/ui/screens/detail.rs: dropdown Enter, Integer Enter, `x` clear, Secret Enter and String Enter. Each save creates `.planning/.lock` with O_EXCL and gsd-core's body `{\"pid\":N,\"cwd\":\"<project root>\",\"acquired\":\"<ISO-8601 ms Z>\"}`. It then re-reads config.json inside the lock, applies the one edit to that fresh copy, writes it as a same-directory temp file with fsync, carries over the target's permission bits, renames it into place, and releases the lock."
    - "If a live process holds `.planning/.lock` for the whole 2 s TUI budget, a Defaults edit writes nothing. config.json stays byte-identical, the lock file stays byte-identical, the cached config still equals disk, and the status line reads `Config NOT saved` and names the holder pid. The TUI stays usable."
    - "A lock is stolen only when its recorded holder is not verified-live: a dead pid, a body that does not parse, or a pid that is missing or not a positive integer. A lock body that cannot be read, such as a symlink, FIFO or oversized file, is never stolen. Stealing uses a (dev, ino, body) re-confirm followed by rename-aside, as gsd-core does. A verified-live holder is never stolen at any age."
    - "Saves to `$GSD_HOME/.gsd/defaults.json` (Global scope) take `$GSD_HOME/.gsd/gsd-install-migration.lock` with gsd-core's installer semantics: body `{\"pid\":N,\"acquiredAt\":\"...\"}` plus a newline, a dead or own pid is reclaimed by unlink, and an unparseable body is never reclaimed. They use the same atomic write."
    - "A concurrent reader never sees a partial config file during a save. An existing file's mode bits (for example 0600 on a config holding API keys) survive the save, and no `*.tmp-*` or `.lock.stale-*` file is left behind."
    - "`capabilities.strict_known_registries` behaves as follows. A string array (including `[]`), an explicit `null`, an absent key and a legacy bool (written by pre-fix builds) all parse without making the whole config unreadable. Each round-trips through a save verbatim, and an absent key stays absent. The Defaults row is read-only, and no set, toggle or clear path can write it."
    - "`rtk proxy cargo test --no-fail-fast` fails only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, and `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0."
  artifacts:
    - path: src/state_reader/config_write.rs
      provides: "gsd-core-compatible lock acquisition (planning + installer flavors), bounded lock-body reads, atomic permission-preserving write, and update_gsd_config read-modify-write under the lock. This is the write path Phase 32 reuses."
      contains: "gsd-install-migration.lock"
    - path: src/state_reader/config_json.rs
      provides: "strict_known_registries modelled as upstream array|null, tri-state lossless, with a Malformed catch-all"
      contains: "RegistryAllowlist"
    - path: src/ui/screens/detail.rs
      provides: "persist_active_config routed through config_write with a closure-applied edit and a non-fatal error status; read-only strict_known_registries row with no mutation arms"
      contains: "update_gsd_config"
    - path: docs/GSD-CORE-SYNC.md
      provides: "Capabilities row since v1.6.0, read-only note, Modelled counts re-tallied"
      contains: "v1.6.0"
  key_links:
    - from: "src/ui/screens/detail.rs persist_active_config"
      to: "src/state_reader/config_write.rs update_gsd_config"
      via: "every Defaults-tab save; the edit is passed as a closure and applied inside the lock"
    - from: "src/state_reader/config_write.rs lock path"
      to: "gsd-core withPlanningLock (planning-workspace.cjs:316)"
      via: "lock = <config file's parent>/.lock, so the TUI and `gsd config-set` contend on the same file"
    - from: "src/state_reader/config_write.rs Global flavor"
      to: "gsd-core acquireInstallMigrationLock (installer-migrations.cjs:499)"
      via: "lock = <defaults.json's parent>/gsd-install-migration.lock"
---

<objective>
Fix two `config.json` defects that block Phase 32 (GSD Capabilities: Visibility & Per-Project Control):

1. **Bug 2 (tracer, Tasks 1-2): unlocked, non-atomic config writes.** `persist_active_config` (src/ui/screens/detail.rs:12539-12562) writes a cached snapshot over the file with a plain write. It ignores gsd-core's `.planning/.lock`, can expose a truncated file to a concurrent reader, resets permission bits, and overwrites any change GSD made since the tab loaded. Replace it with a gsd-core-compatible lock plus a read-modify-write and atomic write path that Phase 32 will reuse for capability toggles.
2. **Bug 1 (Task 3): `capabilities.strict_known_registries` type.** The app models it as `Option<bool>` (src/state_reader/config_json.rs:425-434), but upstream it is `array | null`. Toggling it writes a bool, which gsd-core treats as a malformed policy and fails closed, refusing every external install. A real array makes `parse_gsd_config` (config_json.rs:645-655) return None, so the whole Defaults tab goes dark.

Purpose: the TUI must never corrupt, race or silently clobber a GSD project's config. Phase 32's capability toggles must sit on a write path that is already proven.
Output: new `src/state_reader/config_write.rs`; rewired `persist_active_config`; the strict_known_registries model, row and tests; docs/GSD-CORE-SYNC.md updated.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@src/ui/screens/detail.rs (lines 12495-12562: config_scope_status / active_config_mut / config_write_path / persist_active_config; the five call sites at 4797-4875, 5008-5052, 6265-6324; the `r` reload at 5054-5077; arrival load at 3249-3267; row push at 11923-11933; set arm 13059-13064; clear arm 14172-14177; toggle arm 14573-14578; populated fixture 14827-15012; DEFAULTS_OPTION_COUNT 15035; test helpers ctx_on_config_row 22195, test_ctx 18495, press 18602; the scope-switch test 22224-22310 is the TUI-level test analog)
@src/state_reader/config_json.rs (lines 60-115 ExtraKeys + the ApiKeySetting untagged-enum precedent; 425-434 CapabilitiesConfig; 643-737 parse/serialize/user_defaults_path/load_user_defaults_from; tests 885-936)
@src/driverctl/scan.rs (lines 24-66: `read_regular_bounded`, the O_NONBLOCK|O_NOFOLLOW + fstat + `take` analog for reading plantable files)
@src/driver/direction.rs (lines 468-478: `write_temp`, the create_new + mode + sync_all analog)
@src/config.rs (lines 470-486: `save_config`, the app's existing temp+persist write for its OWN config; do not touch it)
@docs/GSD-CORE-SYNC.md (lines 96-125 since recipe; 415-432 Modelled counts; 612-620 Capabilities table)

## gsd-core lock protocol (measured on 2026-10-19 against ~/projects/node/gsd-core at v1.18.0-6-g8d3d29c46; compiled `gsd-core/bin/lib/*.cjs`, TS source `src/*.cts`)

**`.planning/.lock`: `withPlanningLock(cwd, fn)`** (planning-workspace.cjs:313-472, src/planning-workspace.cts:425-)
- Taken by every gsd-core config write: unsetConfigValue config.cjs:569, setConfigValue :600, setConfigValues :637 (src/config.cts:709/734/794). The same lock also serialises phase.cjs, roadmap.cjs, quick-batch.cjs and state-contract.cjs writes.
- Path: `planningDir(cwd)/.lock`, which is `<project>/.planning/.lock` unless GSD_PROJECT/GSD_WORKSTREAM are set (:316, :103-122). config.json is `planningDir(cwd)/config.json`, so **lock = config.json's parent + `.lock`**.
- Before the loop, `platformEnsureDir(planningDir)` (:333).
- Create: `writeFileSync(lockPath, JSON.stringify({pid, cwd, acquired: new Date().toISOString()}), {flag:'wx'})` (:334-342). That is O_CREAT|O_EXCL with a compact JSON body, keys in the order pid, cwd, acquired, and no trailing newline.
- Budget: `lockTimeout = 10000` ms (:317). Every wait is `sleep(100)` ms (:370, :428, :448, :456).
- Transient create errors are retried after 100 ms: EPERM, EBUSY, EAGAIN, EINTR, EINVAL, EIO, ENOENT, ESTALE (:93-102, :369-372). Any other error except EEXIST is thrown.
- EEXIST: take a decision snapshot, `statSync` plus the body read (null if unreadable) (:384-395).
  - **Verified-live** means the body JSON-parses, `pid` is an integer > 0, and `process.kill(pid,0)` succeeds or fails with EPERM (:49-59, :74-86).
  - A holder that is not verified-live (dead pid, garbage body, missing or invalid pid) is stealable.
  - A verified-live holder is waited on (:455-457). Exception: the **deadman** steals it anyway once the lock's mtime age exceeds 60000 ms (:323, :397-403). That is meant for pid-reuse aliases.
- Steal (:404-450):
  - Re-stat and re-read. Steal only if `(dev, ino)` is unchanged AND the decision body is not null AND the bodies are equal. Otherwise sleep 100 ms and retry. An unreadable body is therefore never stolen.
  - If the lock vanished, retry at once.
  - `rename(lock, lock + '.stale-<pid>-<now>-<seq>')` decides the winner: only one racer's rename succeeds. The winner then runs `rmSync` on the renamed file and retries immediately. A failed rename means sleep 100 ms and retry.
  - A stat failure in the snapshot means retry at once (:452-454).
- Release: `finally` runs an unconditional unlink of lockPath (:343-354). An exit handler also unlinks any lock still held (:29-38).
- Timeout: throws `withPlanningLock: <path> held by a live process for Nms (exceeded 10000ms budget)` with `lockTimeout=true`, and **never force-steals** (:462-471).
- Write inside the lock: read config.json, then `JSON.parse`, set the key, then `platformWriteSync` (config.cjs:637-660). That function writes a temp file `<file>.tmp.<pid>` and renames it into place, and it does NOT preserve mode (shell-command-projection.cjs:1168-1198).
- Stable: the last protocol change was 2026-08-14 (0c43d853e, mkdir surfacing). The body format has not changed since the 2026-06-21 liveness/steal hardening (fc9bd70af, b2602cc61, fc019b268).

**`~/.gsd/defaults.json`.** gsd-core's config loader only READS it (config.cjs:256-257). Its one code writer is the installer: `writeNonClaudeDefaults` (bin/install.js:7088-7130) calls `acquireInstallMigrationLock($GSD_HOME/.gsd)` (installer-migrations.cjs:499-597), giving the lock `$GSD_HOME/.gsd/gsd-install-migration.lock` (:37). Its protocol:
- `openSync(lock,'wx')`, then the body `JSON.stringify({pid, acquiredAt}) + '\n'` is written through the fd. If the write fails, the lock it just created is unlinked.
- EEXIST: `readLockFile` parses the body and returns it only when it is an object with a numeric pid; otherwise null.
  - If `pid === process.pid` or the pid is not alive (`kill(pid,0)` gives ESRCH; any other error counts as alive, :470-479), the lock is reclaimed by `unlinkSync` and the create is retried at once.
  - An unparseable body is **never** reclaimed.
- Poll interval is 50 ms; the timeout is 30000 ms. On timeout it throws `installer migration lock is held: <path> (held by pid N since T)`.
- The write is `atomicWriteFileSync` (runtime-hooks-surface.cjs:141-): an O_EXCL temp file `<target>.tmp-<pid>-<counter>`, chmod to the prior mode, then rename.
- The settings workflow (gsd-core/workflows/settings.md:570-576) has the agent write defaults.json with no lock.

## Decision: (a), replicate the protocol in Rust

Recorded as [INFERRED I-guw-1]. Option (b), routing writes through `gsd-tools config-set`, was rejected for these reasons:
- It writes one key per process spawn, while this tab saves a whole lossless `GsdConfig`: pass-through keys, ExtraKeys and Secret slots.
- Secret values would end up on argv, readable from `/proc/<pid>/cmdline`, which breaks T-jnf-01.
- `config-set` rejects the retired `gates.*` keys that the tab must preserve.
- It has no defaults.json target.
- It adds a node spawn per keypress, and the `Command::new` audit in tests/spawn_seam_guard.rs would be tripped.

The protocol itself is small: O_EXCL create, a JSON body, a pid-liveness gate, and rename-aside stealing. It has been stable since 2026-06/08, and Phase 32 needs this exact path for capability toggles.

## Inferred decisions (human unavailable). The executor copies these into SUMMARY.

- [INFERRED I-guw-1] Option (a), for the reasons above.
- [INFERRED I-guw-2] The TUI lock budget is **2 s**, against upstream's 10 s, with polls at upstream's intervals (100 ms planning, 50 ms installer). The save runs synchronously on the key-handler thread. gsd-core holds the lock only for a sub-second read-modify-write, so 2 s waits out any real writer without freezing the UI for 10 s.
- [INFERRED I-guw-3] **No deadman steal.** The requester's rule "never steal a lock held by a live holder" overrides upstream's 60 s pid-reuse recovery. A verified-live holder always ends in a timeout error that names the pid. gsd-core's own next write still applies its deadman.
- [INFERRED I-guw-4] **Read-modify-write inside the lock.** The edit is passed in as a closure and applied to a FRESH read of the on-disk file, as upstream setConfigValue does. Under a lock, a stale-snapshot write would still lose a concurrent `gsd config-set`.
  - If the on-disk file exists but does not parse, the save is refused with no write.
  - If it is absent, the cached config is the base. This keeps today's create-on-save behaviour, including the Global seed at detail.rs:12486.
  - The cache is replaced only after a successful write, so a failed save leaves the display equal to disk.
  - With no resolvable path (no registered project), the edit is applied in memory only, as it is today.
- [INFERRED I-guw-5] `defaults.json` gets the same atomic write plus the installer's `gsd-install-migration.lock` with installer semantics. That is the only lock gsd-core ever takes around this file.
- [INFERRED I-guw-6] Lock bodies are read with O_NOFOLLOW|O_NONBLOCK, a regular-file fstat check and a 4 KiB cap. The config file read inside the lock follows symlinks, as gsd-core does, but still uses O_NONBLOCK, a regular-file check and a 4 MiB cap. A file that fails these checks reads as unreadable. Both locations are plantable by an agent running in the project (the Phase 27 CR-01 class).
- [INFERRED I-guw-7] A symlinked config.json or defaults.json is replaced by a regular file on save. gsd-core's own `platformWriteSync` and `atomicWriteFileSync` behave the same way, because rename replaces the link.
- [INFERRED I-guw-8] strict_known_registries is modelled as tri-state (absent / null / value). A `Malformed(serde_json::Value)` catch-all means a bool written by pre-fix builds round-trips instead of making config.json unreadable.
  - The row is read-only with no clear, because removing `[]` lockdown with one keypress would downgrade security.
  - Its help text points to `gsd config-set`.
  - `since` is MEASURED as `v1.6.0`: the recipe at GSD-CORE-SYNC.md:96-108 gives first commit 9219af336, and `tag --contains` gives v1.6.0.
- [INFERRED I-guw-9] On release, the lock is unlinked only if its body still equals the body we wrote. This hardening goes beyond upstream's unconditional unlink. A release failure is a `tracing::warn`, never a failed save.

## Repo guards the new code must respect
- tests/spawn_seam_guard.rs rejects `Command::new(` outside its allowlist. Test pids therefore come from no spawned child: use pid `1` for a LIVE holder (it always exists, and EPERM counts as alive) and `i32::MAX` for a DEAD one (above any pid_max, so ESRCH).
- The same file rejects the call spelling `.finish(` anywhere in src/. Write any manual `Debug` with `write!`, as config_json.rs:105-114 does.
- tests/async_blocking_guard.rs scans `async fn` bodies only. The key handlers in detail.rs (`fn handle_key` :3416, `fn handle_text_input_key` :6239) are sync, so blocking file I/O and `std::thread::sleep` in config_write are allowed. Do not add an `async fn` to config_write.
- Use `rustix::process::test_kill_process(Pid)` (rustix 1.1.5 is already a dependency) for liveness, with `Pid::from_raw`: `Ok` or `Errno::PERM` means alive. For the installer flavor, anything except `Errno::SRCH` means alive. Do not add any crate.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: Tracer, a lock-aware atomic save from the Defaults tab, end to end for both scopes</name>
  <files>src/state_reader/config_write.rs, src/state_reader/mod.rs, src/ui/screens/detail.rs</files>
  <action>
Start with the failing end-to-end tests in the detail.rs `tests` module. Model them on `g_switches_the_config_scope_and_writes_land_in_that_scopes_file` (detail.rs:22224): a TempDir project registered in `ctx.config.projects`, a real `.planning/config.json`, and `press` driving `commit_docs` Enter/Down/Enter. Create three tests:

- `config_save_takes_and_releases_the_planning_lock`: after the edit, disk has the new value, `.planning/.lock` does not exist, and `.planning/` contains no entry whose name contains `.tmp-`.
- `config_save_refuses_while_a_live_gsd_holds_the_planning_lock`: pre-write `.planning/.lock` with the body `{"pid":1,"cwd":"<root>","acquired":"2026-10-19T00:00:00.000Z"}`. After the edit (about 2 s), check all of the following. config.json bytes are unchanged. The lock bytes are unchanged. `cache.defaults_config.commit_docs` still equals the disk value. `ctx.status_message` contains `Config NOT saved` and `pid 1`.
- `config_save_global_defaults_refuses_while_the_installer_lock_is_live`: Global scope (press `g` as in the analog), with `defaults_user_path` inside a TempDir. Pre-write `gsd-install-migration.lock` beside it with body `{"pid":1,"acquiredAt":"..."}` plus a newline. Check that the defaults file is not created or changed and that the status contains `Defaults NOT saved`.

Then create `src/state_reader/config_write.rs` and declare `pub mod config_write;` in src/state_reader/mod.rs. Give it a module doc that states the protocol and cites the gsd-core file:line list from this plan's context. Contents:

- `LockFlavor { Planning, InstallMigration }`.
- `LockPolicy { timeout, poll }`, with a `LockPolicy::tui(flavor)` constructor: 2 s; poll 100 ms for Planning, 50 ms for InstallMigration (per I-guw-2).
- `ConfigWriteTarget`:
  - `Project { config_path, project_root }`: lock = config_path.parent()/`.lock`, flavor Planning, body cwd = project_root.
  - `GlobalDefaults { path }`: lock = path.parent()/`gsd-install-migration.lock`, flavor InstallMigration.
- `ConfigWriteError`, with variants:
  - `LockTimeout { holder_pid: Option<i64>, waited }`.
  - `Unparseable`.
  - `Io { kind: std::io::ErrorKind }`.
  - `Serialize`.

  Give it a `status_text(scope)` that builds ONLY from static text, the integer pid and the ErrorKind's Display. Never include a path, a config value or serde's Display (T-jnf-03, I-12). Texts:
  - Project timeout: `Config NOT saved: .planning/.lock is held by pid N (GSD is writing) — retry`.
  - Global timeout: `Defaults NOT saved: the GSD installer lock is held by pid N — retry after it finishes`.
  - Unparseable: `... NOT saved: the file on disk no longer parses — press r to reload`.
  - Io: `... NOT saved: <kind>`.
  - When the pid is unknown, say "held by another process".

Implement the lock as a RAII guard with Drop-release. Follow this plan's protocol summary exactly; the rules below are what differ from it or need spelling out.

- **Create.** Create the parent dir with create_dir_all, then `OpenOptions` create_new with mode 0o666, so the umask applies as it does to Node's default. Write the body through that same handle:
  - Planning: a `#[derive(Serialize)]` struct with fields in the order pid, cwd, acquired, compact, no newline. `acquired` comes from chrono `Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)`.
  - InstallMigration: pid, acquiredAt, then a trailing newline.
  - If the body write fails, unlink the file just created and return Io.
- **Retry on create error (Planning).** Retry these errnos after `poll`: EPERM, EBUSY, EAGAIN, EINTR, EINVAL, EIO, ENOENT, ESTALE. Return any other non-EEXIST error at once.
- **Retry on create error (InstallMigration).** Retry only on EEXIST.
- **EEXIST, Planning.** Take an `lstat` (dev, ino) plus a bounded lock-body read as the decision snapshot. A failed lstat means retry at once.
  - Verified-live means: the body parses as a JSON object, its `pid` is an integer > 0 (an integral float counts, as with JS Number.isInteger) that fits i32, and the pid is alive.
  - Not verified-live: re-lstat and re-read. If (dev, ino) differs, or either read was unreadable, or the bodies differ, sleep `poll` and retry. Otherwise rename the lock to `<lock>.stale-<pid>-<unix ms>-<seq>`, remove the renamed file and retry immediately. A failed rename means sleep `poll` and retry.
  - Verified-live: sleep `poll` and retry. There is no age-based steal, per I-guw-3.
- **EEXIST, InstallMigration.** Do a bounded read and parse. Reclaim by `remove_file` only if the body parses with a numeric pid AND one of these holds: the pid equals `std::process::id()`, the pid is ≤ 0 (upstream isPidAlive :470-473 treats that as dead), or `test_kill_process` returns SRCH. Then retry immediately. Otherwise wait `poll`. An unparseable body is never reclaimed.
- **Timeout.** When the deadline passes, return `LockTimeout`, with `holder_pid` taken from the last parsed body.
- **Bounded reader.** One private reader, modelled on driverctl/scan.rs:32-66: a `follow_symlinks` flag, O_NONBLOCK always, O_NOFOLLOW when not following, an fstat regular-file check, and `take(cap + 1)`. The lock body uses cap 4096 and no-follow. The config uses 4 MiB and follow (I-guw-6).
- **`atomic_write(path, bytes)`.** In the same directory:
  - Create the temp file `<target>.tmp-<pid>-<counter>` (an AtomicU64 counter) with create_new and mode 0o666 (the umask applies). Retry the next counter on EEXIST, up to 16 attempts, and never delete a file you did not create.
  - If the target is an existing regular file, `set_permissions` the temp file to the target's `mode & 0o7777`.
  - write_all, then `sync_all`, then `std::fs::rename` over the target.
  - Fsync the parent dir on a best-effort basis.
  - On any failure, remove the temp file and return the error.
  - Use the write_temp analog at driver/direction.rs:468-478.
- **`update_gsd_config(target, policy, fallback: &GsdConfig, mutate: impl FnOnce(&mut GsdConfig) -> bool) -> Result<Option<GsdConfig>, ConfigWriteError>`.** Inside the lock (per I-guw-4):
  - Bounded-read the target. NotFound means clone `fallback`. Other errors become Io. A parse failure from `parse_gsd_config` is Unparseable.
  - If `mutate` returns false, return `Ok(None)` and write nothing.
  - Otherwise `serialize_gsd_config`, then `atomic_write`, then return `Ok(Some(fresh))`.

Rewire detail.rs:
- `persist_active_config` keeps its name and gains `mutate: impl FnOnce(&mut GsdConfig) -> bool`. It takes the active config as `&mut GsdConfig` and returns a `PersistOutcome { Applied, NotApplied, Failed }`.
  - With no write path from `config_write_path`, apply `mutate` to the in-memory config and return Applied or NotApplied (today's behaviour).
  - Otherwise build the `ConfigWriteTarget`. Project needs `project_path`; Global uses the resolved `user_defaults_path`.
  - Call `update_gsd_config` with `LockPolicy::tui`.
  - Ok(Some(fresh)): replace the cached config with `fresh`, set the existing `Config saved` / `Defaults saved` status, and return Applied.
  - Ok(None): NotApplied.
  - Err: set `ctx.status_message` to `status_text`, leave the cache untouched, and return Failed.
- Convert all five call sites so they no longer mutate before persisting. Each passes its mutation as the closure instead: `set_config_value`, `mutate_config_entry`, `clear_config_value`, `set_secret_value`, and the String branch's clear-or-`set_string_value`.
- In the `x` arm (5027-5046):
  - Applied shows `Cleared {key}`.
  - NotApplied shows `{key} cannot be cleared`.
  - Failed keeps the error status. It must NOT be overwritten.
- Leave the arrival load and the `r` reload as unlocked reads. gsd-core readers do not lock, and atomic writes make those reads safe.

If an existing test fails only because it relied on the old in-memory-first ordering, update it and list it in SUMMARY. Do not weaken any assertion about what reaches disk.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib config_save_</automated>
    <automated>rtk proxy grep -c "std::fs::write(&path, &json)" src/ui/screens/detail.rs</automated>
  </verify>
  <done>
- The three `config_save_` tests pass.
- The grep prints 0. Its baseline at HEAD was 1, which is the plain write inside persist_active_config.
- Every one of the five call sites routes its edit through `update_gsd_config`.
- The commit message is `fix(quick-261019-guw): lock-aware atomic config.json saves (tracer)`.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Prove each upstream lock rule and the atomicity guarantees in config_write's own tests</name>
  <files>src/state_reader/config_write.rs</files>
  <behavior>
    - atomic_write_never_exposes_a_partial_file. Pre-write payload A (about 512 KiB). A writer thread alternates atomic_write of A and of B (a different length) 200 times. A reader thread runs `std::fs::read` 2000 times, and every read must equal A or B exactly.
    - atomic_write_preserves_existing_permission_bits. Mode 0o600 before the write is still 0o600 after; 0o640 likewise.
    - atomic_write_leaves_no_temp_file_behind. After 50 writes, the directory holds only the target.
    - planning_lock_live_holder_times_out_without_stealing. With a pid 1 body and a 300 ms policy, the result is LockTimeout with holder_pid Some(1), the elapsed time is at least 300 ms, the lock bytes are unchanged, and the target is untouched.
    - planning_lock_released_mid_wait_is_acquired. With a pid 1 lock, a thread removes it after 200 ms. update_gsd_config with a 2 s policy returns Ok, the elapsed time is at least 200 ms, the target holds the edit, and the lock is gone.
    - planning_lock_dead_holder_is_stolen. A lock with pid i32::MAX gives Ok well inside the budget, and afterwards no `.lock` and no `.lock.stale-*` remain.
    - planning_lock_garbage_empty_or_pidless_body_is_stolen. Cover these bodies: `not json`, empty, `{"cwd":"x"}`, `{"pid":0}`, `{"pid":-5}`, `{"pid":"12"}`. Each one is stolen.
    - planning_lock_unreadable_body_is_never_stolen. Cover a symlink at `.lock` (to an existing file) and a lock larger than 4 KiB. Each one gives LockTimeout, and the symlink and the large lock are still present afterwards.
    - planning_lock_body_matches_gsd_core_shape. While the lock is held (read it from inside the closure), the body parses with exactly the keys pid (our pid), cwd (the project root) and acquired (ends in `Z`, millisecond precision). It is compact and has no trailing newline.
    - install_lock_rules. A dead pid is reclaimed. Our own pid is reclaimed. An unparseable body gives LockTimeout and the file is unchanged. pid 1 gives LockTimeout. The body we write is `{"pid":N,"acquiredAt":"..."}` plus a newline.
    - update_applies_the_edit_to_the_on_disk_config_not_the_cached_snapshot. The fallback has mode `yolo`, disk has mode `interactive` plus an unknown key. A commit_docs edit keeps both the disk mode and the unknown key.
    - update_refuses_an_unparseable_config_on_disk. The result is Err(Unparseable) and the bytes are unchanged.
    - update_creates_an_absent_file_from_the_fallback.
    - release_does_not_delete_a_lock_it_no_longer_owns. Inside the closure, replace the lock with a different body. After return, that foreign lock still exists.
    - update_without_a_mutation_writes_nothing. The closure returns false, the result is Ok(None), and the mtime and bytes are unchanged.
  </behavior>
  <action>
Write the behaviours above as unit tests in a `#[cfg(test)] mod tests` at the bottom of src/state_reader/config_write.rs, using `tempfile::TempDir`. Use pid `1` as the live holder and `i32::MAX` as the dead one, and spawn no child process (see "Repo guards" in context). Pass short `LockPolicy` values directly so the suite stays fast. Use `std::thread` with `Arc`/channels for the concurrent tests; tokio is not needed. If a test exposes a bug in the Task 1 implementation, fix the implementation, not the test, and note the fix in SUMMARY. The atomicity test is the regression proof for the plain-write defect: before relying on it, temporarily point it at a plain `std::fs::write` to confirm it fails, revert that, and record the observation in SUMMARY. The tests live in module `state_reader::config_write::tests`, so the `config_write` filter in `<verify>` selects all of them.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib config_write</automated>
  </verify>
  <done>
- All the listed tests exist and pass.
- SUMMARY records that the atomicity test failed against a plain write.
- The commit message is `test(quick-261019-guw): pin gsd-core lock semantics and atomic write`.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Model capabilities.strict_known_registries as upstream array|null, as a read-only row</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - strict_known_registries_array_parses. A string array `["github.com","npm"]` parses as Hosts with those two entries, and `[]` parses as Hosts with none.
    - strict_known_registries_null_parses_and_saves_as_null. An explicit null stays distinct from absent and serialises back as `"strict_known_registries": null`.
    - strict_known_registries_absent_stays_absent_on_save. A capabilities block holding only auto_update serialises with no strict_known_registries key.
    - strict_known_registries_save_preserves_the_array. parse, then serialize_gsd_config, then parse gives the identical Vec, in order, for both the populated and the empty array.
    - strict_known_registries_legacy_bool_does_not_fail_the_whole_config. `true` gives Some(config) with a Malformed value, and the bool round-trips verbatim. So does `[1,"a"]`.
    - strict_known_registries_row_is_read_only_and_no_mutation_path_writes_it:
      - The row's kind is ReadOnly when set and Null when absent.
      - Its value shows the JSON array, and a Malformed value is flagged as malformed.
      - `set_config_value(cfg, key, "true")`, `clear_config_value(cfg, key)` and `mutate_config_entry(cfg, key, &kind)` each return false and leave the field unchanged.
      - The help summary is at most 160 chars, and since is `v1.6.0`.
  </behavior>
  <action>
In src/state_reader/config_json.rs (425-434), add a `pub enum RegistryAllowlist` deriving Debug, Clone, PartialEq, Serialize and Deserialize, with `#[serde(untagged)]` and two variants in this order:
- `Hosts(Vec<String>)`: upstream's allowlist, where `[]` means lockdown.
- `Malformed(serde_json::Value)`: any other non-null shape, preserved verbatim. gsd-core fails closed on these.

Document the variants with upstream's semantics (CONFIGURATION.md:1035) and cite this quick id, following the ApiKeySetting doc style at :75-103.

Change the field to the tri-state `Option<Option<RegistryAllowlist>>`. Use `#[serde(default, deserialize_with = ..., skip_serializing_if = "Option::is_none")]` with a small generic helper that wraps any present value, including null, in `Some`. The result: absent becomes None, null becomes Some(None), and a value becomes Some(Some(..)). Do not touch `auto_update`. Update the existing fixture and assertion at :897-900 and :929-931 to an array, and add the five config_json behaviours above to the tests module.

In detail.rs:
- Replace the row at 11923-11929 with a helper that returns:
  - `("(unset)", ConfigValueKind::Null)` when the key is absent;
  - otherwise `ConfigValueKind::ReadOnly`, with the value from `config_secrets::display_config_value` on the JSON form (null, the array, or the malformed value). For Malformed, append the static suffix ` (malformed: GSD blocks every external install)`.
- Give the row a ConfigHelp summary of at most 160 chars, for example: `Host allowlist for third-party capability installs: null permits any source, [] blocks every external install. Read-only here; set it with gsd config-set.`
- Add `.since("v1.6.0")` (measured per I-guw-8).
- Delete the `capabilities.strict_known_registries` arms in `set_config_value` (13059-13064), `clear_config_value` (14172-14177) and `mutate_config_entry` (14573-14578). The `x` key then reports "cannot be cleared" (I-guw-8).
- In `populated_gsd_config` (around 15006), change the value to `["github.com", "npm"]`.
- DEFAULTS_OPTION_COUNT stays 135: no row is added or removed.
- Add the detail.rs behaviour test. If render_escape_guard.rs has a completeness list of string-bearing config keys that now fails, add this key to it.

In docs/GSD-CORE-SYNC.md:
- Set the Capabilities table row (line 618) since to `v1.6.0`.
- Below that table, add one sentence: the row is read-only because upstream is `array | null`, and since 261019-guw a bool is preserved and flagged as malformed.
- In the Modelled paragraph (421-427), re-tally: 73 rows without a since become 72, and add "**1** carries `v1.6.0`, measured at 261019-guw". The total stays 135.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib strict_known_registries</automated>
    <automated>rtk proxy grep -c "strict_known_registries: Option<bool>" src/state_reader/config_json.rs</automated>
    <automated>rtk proxy grep -c "strict_known_registries = Some(" src/ui/screens/detail.rs</automated>
  </verify>
  <done>
- All six strict_known_registries tests pass.
- Both greps print 0; their baselines at HEAD were 1 and 2.
- DEFAULTS_OPTION_COUNT is still 135 and its test passes.
- The docs row reads `v1.6.0` and the counts sum to 135.
- The commit message is `fix(quick-261019-guw): model capabilities.strict_known_registries as array|null`.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project `.planning/` → TUI | `.lock`, `config.json` and the temp names are writable by anything running in the project, including an autonomous driver or agent. Their content and file type are untrusted. |
| gsd-core processes ↔ TUI | Two independent writers of the same config.json, coordinated only by the `.planning/.lock` protocol. |
| `$GSD_HOME/.gsd/` → TUI | defaults.json and the installer lock are shared by every runtime and project on the machine. |
| config values → status line | Status text is rendered in a TUI cell and must carry no secret and no untrusted text. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-guw-01 | Tampering | config.json during save (atomic_write) | high | mitigate | The data is written to a same-directory O_EXCL temp file, synced with `sync_all`, then renamed into place. A reader only ever sees the old or the new file. Pinned by `atomic_write_never_exposes_a_partial_file`. |
| T-guw-02 | Tampering | concurrent `gsd config-set` vs TUI save (update_gsd_config) | medium | mitigate | gsd-core's `.planning/.lock` is taken, and the edit is applied to a fresh read inside the lock (I-guw-4), so neither writer loses the other's key. Pinned by `update_applies_the_edit_to_the_on_disk_config_not_the_cached_snapshot`. |
| T-guw-03 | Elevation of Privilege | lock steal path | high | mitigate | Only a holder that is not verified-live is stolen. The steal re-confirms (dev, ino, body) and moves the lock aside with rename, which only one racer can win. There is no deadman steal (I-guw-3), and release unlinks only our own body (I-guw-9). Pinned by the live-holder, release-ownership and dead-holder tests. |
| T-guw-04 | Denial of Service | planted `.lock` as FIFO, symlink or huge file | medium | mitigate | Lock bodies are read with O_NONBLOCK, O_NOFOLLOW, an fstat regular-file check and a 4 KiB cap. Anything else reads as unreadable, is never stolen, and ends in a timeout after 2 s with the UI still responsive. Pinned by `planning_lock_unreadable_body_is_never_stolen`. |
| T-guw-05 | Denial of Service | planted lock naming a live pid (for example 1) | low | accept | Each save is bounded to 2 s and ends in an error that names the pid, so the operator can remove the file. gsd-core is equally blocked by such a file, so the TUI adds no new exposure. |
| T-guw-06 | Tampering | symlink planted at a temp or lock path | medium | mitigate | Both are created with create_new (O_CREAT\|O_EXCL), which never follows a link. On EEXIST the temp name moves to the next counter and the foreign file is never deleted. Rename replaces a link rather than writing through it (I-guw-7). |
| T-guw-07 | Information Disclosure | permission bits on a secret-bearing config.json | high | mitigate | The temp file is chmodded to the target's existing `mode & 0o7777` before the rename. Pinned by `atomic_write_preserves_existing_permission_bits` (0600). |
| T-guw-08 | Information Disclosure | status line after a failed save | medium | mitigate | `status_text` builds only from static text, the integer pid and `io::ErrorKind`. It never includes a path, a config value or serde's Display (T-jnf-03, I-12). |
| T-guw-09 | Tampering | capability source policy (strict_known_registries) | high | mitigate | The row is read-only and its set, toggle and clear arms are removed, so the TUI can no longer turn an allowlist into a malformed bool or delete a `[]` lockdown. Legacy values are preserved verbatim. Pinned by `strict_known_registries_row_is_read_only_and_no_mutation_path_writes_it`. |
| T-guw-SC | Tampering | npm/pip/cargo installs | high | mitigate | This plan has no package-install task and adds no crate. rustix 1.1.5, chrono, serde_json and tempfile are existing dependencies. Cargo.toml and Cargo.lock must be unchanged in all three commits. |
</threat_model>

<verification>
Run from the repo root after Task 3's commit. Do NOT pipe either command through grep or awk; read the raw output.

- `rtk proxy cargo test --no-fail-fast`
  - The only acceptable failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. It is expected on this machine, so do not fix it.
  - Confirm that the envelope integration suites ran, since fail-fast would hide them. The `--no-fail-fast` flag covers this.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- `rtk proxy git diff --stat HEAD~3 -- Cargo.toml Cargo.lock` prints nothing.
</verification>

<success_criteria>
- Every Defaults-tab save to `.planning/config.json` takes and releases gsd-core's `.planning/.lock`, re-reads the file inside the lock, writes atomically and keeps the file's mode.
- If a live holder keeps the lock for 2 s, the save is refused without stealing it, and a non-fatal `NOT saved` status names the holder pid.
- `~/.gsd/defaults.json` saves honour gsd-core's installer lock with its exact reclaim rules, and use the same atomic write.
- Dead, garbage or pid-less locks are recovered as gsd-core recovers them, and an unreadable lock is never stolen.
- `capabilities.strict_known_registries`:
  - an array, `[]`, null, an absent key and a legacy bool all parse;
  - each round-trips losslessly;
  - the row is read-only with since `v1.6.0`;
  - DEFAULTS_OPTION_COUNT is still 135.
- The full suite fails only the known git-version witness, and clippy is clean. There are three atomic commits.
</success_criteria>

<output>
Create `.planning/quick/261019-guw-fix-two-config-json-defects-strict-known/261019-guw-SUMMARY.md` when done. It must include:
- the I-guw-1..9 inferred decisions verbatim, marked for audit;
- the gsd-core protocol citations;
- any test updated because of the closure-first reordering;
- the atomicity-test negative-control observation;
- the final test and clippy result lines.
</output>
