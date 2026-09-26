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
