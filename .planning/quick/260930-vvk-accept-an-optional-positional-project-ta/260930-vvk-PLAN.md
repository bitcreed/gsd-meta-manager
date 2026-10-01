---
quick_id: 260930-vvk
mode: quick
phase: quick-260930-vvk
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/launch_target.rs
  - src/lib.rs
  - src/cli.rs
  - src/app.rs
  - src/main.rs
  - tests/launch_target_cli.rs
  - README.md
autonomous: true
requirements: [QUICK-260930-vvk]

estimate:
  tokens: 140000
  raw_tokens: 140000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "`gsd-meta-manager <alias>` starts the TUI with that project's detail view on top of the overview. Underneath, the overview's selection is already on that alias (D-01, D-05)"
    - "`gsd-meta-manager .`, `gsd-meta-manager $(pwd)`, a relative path and an absolute path all open the registered project at that root. So does a subdirectory of a registered project. With nested registrations, the nearest (deepest) registered ancestor wins. An exact alias match always takes precedence over the path reading (D-02)"
    - "An unknown alias, a path that does not exist, or an existing path under no registered project root exits 1 BEFORE any terminal setup: no alternate screen and no raw mode. The error names the target (terminal-escaped) and suggests `gsd-meta-manager add <path>` (D-03)"
    - "Esc from a launched detail view reaches the overview in exactly as many presses as Esc from a detail view drilled into with Enter. It lands with that project selected (D-05, D-06)"
    - "Subcommands keep priority: `list`, `--config X list`, and the TUI's own driver respawn argv (`--config X drive …` from `driver::spawn::drive_argv`) all still parse as their subcommand. A target combined with a subcommand is refused with a clap usage error (exit 2) (D-01)"
    - "`--config X <target>` and `<target> --config X` both parse, carrying both values (D-01)"
    - "`--help` and README.md document the target argument and the rule that an alias colliding with a subcommand name must be opened by path (D-04)"
  artifacts:
    - path: src/launch_target.rs
      provides: "resolve_launch_target(config, target, cwd) -> Result<String, LaunchTargetError>; LaunchTargetError with escaping Display + suggestion()"
    - path: src/cli.rs
      provides: "Cli.target positional; Cli::try_parse_checked_from / Cli::parse_checked rejecting target+subcommand"
    - path: src/app.rs
      provides: "App::open_project_view(alias) -> bool, driving the overview's own Enter handler"
    - path: src/main.rs
      provides: "TUI arm: App::new + target resolution before tui::init(); open_project_view before the event loop starts"
    - path: tests/launch_target_cli.rs
      provides: "binary-level checks: refusal exits 1 with no terminal setup; a registered alias named `list` still runs the subcommand"
  key_links:
    - from: src/main.rs
      to: src/launch_target.rs
      via: "resolve_launch_target called after App::new and BEFORE tui::init()"
    - from: src/app.rs
      to: src/ui/screens/normal.rs
      via: "open_project_view selects the row, then calls App::handle_key(KeyCode::Enter), i.e. NormalScreen's Enter arm (normal.rs:636-644), which pushes DetailScreen::new"
    - from: src/main.rs
      to: src/cli.rs
      via: "Cli::parse_checked() replaces Cli::parse()"
---

<objective>
Accept an optional positional project target on the top-level CLI so that `gsd-meta-manager ttbook`, `gsd-meta-manager .` and `gsd-meta-manager $(pwd)` start the TUI directly in that project's detail view. Esc from that view must behave exactly as if the user had drilled in from the overview.

Purpose: one command from any project directory lands on that project. There is no overview-then-navigate step.
Output: new `src/launch_target.rs` resolver, a `target` positional on `Cli`, `App::open_project_view`, main.rs wiring, a binary-level test file, and README/help docs.

