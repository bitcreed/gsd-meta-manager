---
created: 2026-10-24T00:00:00.000Z
title: Session detector binds the successor of -r/--resume although claude treats the value as optional
area: session_detector
severity: medium
files:
  - src/session_detector.rs:728-734
  - src/session_detector.rs:425
  - src/session_detector.rs:469
---

## Problem

`claude` 2.1.250 documents `-r, --resume [value]`: the value is **optional**. `--session-id <uuid>` takes a required
value. `session_id_in_cmdline` does not model that difference. At `src/session_detector.rs:728-734` the branch
`element == RESUME_OPTION_NAME || element == RESUME_OPTION_SHORT_NAME` (`b"--resume"` at `:425`, `b"-r"` at `:469`)
reads `args.get(index + 1)` unconditionally and steps over it, so for `claude -r --model x` the detector reports
`--model` as a session id and never sees `--model` as an option.

Measured on 2026-08-28 for 21-VERIFICATION CR-01: `claude --resume --version < /dev/null` and
`claude -r --version < /dev/null` both print the version and exit 0, because `--version` was parsed as a new option
and not bound as the value. By contrast `claude --session-id --version` prints `Error: Invalid session ID. Must be a
valid UUID.` and exits 1, so `--session-id` does bind a required successor.

The wrong model is also pinned: `the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped` in
`src/ui/screens/detail.rs` asserts `Some("-h")` for `["claude", "--resume", "-h"]` and cites the 2.1.250 measurement
that the re-run contradicts, and the doc paragraph at `session_detector.rs:468-482` states the rule as fact for both
options.

This was Phase 21's CR-01, which Phase 31 did not close as superseded: the parser and the detail screen survive the
goal-layer retirement. It is also a backlog item, not a resumption of Phase 21's gap loop.

## Solution direction

For the `--resume` / `-r` branch only, reject an option-shaped successor (starts with `-` and is not the single byte
`-`) and do not step over it; keep the `--session-id` branch unconditional. Rewrite the pinning test to assert `None`
for the `-h`, `--resume` and `--version` successors under `--resume`/`-r`, keep a positive arm for `--session-id`, and
record the re-measured probe transcripts beside it. Exclude option-shaped values from the `split` and `short-split`
arms of the generated property (or give those forms their own oracle branch), so the certificate stops agreeing with
the wrong model by construction. Re-run the probes on any `claude` upgrade, since the arity is a dependency behaviour.

## Resolution

Closed 2026-10-28 by quick 261028-n0v, fix commit `3103f5e` (docs corrections in the following commit).

- `session_id_in_cmdline` refuses an option-shaped successor (first byte `-`, longer than one byte) of a bare
  `--resume`/`-r` and does not step over it, so it is re-examined as an element. `--session-id` binds unconditionally.
- Probes re-measured at `claude` 2.1.312: `--resume --version`, `-r --version` and `--resume -h` print version/usage
  with exit 0; `--session-id --version` exits 1 with `Invalid session ID`. Transcripts are recorded verbatim in
  `tests::an_option_shaped_successor_is_bound_by_session_id_and_refused_by_resume`.
- The generated round-trip property gained a `WireForm.optional_successor` oracle branch; the enumerated SPLIT arm is
  partitioned 20 round-trip / 8 option-shaped reading back `None`.
- Standing requirement: re-run the probes on any `claude` upgrade; the arity is a dependency behaviour.
- Disclosed residuals `[inferred — cleanup 2026-10-28]`: (I2) the single byte `-` stays a value under `--resume`, not
  probeable at 2.1.312; (I3) a negative-number-shaped successor (`-r -5`) counts as option-shaped, and if `claude`
  binds it the direction is under-detection, silent.
