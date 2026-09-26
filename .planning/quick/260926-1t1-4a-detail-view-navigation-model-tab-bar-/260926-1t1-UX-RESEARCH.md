# UX research: waves, plans and agents, and how to navigate into sub-views

Date: 2026-09-26. This was read-only research. Line numbers are from `src/ui/screens/detail.rs` as read today, unless another file is named. Another agent is committing on master at the same time, so the numbers may drift by a few lines.

Evidence: I built the binary into the scratch target dir and ran it in tmux (`--config` pointed at a scratch config) at 100x32, 100x62 and 80x24. The "as-is" captures quoted below are real renders, not reconstructions.

Judgement calls are marked **[inferred]**.

---

## 1. The current navigation model (inventory)

### 1.1 Keys in the detail view (`DetailScreen::handle_key`, around 1917-4005)

| Key | Behaviour today | Where |
|---|---|---|
| `1`-`8` | Jump to a tab. `9` and `0` are deliberately inert. | 2743-2750 |
| `D` (experimental) | Driver tab | 2761 |
| `←` / `→` | **Switch top-level tab**. This is global, and it clamps at both ends (no wrap). | 2765-2781 |
| `j`/`↓`, `k`/`↑` | Move the selection or scroll inside the **content**, per tab. | 2106-2400 |
| `PgUp` / `PgDn` | Page the content or a pane, per tab. | 2409, 2592 |
| `Enter` / `Space` | Per tab, and very different on each: Queue = mark done; Backlog = open **and focus** the content pane; Git = load the commit pane; **Sessions = resume the session in a new terminal**; Agents = no-op; Archive and Browse = descend; Config = edit; Roadmap = jump to the phase in Phases. **Phases: unbound.** | 2783-3247 |
| `Esc` / `q` | These are the same key. Each first pops one inner level (Archive depth, Backlog pane, Git commit pane, Browse dir/file, Config dropdown or filter). Otherwise it pops the screen back to the dashboard. | 1983-2105 |
| `m` | Switches between two sub-views, only on Docs (Files/Milestones) and Sessions (Sessions/Agents). | 3285-3312 |
| `Tab` | **Switches the host terminal to a Claude/Codex session** (tmux), not a UI tab. | 3396 |
| `[` / `]` | Roadmap list only: previous/next phase in the same wave. | 3998-3999 |
| `h` / `l` | Roadmap list only: walk dependency edges. | 3992-3996 |
| `e` | Per tab: edit (Archive, Browse, Backlog pane, Queue), otherwise enqueue. | 3793 |
| `v`, `g`/`G`, `/`, `x`, `n`, `p`, `o` | Tab-specific extras | various |

### 1.2 How focus works today

- **There is no tab-bar focus.** When you land on a tab by digit or arrow, you are already "in" the content. `j`/`↓` immediately moves that tab's list. The tab bar is only an indicator.
- **Pane focus exists on one tab only, Backlog** (quick 260924-drx). `Enter` opens and focuses the content pane (side by side at ≥100 cols, `ROADMAP_SIDE_BY_SIDE_MIN_COLS`, stacked below that). `j`/`k`/`PgUp`/`PgDn` then scroll the pane, and `Enter` or `Esc` closes it (2030-2040, 2117-2131, 2825-2868, 4410-4420). While the pane is focused the footer changes to `backlog_focused_footer_spans` (7121).
- Git has a *non-focused* pane. `Enter` opens the commit pane, but `j`/`k` still move the list and close the pane (2113). Only `PgUp`/`PgDn` scroll it.
- Archive and Browse use depth. `Enter` descends and `Esc` climbs, like ranger.
- Driver deliberately has no pane focus (comment at 2255-2260).

So "Enter" means five different things: act, open a pane and focus it, open a pane without focusing it, descend, and jump to another tab.

### 1.3 Sub-tab mechanism

- `two_sub_tab_strip` (6623) draws `[Files] │ Milestones   m switch`. The active label is bracketed and shown cyan, bold and reversed.
- Docs uses it through `docs_sub_tab_row` (6645). Sessions uses it through `sessions_sub_tab_row` (6661).
- `tab_index(Agents) == tab_index(Sessions) == 5`. `sub_view_from_index(5)` always returns `Sessions`, so pressing `6` from anywhere forgets that you were last on Agents (1033).

