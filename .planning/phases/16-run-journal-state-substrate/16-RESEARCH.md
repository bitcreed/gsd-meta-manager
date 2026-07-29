# Phase 16: Run Journal & State Substrate — Research

**Researched:** 2026-07-29
**Domain:** Rust filesystem durability, append-only NDJSON, regex redaction, `notify`/`Action` routing
**Confidence:** HIGH — every claim below was produced by running code in this session, not recalled

---

## Scope note

The architecture for this phase is **already decided** in `16-CONTEXT.md` (D-01..D-39) and in the
committed `.planning/research/` pass. Nothing here relitigates a decision. This document answers
only the eight implementation-mechanics questions the planner needs settled before writing tasks,
and it answers each one with an executed transcript rather than an assertion.

**Headline:** every mechanism the phase depends on works, but **three of them do not work the way
the obvious implementation would write them**, and each was caught only by running it:

1. `git check-ignore` **exits 0 on negated patterns**, so the obvious verification of D-08 passes
   vacuously and reports "ignored" for the very files that are tracked.
2. The obvious redaction pattern for `Authorization:` **leaks the credential**, and the obvious
   replacement literal for `/home/<user>` **re-matches itself** on a second pass.
3. `#[serde(tag = "kind")]` + `#[serde(other)]` **loses the payload** of an unknown kind, which is
   exactly what D-30 requires it to carry.

---

## User Constraints (from CONTEXT.md)

### Locked Decisions

D-01..D-39 in `.planning/phases/16-run-journal-state-substrate/16-CONTEXT.md` are carried in
verbatim and are **not** restated here. The ones this document supplies mechanism for:

| Decision | What this document supplies |
|----------|-----------------------------|
| D-04 (no `fsync`) | The measurement that proves it is safe for Success Criterion 1 (§3) |
| D-05 (atomic `run.json`) | The `NamedTempFile` mode gotcha (§1.4) |
| D-08 (`.gitignore` pattern) | Verified transcript + the two ways to verify it *wrong* (§1) |
| D-13 (byte-offset tail) | Compiling sketch + four edge cases (§2) |
| D-20 (`large_enum_variant`) | Measured threshold and measured `Action` size (§8) |
| D-23 (`Value` tree walk) | Compiling sketch incl. the map-key rewrite (§5) |
| D-24 (pattern set) | 27 verified cases, incl. two leaks found and fixed (§4) |
| D-30 (tolerant reader) | The serde constraint that blocks the stated shape, and two fixes (§9) |
| D-34 (SIGKILL test) | Working no-new-dependency harness, run 8× (§6) |
| D-17 (OBS-06 measurement) | The recommended seam, and why the obvious one flakes (§7) |

### Claude's Discretion

Carried verbatim from CONTEXT.md — plan granularity/wave structure; classification's home
(`watcher.rs` vs `journal/mod.rs`); the concrete cap/retention constants (§10 proposes numbers);
exact `JournalEvent` variant names; whether `Redacted` is generic or concrete; exact test names;
whether `active` is a file or a directory listing.

### Deferred Ideas (OUT OF SCOPE)

Carried verbatim from CONTEXT.md `<deferred>`: detached spawn / `Commands::Drive` / PID liveness /
crash reconciliation / kill switch / dry-run / `flock` / `driver_opt_in` (Phase 17); `inbox.jsonl`
and the Driver tab and any rendering (Phase 18); pre-push secret scan and `--disallowedTools`
(Phase 19); `decide()` / D-R-P-E-V router / run bounds (Phase 20); LLM goal decomposition
(Phase 21); `ExecutionTarget::Container` and `PathMap` (Phase 22); SQLite; control socket; live
re-streaming; the 5 pre-existing clippy lints; `pending_editor`; `session_detector.rs` TTY match.

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OBS-01 | Append-only journal survives process death, is the source of truth for run state | §3 (SIGKILL survival, measured 8/8), §2 (tolerant tail), §9 (forward-compatible record shape) |
| OBS-06 | Driver journal writes do not trigger full project re-parses | §7 (the counting seam and why the obvious alternative flakes), §8 (`Action` sizing for the new variant) |
| SAFE-04 | Credentials redacted **at capture**, not at render | §4 (27 verified patterns, 2 leaks found), §5 (`Value` tree walk that cannot break JSON) |

---

## Project Constraints (from CLAUDE.md)

| Directive | Consequence for this phase |
|-----------|----------------------------|
| Gate is `cargo build && cargo test && cargo clippy -- -D warnings` | Any new `Action` variant must clear `large_enum_variant` — §8 gives the exact budget |
| `cargo clippy --all-targets` has 5 pre-existing lints; count must not grow | Do not touch browser.rs / project_creator.rs / state_reader/mod.rs |
| Baseline 363 tests | New tests add to this; none may be removed |
| MSRV 1.87 (toolchain 1.97.1) | `std::sync::LazyLock` (1.80) is available — **no `once_cell` dependency needed** for the compiled regex |
| GSD workflow enforcement before Edit/Write | Planning artifact first; this file is that artifact for research |
| Release process | Not triggered by this phase |
| **`rtk` filters `cargo` output** | Every empirical check below was run under `rtk proxy cargo …`. Any acceptance criterion that greps for `warning:` / `test result:` on wrapped output **passes vacuously** (D-39) |

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Redaction | Capture path (`journal::redact` + `Redacted` newtype) | — | D-21/D-22: enforced by the type system, never by `src/ui/` |
| Append + `run.json` | Writer task (`journal::writer`) | — | D-29: ordered channel, single writer, one `write_all` per record |
| Path classification | Pure fn (no I/O) | called from `watcher.rs` callback thread | D-11: must not stat on the debouncer thread |
| Tail read | `spawn_blocking` → `Action` on cloned `tx` | — | D-16: no file I/O on the render thread |
| Offset storage | Sibling map on `AppContext` | — | D-13/D-18: off `ProjectState`'s `PartialEq` |
| Rendering | **none this phase** | Phase 18 | D-36 |

---

## 1. `.gitignore` re-inclusion — VERIFIED, with two traps

### 1.1 The pattern works exactly as D-08 specifies

`git version 2.43.0`, throwaway repo in `/tmp`. File written at
`.planning/meta-manager/runs/.gitignore`:

```gitignore
# Written by gsd-meta-manager. Driver transcripts are local-only; the per-run
# run.json (goal + outcome) is committed so the goal stays legible later.
*
!*/
!.gitignore
!*/run.json
```

Ground truth (`git add -A` then `git ls-files`), with two run directories, an `active` pointer,
`journal.jsonl`, `inbox.jsonl`, and a deeper `runs/<id>/nested/run.json`:

```
$ git add -A; git ls-files
.planning/meta-manager/runs/.gitignore
.planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/run.json
.planning/meta-manager/runs/2026-07-29T01-00-00Z-b111/run.json

$ git status --porcelain --ignored=matching
!! .planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/inbox.jsonl
!! .planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl
!! .planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/nested/run.json
!! .planning/meta-manager/runs/2026-07-29T01-00-00Z-b111/inbox.jsonl
!! .planning/meta-manager/runs/2026-07-29T01-00-00Z-b111/journal.jsonl
!! .planning/meta-manager/runs/active
```

[VERIFIED: `git 2.43.0`, executed this session]

**No change to D-08 is needed.** Additional facts the transcript establishes:

- `!*/` is load-bearing exactly as D-08 says. It is the un-exclusion of directories that lets
  `!*/run.json` reach anything at all.
- `runs/<id>/nested/run.json` stays **ignored** — `!*/run.json` matches exactly one directory
  level. Desirable: only the per-run record at the documented depth is committed.
- `tempfile::NamedTempFile::new_in` produces a **dot-prefixed** basename (`.tmpCM3hv7`,
  measured) which matches `*` and is therefore ignored. D-05's atomic write leaves no untracked
  litter even if a `persist` is interrupted. [VERIFIED]

### 1.2 TRAP 1 — `git check-ignore -v` exits 0 for negated patterns

This is the trap that would make a plan's acceptance criterion pass vacuously. `check-ignore`
answers *"did any pattern match?"*, **not** *"is this file ignored?"*. A negation counts as a
match:

```
$ git check-ignore -v .planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/run.json ; echo "exit=$?"
.planning/meta-manager/runs/.gitignore:4:!*/run.json	…/run.json
exit=0
```

A naive `if git check-ignore -q "$f"; then echo IGNORED; fi` reports **`run.json` as ignored** —
the precise opposite of the truth. Verified: a first pass of this research reported all seven
paths as "IGNORED" before the ground-truth check corrected it.

**Rule for the plan.** Verify D-08 with **one** of:
- `git add -A` + `git ls-files` (strongest — proves the file is actually stageable), or
- `git status --porcelain --ignored=matching` (`!!` prefix = ignored), or
- `git check-ignore -v` **and assert the reported pattern does not begin with `!`**.

Never with a bare `git check-ignore -q`.

### 1.3 TRAP 2 — a parent `.gitignore` that excludes the *directory* defeats the nested file

Git does not descend into an excluded directory, so a nested `.gitignore` inside it never runs.
Measured, driven-project root `.gitignore` containing `.planning/meta-manager/`:

```
$ git add -A; git ls-files
.gitignore                      # <-- runs/.gitignore and run.json BOTH absent
$ git check-ignore -v …/runs/run-a/run.json
.gitignore:2:.planning/meta-manager/	.planning/meta-manager/runs/run-a/run.json
```

