# Phase 25 — Deferred Items (out of scope, logged by executors)

## From 25-01

- **`.planning/WINDOWS.md` is internally inconsistent.** `gsd-tools windows append` refuses to write:
  "Ledger table ... disagrees with the fenced JSON entries (the sole source of truth) for row id(s): 19".
  This predates 25-01. The 25-01 intentional stub (`registered_adapters()` empty until 25-02) could not be
  recorded there because of it; it is listed in 25-01-SUMMARY.md § Known Stubs instead.
- **`cargo clippy --all-targets` has pre-existing warnings** in `src/browser.rs`, `src/project_creator.rs`,
  `tests/envelope_config_resolution.rs`, `tests/envelope_wrapper_class.rs`, `tests/envelope_control_carrier.rs`
  and `tests/envelope_carrier_reach.rs` (bool_assert_comparison, cmp_owned, unused_mut, etc.). None are in
  files 25-01 touched; `cargo clippy -- -D warnings` (lib + bin) is clean.
- **The repository is not `rustfmt`-clean** (`cargo fmt --check` reports most files). 25-01 formatted only
  its new files (`src/agents/**`, `tests/agents_scan.rs`), not `git_ops.rs` or `lib.rs`.

## Resolution (re-verification, 2026-10-28, HEAD 54a73bf)

All three items are **closed**. [inferred — cleanup 2026-10-28]

| Item | Status | Evidence |
|------|--------|----------|
| `.planning/WINDOWS.md` table/JSON drift (row 19) | closed | Row 19's hand-edited table cell was restored from the fenced JSON in `ef78a82` ("close WINDOWS.md ledger — 13 obsolete, 2 fixed, 3 waived (0 open)"). `gsd-tools windows status --raw` → `open_count: 0`, `total_count: 19`. Write-path proof: on a scratch copy of the current ledger, `gsd-tools windows append` (the verb that refused before, via the `WINDOWS_LEDGER_TABLE_DRIFT` check in `broken-windows.cjs`) succeeded (`ok: true`, total 19→20); the real ledger was not touched. |
| `cargo clippy --all-targets` warnings | closed | `rtk proxy cargo clippy --all-targets -- -D warnings` at `54a73bf` → `Finished`, exit 0 (cleared by quick batch 261028-n0u, `ba50776`). |
| repository not `rustfmt`-clean | closed | `rtk proxy cargo fmt --check` at `54a73bf` → exit 0, no diff; last offender fixed in `f393df5` (`src/ui/screens/detail.rs`). |
