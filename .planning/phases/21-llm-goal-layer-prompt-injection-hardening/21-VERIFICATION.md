---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-22T18:21:16Z
head: 5b24022
status: gaps_found
score: 15/19 must-haves verified (5/5 ROADMAP success criteria)
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "pass-5 CR-01 (`execute_run` returned `Err(NoCommandSource)` AFTER committing `run.json` with `\"gsd_command\": \"\"`, a run directory and a 4-record journal): CLOSED, and closed structurally. `command_source` + `iteration_source` resolve at `src/driver/run.rs:2442-2447`; `establish_own_group` is :2449, `establish_envelope` :2493, the lock :2609, `JournalRun::start` :2626. I read the ordering off the file rather than off the SUMMARY. Both `(None, None) => String::new()` arms are GONE — `recorded_command` (:910-915) and `digested_command_fragment` (:938-943) are now total over `IterationSource`'s two variants, so there is no empty case for a fabricated value to answer. Self-measured: `grep -n '=> String::new()' src/driver/run.rs src/driver/mod.rs` -> 0 hits (exit 1); the 4 surviving tree-wide hits are off argv/record paths (I opened two of the four: `archive.rs:154` is kebab->Title display conversion, `executor/outcome.rs:485` is inside a `#[cfg(test)]` envelope builder)."
    - "pass-5 IN-01 (`--goal '   '` beside `--command` persisted verbatim into a committed `run.json` after the lock, journal and run directory existed): CLOSED. `make_run_record` writes `goal` from `args.goal.as_ref().map(NonBlank::as_str)` (`run.rs:813-817`); `unwrap_or_default()` over `Option<String>` is gone. The multi-flag cell pass 5 called unexpressible now exists as a matrix row (`--goal beside a visible --command`, `src/driver/mod.rs:2449`) and is asserted `Err(NoCommandSource)` for all six `DEGENERATE` shapes."
    - "pass-5 CR-02, the WHOLLY-invisible half: CLOSED. `is_plain_path_component` (`src/journal/mod.rs:268-294`) now delegates its blank half to `text::carries_visible_content` while KEEPING its own `is_control()` refusal and `Component::Normal` traversal check — I read all three halves. Measured against the built library: `is_plain_path_component(\"\\u{200b}\") = false`, `(\"   \") = false`; `envelope_dir_in(root, \"\\u{200b}\") = None`; `DriveArgs::from_argv(run_id = \"\\u{200b}\")` -> `Err(RunIdInvalid)`. Pass 5's reproduction — a complete run whose directory name was one invisible character — no longer runs. (The LOOK-ALIKE half of the same sentence is not closed; see gaps.)"
    - "The enumeration moved from human to compiler for `DriveArgs`. All six argv-derived string fields are `payload::NonBlank`/`Option<payload::NonBlank>` — I extracted the struct body and read every field. `DriveArgs::from_argv` (`:405-469`) destructures all twelve `RawDriveArgs` fields by name with NO `..` rest-pattern and reconstructs all twelve, so a seventh field cannot compile unclassified. `NonBlank`'s privacy was not widened: `src/driver/mod.rs:546-592`, private tuple field, exactly `new` + `as_str`, derives `Debug, Clone, PartialEq, Eq`, no `From`, no `into_inner`, no serde, no `Deref`/`AsRef`. I looked for an escape hatch and found none."
    - "pass-5 warning 1 (`ITEM_OPENERS` blind to `pub(crate)`/`pub(super)`, header bounding the gap backwards): CLOSED. `\"pub(\"` is in the list (`tests/spawn_seam_guard.rs:1993-1997`) with the 51-instance measurement recorded beside it; the backwards sentence is deleted; `the_boundary_self_check_sees_restricted_visibility_items` plants both spellings after a marker and asserts the SAME `post_marker_offenders` fn the live assertion consumes reports them. The `scanned_files >= 10` non-vacuity floor survived the extraction (:2167) and the control asserts `scanned_files == 1` (:2135)."
    - "pass-5 warning 2 (`visibly_empty_numbered_entry`'s doc credited itself with breaking the trim tautology): CLOSED and corrected honestly. The doc now says plainly that the tautology is broken by the LITERAL `DEGENERATE` array and that no degenerate payload reaches this detector at all; four direct pins exist at `src/driver/mod.rs:2189-2209` (two must-report, two must-not), so the residual claim is checkable."
    - "pass-5 warning 3 (guard eight's non-vacuity counted attributed LINES, so the 3-and-3 arithmetic stopped biting on the first added line): CLOSED. The floor now counts DISTINCT NEEDLES — each type-qualified needle must match tree-wide, and each allowlisted site must be matched by all three (`tests/spawn_seam_guard.rs:2587`, limits text :2380-2396). The replacement is named in the limits block with the reason."
    - "pass-5 warning 5 (`src/driver/goal.rs`'s length-bound refusal had zero suite coverage after 21-13 tightened the predicate): CLOSED. `tests/driver_goal_seam.rs` carries the control-free over-length fixture again with three assertions (`<= MAX + marker`, `!=` the raw token, `ends_with(TRUNCATION_MARKER)`); the binary is 21 passed, up from 20."
    - "pass-5's `coincidental_reliance_items` half about `DriveArgs`: CLOSED. `alias` and `approved_plan` no longer rest on a registry lookup's and a token parser's incidental behaviour — both are `NonBlank` and refused at the boundary with their own named variants, and `a_blank_approval_token_is_refused_by_the_real_parser` pins the derived refusal as a checked fact rather than an assumption."
    - "Process: `.planning/REQUIREMENTS.md` was not touched by either round-5 plan. `git log -- .planning/REQUIREMENTS.md` still shows `0c4f712` as the last commit; all five phase-21 requirements read `[ ]` (lines 62, 63, 83, 85, 86) and `Gaps Found` (152, 153, 164, 166, 167). Third round running with the prohibition honoured."
  gaps_remaining:
    - "Criterion 1 is VERIFIED for the first time in this phase — the class that failed it four consecutive times (an argv payload carrying no visible instruction reaching a persisted record) is closed by construction, and I attacked it and could not break it. The phase still fails, on TWO gaps that are NOT that class: (a) the identity/nameability of argv-derived aliases and run ids, where the round claimed a mitigation it provably could not deliver, and (b) the round's own two anti-recurrence mechanisms, both of which constrain materially less than their docs and their plan truths assert."
  regressions: []