Locked operator decisions, numbered here in the order the operator gave them so tasks can cite them:
- D-01: `target: Option<String>` positional on `Cli`; subcommands (add/remove/list/drive/envelope) keep priority; global `--config` combines with the target; help text "alias or path of a registered project to open directly". The operator named `args_conflicts_with_subcommands = true` as the mechanism. See the DEVIATION note below.
- D-02: Resolution order. (1) exact registered alias. (2) Otherwise canonicalize the target as a path. Match a registered project whose canonical root equals it, else the nearest registered root that is an ancestor of it.
- D-03: An unknown alias or unregistered path exits non-zero BEFORE the TUI starts (no terminal setup). The error names the target and suggests `gsd-meta-manager add <path>`.
- D-04: Help and README say that an alias colliding with a subcommand name must be opened by path.
- D-05: Reuse the existing overview→project navigation (set the selection, push the project view). Build no parallel state, so Esc behaves identically.
- D-06: Tests cover target resolution (alias, `.`/relative, subdirectory, unknown → error, a subcommand winning over a same-named positional, `--config` + target) and app state (initial view is the project view; Esc lands on the overview with that project selected).
- D-07: Gate is `cargo build && cargo test --no-fail-fast && cargo clippy -- -D warnings`. Only the known git-version constants failure in src/envelope/policy.rs is ignored.

**DEVIATION from the literal D-01 mechanism, intent preserved [INFERRED — the executor MUST record this in SUMMARY under "Inferred decisions / deviations" for operator audit].**
`args_conflicts_with_subcommands = true` breaks D-01's own "subcommands keep priority" and "`--config` combines" clauses. It also breaks the D-07 gate. The planner measured this against the locked clap_builder 4.6.7 with a scratch probe:
- `x list` → List.
- `x --config c list` → **target = "list", no subcommand**.
- `x --config c add p` → **error**.

The cause is that clap sets `valid_arg_found` for ANY long flag, including the global `--config` (`clap_builder-4.6.7/src/parser/parser.rs:821`). Once that flag is set, `possible_subcommand` stops matching subcommand names (parser.rs:592). This tree emits `--config <path>` BEFORE the subcommand on its production path: `src/driver/spawn.rs:58-80` `drive_argv` is how the TUI respawns itself as `current_exe() --config X drive …`, and that child's stdio is /dev/null. The same leading-`--config` form appears in tests/registry_test.rs:272, tests/registry_worktree_guard.rs:268, tests/driver_lock.rs:567 and tests/driver_reattach.rs:268. Without the attribute, clap's default already gives subcommands priority:
- `x list` → List.
- `x --config c list` → List.
- `x --config c ttbook` → target.
- `x ttbook --config c` → target.

The only gap the attribute would have closed is `x ttbook list`, which parses as target + List. Task 3 closes that gap with a post-parse check that returns a clap `ArgumentConflict` usage error. This keeps the attribute's intent (a target and a subcommand never combine) without the regression. Regression tests pin `--config X list` and the real `drive_argv` output, so anyone who later adds the attribute sees red tests that explain why.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/cli.rs
@src/main.rs

