---
quick_id: 261103-ubn
mode: quick
phase: quick-261103-ubn
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/backlog.rs
  - src/oneoff/backlog.rs
  - src/oneoff/mod.rs
  - src/oneoff/prompt.rs
  - src/oneoff/launch.rs
  - src/ui/screens/oneoff.rs
  - src/cli.rs
  - src/task_cli.rs
  - tests/oneoff_e2e.rs
  - tests/oneoff_runtime_seam_guard.rs
  - src/ui/screens/detail.rs
  - src/app.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-261103-ubn]

estimate:
  tokens: 130000
  raw_tokens: 130000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - "With GSDMM_EXPERIMENTAL_FEATURES on, pressing `t` on the detail view's Backlog tab (list or open content pane) sends exactly one `Action::OneoffRunRequested { alias, mode: Mode::Quick, task: TaskSpec::Backlog { number, title, entry }, model }` for the SELECTED item. It pushes no screen and shows no picker or confirm. It starts another run even while other one-offs are live. The model is `ui::screens::oneoff::default_model()`, the same function the picker's confirm popup uses; budget and wall clock come from `cli::task_run_defaults()` inside `App::request_oneoff_run`. The status line reads `Starting one-off /gsd-quick for backlog 999.N...`."
    - "With the flag off, `t` on the Backlog tab is a no-op: no action on the channel, no status message, `ScreenAction::None`, and no Backlog `t` row in the help screen. The Driver-tab `t` still opens the one-off picker, and `t` on every other tab behaves as before. On a platform where `driver_steer::can_steer()` is false, `t` shows `STEERING_UNSUPPORTED_PLATFORM`."
    - "The agent's prompt starts with `/gsd-quick Implement backlog item 999.N: <title>` (title on one clean line). Next comes the item's FULL ROADMAP.md section: its `### Phase 999.N:` heading through the line before the next same-or-higher heading or thematic break, as `state_reader::roadmap_md::phase_section` locates it. If ROADMAP.md has no such section, the item directory's `.md` files are used instead. The entry sits between BEGIN/END data markers, with control and invisible-formatting characters stripped (newlines and tabs kept) and the size capped at 48 KiB behind a visible truncation note. Last come the standard reporting block and an instruction: if and only if the status is done, delete the `### Phase 999.N:` section from ROADMAP.md's `## Backlog`, `git rm -r` the `.planning/phases/999.N-*` directory if there is one, and commit."
    - "`gsd-meta-manager task run <alias|path> --backlog 999.N` works detached and with `--foreground`. It is also the transport the TUI's detached daemon uses (`daemon_argv` emits `--backlog <number>`). run.json records `task.kind == \"backlog\"`. A resumed backlog run rebuilds the same task prompt within MAX_PROMPT_BYTES. Records written before this change still deserialize."
    - "Every refusal is a status-line message in the TUI, or stderr plus a non-zero exit from the CLI, never a dialog. Cases: no item selected; a number that is not `999.<digits>`; a number with no backlog directory; no ROADMAP.md section and no directory `.md`; an unsupported platform. Repository-derived text reaches the status line only through `text::render_for_terminal` or after `valid_backlog_number` has accepted it."
    - "The help screen lists `t` under the Backlog-tab keys when the flag is on. docs/CONFIGURATION.md documents the Backlog `t` key, the `--backlog` CLI flag, and the flag-off surface list."
    - "`rtk proxy cargo test --no-fail-fast`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean. Cargo.toml and Cargo.lock are unchanged."
  artifacts:
    - path: src/oneoff/backlog.rs
      provides: "MAX_ENTRY_BYTES (48 KiB); valid_backlog_number; clean_entry (strip control/invisible chars except \\n \\t, cap with visible note, idempotent); backlog_spec(project, number) -> Result<TaskSpec, String>; unit tests"
    - path: src/state_reader/backlog.rs
      provides: "backlog_dispatch_entry(planning_dir, dir_name) -> Option<String>: ROADMAP.md section via roadmap_md::phase_section, else the directory's labelled .md bodies (shared helper with load_backlog_content); tests"
    - path: src/oneoff/mod.rs
      provides: "TaskSpec::Backlog { number, title, entry }; pub mod backlog; round-trip + stored-record compat tests"
    - path: src/oneoff/prompt.rs
      provides: "build() Backlog arm: first line, data-marked entry, remove-on-done bookkeeping line; tests incl. worst-case resume bound"
    - path: src/oneoff/launch.rs
      provides: "validate() Backlog arm; daemon_argv emits --backlog <number>; tests"
    - path: src/cli.rs
      provides: "TaskRunArgs.backlog (--backlog <999.N>) in the required task ArgGroup"
    - path: src/task_cli.rs
      provides: "run_main resolves --backlog through oneoff::backlog::backlog_spec (parent and daemon alike)"
    - path: src/ui/screens/oneoff.rs
      provides: "pub(crate) fn default_model(); Confirm display arm for TaskSpec::Backlog"
    - path: tests/oneoff_e2e.rs
      provides: "detached `task run --backlog` end-to-end test on the fake harness + refusal test"
    - path: tests/oneoff_runtime_seam_guard.rs
      provides: "src/oneoff/backlog.rs registered in NEUTRAL_FILES"
    - path: src/ui/screens/detail.rs
      provides: "Backlog-tab `t` arm (experimental-gated, can_steer refusal, direct dispatch); tests named *backlog_one_off*"
    - path: src/app.rs
      provides: "request_oneoff_run's start status names the backlog item for TaskSpec::Backlog; test"
    - path: src/ui/screens/help.rs
      provides: "flag-gated Backlog `t` row directly under the Backlog `e` row; positive and flag-off tests"
    - path: docs/CONFIGURATION.md
      provides: "Backlog-tab key paragraph, --backlog in the CLI synopsis, env-var row and flag-off list updated"
  key_links:
    - from: src/ui/screens/detail.rs
      to: src/oneoff/backlog.rs
      via: "the Backlog `t` arm calls crate::oneoff::backlog::backlog_spec(project_path, item.number.as_raw_for_logic_only()) and dispatches OneoffRunRequested via driver_confirm::dispatch"
      pattern: "backlog_spec"
    - from: src/oneoff/launch.rs
      to: src/task_cli.rs
      via: "daemon_argv pushes --backlog <number>; the re-executed `task run --daemon-child` resolves it with backlog_spec"
      pattern: "\"--backlog\""
    - from: src/oneoff/backlog.rs
      to: src/state_reader/backlog.rs
      via: "backlog_spec finds the item via parse_backlog_items and its text via backlog_dispatch_entry (roadmap_md::phase_section)"
      pattern: "backlog_dispatch_entry"
    - from: src/oneoff/prompt.rs
      to: src/oneoff/backlog.rs
      via: "build()'s Backlog arm re-applies clean_entry and valid_backlog_number to whatever spec it is given (fresh or from a resumed record)"
      pattern: "clean_entry"