### 1.4 Inconsistencies found

1. **The help popup documents neither the digits nor `←`/`→` as tab switching** (help.rs 200-248). The footer shows `[1-8]tabs` but never `←→` (`footer_spans`, 6996-7010).
2. **The focused Backlog pane has no visual focus cue.** Its border and title look the same whether it is focused or not (4432). Only the footer changes.
3. **The active top-level tab is shown by colour and bold only** (`Tabs::highlight_style`, 4047). A text scrape of the bar shows no marker. The sub-tab strip, by contrast, uses brackets plus reverse video. The two indicators do not match.
4. The sub-tab strip is drawn at column 0 with no gutter. The Sessions sub-view nests two full borders (4971 and 5043), which renders as `│┌ Sessions (1) ──┐│`.
5. The footer spells the same key differently: `[m] agents` / `[m] sessions` / `[m] files` (with a space) versus `[m]ilestones` (mnemonic style), at 7048-7079.
6. **The Phases tab already has a waves section, but nobody can see it.** `render_pipeline_tab` (4740-4906) draws a non-scrolling `Paragraph` in this order:
   - the ladder;
   - 5 stage lines;
   - **11 "Plan sub-stages" rows, mostly `○ … not run`**;
   - 6 "Execute sub-stages" rows;
   - up to 12 plan-token rows;
   - and only *then* `Waves (parallelism):`.

   At 100x32 the waves never render. At 100x62, phase 19's 13 waves are still cut off at `w13` of ≥13. This is why it reads as "no per-wave breakdown".
7. **The render does disk IO.** `render_pipeline_tab` reads `waves.json` with `std::fs::read_to_string` on every frame (4876-4889). That contradicts the "derive during the refresh scan, never at render time" doctrine stated in `state_reader/plan_waves.rs:10-13` and on `DiskInference.plan_waves`.
8. `Enter` on Phases does nothing, although Roadmap's `Enter` lands the user there on purpose.

### 1.5 Conflicts with the user's proposed model ("Enter or ↓ on 6:Sess, then → switches sub-tabs")

