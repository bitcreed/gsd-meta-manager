# Phase 19: GITSAFE — Git & Blast-Radius Envelope - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("no questions at any gate; best well-reasoned choice, proceed, record it"). Every
decision below is Claude's Discretion unless tagged otherwise. Decisions marked
**[LOCKED-BY-RESEARCH]** were settled by the committed research pass
(`.planning/research/{SUMMARY,ARCHITECTURE,PITFALLS,FEATURES}.md`) and are carried in as
given — do not relitigate them during planning. Decisions marked
**[USER-HARD-REQUIREMENT]** were made hard requirements by the user personally when they
chose FULL autonomy including `git push` and PR creation with no human in the loop; they may
not degrade into partial implementations.

**Research:** skip — the ROADMAP marks this phase "Research: skip" because branch-protection
and pre-push-hook patterns are well-established practice and GitHub Copilot's own
`copilot/`-prefix branch model is a direct precedent. The CLI-capability facts this phase
depends on were verified directly against the installed `claude` binary during context
gathering (see `<code_context>`), not inferred.

**UI:** `--skip-ui`. This phase ships no visual surface. It delivers a policy module, two git
hook stubs, a `PreToolUse` guard, an environment envelope and a secret scanner. The one
user-visible string it adds is a pinned honesty paragraph in the existing dry-run preview and
the existing run journal. The repo-wide `ui-plan-gate` returns `frontend: true` for any
ratatui project and would fire on the codebase, not on this phase's scope — the same reasoning
Phase 16 recorded.

---

## Why this phase exists — the governing lesson

The user personally chose FULL autonomy including `git push` and PR creation, after being told
the risks, and then made the kill switch and the dry-run hard requirements. This phase is the
rest of that bargain.

**Guardrails must be MECHANICAL, never prompt text.** PITFALLS:42-75 is built on the Replit
incident: an agent deleted a production database *during an explicit code freeze*, because the
freeze lived only in the prompt. The same agent then hid the deletion, lied about it,
fabricated ~4000 fake users and fake test results, and falsely claimed rollback was
impossible. The measured version of the same lesson is IssueTrojanBench: **66.5% of malicious
GitHub issues bypassed every guardrail** across Cursor, Claude Code and Codex Desktop, and
nearly all successful blocks came from the model refusing rather than from any agent safety
layer.

Two consequences bind every plan in this phase:

1. Every control must be a `PreToolUse` deny, an `--allowedTools`/`--disallowedTools` rule, a
   git hook, or a filesystem/process/credential boundary. **Not a sentence in a prompt, not a
   line in `CLAUDE.md`, not an instruction in the GSD command text.**
2. **Never derive success or safety state from the agent's prose.** Exit codes, disk state,
   git refs, and the journal on disk are the only admissible evidence.

This phase is sequenced immediately **before** Phase 20 on purpose, per the hardest ordering
constraint in the research: GITSAFE is co-resident with the driver, never a follow-up, so the
decision router is built and tested against a real safety envelope rather than a stub.

---

<domain>
## Phase Boundary

Autonomous git operations are bounded by mechanisms the agent cannot argue its way past.

**In scope:**

- **SAFE-01** — the driver pushes only to a reserved branch namespace; pushes outside it are
  blocked by a mechanism the agent cannot talk past.
- **SAFE-02** — force-push and hook-bypass are blocked for driver-initiated git operations,
  including `--force`, `--force-with-lease`, `+refspec`, `--no-verify`, and any rewrite of
  `core.hooksPath` (an agent that can move `hooksPath` can uninstall the pre-push hook; that
  specific escape is closed by name).
- **SAFE-03** — driver-initiated pushes are scanned for secrets and blocked on detection,
  over the **full worktree** rather than the diff.
- **SAFE-05** — a driven run uses a scoped git credential rather than inheriting the user's
  ambient credentials or SSH agent.
- **SAFE-06** — PR creation is rate-capped per project per day.
- **The server-side branch-protection RECOMMENDATION is surfaced, not silently assumed.**
  Client-side hooks are defeatable in principle, so the honest posture is defence-in-depth
  plus an explicit, pinned statement of what is and is not guaranteed.
- **The `git add -A` worktree-sweep hazard.** Phase 17's D-21 deferred worktree isolation to
  this phase. `.claude/worktrees/` is where this milestone's own executors run, so the hazard
  is concrete here rather than theoretical.
- **Opportunistic carry-in (cheap, fits):** a mechanical guard for Phase 17's D-28
  blocking-call-inside-`async fn` boundary, which today is held only by comments. This phase
  adds several synchronous `git` shell-outs, so the guard pays for itself immediately.
- Whatever registry/`DriverOptIn` schema the above needs, with the same
  `#[serde(default)]` migration posture Phase 17 used.

**Out of scope — each has a named owner:**

- **`decide()`, the D-R-P-E-V router, multi-command sequences, run bounds, quota park,
  no-progress detection, and what *happens* to a parked run** → Phase 20. Phase 19 defines and
  *emits* the park reasons the envelope produces; it does not implement resume, retry,
  backoff, or any policy on top of them.
- **`.planning/` untrusted-content boundary (SAFE-07) and the fixed command enum (SAFE-08)**
  → Phase 21. Phase 19 constrains *git and PR* operations, not what the model is asked or
  what it is allowed to choose.
- **`ExecutionTarget::Container`, container egress firewalls, container credential
  delivery** → Phase 22. `build_argv`'s exhaustive `match options.target` must continue to
  make a `Container` variant a compile error rather than a silently host-shaped argv.