gaps:
  - truth: "Exactly ONE production spelling of the invisibility judgment governs every argv-derived identity, so two visually identical aliases can no longer resolve to two different envelope paths (21-15 must_haves truths 4 and 5; threat T-21-15-02, severity high, disposition mitigate)"
    status: failed
    reason: "REPRODUCED against the built library at HEAD, in two independent halves that the round conflated. Truth 5 says the look-alike harm is closed; it is not, and the mitigation as designed could never have closed it. Truth 4 says one production spelling exists; a fourth lives on a command the unification never opened. Neither half is criterion 1's blank-payload class — this is a *naming* property, which is why criterion 1 passes and this fails."
    artifacts:
      - path: "src/registry.rs"
        issue: "REPRODUCED (21-REVIEW.md CR-01, confirmed independently at library level; the reviewer's binary-level transcript is consistent with what I measured). `add_project` (:13-20) and `add_project_unchecked` (:64-71) judge alias blankness with `alias.is_empty()` plus `alias.contains(char::is_whitespace)` — a FOURTH production predicate, on the `gsd-meta-manager add <path> <alias>` argv path, untouched by 21-15's unification. `U+200B` is neither empty nor whitespace (measured: `is_whitespace=false is_control=false`). Measured directly against the real `add_project`: registering an alias of ONE `U+200B` returns `Ok`; the key lands in `config.projects` and `list` renders it as a blank ALIAS column. This is a genuinely NEW surface — a different subcommand, not a sixth instance of the `DriveArgs` miss."
      - path: "src/journal/mod.rs"
        issue: "REPRODUCED (21-REVIEW.md WR-05), and this half is NOT a new surface — pass 5 named it verbatim in its own `coincidental_reliance_items` (\"`is_plain_path_component` ... accepts `\\u{200b}`, so two visually identical aliases resolve to two different envelope and credential paths\"), and 21-15 truth 5 claimed it closed. `is_plain_path_component` (:268-294) refuses a value that is WHOLLY invisible and accepts one with an EMBEDDED invisible, because `carries_visible_content` asks 'is any character visible?' and a look-alike has plenty. Measured: `is_plain_path_component` returns `true` for `\"demo\\u{200b}\"`, `\"de\\u{200b}mo\"` and `\"demo\\u{feff}\"`. Driven through the production seams: `add_project(\"demo\", projA)` and `add_project(\"demo\\u{200b}\", projB)` both `Ok` -> two keys, two projects; `envelope_dir_in(root, \"demo\")` -> `<root>/demo` and `envelope_dir_in(root, \"demo\\u{200b}\")` -> `<root>/demo\\u{200b}` — TWO ENVELOPE ROOTS for two aliases that render identically, which is the exact harm T-21-15-02 describes and 21-15 truth 5 declares removed. The same predicate governs `--run-id`: `DriveArgs::from_argv(run_id = \"aaaa\\u{200b}\")` -> `Ok`, and `run_paths(planning, \"…-aaaa\")` / `run_paths(planning, \"…-aaaa\\u{200b}\")` are two distinct directories. That is pass 5's own sentence — 'a persisted terminal run record no human can distinguish from a sibling run called abc vs abc\\u{200b}' — still true. Blast radius unchanged at seven call sites: `--run-id` (driver/mod.rs, journal/mod.rs:329, journal/writer.rs:491), registry aliases (main.rs, envelope/mod.rs:204, envelope/cred.rs:758) and the model-supplied phase token (goal.rs:667)."
      - path: "src/cli.rs"
        issue: "THE BOUND, stated as a fact rather than as advice. `Commands` and `EnvelopeAction` declare EIGHT argv-derived alias fields — `Add:30` (`Option<String>`), `Remove:35`, `Drive:50`, `PrePush:306`, `PreCommit:319`, `Askpass:337`, `Guard:361`, `Scan:371` — and exactly ONE of the eight (`Drive`) passes through `DriveArgs::from_argv`. The class round 5 closed is `DriveArgs`; the class that remains open is `Commands`, and `Commands` is itself a compiler-enumerable Rust enum, so the same move that worked for `DriveArgs` applies without invention."
      - path: "src/driver/mod.rs"
        issue: "CONFIRMED by reading (21-REVIEW.md WR-06), a round-attributable message regression. D-15-4 hoisted the supplied-but-blank alias refusal above the registry lookup (`from_argv:426-432`) and reuses `OptInError::UnknownAlias`, whose Display is `no project is registered under the alias `{alias}`` (`src/error.rs:328`). An alias of one `U+200B` CAN be registered (see above), so `list` shows it and `drive` says it is not registered. The refusal states a falsehood about durable state, which is the class this phase exists to close. Before the hoist the value reached the lookup, matched, and produced the truthful opt-in refusal."
    missing:
      - "Add the missing CLAUSE, not another consumer. Routing `registry::add_project` through `text::carries_visible_content` closes only the wholly-invisible alias; both look-alikes carry a visible `d` and would still register. What closes the look-alike half is a second shared judgment beside the first — `text::carries_invisible_formatting(value)` over `'\\u{200b}'..='\\u{200f}' | '\\u{2060}'..='\\u{2064}' | '\\u{feff}'` — consumed by `is_plain_path_component` (after its `is_control()` refusal) and by both registry entry points. That is one clause in one module and it closes the run-id, alias, envelope, credential and phase-token halves in the same change. Verify nothing legitimate regresses against the existing acceptance list (`\"20\"`, `\"2.1\"`, `\"2026-08-19T12-00-00Z-aaaa\"`, `\"demo\"`, `\"99\"`) — none contains a format character."
      - "Make `Commands` the domain the way 21-15 made `DriveArgs` the domain, and prove it the same way. Eight argv alias fields, one protected. The provable bound is a parse boundary over the raw clap enum with an exhaustive no-`..` destructure per variant, yielding a validated alias newtype that `add_project`, `remove_project`, `envelope_dir_in`, `askpass_with_config` and the four hook re-entry points take instead of `&str`. Then the compiler enumerates the entry points, exactly as `from_argv` now enumerates the fields — and a ninth subcommand cannot be added unclassified."
      - "Give `add` the look-alike refusal at REGISTRATION, with a message that says why (`two aliases that render identically would name two different projects, two opt-ins and two envelope roots`). Downstream refusal is not enough: `envelope_dir_in` already returns `None` for a wholly-invisible alias and the entry still sits in `config.json`, which is what produces WR-06."
      - "Give the boundary its own refusal instead of borrowing `UnknownAlias` — e.g. `DriveError::AliasNotVisible { alias }` — so a refusal about a value that carries no name stops asserting a falsehood about the registry's contents. Entries already in a user's config need this even after registration is closed."
      - "Pin the look-alike pair (`\"demo\"`, `\"demo\\u{200b}\"`) as a two-value fixture in the journal's both-directions test and in a registry test, since `DEGENERATE`'s six shapes are all WHOLLY invisible and structurally cannot express this case — the same shape-of-fixture blindness that let the round's matrix pass while the harm stayed open."
  - truth: "The two anti-recurrence mechanisms round 5 built constrain what their docs and their plan truths say they constrain (21-16 must_haves truths 1 and 4; 21-16 prohibition 1)"
    status: failed
    reason: "Both mechanisms are materially weaker than documented, both reproduced by me in standalone programs rather than taken from 21-REVIEW.md, and 21-16-SUMMARY.md's own prohibition-audit table certifies prohibition 1 as PASS while naming the bound I measured not to bind. This is the FOURTH consecutive round in which a newly built guard's header overclaims its reach, and the SECOND consecutive round in which a plan ships a violation of a prohibition it wrote itself. It matters more than the individual Critical: these are the artifacts a sixth reader will trust instead of re-deriving."
    artifacts:
      - path: "src/driver/mod.rs"
        issue: "REPRODUCED (21-REVIEW.md WR-01) with a standalone `rustc` program mirroring the shipped shape. `one_of_each() -> [(&'static str, CommandSource); 3]` (:2078-2084) is an array literal of three constructed values. Constructing values of an enum carries NO exhaustiveness obligation, and an array of declared length 3 holding 3 entries constrains nothing about the enum's arity. Its doc (:2067-2072) says a fourth variant 'is a compile error HERE too, in two ways: the array's declared length and the missing construction', and 21-16 truth 4 says 'a compile error in TWO places (`variant_name`, `one_of_each`)'. Neither is true; only `variant_name`'s wildcard-free match errors. Measured output with a fourth variant `Resumed` added and `variant_name`'s single arm fixed: `all_variant_names_matches_the_variant_set_in_both_directions: PASSES` — `built.len() == ALL_VARIANT_NAMES.len() == 3`, the sorted name sets are equal, and the matrix's coverage assertion sweeps three of four. That is pass-5 warning 4 verbatim, recurring INSIDE the test written to close it. The only residual signal is a `dead_code` warning, which appears solely while the new variant is constructed nowhere in production — i.e. it disappears the moment the variant is real."
      - path: "tests/spawn_seam_guard.rs"
        issue: "REPRODUCED (21-REVIEW.md WR-02). I extracted `raw_string_argv_fields`, `drive_args_body` and `names_bare_string` VERBATIM from :2727-2808 into a scratch binary and fed it planted declarations. Nine of nine rows match the reviewer's table: `pub goal_file: String,` and `pub goal_file: Option<String>,` reported; `pub(crate)`, `pub(super)`, `Option<Box<str>>`, `Option<Cow<'static, str>>`, `Option<&'static str>`, `Option<OsString>` and a LAST field carrying a trailing `// comment` all SILENT. A mid-struct trailing comment is worse than silent — it reports the field but swallows the next declaration line into the joined text (`\"pub goal_file: Option<String>, // seventh pub dry_run: bool,\"`), so attribution is wrong there too. THREE distinct problems: (1) `!trimmed.starts_with(\"pub \")` at :2743 cannot see `pub(` — the EXACT spelling `ITEM_OPENERS` was widened for in commit `2eefa73`, four commits later, in the SAME FILE, with a comment calling it 'the tree's DOMINANT restricted-visibility style, 51 items at column zero'. One guard in this file learned the lesson this round; the guard added in the same round did not. (2) `names_bare_string` only knows the token `String`; `Box<str>`, `Cow<str>` and `&'static str` all hold `\"\"`, and `OsString` is not hypothetical — `DriveArgs` ALREADY carries `claude_args: Vec<OsString>` and `OsString` is the type argv actually arrives in. None of the four is named in the limits block. (3) The floor provably does not bind: fed a full twelve-field `DriveArgs` body plus one extra `pub(crate) goal_file: Option<String>`, I measured `offenders=[] field_lines=12 protected=6 -> PASSES (silent)`. 21-16 truth 1 asserts that the declaration-style gap is 'bounded by the >= 10-field / >= 6-`NonBlank` non-vacuity floor'; it is not."
      - path: ".planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-16-SUMMARY.md"
        issue: "CONFIRMED by reading against my own measurements. The prohibition audit (:251-256) records prohibition 1 ('MUST NOT let any guard header claim more than its scan performs') as PASS, and the detail table (:264) states guard nine's limit 2 is 'Bounded by the `>= 10` field-lines and `>= 6` NonBlank floors inside the live assertion'. I measured that bound not binding. The audit is a genuine process improvement over round 4 — round 4 wrote prohibitions and never checked them — but a self-audit that certifies a bound nobody measured is the same failure one layer up. The one useful contrast is 21-15-SUMMARY.md residual 1, which DISCLOSED its own prohibition-2 scope gap and asked for adjudication; that is the register the guard-nine audit should have been written in."
      - path: "tests/spawn_seam_guard.rs"
        issue: "CONFIRMED by grep (21-REVIEW.md WR-04), and materially softer than the review grades it. `the_degenerate_payload_set_is_spelled_in_exactly_one_place` (:3067-3097) scans `src/` only via `source_files()`, while three hand-picked blank subsets live in `tests/`: `driver_dry_run.rs:611` `[\"\", \"   \"]` (2 of 6), `driver_dry_run.rs:704` `[\"   \", \"\\t\", \"\\n  \\n\", \"\"]` (4 of 6) and `driver_goal_seam.rs:335` `[\"\", \"   \\t \", \"\\u{200b}\", \"\\u{feff}\"]` (a DIFFERENT 4 of 6). Its failure message claims tree-wide uniqueness. BUT: `src/lib.rs:19-21` gates `pub mod test_support` behind `#[cfg(test)]`, so an integration crate genuinely cannot reach `DEGENERATE`, the executor disclosed exactly this in place at `driver_dry_run.rs:602-610` and in `deferred-items.md` BEFORE the review raised it, and the exhaustive 7x6 sweep does live in-crate and passes. This is a message that overclaims its scan's scope, not a hole. The one-word fix (drop the `#[cfg(test)]`) is available."
      - path: "src/driver/mod.rs"
        issue: "CONFIRMED by reading and by direct measurement (21-REVIEW.md IN-01, which I grade Warning rather than Info). `one_visible_character_is_accepted_in_every_argv_position` (:2560-2580) narrows `--alias` and `--run-id` to `&[\"x\"]` on the reasoning that both 'are composed into path components downstream, where `is_plain_path_component` answers a stricter structural question that is not this test's subject'. The function under test is `DriveArgs::from_argv`, which never calls `is_plain_path_component` for either field — `NonBlank::new` is the only judge. I measured the rationale false: `from_argv(run_id = \"aaaa\\u{200b}\")` -> `Ok`. So the exemption costs two of seven columns of coverage for a reason that does not bear on the assertion — structurally the cycle-3 defect ('the matrix hand-exempted a column on a comment true for 1 of 4 payloads') recurring in the acceptance matrix. It cannot hide a refusal failure, which is why it is a Warning and not a Blocker."
    missing:
      - "Stop `one_of_each` claiming a compile error it does not produce. Either delete the claim from the doc and from the plan truth, or make the pairing exhaustive by DESTRUCTURING (a `named(source)` helper whose wildcard-free `match` errors on a fourth variant) — noting honestly that even that forces classification, not growth of the collection. The only real anchor is a `const ARITY` derived from a wildcard-free match, or accepting the residual and naming it."
      - "Guard nine, three small fixes with controls: `(trimmed.starts_with(\"pub \") || trimmed.starts_with(\"pub(\"))` at :2743 AND at the floor's own filter :2921; widen `names_bare_string` to `String`/`str` with `OsStr` handled by field NAME rather than by type exemption; strip a trailing `// …` before the `ends_with(',')` test and flush `pending` when the body ends. Then add control arms for `pub(crate)`, `Option<Box<str>>` and the trailing-comment shape beside the planted-field control at :2817 — each new claim a measured fact, which is the standard this file's other guards already meet."
      - "Rewrite guard nine's limits block to name the four silent type spellings and the visibility spelling, and to say what the floor actually bounds. As shipped, limit 2 cites a floor that does not bind for a single added field, and `Box<str>`/`Cow<str>`/`&'static str`/`OsString` are unnamed."
      - "Correct 21-16 truth 4 and truth 1 in the record. A verification that accepted them would hand round 6 two mechanisms it believes are compile-enforced and are not — which is exactly how the previous five rounds each inherited one wrong assumption."
      - "Delete the `--alias`/`--run-id` narrowing at `src/driver/mod.rs:2563-2566` and sweep all seven columns with all four padded payloads."
      - "Either drop `#[cfg(test)]` from `pub mod test_support` (one const, no runtime cost) and have the three `tests/` pins consume `DEGENERATE`, or narrow the guard's name and message to `src/` and give it a limits block. A guard that reports green about a region it never read is the mechanism this round exists to remove."