Code facts the planner verified (cite these; do not re-derive):
- `src/cli.rs:8-21` `Cli { command: Option<Commands>, config: Option<PathBuf> (long, global) }`. Its convention (cli.rs:39-41): every `///` is rendered verbatim by clap as help, so rationale goes in `//` comments. The `use std::ffi::OsString` at cli.rs:4-5 is `#[cfg(debug_assertions)]`-gated. New ungated code must spell `std::ffi::OsString` fully qualified, or ungate the import and update its comment.
- cli.rs test rule (cli.rs:473-475): `tests/spawn_seam_guard.rs` scans src/ WITHOUT stripping test modules and counts exactly 8 lines whose trimmed form is `alias: String,` or `alias: Option<String>,`. New code and tests in src/ must never produce such a line. Use struct-pattern shorthand (`Commands::Drive { alias, .. }`). The new field is `target`, which that census does not match. Classify it in a `//` comment as a membership-checked raw lookup that creates nothing, the same class as `Remove`'s D-17-3, echoed only through `text::render_for_terminal`.
- `cli.rs` test `the_drive_subcommand_is_absent_from_rendered_help` fails if any rendered help line, trimmed, starts with `drive`. New help text must not name the hidden subcommands (drive, envelope). Use `list` as the collision example.
- `src/main.rs:80` `Cli::parse()`. `src/main.rs:520-523` TUI arm currently calls `tui::init()` BEFORE `App::new(config_path)?`. Everything up to main.rs:603 is startup before `event_bus.spawn_crossterm_reader()` at main.rs:604. main.rs:527-529 states the house rule: fs reads live in library functions, not inline in this `async fn main` body (tests/async_blocking_guard.rs scans async fn bodies).
- House shape for user-facing CLI refusals: `eprintln!("Error: …")` + hint line + `std::process::exit(1)` (main.rs:97-106). Untrusted echoes go through `gsd_meta_manager::text::render_for_terminal` (src/text.rs:487). Escaping happens once, in the error type's `Display` (the D-21-3 precedent at main.rs:42-52).
- `src/config.rs:24-29` `Config { version, projects: HashMap<String, RegisteredProject>, preferences }`. `RegisteredProject { path: PathBuf, added: String, driver_opt_in, extra }` (test literal shape: src/app.rs:5729-5737). `load_config` on a missing file returns `Config::new()` (config.rs:450-453). `Commands::Add` stores `path.canonicalize()` (main.rs:85). `tempfile = "3"` is a regular dependency.
- `src/app.rs:550-641` `App::new` → `registry::load_config_pruning_worktrees` → `from_config` (screen_stack = [NormalScreen], table_state selects 0, `ctx.filtered_aliases = ctx.sorted_aliases()`). Private `App::handle_key(code, modifiers)` at app.rs:2422 dispatches to the top screen and `process_screen_action` (Push/Pop at app.rs:2440-2459).
- Overview→project path: `src/ui/screens/normal.rs:636-644`. On `KeyCode::Enter` with `ctx.selected_alias()` set, it sets `ctx.detail_scroll_offset = 0` and `ctx.needs_redraw = true`, then returns `ScreenAction::Push(Box::new(DetailScreen::new(alias)))`. `NormalScreen::name()` is "normal". `DetailScreen::NAME` is "detail" (src/ui/screens/detail.rs:1184).
- Detail Esc: a fresh DetailScreen starts with Content focus. The first Esc moves focus to TabBar and returns None. Esc at TabBar returns `ScreenAction::Pop` (detail.rs:19604-19610 test). Only the root screen is never popped.
- App test fixtures in `src/app.rs` `mod tests`: `mouse_app(&[...])` (app.rs:5725-5742) builds an App with registered projects. `press(app, KeyCode)` (app.rs:5097) sends `Action::RawKey` through `App::update`.
- `driver::spawn::drive_argv(config_path, alias, command, run_id, goal)` is `pub` (src/driver/mod.rs:111 `pub mod spawn`) and emits `["--config", <path>, "drive", alias, "--command", cmd, "--run-id", id]`.
- Integration-test binary pattern: tests/registry_worktree_guard.rs:262-273 `run_bin` runs `env!("CARGO_BIN_EXE_gsd-meta-manager")` with `XDG_DATA_HOME`/`XDG_CONFIG_HOME` inside a tempdir plus `--config <tmp>/config.json`.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Tracer — `gsd-meta-manager <alias>` opens the project view end-to-end, and an unknown alias refuses before the TUI</name>
  <files>src/launch_target.rs, src/lib.rs, src/cli.rs, src/app.rs, src/main.rs</files>
  <behavior>
    - resolve_launch_target(config with aliases "alpha","beta", "beta", any cwd) == Ok("beta")
    - resolve_launch_target(config, "no-such-alias", cwd where no such path exists) == Err(LaunchTargetError::NotFound { .. }). Its Display contains the target, and suggestion() contains "gsd-meta-manager add"
    - A blank or whitespace-only target is refused as NotFound, never resolved as cwd [INFERRED: `cwd.join("")` would otherwise silently open the project at cwd]
    - Display of an error whose target carries "\u{1b}[31m" contains no raw ESC byte (escaped via text::render_for_terminal)
    - App (mouse_app with "proja","projb","projc"): open_project_view("projb") returns true. Afterwards the screen_stack has 2 entries, the top named "detail", and ctx.selected_alias() == Some("projb")
    - The launched app and a twin that pressed j then Enter both reach screen_stack.len() == 1 after the SAME number of Esc presses (bounded loop, at most 5). Both then have table_state.selected() == Some(1) and selected_alias() == Some("projb")
    - open_project_view("absent") returns false and leaves screen_stack.len() == 1
  </behavior>
  <action>
    Wire one path, alias only, through every layer. Task 2 adds the path-resolution branch. That is a functionality gap, not an architectural one.

    (a) New `src/launch_target.rs` (register `pub mod launch_target;` in src/lib.rs, with a one-line `///` like its neighbours). Add `pub fn resolve_launch_target(config: &Config, target: &str, cwd: &Path) -> Result<String, LaunchTargetError>`, returning the registry key (alias) to open. Step 1 per D-02 is an exact, case-sensitive `config.projects.contains_key(target)` lookup on the RAW bytes. This is a membership lookup that creates nothing, like `Remove` (D-17-3), so legacy aliases stay reachable. Refuse a blank or whitespace-only target first. For now, anything that is not an alias returns `LaunchTargetError::NotFound { target }`. Define `pub enum LaunchTargetError { NotFound { target: String }, Unregistered { target: String, path: PathBuf } }`. Task 2 starts producing `Unregistered`; declare it now so the error surface is settled in one place. Implement `Display` so that the target, and in `Unregistered` the path via `path.display().to_string()`, are rendered ONLY through `crate::text::render_for_terminal`, the D-21-3 single-producer precedent. Suggested wording: NotFound → "'<target>' is not a registered project alias or an existing path". Unregistered → "'<target>' (<path>) is not inside any registered project". Implement `std::error::Error`. Add `pub fn suggestion(&self) -> String` per D-03. NotFound → "Register the project first: gsd-meta-manager add <path>   (registered aliases: gsd-meta-manager list)". Unregistered → "Register it first: gsd-meta-manager add <escaped canonical path>". The `path` field holds the canonical path. Keep all filesystem access inside this module, so main.rs stays free of inline fs calls (main.rs:527-529, tests/async_blocking_guard.rs). Write the `#[cfg(test)] mod tests` cases from <behavior> first and run them red, then implement. Build Config fixtures as `Config::new()` plus `projects.insert(alias, RegisteredProject { path, added, driver_opt_in: None, extra: Default::default() })`.

    (b) `src/cli.rs`: add a positional `pub target: Option<String>` field on `Cli` per D-01. Its `///` first paragraph is the short help, "Alias or path of a registered project to open directly". A second `///` paragraph (long help) says that an alias sharing its name with a subcommand (e.g. `list`) must be opened by its path instead, such as `gsd-meta-manager ./list` or the absolute path, per D-04. Do NOT name the hidden subcommands in help text (see context). Add a `//` comment that classifies the field (raw membership lookup, creates nothing, echoed escaped) and points at the DEVIATION in this plan's objective: why the conflicts-with-subcommands command attribute is NOT used, with the clap parser.rs:821/592 evidence and the `drive_argv` respawn it would break. Do NOT add that attribute.

    (c) `src/app.rs`: add `pub fn open_project_view(&mut self, alias: &str) -> bool` on `App`, per D-05. Return false and do nothing unless `self.screen_stack.len() == 1` (launch-time only, root overview on top). Find the alias's index in `self.ctx.filtered_aliases` and return false if absent. Then `self.ctx.table_state.select(Some(index))`, then `self.handle_key(KeyCode::Enter, KeyModifiers::NONE)`. That runs the overview's OWN Enter arm (normal.rs:636-644), so scroll reset, redraw flag and `DetailScreen::new(alias)` all come from the one existing path and Esc is identical by construction. Return whether the top screen's `name()` is now `DetailScreen::NAME`. Doc-comment why it drives the key handler rather than pushing a screen directly: a parallel push would drift from Enter the first time Enter gains behaviour. Add the app-state tests from <behavior> to app.rs's `mod tests`, named with the prefix `launch_target_` so `cargo test --lib launch_target` selects them. Use `mouse_app` and `press`.

    (d) `src/main.rs` TUI arm (`None =>`, main.rs:520): move `let mut app = App::new(config_path)?;` ABOVE `tui::init()`. This also stops a config-load error from leaving the terminal in raw mode; note that in a `//` comment. If `cli.target` is `Some(target)`, resolve it immediately, before `tui::init()` (D-03). Get cwd from `std::env::current_dir()`. If that fails, print "Error: cannot resolve the current directory: {err}" and exit 1. On `Err(err)` from `gsd_meta_manager::launch_target::resolve_launch_target(&app.ctx.config, &target, &cwd)`, print `eprintln!("Error: {err}")` then `eprintln!("{}", err.suggestion())`, then `std::process::exit(1)`. Hold the resolved alias in `Option<String>`. After all startup state is set (after `app.ctx.last_outcomes` at main.rs:601-602, immediately before `event_bus.spawn_crossterm_reader()`), call `app.open_project_view(&alias)`. If it returns false, `tracing::warn!` and set `app.ctx.status_message` saying the project could not be opened; that state is unreachable in practice, but a silent fallback would hide a bug. Keep `Cli::parse()` for now; Task 3 replaces it.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast launch_target</automated>
    <automated>cargo build -q && bash -c 'd=$(mktemp -d); XDG_DATA_HOME=$d/x XDG_CONFIG_HOME=$d/c ./target/debug/gsd-meta-manager --config "$d/config.json" no-such-alias-vvk </dev/null >"$d/out" 2>"$d/err"; test $? -eq 1 && grep -q no-such-alias-vvk "$d/err" && grep -q "gsd-meta-manager add" "$d/err" && ! grep -qF "[?1049h" "$d/out"'</automated>
  </verify>
  <done>Unit tests for alias resolution, NotFound, escaping and the launch/Esc parity app-state test pass. The built binary given an unknown alias exits 1 with the error and the `add` suggestion on stderr, and never writes the alternate-screen sequence to stdout. Committed as the tracer.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Path targets — `.`, `$(pwd)`, relative, absolute and subdirectory resolve to the nearest registered root; unregistered paths refuse with an `add` suggestion</name>
  <files>src/launch_target.rs, tests/launch_target_cli.rs</files>
  <behavior>
    Fixture: tempdir T containing registered project roots T/alpha (alias "alpha"), T/alpha/nested (alias "inner"; a project inside a project) and T/foo (alias "foo"), plus unregistered directories T/foobar and T/alpha/src/deep.
    - target "." with cwd T/alpha → Ok("alpha")
    - target equal to the absolute T/alpha path (the `$(pwd)` form) → Ok("alpha")
    - target "alpha/src" with cwd T → Ok("alpha"). A relative path is joined onto cwd, so an exact alias lookup on "alpha/src" misses first
    - target "src/deep" with cwd T/alpha → Ok("alpha") (subdirectory → ancestor root)
    - target "." with cwd T/alpha/nested/x (a dir) → Ok("inner") (nearest/deepest ancestor wins)
    - target "../foobar" with cwd T/foo → Err(Unregistered). Component-wise ancestry, so T/foo is NOT an ancestor of T/foobar. suggestion() contains "gsd-meta-manager add" and the canonical T/foobar path
    - target "does/not/exist" with cwd T → Err(NotFound)
    - alias precedence: a directory T/beta-dir registered as alias "x", plus an unrelated registered alias literally named "beta-dir". Target "beta-dir" with cwd T → Ok("beta-dir"), the alias, per D-02 step 1
    - a registered path containing a symlink component: registered via the symlink and targeted via the real path (and vice versa) → resolves, because both sides are canonicalized
    - two aliases registered at the same canonical root → the lexicographically smallest alias wins, deterministically [INFERRED: `projects` is a HashMap, so without a tie-break the choice would vary run to run]
    - binary: `--config <cfg> <existing unregistered tempdir>` exits 1, stderr names the path and contains "gsd-meta-manager add", stdout has no "\x1b[?1049h"
    - binary: `--config <cfg> no-such-alias` exits 1 with the same no-terminal-setup property
  </behavior>
  <action>
    Extend `resolve_launch_target` in src/launch_target.rs with D-02 step 2, after the exact-alias lookup and before the NotFound fallback. Join the target onto `cwd` (`cwd.join(target)`, which keeps absolute targets as-is) and `canonicalize()` it. If that fails (path does not exist), return NotFound. Compute each registered project's canonical root as `project.path.canonicalize()`, falling back to the stored path when the root no longer exists so a stale entry can never panic or abort the lookup. Keep only roots where `canonical_target.starts_with(root)`. That is `Path::starts_with`, which is component-wise; never use a string prefix, or /x/foo would claim /x/foobar. Choose the root with the most components (nearest ancestor; an exact match is the deepest case). Break ties by the lexicographically smallest alias and mark this [INFERRED] in a `//` comment. If nothing matches, return `Unregistered { target, path: canonical_target }`. Take the cwd as a parameter and never read the process cwd inside the function, so tests stay parallel-safe. Use `std::os::unix::fs::symlink` under `#[cfg(unix)]` for the symlink case. Write the unit tests from <behavior> first (tempfile::TempDir fixtures, `RegisteredProject` literals as in Task 1) and see them fail, then implement.

    Create `tests/launch_target_cli.rs` with a `run_bin(tmp, config, args)` helper modelled on tests/registry_worktree_guard.rs:262-273: `env!("CARGO_BIN_EXE_gsd-meta-manager")`, XDG_DATA_HOME and XDG_CONFIG_HOME inside the tempdir, `--config <tmp>/config.json`, and stdin from `Stdio::null()`. Add the two binary refusal cases from <behavior>. Assert `status.code() == Some(1)`. Exactly 1, because a run that reached `tui::init()` without a TTY dies differently, so code 1 is itself evidence that the refusal came before terminal setup. Also assert stderr content and that stdout lacks the alternate-screen enter sequence `\x1b[?1049h`. Leave room in this file for Task 3's cases.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast launch_target</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test launch_target_cli</automated>
  </verify>
  <done>All path forms resolve per D-02 (exact root, `.`, absolute, relative, subdirectory, nearest ancestor, alias precedence, symlink, deterministic tie). Unregistered and nonexistent paths refuse per D-03 with the `add` suggestion, proven at the built binary with exit code 1 and no terminal setup.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Subcommands keep priority, target+subcommand is refused, `--config` combines; help and README document the target and the collision rule</name>
  <files>src/cli.rs, src/main.rs, tests/launch_target_cli.rs, README.md</files>
  <behavior>
    - try_parse_checked_from(["gsd-meta-manager","list"]) → command Some(List), target None. A subcommand wins over a same-named positional (D-06)
    - try_parse_checked_from(["gsd-meta-manager","--config","/c.json","list"]) → command Some(List), config Some, target None. This pins the DEVIATION: the conflicts-with-subcommands attribute would turn "list" into a target here
    - try_parse_checked_from(["gsd-meta-manager"] followed by the real `driver::spawn::drive_argv(Path::new("/cfg/config.json"), "demo", "/gsd-progress", "run1", None)` output) → Commands::Drive with alias "demo" and config Some. The TUI's own respawn must keep parsing
    - try_parse_checked_from(["gsd-meta-manager","--config","/c.json","ttbook"]) and (["gsd-meta-manager","ttbook","--config","/c.json"]) → target Some("ttbook"), config Some, command None (D-01)
    - try_parse_checked_from(["gsd-meta-manager","ttbook","list"]) → Err with kind ErrorKind::ArgumentConflict; the message names the target and says to pass a path for subcommand-named aliases
    - try_parse_checked_from(["gsd-meta-manager","."]) → target Some(".")
    - rendered long help contains the target's help sentence and the collision note naming `list`. The existing the_drive_subcommand_is_absent_from_rendered_help test still passes
    - binary: register a tempdir project whose folder is named `list` via `--config <cfg> add <tmp>/list`, so the derived alias is "list". Then `--config <cfg> list` exits 0 and prints the list table header "ALIAS". The subcommand ran, not a TUI launch
  </behavior>
  <action>
    In src/cli.rs add `impl Cli`. First, `pub fn try_parse_checked_from<I, T>(argv: I) -> Result<Self, clap::Error>`, where `I: IntoIterator<Item = T>` and `T: Into<std::ffi::OsString> + Clone`. It calls `Self::try_parse_from(argv)`. If both `target` and `command` are Some, it returns `Err(<Self as clap::CommandFactory>::command().error(clap::error::ErrorKind::ArgumentConflict, msg))`. The message names the target through `crate::text::render_for_terminal` and says that a project alias equal to a subcommand name is opened by path, e.g. `./list`. Second, `pub fn parse_checked() -> Self`, which calls `Self::try_parse_checked_from(std::env::args_os())` and on error calls `err.exit()`; clap then prints, exits 2 for usage errors, and exits 0 for --help/--version. A `//` comment says this post-parse check is what replaces the conflicts-with-subcommands attribute, per the DEVIATION in this plan. In src/main.rs replace `Cli::parse()` (main.rs:80) with `Cli::parse_checked()` and drop the now-unused `clap::Parser` import if clippy flags it. Add the cli unit tests from <behavior> to cli.rs's `mod tests`, prefixed `the_project_target_` so `cargo test --lib the_project_target` selects them. Use match/`if let` struct shorthand for the Drive case (never a typed `alias:` field line; see context). Each assertion message should explain WHY the case matters: for the `--config X list` and `drive_argv` cases, name the respawn path that goes dark with /dev/null stdio.

    Add the binary case from <behavior> to tests/launch_target_cli.rs. Create `<tmp>/list/.planning/` and run `add <tmp>/list` through `run_bin`; the tempdir is not a git repo, so the linked-worktree refusal does not fire. Then run `list` and assert exit 0 and that stdout contains "ALIAS" and "list".

    README.md (D-04): replace the help block under `## Usage` (README.md:182-198) with the actual output of `./target/debug/gsd-meta-manager --help` after this change; it will show the `[TARGET]` argument. Under `### Examples` (README.md:317), before the `add` example, add a short "Open a project directly" example set: `gsd-meta-manager my-app` (by alias), `gsd-meta-manager .` (from inside a project, including any subdirectory) and `gsd-meta-manager ~/projects/my-app` (by path). Add one sentence each: Esc returns to the overview with that project selected; an unregistered target exits with a hint to `gsd-meta-manager add <path>`; an alias that collides with a subcommand name (e.g. a project registered as `list`) must be opened by its path. Do not mention hidden subcommands.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast cli::tests</automated>
    <automated>rtk proxy cargo test --no-fail-fast --test launch_target_cli --test spawn_seam_guard --test async_blocking_guard</automated>
    <automated>rtk proxy grep -c "gsd-meta-manager \." README.md</automated>
  </verify>
  <done>Subcommands, including the `--config X drive …` respawn form, parse exactly as before. Target+subcommand is a clap usage error. `--config` combines with the target in either order. Help and README document the target, the open-by-path collision rule and the refusal hint. The existing help-hiding test and the source-census guards stay green.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| argv → CLI | The positional target is untrusted user/script input. It reaches a registry lookup, a filesystem canonicalize, and terminal echo |