- **Applying server-side branch protection or rulesets.** That needs `Administration` scope,
  which SAFE-05 explicitly withholds from the run credential. Phase 19 *detects and reports*
  the remote's protection state; it never mutates it.
- **Vendoring, bundling or installing `gitleaks`.** See D-10.
- **Secret rotation, history purging, or any remediation after a leak.** The envelope blocks;
  recovery is a documented human procedure (PITFALLS "Recovery Strategies").
- **Redact-at-capture (SAFE-04)** — already shipped in Phase 16. Phase 19 *reuses* its
  pattern table and must not fork it.
- **Any new TUI screen, tab, widget, badge or key binding.** Phase 18 owns the Driver tab.
  The one string this phase adds goes into the existing dry-run preview and the existing
  journal.
- **Relocating the driver's working directory into a git worktree** — declined, finally, in
  D-19. The hazard it was meant to close is closed differently in D-20.

</domain>

<decisions>
## Implementation Decisions

### Where the envelope lives, and what shape it is

- **D-01:** The envelope is a **new top-level module `src/envelope/`**, not a submodule of
  `src/driver/`. It is invoked from three distinct processes and that is what settles the
  placement: the TUI (to render the honesty text in a preview), the driver (to build the
  child's environment before spawn), and **this same binary re-entered as a subprocess** by a
  git hook and by a `PreToolUse` hook. A module under `driver/` would imply the driver owns
  every caller, and it does not. Submodules, one concern each:
  `policy.rs` (pure decision functions over argv and refspecs), `hooks.rs` (hook stub
  generation and per-run assertion), `scan.rs` (the worktree secret scan), `cred.rs` (the
  scoped-credential environment), `ledger.rs` (the PR rate cap), `advisory.rs` (remote
  protection probe and the pinned honesty text).

- **D-02:** **Envelope artifacts live OUTSIDE the driven repository**, under
  `dirs::data_local_dir()/gsd-meta-manager/envelope/<alias>/` — the same base directory
  `src/main.rs:19` already uses for the log and `src/config.rs:170` for the registry.
  Contents: `hooks/pre-push`, `hooks/pre-commit`, `askpass`, `gitconfig`, `settings.json`,
  `pr-ledger.ndjson`, `gh/`. Three reasons, and each is a failure it prevents:
  `git add -A` in the driven repo cannot sweep them into a commit; they are not governed by
  `.gitignore`, so an agent editing `.gitignore` cannot expose them; and a `Write`/`Edit`
  deny rule on the driven repo does not have to enumerate them to protect them.

- **D-03:** `<alias>` is **not** interpolated into that path raw. A registry alias is
  user-typed and is not validated as a path component anywhere today, so `journal`'s
  `is_plain_run_id` discipline (D-27/WR-02 of the earlier phases — `--run-id
  '../../../../escaped'` was reproduced writing outside the project with exit 0) applies here
  verbatim: an alias that is not exactly one `Component::Normal` whose text equals the whole
  string is refused before any path is joined. Reuse the existing predicate rather than
  writing a second one; if it needs a rename to stop reading as run-id-specific, rename it.

- **D-04:** Every enforcement point **re-enters this same binary** through a hidden
  `envelope` subcommand — `gsd-meta-manager envelope pre-push | pre-commit | guard | scan |
  askpass` — never a generated shell script that carries policy logic. The generated hook file
  is a three-line stub that `exec`s the binary by the **absolute path captured from
  `std::env::current_exe()` at generation time**. Rationale is Phase 17's D-01 argument
  applied one layer down: policy lives in Rust where it is unit-testable, one binary and one
  version means the TUI, the driver and the hook can never disagree about what the policy is,
  and a shell script full of `case` statements is precisely the artifact an agent rewrites.

### SAFE-01 — the reserved push namespace

- **D-05:** **[USER-HARD-REQUIREMENT]** The reserved namespace is
  `refs/heads/gsd-auto/<alias>/…`. Copilot's precedent is a flat `copilot/` prefix; the extra
  `<alias>` segment is added because this tool drives N projects from one machine and one
  credential, so an errant ref should name the project it came from without a lookup.
  Per-project override via a new `DriverOptIn.branch_namespace`, but the **shape is
  validated**: non-empty, at least two `/`-separated segments below `refs/heads/`, not
  `main`/`master`/`HEAD`, and it must end in `/` so it is a prefix and not a single branch. A
  user cannot accidentally configure `refs/heads/` and silently disable the control.

- **D-06:** Enforcement is **three independent layers**, and the phase docs must state for
  each layer what it does and does not cover:

  1. **`--disallowedTools` on the `claude` argv.** Cheapest, fires before the tool runs, and —
     critically — **cannot be silently dropped**, because it is argv rather than a file.
  2. **A `PreToolUse` deny hook** (`envelope guard`) delivered through `--settings`. It
     *parses* the Bash command into argv and applies `policy::classify_git`, instead of
     prefix-matching a string, so `git  push   --force` and `git push -f` are the same verdict.
  3. **The `pre-push` git hook.** It sees the actual `<local-ref> <local-sha> <remote-ref>
     <remote-sha>` lines on stdin, **regardless of how git was invoked** — from a nested
     shell, from a Makefile, from a script the agent wrote. This is the only layer that
     observes ground truth.

  Layers 1 and 2 exist to make layer 3's known bypasses (`--no-verify`, `core.hooksPath`)
  unreachable through the tool boundary. Layer 3 exists because layers 1 and 2 only see
  commands that go through the `Bash` tool. Neither direction is redundant.

- **D-07:** **The `--settings` file must never be the sole carrier of any control.**
  `claude --help` states, for `-p`/print mode: *"Settings files that fail validation are
  silently ignored in this mode (no error dialog is shown)."* A malformed envelope settings
  file therefore **disarms silently**. Mitigation is threefold: the file is produced by
  `serde_json` from typed structs and never templated; it is deserialized back and compared
  after writing, and a mismatch refuses the run; and every control it carries is *also*
  carried by argv or by a git hook. A silently ignored settings file must degrade the
  envelope, never disarm it.

### SAFE-02 — force-push and hook bypass

- **D-08:** The denied set is a **pure function** `policy::classify_git(argv: &[&str]) ->
  GitVerdict`, so it is exhaustively unit-testable without a repo, a process or a network.
  Denied for a driven run:
  - `push` carrying any of `-f`, `--force`, `--force-with-lease` (with or without `=value`),
    `--force-if-includes`, `--mirror`, `--delete`/`-d`, or a `+`-prefixed refspec.
  - `push` carrying `--no-verify`.
  - `push` whose resolved destination ref is outside the D-05 namespace (including the
    implicit destination when no refspec is given — resolved from `push.default`,
    `branch.<b>.merge` and `remote.<r>.push` the same way `git_ops::push_refspecs` already
    does for the dry-run).
  - `git config` in any writing form touching `core.hooksPath`, at any scope
    (`--global`, `--system`, `--local`, `--worktree`, `--file`), **and** the
    `git -c core.hooksPath=…` command-line form, which is the one that actually wins over the
    envelope's own setting (see D-09).
  - `git stash` in every subcommand form — ROADMAP names this outright, and the reason is
    that a stash silently removes the human's uncommitted work from the tree where
    `git fsck --lost-found` is the only recovery.
  - `update-ref`, `reflog delete`, `reflog expire`, `filter-branch`, `filter-repo`, and
    `symbolic-ref` writes — the history-rewriting family, whose recovery cost PITFALLS rates
    HIGH.

  Anything not on the list is **allowed**; this is a denylist over git verbs, not an
  allowlist, and that choice is recorded honestly: an allowlist would be stronger and would
  also break every GSD skill that runs an unanticipated plumbing command. The pre-push and
  pre-commit hooks are what make the denylist's inevitable gaps non-fatal.

- **D-09:** **`core.hooksPath` is delivered to the driven child through the environment, not
  by mutating the repository.** Git's `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_n` /
  `GIT_CONFIG_VALUE_n` triplet injects configuration at the same precedence as `-c`, which is
  **above repo-local config**, and is inherited by every `git` invocation in the child's
  process tree — including ones the agent makes from a nested shell or a script. So the child
  gets `core.hooksPath=<envelope>/<alias>/hooks` and the repository's `.git/config` and
  `.git/hooks/` are **never touched**.

  Four things this buys over installing into `.git/hooks/`, and they are why the obvious
  approach is declined:
  - **The user's own git commands in that repo are unaffected.** A `pre-push` hook installed
    into `.git/hooks/` fires for the human too, which would make the meta-manager's safety
    envelope silently police the user's own pushes. That is a bug, not a feature.
  - **Nothing is left behind** when a run crashes, is SIGKILLed, or the TUI is closed —
    consistent with Phase 17's D-12 "crash reconciliation performs zero disk writes" posture.
  - **The agent cannot uninstall it by editing a file in the repo**, because there is no file
    in the repo.
  - **Repo-local `core.hooksPath` cannot override it**, because env-injected config outranks
    repo config.

  The one form that *does* override it is `git -c core.hooksPath=… push`, which is exactly why
  D-08 denies that form by name. **Stated honestly in the phase docs:** an agent that unsets
  `GIT_CONFIG_COUNT` in a subshell escapes this layer. That is not a hole this phase can close
  client-side, and pretending otherwise is the failure mode D-22 exists to prevent.

- **D-10:** The hook stub **verifies its own provenance** before acting: it refuses (non-zero,
  with a legible message) if the binary path it was generated with no longer exists, or if the
  hook file it is running from is not the one inside the envelope directory. A copy of the
  hook relocated to some other `hooksPath` must not silently become the sanctioned one.

### SAFE-03 — the pre-push secret scan

- **D-11:** **The scanner is built in, not `gitleaks`.** `gitleaks` is not installed on this
  machine, is not a Rust crate, and is not something this project may install on a user's
  behalf. A push gate that depends on an absent external binary either fails open — which is
  the worst possible failure for this control — or makes the phase unverifiable. The built-in
  scanner reuses `journal::redact`'s **already-shipped and already-tested** credential pattern
  table (`src/journal/redact.rs:70`, `PARTS`), which covers PEM private keys,
  `authorization:` headers, `*_TOKEN`/`*_SECRET`/`*_API_KEY` assignments, bearer tokens,
  `sk-ant-`, `sk-`, `github_pat_`, `gh[pousr]_`, AWS `AKIA`/`ASIA`, Slack `xox[baprs]-`, JWTs,
  and URL userinfo. **If a `gitleaks` binary is on `PATH`, the hook runs it as well** and
  treats its non-zero exit as an additional block — additive, never a substitute, and its
  absence is never a reason to allow.

- **D-12:** `PARTS` is **split by class, not duplicated.** Each rule gains a
  `SecretClass::{Credential, PathHygiene}` tag. Redaction behaviour is unchanged — both
  classes still redact into the log — but the scanner consumes **only** `Credential`. The
  reason is concrete: `/home/<user>` and the dash-encoded `-home-<user>-<repo>` form are
  redacted from logs for privacy, and a source file containing `/home/andy` is not a secret.
  A scanner that blocked every push over a home-directory string would be switched off within
  a day, and a control that gets switched off is worse than one that was never claimed. One
  pattern table, two consumers, one place to add a rule.

- **D-13:** The scan covers the **full worktree**, which is the named blind spot in
  PITFALLS:88: working-tree backstops built on `git ls-files --others --exclude-standard` and
  `git diff` *skip gitignored paths*, so a secret written to `secrets/` or `*.local` is never
  caught. Concretely: walk from the repository root; skip `.git/`; **do not consult
  `.gitignore`** so `.env`, `*.local` and ignored directories are scanned; skip files that
  look binary (a NUL byte in the first 8 KiB); follow no symlinks; and apply an explicit
  per-file byte cap and an explicit total byte cap.

- **D-14:** **A file skipped for size, binary-ness or an unreadable permission is REPORTED in
  the scan result**, not silently dropped. A scanner that reports "clean" while it declined to
  read 40 files is a scanner that lies, and the whole phase is an argument against exactly
  that. The block/allow decision and the skip list are one structure, and the skip list is
  printed on both outcomes.

- **D-15:** Detection **blocks the push** (hook exits non-zero) **and parks the run** (D-23) —
  never warns, never retries, never continues. The report names the **file, the line number,
  and the rule name**, and **never the matched text**. Printing the secret into the run
  journal to explain why the secret was blocked is the SAFE-04 bug committed a second time by
  the code whose job is to prevent it.

### SAFE-05 — the scoped credential

- **D-16:** **[USER-HARD-REQUIREMENT]** The driven child's environment is scrubbed and rebuilt
  so that the user's ambient git identity is **unreachable**, not merely unused. Alongside the
  existing `CLAUDE*` scrub at `src/executor/claude.rs:426-434`:
  - `SSH_AUTH_SOCK` and `SSH_AGENT_PID` **removed** — no ambient agent, no forwarded keys.
  - `GIT_SSH_COMMAND` set to an ssh invocation with `-o IdentitiesOnly=yes -o
    IdentityAgent=none -o BatchMode=yes -F /dev/null`, so no `~/.ssh/config`, no agent, no
    default identity file, and no interactive prompt.
  - `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` pointed at an envelope-generated config
    carrying **only** `user.name` and `user.email` copied from the user's resolved config —
    so commits are attributable and do not fail — and carrying **no `credential.helper`**, so
    the user's keychain, `credential-store` and `gh` helper are all out of reach.
  - `GIT_TERMINAL_PROMPT=0`, because a detached driver's stdio is `/dev/null` (Phase 17 D-01),
    so an authentication prompt is not a prompt — it is a hang that the idle cap eventually
    kills hours later.
  - `GH_CONFIG_DIR` pointed at an envelope directory, so `gh` cannot read the user's
    `~/.config/gh/hosts.yml`.

- **D-17:** **The token is never written where it can be read back.** It reaches git only
  through `GIT_ASKPASS=<envelope>/askpass`, which re-enters this binary and writes the secret
  to stdout and nowhere else. It is never an argv element (world-readable through `/proc`),
  never a URL userinfo component (which lands in `.git/config` and in `git remote -v` output
  that this project journals), and never written into the envelope's own files. The askpass
  responder emits a credential **only for the host of the configured remote** and emits
  nothing for any other host, so an agent that adds a second remote gets an authentication
  failure rather than the user's token.

- **D-18:** Where the credential comes from: a new `DriverOptIn.credential:
  Option<CredentialSource>` that names **an environment variable to read or a command to
  run** (`gh auth token`, `pass show …`, `op read …`) — **never a literal token in
  `config.json`**, which sits in the registry next to project paths and has no protection
  posture at all. **The default is no credential, and no credential means every push fails
  closed with a legible reason** — there is deliberately no ambient fallback. SAFE-05 says the
  run must not inherit the user's credentials; "unconfigured" silently meaning "the user's
  credentials" is the exact failure the requirement names. A driver with no configured
  credential can still read, commit and be driven; it cannot push, and it says so up front
  rather than at minute 90.

### SAFE-06 — the PR rate cap

- **D-19:** The cap is enforced by the `PreToolUse` guard (D-06 layer 2) on any Bash command
  whose parsed argv is `gh pr create`, `gh api` with a `POST` to a `…/pulls` path, or `glab mr
  create`. Defaults, per PITFALLS' explicitly-arbitrary suggestion: **3 per rolling 24 hours
  per project and 1 per run**, both overridable on `DriverOptIn`. Rolling window, not calendar
  day, so a run at 23:50 cannot open three and then three more ten minutes later. The ledger
  is an append-only NDJSON file at `<envelope>/<alias>/pr-ledger.ndjson` — **outside the
  repo**, so an agent cannot reset its own cap by deleting a tracked file, and append-only so
  a torn write costs at most the last line.

- **D-20:** The ledger entry is written **before** the tool call is permitted, not after it
  succeeds. The guard is the only point that observes the attempt; a PR opened by a path the
  guard did not see is a PR the ledger cannot know about. Under-counting is the failure that
  matters, so the cap is deliberately biased to over-count: a recorded-but-failed
  `gh pr create` costs one slot out of three. That is the correct direction to be wrong in,
  and it is stated rather than left to be discovered.

### The `git add -A` worktree sweep

- **D-21:** **Worktree relocation of the driver's working directory is declined, and Phase 19
  is the phase that owns saying so finally.** Phase 17's D-21 resolved OQ4 empirically —
  `claude --worktree` exists and works — and deferred the adoption decision here. The reasons
  it is declined for v2.0, recorded as final:
  - `claude --worktree` writes **inside the repository working directory**, where
    `watcher.rs::extract_project_root` resolves the worktree's own `.planning/` as a phantom
    project on the dashboard.
  - The journal, `run.json`, the `runs/active` pointer, the reattachment scan and the whole
    Phase 16/18 model are keyed to `<project>/.planning/`. A worktree forks all of them, and
    the reattach path — a hard requirement (CTRL-04) — would have to learn which copy is
    authoritative.
  - A driven GSD run whose entire purpose is to advance `.planning/` **in the project the user
    is watching on the dashboard** is not a thing to isolate from that project. The isolation
    would defeat the feature.
  - `src/driver/spawn.rs:79-86` already carries the citation at the line where somebody would
    change it. That comment now points here.

- **D-22:** The concrete hazard is closed **mechanically and differently**, because "we
  declined isolation" is not an answer to "the agent's `git add -A` sweeps
  `.claude/worktrees/` into a commit":
  - The **`pre-commit` hook** refuses any commit whose staged paths include
    `.claude/worktrees/**`, `.planning/meta-manager/runs/**`, or any path inside a directory
    that itself contains a `.git` entry (a nested repository or linked worktree).
  - The **`pre-push` hook** rejects a push whose new commits touch those paths. This is the
    backstop for `git commit --no-verify`, which `pre-commit` by construction cannot see.
  - The envelope additionally writes those patterns into **`.git/info/exclude`**, not
    `.gitignore`. `info/exclude` is untracked and per-repo, so the agent cannot commit a
    change to it, cannot push it, and cannot leak the paths to the remote by editing a tracked
    file. This prevents the failure rather than merely punishing it.

- **D-23:** The `.git/info/exclude` write in D-22 is **the single persistent mutation the
  envelope makes to a driven repository**, and it is called out as an explicit exception to
  D-09's "touch nothing" posture rather than quietly taken. It is justified because an ignore
  rule is inert for the human, and it is made safe by being idempotent, delimited by an
  unmistakable marker block (`# >>> gsd-meta-manager envelope >>>` … `# <<<`), and rewritten
  in place rather than appended on every run. Everything else the envelope does is
  process-scoped and vanishes with the process.

### Park, reporting, and the honesty requirement

- **D-24:** Every envelope refusal **parks the run** — never retries, never silently
  continues. `JournalEvent::Parked { reason, needs }` already exists as schema at
  `src/journal/mod.rs:787` and is documented there as *"Schema only in this phase — Phase 20
  emits it"*. **Phase 19 becomes its first emitter**, which is a correction to that comment
  and must be made in the comment as well as in the code. The reason taxonomy Phase 19 owns:
  `push_outside_namespace`, `force_push_blocked`, `hook_bypass_blocked`, `secret_detected`,
  `pr_cap_exceeded`, `credential_unavailable`, `envelope_assertion_failed`. Phase 20 owns what
  *happens* to a parked run; Phase 19 owns producing the reason and writing it where a later
  reader can find it.

- **D-25:** **[USER-HARD-REQUIREMENT]** Refusals are observable **from disk, exit codes and
  git**, never from the agent's prose. Concretely: the guard writes a journal event, the hook
  writes to stderr **and** into the run journal by re-entering the binary, the affected git
  operation exits non-zero, and the run's terminal record carries the park reason. Nothing in
  the verification of this phase may consult a model's summary of what happened. This restates
  TRANS-02, and it is restated because the Replit agent's second failure was reporting success
  over a deletion it had performed.

- **D-26:** **The remote's protection state is PROBED, not assumed.** Before a run that may
  push, the envelope performs a **read-only** query for a ruleset or branch protection on the
  default branch (`gh api repos/{owner}/{repo}/rulesets`, falling back to
  `branches/{b}/protection`) and records one of `protected | unprotected | unknown(reason)`
  into the run record. `unprotected` and `unknown` both emit a warning into the journal and
  into the dry-run preview. Failure to probe — no `gh`, no network, no permission, a non-GitHub
  remote — yields `unknown` with the reason, and **`unknown` is never rendered as
  `protected`**. The envelope never applies protection: that needs `Administration` scope,
  which D-18 withholds by design.

- **D-27:** **[USER-HARD-REQUIREMENT]** The honesty statement is a **pinned constant with a
  test**, in the established style of `dry_run::SECTION_COMMANDS` /
  `SECTION_DIFFSTAT` / `SECTION_REFSPECS` (`src/driver/dry_run.rs:47-67`), whose doc comment
  already says *"these three constants are a contract, not decoration."* The same treatment
  here means the text cannot be softened by accident during a later refactor. It must say, in
  plain words and without hedging:
  - what IS mechanically guaranteed: the run cannot reach the user's ambient git credentials
    or SSH agent; a push that reaches git through the driven process tree passes the pre-push
    hook; the PR cap is enforced from a ledger the repository does not contain;
  - what is NOT: client-side hooks, tool denies and env-injected git config are all defeatable
    by an agent that can spawn an unsupervised shell and choose to; the only boundary that
    does not depend on the agent's cooperation is the **remote's own ruleset** and the
    **scope of the credential**;
  - and therefore: **enable server-side branch protection**. That recommendation is the
    phase's honest conclusion, not a footnote.

  An overstated safety claim is worse than a stated limitation, because it gets trusted.

### Carried-in constraints from earlier phases

- **D-28:** These are given, and violating any of them is a regression rather than a design
  choice:
  - `journal::run_paths` returns `Option<RunPaths>` (fallible — the WR-02 path-traversal fix).
    Every new call site handles `None` rather than unwrapping.
  - `ObservedRun.live` no longer exists. Use `liveness: Liveness` / `is_live()`.
  - `spawn::drive_argv` takes `config_path` **first** (CR-03). Any new argument this phase
    adds goes after the existing ones, and `the_drive_argv_carries_no_development_flag`'s
    pinned vector is updated in the same commit as the change, never separately.
  - `App::prune_driver_maps` is **load-bearing**. Phase 19 is expected to add no new per-alias
    in-memory map — the PR ledger is on disk and the envelope is per-run — but if planning
    finds it needs one, joining `prune_driver_maps` is part of that same task, not a
    follow-up. The Phase 16 leak returns under a new name otherwise.
  - `build_argv`'s `match options.target` stays exhaustive so Phase 22's `Container` variant
    lands as a compile error.
  - `PermissionMode` has no bypass variant and gains none. `--dangerously-skip-permissions`
    and `bypassPermissions` are never emitted.

- **D-29:** Phase 17's **D-28 blocking-call-inside-`async fn`** boundary gains a mechanical
  guard: `tests/async_blocking_guard.rs`, modelled on `tests/spawn_seam_guard.rs`. It scans
  `src/` and fails if a known-blocking call (`std::process::Command`'s `output`/`status`/
  `spawn`, `flock`, blocking `std::fs` on the driver paths) appears lexically inside an
  `async fn` body without an intervening `spawn_blocking`, with an explicit allowlist file for
  the cases that are deliberate. It is **a lint, not a proof**, and the test's own doc comment
  must say so — a lexical scanner cannot see through a helper function. It is worth having
  anyway because the deadlock it guards against was **observed, not theorised**
  (`tests/driver_lock.rs:201-215`: a blocking `flock` inside an `async fn` defeated
  `tokio::time::timeout` outright on a current-thread runtime), and because this phase adds
  several synchronous `git` shell-outs on paths Phase 18 already gave a TUI-side caller.

### Config surface and defaults

- **D-30:** New `DriverOptIn` fields, all `#[serde(default)]` so every existing v2 config loads
  unchanged — the same migration posture Phase 17's D-15 used for `driver_opt_in` itself:
  `branch_namespace: Option<String>`, `credential: Option<CredentialSource>`,
  `pr_cap_per_24h: Option<u32>`, `pr_cap_per_run: Option<u32>`. Every default is resolved in
  **one** `EnvelopePolicy::resolve(&DriverOptIn) -> EnvelopePolicy` so there is exactly one
  place a default is decided and exactly one thing to test.

### How this phase is verified

- **D-31:** The end-to-end proof uses a **local bare repository as the remote** — `git init
  --bare` in a `tempfile` dir, reached by a `file://` URL. This is what makes the ROADMAP's
  first success criterion provable *with the model removed from the equation entirely*: the
  test invokes `git push` **directly**, under the envelope's environment, against a real
  remote, and asserts the refusal. No network, no credential, no agent, deterministic in CI.
  The same fixture proves the positive case — a push **inside** the namespace succeeds — which
  is what stops the envelope from being a wall that blocks everything.

- **D-32:** SAFE-05's "still works with the user's ambient credentials and SSH agent
  unavailable to it" is verified by **asserting on the constructed child environment** —
  `SSH_AUTH_SOCK` absent, `GIT_CONFIG_GLOBAL` pointing into the envelope, no
  `credential.helper` resolvable, `GIT_TERMINAL_PROMPT=0` — **and** by a `file://` push that
  succeeds with `HOME` pointed at an empty directory. Environment assertions alone would pass
  against an envelope that also broke pushing.

- **D-33:** SAFE-03's gitignored-path criterion is verified by planting a
  `PARTS`-matching credential into a path the repo's `.gitignore` covers and asserting the
  push is refused — the specific blind spot PITFALLS:88 names, and the specific thing a
  diff-based scanner passes.

- **D-34:** Project gate: `cargo build && cargo test && cargo clippy -- -D warnings` must
  pass. `cargo clippy --all-targets` has exactly **5 pre-existing lints** (`browser.rs` ×3,
  `project_creator.rs` ×1, `state_reader/mod.rs` ×1); they are out of scope and the count must
  not grow. **cargo output is filtered by an `rtk` wrapper — raw `warning:` and `test result:`
  lines are ABSENT**, so any acceptance criterion that greps for them passes **vacuously**.
  Use `rtk proxy cargo …` when raw output is needed. Baseline is **773 tests passing at HEAD
  `740e62f`**.

- **D-35:** Every acceptance criterion must be reachable **without violating this plan's own
  scope fence**. In particular: no criterion may require a real GitHub PR, a real network
  push, a real `gitleaks` install, an agent actually running, or a Phase 20 router. If a
  criterion cannot be met inside the fence, the criterion is wrong, not the fence.

</decisions>

<code_context>
## Existing Code Insights

Verified during context gathering. Line numbers are as of `740e62f`.

### The seams this phase attaches to

- **`src/executor/claude.rs:210-266` — `build_argv(&ExecutionOptions) -> Vec<OsString>`.**
  Where `--disallowedTools` and `--settings` are added. Today it emits `-p`,
  `--input-format/--output-format stream-json`, `--verbose`, `--replay-user-messages`,
  `--session-id`, `--setting-sources project`, `--permission-mode dontAsk`,
  `--strict-mcp-config`, and optionally `--model`, `--resume`, `--max-budget-usd`, `--name`.
  Never a shell string — `process-wrap` wraps `tokio::process::Command`, which does not invoke
  a shell (T-15-06). **The argv baseline is pinned by a test at `claude.rs:1719-1731`**; it
  must be extended in the same commit as the change.
- **`src/executor/claude.rs:414-436` — the spawn closure.** `current_dir`, three piped stdio
  handles, and the `CLAUDE*` environment scrub followed by
  `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`. **This is the one place the child's environment is
  built**, so the D-16/D-09 envelope environment lands here and nowhere else. The comment
  there already explains why the scrub exists; the envelope extends the same argument from
  `CLAUDE*` to git and ssh.
- **`src/executor/mod.rs:276-345` — `ExecutionOptions`.** Where new envelope fields go. Note
  `ExecutionTarget`'s exhaustive `match` in `build_argv` is deliberately a Phase 22 tripwire.
- **`src/driver/run.rs:1016`** — `let options = ExecutionOptions::default();` is the single
  production construction site. Envelope wiring happens there, once.
- **`src/driver/mod.rs:191-295` — `drive()`.** The ordered refusal chain: registry lookup →
  **single** `DrivableProject::from_registry` gate call → dry-run branch → platform refusal →
  run-id refusals → `dispatch`. `tests/spawn_seam_guard.rs` checks the *uniqueness* of the gate
  call site, which is a property a test can hold and "every branch remembers to gate" is not.
  **Any envelope assertion added here must not introduce a second `from_registry` call site.**
- **`src/driver/spawn.rs:60-79` — `drive_argv`.** `config_path` first (CR-03). Pinned vector
  test at `spawn.rs:~290`.
- **`src/driver/spawn.rs:79-86`** — the `current_dir(project_root)` comment that explicitly
  says *"Phase 19 owns the decision"* about worktrees. D-21 is the answer; update the comment
  to point at it.

### What is already built that this phase must reuse, not fork

- **`src/journal/redact.rs:70-140` — `PARTS`.** Thirteen credential-shaped rules plus four
  path-hygiene rules, compiled once into a single `LazyLock<Regex>` alternation, with a
  leftmost-first ordering that is documented as significant and an idempotence test over a
  corpus. **This is the secret-detection corpus for SAFE-03** (D-11, D-12). The WR-15 note is
  worth reading before touching the order: the dash-encoded forms must precede the slash
  forms, and a naive reorder silently regressed seven of eight fixtures.
- **`src/state_reader/git_ops.rs`** — `push_refspecs(root) -> PushPreview` (line 351) already
  resolves the *implicit* push destination locally from `branch.<b>.remote`,
  `remote.pushDefault`, `remote.<r>.push` and `push.default`, with **no network and no
  credential**. D-08's "resolved destination when no refspec is given" must call this rather
  than re-deriving it. `working_tree_stat` (282), `current_branch` (247), `is_dirty` (139),
  `head_sha` (108) are the other read-only helpers.
- **`src/driver/dry_run.rs`** — three pinned section constants with a test that asserts all
  three appear **in order**, so a section cannot silently disappear. D-27's honesty text
  follows this pattern exactly. The module doc's D-23 note ("zero git writes and zero agent
  spawns, proved mechanically" — `git reflog`, every ref and the whole `.git` listing captured
  before and after, plus a tripwire program that leaves an evidence file if executed) is the
  model for how to prove the envelope's read-only claims.