deferred:
  - truth: "`registry::current_prompt_inputs` absent from `tests/async_blocking_guard.rs`'s BLOCKING_HELPERS"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4 and 5)"
    evidence: "Async-hygiene class, not criterion 1's class; the fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run`."
  - truth: "The spawn-gate plan-half argument lives in a comment rather than a checked property"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "The comment states plainly that the plan half is a no-op and why that is sound; converting a sound, honestly-documented argument into a checked property is hardening, not gap closure."
  - truth: "Dead `PlanStep::rationale` field in src/driver/goal.rs"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "No production reader; cosmetic dead field with no security or honesty bearing."
  - truth: "`tests/driver_reattach.rs` and `tests/envelope_tracer.rs` flake under parallel execution"
    addressed_in: "Next phase backlog (deferred-items.md)"
    evidence: "Documented pre-existing flakes (a write-then-exec `ExecutableFileBusy` race and a run-record NotFound); confirmed against the unmodified base in round 4 and green in the orchestrator's round-5 full run. Not a round-5 regression."
  - truth: "The degenerate matrix's ROW table stays hand-maintained (`positions()`)"
    addressed_in: "Disclosed residual, not a gap (deferred-items.md 'OUT — by design')"
    evidence: "`from_argv`'s exhaustive destructure bounds the TYPE of a seventh argv field; neither it nor guard nine forces a matrix row. A correctly-typed seventh field with no row leaves the matrix at 7x6 silently. That is a coverage residual, not a blank-payload route, and 21-15 truth 6 discloses it in those words rather than claiming closure. I verified the disclosure is accurate."
behavior_unverified_items: []
coincidental_reliance_items:
  - truth: "A seventh argv field cannot be added to `DriveArgs` in a restricted-visibility spelling"
    reason: fixture-only
    harden: "Guard nine is silent on `pub(crate)`/`pub(super)` (measured). What actually prevents such a field today is that THIRTEEN integration-test files construct `DriveArgs { … }` literally (`driver_dry_run`, `driver_escalation_cap`, `driver_goal_seam`, `driver_inbox`, `driver_iteration_loop`, `driver_lock`, `driver_optin`, `driver_rate_limit`, `driver_refusal_record`, `driver_tracer`, `envelope_wiring`, `journal_run_paths`, `spawn_seam_guard`), so a non-`pub` field breaks them at compile time. That is an accident of where the fixtures live, not a property of the design, and it evaporates the day those fixtures move in-crate or adopt a constructor. Fix guard nine's opener test rather than resting on it."
  - truth: "`--target-phase` cannot carry an embedded invisible character into a phase-disk-status lookup"
    reason: undeclared-precondition
    harden: "`target_phase` is `NonBlank` (blank refused) and is separately checked with `is_plain_path_component` at the seam — but that predicate accepts embedded zero-width (measured). What keeps a look-alike phase token harmless is roadmap membership: `src/driver/goal.rs:704` refuses a token the roadmap does not declare, and no roadmap declares one with a `U+200B`. That is a property of today's roadmap contents, not of the value's validation. It is why criterion 5 is unaffected by the gap above, and it should be stated as a precondition rather than relied on silently."
human_verification: []
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-22T18:21:16Z
**HEAD:** `5b24022`
**Status:** gaps_found
**Re-verification:** Yes — sixth verification pass, after the fifth gap-closure cycle

## Goal Achievement

**Criterion 1 passes for the first time in this phase.** After four consecutive
cycles failing on the same class — an argv payload carrying no visible
instruction reaching a persisted record that is supposed to be evidence — I
attacked that class from every angle I could construct and could not break it.
That is the headline, and it is real.

**The phase still fails**, on two gaps that are *not* that class. Separating
them is the whole point of this report, because the previous five rounds each
inherited a wrong assumption from the round before, and the two things most
likely to be inherited wrong this time are (a) that "the fifth Critical is a
brand-new surface" and (b) that "the enumeration is now owned by the compiler."
Neither is fully true.