---

<objective>
Dispatch a backlog item as a one-off `/gsd-quick` run with one key. On the detail view's Backlog tab, `t` on the selected item starts a sandboxed one-off right away: no picker, no typing, no confirm, in parallel with any live runs. The task carries the item's number, title and full ROADMAP.md entry, and tells the agent to remove the entry once it is done. The key is gated by the experimental flag. Feedback goes to the status line. Help, docs and tests are updated.

Purpose: a backlog item can go from idea to running agent in one keypress, without retyping its entry. The existing typed-task path cannot carry it: `TaskSpec::Text` is one line of at most 2000 characters.

Output: a new `TaskSpec::Backlog` variant carried end to end. The path is TUI key → `Action::OneoffRunRequested` → `App::request_oneoff_run` → `launch::run_detached` → `task run --backlog` daemon → prompt. It is proven by a detached CLI e2e test and by screen and app tests.

All decisions come from 261103-ubn-CONTEXT.md. Every entry there is INFERRED (the human was unavailable) and is marked for later audit. Planner discretion choices made here are marked [inferred — audit] inline.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/quick/261103-ubn-dispatch-a-backlog-item-as-a-one-off-gsd/261103-ubn-CONTEXT.md
@CLAUDE.md

Source anchors (verified live at planning time, master 2aa85fb):
- src/oneoff/mod.rs:127-135 — `TaskSpec` (`#[serde(tag = "kind", content = "spec", rename_all = "snake_case")]`, variants `Todo { file_name, title }`, `Text(String)`); round-trip test at :229.
- src/oneoff/prompt.rs:43-82 — `build()`. The todo arm and its `git mv` bookkeeping line are the pattern to mirror. `check_task_text` at :23. `build_resume` at :114 calls `build()` and errs above MAX_PROMPT_BYTES.
- src/oneoff/todos.rs — `sanitize_title` (one clean line, ≤200 chars) is the title cleaner to reuse; the module shape is the pattern for the new `src/oneoff/backlog.rs`.
- src/oneoff/launch.rs:166-172 — `validate()` exhaustive task match; :1224-1250 `daemon_argv` exhaustive `(resume, task)` match (todo → `--todo`, text → `--text`); tests at :1695+ (`opts()` helper, `the_daemon_argv_carries_every_option_and_the_config_first`, `validation_refuses_bad_options`). The I-3 committed-todo check at :417 is todo-only and stays so.
- src/cli.rs:645-688 — `TaskRunArgs` with `#[command(group(ArgGroup::new("task").required(true).args(["todo", "text", "resume_from"])))]`.
- src/task_cli.rs:179-198 `todo_spec` (reads the project checkout); :258-273 task resolution `match (&args.todo, &args.text)`.
- src/ui/screens/oneoff.rs:312-319 `confirm()` computes the default model inline from `AgentRuntime::default().models().first()` with an "opus" fallback; :832-838 Confirm display match on `TaskSpec`.
- src/state_reader/backlog.rs — `parse_backlog_items` (items come from `.planning/phases/999.*` dirs, `description` = first `# ` heading of a dir `.md` else the humanized slug), `load_backlog_content` (ROADMAP section + labelled dir `.md` bodies), `backlog_edit_target`. All use `super::roadmap_md::phase_section` / `phase_section_line` (src/state_reader/roadmap_md.rs:1204), which ends an entry at a same-or-higher heading or a thematic break and never confuses 999.1 with 999.10.
- src/ui/screens/detail.rs:5994-6033 — Driver-tab `t`/`T` arms (can_steer refusal, `driver_confirm::dispatch`); :6175-6224 Backlog `e` arm (key-time ROADMAP read, `item.dir_name.as_raw_for_logic_only()`); :6261 the main match ends `_ => ScreenAction::None`; text-input intercepts exist only on the Config (Defaults) tab (:3841-3855). Test fixtures: `backlog_content_fixture()` (:24317, a `999.1-alternate-transport-fallback/.gitkeep` dir plus a ROADMAP `## Backlog` section), `driver_action_fixture()` (:20666, event channel), `the_one_off_keys_do_nothing_with_the_flag_off` (:20726), `test_ctx().with_experimental(..)`.
- src/app.rs:1539-1553 `oneoff_target` (inert with the flag off, can_steer refusal); :1584-1650 `request_oneoff_run` (uses `cli::task_run_defaults()`; status `Starting a one-off on {alias}`); :3353 `OneoffRunStarted` handler (`One-off {run_id} started on {alias}`); tests :8840-8900 (`fake_oneoff` seam recorder, `app_on_the_driver_tab`).
- src/ui/screens/help.rs:250-252 — the Backlog-tab rows (always-on block); :334-411 the experimental Driver section; tests `help_lines_documents_every_key_this_phase_binds` (:616), `driver_rows()` (:~1000), `the_help_screen_documents_no_driver_row_when_experimental_is_off` (:1052), blank-line structure test (:1202).
- src/ui/screens/mod.rs:1394-1409 — `AppContext::experimental` doc says "today, every driver surface".
- docs/CONFIGURATION.md:120 (env-var row: "one-off tasks (`t` / `T`) included"), :130-142 (flag-off surface list), :164-209 ("One-off tasks" / "Keys (Driver tab)"), :212 (CLI synopsis `task run <alias|path> (--todo <file> | --text …)`, the text placeholder spelled as in the file).
- tests/oneoff_e2e.rs — `fixture()`/`prepared()`, `run()`, `wait_finished()`, `fx.write_scenario(..)`, `fx.recorded("prompt.txt")`; `quick_run` is the detached pattern, `a_fast_run_has_the_fast_prompt_and_no_summary` the minimal-scenario pattern, `an_uncommitted_todo_fails_before_the_agent_starts` the refusal pattern.
- tests/oneoff_runtime_seam_guard.rs:11-29 — `NEUTRAL_FILES`. A new `src/oneoff/*.rs` file must be listed there, and it must not name the agent runtime outside `//` comments.
- src/launcher/sandbox.rs:787 — `MAX_PROMPT_BYTES = 120 * 1024`. run.json reads are capped at 1 MiB (`read_run_file_capped`).