| registry (config.json) → resolver | Registered paths and aliases may be legacy entries an older build accepted (invisible or look-alike aliases, stale paths) |
| CLI → TUI respawn | The TUI re-execs itself as `--config X drive …` with /dev/null stdio, so any parser regression there is invisible |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-vvk-01 | Tampering | LaunchTargetError Display (src/launch_target.rs) | medium | mitigate | Target and path are echoed only through `text::render_for_terminal` inside `Display`, the single-producer rule (D-21-3). A unit test plants `\u{1b}[31m` and asserts no raw ESC reaches the message |
| T-vvk-02 | Spoofing | resolve_launch_target alias step | low | mitigate | Exact, byte-equal key lookup only; no case folding or fuzzy matching. Ancestor matching uses component-wise `Path::starts_with` on canonical paths, so `/x/foo` never claims `/x/foobar`. Ties broken deterministically |
| T-vvk-03 | Denial of Service | CLI parser / driver respawn (src/cli.rs, src/driver/spawn.rs:58-80) | high | mitigate | Do not use the conflicts-with-subcommands attribute (it turns `--config X drive …` into a target and breaks the respawn). Regression tests parse the real `drive_argv` output and `--config X list` |
| T-vvk-04 | Elevation of Privilege | target + subcommand combination | low | mitigate | Post-parse `ArgumentConflict` refusal, so a target can never ride along with, or be silently dropped beside, a subcommand. Opening a detail view runs no agent and writes nothing |
| T-vvk-05 | Information Disclosure | refusal message echoes the canonical path | low | accept | The path is derived from the caller's own argument on their own machine. Echoing it is the actionable `add` hint D-03 requires |
| T-vvk-06 | Denial of Service | stale registered root during resolution | low | mitigate | A failing canonicalize on a registered root falls back to the stored path rather than erroring or panicking, so one stale entry cannot block launching any other project |
| T-vvk-SC | Tampering | npm/pip/cargo installs | low | accept | No new dependencies: clap, tempfile and std only. No package-manager install task, so no legitimacy gate applies |
</threat_model>

