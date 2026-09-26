# Roadmap phase-line shape fixtures

`v1-era-ROADMAP.md` is **synthetic**. It has no private source: every title is
neutral and no line is copied from a real project's prose. It pins the phase-line
shapes surveyed on 2026-09-26 for quick task 260926-fi9 (parse v1-era ROADMAP.md
phase shapes), observed in these GSD projects' roadmaps and milestone archives:
aiFlowAgent, cdr-configurator, picsync, ttbook, predix, daily-vow, hitchmatch,
hm-relverify, shopify-orderly-rescue, wordoclock, mailbot, usbee, sentriq,
nomosquitoz, and this repository's own `.planning/ROADMAP.md`.

It covers:

- shipped-milestone `<details>` collapses (the block GSD's `complete-milestone`
  workflow writes) holding plain checkbox lines with and without a plan tally,
  bold lines with a non-dash tail, bare `Phase NN: Name (N plans, complete)`
  lines and `### Phase N:` headings with plan items;
- the current milestone's bold-with-dash, bold-bare, bold-tagged and plain
  checkbox lines, a retired line, a `Phase 16+` placeholder, a backlog sentinel,
  and an active milestone's own (not closed) collapse;
- dash-separated detail headings (`—`, `-`, `–`);
- the negative shapes that must stay prose (a separator-less heading, bold
  prose, numbered lists, possessives, a bare line outside a closed collapse,
  a `Depends on` line, Progress and requirement table rows).

Tests read it only through `include_str!`, never from a project path. It
deliberately lives outside `tests/fixtures/roadmaps/`, so that directory's
sanitisation guard (`vendored_roadmap_fixtures_are_sanitised`) and its pinned
goal count stay unchanged.