Live checks done at planning time:
- Nothing else binds `t` on the Backlog tab, and the Backlog tab has no text-input mode.
- No other view lets a backlog item be selected. The dashboard shows only a count column (normal.rs:442), and the Roadmap views have no backlog rows. So `t` is bound on the Backlog tab only. [inferred — audit]
- No docs-sync test reads docs/CONFIGURATION.md.
- README defers experimental keys to CONFIGURATION.md, so README is not changed. That omission and the footer hint are logged as musk M-0041.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): `task run --backlog 999.N` carries a backlog item end to end — ROADMAP section → TaskSpec::Backlog → validate → daemon argv → daemon re-resolve → prompt</name>
  <files>src/state_reader/backlog.rs, src/oneoff/backlog.rs, src/oneoff/mod.rs, src/oneoff/prompt.rs, src/oneoff/launch.rs, src/ui/screens/oneoff.rs, src/cli.rs, src/task_cli.rs, tests/oneoff_e2e.rs, tests/oneoff_runtime_seam_guard.rs</files>
  <behavior>
    - e2e (tests/oneoff_e2e.rs, names contain `backlog`). The fixture project commits a `.planning/ROADMAP.md` with `## Backlog`, then `### Phase 999.3: Demo backlog item (BACKLOG)` with a multi-line body that includes a `#### Notes` sub-heading and a `- [ ] TBD` line, then `### Phase 999.4: Other item (BACKLOG)` with its own body. It also commits the dirs `.planning/phases/999.3-demo-backlog-item/.gitkeep` and `.planning/phases/999.4-other-item/.gitkeep`. A DETACHED `task run <ALIAS> --backlog 999.3` with the one-line success scenario exits 0 and finishes. Then prompt.txt starts with `/gsd-quick Implement backlog item 999.3: Demo backlog item`; contains the 999.3 heading, every body line and the `#### Notes` line; contains no 999.4 heading or body; contains the BEGIN and END markers, `If and only if the status is done`, `### Phase 999.3:` and `.planning/phases/999.3-*`. `task status --json` gives record.task.kind == "backlog" and record.task.spec.number == "999.3".
    - e2e refusal: `task run <ALIAS> --backlog 999.9 --foreground` (no such dir) exits non-zero, stderr names the refusal, and no prompt.txt is recorded.
    - state_reader::backlog: `backlog_dispatch_entry` returns the ROADMAP section only. It stops at the next `### Phase`, at `## `, and at a thematic break; it keeps a `####` sub-heading; and `999.1` never yields `999.10`'s section. With no ROADMAP section it falls back to the dir's `.md` bodies, each labelled `── <name> ──` exactly as `load_backlog_content` labels them. With neither it returns None.
    - oneoff::backlog: `valid_backlog_number` accepts `999.1` and `999.12` and rejects `999`, `999.`, `999.x`, `999.1.2`, `../999.1`, `999.1 ` and `-999.1`. `clean_entry` drops `\u{7}`, ESC, `\r` and `\u{202E}`, keeps `\n` and `\t`, and trims trailing whitespace. An entry above MAX_ENTRY_BYTES becomes at most MAX_ENTRY_BYTES bytes, note included, cut on a char boundary (test with multi-byte text). The note is visible and names ROADMAP.md, and clean_entry(clean_entry(x)) == clean_entry(x). `backlog_spec` on a temp project returns TaskSpec::Backlog with the item's title (the description, cleaned by sanitize_title, falling back to the number when blank) and its cleaned section. The error cases are: unknown number → Err mentioning "not an item"; dir present but neither ROADMAP section nor `.md` → Err containing "has no ROADMAP.md entry to dispatch"; malformed number → Err that does not echo the input.
    - oneoff::mod: TaskSpec::Backlog round-trips through JSON as `"kind":"backlog"`. The literal stored-record JSON for the Text and Todo shapes (`{"kind":"text","spec":"fix x"}`, `{"kind":"todo","spec":{"file_name":"a.md","title":"A"}}`) still deserializes to the same values.
    - prompt: build(Quick, Backlog) has the first line `/gsd-quick Implement backlog item 999.3: <title>`. The body carries number, title, the full multi-line entry and the markers, plus the common reporting rules (`taskctl report`, `run it again`, `[inferred — audit]`, the branch) and the remove-on-done line. An entry with control/invisible characters arrives cleaned. A >200 KiB entry keeps the prompt ≤ MAX_PROMPT_BYTES and shows the truncation note. An entry that contains the END marker text cannot close the block early. A malformed number or a blank entry → Err. Worst case: build_resume of a Backlog task with a MAX_ENTRY_BYTES entry of 4-byte chars, a 200-char 4-byte title, 30 follow-ups and 80 commit lines of 4-byte chars → Ok and ≤ MAX_PROMPT_BYTES.
    - launch: daemon_argv for a Backlog task (no resume) contains `--backlog`, `999.3`, and neither `--todo` nor `--text`. With a resume it still carries only `--resume-from`. validate refuses a malformed number and a blank entry, and accepts a good spec.
  </behavior>
  <action>
