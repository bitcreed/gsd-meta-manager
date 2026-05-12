<!-- generated-by: gsd-doc-writer -->
# Contributing to GSD Meta Manager

Thanks for your interest in contributing! This is a small Rust TUI project, and
contributions of all sizes are welcome — bug reports, fixes, features, and
documentation improvements.

## Reporting Issues

File issues at https://github.com/bitcreed/gsd-meta-manager/issues. A useful
bug report includes:

- What you ran and what you expected to happen
- What actually happened (paste terminal output if relevant)
- Your OS and `rustc --version`
- Steps to reproduce, ideally minimal

For feature requests, describe the use case first — what you're trying to do —
before proposing a specific design.

## Development Setup

Prerequisites: Rust 1.85+ (stable) with `cargo` on your `PATH`.

```bash
git clone git@github.com:bitcreed/gsd-meta-manager.git
cd gsd-meta-manager
cargo build
cargo test
```

Before submitting changes, make sure all three of these pass cleanly:

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

Clippy warnings are treated as errors in this project, so please fix them
locally before opening a PR.

## Commit Message Style

Commits follow [Conventional Commits](https://www.conventionalcommits.org/).
Looking at the git log, common prefixes are:

- `feat(scope): …` — new functionality
- `fix(scope): …` — bug fixes
- `docs(scope): …` — documentation changes
- `test(scope): …` — test-only changes
- `chore(scope): …` — release bumps, tooling, housekeeping

The `scope` is optional but encouraged (for example, `feat(defaults): …`).
Keep the subject line under ~72 characters and use the imperative mood
("add X", not "added X").

## Pull Requests

1. Fork the repository and create a topic branch off `master`.
2. Make your change. Keep PRs focused — one concern per PR is easier to review.
3. Run `cargo build`, `cargo test`, and `cargo clippy -- -D warnings` locally.
4. Push your branch and open a PR against `bitcreed/gsd-meta-manager:master`.
5. Describe what changed and why in the PR body. Link any related issue.

You don't need to use any particular planning workflow to contribute — the
`.planning/` directory in this repo is the maintainer's internal process, not
a requirement for external contributors. Just open a PR with a clear
description and tests where applicable.

## Code of Conduct

Be kind and constructive. Assume good intent, give thoughtful feedback, and
keep discussion focused on the code and the problem. Harassment of any kind
is not welcome.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE) that covers this project.