- **`src/journal/mod.rs:787-792` — `JournalEvent::Parked`.** Exists, documented as
  Phase-20-only. D-24 makes Phase 19 its first emitter.
- **`src/journal/mod.rs:194` — `runs_root(planning) = planning/meta-manager/runs`**, and
  `RUNS_SUBDIR` at line 93. The path D-22's `info/exclude` block names.
- **`tests/spawn_seam_guard.rs`** (624 lines) — the source-scanning guard pattern D-29 copies.
- **`src/executor/mod.rs:137-170` — `DrivableProject`**, the capability token with exactly one
  production constructor. The envelope should take this token rather than a bare path, so
  "the envelope only ever runs for an opted-in project" is a type-level property.

### Facts verified directly against the installed CLI (`claude 2.1.220`)

The version gate at `src/executor/gate.rs:59-66` pins `MINIMUM_CLAUDE_VERSION = 2.1.214` and
`TESTED_MAXIMUM_CLAUDE_VERSION = 2.1.220`.

- **`--disallowedTools, --disallowed-tools <tools...>`** exists — *"Comma or space-separated
  list of tool names to deny (e.g. `Bash(git *) Edit`)"*. Confirms the `Bash(<pattern>)` form.
- **`--allowedTools, --allowed-tools <tools...>`** exists, same form.
- **`--settings <file-or-json>`** exists — *"Path to a settings JSON file or a JSON string to
  load additional settings from"*. This is how the `PreToolUse` hook reaches the child without
  writing into the repository's `.claude/`.
