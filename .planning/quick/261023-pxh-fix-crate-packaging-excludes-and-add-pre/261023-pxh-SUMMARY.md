---
phase: quick-261023-pxh
plan: 01
subsystem: release-tooling
tags: [cargo-package, crates-io, pre-tag-check, release]
requires: []
provides:
  - root-anchored Cargo.toml package excludes
  - pre-tag-check.sh gate 6 (cargo package --locked --allow-dirty)
affects: [Cargo.toml, scripts/pre-tag-check.sh, CLAUDE.md, .github/workflows/release.yml]
tech-stack:
  added: []
  patterns: [gitignore-style root anchoring, advisory-never-alters-verdict]
key-files:
  created: []
  modified:
    - Cargo.toml
    - scripts/pre-tag-check.sh
    - CLAUDE.md
    - .github/workflows/release.yml
decisions:
  - "Anchor the three excludes with a leading / rather than switching to an include allow-list [INFERRED — audit]"
  - "Gate 6 always passes --allow-dirty [INFERRED — audit]"
  - "Dirty-tree advisory after gate 6, never alters the verdict [INFERRED — audit]"
  - "No --offline and no offline fallback; CARGO_NET_OFFLINE=true documented as the bare-run escape hatch [INFERRED — audit]"
  - "Leave target/package in place after gate 6 [INFERRED — audit]"
  - "release.yml change is comment-only [INFERRED — audit]"
  - "Keep the two pretag volumes and the image after the run [INFERRED — audit]"
metrics:
  duration: ~45 min (container run 837 s)
  completed: 2026-10-23
status: complete
actuals:
  tokens: 2500
  tasks: 3
  commits: 3
plan_head_before: 6c805addc6667c32561f3475ea793336bf3217d7
plan_head_after: eccf0feca3e3c9ade021e31a4c173706d51a689c
---

# Phase quick-261023-pxh Plan 01: Fix crate packaging excludes and add pre-tag gate 6 Summary

Root-anchored `exclude` entries in Cargo.toml restore the three dropped `include_str!` assets to the crate tarball, and `scripts/pre-tag-check.sh` gains gate 6 (`cargo package --locked --allow-dirty`) so the publish job's tarball-verify step is now rehearsed before tagging. A cold `./scripts/pre-tag-check.sh --container v1.15.0` passes all six gates.

## Commits

| Task | Commit | Subject |
| ---- | ------ | ------- |
| 1 | 699ae75 | fix(quick-261023-pxh): anchor crate excludes so src/**/assets ship in the package |
| 1 | 59af61e | feat(quick-261023-pxh): add pre-tag-check gate 6 — cargo package --locked |
| 2 | eccf0fe | docs(quick-261023-pxh): gate 6 dirty-tree advisory, offline policy and six-gate docs |

Task 3 made no code change (the live run passed first time), so it has no commit.

## Package file counts

- Before (base 6c805ad, planner-measured): 323 files, missing the three asset files plus nine fixture files.
- After (host, dirty tree, `--allow-dirty`): 335 files, 13.0 MiB (3.4 MiB compressed). `Packaged 335 files, 13.0MiB (3.4MiB compressed)`.
- After (container, clean tree): 336 files. The one extra file is the `.cargo_vcs_info.json` that cargo only generates for a clean tree; it is not a tracked file.
- Every `git ls-files src tests` path is in the list; no path starts with `assets/` or `.planning/`; no root `CLAUDE.md`.
- Include-target cross-check: a scripted resolve of every `include_str!`/`include_bytes!` literal gave 59 raw matches, one of which was the literal `CARGO_MANIFEST_DIR` inside an `env!` call, not a file. That leaves the 58 real targets the planner counted, all packaged.
- `cargo package --locked --allow-dirty` on the host exited 0 with Packaging, Verifying and `Packaged 335 files` lines (at base it died with `failed to verify package tarball`).

## Task 3 evidence (cold container run)

Command: `./scripts/pre-tag-check.sh --container v1.15.0`. Exit code 0, wall-clock 837 s.

Before: `docker system df -v` showed no gsdmm-pretag-target volume, no gsdmm-pretag-cargo-home volume and no gsdmm-pre-tag-check image (cold run, as the planner expected).