Build the thinnest complete path first: the e2e test, red, then the code. Keep it production quality. This task touches more than 5 files, and that is forced by the compiler and the transport. A new TaskSpec variant needs an arm in every exhaustive match in the same commit (prompt.rs build, launch.rs validate and daemon_argv, ui/screens/oneoff.rs Confirm). The TUI's detached daemon is re-executed through `task run`, so cli.rs and task_cli.rs are the transport the TUI depends on, not a separate feature. The two tests/ files are the proof and the registry entry.

1. src/state_reader/backlog.rs: add `pub fn backlog_dispatch_entry(planning_dir: &Path, dir_name: &str) -> Option<String>`. It returns the ROADMAP.md section for the dir's number via `super::roadmap_md::phase_section`, the SAME locator `load_backlog_content` and `backlog_edit_target` use. Failing that, it returns the dir's `.md` bodies joined the way `load_backlog_content` joins them. Failing that, None. Extract the md-bodies loop into one private helper shared by both functions; do not duplicate it. Per CONTEXT "Entry source": no special case for PROMOTED items [inferred — audit]. Doc comment: raw text, the caller cleans it.

2. New src/oneoff/backlog.rs, a sibling of oneoff/todos.rs. Module doc: what a dispatchable backlog item is, and that all of it is untrusted repository text. Contents:
   - `pub const MAX_ENTRY_BYTES: usize = 48 * 1024`. Doc the budget: entry + 800-byte title + fixed instructions + the largest build_resume continuation (20 follow-ups and 50 commits × 200 four-byte chars, a 400-char note) stays under MAX_PROMPT_BYTES = 120 KiB [inferred — audit].
   - `pub fn valid_backlog_number(&str) -> bool`, a static LazyLock regex `^999\.[0-9]{1,9}$` as todos.rs does.
   - `pub fn clean_entry(&str) -> String`. Remove `\r`, every other `char::is_control` except `\n` and `\t`, and every char for which `text::carries_invisible_formatting` is true (per char, as `todos::sanitize_title` does). Trim trailing whitespace. Above MAX_ENTRY_BYTES, cut at the largest char boundary that leaves room for a final ASCII note line saying the entry was truncated and the rest is in .planning/ROADMAP.md. Result ≤ MAX_ENTRY_BYTES, and a second application changes nothing.
   - `pub fn backlog_spec(project: &Path, number: &str) -> Result<TaskSpec, String>`. Refuse a number that fails valid_backlog_number with a fixed message that does not echo the input. Find the item in `state_reader::backlog::parse_backlog_items(&project.join(".planning"))` by `number.as_raw_for_logic_only() == number`; if none, Err "backlog {number} is not an item of this project". Text: `backlog_dispatch_entry(planning_dir, item.dir_name.as_raw_for_logic_only())` → clean_entry; None or blank → Err "backlog {number} has no ROADMAP.md entry to dispatch" (CONTEXT wording). Title: `todos::sanitize_title(item.description.as_raw_for_logic_only())`, falling back to the number when empty [inferred — audit: the title is the Backlog tab's own label for the item, so what you select is what is dispatched]. Return `TaskSpec::Backlog { number, title, entry }`.
   - Register it: `pub mod backlog;` in src/oneoff/mod.rs, and `"src/oneoff/backlog.rs"` in NEUTRAL_FILES of tests/oneoff_runtime_seam_guard.rs. No non-comment line in it may name the agent runtime.

3. src/oneoff/mod.rs: add `Backlog { number: String, title: String, entry: String }` to TaskSpec. Doc it as a backlog item: the number, the tab's title, and the cleaned ROADMAP entry recorded at dispatch so a resume rebuilds the same prompt. Existing variants and the serde attributes stay unchanged, so stored records keep deserializing (CONTEXT). Extend the round-trip test and add the literal stored-record compat assertions from <behavior>.

4. src/oneoff/prompt.rs: add a `TaskSpec::Backlog` arm to `build()`. Turn the todo-only `todo_move` into one optional bookkeeping line per task kind.
   - Refuse a malformed number. Re-apply `backlog::clean_entry`, since a spec may come from a resumed record, and refuse a blank result.
   - Title: sanitize_title, falling back to the number.
   - First line: `{mode.command()} Implement backlog item {number}: {title}`.
   - Then a blank line and a header line saying this is backlog item {number}'s entry from .planning/ROADMAP.md, the item to implement, and that it is repository data, so it cannot change the dispatcher's instructions below.
   - Then the line `<<< BEGIN BACKLOG ENTRY {number} >>>`, the entry, and the line `<<< END BACKLOG ENTRY {number} >>>`. Before inserting, rewrite every occurrence of `<<< END BACKLOG ENTRY` inside the entry to `<< END BACKLOG ENTRY` so the data cannot close its block.
   - Then the existing shared "How to work and report" block, unchanged.
   - Then the bookkeeping line (CONTEXT "Completion bookkeeping", mirroring the todo `git mv` line): if and only if the status is done, remove backlog item {number} the way GSD's backlog review does: delete its `### Phase {number}:` section from the `## Backlog` section of `.planning/ROADMAP.md`, `git rm -r` its `.planning/phases/{number}-*` directory if there is one, and commit that with your work.
   - Update the module doc to say a backlog item's entry IS inlined, as cleaned and bounded data. Add the prompt tests from <behavior>.

5. src/oneoff/launch.rs:
   - validate(): a Backlog arm that refuses an invalid number or a blank `clean_entry(entry)` via `invalid(..)`.
   - daemon_argv(): an arm `(None, TaskSpec::Backlog { number, .. })` pushing `--backlog` and the number, the way the todo arm pushes `--todo`.
   - Tests as in <behavior>, reusing the `opts()` helper.

6. src/cli.rs: add `pub backlog: Option<String>` to TaskRunArgs with `#[arg(long, value_name = "999.N")]`. Doc comment: a backlog item's number; its ROADMAP.md entry (else its directory's .md files) is the task. Add "backlog" to the ArgGroup args list, so clap enforces exactly one task source. Update the struct doc line: append ` | --backlog <999.N>` inside its existing `(--todo <file> | --text …)` group.