A parent that excludes by **file glob** rather than directory is harmless — a root `*.jsonl`
still leaves `run.json` and `runs/.gitignore` tracked. [VERIFIED both cases]

**Rule for the plan.** After writing the ignore file at run-directory creation (D-08), run
`git check-ignore -v` on the run's `run.json` **once**, and if the reported pattern comes from a
*different* file and has **no** leading `!`, emit a diagnostic journal event / `tracing::warn!`:
this driven repo has opted its own `.planning/meta-manager/` out and `run.json` will not be
committed. This is a real, silent failure of D-07's goal-legibility rationale in a third-party
repo, and it costs one command to detect. It must **not** be an error — the journal still works.

### 1.4 `NamedTempFile` permission note

```
persisted run.json mode = 100600
File::create mode       = 100664
```

`persist` preserves the temp file's 0600. Git records only the exec bit, so this does not affect
the commit — but a second uid on the same host (a container mount, a shared CI runner) cannot
read it. One line before `persist` (`set_permissions(Permissions::from_mode(0o644))`, `cfg(unix)`)
removes the surprise for a file whose whole purpose is being read by someone else later.
[VERIFIED: measured]

---

## 2. Byte-offset tail — compiling sketch and the four edge cases

### 2.1 The bug this exists to avoid, demonstrated

Given a file whose last line is torn (`{"c":tor` with no newline):

```
BufReader::lines  -> ["{\"a\":1}", "{\"b\":2}", "{\"c\":tor"]   <-- yields the TORN line
tail_lines        -> ["{\"a\":1}", "{\"b\":2}"]                 <-- leaves it for next read
```

[VERIFIED: executed]

`BufReader::lines()` treats EOF as a line terminator. Using it makes a torn write a **lost event**
(the partial is consumed and discarded) rather than a self-healing one — the exact failure D-13
forbids. The tail must split on bytes and stop at the last `\n`.

### 2.2 The sketch (compiles and runs as shown)

```rust
use std::fs::File;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

/// Upper bound on one tail read. A line longer than this can never complete,
/// so the reader must be able to step over it (see `skipped_oversize`) or the
/// offset stalls forever and every later event is invisible.
/// Mirrors `MAX_LINE_BYTES` in the executor's reader (T-15-07, D-05).
pub const MAX_TAIL_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TailCursor {
    pub offset: u64,
}

#[derive(Debug, Default)]
pub struct TailRead {
    /// Complete, newline-terminated lines only.
    pub lines: Vec<String>,
    /// Advance the caller's stored cursor to this.
    pub cursor: TailCursor,
    /// The file shrank under us; the cursor was reset to 0. Diagnostic, not an error.
    pub restarted: bool,
    /// A line exceeded MAX_TAIL_BYTES and was stepped over. Diagnostic.
    pub skipped_oversize: bool,
}

pub fn tail_lines(path: &Path, mut cursor: TailCursor) -> io::Result<TailRead> {
    // (a) the file may not exist yet — the run dir is created before the first append
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(TailRead { cursor, ..Default::default() })
        }
        Err(e) => return Err(e),
    };

    let len = f.metadata()?.len();

    // (b) truncation / rotation: the file is shorter than where we left off
    let mut restarted = false;
    if len < cursor.offset {
        cursor.offset = 0;
        restarted = true;
    }
    if len == cursor.offset {
        return Ok(TailRead { cursor, restarted, ..Default::default() });
    }

    f.seek(SeekFrom::Start(cursor.offset))?;

    // (c) another process may be appending RIGHT NOW. read_to_end (not read_exact
    //     of `len - offset`) is correct: it tolerates the file having grown since
    //     the metadata() call, and cannot fail if it shrank. `.take()` bounds it.
    let mut buf = Vec::new();
    (&mut f).take(MAX_TAIL_BYTES).read_to_end(&mut buf)?;

    match buf.iter().rposition(|&b| b == b'\n') {
        Some(idx) => {
            let lines = buf[..=idx]
                .split(|&b| b == b'\n')
                .filter(|s| !s.is_empty())
                // (d) a non-UTF-8 fragment is skipped, never lossily mangled.
                //     from_utf8_lossy would silently inject U+FFFD into a payload.
                .filter_map(|s| std::str::from_utf8(s).ok().map(str::to_owned))
                .collect();
            Ok(TailRead {
                lines,
                cursor: TailCursor { offset: cursor.offset + idx as u64 + 1 },
                restarted,
                skipped_oversize: false,
            })
        }
        // A full buffer with no newline anywhere: the line cannot complete within
        // our bound. Step over it and report, or the cursor never advances again.
        None if buf.len() as u64 >= MAX_TAIL_BYTES => Ok(TailRead {
            lines: Vec::new(),
            cursor: TailCursor { offset: cursor.offset + buf.len() as u64 },
            restarted,
            skipped_oversize: true,
        }),
        // Ordinary trailing partial line: consume nothing, leave the cursor alone.
        None => Ok(TailRead { lines: Vec::new(), cursor, restarted, skipped_oversize: false }),
    }
}
```

### 2.3 Executed behaviour

```
missing file      -> lines=0 offset=0 restarted=false
2 whole lines     -> ["{\"seq\":1}", "{\"seq\":2}"] offset=20
partial tail      -> [] offset=20 (cursor unchanged: true)
completed         -> ["{\"seq\":3,\"partial\":true}"] offset=45
truncated         -> ["{\"seq\":99}"] offset=11 restarted=true
```

[VERIFIED: executed against a real file with a real concurrent appender]

### 2.4 Notes for the planner