<verification>
Full gate per D-07, run from the repo root:

`cargo build && rtk proxy cargo test --no-fail-fast 2>&1 | tail -40 && cargo clippy -- -D warnings`

Expected: build and clippy clean. Tests have exactly one failure, the git-version constants witness in src/envelope/policy.rs, which is environmental on this machine. Any other failure is a real regression. Use `--no-fail-fast` so the envelope suites actually run. Use `rtk proxy` for any test output you grep, and never pipe it through a second filter that drops lines before counting.
</verification>

<success_criteria>
- `gsd-meta-manager <alias|.|path|subdir>` opens that project's detail view at launch (app-state test plus resolver tests).
- Esc parity with drilled-in navigation is proven by a side-by-side test.
- Unknown and unregistered targets exit 1 before terminal setup, proven at the built binary.
- `list`, `--config X list` and the `drive_argv` respawn still parse as subcommands. Target+subcommand is refused.
- Help and README document the target and the open-by-path collision rule.
- The D-07 gate is green apart from the single known policy.rs failure.
- SUMMARY records every [INFERRED] item: the D-01 mechanism DEVIATION with its clap evidence, the blank-target refusal, the deterministic tie-break, App::new moved before tui::init, and the open-by-Enter-handler reuse choice.
</success_criteria>

<output>
Create `.planning/quick/260930-vvk-accept-an-optional-positional-project-ta/260930-vvk-SUMMARY.md` when done.
</output>