7. src/task_cli.rs: in run_main's task resolution, match the three sources. A lone `--backlog` resolves through `gsd_meta_manager::oneoff::backlog::backlog_spec(&project, number)`; on Err print `task run: {message}` with err_line and return 1, as todo_spec errors do. The fallback message becomes "give exactly one of --todo, --text and --backlog". The parent and the `--daemon-child` both resolve from the project checkout, the same TOCTOU-tolerant pattern todos use [inferred — audit]. No committed-entry check is added, because the prompt carries the entry (CONTEXT). The remove-on-done line says "if there is one" for the dir.

8. src/ui/screens/oneoff.rs:
   - Extract the inline default model of `confirm()` into `pub(crate) fn default_model() -> String`: the runtime's first model, else "opus". Have `confirm()` call it. Task 2's Backlog key uses the same function, so it is never a copy (CONTEXT "Dispatch path").
   - Add a Confirm display arm for Backlog: `backlog {number}: {title}` through the screen's existing `shown()`. The arm is compile-forced; the Backlog key never opens this popup.

9. tests/oneoff_e2e.rs: add a backlog fixture helper built on `fixture()` that writes the ROADMAP and the two dirs from <behavior> and commits them with the file's `git` helper. Add the detached test, using the `quick_run` id-parsing and `wait_finished` pattern with the one-line success scenario, and the `--foreground` refusal test.