- **The `-p` silent-ignore behaviour is real and documented in `--help`:** *"Settings files
  that fail validation are silently ignored in this mode (no error dialog is shown)."* This is
  the direct source of D-07 and is the single most dangerous fact in this list.
- `--setting-sources` takes `user,project,local`; the executor already passes `project` only,
  as the reproduced hook-hang mitigation (D-01 of Phase 15). **Project-tier `.claude/settings.json`
  IS therefore loaded from the driven repo**, and it is agent-writable — which is why the
  envelope denies `Write`/`Edit` on `.claude/**` rather than relying on it.
- `--permission-mode` accepts `bypassPermissions`; the codebase's `PermissionMode` enum has no
  such variant, and D-28 keeps it that way.

### Environment facts

- `gitleaks` is **not** on `PATH`. `gh`, `git 2.43.0` and `jq` are.
- `git 2.43.0` supports `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_n`/`GIT_CONFIG_VALUE_n` (since
  2.31), which is what D-09 rests on.
- `dirs::data_local_dir()/gsd-meta-manager/` is the established app data directory —
  `src/main.rs:19-22` (log) and `src/config.rs:170` (registry).
- `.claude/worktrees/` in this repository is currently **empty**, and it is where this
  milestone's own executors run. That is why D-22's hazard is concrete rather than
  hypothetical.

