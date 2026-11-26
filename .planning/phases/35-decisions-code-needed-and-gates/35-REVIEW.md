---
phase: 35-decisions-code-needed-and-gates
reviewed: 2026-11-14T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - CLAUDE.md
  - README.md
  - src/agent_skill.md
  - src/inbox.rs
  - src/inbox_cli.rs
  - src/oneoff/todos.rs
  - src/state_reader/owner_items/context.rs
  - src/state_reader/owner_items/debug.rs
  - src/state_reader/owner_items/deferred.rs
  - src/state_reader/owner_items/frontmatter.rs
  - src/state_reader/owner_items/gate.rs
  - src/state_reader/owner_items/handoff.rs
  - src/state_reader/owner_items/mod.rs
  - src/state_reader/owner_items/rules.rs
  - src/state_reader/owner_items/scan.rs
  - src/state_reader/owner_items/todos.rs
  - src/state_reader/owner_items/uat.rs
  - src/state_reader/owner_items/verification.rs
  - src/status_cli.rs
  - tests/inbox_cli.rs
  - tests/owner_audit_conformance.rs
  - tests/owner_digest.rs
  - tests/owner_open_conformance.rs
  - tests/status_cli.rs
  - tests/fixtures/owner_golden/inbox.txt
  - tests/fixtures/owner_golden/inbox-include-all.txt
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
  necessity: 1
status: issues_found
---

# Phase 35: Code Review Report

**Reviewed:** 2026-11-14
**Depth:** standard
**Files Reviewed:** 26 source/test files, plus the 88 fixture files under `tests/fixtures/owner/{checkpoint-phase,closes-after-milestone,context-questions,debug-awaiting,debug-sessions,handoff-paused,handoff-workstreams,todos-age,verification-gaps-empty,verification-human-mapping}/` and the two golden files
**Status:** issues_found

## Summary

Reviewed the phase diff (`a30adb18..HEAD`) at standard depth. `cargo clippy --all-targets -- -D warnings` is clean and `owner_digest`, `inbox_cli` and `status_cli` pass. The oracle-backed conformance tests were not run (no gsd-core oracle in this session).

The new readers (debug, context, handoff and checkpoint, todos) follow the established read path: containment check, non-blocking open, regular-file check, 1 MiB cap. They are fail-safe by design. I found no injection, traversal or panic path. Untrusted project text reaches JSON and plain output only through `terminal_capped`. The `closes after` line is built from project-derived phase strings and escaped as a whole. `is_command_token` and `is_workstream_name` guard every command string.

No project-private text is in the fixtures. Fixture names, aliases and phase names are synthetic (alpha, bravo, `six`, `old`, and so on). The one grep hit for "mailbot" is in a pre-existing, out-of-scope test file. The fixtures agree with the code and tests that use them. The golden's `307 d` ages come from the fixed `now()` in `tests/owner_digest.rs`, not the wall clock.

The concerns are semantic. The answered-question heuristic can silently hide real questions. An unreadable CONTEXT file is not counted, which breaks the module's stated fail-safe rule. The `closes after:` line is cut by the 200-character cap.

## Warnings

### WR-01: `is_answered` hides open questions that merely contain or start with RESOLVED, ANSWERED or CLOSED

**File:** `src/state_reader/owner_items/rules.rs:690-770`
**Issue:** A dropped question never reaches the owner. The module header says "Only an explicit marker drops a question", but three matcher paths drop real questions:

1. `starts_with_answered_word` treats any non-alphanumeric, non-underscore character after the word as a word boundary, and `-` counts as one. `- Closed-loop control or open-loop?` reads as answered because the word is `Closed` and the next character is `-`. The test pins only the inverse case, `- the CLOSED-loop test?`, which starts with `the`. A line starting `Resolved-by-default behavior?` has the same flaw.
2. The bracket-tag scan matches `(` or `[` anywhere in the line, followed by one of the words. `- How should we export tickets (closed vs resolved)?` and `- Should [answered] polls be archived?` are dropped.
3. `QUESTION_LABEL` accepts any short alphanumeric token containing a digit as an ID label. `- v2 closed beta: who is invited?` drops the `v2 ` label and then matches `closed`.