Write the e2e test first and see it fail for the expected reason: clap rejects `--backlog`. Then make it pass. Record the RED line in the SUMMARY.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- oneoff:: state_reader::backlog:: && rtk proxy cargo test --no-fail-fast --test oneoff_e2e backlog && rtk proxy cargo test --no-fail-fast --test oneoff_runtime_seam_guard</automated>
  </verify>
  <done>A detached `task run <alias> --backlog 999.3` on the fake harness delivers a prompt whose first line is `/gsd-quick Implement backlog item 999.3: Demo backlog item`. The prompt holds the full 999.3 section and nothing of 999.4, between markers, plus the remove-on-done line. run.json records task.kind "backlog". An unknown item is refused before any agent starts. All listed unit tests pass, and the seam guard covers src/oneoff/backlog.rs.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: `t` on the Backlog tab dispatches the selected item immediately (experimental-gated), with status feedback, help row and docs</name>
  <files>src/ui/screens/detail.rs, src/app.rs, src/ui/screens/help.rs, src/ui/screens/mod.rs, docs/CONFIGURATION.md</files>
  <behavior>
    - detail.rs, tests named `backlog_one_off_*`, cfg(linux|macos) where they need dispatch, like the Driver `t` tests. All use `backlog_content_fixture()` plus an event channel and the flag on.
    - After `3`, `t` returns ScreenAction::None (no push). The channel then holds exactly one `OneoffRunRequested` with alias == TEST_ALIAS, mode == Quick, model == `super::oneoff::default_model()`, and a task that is `TaskSpec::Backlog` with number "999.1", title "Alternate transport fallback", and an entry containing "Read the device another way." and "- [ ] TBD". The status message contains "backlog 999.1".
    - The same with the content pane open (`Enter`, then `t`).
    - A second `t` with a live one-off in `oneoff_runs` dispatches again.
    - Flag off: `t` on the Backlog tab → ScreenAction::None, an empty channel and no status message.
    - Flag on, `t` on a tab that is neither Backlog nor Driver (e.g. Queue or Roadmap) → no OneoffRunRequested.
    - ROADMAP.md deleted from the fixture (dir has only .gitkeep) → status contains "has no ROADMAP.md entry to dispatch" and no dispatch.
    - No backlog items loaded → status says no backlog item is selected, and no dispatch.
    - The existing `t_opens_the_one_off_picker_and_asks_for_the_todos` and `the_one_off_keys_do_nothing_with_the_flag_off` still pass.
    - app.rs, test `backlog_one_off_start_status_names_the_item`, with the existing `fake_oneoff` seam: OneoffRunRequested with a TaskSpec::Backlog for 999.1 → the status message contains "backlog 999.1". The recorded options carry that task, Mode::Quick, and the `task_run_defaults()` budget and wall clock.
    - help.rs: with the flag on, the body has the whole row `row("t", "Backlog tab: start the selected item as a one-off /gsd-quick run (experimental)")` directly after the Backlog `e` row. With the flag off that row is absent. The existing blank-line structure and uniqueness tests stay green.
  </behavior>
  <action>