| Key | Today | Conflict |
|---|---|---|
| `Enter` on Sessions | **Resumes the selected session** by spawning a terminal (2918-3041) | This is a side effect, not "enter". It can only become "enter" if a tab-bar focus level exists. |
| `↓`/`j` on arrival | Moves the list | "↓ to enter" needs a level above the list. |
| `←`/`→` | Global tab switch | Must become sub-tab or pane switching once inside, on Sessions, Docs and Phases. |
| `Tab` / `BackTab` | `Tab` = terminal switch; `BackTab` unbound | `Tab` cannot be the sub-tab key (gitui's convention). |
| `[` / `]` | Roadmap same-wave walk | Lazygit's sub-tab keys. Usable as a sub-tab alias on every other tab, because Roadmap has no sub-tabs. |
| `m` | Docs and Sessions sub-view toggle | Keep it as an alias. |
| `Esc` = `q` | Both pop a level or the screen | A tab-bar level wants `Esc` to mean "up one level" and `q` to mean "leave". |
| Config text input | The intercept runs before the match (1954-1980) | No conflict: arrows inside a text edit never reach navigation. |

---

## 2. Conventions in established TUIs

I checked lazygit and gitui against their docs and source (links below). k9s is from its docs and write-ups. The rest (btop, htop, yazi, ranger/lf, tig, bottom) is from known behaviour **[inferred, not re-verified]**.

- **lazygit**:
  - digits `1`-`5` focus side panels;
  - **`[` / `]` = previous/next tab inside a panel**, e.g. Branches → Local / Remotes / Tags (pressing the panel digit again also cycles in newer versions);
  - `Enter` drills into an item (commit → files);
  - **`Esc` goes back one level**;
  - `0` focuses the main view;
  - the focused panel is shown with a green border.
- **gitui**:
  - `1`-`5` jump to top-level tabs;
  - **`Tab` / `Shift+Tab` cycle tabs**;
  - `←`/`→` move focus between panes (e.g. the Status tab's diff ↔ file list);
  - `Enter` drills in, `Esc` closes popups;
  - the focused pane has a highlighted border.
- **k9s**: `Enter` goes one level deeper, **`Esc` goes back one level**, and a breadcrumb shows the path. `:` jumps to a resource. The key hints for the current view are always shown.
- **btop / htop / bottom**: `←`/`→` (or `h`/`l`) move *within* a focused box. Digits toggle boxes. `Esc`/`q` go back or quit. bottom has `e` to expand the focused widget and `Esc` to collapse it.
- **ranger / lf / yazi**: `←`/`h` goes to the parent and `→`/`l`/`Enter` to the child, which is strictly hierarchical. Yazi uses `[` / `]` for its tabs.
- **tig**: `Enter` opens a child view in a split, and `q` closes the current view (one level).

**Takeaways**:

1. Horizontal keys mean "siblings at the current level". Enter/↓ go down a level and Esc goes up one.
2. `[`/`]` is the most portable "sub-tab" key pair, and `Tab` the most common "next tab" key (unavailable here).
3. The focused region is always shown by its **border and title**, not just by the footer.
4. Digits are always direct jumps.

---

## 3. Recommended navigation model

**The principle.** Arrow directions follow the layout on screen:

- the tab bar is at the top;
- sub-tabs sit in a strip under it;
- panes sit side by side below that.

`↓`/`Enter` go down a level, `↑`-at-top/`Esc` go up one, and `←`/`→` move between siblings at the level you are on.

### 3.1 Focus levels (a new `DetailFocus` enum)

| Level | Name | Examples |
|---|---|---|
| T | Tab bar | The `6:Sess` label highlighted |
| C | Tab content (with its sub-tab strip, if the tab has one) | The Sessions list, the Agents list, the Phases list |
| P | A pane inside the content | The Backlog content pane, the Phases waves pane, the Git commit pane |

Existing depth states (Archive, Browse, Config dropdown) stay as they are. They are inner levels of C.

### 3.2 Key table

| Key | At T | At C | At P |
|---|---|---|---|
| `←` `→` | Previous/next tab. Focus stays at T. | **On tabs with sub-tabs (Sess, Docs): previous/next sub-tab**, clamped at the ends [inferred: no spill-over to the next tab, so the sub-tab boundary stays visible]. **On Phases: `→` = focus the waves pane (P).** Other tabs: switch tab, as today (so no current flow gets slower), landing at T. | `←` = back to the list (C). `→` = no-op. |
| `↓` `j` | Enter C. The remembered selection is highlighted and does not move. | Move down (as today) | Scroll or move within the pane |
| `↑` `k` | No-op | Move up. **At the first row, go to T.** | Scroll up (clamped at the top) [inferred: `↑` never leaves a pane, because scrolling is its main job] |
| `Enter` | Enter C (**no action**; this fixes the Sessions resume conflict) | As today (Sessions = resume, Backlog = open pane, …). **Phases: focus the waves pane.** | Per pane (Phases: expand or collapse the wave; Backlog: close) |
| `Esc` | Pop to the dashboard | Close an open pane or depth first (as today). **Otherwise go to T** [inferred]. | Back to C |
| `q` | Pop to the dashboard | **Pop to the dashboard** (split from `Esc`) [inferred] | Pop to the dashboard |
| `1`-`8`, `D` | Jump, landing at **C** | Same | Same |
| `[` `]` | Previous/next sub-tab (alias) | Previous/next sub-tab (alias); on Roadmap, unchanged (same-wave walk) | Same |
| `m` | Hidden alias for next sub-tab: kept, documented in help only, removed from the strip and footer [inferred] | Same | Same |

Where each way of switching tabs lands you:

- **Digits land at C** [inferred]. A digit is an explicit jump, so `6` then `j` keeps working exactly as it does today, and no existing test or muscle memory breaks.
- **`←`/`→` at T stay at T**, so browsing the tab bar is fluid.
- Opening the detail screen lands at C of the remembered tab, as today.

The user's flow then works in both ways they might try:

- `6`, then `↓` (moves the list, harmlessly) or `↑` (goes to T), then `→`.
- From T: `→` to `6:Sess`, then `↓` or `Enter` (enters, no resume), then `→` (Agents).

### 3.3 Which sub-tab a digit lands on

`6` should re-open the **last-used** sub-tab of that tab, not always Sessions. Keep a per-project `last_sub_view_for_tab[5]`, and do the same for Docs [inferred]. Lazygit and gitui both remember the inner tab.

### 3.4 Focus styling (text as well as colour)

- **At T**: the active tab label is drawn `[6:Sess]`, reversed. Today it is cyan and bold only.
- **At C/P**:
  - the tab label drops back to bold with no brackets;
  - the focused region's border turns cyan and its title gets a leading `▸` (e.g. `┌▸Waves ─`);
  - an unfocused region keeps a dark-gray border.
  - Apply the same rule to the Backlog pane, which today has no cue.
- The sub-tab strip: keep the brackets and reverse video, give it the 1-cell left gutter, and replace `m switch` with `←/→ switch` (or `[ ] switch` at C on Roadmap-free tabs). At T the strip is dimmed.

### 3.5 Footer hints

The footer always shows the **current level's** keys first.

At T:

```
  [←/→]tabs  [↓/Enter]open  [1-8]jump  [Esc]back  [?]help
```

At C on Sessions:

```
  [↑]tab bar  [←/→]Sessions|Agents  [j/k]move  [Enter]resume  [Tab]switch  [n]ew  [?]help
```

At P on Phases:

```
  [←]phases  [j/k]move  [Enter]expand  [e]dit plan  [Esc]back  [?]help
```

The help popup gains a "Detail view navigation" block that documents:

- digits, `←`/`→` and `[`/`]`;
- `↑`/`↓` between levels;
- the `Esc` vs `q` split;
- `m` (listed as an alias).

### 3.6 What `m` becomes

Keep `m` as a hidden alias [inferred]. It is guarded, it is harmless, and the tests that pin it (`m_switches_sessions_between_sessions_and_agents` etc.) keep passing. Drop it from the strip and footer so users learn one model.

### 3.7 Alternatives considered

- **(a) Flattened walk with no T level.** `→` from Sessions goes to Agents, then on to Cfg. This is the cheapest option. It gives "→ switches sub-tab" but **does not fix `Enter` = resume on arrival**, and it hides the sub-tab boundary. It is acceptable as a first step (a quick task) if a phase is not wanted.
- **(b) Every arrival lands at T, digits included.** This is more uniform, but it costs an extra keypress on every digit jump and churns many tests. Rejected [inferred].

---

## 4. Information design for waves, plans and agents

### 4.1 Where each thing lives (no duplication of wave rows)

| Information | Home | Elsewhere |
|---|---|---|
| Per-wave, per-plan structure: plan ids, titles, state, tokens, current wave | **Phases tab, right pane (the new "Waves" pane)** | — |
| Live agents: state word, plan, agentType, `+commits ~dirty`, age, children, worktree-less | **Sessions › Agents** | Phases shows only a 1-cell state glyph and word per plan |
| One-line wave ribbon (`w1✓ w2▸ 3/5 w3· w4·`) | Sessions › Agents header, **replacing** its multi-row `wave_line` block [inferred] | Link hint: `waves → 2:Phases` |
| Summary ladder `P25 · w2/5 · 3 run · 8/14 done` | Dashboard Status cell (already there), plus the Agents header | — |

The rule: **Phases answers "what is the plan and where are we"; Agents answers "what is running right now".** Today's Agents wave rows (counts only) become redundant once Phases shows the same thing per plan. Collapse them to a one-line ribbon, so the Agents list gains about `min(waves, h/3)` rows of height.

**Cross-links** [inferred]:

- `Enter` on an Agents row jumps to 2:Phases with that plan's phase selected and the plan focused in the Waves pane. This is navigation, not an action on the agent, so it respects T-25-24's "observe only".
- `Enter` on a running plan in the Phases pane jumps to Sessions › Agents with that agent selected.

### 4.2 The Phases right pane, restructured

This fixes the invisible-waves problem. Top to bottom:

1. `Phase 25: Running Agents & Live Wave View` and the ladder `[D]-[R]-[P]-[E 3/7]-[V]`. The same as today.
2. **A compact stage block**, at most 2 lines:
   - `Discuss ✓  Research ✓  Plan 7  Execute 3/7  Verify –`
   - `Checks: ✓Patterns ✓Plan-Check ✓Code-Review` (only the sub-stages that were *done*, plus a `+N not run` count).

   The full 11+6 sub-stage list moves behind `s` (toggle) [inferred]. This replaces about 25 rows.
3. **The Waves pane.** It fills the rest of the height, is scrollable and focusable (P), and has its own border and title `Waves 5 · plans 3/7 done`.
4. The token table **merges into the plan rows**: an `act/est` column on each plan. The separate "Plan tokens" section and its `MAX_PLAN_TOKEN_ROWS` cap go away.

**One plan row:**

```
 ✓ 25-02  Agents scan adapter + ledger        15k/95k
```

It shows:

- a state glyph **and** word (the word appears at ≥ 90 cols of pane; the glyph alone below that, but every glyph is distinct in shape, not just colour);
- the plan id;
- the title, cut with `fit_cells` and `…`;
- tokens.

**Plan states** (the same vocabulary as `PlanState`, waves.rs:103):

| Glyph | Word | Meaning | Colour |
|---|---|---|---|
| `✓` | `done` | SUMMARY paired in main | green |
| `◐` | `unmerged` | finished in the worktree, not merged | cyan |
| `▶` | `run` | a live or idle agent is on it | green bold |
| `!` | `stall` | every agent on it is stalled | red |
| `·` | `queued` | active phase, not started | default |
| `○` | `planned` | phase not active | dim |

Glyph width: this project carries an open todo about badge glyphs being misaligned by one cell. Use the glyphs already in use in this file (`✓`, `○`, `▸`), and put the numeric columns before or after them only after a `Span::width` measure.

**One wave header row:**

```
▸ w2  2 parallel · 1 run · 1 done        ← current wave: ▸ plus bold
  w1  1 plan · done ✓                     ← collapsed when fully done
```

**Height policy** (phase 19 has 13 waves and 33 plans):

- Fully-done waves **collapse to a single header row**, and consecutive done waves merge into one row: `w1–w8  ✓ 18/18 done` [inferred].
- The **current wave** and the wave after it are expanded. Later queued waves are header rows only: `w11  1 plan · queued`.
- Once focused, `j`/`k` move a cursor over the header and plan rows. `Enter`/`Space` expand or collapse the wave under the cursor. `g`/`G` go to the top or bottom.
- If the pane still overflows, it scrolls with the cursor. The window is centred on the current wave on arrival. The pane records `ViewportMetrics` the way the Backlog pane does.
- Unfocused, the pane shows the same collapsed layout, windowed so that the current wave is visible, with `↓ +N more` at the bottom when clipped.

**Acting on a plan** (existing conventions):

- `e` opens `NN-MM-PLAN.md` in `$EDITOR`, or `NN-MM-SUMMARY.md` if the plan is done (`E` for the other one [inferred]). This is `ScreenAction::SuspendAndEdit(path, line)`, the convention from 260924-drx. The line is the `<objective>` tag, if found.
- `Enter` on a plan row opens a detail sub-pane: the objective's first lines, `depends_on`, files_modified, and the agent line if one is running. The Backlog side-by-side/stacked rule applies [inferred, optional; `e` alone is enough for a first cut].
- `depends_on` hints: show them only in the detail sub-pane, not on the row. On the row they cost width, and in a phase the wave ordering already carries that information [inferred].

### 4.3 Data this needs (all in the refresh scan, none at render)

- **Plan titles.** The first `# ` heading or the `<objective>` first line of each PLAN.md, captured in `infer_disk_status` from the same read that already extracts `wave:`. The pattern is `plan_waves.rs:8-13` / `disk_status.rs:867`. Store them as `Untrusted`. Optionally capture `depends_on` from the frontmatter at the same time.
- **Per-plan state.** `waves::derive` computes a `PlanState` per plan but only *tallies* it (waves.rs:293-301, 349-354). Expose `WaveRow.plans: Vec<(String /*stem*/, PlanState)>` so Phases can draw rows without re-deriving. For a phase that is not the `AgentView.active_phase`, the state comes from `summarized_plans` alone (done/planned).
- **Move the `waves.json` read out of `render_pipeline_tab`** (4876-4889) into the scan. This fixes inconsistency 7.
- **Escape guard.** Plan titles, and objectives in the detail sub-pane, are third-party text. Draw them through `shown()` / `Untrusted`, and add a `Phases waves (focused, expanded)` state to `ALL_SUB_VIEWS` in render_escape_guard.rs with a hostile plan title, following the Agents pattern in 25-05.

### 4.4 Phases in each lifecycle state

| Phase state | What the Waves pane shows |
|---|---|
| Executing (the `AgentView` active phase) | Live states, the current wave `▸` and bold, done waves collapsed |
| Executing, but no agents visible (e.g. the process died) | done/queued from disk; stalled leftovers marked `!`; header note `no live agents · last activity 2h` |
| Planned, not started | Every wave expanded if it fits, plans `○ planned`; header `5 waves · 7 plans · not started` |
| Completed | One collapsed row per wave, merged: `w1–w5 ✓ 7/7 done · est 750k act 94k`. Expand on focus. |
| Plans without `wave:` metadata | Flat list titled `Plans (no wave metadata)`; the `w?` bucket is shown last |
| No plans yet | Dim `No plans yet — /gsd:plan-phase 26` (`e` enqueues it, as the generic `e` already does) |
| Project not running anything | Same as above, from disk only; the Sessions › Agents ribbon reads `no active agents` |

### 4.5 The dashboard

The Status-cell ladder (normal.rs 405-430, waves.rs 457-520) already carries `w2/11 · 13 run · 8/35 done` and tiers by width. **This is enough** [inferred]. Do not add `wave 2/11` to the Phase column: it would duplicate the ladder, and the Phase column is already the one truncated first at 80 cols (as the captures show: `GITSAFE — Git & Blast-Radi`). One optional nicety: when agents run, show the Status cell's first form in bold so running projects stand out without colour.

### 4.6 Narrow terminals (80 cols)

- The Phases split stays at 40/60 (the pane is about 47 cols).
- At <100 cols, **focusing the Waves pane widens it to the full width** and hides the phase list, keeping a 1-line `‹ P25 Running Agents…` breadcrumb [inferred]. This is the analogue of the Backlog stacked rule, which uses the same 100-col breakpoint.
- The plan row drops the state word first, then the tokens. The title is cut with `…`. The id and glyph never drop.
- The tab bar already tiers (full / compact / windowed).

---

## 5. Mockups

### 5.1 Phases tab, executing phase, about 100 cols, Waves pane focused (P)

```
 Project: gmm
 1:Roadmap | [2:Phases] | 3:Backlog | 4:Git | 5:Queue | 6:Sess | 7:Cfg | 8:Docs
────────────────────────────────────────────────────────────────────────────────────────────────────
 Phases                              │ Phase 25: Running Agents & Live Wave View
  P22: Container Execution Targ  3w  │ [D]---[R]---[P]---[E 3/7]---[V]
  P23: Gate Policy & Auto-Valid      │ Discuss ✓  Research ✓  Plan 7  Execute 3/7  Verify –
  P24: Roadmap Tab Redesign & D  6w  │ Checks ✓Patterns ✓Deferred  +9 not run            [s]how all
> P25: Running Agents & Live Wa  5w  │┌▸Waves 5 · plans 3/7 done · est 750k act 52k ────────────────┐
  P26: Tab Navigation Model          ││  w1  1 plan · done                           ✓            ││
                                     ││▸ w2  2 parallel · 1 run · 1 unmerged                       ││
                                     ││> ▶ run      25-02  Agents scan adapter + led…   15k/95k    ││
                                     ││  ◐ unmerged 25-03  Wave derivation & plan at…   16k/110k   ││
                                     ││  w3  1 plan · queued                                       ││
                                     ││    · queued 25-04  AgentsScanned handler & ti…    –/120k   ││
                                     ││  w4  2 parallel · queued                                   ││
                                     ││  w5  1 plan · queued                                       ││
                                     │└────────────────────────────────────────────────────────────┘│
  [←]phases  [j/k]move  [Enter]expand  [e]dit plan  [E]summary  [Esc]back  [q]uit  [?]help
```

Notes:

- `>` is the row cursor. `▸` on the wave header marks the current wave; `▸` in the pane title marks focus.
- When the pane is not focused, its border is dark gray and its title has no `▸`. The footer then reads `[→/Enter]waves  [j/k]phase  [↑]tab bar …`.

### 5.2 Phases tab, completed phase with 13 waves, about 100 cols, unfocused

```
 Phases                              │ Phase 19: GITSAFE — Git & Blast-Radius Envelope
  P18: Driver Tab, Live Watch &  5w  │ [D]---[--]---[P]---[E 33/33]---[V]
> P19: GITSAFE — Git & Blast-Ra 13w  │ Discuss ✓  Research –  Plan 33  Execute 33/33  Verify ✓
  P20: Deterministic Decision R  4w  │ Checks ✓Security ✓Patterns ✓Plan-Check ✓Code-Review ✓UAT +3
                                     │┌ Waves 13 · plans 33/33 done · est 3.5M act 2.0M ────────────┐
                                     ││  w1–w13  ✓ 33/33 done                  [Enter] to expand    ││
                                     ││                                                             ││
                                     │└─────────────────────────────────────────────────────────────┘│
```

Expanded (after focusing and pressing `Enter`), it windows with `↓ +N more`:

```
                                     ││  w1  4 parallel · done ✓                                    ││
                                     ││    ✓ 19-01  Envelope policy types               16k/72k     ││
                                     ││    ✓ 19-09  Git version witness                  3k/48k     ││
                                     ││    …                                                        ││
                                     ││  w2  2 parallel · done ✓                                    ││
                                     ││                                        ↓ +27 more           ││
```

### 5.3 Phases tab at 80x24, pane focused (full width)

```
 Project: gmm
 1:Roadmap | [2:Phases] | 3:Backlog | 4:Git | 5:Queue | 6:Sess | 7:Cfg | 8:Docs
────────────────────────────────────────────────────────────────────────────────
 ‹ P25 Running Agents & Live Wave View   [E 3/7]  w2/5 · 1 run
┌▸Waves 5 · 3/7 done ──────────────────────────────────────────────────────────┐
│  w1  1 plan · done ✓                                                         │
│▸ w2  2 parallel · 1 run · 1 unmerged                                         │
│> ▶ 25-02  Agents scan adapter + ledger attribution             15k/95k       │
│  ◐ 25-03  Wave derivation & plan attribution                   16k/110k      │
│  w3  1 plan · queued                                                         │
│    · 25-04  AgentsScanned handler & tick wiring                  –/120k      │
│  w4  2 parallel · queued                                                     │
│  w5  1 plan · queued                                                         │
└──────────────────────────────────────────────────────────────────────────────┘
  [←]phases  [j/k]move  [Enter]expand  [e]dit  [Esc]back  [?]help
```

At 80 cols the state *word* drops and the glyph stays. Each glyph has a distinct shape.

### 5.4 Sessions tab at the tab-bar level (T), then entered (C), about 100 cols

T (arrived with `→`; the content is dimmed):

```
 Project: gmm
 1:Roadmap | 2:Phases | 3:Backlog | 4:Git | 5:Queue | [6:Sess] | 7:Cfg | 8:Docs
────────────────────────────────────────────────────────────────────────────────────────────────────
  [Sessions] │ Agents
┌ Sessions (1) ────────────────────────────────────────────────────────────────────────────────────┐
│  Claude  PID 458786  new session  active                                                         │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
  [←/→]tabs  [↓/Enter]open  [1-8]jump  [Esc]back  [?]help
```

C, after `↓`, then `→` (the Agents sub-tab):

```
 Project: gmm
 1:Roadmap | 2:Phases | 3:Backlog | 4:Git | 5:Queue | 6:Sess | 7:Cfg | 8:Docs
────────────────────────────────────────────────────────────────────────────────────────────────────
   Sessions │ [Agents]                                                   ←/→ switch
┌▸Agents  P25 · w2/5 · 2 run · 3/7 done ───────────────────────────────────────────────────────────┐
│ w1✓  ▸w2 1run 1unm  w3·  w4··  w5·                                          waves → 2:Phases    │
│> live   25-02  gsd-executor  Agents scan adapter + ledger…        +2  ~0   45s                   │
│    └ live  general-purpose  explore adapters/                                    12s             │
│  done   25-03  gsd-executor  Wave derivation & plan attribution    +4  ~0   3m                   │
│  Worktree-less (live)                                                                            │
│  live   —      Explore       search session_detector                            8s               │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
  [↑]tab bar  [←/→]Sessions|Agents  [j/k]move  [Enter]show in Phases  [?]help
```

- The one-line ribbon replaces the `min(waves, h/3)` multi-row `wave_line` block.
- `Enter` on an agent jumps to 2:Phases with its plan focused [inferred].

### 5.5 Sessions › Agents at 80x24

```
   Sessions │ [Agents]                                   ←/→
┌▸Agents  w2/5 · 2 run · 3/7 ────────────────────────────────────────────────────┐
│ w1✓ ▸w2 1run 1unm  w3· w4·· w5·                                               │
│> live  25-02 gsd-executor  Agents scan adap…  +2 ~0  45s                      │
│  done  25-03 gsd-executor  Wave derivation…   +4 ~0   3m                      │
└──────────────────────────────────────────────────────────────────────────────┘
  [↑]tabs  [←/→]sub-tab  [j/k]move  [Enter]→Phases  [?]help
```

### 5.6 Footer summary per level

```
T      : [←/→]tabs  [↓/Enter]open  [1-8]jump  [Esc]back  [?]help
C plain: [↑]tab bar  [←/→]tabs  [j/k]move  <tab-specific>  [?]help
C sub  : [↑]tab bar  [←/→]<A>|<B>  [j/k]move  <tab-specific>  [?]help
P      : [←]<list name>  [j/k]move  <pane-specific>  [Esc]back  [?]help
```

---

## 6. Scope estimate

**A phase, not a quick task.** The work touches:

- the key handler of all 8 tabs;
- a new focus state in `ProjectViewCache`/`DetailScreen`;
- footer, help, and the escape guard (a new probe state);
- a new scan field (plan titles; `waves.json` moved into the scan);
- an `AgentView` change (per-plan state);
- a rewrite of the Phases right pane with its scroll and viewport;
- trimming the Agents view;
- many existing footer, help and navigation tests.

Suggested waves:

- **W1a (navigation):**
  - T/C/P focus;
  - `←`/`→` sub-tabs, and `[`/`]` as an alias;
  - `Esc`/`q` split;
  - focus styling, footer and help;
  - `m` kept as an alias;
  - the tab remembers its last sub-tab.
- **W1b (data), in parallel with W1a, which touches different files:**
  - plan titles and `depends_on` in the scan;
  - `WaveRow.plans`;
  - the `waves.json` read moved out of the render.
- **W2 (Phases Waves pane + Agents ribbon + cross-links)**, which depends on both.

If it has to be quick-task sized: W1a without the T level (flattened `←`/`→` over sub-tabs, `[`/`]` alias, strip hint, remembered sub-tab), plus moving the existing waves section **above** the sub-stage block. That makes today's waves visible at 24-32 rows. It is roughly one quick task. The `Enter`-resumes-on-arrival conflict would stay unresolved.

## Sources

- [lazygit keybindings (docs)](https://github.com/jesseduffield/lazygit/blob/master/docs/keybindings/Keybindings_en.md)
- [lazygit: switching tabs inside a panel, discussion #1861](https://github.com/jesseduffield/lazygit/discussions/1861)
- [lazygit: cycling tabs, issue #566](https://github.com/jesseduffield/lazygit/issues/566)
- [gitui default key list (source)](https://raw.githubusercontent.com/gitui-org/gitui/master/src/keys/key_list.rs)
- [k9s navigation, KodeKloud walkthrough](https://notes.kodekloud.com/docs/Kubernetes-Troubleshooting-for-Application-Developers/Prerequisites/k9s-Walkthrough/page)
- [k9s cheatsheet](https://www.hackingnote.com/en/cheatsheets/k9s/)
