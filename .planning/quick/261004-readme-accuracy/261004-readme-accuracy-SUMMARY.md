---
status: complete
---
# Summary 261004-readme-accuracy

Changes: README.md (tagline defines GSD; compat note without numbers; real `--help`; Features/Quick start dedupe; Config `g` key; macOS config path note), src/cli.rs clap `about` now equals Cargo.toml `description`, docs/GSD-CORE-SYNC.md keep-in-step note, STATE.md row fix.

Factual errors fixed: Config tab scope-switch key is `g` (`d` is only an alias), not `d`; `--help` block was a stale hand-written copy (about text differed); config path is platform-dependent (dirs::config_dir).

Verified OK: 8 tabs plus experimental Driver; keys 1-8; dashboard keys a/c/d/s/b/M/r/x/o; GSDMM_EXPERIMENTAL_FEATURES (truthy 1/true/yes/on); rust-version 1.88; subcommands add/remove/list.

Inferred decisions: command form `/gsd-progress` kept (matches installed skill names and src defaults); clap about changed to the Cargo wording (no tests reference it); Cargo description left unchanged (acronym does not read naturally there); baseline lives in docs/GSD-CORE-SYNC.md (confirmed present).

Tests: cargo test --no-fail-fast: only the expected envelope/policy.rs git-version test fails.