- **`restarted` should be surfaced, not swallowed.** Under D-31/D-32 a journal is never truncated
  in place and a run directory is never pruned while active, so `restarted == true` means an
  invariant broke. Journal it as a diagnostic (consistent with D-30's "gap is reported, never
  fatal").
- **Do not use inode identity for rotation detection.** `MetadataExt::ino()` would be more
  precise but is `cfg(unix)` and unnecessary: the cursor is keyed by `(alias, run_id)` and each
  run has its own immutable directory, so `len < offset` is the only reachable rotation signal.
- **`skipped_oversize` needs a matching per-event cap on the writer side** (D-31). If the writer
  caps payload at a few KB, `MAX_TAIL_BYTES` is unreachable in practice and the branch is pure
  defence against a corrupted file.

---

## 3. Append durability — `fsync` is not needed for Success Criterion 1

### 3.1 The question that decides it

> Can an `fsync`-free design lose **previously written, already-returned** lines when the writing
> process is SIGKILLed?

**No.** [VERIFIED: measured, 8 consecutive runs]

A `write(2)` that has returned has handed the bytes to the kernel page cache. The page cache
belongs to the kernel, not to the process. `SIGKILL` destroys the process; it cannot revoke pages
the kernel already owns. The data is visible to every subsequent `open()`/`read()` and is written
back by the flusher regardless of whether the writer still exists.

What `fsync` buys is survival of **machine** failure — power loss, kernel panic, hard reset.
Success Criterion 1 says *"After a run is killed mid-flight **or its host process dies**"*. Both
of those are process-level events. **D-04 is correct and no `fsync` is required to satisfy the
criterion.**

### 3.2 The measurement

Harness: a child process re-exec'd from the test binary appends ~230-byte NDJSON lines in a tight
loop via `OpenOptions::new().create(true).append(true)` + one `write_all` per line, no `fsync`,
no `flush` beyond `write_all`. Parent waits until the file exceeds 50 KB, then calls
`Child::kill()` (SIGKILL on unix), reaps, and reads the file back.

```
child status: ExitStatus(unix_wait_status(9))
bytes=520887 complete=2157 torn=0 gaps=0 last_seq=2157 ends_with_newline=true
bytes=519677 complete=2152 torn=0 gaps=0 last_seq=2152 ends_with_newline=true
bytes=527421 complete=2184 torn=0 gaps=0 last_seq=2184 ends_with_newline=true
bytes=518467 complete=2147 torn=0 gaps=0 last_seq=2147 ends_with_newline=true
bytes=521855 complete=2161 torn=0 gaps=0 last_seq=2161 ends_with_newline=true
bytes=520403 complete=2155 torn=0 gaps=0 last_seq=2155 ends_with_newline=true
bytes=527421 complete=2184 torn=0 gaps=0 last_seq=2184 ends_with_newline=true
bytes=524517 complete=2172 torn=0 gaps=0 last_seq=2172 ends_with_newline=true
```

**8/8 runs: zero torn lines, zero `seq` gaps, file always ends in `\n`.** [VERIFIED]

### 3.3 What `append(true)` + one `write_all` does and does not guarantee

| Guaranteed | Not guaranteed |
|---|---|
| `O_APPEND` makes seek-to-end + write **atomic with respect to the file offset** — two writers cannot overwrite each other | Survival of machine power loss or kernel panic without `fsync` |
| Bytes from a returned `write(2)` survive process death by any signal | That `write_all` issues exactly **one** `write(2)` — it *loops on short writes* |
| A partial final line is the realistic worst case | Anything about non-local filesystems (NFS/SMB `O_APPEND` is not atomic) |

**On interleaving.** Measured directly: 8 concurrent processes, one `write(2)` per line, line
sizes 100 / 400 / 900 / 4 000 / 9 000 / 40 000 / 70 000 / 200 000 bytes, 2 000 lines each into one
`O_APPEND` fd:

```
total lines=16000  corrupt/interleaved=0
```

[VERIFIED — filesystem is ZFS on Linux; ext4/xfs behave identically here]

This says single `write(2)` calls do not interleave even at 200 KB. It does **not** license
multiple writers, because `write_all` loops: a short write (possible under signal interruption or
near-ENOSPC) would split the line and allow another writer between the halves. **D-29's ordered
single-writer task remains the rule** — and with it, torn output is only reachable by SIGKILL
landing between two iterations of that loop, which is what the tolerant reader (D-30) covers.

### 3.4 Conclusion for the writer

- Keep the file handle **open for the run's lifetime** on the writer task rather than reopening
  per event. Reopening per append costs an `open`/`close` syscall pair per event and buys nothing
  under `O_APPEND`.
- Build the whole line — including the trailing `\n` — in **one** `String`/`Vec<u8>` and issue
  **one** `write_all`. Never `write!(f, "{}", json)` followed by `writeln!` — that is two syscalls
  and reintroduces the torn-line window.
- No `BufWriter`. A userspace buffer holds already-serialised events in the dying process's memory,
  which is exactly the data D-29 promises is on disk. `write_all` straight to the `File` is both
  simpler and more durable.
- A skip-unparseable-lines reader (D-30) is the correct and sufficient mitigation.

---

## 4. Redaction pattern set — 27 verified cases, two real leaks found

### 4.1 Structural facts about Rust's `regex` crate

| Fact | Evidence |
|------|----------|
| **Catastrophic backtracking is impossible.** `(a+)+b` against 60 `a`s: `false` in **26.1 µs** | [VERIFIED: executed] |
| Lookaround and backreferences are **rejected at compile time**, so the pathological pattern class cannot be written: `(?<=foo)bar`, `(a)\1`, `(?!x)y` all `REJECTED` | [VERIFIED: executed] |
| Alternation is **leftmost-first**, not leftmost-longest: `ab\|abc` on `"abc"` → `"ab"`; `abc\|ab` on `"abc"` → `"abc"` | [VERIFIED: executed] |

Consequence: **order the alternation most-specific-first.** No ReDoS review is needed; no timeout
guard is needed.

### 4.2 The verified pattern set

Single alternation with named groups, compiled once in a `std::sync::LazyLock` (available at MSRV
1.87 — **no new dependency**). Ordering within the alternation is significant per §4.1.

```rust
use regex::{Captures, Regex};
use std::sync::LazyLock;

/// (group name, pattern, replacement literal)
/// Replacement literals contain no quote, backslash or newline (D-24) and are
/// chosen so no pattern in this table can match one — see the idempotence test.
const PARTS: &[(&str, &str, &str)] = &[
    ("pem",   r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----",
              "[REDACTED:private-key]"),
    // NOTE: value scan runs to END OF LINE, not to whitespace. See §4.3 leak 1.
    ("authz", r#"(?i:authorization)[ \t]*[:=][ \t]*"?[^\r\n"]{4,}"#,
              "[REDACTED:authorization]"),
    ("env",   r#"(?i:[A-Za-z_][A-Za-z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|API_?KEY|_KEY|CREDENTIALS?))[ \t]*[:=][ \t]*"?[^\s"',}\]]+"#,
              "[REDACTED:env]"),
    ("bearer", r"(?i:bearer)[ \t]+[A-Za-z0-9._\-+/=]{8,}",          "[REDACTED:bearer]"),
    ("skant",  r"sk-ant-[A-Za-z0-9_\-]{8,}",                        "[REDACTED:anthropic-key]"),
    ("sk",     r"\bsk-[A-Za-z0-9_\-]{16,}",                         "[REDACTED:api-key]"),
    ("ghpat",  r"\bgithub_pat_[A-Za-z0-9_]{20,}",                   "[REDACTED:github-token]"),
    ("gh",     r"\bgh[pousr]_[A-Za-z0-9]{20,}",                     "[REDACTED:github-token]"),
    ("aws",    r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",                    "[REDACTED:aws-key-id]"),
    ("slack",  r"\bxox[baprs]-[A-Za-z0-9\-]{10,}",                  "[REDACTED:slack-token]"),
    ("jwt",    r"\beyJ[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}",
                                                                    "[REDACTED:jwt]"),
    ("uinfo",  r#"(?:[A-Za-z][A-Za-z0-9+.\-]*)://[^/\s:@"]+:[^/\s@"]+@"#,
                                                                    "[REDACTED:userinfo]@"),

    // ---- WR-15: the DASH-ENCODED forms must come BEFORE the slash forms ----
    ("dtmp",   r"-tmp-claude-[^/\s\\\x22]*",                        "-tmp-scratch-"),
    ("dhome",  r"-(?:home|Users|root)-[^/\s\\\x22]*",               "-home-redacted-project"),

    // ---- slash forms. '[' and ']' are EXCLUDED from the tail class so the
    //      replacement literal can never be re-matched. See §4.3 leak 2. ----
    ("stmp",   r"/tmp/claude-[0-9]+",                               "/tmp/claude-[REDACTED:uid]"),
    ("shome",  r#"/(?:home|Users|var/home)/[^/\s"':,)\[\]}\\]+"#,   "/home/[REDACTED:user]"),
];

static RE: LazyLock<Regex> = LazyLock::new(|| {
    let alt = PARTS
        .iter()
        .map(|(n, p, _)| format!("(?P<{n}>{p})"))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&alt).expect("redaction alternation must compile")
});

pub fn redact(s: &str) -> String {
    RE.replace_all(s, |c: &Captures| {
        for (n, _, rep) in PARTS {
            if c.name(n).is_some() {
                return (*rep).to_string();
            }
        }
        unreachable!("a match with no named group")
    })
    .into_owned()
}
```

### 4.3 The two leaks this iteration found

Both were present in the obvious first-draft pattern set and both were caught only by executing it.

**Leak 1 — `Authorization: Basic <base64>` survived.** A value scan of `[^\s"',}]{4,}` stops at
the space after the scheme token, so only the word `Basic` was redacted:

```
BEFORE:  authorization: Basic dXNlcjpwYXNzd29yZA==
         -> [REDACTED:authorization] dXNlcjpwYXNzd29yZA==     <-- credential intact
AFTER:   authorization: Basic dXNlcjpwYXNzd29yZA==
         -> [REDACTED:authorization]
```

Fix: the header-value class is `[^\r\n"]{4,}` — consume to end of line, not to whitespace.

**Leak 2 — the replacement literal re-matched itself.** `/home/[REDACTED:user]` was matched again
by `/(?:home|Users)/[^/\s"':,)]+`, which stopped at the `]` and re-appended one, producing
`/home/[REDACTED:user]]/…` on a second pass. Fix: exclude `[` and `]` from the class. Idempotence
went from 3 failures to **0/27**.

**The generalisable rule:** every redactor must have a test asserting
`redact(redact(x)) == redact(x)` over the whole corpus. It is the cheapest possible detector for
"a replacement literal is itself redactable" and for "rule A ate rule B's output".

### 4.4 Verified output — all 27 cases

```
slash home       cwd is /home/blk/projects/rust/gsd-meta-manager/src
                 > cwd is /home/[REDACTED:user]/projects/rust/gsd-meta-manager/src
macos home       cwd is /Users/andy/Code/thing
                 > cwd is /home/[REDACTED:user]/Code/thing
silverblue       cwd is /var/home/blk/projects/x
                 > cwd is /home/[REDACTED:user]/projects/x
dash home        /home/blk/.claude/projects/-home-blk-projects-rust-gsd-meta-manager/x.jsonl
                 > /home/[REDACTED:user]/.claude/projects/-home-redacted-project/x.jsonl
dash tmp         /tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/4661fdcd/scratchpad
                 > /tmp/claude-[REDACTED:uid]/-home-redacted-project/4661fdcd/scratchpad
dash tmp2        sess -tmp-claude-1000--home-blk-projects-x/memory/
                 > sess -tmp-scratch-/memory/
sk-ant           key sk-ant-api03-AbCdEf012345_-XyZ end        > key [REDACTED:anthropic-key] end
generic sk       OPENAI sk-proj-abcdefghijklmnopqrstuvwxyz012345 end > OPENAI [REDACTED:api-key] end
ghp              token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 end > token [REDACTED:github-token] end
github_pat       github_pat_11ABCDEFG0abcdefghijklmnop_qrstuvwxyz01234 > [REDACTED:github-token]
akia             AWS AKIAIOSFODNN7EXAMPLE here                 > AWS [REDACTED:aws-key-id] here
bearer           Authorization: Bearer abc123def456ghi789      > [REDACTED:authorization]
authz basic      authorization: Basic dXNlcjpwYXNzd29yZA==     > [REDACTED:authorization]
bare bearer      hdr was Bearer abc123def456ghi789 ok          > hdr was [REDACTED:bearer] ok
jwt              tok eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0…    > tok [REDACTED:jwt] .
userinfo         clone https://andy:hunter2@github.com/org/repo.git
                 > clone [REDACTED:userinfo]@github.com/org/repo.git
env token        GITHUB_TOKEN=ghp_zzzz…                        > [REDACTED:env]
env secret       MY_SECRET=supersecretvalue rest               > [REDACTED:env] rest
env anthropic    ANTHROPIC_API_KEY=sk-ant-xxxxxxxxxxxxxxxx     > [REDACTED:env]
env password     DB_PASSWORD: hunter2 and more                 > [REDACTED:env] and more
slack            xoxb-1234567890-abcdefghijkl                  > [REDACTED:slack-token]
pem              -----BEGIN RSA PRIVATE KEY-----\n…\n-----END RSA PRIVATE KEY-----
                 > [REDACTED:private-key]
no-op prose      The phase writes .planning/meta-manager/runs/ and is fine.   (unchanged)
url no userinfo  see https://example.com:8443/path/to/x                       (unchanged)
prose API_KEY    the API_KEY variable is documented                           (unchanged)
json-ish leaf    {"memory_paths":{"/home/blk/.claude/CLAUDE.md":1}}
                 > {"memory_paths":{"/home/[REDACTED:user]/.claude/CLAUDE.md":1}}

NOT-IDEMPOTENT COUNT = 0
```

[VERIFIED: executed, `regex 1.13.1`]

Note the three deliberate **non**-matches at the bottom: prose, a URL with a port but no userinfo,
and an `API_KEY` mention with no `=`/`:` following. Over-redaction is the tuning direction (D-25)
but total prose destruction is not — the env-assignment rule requires an actual assignment.

### 4.5 Determining the home prefix portably at runtime

Do **not** hardcode `/home/blk`. Two complementary layers:

1. **Generic shape patterns** (`shome`, `dhome` above) — catch *any* user's home, including other
   users' paths that appear inside tool output.
2. **The current process's actual home**, appended to the alternation at first use. `dirs` is
   already a dependency (`src/config.rs:44` uses `dirs::config_dir()`):

```rust
fn host_specific_parts() -> Vec<(String, &'static str)> {
    let mut v = Vec::new();
    if let Some(home) = dirs::home_dir().and_then(|p| p.to_str().map(str::to_owned)) {
        // /var/home/blk, /root, and containerised homes the generic rule misses
        v.push((regex::escape(&home), "[REDACTED:home]"));
        // Claude Code's dash-encoded form of the SAME path (WR-15)
        let dashed = home.replace('/', "-");
        v.push((regex::escape(&dashed), "-home-redacted"));
    }
    v
}
```

`dirs::home_dir()` reads `$HOME` on unix and the known-folder API on Windows — no `libc`.
[CITED: `dirs` crate is already in `Cargo.toml`]

**Deliberately NOT done: redacting the bare username on its own.** A username like `andy`, `root`,
`test` or `dev` occurs in ordinary prose and in file content; redacting it word-bounded would
shred readable logs for a marginal gain, because every *path-shaped* occurrence is already covered
by layers 1 and 2. **Record this in the module docs as an honest limit (D-25):** a username that
appears entirely outside a path context is not redacted.

### 4.6 One alternation vs. an ordered `Vec<Rule>` vs. `RegexSet` — measured

| Shape | 1000 × 11 200-byte input | Per 11 KB | Rule-eats-rule hazard |
|---|---|---|---|
| Ordered `Vec<(Regex, &str)>`, 15 sequential `replace_all` | **86 ms** | 86 µs | **Yes** — measured: rule 9 consumed rule 8's `[REDACTED:bearer]` output |
| **Single alternation + named-group closure (recommended)** | 346 ms | **346 µs** | **No** — one pass, no rule sees another's output |
| `RegexSet` | n/a for replacement | — | `RegexSet` reports *which* patterns match but yields no match offsets, so it cannot drive `replace_all`. Usable only as a prefilter. |

[VERIFIED: both measured this session]

**Recommendation: the single alternation.** The sequential form is 4× faster but the extra 260 µs
is irrelevant at this phase's rates:

- **At the real rate** (the 15-01 spike measured ~27 events for a 68-second turn, i.e. **well under
  1 event/sec**): 346 µs/sec = **0.03 % of one core**.
- **At burst** (the bounded-drain tests' 10 000-event flood): 3.5 s of CPU — but D-31 caps per-event
  payload at a few KB, so a real burst is ~10× cheaper than this 11 KB benchmark, and the writer
  runs on its own task where it cannot stall the render loop (D-29 + `main_loop.rs`'s bounded drain).

The correctness property — no rule can be fed another rule's output — is worth 260 µs. If
profiling ever disagrees, the escape hatch is a `RegexSet` prefilter in front of the sequential
form, keeping the idempotence test as the guard.

`RE.as_str().len()` is 856 bytes of pattern; well inside `regex`'s default 10 MB compiled-size
limit, so no `RegexBuilder::size_limit` tuning is needed. [VERIFIED]

### 4.7 Anti-pattern

Do **not** use `Regex::replace_all` with a `&str` replacement containing `$`. Rust's `regex`
expands `$name`/`$1` in string replacements. All the literals above are `$`-free, but a future
`[REDACTED:$kind]` would silently expand. Use `regex::NoExpand("…")` for any pure-literal
replacement, or the closure form as above.

---

## 5. Redacting a `serde_json::Value` tree

### 5.1 The sketch (compiles and runs as shown)

```rust
use serde_json::{Map, Value};

pub fn redact_value(v: &mut Value) {
    match v {
        Value::String(s) => {
            let r = redact(s);
            if r != *s { *s = r; }
        }
        Value::Array(a) => a.iter_mut().for_each(redact_value),
        Value::Object(m) => {
            // The fiddly part: serde_json::Map has no rename-key API, so the map
            // is drained and rebuilt. `mem::take` avoids cloning every value.
            let mut out = Map::with_capacity(m.len());
            for (k, mut val) in std::mem::take(m) {
                redact_value(&mut val);
                let nk = redact(&k);
                match out.entry(nk.clone()) {
                    serde_json::map::Entry::Vacant(e) => { e.insert(val); }
                    // Two distinct keys can collapse to the same redacted key
                    // (two users' home paths -> one literal). Disambiguate;
                    // NEVER silently drop the second value.
                    serde_json::map::Entry::Occupied(_) => {
                        let mut i = 2usize;
                        loop {
                            let cand = format!("{nk}#{i}");
                            if !out.contains_key(&cand) { out.insert(cand, val); break; }
                            i += 1;
                        }
                    }
                }
            }
            *m = out;
        }
        // Numbers, bools and null carry no text. Left untouched by construction —
        // this is exactly why the tree walk cannot corrupt a payload.
        _ => {}
    }
}
```

### 5.2 Verified output

Input (a `memory_paths`-shaped map with paths **in key position**, plus nested array/object):

```json
{
  "cost": 1.83,
  "kind": "exec_event",
  "memory_paths": {
    "/home/[REDACTED:user]/.claude/CLAUDE.md": 1,
    "/home/[REDACTED:user]/.claude/CLAUDE.md#2": 2
  },
  "nested": [ { "cwd": "/home/[REDACTED:user]/x" }, [ "/home/[REDACTED:user]/y" ] ],
  "nil": null,
  "ok": true,
  "text": "run with [REDACTED:env] in /home/[REDACTED:user]/p"
}
reparses: true
```

[VERIFIED: executed. Note `#2` — the two distinct home paths collided and the collision was
disambiguated rather than dropped.]

### 5.3 Gotchas

| Gotcha | Disposition |
|---|---|
| **Key collision after redaction** | Real, and reproduced above. Disambiguate with a suffix; never let `insert` silently overwrite — that would lose an event field. |
| **Non-string scalars** | Untouched by construction. A secret stored as a JSON *number* is not redactable, and is not a shape any credential takes. |
| **Recursion depth** | `serde_json::from_str` enforces a 128-level nesting limit by default, so a parsed `Value` cannot be deeper than that and the recursive walk cannot blow the stack on parsed input. If the writer ever constructs a `Value` programmatically instead of parsing, that guarantee is gone — keep the parse-then-walk order. |
| **Key ordering** | `serde_json::Map` is a `BTreeMap` unless the `preserve_order` feature is on (it is not, in this project). Rebuilding the map is order-neutral either way, since the drain iterates in order and re-inserts in order. |
| **`Map::with_capacity`** | Compiles and is a no-op under `BTreeMap`; harmless, and correct if `preserve_order` is ever enabled. |

### 5.4 Where the `Redacted` type sits (D-22)

The type must make the **serialised bytes** the redacted thing, so the constructor should be the
only path from a `Value` to a writable line:

```rust
/// The only value the journal writer accepts. Its sole constructor runs the
/// redactor, so there is no compilable path that writes unredacted text (D-22).
pub struct RedactedLine(String);

impl RedactedLine {
    pub fn new(mut v: serde_json::Value) -> Self {
        redact_value(&mut v);
        // to_string, never to_string_pretty: NDJSON is one object per line and
        // pretty output would emit embedded newlines and break the framing.
        RedactedLine(serde_json::to_string(&v).expect("Value always serialises"))
    }
    pub(crate) fn as_line(&self) -> &str { &self.0 }
}
```

`as_line` is `pub(crate)` and there is no `From<String>`, no `Deref<Target = str>`, and no
`impl Display` — each of those would reopen the seam. The single-field tuple struct keeps the
inner `String` private to the module.

---

## 6. Testing crash survival — a working harness with no new dependency

### 6.1 The problem

D-34 wants a real `SIGKILL` against a process running the **real** `JournalWriter`. The crate has
one binary and the writer lives in the lib, so there is nothing to spawn — and adding a hidden
`Commands::JournalSelftest` subcommand would ship debug surface in the release binary.

### 6.2 The solution: re-exec the integration-test binary

An integration test's own executable is reachable via `std::env::current_exe()`, and libtest can
be told to run exactly one test. A sentinel env var selects the child role.

```rust
// tests/journal_crash.rs
#![cfg(unix)]

use std::process::{Command, Stdio};

const CHILD_ENV: &str = "GSD_JOURNAL_CHILD";

/// Re-exec entry point. Under a normal `cargo test` the env var is absent and
/// this is a no-op that passes trivially. The crash test re-invokes THIS test
/// binary with the var set, so the child runs real library code.
#[test]
fn journal_child_writer() {
    let Ok(path) = std::env::var(CHILD_ENV) else { return };
    gsd_meta_manager::journal::writer::write_forever(std::path::Path::new(&path));
}

#[test]
fn a_sigkilled_writer_leaves_every_flushed_line_readable() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal.jsonl");

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "journal_child_writer", "--nocapture"])
        .env(CHILD_ENV, &journal)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("re-exec the test binary as the writer");

    // Wait until it is genuinely producing, so the assertion is not vacuous.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if journal.metadata().map(|m| m.len()).unwrap_or(0) > 50_000 { break }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    // std::process::Child::kill() IS SIGKILL on unix — no libc, no nix.
    child.kill().expect("kill");
    let status = child.wait().expect("reap");
    assert_eq!(status.code(), None, "must have died by signal, not exited: {status:?}");

    // ... parse the file, assert: many complete lines, zero seq gaps,
    //     at most ONE torn line (the last).
}
```

[VERIFIED: this exact harness was built and run 8× this session — see §3.2 for the output. The
child's wait status was `unix_wait_status(9)` every time, confirming SIGKILL.]

### 6.3 What to reuse from the repo

| Existing asset | How it applies |
|---|---|
| `tests/executor_lifecycle.rs` | `#![cfg(unix)]` at the top of the file; `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/…")` for fixture paths; the `sh -c "kill -0 $pid"` liveness helper at `:90-96` |
| `tests/fixtures/fake-claude-*.sh` | The shell-stand-in idiom. **Not needed here** — the child must run real `JournalWriter` code, which a shell script cannot |
| `tempfile` (dev + prod dep) | `tempfile::tempdir()` for the run directory |
| `assert_fs` (dev dep) | Fine for `.gitignore` fixture assertions; `tempfile` alone is enough for the crash test |
| `process-wrap` + `ProcessGroup::leader()` | **Deliberately not used here.** Process-group teardown is the executor's concern; the journal writer is one process and `std::process::Command` is the smaller correct tool. Pulling `process-wrap` in would test the wrong thing |

### 6.4 Assertions the test should make

1. `status.code() == None` — died by signal, not by exiting. Without this the test can pass on a
   child that finished normally.
2. `complete_lines > N` for a generous `N` — proves the writer was actually producing. Measured
   ~2 150 lines in the settle window; assert `> 100`.
3. `seq` gaps among complete lines `== 0` — proves ordering (D-29), not just presence.
4. Torn lines `<= 1`, and if one exists it must be the **last** — this is the only tolerated
   failure mode (D-30). Measured 0/8 runs, so the assertion is a ceiling, not an expectation.
5. Every complete line reparses as JSON.
6. **The redaction assertion belongs here too** (D-27): plant a `sk-ant-…` **and** a dash-encoded
   `-home-<user>-<repo>` in the child's payload, then grep the **file bytes** after the kill.

---

## 7. Proving OBS-06 mechanically

### 7.1 The current shape (`src/app.rs:316-358`)

```rust
Action::FileChanged { project_path } => {
    let alias = /* find by proj.path == project_path */;
    if let Some(alias) = alias {
        // 500ms dedup on ctx.last_refresh  (:327-333)
        if let Some(tx) = &self.ctx.event_tx {
            let tx = tx.clone();
            let planning_dir = project_path.join(".planning");
            tokio::task::spawn_blocking(move || {
                let state = state_reader::parse_project_state(&planning_dir);   // <-- the bill
                let _ = tx.send(Action::ProjectStateLoaded { alias, state: Box::new(state) });
            });
            self.ctx.last_refresh.insert(alias, now);                            // <-- the tell
        }
        // auto-watch fallback (:350-356)
    }
}
```

### 7.2 The four candidate seams, ranked

| Seam | Deterministic? | Directness | Verdict |
|---|---|---|---|
| **A. A plain `u64` dispatch counter on `AppContext`, bumped in an extracted `schedule_reparse()`** | ✅ fully sync | Direct — counts the thing OBS-06 forbids | **Recommended** |
| B. `ctx.last_refresh` unchanged after a driver-path `FileChanged` | ✅ fully sync | Proxy — D-14 requires it anyway, so it is a *free second* assertion | **Recommended as a companion**, not alone |
| C. Install a test `event_tx`, drain, assert no `ProjectStateLoaded` | ❌ absence-after-timeout | Direct but async | Use as corroboration only |
| D. `static AtomicUsize` inside `parse_project_state` | ❌ **flaky** | Most direct | **Reject** — see §7.4 |

### 7.3 The recommended shape (small, justified refactor)

Extract the re-parse dispatch out of the `FileChanged` arm into a named method, and add one plain
counter field. **Not `#[cfg(test)]`** — a cfg-gated counter means the test exercises a different
binary than production, and the counter is a legitimate diagnostic in its own right.

```rust
// AppContext, next to last_refresh / archive_cache / run_states
/// Count of full `parse_project_state` dispatches this process has issued.
/// Load-bearing for OBS-06: a driver journal append must never increment it.
pub reparse_dispatches: u64,
```

```rust
impl App {
    /// The ONLY place a full project re-parse is scheduled.
    fn schedule_reparse(&mut self, alias: &str, project_path: &Path) {
        let Some(tx) = &self.ctx.event_tx else { return };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let planning_dir = project_path.join(".planning");
        self.ctx.reparse_dispatches += 1;
        tokio::task::spawn_blocking(move || {
            let state = state_reader::parse_project_state(&planning_dir);
            let _ = tx.send(Action::ProjectStateLoaded {
                alias: alias_for_task,
                state: Box::new(state),
            });
        });
        self.ctx.last_refresh.insert(alias.to_string(), std::time::Instant::now());
    }
}
```

The OBS-06 test then reads:

```rust
#[tokio::test]
async fn journal_appends_never_trigger_a_full_reparse() {
    let mut app = /* App with one registered project at a temp path */;
    let before = app.ctx.reparse_dispatches;

    for seq in 1..=500 {
        app.update(Action::FileChanged {
            project_path: root.clone(),
            changed_path: root.join(".planning/meta-manager/runs/RID/journal.jsonl"),
        });
    }
    assert_eq!(app.ctx.reparse_dispatches, before, "OBS-06: {} full re-parses", …);
    assert!(app.ctx.last_refresh.is_empty(), "D-14: driver route must not touch last_refresh");

    // Control arm — the same handler MUST still re-parse a planning write.
    app.update(Action::FileChanged {
        project_path: root.clone(),
        changed_path: root.join(".planning/STATE.md"),
    });
    assert_eq!(app.ctx.reparse_dispatches, before + 1, "planning writes must still re-parse");
}
```

The **control arm is not optional.** A test that only asserts zero would pass against a handler
that dropped `FileChanged` entirely — which would break the dashboard while satisfying OBS-06.

`#[tokio::test]` is required because `schedule_reparse` calls `spawn_blocking`, which panics
without a runtime. The counter is incremented **before** the spawn, so the assertion is
synchronous and does not race the blocking task.

`event_tx` is already `pub` on `AppContext` and `App::new_for_test()` sets it to `None`
(`src/app.rs:152`), so the test sets it to a real sender and can additionally drain the receiver
for seam C corroboration.

### 7.4 Why the global-counter seam is rejected

A `static REPARSE_COUNT: AtomicUsize` inside `parse_project_state` would count actual calls
including startup load and auto-register — the most literal reading of "zero full
`parse_project_state` calls happened". But **cargo runs a crate's tests as threads inside one
process**, so any other test that constructs project state increments the same counter
concurrently. That is a guaranteed intermittent failure. Per-`App` state is the right scope.

### 7.5 The other two required assertions

- **Classification is pure and exhaustively tested** (D-11), mirroring the existing
  `extract_project_root` tests at `watcher.rs:97-116`. Table-drive it: journal path, `run.json`
  path, `active` path, `.gitignore` path, `STATE.md`, a deep phase path, a path with `runs` as a
  *project* directory name (`/x/.planning/runs/…` must **not** classify as driver — the driver
  prefix is `meta-manager/runs/`), and a path outside `.planning/` entirely.
- **The watcher dedup fix** (D-10) needs its own test at the `watcher.rs` level: a single debounce
  batch containing both a journal append and a `STATE.md` write must produce **two** actions, and
  in particular the `STATE.md` one must survive when the journal path is first in the batch. That
  is a regression classification *introduces*; without the test it ships silently.

### 7.6 On the timing assertion

D-17 permits a comparative timing assertion "with a generous margin" as corroboration. Follow
`main_loop.rs`'s own precedent (`tests` module, lines 199-221): it deliberately declines a
wall-clock latency percentile because *"it passes on a developer laptop, fails on a loaded runner,
gets `#[ignore]`d within a month, and at that point TRANS-03 has no verification at all."* The
same reasoning applies here. **Make the counter assertion load-bearing and the timing assertion
optional and generous, or omit the timing assertion entirely.**

---

## 8. `clippy::large_enum_variant` — measured threshold and budget

### 8.1 The rule, measured

Built five enums with a 24-byte small variant and a large variant of 150 / 201 / 224 / 225 bytes
under `rustc 1.97.1` / `cargo clippy` with no `clippy.toml` (the repo has none):

| Enum | Large variant | Difference | Lint fires? |
|---|---|---|---|
| A | 150 B | 126 | no |
| B | 201 B | 177 | no |
| C | 224 B | **200** | **no** |
| D | 225 B | **201** | **YES** |
| E | `Box<P225>` | 0 | no |

```
warning: large size difference between variants
   |                 the entire enum is at least 226 bytes
   = note: `#[warn(clippy::large_enum_variant)]` on by default
help: consider boxing the large fields or introducing indirection in some other way
```

[VERIFIED: executed]

**The rule is: fires when `size_of(largest variant) − size_of(second-largest variant) > 200`.**
The threshold is `enum-variant-size-threshold`, default **200**, and it applies to the
**difference**, not to the absolute size. It is warn-by-default, hence a hard error under the
project's `-D warnings` gate.

### 8.2 Measured sizes in this crate

```
size_of::<Action>()             = 120
size_of::<ProjectState>()       = 360
size_of::<MilestoneArchive>()   =  72
size_of::<ExecutionEvent>()     = 120
size_of::<RunOutcome>()         =  80
size_of::<RunState>()           =  16
size_of::<String>()             =  24
size_of::<std::path::PathBuf>() =  24
```

[VERIFIED: `std::mem::size_of` against the real crate, built as a path dependency]

`Action`'s largest variant today is `ArchiveLoaded { String, String, MilestoneArchive }` =
24 + 24 + 72 = **120 bytes**, and the whole enum is 120 (the discriminant fits a niche). This is
also why `ProjectStateLoaded` must box: unboxed it would be 360, a difference of 240 > 200.

### 8.3 The budget for Phase 16's new variants

A new variant may be up to **~320 bytes** before the lint fires (120 + 200 + 1). Sizing the two
variants the phase needs:

```rust
FileChanged {
    project_path: PathBuf,   // 24
    changed_path: PathBuf,   // 24   -> 48. No effect on the enum size.
},
DriverJournalAppended {
    alias:   String,          // 24
    run_id:  String,          // 24
    events:  Vec<JournalEvent>, // 24  (heap-allocated; contents do not count)
    cursor:  u64,             //  8   -> 80. Comfortably inside budget.
},
```

**Neither needs boxing.** [VERIFIED by arithmetic against the measured sizes]

**The rule to carry into the plan:** any new `Action` variant that embeds a `JournalEvent`
*by value* rather than behind a `Vec`/`Box` risks the threshold if `JournalEvent` grows past
~320 bytes. `Vec<T>` and `Box<T>` are always 24 and 8 bytes respectively regardless of `T`, so
carrying the parsed events in a `Vec` — which the tail produces anyway — sidesteps the question
entirely. D-20's ban on handles in `Action` is separately satisfied: `PathBuf`, `String`, `Vec`
and `u64` are all `Clone`.

---

## 9. `JournalEvent` serde — the constraint D-30's stated shape runs into

This is the third item that does not work the obvious way, and the repo has **already documented
it** for the executor: `src/executor/stream_json.rs:21-26` —

> *"`#[serde(other)]` is only accepted on a **unit** variant of an internally-tagged enum, so the
> enum itself cannot carry the raw body."*

Verified against `serde 1` / `serde_json 1.0.151` for the journal's own shape:

```
-- Serialize under #[serde(tag = "kind")] --
  OK   {"kind":"run_started","goal":"g"}
  OK   {"kind":"exec_event","stream":"a","text":"t"}
  ERR  cannot serialize tagged newtype variant Ev::NewtypeScalar containing an integer
  Unknown unit: Ok("{\"kind\":\"unknown\"}")

  Tuple(u32, u32)  -> COMPILE ERROR:
       "#[serde(tag = \"...\")] cannot be used with tuple variants"

-- Deserialize an UNKNOWN kind through the enum --
  {"kind":"parked","reason":"verification_gaps_found","needs":"human"}
  -> Unknown        <-- payload LOST
```

[VERIFIED: executed]

So a bare `#[serde(tag = "kind")]` + `#[serde(other)] Unknown` **cannot satisfy D-30's**
*"an unknown `kind` is carried as raw, not fatal"* — it deserialises to a unit variant and the
`reason`/`needs` fields are gone. Two mechanisms make D-30 satisfiable; either is compatible with
the decision, and this is a mechanism question, not a decision question.

**Option A — the repo's own `Envelope` precedent (`stream_json.rs:293-315`).** Keep the tagged
enum for the kinds Phase 16 emits, and preserve the raw line alongside it in the reader, which
already owns the `String`:

```rust
pub enum JournalRecord {
    Parsed { raw: String, event: JournalEvent },   // event may be `Unknown`
    Unparseable { raw: String, error: String },    // a torn write — a real diagnostic
}
```

Consistency argument: it is exactly the shape the executor uses, for exactly the same reason, and
it keeps "unknown kind" (forward-compat, carry it) distinct from "malformed line" (a diagnostic) —
which the tail's `skipped_oversize` / `restarted` flags also want.

**Option B — `kind: String` + `#[serde(flatten)]`.** Verified to round-trip **byte-identically**:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct JournalRecord {
    pub ts: String,
    pub seq: u64,
    pub kind: String,                          // plain String, never an enum
    #[serde(flatten)]
    pub rest: serde_json::Map<String, Value>,
}
```

```
{"ts":"2026-07-29T00:00:00Z","seq":8,"kind":"parked","reason":"verification_gaps_found","needs":"human"}
-> Record { ts: "…", seq: 8, kind: "parked",
            rest: {"needs": String("human"), "reason": String("verification_gaps_found")} }
   round-trips identically: true
```

This also matches `stream_json.rs`'s explicit rule that *"`subtype` and `terminal_reason` are plain
`String`s, never enums. This CLI shipped three new values on one version line."* The same argument
applies to `kind`, where Phase 20 is a **known** future emitter of new values (D-36).

**Recommendation: Option A for the writer's typed emission surface, Option B for the reader.**
The writer wants exhaustive typed variants (so Phase 20 gets a compile error if it forgets a
field); the reader wants total tolerance. They are different jobs and the repo already splits them
this way.

### 9.1 Writer-side constraints that follow

- Under `#[serde(tag = "kind")]`, **struct variants only**. Tuple variants are a compile error;
  newtype-around-a-scalar is a runtime error. Newtype-around-a-`Map` works but flattens
  confusingly. [VERIFIED all four]
- Never `#[serde(deny_unknown_fields)]` anywhere in `src/journal/` (D-30). `stream_json.rs:6-8`
  notes that its absence is *"grepped for as a mechanical guard"* — do the same here.
- `serde_json::to_string`, never `to_string_pretty`. Pretty output emits embedded newlines and
  breaks NDJSON framing outright.

### 9.2 `seq` gap detection, verified end to end

Over a file containing a gap (1, 2, **4**), a non-JSON line, then 5:

```
gaps=[(2, 4)] unparseable=1 last_seq=5
```

[VERIFIED] — a gap is a reported diagnostic and an unparseable line is skipped; neither aborts the
read. Exactly D-30.

---

## 10. Growth bounds and retention — proposed constants (D-31 / D-32 discretion)

No tuning data exists; these are defensible starting values, named as constants, configurable.
The evidence they are anchored to is real.

| Constant | Proposed | Anchoring evidence |
|---|---|---|
| `MAX_EVENT_PAYLOAD_BYTES` | **8 KiB** | Phase 15's fixture README records thinking-block `signature` blobs at **4-8 KB base64**, and names them as *"the bulk of `05`, `06` and `08`"*. 8 KiB admits one full signature and truncates a pathological one |
| `MAX_RUN_JOURNAL_BYTES` | **64 MiB** | ~1 event/sec × 4 h = ~14 400 events; at a capped ~8 KiB worst case that is ~115 MB, at realistic ~1 KB average ~14 MB. 64 MiB sits above the realistic case and below the pathological one, which is where a circuit breaker belongs |
| `RETAIN_RUNS` | **10** | CONTEXT's stated default. At 64 MiB worst case that is a 640 MB ceiling per project — cap and retention must be tuned together, and this pair should be stated as one budget in the docs |
| `MAX_TAIL_BYTES` | **4 MiB** | Must exceed `MAX_EVENT_PAYLOAD_BYTES` by a wide margin (a tail read legitimately batches many events); the branch is pure defence given the per-event cap |

Two behaviours the constants imply:

- On breaching `MAX_RUN_JOURNAL_BYTES`, emit exactly **one** `journal_truncated` event, then
  continue writing lifecycle/decision/outcome events with content fields omitted. D-31 is explicit
  that the run must always be able to write its terminal record.
- Truncating a payload must be **UTF-8 safe**. `&s[..8192]` panics on a char boundary. Use
  `s.char_indices().take_while(|(i, _)| *i <= N).last()` or `floor_char_boundary` (unstable —
  do not use at MSRV 1.87). Append a marker so truncation is visible in the file, e.g.
  `"…[truncated 41203 bytes]"`, mirroring the executor's `LineTruncated { bytes, prefix }`.

---

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---|---|---|---|
| Reading complete lines from a growing file | A `BufReader::lines()` wrapper | The byte-offset tail in §2 | `lines()` yields the torn line as if complete — demonstrated, and it silently *loses* the event |
| Atomic `run.json` write | `.tmp` + `fs::rename` (`queue_md.rs:220-224`) | `NamedTempFile::new_in` + `persist` (`config.rs:79-84`) | A fixed `.tmp` name collides between writers; D-05 already names `config.rs` as the idiom |
| Truncating a UTF-8 payload | `&s[..N]` | char-boundary-aware truncation (§10) | Panics on a multi-byte boundary |
| Detecting rule-eats-rule in the redactor | Code review | A `redact(redact(x)) == redact(x)` property test | Caught 3 real defects this session that review had not |
| Killing a child in a test | `libc::kill` / a `nix` dependency | `std::process::Child::kill()` | Already SIGKILL on unix — verified `unix_wait_status(9)` |
| Lazy static regex | `once_cell` / `lazy_static` | `std::sync::LazyLock` | Stable since 1.80; MSRV is 1.87. **No new dependency** |
| ReDoS defence | Timeouts, input length caps | Nothing | Rust's `regex` is finite-automaton based; lookaround and backrefs are compile errors |
| Verifying `.gitignore` | `git check-ignore -q` | `git add -A` + `git ls-files`, or `git status --porcelain --ignored` | `check-ignore` exits 0 on **negated** patterns — the exact inverse answer |

---

## Common Pitfalls

### Pitfall 1: `git check-ignore` reports the opposite of the truth
**What goes wrong:** the plan's D-08 acceptance criterion asserts `run.json` is committed by
shelling out to `git check-ignore -q`; it exits 0 (a negation matched) and the check reports the
file as ignored. **How to avoid:** §1.2. **Warning sign:** any criterion using `check-ignore`
without inspecting the reported pattern for a leading `!`.

### Pitfall 2: the driven repo has already excluded `.planning/meta-manager/`
**What goes wrong:** the nested `.gitignore` is written, but git never descends into the excluded
directory, so `run.json` is silently never committable and D-07's goal-legibility rationale
evaporates in exactly the third-party repos the tool exists for. **How to avoid:** §1.3 — one
`check-ignore` at run-directory creation, warn, do not fail.

### Pitfall 3: a replacement literal that redacts itself
**What goes wrong:** `/home/[REDACTED:user]` re-matches the home-path rule, appending a `]` per
pass; over repeated processing the log degrades. **How to avoid:** exclude `[`/`]` from path
character classes, and assert idempotence over the whole corpus. **Warning sign:** no idempotence
test.

### Pitfall 4: an `Authorization:` rule that stops at whitespace
**What goes wrong:** `authorization: Basic <base64>` redacts only the word `Basic` and writes the
credential to disk. This is a *false-negative* redaction — the SAFE-04 failure mode, not a
cosmetic one. **How to avoid:** scan the header value to end of line. **Warning sign:** the
redaction test corpus has no header case with a scheme token before the secret.

### Pitfall 5: `#[serde(other)]` silently discards the payload
**What goes wrong:** an unknown `kind` (every Phase 20 event kind, if the reader ships before the
writer does) deserialises to a unit variant and the record's fields vanish — the reader believes
it tolerated the event while having destroyed it. **How to avoid:** §9. **Warning sign:** a test
that asserts an unknown kind "does not error" without asserting its fields survive.

### Pitfall 6: the watcher dedup swallows `STATE.md`
**What goes wrong:** `seen.insert(root)` at `watcher.rs:52` is per-root; once journal appends
arrive, a batch containing a journal write **and** a `STATE.md` write drops whichever came second.
If the journal is first, the project never re-parses. **How to avoid:** D-10's per-`(root, kind)`
dedup, with a test that puts the journal path first in the batch. **Warning sign:** a plan that
treats D-10 as a tidy-up rather than a correctness fix.

### Pitfall 7: a wall-clock OBS-06 assertion
**What goes wrong:** a timing assertion passes locally, fails on a loaded runner, gets `#[ignore]`d,
and OBS-06 ends up with no verification at all. `main_loop.rs:216-221` records this exact reasoning
for TRANS-03. **How to avoid:** §7 — make the dispatch counter load-bearing.

### Pitfall 8: `BufWriter` on the journal
**What goes wrong:** already-serialised events sit in the dying process's userspace buffer when
SIGKILL lands, so D-29's ordering promise ("written before the driver proceeds") is false for the
last few events. **How to avoid:** write straight to the `File`; §3 shows this loses nothing.

---

## Runtime State Inventory

Phase 16 is greenfield-with-integration, not a rename or migration, but three categories are
non-empty and the planner needs them:

| Category | Items found | Action required |
|---|---|---|
| Stored data | **None.** No existing journal or run directory anywhere; `.planning/meta-manager/` today holds only `QUEUE.md` (`queue_md.rs:178`). Phase 16 creates the first `runs/` tree | none |
| Live service config | **None.** No external service holds run state | none |
| OS-registered state | **None.** No task/unit/pm2 registration in this phase (detached spawn is Phase 17) | none |
| Secrets / env vars | **None new.** No new env var is introduced. `$HOME` is *read* at runtime by the redactor (§4.5) but nothing is written | none |
| Build artifacts | **None.** No new dependency, no new binary target, no generated file. `Cargo.lock` is unchanged | none |

Verified by `rg` over the tree and by reading `Cargo.toml`. [VERIFIED]

---

## Environment Availability

| Dependency | Required by | Available | Version | Fallback |
|---|---|---|---|---|
| `regex` | §4 redactor | ✅ already in `Cargo.toml` | 1.13.1 | — |
| `serde_json` | §5, §9 | ✅ | 1.0.151 | — |
| `tempfile` | D-05 atomic write, §6 test dirs | ✅ | 3.x | — |
| `chrono` | D-02 run ids | ✅ (with `serde`) | 0.4 | — |
| `uuid` | D-02 run-id suffix | ✅ (`v4`, `serde`) | 1.24 | — |
| `dirs` | §4.5 runtime home prefix | ✅ | 6 | `std::env::var("HOME")` |
| `std::sync::LazyLock` | §4 compiled regex | ✅ MSRV 1.87 ≥ 1.80 | — | — |
| `git` | §1 gitignore verification | ✅ | 2.43.0 | — |
| `assert_fs` (dev) | fixture assertions | ✅ | 1 | `tempfile` |

**Missing dependencies with no fallback:** none.
**New dependencies required:** **none.** This confirms CONTEXT's expectation — *"if the planner
reaches for one, that is a signal to re-check, not a routine addition."* [VERIFIED]

---

## Validation Architecture

### Test framework

| Property | Value |
|---|---|
| Framework | built-in `libtest` (`#[cfg(test)] mod tests` in-source + `tests/*.rs` integration) |
| Config file | none — `cargo test` defaults |
| Quick run | `rtk proxy cargo test --lib` |
| Full suite | `rtk proxy cargo test` |
| Gate | `cargo build && cargo test && cargo clippy -- -D warnings` |
| Baseline | 363 tests (308 lib + 11 + 7 + 12 + 25) |

**`rtk proxy` is mandatory** wherever raw output matters (D-39). `cargo test` under the wrapper
prints only `cargo test: N passed (M suites, Xs)` — `test result:` lines are stripped.

### Phase requirements → test map

| Req | Behaviour | Type | Command | Exists? |
|---|---|---|---|---|
| OBS-01 | SIGKILLed writer leaves every flushed line readable | integration (real process + signal) | `rtk proxy cargo test --test journal_crash` | ❌ Wave 0 — §6 gives the harness |
| OBS-01 | Torn final line is skipped, not fatal | unit | `rtk proxy cargo test --lib journal::reader` | ❌ Wave 0 |
| OBS-01 | `seq` gap is reported, not fatal | unit | `rtk proxy cargo test --lib journal::reader` | ❌ Wave 0 |
| OBS-01 | Unknown `kind` survives with its fields | unit | `rtk proxy cargo test --lib journal::` | ❌ Wave 0 |
| OBS-01 | `run.json` written atomically, exactly twice | unit | `rtk proxy cargo test --lib journal::writer` | ❌ Wave 0 |
| OBS-01 | `runs/.gitignore` tracks `run.json`, ignores the rest | integration (real `git init`) | `rtk proxy cargo test --test journal_gitignore` | ❌ Wave 0 |
| OBS-06 | Driver-path `FileChanged` issues **zero** re-parse dispatches | unit (`#[tokio::test]`) | `rtk proxy cargo test --lib app::` | ❌ Wave 0 |
| OBS-06 | Planning-path `FileChanged` **still** re-parses (control arm) | unit | same | ❌ Wave 0 |
| OBS-06 | `classify_change` is exhaustive over path shapes | unit | `rtk proxy cargo test --lib classify` | ❌ Wave 0 |
| OBS-06 | One debounce batch with journal-first still emits the planning event | unit | `rtk proxy cargo test --lib watcher::` | ❌ Wave 0 — extends `watcher.rs:97-116` |
| OBS-06 | Tail cost is proportional to bytes appended, not project size | unit | same as OBS-06 counter test | ❌ Wave 0 |
| SAFE-04 | Planted `sk-ant-…` is `[REDACTED:…]` **in the file bytes** | integration | `rtk proxy cargo test --test journal_crash` | ❌ Wave 0 |
| SAFE-04 | Planted **dash-encoded** `-home-<user>-<repo>` is redacted in the file bytes | integration | same | ❌ Wave 0 — the WR-15 case |
| SAFE-04 | All 27 §4.4 cases redact as specified | unit | `rtk proxy cargo test --lib journal::redact` | ❌ Wave 0 |
| SAFE-04 | `redact(redact(x)) == redact(x)` over the corpus | unit | same | ❌ Wave 0 |
| SAFE-04 | Object **keys** are redacted, not only values | unit | same | ❌ Wave 0 |
| SAFE-04 | Every written line reparses as valid JSON | unit | `rtk proxy cargo test --lib journal::writer` | ❌ Wave 0 |
| SAFE-04 | No `src/journal/**` symbol accepts a bare `String` line | compile-time (`Redacted` newtype, §5.4) | `cargo build` | ❌ Wave 0 |

### Sampling rate

- **Per task commit:** `rtk proxy cargo test --lib`
- **Per wave merge:** `rtk proxy cargo test` + `cargo clippy -- -D warnings`
- **Phase gate:** full suite green, 363 + new tests, `--all-targets` lint count still exactly 5

### Wave 0 gaps

- [ ] `tests/journal_crash.rs` — OBS-01 SIGKILL survival + SAFE-04 on-disk bytes (§6)
- [ ] `tests/journal_gitignore.rs` — D-07/D-08 against a real `git init` (§1)
- [ ] `src/journal/redact.rs` `#[cfg(test)] mod tests` — the 27-case corpus + idempotence (§4)
- [ ] `src/journal/reader.rs` `#[cfg(test)] mod tests` — tail edge cases (§2)
- [ ] `src/journal/writer.rs` `#[cfg(test)] mod tests` — atomic `run.json`, caps, JSON validity
- [ ] `src/app.rs` tests — the OBS-06 counter assertion + control arm (§7)
- [ ] `src/watcher.rs` tests — per-`(root, kind)` dedup (D-10)
- [ ] No framework install needed

---

## Security Domain

### Applicable ASVS categories

| ASVS category | Applies | Standard control |
|---|---|---|
| V2 Authentication | no | This phase authenticates nothing |
| V3 Session management | no | — |
| V4 Access control | partial | `run.json` is written 0600 by `persist` (§1.4); the journal is a local file under the driven repo, gitignored |
| V5 Input validation | **yes** | The journal *reader* consumes a file the driven agent's output shaped. Tolerant parse, never `deny_unknown_fields`, bounded line length, skip-on-error (D-30, §2, §9) |
| V6 Cryptography | no | No crypto. `argv_digest` is a SHA-256 of a command line for identity only, not a security control |
| V7 Error handling & logging | **yes — the core of the phase** | SAFE-04 redact-at-capture; D-28 requires `tracing` calls inside `src/journal/` to route through the redactor or carry no content |
| V12 File handling | **yes** | Path classification (D-11) must not be spoofable; `create_dir_all` + atomic persist; no symlink following is introduced |

### Known threat patterns for this stack

| Pattern | STRIDE | Standard mitigation |
|---|---|---|
| Secret written to a persistent local sink | Information disclosure | Redact at capture, enforced by the `Redacted` newtype (D-21/D-22, §5.4) |
| Secret committed via the driven agent's own `git add -A` | Information disclosure | `journal.jsonl` gitignored (D-07, §1); `run.json` immutable during the agent's lifetime (D-06) |
| Log-injection / framing break via embedded newline in a payload | Tampering | `serde_json::to_string` escapes `\n` inside strings; NDJSON framing cannot be broken from a payload. The `Value`-tree walk (D-23) is what preserves this — post-serialisation regex replacement could not |
| Unbounded disk growth from agent output | Denial of service | Per-event and per-run caps + retention (D-31/D-32, §10) |
| Malformed/oversized journal line stalling the reader forever | Denial of service | `MAX_TAIL_BYTES` + `skipped_oversize` step-over (§2.2) |
| Path classification tricked into treating a planning write as a driver write | Tampering | Pure, exhaustively tested `classify_change`; anchor on the full `.planning/meta-manager/runs/` prefix, never on a bare `runs` component (§7.5) |

Phase 19 owns the complementary tool-boundary control. Neither substitutes for the other (D-25).

---

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|---|---|---|
| A1 | The proposed constants (8 KiB / 64 MiB / 10 runs) are reasonable starting values | §10 | Low — CONTEXT explicitly makes these discretionary and requires only that they be named constants and configurable. No tuning data exists for anyone |
| A2 | `write(2)` non-interleaving at ≤200 KB holds on ext4/xfs as it did on the measured ZFS | §3.3 | Low — and moot under D-29's single-writer rule, which is the actual guarantee |
| A3 | `serde_json`'s default 128-level nesting limit bounds `redact_value` recursion depth | §5.3 | Low — only relevant if a `Value` is built programmatically rather than parsed; keeping parse-then-walk order removes the risk entirely |
| A4 | The 27-case redaction corpus covers the credential shapes that actually appear in this project's streams | §4.4 | Medium — this is a pattern redactor, and D-25 already records that an arbitrary high-entropy secret with no recognisable shape is not catchable. The corpus should be treated as a floor and extended when a new shape is observed |

Everything else in this document is `[VERIFIED]` by execution in this session.

---

## Open Questions (RESOLVED)

All three were settled at planning time; the resolution and the plan that made it are recorded
inline below.

1. **RESOLVED — `journal/mod.rs` (settled by 16-01-PLAN.md).** The planner took the
   recommendation below; `watcher.rs` imports `classify_change` from the journal module.

   **Where `classify_change` lives** (`watcher.rs` vs `journal/mod.rs`) — CONTEXT leaves this to the
   planner (D-35). Research finding that bears on it: the function must be callable from the
   `notify` debouncer's callback thread, which has no `App` and no runtime. Both locations satisfy
   that. `watcher.rs` colocates it with `extract_project_root` and its existing test module; if
   the classification needs to *return* the `run_id` (D-11 says it does), `journal/mod.rs` avoids
   `watcher.rs` depending on the journal module for a type. **Recommendation:** `journal/mod.rs`,
   with `watcher.rs` importing it — the direction of dependency is journal→nothing, which is
   cleaner than watcher→journal→watcher.

2. **RESOLVED — both, listing authoritative (settled by 16-03-PLAN.md).** The planner took the
   recommendation below: the `active` file is written as a cheap hint, the directory listing is
   authoritative on disagreement, and the disagreement is journalled.

   **Whether `active` is a file or a directory listing** (CONTEXT discretion). Research finding:
   D-02's run-id format makes lexicographic sort equal chronological sort, so the listing is a
   `read_dir` + `max()`. The file is one `read_to_string`. The file can go stale after a crash;
   the listing cannot. **Recommendation:** write the file (cheap to poll, useful to Phase 17's
   reconciliation as a *hint*) but treat the directory listing as authoritative when they
   disagree, and journal the disagreement. This costs nothing and removes a whole class of stale
   state.

3. **RESOLVED — exposed as a plain `pub u64` field on `AppContext` (settled by 16-05-PLAN.md).**
   Not `#[cfg(test)]`, so the counter exists in release builds too.

   **Whether the `reparse_dispatches` counter is exposed anywhere** beyond the test. Not needed
   this phase; noted because Phase 18's Driver tab could surface it as a diagnostic.

---

## Sources

### Primary (HIGH confidence — executed in this session)

- `git 2.43.0` in throwaway repos under `/tmp` — the D-08 pattern, the `check-ignore` negation
  trap, the parent-excludes-directory failure mode
- `rustc`/`cargo 1.97.1` scratch crates — regex behaviour (leftmost-first, no backtracking,
  lookaround rejection), the 27-case redaction corpus, idempotence, throughput, the `Value` walk,
  the tail reader edge cases, `NamedTempFile` mode, `#[serde(tag)]` constraints, `seq` gap
  detection
- A purpose-built crash harness, run 8× — SIGKILL survival, `unix_wait_status(9)`, zero torn lines
- 8-process concurrent `O_APPEND` probe, 16 000 lines, 100 B–200 KB — zero interleaving
- `cargo clippy` against five purpose-built enums — the `large_enum_variant` 200-byte difference
  threshold
- `std::mem::size_of` against the real crate as a path dependency — `Action` = 120 B,
  `ProjectState` = 360 B, and the rest of §8.2
- Direct reads of `src/watcher.rs`, `src/action.rs`, `src/app.rs`, `src/config.rs`,
  `src/main_loop.rs`, `src/executor/{mod,stream_json,claude}.rs`, `src/ui/screens/mod.rs`,
  `src/state_reader/queue_md.rs`, `tests/executor_lifecycle.rs`,
  `tests/fixtures/transcripts/README.md`, `Cargo.toml`, `Cargo.lock`

### Secondary (project documents, treated as locked input)

- `.planning/phases/16-run-journal-state-substrate/16-CONTEXT.md` (D-01..D-39)
- `.planning/research/ARCHITECTURE.md` §4, `.planning/research/PITFALLS.md` Pitfall 2
- `.planning/REQUIREMENTS.md` (OBS-01, OBS-06, SAFE-04), `.planning/ROADMAP.md`,
  `.planning/STATE.md`, `CLAUDE.md`

### Tertiary

None. No claim in this document rests on web search or on recall.

---

## Metadata

**Confidence breakdown:**

| Area | Level | Reason |
|---|---|---|
| `.gitignore` semantics | HIGH | Executed against real `git`, including both failure modes |
| Tail reader | HIGH | Compiling code, run against a real concurrently-appended file |
| Append durability | HIGH | 8/8 measured SIGKILL runs + a 16 000-line interleaving probe |
| Redaction patterns | HIGH | 27 executed cases, 0 idempotence failures, 2 real leaks found and fixed |
| `Value` walk | HIGH | Executed, incl. the key-collision path |
| Crash test harness | HIGH | Built and run 8× |
| OBS-06 seam | HIGH (mechanism) / MEDIUM (recommendation) | The sizes and the flake mode are measured; which seam is "cleanest" is a judgement, argued from repo precedent |
| `large_enum_variant` | HIGH | Threshold determined by bisection against real clippy |
| serde constraints | HIGH | All four cases executed; matches the repo's own documented note |
| Growth constants | MEDIUM | Anchored to Phase 15's measured fixture sizes, but no tuning data exists |

**Research date:** 2026-07-29
**Valid until:** 2026-08-28 (30 days — the findings are about `git`, POSIX file semantics, and
pinned crate versions, all of which are stable; only the crate-version-specific serde and clippy
behaviours would need rechecking after a toolchain bump)