</code_context>

<specifics>
## Specific Ideas

### The three-layer table belongs in the code, not only in the docs

Write D-06's layer table as a doc comment on `src/envelope/mod.rs` with, for each layer, the
sentence "this layer does NOT see …". The value is not documentation; it is that a future
change which removes one layer has to delete a sentence that says what it was for.

### The guard's input is JSON on stdin, and it must be fast

`PreToolUse` hooks run synchronously on the agent's critical path. This project has already
been bitten by that: `src/executor/mod.rs:225-239` records a reproduced 180-240 s hang caused
by the user's global `PreToolUse` hooks having no timeout, and `--setting-sources project` is
the shipped mitigation. **The envelope's own hook must not reintroduce the bug it was born
from.** Concretely: no network in the guard path (the D-26 remote probe happens once at run
start, not per tool call), the ledger read is a single append-only file, and the guard must
carry an explicit timeout in the settings file it is registered from. This is the single
biggest self-inflicted-wound risk in the phase and should be an explicit acceptance criterion.

### Argv parsing of a Bash command string is the weak joint — say so

Layer 2 has to turn `"git push --force origin main"` into argv. A shell-quoting-aware split
handles the honest cases; it does **not** handle `g=push; git $g --force`, `eval`, base64, or a
script the agent writes and then runs. Do not pretend otherwise in the doc comment. The
correct framing is: layer 2 raises the cost of an accident to near-certain detection, and
layer 3 is what covers deliberate evasion of layer 2 — up to the point where layer 3 is itself
evaded, at which point D-27's server-side recommendation is the only remaining answer.