In all three cases the whole question disappears from `decide`, from `closes_after.decide` and from the listing. If the file's only questions are hit, the file contributes no group at all.

**Fix:** Tighten the boundary so a marker word is "answered" only when followed by end of text, whitespace, or one of `:.,;—–)]*`. Do not accept `-`, so `Closed-loop` stays open. For the bracket path, require the tag to be the whole bracket content or to start the bracket, and to be closed by `)`, `]` or end of line, for example `\(\s*(resolved|answered|closed)\b[^)]*\)`. Add negative tests for `Closed-loop ...?`, `(closed vs resolved)` and `v2 closed beta`.

```rust
let whole = text[end..]
    .chars()
    .next()
    .is_none_or(|c| c.is_whitespace() || ":.,;—–)]*".contains(c));
```

### WR-02: An unreadable CONTEXT file is listed but counted nowhere (summary `decide`, `closes_after.decide`)

**File:** `src/state_reader/owner_items/rules.rs:1066-1078` (counting), `src/state_reader/owner_items/gate.rs:57-65`; pinned by the test at `rules.rs:2147-2155`
**Issue:** Fail-safe rule 4 in `owner_items/mod.rs` says a file that cannot be read is "reported as unreadable checks, never as clean". For Fix-lane groups this holds: `unreadable` feeds `fix_phases`, `fix_debug` and `closes_after.fix`. For a CONTEXT file that is too large or hits an I/O error, the group is a Decide group with `items == []` and `counts.unreadable == 1`. The digest adds `group.items.len()` for every Decide kind except Handoff and Checkpoint, so it adds 0 here. `gate.rs` also adds only `group.items.len()` for Decide.

The result is a project with an unreadable CONTEXT file in an open phase. It shows `decide 0` in the header and `closes after: no owner work (ph N)`, and `status --json` reports `owner.decide: 0`. The `unreadable file X` row appears only in the listing, and only under the default lane filter. `rules.rs:2155` asserts `decide == 0` deliberately, so the inconsistency was accepted rather than missed. It is still a "looks clean" signal for exactly the case the fail-safe rule covers, and the handoff and checkpoint cases do count 1.

**Fix:** Count an unreadable Decide group as 1 in both places, as handoffs and checkpoints do.

```rust
digest.decide += if matches!(group.kind, Kind::Handoff | Kind::Checkpoint) {
    1
} else {
    group.items.len() + group.counts.unreadable
};
```

Apply the same `+ group.counts.unreadable` in the `Lane::Decide` arm of `gate.rs`, and update the `rules.rs:2155` assertion.

### WR-03: The `closes after:` line is silently truncated by the 200-character cap, dropping the lane parts that matter most

**File:** `src/inbox.rs:517-523` (`push_closes_after`)
**Issue:** The whole line (alias, version, and every lane with its phase list) is passed through `terminal_capped`, which cuts at `MAX_TEXT_CHARS` (200). The parts run in the order test, decide, unblock, fix, `not built`. Each lists its phases, so an eight-phase milestone with work in several lanes and a long alias can exceed 200 characters. The cut removes the tail, which is the `fix` and `not built` parts. The golden's longest line is about 90 characters, so the tests never hit the cap. The failure is silent: the line looks complete and the user will conclude there is no fix work. The JSON form is unaffected.
**Fix:** Cap only the untrusted pieces (alias, version) and leave the generated structure uncapped, or cap each phase list separately.

```rust
let line = format!(
    "  {}{version} closes after: {}",
    terminal_capped(&alias_of(entry)),
    parts.iter().map(|p| terminal_capped(p)).collect::<Vec<_>>().join(" · ")
);
```

Escaping still happens per piece. Add a test with about 12 open phases in several lanes.