Inner reconciliation banner: `MATCH: this tree can validate the version witness test the way CI will.`

Inner summary:

```
  gate 1  tag / Cargo.toml version   PASS
  gate 2  MSRV check                 PASS
  gate 3  cargo build --release      PASS
  gate 4  cargo test                 PASS
  gate 5  cargo clippy -D warnings   PASS
  gate 6  cargo package --locked     PASS
```

Gate 6 streamed `Packaged 336 files, 13.0MiB (3.4MiB compressed)` then `Verifying gsd-meta-manager v1.15.0 (/work)`. Gate 6 carries no dirty-tree note. The final banner read `inner run exited 0`. `git status --porcelain -- Cargo.toml Cargo.lock` was empty afterwards.

Post-run cache sizes:

```
post-run cache size: gsdmm-pretag-target  15G (14124807842 bytes)
post-run cache size: gsdmm-pretag-cargo-home  198M (197982074 bytes)
```

After: `docker system df -v` lists gsdmm-pre-tag-check:latest (2.04 GB), gsdmm-pretag-target (14.12 GB) and gsdmm-pretag-cargo-home (198 MB).

Task 3's automated verify block was run against the log (stored in the scratchpad rather than `${TMPDIR:-/tmp}`) and printed `VERIFY-OK`.

## Inferred decisions [INFERRED — audit]

1. **Exclude anchoring scope and shipped fixtures.** All three entries are anchored (`/.planning/`, `/CLAUDE.md`, `/assets/`). Anchoring also ships the nine `tests/fixtures/injection-corpus/` files (eight under `.planning/`, plus that directory's CLAUDE.md). They are synthetic fixtures, already public in the GitHub repo, and the crate's own tests read them (threat T-pxh-02, accepted).
2. **No `include = [...]` allow-list.** It would change what ships beyond this bug and need upkeep for every new top-level directory.
3. **Always `--allow-dirty`.** Release step 3 runs before the step-5 release commit, so Cargo.toml and Cargo.lock are uncommitted by design; the container bind-mounts the same working tree, so one flag serves both modes.
4. **Dirty-tree advisory.** After gate 6, `git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock` is printed with a warning, and on PASS the summary note counts the paths. It never changes the verdict. If git fails, it prints one line saying the state is unknown.
5. **No `--offline`, no fallback.** `cargo package` contacts the index even on a warm cache and the publish job is online. `CARGO_NET_OFFLINE=true` is documented in the header and `--help` as the bare-run escape hatch.
6. **`target/package` left in place** after gate 6 (cargo overwrites it next run; the .crate is useful evidence; in container mode it sits in the capped volume).
7. **release.yml is comment-only:** three comment lines above the Publish step cross-reference gate 6.
8. **Storage outcome.** Both volumes are below the 20 GB default cap (14.12 GB and 198 MB), so the volumes and image were left in place and `--clean` was not run. The operator's next release benefits from the warm cache. Other volumes (gsdmm-target, gsdmm-cargo) were not touched.
9. **Commit trailer.** The executor initially wrote a Sonnet trailer; the orchestrator reworded all three task commits (pre-push, unpublished) to the required `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and updated the shas in this summary.

## Deviations from Plan

None - plan executed exactly as written, apart from the trailer note above (decision 9) and the container log being written to the scratchpad directory instead of `${TMPDIR:-/tmp}`.

## Known Stubs

None.

## Threat Flags

None. Gate 6 runs `cargo package` only and neither reads nor forwards CARGO_REGISTRY_TOKEN.

## Next step (operator's call)

No push happened and no tag was created, moved or deleted. The local `v1.15.0` tag still points at the pre-fix release commit 450713f. Republishing needs an operator decision: move that tag to a commit that carries this fix, or cut v1.15.1. That is outside this task.

## Self-Check: PASSED

- Commits 699ae75, 59af61e, eccf0fe are all ancestors of HEAD.
- Cargo.toml, scripts/pre-tag-check.sh, CLAUDE.md and .github/workflows/release.yml all carry the described changes.
- `git tag -l 'v1.15*'` shows only `v1.15.0`, unchanged.