| Cycle | What closed | What the SAME cycle's diff left open |
|---|---|---|
| 1 → 2 | round-2 CR-01/CR-02, WR-01, WR-05 | blank `--command`; approval token parsed after a live consultation |
| 2 → 3 | both | blank `--target-phase` (the `Routed` arm) |
| 3 → 4 | the `Routed` arm, the matrix exemption, the trim tautology | blank `--run-id`, blank `--goal` beside a source, blank `gsd_command` on disk |
| 4 → 5 | `NonBlank` held; scope was 3 of `DriveArgs`' 6 fields | the other three fields |
| **5 → 6** | **all three pass-5 Criticals, four of five pass-5 warnings, the `DriveArgs` domain** | **the look-alike (embedded-invisible) half of the SAME sentence pass 5 wrote; a fourth predicate on a different subcommand; both new anti-recurrence mechanisms overclaiming** |

### Judgment 1 — Did round 5 close what it was scoped to close? Mostly yes, and the exception is not what the review says it is.

**Yes, and precisely:**

- **pass-5 CR-01** (`execute_run` corrupting `run.json` before refusing) is closed
  *structurally*, not by reordering. The source resolves once at `run.rs:2442-2447`;
  `establish_own_group` (:2449), the envelope (:2493), the lock (:2609) and
  `JournalRun::start` (:2626) are all below it. Both `(None, None) => String::new()`
  arms are **deleted as unrepresentable** — `recorded_command` and
  `digested_command_fragment` are total over `IterationSource`'s two variants, so
  there is no empty case for a fabricated value to answer. Self-measured:
  `grep -n '=> String::new()' src/driver/run.rs src/driver/mod.rs` → 0 hits.
- **pass-5 IN-01** (`--goal '   '` beside `--command` persisted) is closed:
  `RunRecord.goal` is written from `Option<NonBlank>` and the multi-flag cell that
  pass 5 called *unexpressible* is now a matrix row asserted `Err` for all six shapes.
- **pass-5 CR-02's wholly-invisible half** is closed and I re-measured it:
  `--run-id '\u{200b}'` → `Err(RunIdInvalid)`; `envelope_dir_in(root, "\u{200b}")` → `None`.
- **Four of five pass-5 warnings** are closed, and two of them are closed *better
  than asked*: `ITEM_OPENERS` gained `"pub("` **plus** a planted-offender control
  calling the same extracted fn, and guard eight's line count became a per-needle
  floor with the reason recorded in its limits block.
- **The compiler-owned enumeration is real *for `DriveArgs`*.** All six argv string
  fields are `NonBlank`; `from_argv` destructures all twelve raw fields by name with
  **no `..`** and reconstructs all twelve; `NonBlank`'s privacy was not widened
  (private tuple field, exactly `new`/`as_str`, no `From`, `into_inner`, serde,
  `Deref` or `AsRef`). I looked for an escape hatch and found none.

**The exception, stated more precisely than 21-REVIEW.md states it.** The review
frames CR-01 as one finding on a new surface. It is **two findings**, and only
one of them is new:

1. **`registry.rs`'s fourth predicate is genuinely a new surface.** `add_project`
   judges alias blankness with `is_empty()` + `contains(char::is_whitespace)`, on
   the `add` subcommand, which nobody had named in five rounds. An alias of one
   `U+200B` registers.
2. **The look-alike half is NOT a new surface. It is the fifth instance of the
   same signature.** Pass 5 wrote, in its own `coincidental_reliance_items`:
   *"`is_plain_path_component` … accepts `\u{200b}`, so two visually identical
   aliases resolve to two different envelope and credential paths."* 21-15 truth 5
   answered that sentence by claiming it closed. It did not. `carries_visible_content`
   asks *"is any character visible?"*, so it refuses `"\u{200b}"` and accepts
   `"demo\u{200b}"`. **The unification closed exactly the shapes pass 5 had
   *reproduced*, and left the shape pass 5 had only *named* — one shape short of
   the class, for the fifth time.** `DEGENERATE`'s six payloads are all *wholly*
   invisible, so the 7×6 matrix structurally cannot express the look-alike case and
   went green over it.

That distinction determines round 6's shape. The `add`-path predicate says
"extend the domain to every argv entry point." The look-alike half says something
else and more uncomfortable: **the mechanism is right, the domain is now right,
and the *predicate's content* was still chosen by hand from the reproduced
instances.** Both need fixing, and they are different fixes — one is a parse
boundary, the other is a single missing clause.

### Judgment 2 — Are WR-01 and WR-02 genuinely weaker than their docs claim? Yes, both, reproduced independently. This is the more important finding.

I did not take either on the reviewer's word.

**`one_of_each` constrains nothing.** I built a standalone `rustc` program with a
fourth `CommandSource` variant and `variant_name`'s single forced arm added,
leaving `one_of_each` exactly as shipped. Output:

```
all_variant_names_matches_the_variant_set_in_both_directions: PASSES
  ...with a FOURTH variant `Resumed` constructed nowhere and swept nowhere.
```

An array literal of declared length 3 holding 3 entries has no relationship to an
enum's arity; *constructing* values carries no exhaustiveness obligation. The fn's
doc claims a compile error "in two ways"; 21-16 truth 4 claims it "in TWO places."
Both are false. **This is pass-5 warning 4 recurring inside the test written to
close it** — and the residual `dead_code` warning is no safety net, because it
appears only while the new variant is constructed nowhere in production, i.e. it
vanishes the moment the variant is real.

**Guard nine is blind to spellings `DriveArgs` already uses.** I extracted
`raw_string_argv_fields`, `drive_args_body` and `names_bare_string` verbatim into
a scratch binary and fed them planted declarations:

| Planted field | Guard nine |
|---|---|
| `pub goal_file: String,` | reported |
| `pub goal_file: Option<String>,` | reported |
| `pub(crate) goal_file: Option<String>,` | **SILENT** |
| `pub(super) goal_file: Option<String>,` | **SILENT** |
| `pub goal_file: Option<Box<str>>,` | **SILENT** |
| `pub goal_file: Option<Cow<'static, str>>,` | **SILENT** |
| `pub goal_file: Option<&'static str>,` | **SILENT** |
| `pub goal_file: Option<OsString>,` | **SILENT** |
| `pub goal_file: Option<String>, // seventh` (last field) | **SILENT** |
| same, mid-struct | reported, but **swallows the next field line** |

```
FLOOR: offenders=[] field_lines=12 protected=6 -> verdict: PASSES (silent)
```

Three things make this worse than a normal token-list gap:

- `pub(` is the **exact spelling `ITEM_OPENERS` was widened for in commit `2eefa73`,
  four commits later, in the same file**, with a comment recording 51 column-zero
  instances as "the tree's DOMINANT restricted-visibility style." One guard in this
  file learned the lesson this round; the guard added in the same round did not.
- `OsString` is not hypothetical — **`DriveArgs` already carries
  `claude_args: Vec<OsString>`**, and `OsString` is the type argv actually arrives in.
- 21-16 truth 1 asserts the declaration-style gap is "bounded by the ≥ 10-field /
  ≥ 6-`NonBlank` non-vacuity floor," and `21-16-SUMMARY.md:264` repeats that in a
  table certifying prohibition 1 as PASS. **I measured that bound not binding.**

**So yes — the round's structural claims are overstated, and I agree that matters
more than the individual Critical.** With one important qualification that keeps
this honest: the *compile-side* half of the `DriveArgs` bound is genuinely real. I
read `from_argv`'s destructure and confirmed there is no `..`, so a seventh argv
field cannot compile unclassified regardless of guard nine. What guard nine adds
— *and what it does not currently deliver* — is the assurance that the
classification uses a payload TYPE. A seventh field spelled
`pub(crate) goal_file: Option<Box<str>>` would be handled by the destructure and
completely unprotected, silently. And what prevents that today is not guard nine
but the accident that thirteen integration-test crates build `DriveArgs { … }`
literally, which is coincidental reliance of exactly the kind pass 5 flagged.

### Judgment 3 — Five cycles, five new Criticals. The surface has moved, and it is now provably boundable.

Rounds 2–4 each missed a sibling *arm* in the same function. Round 5 missed a
sibling *field*. Round 6's finding splits in two, and the split is the answer:

**(a) A different command path — and this one IS bounded by the compiler already,
if anyone reads it.** `src/cli.rs` declares **eight** argv-derived alias fields
across `Commands` and `EnvelopeAction`: `Add:30`, `Remove:35`, `Drive:50`,
`PrePush:306`, `PreCommit:319`, `Askpass:337`, `Guard:361`, `Scan:371`.
**Exactly one of the eight** — `Drive` — passes through `DriveArgs::from_argv`.
So the remaining work is *not* an open-ended "harden every entry point"; it is a
finite eight-row table on a Rust enum the compiler can enumerate. The provable
move is the same one 21-15 already proved works: a parse boundary over the raw
clap enum with an exhaustive per-variant destructure and **no `..`**, yielding a
validated alias newtype that `add_project`, `remove_project`, `envelope_dir_in`,
`askpass_with_config` and the four hook re-entry points take instead of `&str`.
A ninth subcommand then cannot be added unclassified, exactly as a seventh
`DriveArgs` field cannot. **That is the mechanism extending, and it will work.**

**(b) Something structural is still wrong, and it is not the domain — it is how
the predicate's *content* gets chosen.** Every round has fixed exactly the shapes
the previous round *reproduced*, and every round the class contained one more
shape that had only been *named*. Round 5 is the cleanest example yet: pass 5's
report contains the sentence describing the look-alike harm, 21-15 wrote a truth
claiming to close it, the fix could not close it, the matrix could not express it,
and the SUMMARY's coverage table recorded it as closed. `DEGENERATE`'s six values
are all wholly invisible; there is no fixture shape in the tree that pairs two
values which differ in bytes and agree on screen.

The structural fix is small and specific: **`DEGENERATE` is an enumeration of
"carries nothing," and the tree has no enumeration of "carries something
different from what it renders."** Add `LOOK_ALIKE_PAIRS: [(&str, &str); N]` beside
it (`("demo", "demo\u{200b}")`, `("abc", "a\u{200b}bc")`, `("x", "x\u{feff}")`), give
`text::carries_invisible_formatting` the clause, and make every seam that turns a
value into an *identity* — path component, registry key, envelope root, credential
scope — assert that no two members of a pair resolve to different things. That is
one const, one predicate clause and one property, and it covers the run-id, alias,
envelope, credential and phase-token halves at once.

Round 6 should do **both**: extend the mechanism to `Commands` (bounded, eight
rows, provable) *and* close the predicate-content gap (one clause, one fixture
shape). Neither alone finishes the phase.

### Observable Truths

#### ROADMAP success criteria (the contract)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✓ **VERIFIED (first time in this phase)** | The class that failed this four times is closed by construction and I could not break it. `DriveArgs::from_argv` is **pure** (opens no file, starts no process) and runs in `main.rs` **before `drive` is entered**, so it cannot consult `dry_run` — preview/real symmetry is a construction-time fact, not two branches that must agree. All six argv string fields are `NonBlank`; the in-crate matrix drives **7 argv positions × 6 `DEGENERATE` payloads = 42 cells** through the production boundary with **no `Ok` branch and no exemption**, each asserting that position's own named variant, plus a realistic non-vacuity control per column. `execute_run` resolves its source at `run.rs:2442-2447`, above `establish_own_group` (:2449), the envelope (:2493), the lock (:2609) and `JournalRun::start` (:2626); `a_run_with_no_command_source_writes_nothing_before_refusing` pins it. `RunRecord.goal` comes from `Option<NonBlank>` and `gsd_command` from a total function over `IterationSource`, so `""` on disk provably means *absent* (D-30). Self-run: `cargo test --lib` **1031 passed / 0 failed**; `driver_dry_run` 15/15. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | Self-run `driver_goal_seam` **21 passed / 0 failed** (up from 20 — the restored length-bound fixture). `recheck_approval` sits at `run.rs:2410-2415`, above the source resolution and therefore above every write. Decomposition → plan-digest → approval → recheck-at-spawn unchanged and green. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | Self-run `driver_escalation_cap` **8 passed / 0 failed** — both directions (a cap at or above the *resolved* step cap refused at the seam; a cap below it accepted) and the typed park reason on the journal. Guard six's marker-to-EOF blind region is still asserted empty, now by openers that can actually see the tree's 51 `pub(crate)`/`pub(super)` items. `clippy --all-targets` reports **4** warnings, all pre-existing (`src/browser.rs:131-133`, `src/project_creator.rs:146`); `items_after_test_module` remains cleared. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | ✓ VERIFIED (reconfirmed) | Self-run `driver_injection_corpus` **12 passed / 10 ignored**, arrival-before-influence assertions intact. Guard seven (`FIELD_OBSERVED_MARKERS` named only where the schema declares it) passes inside my own `spawn_seam_guard` run (32/32). |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED (reconfirmed) | Self-run `driver_refusal_record` **9/9**, `driver_model_seam` **4 passed / 3 ignored**. I checked whether this pass's gap reaches this seam and it does **not**: a look-alike phase token survives `is_plain_path_component`, but `src/driver/goal.rs:704`'s roadmap-membership check refuses it. That is recorded as coincidental reliance (a fact about today's roadmap contents), not as a defect here. |

**ROADMAP score: 5/5.**

#### Round-5 plan must-have truths (what the round contracted to deliver)

| # | Truth (source) | Status | Evidence |
|---|---|---|---|
| 6 | `DriveArgs` is the domain — all six argv string fields `NonBlank` (21-15 t1) | ✓ VERIFIED | Struct body read field by field: `alias`, `command`, `target_phase`, `approved_plan`, `run_id`, `goal` all `NonBlank`/`Option<NonBlank>`. |
| 7 | A blank in ANY of the six positions is refused at `from_argv` before `drive` (21-15 t2) | ✓ VERIFIED | 7 positions × 6 payloads, each with its own named variant (`NoCommandSource`, `RunIdInvalid`, `PlanApprovalMalformed`, `UnknownAlias`); the `--approved-plan` refusal is *derived* by running the real parser and that derivation is itself pinned. |
| 8 | `execute_run` resolves once above every write; the record builders are total (21-15 t3) | ✓ VERIFIED | Line ordering read directly; both `String::new()` arms gone; `iteration_source` has three arms and `Goal(_) => Err(NoCommandSource)` — no `_`, no manufactured value. |
| 9 | Exactly ONE production spelling of the invisibility judgment (21-15 t4) | ✗ **FAILED** | `src/registry.rs:14-20` and `:65-71` are a fourth, weaker judge on the `add` path. Reproduced. |
| 10 | `is_plain_path_component` refuses the invisible; the same predicate governs registry aliases; two visually identical aliases can no longer resolve to two different envelope paths (21-15 t5) | ✗ **FAILED** | Wholly-invisible half closed. Look-alike half reproduced end to end: two envelope roots, two projects, two run directories. |
| 11 | The matrix is 7 positions × the full shared `DEGENERATE`, uniform, no exemption (21-15 t6) | ✓ VERIFIED | Read the loop body: one nested loop, no `Ok` arm, no exemption comment. (The *acceptance* matrix does carry a hand-exemption — Warning, below.) |
| 12 | A blank `goal` can no longer be persisted (21-15 t7) | ✓ VERIFIED | `run.rs:813-817`. |
| 13 | **backstop** — no `run.json` carries `goal`/`gsd_command`/`run_id`/`target_phase` as a non-empty value without visible content, and `gsd_command` is never `""` (21-15) | ✓ VERIFIED | Behavioural evidence, not presence: `a_run_with_no_command_source_writes_nothing_before_refusing` passes (nothing on disk after a refusal) and the 42-cell boundary matrix passes. `gsd_command` cannot be `""` because its producer is total over two variants. A look-alike `run_id` **does** carry visible content, so this truth as worded is not falsified by gap 1. |
| 14 | Guard nine reads the TYPE; its silent gaps are named in its limits block and bounded by the ≥ 10 / ≥ 6 floor (21-16 t1) | ✗ **FAILED** | Floor measured non-binding; five spellings silent, four of them unnamed. |
| 15 | `ITEM_OPENERS` sees `pub(crate)`/`pub(super)`; the backwards bound deleted; a control proves it (21-16 t2) | ✓ VERIFIED | `"pub("` present at `:1993-1997` with the 51-instance measurement; `the_boundary_self_check_sees_restricted_visibility_items` passes; `scanned_files >= 10` floor survived the extraction (:2167) with the control asserting `== 1` (:2135). |
| 16 | Guard eight's non-vacuity counts DISTINCT NEEDLES (21-16 t3) | ✓ VERIFIED | Per-needle loop at `:2587`; limits text records the replacement and why the line count stopped biting. |
| 17 | `one_of_each` makes a fourth variant a compile error in TWO places (21-16 t4) | ✗ **FAILED** | Reproduced with standalone `rustc`: zero compile errors, pin passes, variant unswept. |
| 18 | `visibly_empty_numbered_entry` earns its doc; direct pins make it falsifiable (21-16 t5) | ✓ VERIFIED | Doc corrected to credit the literal array; four direct pins at `:2189-2209`. |
| 19 | `goal.rs`'s length-bound refusal has live coverage again (21-16 t6) | ✓ VERIFIED | Over-length control-free fixture restored with the strengthened `!=` and `ends_with(TRUNCATION_MARKER)` assertions; `driver_goal_seam` 21/21. |