## Info

### IN-01: Stale and misleading doc comment on `OPEN_CACHE`

**File:** `src/state_reader/owner_items/mod.rs:229-231`
**Issue:** The comment says "Parsed debug sessions (and, later, context files)". Context files already live in the separate `CONTEXT_CACHE` directly above it. The `CONTEXT_CACHE` comment also says "a second map because the parsed value differs", which contradicts "later".
**Fix:** Reword `OPEN_CACHE` as "Parsed debug sessions by canonical path".

### IN-02: Generality without instantiation, `OpenKind` as a cache key discriminator

**File:** `src/state_reader/owner_items/mod.rs:219-225`
**Category:** necessity
**Issue:** `OpenKind { Debug, Context }` is the `K` of `ParseCache<K, V>`, which exists to tell entries of different kinds apart under one path. Each static cache holds a single kind. `OPEN_CACHE` is only ever called with `OpenKind::Debug` (`debug.rs:136`, `debug.rs:151`) and `CONTEXT_CACHE` only with `OpenKind::Context` (`context.rs:309`, `context.rs:313`), and the two maps are separate statics. The discriminator never discriminates. The generic `K` is justified only for `AuditKind`, whose three kinds share `PARSE_CACHE`. This is judgment, not provable removal, since a future shared cache would need it.
**Fix:** Either key these caches by `()` or a unit type, or drop `OpenKind` and keep `ParseCache<K, V>` generic for `AuditKind` alone.

### IN-03: Docs drift for item ids, `acknowledged_files`, and the Old debt fold

**File:** `README.md` (the `items[].id` row, the `summary.acknowledged_files` and `hidden.acknowledged_files` rows), `src/inbox.rs:633-640`
**Issue:**
- `items[].id` is still documented as "`<file name>#<test>`". Phase 35 adds `#q<k>`, `#a<k>`, `#b<k>` and `#1`.
- `acknowledged_files` is still documented as "UAT files". `debug_groups` and `context_groups` now add acknowledged debug sessions and CONTEXT files to the same counter.
- The text fold says "Old debt: N checks", but `archived.decide` now adds archived open questions, which are not checks.

**Fix:** Update the README rows and reword the fold, for example "N items".

### IN-04: `hidden.todos_unreadable` is JSON-only, so the plain checklist still shows no sign of an unreadable todo folder

**File:** `src/inbox.rs:652-662` (the Triage fold), `src/state_reader/owner_items/todos.rs:55-63`
**Issue:** An unreadable pending folder gives `count 0` and `unreadable true`. The plain `Triage: N pending todos` fold is suppressed when the total is 0, so a terminal user sees no difference from a clean folder. Only `--json` consumers see the flag. The README states "0 is not a clean list" for JSON but the text renderer never says so.
**Fix:** Add a fold such as `Triage: pending todos unreadable in N projects` when any `digest.hidden.todos_unreadable` is true.

### IN-05: A phase with owner work but no plan is reported as `not built` and contributes nothing to the gate

**File:** `src/state_reader/owner_items/gate.rs:33-39`; visible in `tests/fixtures/owner_golden/inbox.txt` lines 5 and 7
**Issue:** `closes_after` treats `plan_count == 0` as "not built" and skips the phase. That follows D-12, so it is documented. But the golden shows `gaps closes after: not built ph 01` directly above a FIX group for phase 01 with `10 failed checks, 14 gaps`. The fixtures lack PLAN files, so this is a fixture artifact, but the code does not guard the case. Any real phase that has a UAT or VERIFICATION file with open items and no PLAN file, such as plans cleaned up by hand, would show open work in the listing and "not built, no work" in the gate.
**Fix:** In `gate.rs`, treat a phase with any current, unacknowledged owner group as open even when `plan_count == 0`. Alternatively add a PLAN file to the `gaps` and `ack` golden fixtures so the golden does not show the contradiction.

---

_Reviewed: 2026-11-14_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