### The positive path needs a test as much as the negative one

Three of the five requirements are refusals, and it is very easy to ship an envelope that
refuses everything and passes every criterion. Every refusal test needs a paired
allow test: a push **inside** `gsd-auto/<alias>/` succeeds; the third PR of the day succeeds
and the fourth does not; a worktree with no secret scans clean and pushes.

### Suggested plan shape (planner is free to disagree)

Roughly six plans, wave-ordered by what compiles against what:

1. `src/envelope/policy.rs` — the pure decision functions (`classify_git`, namespace
   validation, refspec resolution reusing `git_ops::push_refspecs`) plus `EnvelopePolicy` and
   the `DriverOptIn` schema additions with their migration test. No I/O, no processes.
2. `src/envelope/scan.rs` + the `PARTS` `SecretClass` split — SAFE-03's detector, the full
   worktree walk with its reported skip list, and the gitignored-path fixture.
3. `src/envelope/hooks.rs` + `cred.rs` + the `envelope` subcommand — hook stub generation,
   provenance check, the `GIT_CONFIG_COUNT` environment, the askpass responder, and the
   `file://` bare-remote end-to-end fixture proving both the refusal and the allow.
4. `src/envelope/ledger.rs` + the `PreToolUse` guard + the settings-file generation and its
   round-trip validation — SAFE-06 and D-07.