*(21-16's four probe-authored truths restate ROADMAP criteria 3 and 4 and are
scored there rather than double-counted.)*

**Score: 15/19 must-haves verified (5/5 ROADMAP success criteria); 0 present-but-behavior-unverified.**

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/mod.rs` (`DriveArgs`) | All six argv string fields `NonBlank` | ✓ VERIFIED | Read field by field; 0 bare `String`/`Option<String>` in the body. |
| `src/driver/mod.rs` (`from_argv`) | One parse boundary, exhaustive destructure, no `..` | ✓ VERIFIED | `:409-424` destructures twelve fields by name; `:454-469` reconstructs twelve. Pure — no file, no process. |
| `src/driver/mod.rs` (`mod payload` / `NonBlank`) | Privacy not widened | ✓ VERIFIED | `:546-592`, private tuple field, exactly `new`/`as_str`, no `From`/`into_inner`/serde/`Deref`/`AsRef`. |
| `src/driver/mod.rs` (7×6 refusal matrix) | Uniform, exemption-free | ✓ VERIFIED | 7 positions incl. the multi-flag row; no `Ok` branch. |
| `src/driver/mod.rs` (padded acceptance matrix) | All seven columns swept | ⚠️ HAND-EXEMPTED | `:2563-2566` narrows `--alias`/`--run-id` to `["x"]` on a rationale about `is_plain_path_component`, which `from_argv` never calls for either field. Measured false. |
| `src/driver/mod.rs` (`one_of_each`) | Second compile-time anchor for `ALL_VARIANT_NAMES` | ✗ CONSTRAINS NOTHING | Reproduced: no compile error, pin green with a variant unswept. |
| `src/driver/run.rs` (`execute_run`) | Source resolved above every write | ✓ VERIFIED | :2442 < :2449 < :2493 < :2609 < :2626. |
| `src/driver/run.rs` (`recorded_command`, `digested_command_fragment`) | Total over `IterationSource`, no manufactured blank | ✓ VERIFIED | :910-915, :938-943; 0 `=> String::new()` in either driver file. |
| `src/driver/run.rs` (`make_run_record.goal`) | Written from `Option<NonBlank>` | ✓ VERIFIED | :813-817. |
| `src/text.rs` (`carries_visible_content`) | The one production visibility predicate | ⚠️ PRESENT, INCOMPLETE | It is the one predicate for *emptiness*; it has **no clause for embedded invisibles**, which is the missing half. Four consumers confirmed by grep (`NonBlank::new`, `is_plain_path_component`, `app::goal_or_none`, `ui::screens::driver::goal_lines`). Both-directions unit pins present. |
| `src/journal/mod.rs` (`is_plain_path_component`) | One definition of blank, composed with the structural halves | ⚠️ HALF-CLOSED | Delegates the blank half; keeps `is_control()` (:283) and `Component::Normal` + single-component + `name == value` (:286-293) — I verified neither was dropped. Accepts `"demo\u{200b}"`. |
| `src/registry.rs` (`add_project`, `add_project_unchecked`) | Alias blankness judged by the shared predicate | ✗ FOURTH PREDICATE | `is_empty()` + `contains(char::is_whitespace)`, unchanged; a `U+200B` alias registers. |
| `src/test_support.rs` (`DEGENERATE`) | The one shared blank-shape const | ✓ VERIFIED (with a named residual) | Six literals incl. `\u{200b}`/`\u{feff}`, deliberately *not* derived from the predicate. `#[cfg(test)]`, so integration crates cannot consume it — disclosed in place and in `deferred-items.md`. **All six are wholly invisible; no look-alike fixture shape exists anywhere in the tree.** |
| `tests/spawn_seam_guard.rs` (guard nine) | `DriveArgs` declares no raw argv `String` field | ⚠️ ASSERTS TRUE, REACH OVERCLAIMED | The assertion passes and the planted-field control is real. Blind to `pub(`, `Box<str>`, `Cow<str>`, `&'static str`, `OsString`, and a trailing `//`; floor measured non-binding. |
| `tests/spawn_seam_guard.rs` (`ITEM_OPENERS` / `post_marker_offenders`) | Blind region provably empty, with a self-test | ✓ VERIFIED | Pass-5 warning 1 closed properly — the wrong sentence deleted, the right control added, the floor carried through the extraction. |
| `tests/spawn_seam_guard.rs` (`…spelled_in_exactly_one_place`) | Prohibition 2 enforceable | ⚠️ SCAN NARROWER THAN ITS MESSAGE | `src/` only; three hand-picked subsets in `tests/`. Mitigated by the `#[cfg(test)]` fact, the in-place disclosure, and the in-crate exhaustive sweep. |
| `src/driver/run.rs` (`a_run_with_no_command_source_writes_nothing_before_refusing`) | The CR-01 property as an assertion | ⚠️ ASSERTION NARROWER THAN THE PROPERTY | Asserts only `.planning/meta-manager`. `establish_envelope` writes to `GSD_MM_ENVELOPE_ROOT`/`dirs::data_local_dir()`, outside the project root by construction, so moving the resolution back below `:2493` would keep this test green. The property holds at HEAD (ordering read directly); the *test* cannot see half of it. |
| `.planning/REQUIREMENTS.md` | Accurate, internally consistent, untouched by the round | ✓ VERIFIED | Last touch `0c4f712`; all five read `[ ]` / `Gaps Found`. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `main.rs` (Drive arm) | `DriveArgs::from_argv` | The single production conversion from raw argv to `NonBlank` | ✓ WIRED | Print-and-exit refusal before `drive`; `RawDriveArgs` is the only crossing shape. |
| `payload::NonBlank::new` | `text::carries_visible_content` | Delegation, not re-implementation | ✓ WIRED | `mod.rs:580`. |
| `journal::is_plain_path_component` | `text::carries_visible_content` | The blank half shared; control/traversal halves its own | ✓ WIRED | `journal/mod.rs:273`; both other halves verified present. |
| `app::goal_or_none` / `ui::screens::driver::goal_lines` | `text::carries_visible_content` | The render-side judgment | ✓ WIRED | `app.rs:111`, `ui/screens/driver.rs:760`. |
| `registry::add_project` | `text::carries_visible_content` | The alias registration judgment | ✗ **NOT WIRED** | A fourth predicate; reproduced. |
| `is_plain_path_component` | a look-alike refusal | An invisible-formatting clause | ✗ **NOT WIRED** | No such clause exists anywhere in production. |
| `execute_run` | `iteration_source` | Source resolved once above the first write | ✓ WIRED | Ordering read directly; corroborated by the reviewer's independent envelope-mtime observation. |
| `tests/spawn_seam_guard.rs` (guard nine) | `DriveArgs` declaration | `ITEM_OPENERS`-style token match on field lines | ⚠️ PARTIAL | Reported for `pub `; silent for `pub(`, and for four non-`String` string types. |
| `one_of_each` | `ALL_VARIANT_NAMES` | A compile-forced pairing | ✗ **NOT WIRED** | No compile-time relationship exists. |
| `tests/driver_goal_seam.rs` (over-length fixture) | `goal.rs`'s `untrusted::bounded` refusal | A control-free 201-char token | ✓ WIRED | Three assertions incl. `ends_with(TRUNCATION_MARKER)`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `run.json` | `gsd_command` | `recorded_command(&IterationSource)`, total over two variants | ✓ | ✓ FLOWING — pass-5's `""` reproduction no longer runs |
| `run.json` | `goal` | `args.goal.as_ref().map(NonBlank::as_str)` | ✓ | ✓ FLOWING — pass-5's `"   "`/`\u{200b}` reproductions no longer run |
| `run.json` / run directory | `run_id` | `NonBlank` + `is_plain_path_component` | ⚠️ | ⚠️ AMBIGUOUS — wholly-invisible refused; **`"…-aaaa"` and `"…-aaaa\u{200b}"` both accepted, two directories, one rendering** |
| `config.json` | registry key (alias) | `registry::add_project`'s own predicate | ✗ | ✗ DISCONNECTED — a `U+200B`-only key registers; look-alike keys register as distinct |
| `<envelope>/<alias>/` | envelope root | `envelope_dir_in` → `is_plain_path_component` | ⚠️ | ⚠️ AMBIGUOUS — `None` for wholly-invisible (good); **two roots for two look-alike aliases** (measured) |
| `run.json` | `target_phase` | `Option<NonBlank>` + structural check at the seam | ✓ | ✓ FLOWING |
| TUI driver screen | `goal` line | `goal_lines` → `carries_visible_content` | ✓ | ✓ FLOWING — pass-5's label-with-nothing-after-it is fixed |

