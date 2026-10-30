---
phase: 19-gitsafe-git-blast-radius-envelope
reviewed: 2026-08-28T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/envelope/advisory.rs
  - tests/driver_lock.rs
  - tests/envelope_advisory.rs
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 19: Code Review Report

**Reviewed:** 2026-08-28T00:00:00Z
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

This is a `--gaps-only` re-review of the two files changed to close phase 19's two
MINOR-severity gaps, plus their test coverage. Both files are heavily
self-documenting about their own design tradeoffs, and most of the choices
called out in the scope note (indentation escapes, byte-unchanged honesty
statement, the 215-token cap, `TempDir`-backed capture files, per-test
`ALIAS_*` constants) hold up on inspection and are not repeated here.

No BLOCKER-tier defects were found: the redaction choke point in
`ProtectionState::unknown` is genuinely the single path every `Unknown`
reason travels through, the `Unknown`-never-renders-as-`Protected` invariant
holds across every code path traced, and the two-process (`driver_lock.rs`)
lock tests exercise real contention rather than in-process fakes. The
findings below are three correctness/robustness gaps worth closing and one
minor test-quality note — none change the module's core safety property, but
each is a real gap between what the code (or its own doc comments) claims and
what it does.

## Warnings

### WR-01: The remote-write guard test can be defeated by a lowercased HTTP verb

**File:** `tests/envelope_advisory.rs:373-381`
**Issue:** `the_module_has_no_write_path_at_all` enforces "this module has no
write path" by scanning the raw source for the literal, all-uppercase
substrings `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`. `String::contains` is
case-sensitive, so a future edit that added `.args(["api", "-X", "post", ...])`
or `--method Post` (both accepted by `gh api`, which does not require the
method name in any particular case) would add a genuine write path while this
control stays green. The module's own doc (`advisory.rs:33-35`) states the
entire point of this test is that the write-path property is "a build failure
rather than a promise" — a check that only catches one letter-casing of the
verb it is guarding against does not deliver that promise. Today's source
contains none of these substrings in any case, so the control is not
currently vacuous, but it is one careless edit away from becoming so with no
signal to the editor.
**Fix:**
```rust
for verb in ["POST", "PUT", "PATCH", "DELETE"] {
    assert!(
        !source.to_ascii_uppercase().contains(verb),
        "src/envelope/advisory.rs mentions {verb} (in some casing): applying or \
         removing protection needs an Administration scope the run credential \
         withholds by design (D-18/D-26), so this module has no write path \
         even in its prose"
    );
}
```

### WR-02: A 404 from the branch-protection endpoint is treated as a definitive "Unprotected" verdict, which can also mean "permission denied"

**File:** `src/envelope/advisory.rs:417-433`, `src/envelope/advisory.rs:550-555`
**Issue:** The third probe step folds any `ClientAnswer::Absent` from
`repos/{slug}/branches/{default_branch}/protection` straight into
`ProtectionState::Unprotected`, via `says_absent`'s check for
`"branch not protected"`, `"http 404"`, or `"not found"`. GitHub's branch-protection endpoint is documented to require
elevated (admin/push) access to view, and a caller lacking that access on an
otherwise-readable repository is not universally guaranteed to receive a
`403` rather than a `404` for this specific endpoint — the two failure modes
("not protected" and "not visible to this credential") are not always
distinguishable from the HTTP status/text alone. The function's own doc
comment at `advisory.rs:331-334` promises "a permission failure ... yields
`Unknown` carrying that specific reason," but the current dispatch cannot
keep that promise if a permission failure surfaces through this endpoint
shaped like the genuine-absence case. The failure direction is the safer one
(a false `Unprotected` triggers an extra warning rather than a false sense of
safety), but it is still a false claim about the remote's actual state, which
is precisely what this module exists to avoid emitting.
**Fix:** At minimum, narrow `says_absent`'s branch-protection-specific check
to the phrase GitHub actually returns for "not protected" (`"branch not
protected"`) rather than falling back to a bare `"http 404"` / `"not found"`
match for this call site specifically, so a generic 404 (which could be a
permission-shaped 404) routes to `Unknown` via `Failed` instead of
`Unprotected`:
```rust
match run_client(env, project_root, &[
    "api",
    &format!("repos/{slug}/branches/{default_branch}/protection"),
]) {
    ClientAnswer::Body(_) => ProtectionState::Protected,
    ClientAnswer::Absent(detail) if detail.to_ascii_lowercase().contains("branch not protected") => {
        ProtectionState::Unprotected
    }
    ClientAnswer::Absent(detail) => ProtectionState::unknown(format!(
        "the branch protection query for {default_branch} answered 404 without the \
         specific \"branch not protected\" reason ({detail}), which this credential's \
         access to the endpoint cannot be told apart from"
    )),
    ClientAnswer::Failed(reason) => ProtectionState::unknown(format!(
        "{ruleset_note} was found, and the branch protection query for {default_branch} \
         could not be completed: {reason}"
    )),
}
```

### WR-03: Cumulative probe latency can reach ~3x the stated per-call budget, understating the module's own worst-case claim

**File:** `src/envelope/advisory.rs:341-434`, module doc at `src/envelope/advisory.rs:43-51`
**Issue:** `probe_protection` performs up to three sequential `run_client`
calls (rulesets, repository/default-branch, branch protection), each
independently bounded by `PROBE_BUDGET_SECS = 10`. The module doc frames
`PROBE_BUDGET_SECS` as the mitigation for a reproduced 180-240 second
`PreToolUse` hang and states "the worst case at run start is a delay measured
in seconds followed by `Unknown`, never a run that never begins." Under a
degraded network (the exact scenario the doc cites — "a proxy swallowed a
connection"), each of the three sequential calls can independently exhaust
its own 10-second budget before the next one starts, so the actual worst
case is close to 30 seconds, not the low single-digit "seconds" the framing
implies. This does not reproduce the original hang (which was unbounded), but
the doc comment overstates the tightness of the bound it is describing, and a
future maintainer tuning `PROBE_BUDGET_SECS` down (say, to keep the
worst case near the original claim) would be working from a number that is
already 3x off.
**Fix:** Either divide one shared deadline across all three calls (a single
`Instant` computed once in `probe_protection` and threaded through
`run_client`), or update the doc comment to state the true multiplier
explicitly, e.g.: "bounded by up to three times its own budget, since the
probe makes up to three sequential reads — worst case on the order of tens of
seconds, never unbounded."

## Info

### IN-01: `first_line` can surface a non-informative line ahead of the client's actual complaint

**File:** `src/envelope/advisory.rs:537-543`
**Issue:** `first_line` returns the first non-empty, trimmed line of stderr.
Some CLI tools (including `gh` itself, e.g. for deprecation or update
notices) can print an advisory line to stderr ahead of the actual error on a
failing invocation. If that ever happens for the `gh` invocations here, the
`Unknown` reason recorded would be the advisory notice rather than the actual
failure cause, which works against this module's stated goal of letting "a
reader ... tell 'you are not authenticated' from 'this host is unreachable'"
(`advisory.rs:333-334`). This is speculative for the current three read-only
`gh api` calls (which are not known to emit such banners), so it is filed as
informational rather than a warning.
**Fix:** If this is ever observed in practice, prefer the last non-empty line
(closer to how `git`/`gh` typically place the terminal error message) or
filter out lines matching a known notice pattern before selecting the first
one.

---

_Reviewed: 2026-08-28T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
