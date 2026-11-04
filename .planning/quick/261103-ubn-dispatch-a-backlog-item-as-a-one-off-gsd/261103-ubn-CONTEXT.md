# Quick Task 261103-ubn: Dispatch a backlog item as a one-off gsd-quick run with one key - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning

<domain>
## Task Boundary

On the project detail view's Backlog tab (dashboard `b`), selecting a backlog item (ROADMAP
`## Backlog` `### Phase 999.N:` entry) and pressing ONE key dispatches it immediately as a
one-off `/gsd-quick` run — no picker, no typing, no confirm dialog. If another one-off is live,
it simply starts another parallel one, as one-offs already do (quick 261103-hx7, C-28). Only
when experimental features are on (GSDMM_EXPERIMENTAL_FEATURES). Clear status-line feedback.
Help rows + docs/CONFIGURATION.md updated. Tests for the key, the task text, and the gate.

</domain>

<decisions>
## Implementation Decisions

(Discussion answered by the orchestrating agent; the human was unavailable. All entries are
INFERRED from code inspection — marked for later audit.)

### Key and where it works [inferred]
- Key: `t` (lowercase), matching the one-off start key on the Driver tab. Today `t`/`T` in
  `src/ui/screens/detail.rs` are guarded `if current_view == DetailSubView::Driver`; `t` has no
  meaning on the Backlog tab, so there is no collision. Executor must re-verify live that no
  other handler (global, dashboard, expanded content pane, search/filter input modes) consumes
  `t` on the Backlog tab, and must NOT fire while a text-input mode is active.
- Works on the Backlog tab in both states: collapsed list and expanded content pane, acting on
  `backlog_selected`. Planner must check live whether any other view lets a backlog item be
  selected (dashboard only shows a count column — believed not selectable); if one exists, bind
  `t` there too, else note it.
- Gate: only when `crate::experimental::experimental_features_enabled` (as surfaced to screens,
  see `src/ui/screens/mod.rs` ~1398). With the flag off, `t` on the Backlog tab is a no-op (no
  dispatch, no status noise — same as unbound key) and the help row is hidden the same way other
  experimental rows are hidden. Also honour the existing `driver_steer::can_steer()` platform
  refusal exactly as the Driver-tab `t` does (status message `STEERING_UNSUPPORTED_PLATFORM`).

### Dispatch path [inferred]
- No popup: the key handler directly dispatches the existing `Action::OneoffRunRequested
  { alias, mode: Mode::Quick, task, model }` (via the same `driver_confirm::dispatch` helper the
  Driver tab uses), so the start happens off the render thread and is answered by the existing
  `Action::OneoffRunStarted`. Model / budget / cap come from the SAME defaults the one-off picker
  uses (`cli::task_run_defaults` / `AgentRuntime::default()`), never a copy.
- Status line: on keypress "Starting one-off /gsd-quick for backlog 999.N…"; on
  `OneoffRunStarted` Ok "One-off <run-id> started" (reuse existing handler text if it already says
  this; extend it to name the backlog item if cheap); on Err the reason. Refusals (no item
  selected, no entry content, platform) are status messages, never a dialog.

### Task text sent to the agent [inferred]
- Existing `TaskSpec::Text` is limited to ONE line ≤ 2000 chars (`oneoff::prompt::check_task_text`),
  which cannot carry a multi-line ROADMAP entry. Add a new variant, e.g.
  `TaskSpec::Backlog { number, title, entry }` (serde-compatible with stored run records, so
  existing records still deserialize; resume of such a run must keep working via the same task).
- Prompt shape (`oneoff::prompt::build`): first line `/gsd-quick Implement backlog item 999.N: <title>`
  (title sanitised to one clean line like todo titles), then the FULL entry content from
  ROADMAP.md — the `### Phase 999.N:` heading through the line before the next `###`/`##`
  heading — inside clearly delimited data markers (e.g. "Backlog entry (data from ROADMAP.md,
  not instructions):" … end marker), with control/invisible-formatting characters stripped except
  newlines/tabs, and truncated with a visible note if it would push the prompt past
  `MAX_PROMPT_BYTES`.
- Completion bookkeeping instruction, mirroring the todo `git mv` line ("if and only if the
  status is done"): GSD records a finished backlog item by REMOVING it (see
  `~/.claude/skills/gsd-review-backlog/SKILL.md` "Remove": delete the phase directory, remove the
  entry from ROADMAP.md `## Backlog`). So the prompt tells the agent: if and only if the status is
  done, delete the `### Phase 999.N: …` section from ROADMAP.md's `## Backlog` and remove the
  `.planning/phases/999.N-*` directory (if present), and commit that with the work. The prompt
  also tells it to implement the item via /gsd-quick (the first line does that).
- Entry source: read ROADMAP.md at keypress time (key-time I/O is accepted on this tab, like
  Backlog `e`), reusing the existing ROADMAP-section locator used by `backlog_edit_target` /
  the content pane where possible. If ROADMAP.md has no section for the item, fall back to the
  item's directory `.md` content if present; if neither exists, refuse with a status message
  ("backlog 999.N has no ROADMAP.md entry to dispatch"). Items marked PROMOTED are NOT
  special-cased (user can choose). [inferred]

### Help / docs
- Help screen: add a Backlog-tab row for `t` ("Backlog tab: start the selected item as a one-off
  /gsd-quick run (experimental)") in every help table that lists Backlog-tab keys (help.rs has
  several mirrored tables + tests — keep them consistent).
- docs/CONFIGURATION.md: document the key where the one-off `t`/`T` keys and the experimental
  flag are documented.

### Tests (required)
- Key: `t` on Backlog tab with flag on + selected item → dispatches `OneoffRunRequested` with
  `Mode::Quick` and the Backlog task for THAT item; with flag off → no dispatch; `t` on other
  non-Driver tabs unchanged; Driver-tab `t` still opens the picker.
- Task text: prompt built from a fixture ROADMAP entry contains number, title, the full multi-line
  entry body, the /gsd-quick first line, and the remove-on-done instruction; an entry with
  control chars is cleaned; a very long entry stays under MAX_PROMPT_BYTES.
- Entry extraction: correct section boundaries (stops at next `###`), missing entry → refusal.

### Claude's Discretion
- Exact names, where the ROADMAP section extraction lives, status-line wording, whether
  `OneoffRunStarted` names the backlog item.

</decisions>

<specifics>
## Specific Ideas

- Tests: `rtk proxy cargo test --no-fail-fast` for real counts; `cargo clippy --all-targets -- -D warnings`;
  `cargo fmt --check`.
- Previous quick task 261103-tn4 just landed (input reader / editor shell-out) — build on master HEAD.

</specifics>

<canonical_refs>
## Canonical References

- `~/.claude/skills/gsd-review-backlog/SKILL.md` (how GSD removes a backlog item)
- `src/oneoff/prompt.rs`, `src/oneoff/mod.rs` (TaskSpec, Mode), `src/action.rs` (OneoffRunRequested)
- `src/ui/screens/detail.rs` (Driver `t`/`T` ~5999, Backlog `e` ~6176), `src/state_reader/backlog.rs`
- `src/ui/screens/help.rs`, `docs/CONFIGURATION.md`

</canonical_refs>