### Behavioral Spot-Checks

Every count- or presence-bearing check ran under `rtk proxy`; each negative grep
carries a positive control.

| Behavior | Command | Result | Status |
|---|---|---|---|
| In-crate suite (matrix, predicates, record builders, tracer) | `cargo test --lib -- --test-threads=2` | **1031 passed, 0 failed** | ✓ PASS |
| Guard suite | `cargo test --test spawn_seam_guard` | **32 passed, 0 failed** | ✓ PASS |
| Criterion-bearing binaries | `cargo test --test driver_escalation_cap --test driver_injection_corpus --test driver_refusal_record --test driver_goal_seam --test driver_dry_run --test driver_model_seam` | 8 / 12(+10 ign) / 9 / **21** / 15 / 4(+3 ign) — all 0 failed | ✓ PASS |
| Gate lint | `cargo clippy --lib -- -D warnings` | exit 0 | ✓ PASS |
| Unfiltered severity | `cargo clippy --all-targets` | **4** warnings, all pre-existing (`browser.rs:131-133`, `project_creator.rs:146`) | ✓ PASS (info) |
| Prohibition 1 (21-15), mechanical | `grep -n '=> String::new()' src/driver/run.rs src/driver/mod.rs` | **0 hits** (exit 1); tree-wide 4, two opened and confirmed off argv/record paths | ✓ PASS |
| **CR-01 (registry) reproduction** | `registry::add_project` against a real `Config` + fixtures | `add_project("\u{200b}")` → **Ok**; `add_project("demo")` and `add_project("demo\u{200b}")` → **both Ok, two keys, two projects** | ✗ CONFIRMS CR-01 |
| **Look-alike envelope roots** | `envelope::envelope_dir_in` | `demo` → `<root>/demo`; `demo\u{200b}` → `<root>/demo\u{200b}`; `\u{200b}` → `None` | ✗ CONFIRMS WR-05 / falsifies 21-15 t5 |
| **Look-alike run directories** | `journal::run_paths` | `"…-aaaa"` and `"…-aaaa\u{200b}"` → two distinct dirs | ✗ CONFIRMS WR-05 |
| **`from_argv` accepts a look-alike run id** | `DriveArgs::from_argv` | `"aaaa"` → Ok; `"aaaa\u{200b}"` → **Ok**; `"\u{200b}"` → `Err(RunIdInvalid)` | ✗ CONFIRMS WR-05 / IN-01 rationale false |
| **Predicate split, both directions** | `is_plain_path_component` / `carries_visible_content` | `"demo\u{200b}"`, `"de\u{200b}mo"`, `"demo\u{feff}"` → true; `"\u{200b}"`, `"   "` → false | ✗ CONFIRMS the wholly/embedded split |
| **WR-01 reproduction** | standalone `rustc`, four-variant enum, `one_of_each` as shipped | pin **PASSES** with the fourth variant unswept; **zero** compile errors | ✗ CONFIRMS WR-01 |
| **WR-02 reproduction** | guard-nine fns extracted verbatim, 10 planted declarations | 6 SILENT, 1 misattributed, floor `field_lines=12 protected=6` → **PASSES (silent)** | ✗ CONFIRMS WR-02 |
| WR-04 | `grep -rn 'for blank in \[' tests/` | 3 hand-picked subsets, no two the same | ⚠️ CONFIRMS WR-04 (disclosed) |
| Guard-nine coincidental bound | `grep -rl '[^w]DriveArgs {' tests/*.rs \| wc -l` | **13** integration files construct `DriveArgs` literally | ⚠️ coincidental reliance |
| Argv alias entry points | `grep -c 'alias: String\|alias: Option<String>' src/cli.rs` | **8**, of which 1 routes through `from_argv` | ℹ️ the bound for round 6 |
| Debt markers in the round-5 diff | `git diff 199d334..HEAD -- src/ tests/ \| grep -E '^\+.*(TBD\|FIXME\|XXX)'` and `(TODO\|HACK\|PLACEHOLDER)` | **0** and **0**; control needle `NonBlank` → **101** hits, so neither zero is vacuous | ✓ PASS |
| Documented flakes | `driver_reattach`, `envelope_tracer` | Not exercised in my targeted runs; green in the orchestrator's full run | ✓ NOT A GAP |

Every scratch artifact I created was removed: `git status` shows only the
pre-existing untracked `.gsd/` and `.planning/milestone.lock`.

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 04, 07, 09, 11, 13, 15 | User states a goal once; driver pursues it without further input | ⚠️ PARTIAL | Criteria 1 and 2 both verified — the requirement's *behaviour* is satisfied. Held back only by the alias-identity gap: the value that selects *which project* the goal is pursued against can name two projects that render identically. |
| DRIVE-03 | 21-01, 03, 04, 07, 08, 09, 11, 13, 15 | Goal decomposed into a structured, machine-checkable, reviewable plan | ✓ SATISFIED | Criterion 1 verified; decomposition refuses rather than repairs; the record that is evidence of it can no longer carry a blank in `goal`, `gsd_command`, `run_id` or `target_phase`. |
| DRIVE-04 | 21-02, 04, 06, 10, 12, 14, 16 | Escalation capped per run; exceeding it parks | ✓ SATISFIED | Criterion 3 verified; both cap directions pinned and green in my own run. |
| SAFE-07 | 21-01, 03, 05, 06, 08, 10, 12, 14, 16 | `.planning/` content passed inside an explicit untrusted boundary | ✓ SATISFIED | Criterion 4 verified; arrival-before-influence assertions intact and non-vacuous. |
| SAFE-08 | 21-01, 05, 06, 12, 14, 16 | Model's action constrained to a fixed enum; no free-form shell strings | ✓ SATISFIED | Criterion 5 verified; I checked and this pass's gap does not reach this seam (roadmap membership refuses a look-alike phase token) — recorded as a precondition, not a guarantee. |