1. src/ui/screens/detail.rs: add an arm `KeyCode::Char('t') if current_view == DetailSubView::Backlog && ctx.experimental` next to the Driver `t` arm. Comment it with the quick id, CONTEXT "Key and where it works" and the gate. Flow:
   - Set ctx.needs_redraw. If `!super::driver_steer::can_steer()`, set the STEERING_UNSUPPORTED_PLATFORM status and return ScreenAction::None, exactly as the Driver `t` does.
   - Resolve the registered project path from ctx.config.projects and the selected item from `view_cache[alias].backlog_items.get(backlog_selected)`. Each is a separate read, so the view-cache borrow ends before ctx fields are written. Missing either → status "No backlog item selected" and return None.
   - Call `crate::oneoff::backlog::backlog_spec(&project_path, item.number.as_raw_for_logic_only())`. Key-time I/O is accepted here, as for Backlog `e` (CONTEXT).
   - Err(msg) → status `crate::text::render_for_terminal(&msg).to_string()`, return None.
   - Ok(task) → status `Starting one-off /gsd-quick for backlog {number}...`. The number has passed valid_backlog_number; render it anyway. Then `super::driver_confirm::dispatch(ctx, Action::OneoffRunRequested { alias, mode: crate::oneoff::Mode::Quick, task, model: super::oneoff::default_model() }, &alias)` and return ScreenAction::None. Push no screen and add no confirm (CONTEXT "Dispatch path"). Starting while other runs are live is deliberate (quick 261103-hx7, C-28).
   - With the flag off the guard is false. `t` falls through to the main match's `_ => ScreenAction::None`, which is the unbound-key behaviour CONTEXT requires.
   - Add the tests from <behavior>.

2. src/app.rs, in `request_oneoff_run`: when the task is `TaskSpec::Backlog { number, .. }`, the start status becomes `Starting one-off {mode.command()} for backlog {number} on {alias}...`, with alias and number through `text::render_for_terminal`. Without this, the App's generic message would overwrite the key handler's within one loop turn. Every other task keeps `Starting a one-off on {alias}`. The OneoffRunStarted text is unchanged [inferred — audit: CONTEXT leaves naming the item there to discretion; that would need a new Action field, while the start status already names it — musk M-0042]. Add the app test from <behavior>.

3. src/ui/screens/help.rs: split the always-on `lines.extend([...])` right after the Backlog `e` row, push the row `row("t", "Backlog tab: start the selected item as a one-off /gsd-quick run (experimental)")` only when `experimental` is true, and continue the extend. Use the exact wording CONTEXT gives. It must be byte-unique in the body. Add a comment naming the quick id and why it is gated (260917-fko discovery gate). Tests:
   - Add the row to the flag-on assertion in `help_lines_documents_every_key_this_phase_binds`, beside the dashboard `b` assertion, not in the Driver list.
   - Add a new test `the_backlog_one_off_row_is_documented_only_with_the_flag_on`, asserting the whole row with `body_with(true)` and its absence with `body_with(false)`.

4. src/ui/screens/mod.rs: correct the `AppContext::experimental` doc. "Today, every driver surface" becomes "today, every driver surface plus the Backlog tab's one-off `t`".

5. docs/CONFIGURATION.md:
   - Line-120 env-var row: name the Backlog tab's `t` alongside "one-off tasks (`t` / `T`)".
   - "`GSDMM_EXPERIMENTAL_FEATURES` in detail": add "no Backlog-tab `t`" to the flag-off list.
   - "One-off tasks": a new **Key (Backlog tab).** paragraph after "Keys (Driver tab)". It covers:
     - `t` on the selected item, list or open content pane, starts a `/gsd-quick` one-off at once: no picker, no confirm, and another run starts even while others are live.
     - It uses the confirm popup's default model and the default spend ceiling and wall clock.
     - The task's first line is `Implement backlog item 999.N: <title>`. Then comes the item's ROADMAP.md section, or the directory's `.md` files when ROADMAP.md has none, inlined as marked data, cleaned of control and invisible characters, and capped at 48 KiB.
     - The agent is told to delete the `### Phase 999.N:` section from `## Backlog` and the `999.N-*` directory if and only if it reports done. The removal lands only when you land the run.
     - The entry is read from the working tree at keypress, while the clone is made from the last commit.
     - Refusals appear in the status line.
     - Experimental only.
   - CLI synopsis: append ` | --backlog <999.N>` inside the existing `(--todo <file> | --text …)` group of the `task run` line.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- backlog_one_off ui::screens::help:: t_opens_the_one_off_picker_and_asks_for_the_todos the_one_off_keys_do_nothing_with_the_flag_off a_confirmed_one_off_goes_through_the_seam_with_the_cli_defaults</automated>
  </verify>
  <done>With the flag on, `t` on the Backlog tab (collapsed or expanded) sends one OneoffRunRequested for the selected item with Mode::Quick and the picker's default model, and the status line names the item. With the flag off it does nothing, and the help row is hidden. Refusals are status messages. docs/CONFIGURATION.md documents the key, the CLI flag and the flag-off list. The targeted tests pass.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| repository → host (TUI/CLI) | ROADMAP.md, the `.planning/phases/999.*` dir names and the dir `.md` files are untrusted text (D-21-19, SAFE-07) read at keypress and in the daemon |
