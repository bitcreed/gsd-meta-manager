---
quick_id: 260722-emn
plan: 10
item: I
wave: 5
status: incomplete
blocker: "clippy -D warnings fails on pre-existing out-of-scope lint (src/action.rs large_enum_variant); orchestrator decides whether it blocks the v1.6.0 release"
files_modified: [Cargo.toml, Cargo.lock]
commit: af92a8d
---

# Quick 260722-emn Plan 10: Release Prep for v1.6.0 Summary

Bumped the crate version to `1.6.0` and refreshed `Cargo.lock`; the integrated
tree builds clean and all 210 tests pass, but the `clippy -- -D warnings`
integration gate fails on one pre-existing, out-of-scope lint that this plan is
forbidden to touch.

## What Was Done

- **`Cargo.toml`**: `version = "1.5.0"` → `version = "1.6.0"` (only the `[package]`
  version line; dependency version requirements left untouched).
- **`Cargo.lock`**: refreshed via `cargo update` (many patch/minor bumps within
  existing semver constraints — tokio 1.52.3→1.53.1, serde_json 1.0.149→1.0.151,
  time 0.3.47→0.3.54, uuid 1.23.1→1.24.0, etc.).
- Committed atomically as `af92a8d` — `chore(260722-emn): bump version to 1.6.0 and refresh Cargo.lock`.
- Scope respected: only `Cargo.toml` and `Cargo.lock` were modified. No source
  files edited.

## Verification Gate (integrated tree)

| Check | Result |
| ----- | ------ |
| `cargo build` | PASS (117 crates, clean finish) |
| `cargo test` | PASS — 210 passed, 5 suites, 0 failed |
| `cargo clippy -- -D warnings` | **FAIL — exit 101, 1 error** |

**Clippy failure (verbatim):**

```
error: large size difference between variants
  --> src/action.rs:6:1
   | the largest variant contains at least 384 bytes
error: could not compile `gsd-meta-manager` (lib) due to 1 previous error
```

This is exactly the **known, pre-existing `large_enum_variant` lint in
`src/action.rs`** called out in the task constraints. It lives in a source file
outside this plan's file scope (`Cargo.toml`/`Cargo.lock` only) and outside the
wave-4 UI ownership. Per the constraints I did **not** fix it — recording the
exact failure here for the orchestrator to decide (e.g. a follow-up fix in
`src/action.rs`, boxing the large variants, or an explicit `#[allow]`). It is
not introduced by the version bump or lockfile refresh (only app-owned enum
sizing drives it), and build + tests are unaffected.

## Deferred Dependency Bumps (direct deps behind latest breaking version)

Held back by current semver constraints — deferred per CLAUDE.md's major-bump
policy; itemized here for the release commit body:

| Dependency | Resolved | Latest available | Notes |
| ---------- | -------- | ---------------- | ----- |
| `notify-debouncer-full` | 0.5.0 | 0.7.0 | direct; semver-breaking 0.x jump |
| `serde_yml` | 0.0.12 | 0.0.13 | direct; 0.0.x treated as breaking |

Explicitly **not** behind (at latest stable): `notify` stays 8.x (9.0 is still
rc/pre-release per CLAUDE.md), `ratatui` 0.30, `tui-textarea` 0.7, `dirs` 6.

Transitive-only gaps (informational, not directly declared): `generic-array`
0.14.7→0.14.9, `unicode-width` 0.2.0→0.2.2.

## Deviations from Plan

None on the scoped work — version bump and lockfile refresh executed exactly as
written. The plan's Task 2 anticipated the clippy-gate case ("otherwise STOP and
report which file/plan needs the fix; do not edit source files here"), which is
the path taken: reported, not fixed.

## Self-Check

- `Cargo.toml` reads `version = "1.6.0"` — verified.
- Commit `af92a8d` exists in git log — verified.
- Only `Cargo.toml` / `Cargo.lock` changed in the commit — verified.

## Self-Check: PASSED (for in-scope deliverable)

Status is `incomplete` solely because the integration clippy gate did not pass
clean — blocked by the pre-existing out-of-scope `src/action.rs` lint. The
version bump and lockfile refresh themselves are complete and clean.