**No orphaned requirements.** The union of `requirements:` across all sixteen
plans is exactly {DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}, matching
REQUIREMENTS.md's phase-21 mapping and the ROADMAP `Requirements:` line.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/registry.rs` | `:14-20`, `:65-71` | A fourth production spelling of blankness (`is_empty()` + `is_whitespace`) on the `add` argv path | 🛑 Blocker | Falsifies 21-15 truth 4. A `U+200B` alias registers; `list` renders a blank ALIAS column. Reproduced. |
| `src/journal/mod.rs` | `:268-294` | The shared predicate has no clause for *embedded* invisibles, so a value that renders identically to another is a different identity | 🛑 Blocker | Falsifies 21-15 truth 5 verbatim; leaves T-21-15-02 (high/mitigate) half mitigated across seven call sites. Reproduced to two envelope roots and two run directories. |
| `src/driver/mod.rs` | `:2078-2084`, doc `:2067-2072` | `one_of_each` claims two compile errors and produces none | 🛑 Blocker (to 21-16 t4) | Pass-5 warning 4 recurring inside the test written to close it. Reproduced with `rustc`. |
| `tests/spawn_seam_guard.rs` | `:2743`, `:2749`, `:2790-2804`, `:2918-2924`, limits `:2694-2712` | Guard nine blind to `pub(`, `Box<str>`, `Cow<str>`, `&'static str`, `OsString` and a trailing `//`; its stated floor does not bind | 🛑 Blocker (to 21-16 t1) | `pub(` is the exact spelling the sibling guard in the same file was widened for four commits later. Four spellings unnamed in the limits block. Reproduced. |
| `.planning/…/21-16-SUMMARY.md` | `:253`, `:264` | Prohibition audit certifies PASS while citing a bound measured not to bind | ⚠️ Warning | A self-audit that certifies an unmeasured bound. The register 21-15-SUMMARY residual 1 used — disclose and ask — is the right one. |
| `src/driver/run.rs` | `:3788-3797` | The CR-01 tracer asserts only `.planning/meta-manager`; the envelope half of the property is outside the project root by construction | ⚠️ Warning | Moving the source resolution back below `:2493` — the exact regression the fix prevents — leaves this test green while every `cargo test --lib` writes into the developer's real data directory. 21-15-SUMMARY residual 4 saw the interaction and did not turn it into an assertion. |
| `tests/spawn_seam_guard.rs` | `:3067-3097` | Prohibition-2 guard scans `src/` while its message claims tree-wide uniqueness; three different hand-picked subsets live in `tests/` | ⚠️ Warning | Coverage, not a hole — `test_support` is `#[cfg(test)]`, the residual was disclosed in place first, and the exhaustive sweep is in-crate. The overclaiming *message* is the defect. |
| `src/driver/mod.rs` | `:2563-2566` | The acceptance matrix hand-exempts `--alias` and `--run-id` on a rationale about a function `from_argv` never calls | ⚠️ Warning | Structurally the cycle-3 defect recurring. I measured the rationale false. Costs two of seven columns; cannot hide a refusal failure. |
| `src/driver/mod.rs` | `:426-432` + `src/error.rs:328` | A refusal that borrows `UnknownAlias` states a falsehood about the registry when the alias *is* registered | ⚠️ Warning | Round-attributable to D-15-4. A refusal that misdescribes durable state is this phase's own subject. |
| `tests/spawn_seam_guard.rs` | `:2380-2423` | Guard eight's limits block numbered 1, 6, 2, 3, 4, 5 | ℹ️ Info | Cosmetic, but this block is the artifact prohibition 1 asks a reader to audit. |
| `tests/driver_goal_seam.rs`, `tests/spawn_seam_guard.rs` | `:1514`+, `:2132`, `:2136` | Assertion messages lost `\` continuations and embed 8–10 space runs | ℹ️ Info | Confirmed by reading (e.g. `"must be REFUSED.          Truncated instead…"`). These are the messages a future reviewer reads when a guard fires. |

**No unreferenced debt markers were introduced.** `git diff 199d334..HEAD -- src/ tests/`
adds zero `TBD`/`FIXME`/`XXX` and zero `TODO`/`HACK`/`PLACEHOLDER`; the control
needle `NonBlank` returns 101 hits, so neither zero is vacuous.

### Human Verification Required

None. Every finding above is a code-level fact — reproduced against the built
library at HEAD, measured with a standalone `rustc` program, or read directly
from the source. Nothing is taken from a SUMMARY or from `21-REVIEW.md`.

### On the process note the reviewer escalated

**I agree that no assertion was lost, and I checked rather than accepted it.** I
read the `ITEM_OPENERS` header and comment (`:1986-2053`) against the property it
is supposed to state: it still names both remaining silent gaps with their
failure directions, still records that gap 2 was live at column zero with 51
instances, and still points at the control that bounds it. Only the literal
sentence the acceptance criterion grepped for is paraphrased. Same for the
`scanned_files >= 10` floor, which I confirmed survives the extraction with the
control asserting `== 1`.

**And I agree the criterion shape should not recur, for a reason worth naming
beyond the one the reviewer gives.** `grep -c "<sentence of prose>" == 0` makes
source *text* the artifact under test. The executor's only compliant move is to
edit prose — and this round the executor did that honestly and disclosed it. But
the failure mode is not that an assertion gets lost; it is that a criterion of
that shape *passes* while the underlying property is false. Guard nine is the
proof: its three literal-grep criteria all read the specified numbers, its limits
block was rewritten, the SUMMARY's audit table quoted it — and the guard is blind
to five spellings and its stated floor does not bind. **Every prose-level
acceptance criterion in round 5 passed; both structural claims they were meant to
certify are false.** Future plans should assert behaviour with a control that
plants the defect, which is precisely what the `ITEM_OPENERS` half of this round
did correctly and the guard-nine half did not.

### Gaps Summary

**What genuinely closed** — verified by direct reading and my own runs, not by
trusting either SUMMARY: all three pass-5 Criticals (CR-01 structurally, IN-01 by
type, CR-02's wholly-invisible half), four of five pass-5 warnings, the
`DriveArgs` domain with a real no-`..` parse boundary, the `NonBlank` privacy
anchor re-attacked and unbroken, both `String::new()` arms deleted as
unrepresentable, the multi-flag matrix row, the restored length-bound fixture,
and REQUIREMENTS.md left alone for the third round running. **Criterion 1 passes
for the first time in six passes.** That is the largest genuine closure this
phase has had.

**Two gaps remain, and neither is criterion 1's class.**

**Gap 1 — the naming property.** Two halves that the round conflated and that
need different fixes. `registry::add_project` is a genuinely new surface: a
fourth predicate on the `add` subcommand, which nobody had named in five rounds.
But the look-alike half is *not* new — pass 5 wrote the sentence, 21-15 truth 5
claimed to close it, and the fix could not, because `carries_visible_content` asks
"is anything visible?" and `demo\u{200b}` has plenty. I measured two envelope
roots, two registry keys and two run directories for values that render as one.
**The fix chose exactly the shapes pass 5 had reproduced and skipped the shape
pass 5 had only named** — one shape short of the class, for the fifth time, in a
matrix whose six payloads are structurally incapable of expressing the miss.

**Gap 2 — the anti-recurrence machinery.** Both mechanisms built this round to
end the cycle are weaker than their own docs, their plan truths and their
SUMMARY's audit table say. `one_of_each` produces no compile error at all;
guard nine is silent on five spellings including one `DriveArgs` already carries
and one the sibling guard in the same file was widened for four commits later,
and its stated non-vacuity floor provably does not bind. **This is the fourth
consecutive round with an overclaiming guard header and the second consecutive
round in which a plan shipped a violation of a prohibition it wrote itself.** It
matters more than gap 1, because these are the artifacts round 6 will trust
instead of re-deriving — and the compile-side half of the `DriveArgs` bound
(`from_argv`'s destructure) is real, so a reader who conflates the two will
believe a seventh field is type-protected when only its *classification* is
forced.

### Recommendation

**Round 6 should do two things, and they are different in kind. Do both.**

**1. Extend the mechanism — the domain is `Commands`, and it is eight rows.**
`src/cli.rs` declares eight argv-derived alias fields; exactly one routes through
`DriveArgs::from_argv`. This is not open-ended hardening; it is the same move
21-15 already proved works, applied to an enum the compiler enumerates. A parse
boundary over the raw clap enum with an exhaustive per-variant destructure and no
`..`, yielding a validated alias newtype that `add_project`, `remove_project`,
`envelope_dir_in`, `askpass_with_config` and the four hook re-entry points take
instead of `&str`. Then a ninth subcommand cannot be added unclassified.

**2. Close the predicate-content gap — one clause and one missing fixture shape.**
`text::carries_invisible_formatting` beside `carries_visible_content`, consumed by
`is_plain_path_component` (after its `is_control()` refusal) and by both registry
entry points. And add the fixture shape the tree has never had:
`LOOK_ALIKE_PAIRS: [(&str, &str); N]` beside `DEGENERATE` — `("demo", "demo\u{200b}")`,
`("abc", "a\u{200b}bc")`, `("x", "x\u{feff}")` — asserted at every seam that turns a
value into an identity. `DEGENERATE` enumerates "carries nothing"; nothing in the
tree enumerates "carries something different from what it renders," which is why
a 42-cell matrix went green over the harm.

**3. Fix the machinery in the same commits, not in a follow-up.** Delete
`one_of_each`'s false compile-error claim (or make the pairing exhaustive by
destructuring and say honestly what that does and does not force). Add `"pub("`
to guard nine's opener test in both places, widen `names_bare_string` to
`String`/`str` with `OsStr` handled by field name, strip a trailing `//` before
the comma test and flush `pending` at the body's end — each with a control arm
beside the existing planted-field control. Rewrite guard nine's limits block to
name the four silent type spellings and to say what the floor actually bounds.
These sit in files round 6 will already have open, and they bear directly on why
this recurs.

**4. Change the shape of the acceptance criteria.** Every prose-grep criterion
this round passed; both structural claims they were meant to certify are false.
Assert behaviour with a control that plants the defect — the `ITEM_OPENERS` half
of this round is the model, and the guard-nine half is the counter-example, in
the same file, from the same plan.

**One thing to carry forward without re-litigating.** The `NonBlank` type-level
mechanism has now survived two consecutive rounds of direct attack, and
`from_argv`'s no-`..` destructure is a genuine compile-time bound. The design is
right. What has failed five times is never the mechanism — it is that the *set*
the mechanism is pointed at, and now the *content of the predicate* it enforces,
keep being chosen from the instances the last reviewer happened to reproduce. The
two recommendations above are the two remaining places that choice is still being
made by hand, and both are enumerable by the compiler.

---

_Verified: 2026-08-22T18:21:16Z_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `5b24022` · sixth verification pass · pass 5 preserved at `7c72339`_