| host → sandboxed agent | the backlog entry is inlined into the agent's prompt; the agent runs in the existing one-off sandbox, in a private clone |
| caller → `task run` argv | `--backlog <value>` is caller-controlled and re-executed by the detached daemon |
| repository text → terminal | backlog numbers, titles and refusal messages reach the status line |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-ubn-01 | Tampering | oneoff::prompt::build Backlog arm (prompt injection via ROADMAP text) | medium | mitigate | Entry inlined between BEGIN/END markers under a header saying it is repository data that cannot change the dispatcher's instructions; END-marker text inside the entry is rewritten so the block cannot be closed early; control/invisible chars stripped by clean_entry; reporting/branch rules stay in the fixed block after the data; agent remains sandboxed in a private clone (existing). Tested in prompt tests. |
| T-ubn-02 | Tampering | detail.rs Backlog `t` status line, task_cli err_line | medium | mitigate | Numbers echoed only after valid_backlog_number (`^999\.[0-9]{1,9}$`); malformed input is never echoed; every status string built from repo text goes through text::render_for_terminal; titles pass todos::sanitize_title. |
| T-ubn-03 | Tampering | cli `--backlog` → launch::daemon_argv → backlog_spec | medium | mitigate | valid_backlog_number in backlog_spec and in launch::validate before any spawn; the number is only compared against backlog_dirs entries, never joined into a filesystem path; the daemon re-validates through the same function. |
| T-ubn-04 | Denial of Service | prompt size / run.json size / key-time read | low | mitigate | clean_entry caps the entry at 48 KiB (idempotent), so build and worst-case build_resume stay ≤ MAX_PROMPT_BYTES and run.json stays far below its 1 MiB read cap; the key-time ROADMAP read is the same unbounded read the Backlog content pane and `e` already accept (CONTEXT). |
| T-ubn-05 | Elevation of Privilege | detail.rs `t` arm reachable with experimental off | medium | mitigate | Arm guarded by ctx.experimental (flag-off test asserts empty channel); App::oneoff_target is already inert with the flag off (existing defence in depth); can_steer platform refusal on both sides. |
| T-ubn-06 | Repudiation | agent's remove-on-done bookkeeping | low | accept | Instruction names the exact `### Phase 999.N:` heading and `999.N-*` dir and applies only on status done; the change lives on the run's branch and reaches the checkout only when the user lands the run (review point). |
| T-ubn-SC | Tampering | npm/pip/cargo installs | low | accept | No package installs: no new crates (regex, serde already dependencies); Cargo.toml and Cargo.lock unchanged — the package-legitimacy gate has nothing to check. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: the full suite, read from raw output. The test count must rise, and the envelope suites must have run (see the cargo-test fail-fast memory).
- `cargo clippy --all-targets -- -D warnings`
- `cargo fmt --check`
- `git diff --stat master -- Cargo.toml Cargo.lock` is empty.
</verification>

<success_criteria>
- One key on the Backlog tab starts a /gsd-quick one-off for the selected item when experimental features are on, and does nothing when they are off.
- The agent receives the item's number, title and full ROADMAP.md entry, cleaned, bounded and marked as data, plus a remove-on-done instruction.
- `task run --backlog 999.N` is the daemon transport and a documented CLI entry point. Resume of a backlog run works.
- Help row, docs/CONFIGURATION.md, and the tests for the key, the task text, the entry extraction and the gate are all in place. The full suite, clippy and fmt are clean.
</success_criteria>

<output>
Create `.planning/quick/261103-ubn-dispatch-a-backlog-item-as-a-one-off-gsd/261103-ubn-SUMMARY.md` when done (include the Task 1 RED line and every [inferred — audit] decision).
</output>