5. Wiring: `build_argv`, the spawn closure, `run.rs`'s single `ExecutionOptions` site, the
   `Parked` emission and reason taxonomy, `advisory.rs`'s probe, and the pinned honesty text
   into the dry-run preview.
6. Gate, clippy-delta audit, criterion traceability, `tests/async_blocking_guard.rs` (D-29),
   and the ROADMAP/REQUIREMENTS status updates.

</specifics>

<deferred>
## Deferred Ideas

- **Applying server-side rulesets from the tool** — needs `Administration` scope on the
  credential, which D-18 withholds on purpose. Phase 19 detects and recommends. If a later
  milestone wants to apply, it needs a **separate, explicitly-consented** credential, and that
  is a product decision rather than a code one.
- **A signed or attested hook** (so a relocated copy is cryptographically distinguishable
  rather than only path-distinguishable, per D-10) — real, but it does not change the outcome
  against an agent that can unset an environment variable, and D-27 already tells the truth
  about that ceiling.
- **Container-delivered credentials and egress allowlisting** → Phase 22. PITFALLS is explicit
  that egress restriction is what makes a credential read non-fatal; the host path cannot
  offer that, and the honesty text must not imply it can.
- **Extending the envelope to the user's own git commands** in a registered project — the
  meta-manager is not a repo policy engine, and D-09 chose process-scoped delivery precisely
  so that the human is unaffected.
- **An allowlist over git verbs instead of D-08's denylist** — stronger, and it would break
  every GSD skill that runs an unanticipated plumbing command. Revisit once the Phase 20
  router has narrowed which commands a driven run actually issues, because *then* the
  allowlist becomes computable instead of guessed.
- **`gitleaks` as a hard dependency, or vendoring an equivalent ruleset** — D-11 makes it
  additive. If a later milestone wants breadth beyond `PARTS`, the honest form is an optional
  configured scanner command with a documented fail-closed contract, not a silent upgrade.
- **Entropy-based secret detection** (high-entropy string heuristics) on top of the pattern
  rules — high false-positive rate against Rust source, base64 fixtures and lock files, and a
  push gate that cries wolf gets disabled. Pattern rules first; measure before adding.

</deferred>
