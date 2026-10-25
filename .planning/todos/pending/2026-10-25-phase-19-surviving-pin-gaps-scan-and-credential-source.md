---
created: 2026-10-25T00:00:00.000Z
title: Two unpinned properties in surviving envelope code (scan Absent never permits, CredentialSource drift)
area: envelope
severity: minor
files:
  - src/envelope/scan.rs:188
  - src/config.rs:440
  - src/config.rs:904
---

## Problem

Phase 31 closed the Phase 19 threat register as superseded except for two `low` ids that sit in code which survives the
guard's retirement. Both are missing-pin defects, so the push step running outside the sandbox does not remove them.

- **T-19-72** (`src/envelope/scan.rs:188`). `ScanReport::is_clean` consults only `findings` and `root_unreadable`.
  The property that an `ExternalScanner::Absent` never permits a push is true by construction but no test pins it, so
  an edit that makes `is_clean()` consult `external` turns nothing red.
- **T-19-73** (`src/config.rs:440`). `CredentialSource` is `Env { var }` or `Command { argv }`. The round-trip test at
  `src/config.rs:904` pins only the two good forms, so a third variant carrying a literal token would compile and pass
  the suite. The credential is now inert config (the askpass reader went with `cred.rs`), which lowers the harm but not
  the drift risk.

## Solution direction

Add a scan test that builds a clean report with `external = ExternalScanner::Absent` and asserts `is_clean()` is
unchanged by it. Add a config test asserting `serde_json::from_str::<CredentialSource>(r#"{"source":"literal","token":"x"}"#)`
is an `Err`. Both are a few lines each. Source: `19-SECURITY.md` register rows T-19-72 and T-19-73, closure table in
`.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` (`## Phase 31 closure (2026-10-25)`).
