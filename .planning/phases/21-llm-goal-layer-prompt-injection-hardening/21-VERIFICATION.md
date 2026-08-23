---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-23T03:23:33Z
head: f1faa3e
status: gaps_found
score: 11/20 must-haves verified (3/5 ROADMAP success criteria; 1 FAILED, 1 behavior-unverified)
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 15/19 must-haves verified (5/5 ROADMAP success criteria)
  gaps_closed:
    - "pass-6 gap 1, the `registry.rs` half (a FOURTH production spelling of blankness — `is_empty()` + `contains(char::is_whitespace)` — on the `add` subcommand): CLOSED, and closed structurally rather than by adding a fifth consumer. `registry::Alias` (`src/registry.rs:29-30`) is a real newtype with a private field and one fallible constructor; `add_project` (:140) and `add_project_unchecked` (:186) both take `&Alias`, so an unjudged alias is unrepresentable at both signatures. I re-derived the reviewer's Claim B independently: `grep -n 'is_empty()\\|is_whitespace\\|trim()' src/registry.rs` returns, in executable code, only `Alias::new`'s delegating clause 3 (:107, a whitespace *usability* rule kept deliberately and documented as such), `opt_in.prompt_inputs.is_empty()` (:389, unrelated) and `derive_alias`'s filter (:494, which feeds `Alias::new` anyway). The fourth predicate is deleted, not relocated. Every clause of `Alias::new` resolves to `text::` or `journal::` — I opened each one."
    - "pass-6 gap 1, `AliasNotVisible` (a boundary refusal that borrowed `UnknownAlias` and so asserted a falsehood about the registry): CLOSED. Measured against the built library: `DriveArgs::from_argv(alias = \"\\u{200b}\")` -> `Err(AliasNotVisible { alias: \"\\u{200b}\" })`. The refusal fires at the same pure, file-free boundary (`src/driver/mod.rs:432-433`), still above `drive`, and now describes the value instead of the registry's contents."
    - "pass-6 gap 1, the `Commands` domain (eight argv alias fields, one protected): CLOSED as a census with a real control. `grep -nE 'alias: (String|Option<String>),' src/cli.rs` -> exactly EIGHT declarations (lines 30, 35, 50, 306, 319, 337, 361, 371) — I counted them myself — and `ARGV_ALIAS_ENTRY_POINTS` (`tests/spawn_seam_guard.rs:3594`) is an 8-row table naming each variant's judge, with one recorded raw-by-design decision (`Commands::Remove`, D-17-3, the recovery path for entries an older build registered). The planted-ninth control consumes `argv_alias_fields`, the same extracted fn the live assertion consumes, not a re-implemented loop."
    - "pass-6 gap 2, `one_of_each`'s false compile-error claim: CLOSED BY HONESTY, which was the right move. The doc (`src/driver/mod.rs:2077-2101`) now states plainly that the verifier measured the pin PASSING with a fourth variant unswept and zero compile errors, that constructing values carries no exhaustiveness obligation, that the only compile-time anchor is `variant_name`'s wildcard-free match, and that a `named(source)` helper would not change it. No pseudo-mechanism was substituted. I read the whole doc."
    - "pass-6 IN-01 (the acceptance matrix hand-exempting `--alias`/`--run-id` on a rationale about a function `from_argv` never calls): CLOSED. `one_visible_character_is_accepted_in_every_argv_position` (`src/driver/mod.rs:2608-2621`) is now one nested loop over `positions()` x four padded payloads with no narrowing and no exemption comment. I read the loop body."
    - "pass-6 WR-04's substantive half (`DEGENERATE` hand-copied in three `tests/` files): CLOSED. `src/lib.rs:21-23` ships `pub mod test_support` unconditionally — the `#[cfg(test)]` gate is gone — and all three pins now consume the const (`tests/driver_dry_run.rs:609`, `:704`, `tests/driver_goal_seam.rs:340`). Tree-wide, `grep -rn 'for blank in \\[' src/ tests/` returns ONE hit, `src/text.rs:122`, whose four values are additive rather than a `DEGENERATE` subset. (The guard's *message* still overclaims — see gaps.)"
    - "pass-6 gap 2's record half: CLOSED. `21-18-SUMMARY.md:260-290` carries explicit Record corrections naming 21-16 truth 1 and truth 4 as FALSE as shipped, with the pass-6 measurement and the now-true statement for each. A seventh reader inherits measurements rather than certifications — this is the process improvement the phase most needed, and it is real."
    - "pass-6 artifacts-table warning (the CR-01 tracer asserting only `.planning/meta-manager`): CLOSED. `src/driver/run.rs:3840-3852` now also asserts the envelope root received no write, reading the REAL resolved root via `envelope::envelope_dir(CR01_ENVELOPE_PROBE_ALIAS)` rather than injecting one. I agree with the reviewer's adjudication of deviation D-18-6: reading the real resolution is strictly stronger than the plan's proposed injected root, and the dedicated probe alias removes the collision risk."
    - "Process: `.planning/REQUIREMENTS.md` was not touched by either round-6 plan. `git log -- .planning/REQUIREMENTS.md` still shows `0c4f712` as the last commit; all five phase-21 requirements read `[ ]` (lines 62, 63, 83, 85, 86) and `Gaps Found` (152, 153, 164, 166, 167). FOURTH round running with the prohibition honoured."
  gaps_remaining:
    - "Criterion 1 does NOT hold, and this is a correction to pass 6 rather than a regression introduced by round 6. `src/text.rs`'s character class is byte-identical to the class at `5b24022` (verified with `git show 5b24022:src/text.rs`), so the round-6 diff regressed nothing — but the class is a hand-enumerated SUBSET, and the same failing class pass 6 declared closed is reproducible today with a different character. Measured against the built library: `DriveArgs::from_argv` returns `Ok` for a `--run-id`, `--goal`, `--command` AND `--alias` consisting of a single U+202E, U+00AD, U+E0041 or U+FE0F, and `run_paths` returns `Some(<dir>)` for a run directory named by one of them. Pass 6 attacked criterion 1 with U+200B and U+FEFF — the two characters the implementation's own list covers — and so measured the implementation against itself."
    - "CR-01 (the class is a hand-enumerated subset) is REPRODUCED end to end against the binary built from HEAD: nine registry keys that all render as `demo`, plus a Trojan-Source alias that renders as `gsd-run` in the tool's own `list` output."
    - "Both round-6 anti-recurrence mechanisms still overclaim: guard nine is blind to a field declared with NO visibility modifier (a spelling in neither its SEEN nor its SILENT list), and its `OSSTRING_ALLOWED` integrity pin is a `contains` rather than an equality, so an allowlisted field can carry a raw `String` payload undetected. FIFTH consecutive round with a guard header claiming more than its scan performs."
    - "SAFE-07's boundary has never been executed end to end in any VERIFICATION pass of this phase. All seven comparison arms, both suppression controls, AND the arms' own non-vacuity meta-check are `#[ignore]`d."
  regressions: []
gaps:
  - truth: "A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs (ROADMAP success criterion 1) — specifically, no argv payload carrying no visible instruction reaches a persisted record"
    status: failed
    reason: "REPRODUCED against the built library at HEAD, in the position pass 6 declared closed by construction. `payload::NonBlank::new` delegates to `text::carries_visible_content`, which asks `!(is_whitespace || is_control || is_invisible_formatting_char)` — and `is_invisible_formatting_char` is three literal ranges. A character that is invisible but OUTSIDE those ranges is therefore judged VISIBLE, so `NonBlank::new` accepts it, `is_plain_path_component` accepts it, and `run_paths` yields a directory. This is not the identity/look-alike question the round-6 review raised; it is the EMPTINESS question, which is criterion 1's own class. Pass 6's VERIFIED verdict was reached by attacking with U+200B and U+FEFF — the characters the list covers — which is the same tautology the round has now committed at three successive levels."
    artifacts:
      - path: "src/text.rs"
        issue: "`is_invisible_formatting_char` (:75-77) is `matches!(c, '\\u{200b}'..='\\u{200f}' | '\\u{2060}'..='\\u{2064}' | '\\u{feff}')`. The gap between the two low ranges (U+2010–U+205F) contains U+202A–U+202E, the bidi embedding/override block (Trojan Source, CVE-2021-42574), and U+2066–U+2069, the bidi isolates. Everything above U+FEFF is missing entirely, including U+E0000–U+E007F, the Unicode tag block — the standard LLM ASCII-smuggling carrier, in a prompt-injection-hardening phase. The doc's own stated boundary (:90-96, 'zero-width and format characters') is WIDER than the implementation, and the TR39/homoglyph carve-out does not cover the gap: every value I measured is `Cf` or default-ignorable and none is a homoglyph. MEASURED, by me, through the production boundary (`cargo test`, scratch integration crate, removed afterwards): `carries_visible_content` returns TRUE for a value of one U+202E, U+2066, U+00AD, U+034F, U+E0041, U+FE0F, U+13430, U+180E or U+FFF9, and FALSE only for U+200B, U+FEFF and whitespace."
      - path: "src/driver/mod.rs"
        issue: "`DriveArgs::from_argv` — the parse boundary criterion 1 rests on — returns `Ok` for all four argv string positions I probed when the payload is a single invisible character outside the ranges. Measured verbatim: `run_id`/`goal`/`command`/`alias` with U+202E, U+00AD, U+E0041 or U+FE0F -> `Ok` (no error). The SAME four positions with U+200B -> `RunIdInvalid` / `NoCommandSource` / `NoCommandSource` / `AliasNotVisible`, and with `\"   \"` likewise. The refusal machinery is correct and wired; the class it consults is short. The 7x6 refusal matrix cannot see this because all six `DEGENERATE` payloads are built from U+200B/U+FEFF and whitespace."
      - path: "src/journal/mod.rs"
        issue: "`run_paths` returns `Some(\"<planning>/meta-manager/runs/\\u{202e}\")` — a complete run directory named by one invisible character, which is pass 5's exact reproduction recurring with a different code point. Measured for U+202E, U+2066, U+00AD, U+034F, U+E0041, U+FE0F, U+13430, U+180E and U+FFF9; `None` only for U+200B and whitespace."
      - path: "src/driver/run.rs"
        issue: "The record builders are total and correct, and that is precisely why the value survives: `recorded_command` (:910-915) returns `command.clone()` from the `NonBlank`, and `make_run_record` (:813-816) writes `goal` from `NonBlank::as_str().to_owned()`. Neither can manufacture a blank — but neither can refuse one it was handed, so a `run.json` can carry `\"gsd_command\"` and `\"goal\"` fields that render as nothing (or, for U+202E, reorder the JSON text around them). The D-30 guarantee that `\"\"` provably means *absent* holds; the guarantee that a non-empty value means *something a human can read* does not."
    missing:
      - "Stop enumerating the class by hand and DERIVE it. The class both judgments need is Unicode `General_Category=Cf` plus `Default_Ignorable_Code_Point` plus the variation selectors and U+034F — available either from a checked-in table generated from UCD `DerivedCoreProperties.txt` or from a crate (`unicode-security` / `unicode-properties`). Whichever is chosen, the decision must be recorded WITH its maintenance obligation: a pinned Unicode version that goes stale on every UCD release, and a test that fails loudly when it does."
      - "Give the class a falsifying corpus, not a fixture drawn from itself. `LOOK_ALIKE_PAIRS` (`src/test_support.rs:53-57`) is three pairs built entirely from U+200B and U+FEFF — inside the predicate's own ranges — so it agrees with any implementation covering those two characters and nothing else, while its doc (:49-52) claims exactly the opposite anti-tautology property. The property test must iterate a corpus derived from the STANDARD (`for c in unicode_default_ignorable_corpus()`), which is the only shape that can go red on a subset. Adding `(\"demo\", \"demo\\u{202e}\")` and `(\"demo\", \"demo\\u{e0041}\")` to the const is necessary but not sufficient — it is one more hand-chosen sample."
      - "Fix `carries_visible_content` FIRST and separately from `carries_invisible_formatting`. The identity half is what the round-6 review reported; the EMPTINESS half is what falsifies criterion 1, and it is the one nobody has named. A fix scoped to the identity seams would leave `--goal '\\u{202e}'` persisting into a committed record."
      - "State the acceptance direction as a fact after the widening. Every value the suite pins as acceptable (`\"20\"`, `\"2.1\"`, `\"2026-08-19T12-00-00Z-aaaa\"`, `\"demo\"`, `\"99\"`, `\"/gsd:progress\"`, `\" x \"`) must still pass, and ZWJ/ZWNJ (U+200D/U+200C) must remain legal in FREE TEXT — they are load-bearing in real scripts and the phase already made that trade explicitly."
  - truth: "Two visually identical identities can no longer resolve to two different registry keys, envelope roots, credential scopes, run directories or phase tokens (21-17 truths 1, 2, 6 and backstop truth 8; threat T-21-15-02, severity high, disposition mitigate)"
    status: failed
    reason: "The MECHANISM this round built is real and I could not break it — one shared class fn, two judgments, every seam delegating, a newtype closing registration at the entry. The CLASS it enforces is a subset, so every seam reopens for the uncovered part. Backstop truth 8 is measurably false. This is the sixth consecutive round in which the fix covered exactly the shapes the previous reviewer had REPRODUCED and missed the shapes that had only been NAMED — the doc at `src/text.rs:91-96` names 'zero-width and format characters', and the implementation covers 22 of them."
    artifacts:
      - path: "src/registry.rs"
        issue: "REPRODUCED END TO END against `target/debug/gsd-meta-manager` built from HEAD, with a scratch config and one fixture project. Nine `add` invocations, nine exit codes: `demo` 0, `demo`+U+202E 0, +U+202D 0, +U+2066 0, +U+061C 0, +U+FE0F 0, +U+E0001 0, +U+00AD 0, +U+13430 0 — and `demo`+U+200B exit 1, the only refusal. `config.json` then held NINE distinct project keys, every one of which renders as `demo`. A second probe registered `gsd-\\u{202e}nur`, exit 0, and `list` printed it raw through `Display` (`src/main.rs:126-131`): in any bidi-aware terminal that row reads `gsd-run`. That is terminal spoofing in the tool's primary identity display, admitted by an `add` that exits 0 — and the same raw `Display` path is used by `judged_alias_or_exit`'s refusal (`main.rs:33-47`) and by the TUI project table."
      - path: "src/text.rs"
        issue: "The systemic cause, stated as the finding rather than as commentary. `carries_visible_content` and `carries_invisible_formatting` genuinely share one class fn — claim A holds and I verified it (`grep -rln '2060' src/` -> `src/text.rs` and `src/driver/mod.rs`, the latter the declared independent test-side oracle inside its `#[cfg(test)]` module). The defect is the class's EXTENT, not its sharing. Sharing a wrong class perfectly is what let one hand-enumeration propagate to five seams at once."
      - path: "src/journal/mod.rs"
        issue: "`is_plain_path_component` returns TRUE for `\"demo\\u{202e}\"`, `\"demo\\u{e0041}\"`, `\"demo\\u{fe0f}\"` and `\"demo\\u{ad}\"` (measured), and FALSE for `\"demo\\u{200b}\"`. So `envelope_dir_in(root, \"demo\")` -> `<root>/demo` and `envelope_dir_in(root, \"demo\\u{202e}\")` -> `<root>/demo\\u{202e}` — two envelope roots for two aliases that render as one, which is T-21-15-02's exact harm, now in its third verification pass. `run_paths` likewise yields two directories for `\"…-aaaa\"` and `\"…-aaaa\\u{202e}\"`."
      - path: "src/driver/goal.rs"
        issue: "21-17 truth 6 claims the model-supplied phase token's look-alike safety became 'a PROPERTY, not a precondition'. MEASURED FALSE for the uncovered class: `is_plain_path_component(\"2\\u{200b}0\")` -> false (the claimed property, for the covered characters), but `is_plain_path_component(\"2\\u{202e}0\")`, `(\"2\\u{ad}0\")`, `(\"2\\u{e0041}0\")` and `(\"2\\u{fe0f}0\")` all -> TRUE. Those tokens still fall through to `goal.rs:704`'s roadmap-membership check — i.e. back to the coincidental reliance pass 6 recorded and this round contracted to eliminate. The precondition was narrowed, not removed."
      - path: "src/test_support.rs"
        issue: "CONFIRMED by reading and by printing every code point at runtime. `LOOK_ALIKE_PAIRS` is `[(\"demo\", \"demo\\u{200b}\"), (\"abc\", \"a\\u{200b}bc\"), (\"x\", \"x\\u{feff}\")]` — three pairs, two distinct invisible characters, both inside the predicate's own ranges. The doc at :49-52 asserts the anti-tautology property verbatim: 'an enumeration derived from `carries_invisible_formatting`'s own structure could not contain a pair that predicate mishandles (round-3 WR-03's tautology)'. Literal spelling is not independence: independence has to live in the SAMPLING, not in the syntax. Every LOOK_ALIKE assertion at every seam (`journal/mod.rs`, `envelope/mod.rs`, `registry.rs`, `text.rs`, `tests/registry_test.rs`) passes for any implementation covering those two characters and nothing else. This is round-3 WR-03 re-entering one level down, and it is why 1346/0 sits on top of a reproduced harm for the seventh consecutive pass."
    missing:
      - "Everything under gap 1 closes this too — one class fix serves both judgments, which is the one genuinely good consequence of the sharing this round built."
      - "Consider inverting the identity judgment from a deny-list to an ALLOW-list, separately from the emptiness judgment. Identities in this tool are already ASCII by every accepting fixture in the tree; `[A-Za-z0-9._-]` closes bidi, tags, variation selectors AND homoglyphs in one clause with no Unicode table and no dependency, and it cannot be one item short because the accepted set is finite. Free text must keep the derived Cf/default-ignorable deny-list, because a `--goal` legitimately carries arbitrary script. If the allow-list is adopted, the cost is a real product decision — a non-Latin-script user cannot name a project in their own script — and it must be recorded as a chosen trade, not discovered later."
      - "Add the pin at the two seams that carry no `LOOK_ALIKE_PAIRS` assertion of their own — `--target-phase` (`driver/mod.rs:877`) and the re-read run id (`journal/writer.rs:491`)."
  - truth: "Both anti-recurrence mechanisms say exactly what they deliver, certified only by planted-defect controls (21-18 truths 1, 2 and 5; 21-18 prohibition 3)"
    status: failed
    reason: "FIFTH consecutive round with a guard header claiming more than its scan performs, and the second consecutive round in which the overclaim lives in the artifact written to end overclaiming. Both findings are in task 1's code — the least-reviewed code in the round, committed by the earlier quota-killed executor — which the reviewer flagged and which I confirm by reading. I did not take either on the reviewer's word; both are deterministic control-flow facts I traced in the source."
    artifacts:
      - path: "tests/spawn_seam_guard.rs"
        issue: "WR-01, CONFIRMED by reading the control flow. `is_field_opener` (:2992-2994) is `trimmed.starts_with(\"pub \") || trimmed.starts_with(\"pub(\")`. In `raw_string_argv_fields` (:2811-2843) the `pending == None` branch does `if !is_field_opener(trimmed) || !trimmed.contains(':') { continue; }`, so a field declared with NO visibility modifier is skipped outright — never joined, never judged. The SAME fn filters the non-vacuity floor (:3305-3312), so the floor skips it too: `field_lines = 12`, `protected = 6`, `offenders = []`. That is pass 6's exact measurement signature, in a spelling that is in neither the SEEN list nor the SILENT list and is bounded by no assertion the guard makes. What actually stops it today is that thirteen integration crates build `DriveArgs { … }` as struct literals — the same coincidental compile bound `21-18-SUMMARY.md` named-shape row 15 reports as RETIRED. It is retired for the two payload-type spellings pass 6 measured; it is intact and load-bearing for this one. 21-18 truth 2 asserts 'No guard header in this plan's diff claims a bound no committed control goes red for'; this is one."
      - path: "tests/spawn_seam_guard.rs"
        issue: "WR-02, CONFIRMED by reading. `judge_declaration` (:2853-2869) short-circuits: `if names_token(type_text, \"OsString\") { … if !OSSTRING_ALLOWED.contains(&name) { out.push(…) } return; }` — the `return` fires BEFORE `names_string_payload(type_text)` is ever evaluated. So a declaration naming BOTH `OsString` and a raw payload, on an allowlisted name, is suppressed wholesale: `pub claude_args: (Vec<OsString>, String),` yields `offenders = []`. The integrity pin meant to catch exactly that (:3355-3366) is `declaration.1.contains(expected_type)` — a SUBSTRING test — and `(Vec<OsString>, String)` contains `Vec<OsString>`, so it passes too. 21-18 truth 1 and the prohibition-3 table both certify that 'an allowlist entry cannot be silently repurposed'. Measured by construction: it can. (`claude_program: Option<PathBuf>` names no `OsString` and falls through to the payload check, so it IS reported — the hole is specific to the early-return branch.)"
      - path: "tests/spawn_seam_guard.rs"
        issue: "WR-04, CONFIRMED by reading (:3446-3455, :3519-3549), and materially softer than the other two. `the_degenerate_payload_set_is_spelled_in_exactly_one_place` detects a hand copy via `executable_hits(&files, degenerate_witness())`, where the witness is ONE literal — the fourth of `DEGENERATE`'s six members, assembled at runtime from two halves. A future subset omitting that member (`[\"\", \"   \", \"\\t\", \"\\u{200b}\"]`, which is four of six and exactly the shape pass 5 found) is invisible to the scan, while the failure message reads 'Every blank-shape pin consumes `test_support::DEGENERATE`'. 21-18 truth 5 certifies 'the guard's message is finally true'; its coverage is one literal wide, and only the over-detection direction is named in the doc. NO standing violation exists — I censused `grep -rn 'for blank in \\[' src/ tests/` and the single hit (`src/text.rs:122`) is additive, not a copy — so this is a message that overclaims its scan, not a hole."
      - path: ".planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-18-SUMMARY.md"
        issue: "WR-05, REPRODUCED. `cargo test --test driver_injection_corpus -- --list --ignored` returns 10 tests: SEVEN `corpus_*_arrives_and_leaves_the_command_unchanged` arms (not eight, as :389-401 states), the two suppression controls, and `both_arms_of_every_class_comparison_were_really_executed` — the meta-check certifying that the hostile and clean arms both really ran, which the SUMMARY's list of non-vacuity pins omits and which is the only one speaking to the COMPARISON's non-vacuity rather than the corpus's. The qualification itself is a genuine and welcome honesty improvement over 21-16, which carried the same claim with the caveat unstated; the enumeration backing it is wrong."
    missing:
      - "Guard nine: widen `is_field_opener` to recognise a bare identifier before `:` (the reviewer's three-line patch is correct), plant a bare private field in `the_raw_argv_field_scanner_sees_every_measured_silent_spelling`, and add the spelling to the limits block naming what bounds it TODAY (thirteen struct-literal fixtures) and that this is a coincidence rather than a property."
      - "Guard nine: do not return early from the `OsString` branch — judge `names_string_payload` inside it — and make the allowlist integrity pin an EQUALITY against the parsed type text, not a `contains`."
      - "The DEGENERATE uniqueness guard: scan for at least two witnesses (the reviewer's `degenerate_witnesses()` is right) and add a limits line naming the under-detection direction, which is prohibition 3's own prohibition."
      - "Correct 21-18-SUMMARY's SAFE-07 qualification to 'seven class arms, two suppression controls, and `both_arms_of_every_class_comparison_were_really_executed` — the last of which is the arms' own non-vacuity check and did not execute either', and move 'SAFE-07 boundary never executed under verification' into `deferred-items.md` where it is tracked rather than into a SUMMARY qualification where it is read once."
deferred:
  - truth: "General Unicode CONFUSABLES / homoglyph defence (a Cyrillic `а` beside a Latin `a`)"
    addressed_in: "Not phase 21 — recommend a new roadmap item"
    evidence: "`src/text.rs:90-96` names this carve-out explicitly and correctly, and none of the 20 values I reproduced is a homoglyph — every one is `Cf` or default-ignorable, i.e. squarely inside the class the doc says it DOES cover. TR39 skeleton/confusables mapping is genuinely separate research with a separate data table and a separate false-positive profile. Keep the carve-out; do not let gap 1's fix quietly grow into it."
  - truth: "`registry::current_prompt_inputs` absent from `tests/async_blocking_guard.rs`'s BLOCKING_HELPERS"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4, 5 and 6)"
    evidence: "Async-hygiene class, not this phase's class; the fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run`."
  - truth: "The spawn-gate plan-half argument lives in a comment rather than a checked property"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "The comment states plainly that the plan half is a no-op and why that is sound; converting a sound, honestly-documented argument into a checked property is hardening, not gap closure."
  - truth: "Dead `PlanStep::rationale` field in src/driver/goal.rs"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "No production reader; cosmetic dead field with no security or honesty bearing."
  - truth: "`tests/driver_reattach.rs` and `tests/envelope_tracer.rs` flake under parallel execution"
    addressed_in: "Next phase backlog (deferred-items.md)"
    evidence: "Documented pre-existing flakes; neither fired in my own full-workspace run (1346 passed / 0 failed / 13 ignored) nor in the orchestrator's."
  - truth: "The degenerate matrix's ROW table stays hand-maintained (`positions()`)"
    addressed_in: "Disclosed residual, not a gap (deferred-items.md 'OUT — by design')"
    evidence: "`from_argv`'s exhaustive destructure bounds the TYPE of a seventh argv field; neither it nor guard nine forces a matrix row. 21-15 truth 6 discloses this in those words; I re-verified the disclosure is accurate."
behavior_unverified_items:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload ARRIVED at the model before asserting the command was unchanged, the two suppression controls show the positive/negative `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging, and `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile and clean arms both really ran."
    why_human: "All ten spawn the real `claude` binary and need an authenticated subscription, so they cannot run inside verification (`#[ignore] = \"spawns the real `claude` binary; run with `--ignored`\"`). Only twelve structural pins execute under an ordinary `cargo test` — they prove the corpus is planted where the shipped reader reads, that every payload survives the production bound whole, and that the typed state carries no marker, i.e. that the channel is the only channel. They cannot prove the model's behaviour ON that channel. The last recorded live run is `21-05-SUMMARY.md:514` (10 passed against `claude` 2.1.238), an executor claim from thirteen commits ago that no verification pass has reproduced; the round-6 reviewer independently adjudicated the same item 'carry as open — SAFE-07's boundary has not been executed end-to-end in any pass of this phase'. Presence and wiring are verified; behaviour is not."
coincidental_reliance_items:
  - truth: "A model-supplied `--target-phase` token cannot name a phase other than the one it appears to name (SAFE-08 / criterion 5)"
    reason: undeclared-precondition
    harden: "21-17 truth 6 contracted to convert this from a precondition into a property, and it did so only for U+200B/U+FEFF. Measured: `is_plain_path_component(\"2\\u{202e}0\")` and `(\"2\\u{e0041}0\")` both return TRUE, so those tokens still reach `goal.rs:704` and are refused only because no roadmap declares them — a property of today's roadmap CONTENTS, not of the value's validation. Gap 1's class fix converts it for real. Until then it should be stated as a precondition rather than relied on silently, which is exactly what pass 6 said one round ago."
  - truth: "A seventh argv field cannot be added to `DriveArgs` in an unprotected spelling"
    reason: fixture-only
    harden: "The compile-side half is genuinely real — `from_argv`'s twelve-field destructure with no `..` (`src/driver/mod.rs:414-429`) is a compile-time bound and I re-read it field by field. What is NOT real is the type half for a field declared with no `pub`: guard nine skips it in both the scan and the floor (WR-01). What prevents it today is that thirteen integration crates build `DriveArgs { … }` literally and a private field breaks all thirteen. `21-18-SUMMARY.md` row 15 reports that coincidental bound as retired; it is retired for two spellings and load-bearing for this one. Fix `is_field_opener` rather than resting on it."
human_verification:
  - test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` and record the CLI version beside the result."
    expected: "10 passed, 0 failed, with arrival asserted before influence in every class arm and `both_arms_of_every_class_comparison_were_really_executed` green."
    why_human: "Requires an authenticated subscription and spawns the real model binary; cannot run inside verification. This is criterion 4's only behavioural evidence and no verification pass of this phase has ever produced it."
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-23T03:23:33Z
**HEAD:** `f1faa3e`
**Status:** gaps_found
**Re-verification:** Yes — seventh verification pass, after the sixth gap-closure cycle

## Goal Achievement

**I have to correct my predecessor.** Pass 6 verified criterion 1 for the first
time in this phase and called it "the headline, and it is real." It was not
real. It was measured against a corpus drawn from the implementation's own
character list, and I broke it in about four minutes with a character the list
does not contain.

This is not a regression. `git show 5b24022:src/text.rs` gives a character class
byte-identical to HEAD's, so the round-6 diff changed nothing about it and every
structural claim pass 6 made about `DriveArgs`, `from_argv` and `execute_run` is
still true — I re-read all three. What changed is the measurement. Pass 6
attacked criterion 1 with U+200B and U+FEFF; those are two of the twenty-two code
points `is_invisible_formatting_char` covers, and the tree's entire blank-payload
fixture set (`DEGENERATE`) and its entire look-alike fixture set
(`LOOK_ALIKE_PAIRS`) are built from exactly those two characters plus whitespace.
Six rounds of fixtures, one character class, and the fixtures were sampled from
inside it.

| Cycle | What closed | What the SAME cycle's diff left open |
|---|---|---|
| 1 → 2 | round-2 CR-01/CR-02, WR-01, WR-05 | blank `--command`; approval token parsed after a live consultation |
| 2 → 3 | both | blank `--target-phase` (the `Routed` arm) |
| 3 → 4 | the `Routed` arm, the matrix exemption, the trim tautology | blank `--run-id`, blank `--goal` beside a source, blank `gsd_command` on disk |
| 4 → 5 | `NonBlank` held; scope was 3 of `DriveArgs`' 6 fields | the other three fields |
| 5 → 6 | all three pass-5 Criticals, the `DriveArgs` domain | the look-alike half; a fourth predicate on `add`; both new mechanisms overclaiming |
| **6 → 7** | **the `Commands` domain, the `Alias` newtype, `AliasNotVisible`, `one_of_each`'s honesty, the matrix exemption, `DEGENERATE`'s hand copies, the record corrections** | **the character class itself — and therefore criterion 1, which was never actually verified** |

### Judgment 1 — Does criterion 1 still hold? No. It never did.

**The reviewer's non-regression evidence is correct, and I sampled it rather
than accepting it.**

- `from_argv`'s destructure (`src/driver/mod.rs:414-429`) names all **twelve**
  `RawDriveArgs` fields with **no** `..`, and reconstructs all twelve. Read line
  by line.
- All six argv-derived string fields are still `payload::NonBlank` /
  `Option<NonBlank>`. Read the struct body field by field.
- `from_argv` is still pure — no file, no process — and still runs in `main.rs`
  above `drive`. The round's entire diff at that function is **one arm**: the
  alias refusal's variant, `UnknownAlias` → `AliasNotVisible`.
- `execute_run`'s ordering is untouched: `iteration_source` at `run.rs:2447`,
  `establish_own_group` :2449, `establish_envelope` :2493, `JournalRun::start`
  :2626, `recheck_approval` :2410. `git diff -U0 0352dda..HEAD -- src/driver/run.rs`
  shows **five hunks, all at line 3766 or below, all inside `mod tests`**.
- Full workspace suite, run by me once: **1346 passed / 0 failed / 13 ignored**.
  `cargo clippy --lib -- -D warnings` exit 0; `--all-targets` 4 warnings, all
  pre-existing (`browser.rs:131-133`, `project_creator.rs:146`).

**And the property is false anyway.** The machinery is sound; the predicate it
enforces is short. `payload::NonBlank::new` delegates to
`text::carries_visible_content`, whose test is
`!(is_whitespace || is_control || is_invisible_formatting_char)` — and
`is_invisible_formatting_char` is three literal ranges. A character that is
invisible but outside those ranges is judged **visible**. Measured through the
production boundary:

```
run_id U+202E RLO       -> Ok        run_id CONTROL U+200B -> Err(RunIdInvalid)
goal   U+202E RLO       -> Ok        goal   CONTROL U+200B -> Err(NoCommandSource)
cmd    U+202E RLO       -> Ok        cmd    CONTROL U+200B -> Err(NoCommandSource)
alias  U+202E RLO       -> Ok        alias  CONTROL U+200B -> Err(AliasNotVisible)
```

Same four `Ok`s for U+00AD, U+E0041 and U+FE0F. And
`run_paths(planning, "\u{202e}")` → `Some(".../runs/\u{202e}")` — **a complete
run directory named by one invisible character**, which is pass 5's reproduction
verbatim, recurring with a different code point. `recorded_command` returns
`command.clone()` and `make_run_record` writes `goal` from
`NonBlank::as_str()`, so a `run.json` can carry `gsd_command` and `goal` fields
that render as nothing.

**This is the emptiness judgment, not the identity judgment.** The round-6 review
frames CR-01 entirely as a look-alike/identity problem —
`carries_invisible_formatting`, registry keys, envelope roots. That is real and
I reproduced all of it. But `carries_visible_content` reads the *same* class fn,
and that is criterion 1's own question. **Nobody has named this half**, in six
rounds or in the review, and a fix scoped to the identity seams would leave it
standing.

### Judgment 2 — Is CR-01 in scope for THIS phase? Yes, decisively — but split it.

The tempting reading is that a general Unicode-identity hardening pass is its own
phase with its own research, and after six closure rounds that reading deserves a
hearing. It loses on three counts, and the third is dispositive.

1. **The threat classes land squarely on this phase's requirements.**
   U+202A–U+202E is Trojan Source (CVE-2021-42574) and I reproduced it as
   terminal spoofing in the tool's own `list` output. U+E0000–U+E007F is the
   standard LLM ASCII-smuggling carrier, in a phase named *prompt-injection
   hardening*, whose SAFE-07/SAFE-08 remit is exactly untrusted text reaching a
   model. These are not adjacent concerns.
2. **The phase wrote the contract itself.** 21-17's backstop truth 8 —
   *"No registry key, envelope root, credential scope, run directory name, or
   accepted phase token created by this build differs from another only by
   invisible formatting characters"* — is a phase-21 commitment, and it is
   measurably false at HEAD. A phase does not get to defer its own backstop.
3. **Criterion 1 is a ROADMAP success criterion of this phase and it is false.**
   That settles it independently of the other two. The class fix is not
   optional hardening; it is what makes a criterion the roadmap already promised
   become true.

**But split the work, because half of it genuinely is a different problem.**

- **IN SCOPE (phase 21, round 7):** derive the *invisible* class from the
  Unicode standard instead of enumerating it — `General_Category=Cf` plus
  `Default_Ignorable_Code_Point` plus the variation selectors and U+034F, from a
  checked-in generated table or a crate — and drive it into **both** judgments.
  Plus the property test over a standard-derived corpus, which is the only
  mechanism that can go red on a subset. This is bounded, it is one module, and
  it closes criterion 1, truth 8, and all five seams at once.
- **OUT OF SCOPE (new roadmap item):** TR39 confusables / homoglyphs. The doc's
  carve-out at `src/text.rs:90-96` is correct and should stay. None of the twenty
  values I reproduced is a homoglyph; every one is `Cf` or default-ignorable, so
  the carve-out never covered them. Do not let round 7's fix quietly grow into
  confusables — that is a different data table, a different false-positive
  profile, and a different conversation about non-Latin aliases.

### Judgment 3 — Is there a level below the character class? Yes. Two, and the second one terminates.

Arms → fields → character class. Each level was closed structurally and the next
one down was hand-enumerated. So what is below?

**Level 4: the SOURCE of the class — a hand list versus a standard.** The honest
move here is exactly what the reviewer recommends and I endorse it: stop writing
ranges and derive them from UCD `DerivedCoreProperties.txt`, or take
`unicode-security` / `unicode-properties` as a dependency. **What it costs, said
plainly:** a new dependency or a generated table plus its generator script; a
pinned Unicode version that goes stale on every UCD release, which is itself one
more hand-maintained thing and needs a test that fails loudly when the pin ages;
and a widening that must be checked against every acceptance fixture in the tree
so the tool does not start refusing `"2.1"`. That is real cost and it is worth
paying, because it converts an enumeration nobody can audit into a derivation
anybody can regenerate.

**Level 5, and this is the one that actually terminates: the DIRECTION of the
judgment.** Every round of this phase has enumerated what to **refuse**. A
deny-list over Unicode is 1.1 million code points and grows with every release —
it can always be one item short, and this phase has now proved that empirically
six times. An **allow-list** cannot be, because the accepted set is finite and
you can print it. For *identities* — aliases, run ids, phase tokens — the tree's
own acceptance fixtures are already `"20"`, `"2.1"`,
`"2026-08-19T12-00-00Z-aaaa"`, `"demo"`, `"99"`. `[A-Za-z0-9._-]` closes bidi,
tags, variation selectors **and** homoglyphs in one clause, with no table and no
dependency. Its cost is a product decision that must be recorded rather than
discovered: a user cannot name a project in a non-Latin script. Note this does
**not** work for free text — a `--goal` legitimately carries arbitrary Unicode,
so the emptiness judgment still needs the derived deny-list. Two judgments, two
directions, which is why level 4 is still required.

**And the honest bottom, since the question was asked.** There is no predicate
over code points that is complete for "renders identically", because rendering is
a property of the font and the shaping engine, not of Unicode. The only move with
no enumeration left in it is to stop letting user bytes **be** an identity:
generate the key, keep the user's string as a display label. That is the level
below all of them, and it is the only one that structurally cannot be one item
short. It is also a bigger change than round 7 should attempt — but it is what
this phase's six-round recurrence is actually pointing at, and it belongs in the
record.

### Observable Truths

#### ROADMAP success criteria (the contract)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✗ **FAILED** | **Correction to pass 6, not a regression.** The structural machinery is intact and I re-verified all of it (twelve-field no-`..` destructure, six `NonBlank` fields, `from_argv` pure and above `drive`, `execute_run` resolving at :2447 above :2449/:2493/:2626, `run.rs`'s only diff inside `mod tests`). But `from_argv` returns `Ok` for `--run-id`, `--goal`, `--command` and `--alias` each consisting of a single U+202E / U+00AD / U+E0041 / U+FE0F, and `run_paths` yields a run directory for each. `carries_visible_content` judges them VISIBLE because `is_invisible_formatting_char` is three literal ranges. The 7×6 matrix cannot see it: all six `DEGENERATE` payloads are U+200B/U+FEFF/whitespace. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `recheck_approval` at `run.rs:2410-2415`, above the source resolution and therefore above every write; unchanged this round. `driver_goal_seam` green inside my full-workspace run; decomposition → plan-digest → approval → recheck-at-spawn intact. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | `driver_escalation_cap` green in my own run — both directions (a cap at or above the *resolved* step cap refused at the seam; a cap below it accepted) and the typed park reason on the journal. Untouched by the round-6 diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | ⚠️ **PRESENT_BEHAVIOR_UNVERIFIED** | Reproduced the reviewer's WR-05 measurement myself: `cargo test --test driver_injection_corpus -- --list --ignored` → **10 tests**, being SEVEN class arms, two suppression controls, and `both_arms_of_every_class_comparison_were_really_executed` — the arms' own non-vacuity check, ignored alongside them. Only twelve *structural* pins execute under an ordinary run; they prove the channel is the only channel, not the model's behaviour on it. Last live run is an executor claim in `21-05-SUMMARY.md:514`, thirteen commits ago, never reproduced under verification. Present and wired; behaviour unexercised. → human verification. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED (coincidental-reliance) | `driver_refusal_record` and `driver_model_seam` green in my full run; the enum refusal itself is sound. Flagged because 21-17 truth 6 contracted to make the phase-token identity check a *property* and delivered it only for the covered characters: I measured `is_plain_path_component("2\u{202e}0")` → **true**, so that token still relies on roadmap membership. The criterion holds; the reason it holds is narrower than the round claims. |

**ROADMAP score: 3/5 verified, 1 FAILED, 1 behavior-unverified.**

#### Round-6 plan must-have truths (what the round contracted to deliver)

| # | Truth (source) | Status | Evidence |
|---|---|---|---|
| 6 | ONE production spelling of the invisible-character CLASS, shared by both judgments (21-17 t1) | ✗ **FAILED** | The **sharing** is real and I verified it — both predicates call `is_invisible_formatting_char`, and `grep -rln '2060' src/` returns only `src/text.rs` and the declared independent test-side oracle. The **class** is a subset of what the doc names: 22 code points against `Cf` + default-ignorable. Sharing a wrong class perfectly is how one hand-enumeration reached five seams at once. |
| 7 | `is_plain_path_component` refuses embedded invisible formatting; two identities that render identically can no longer resolve to two paths (21-17 t2) | ✗ **FAILED** | Measured `is_plain_path_component("demo\u{202e}")` → true; `envelope_dir_in` → two roots; `run_paths` → two directories. Pass 5's sentence, third pass running. |
| 8 | Registration closed at the ENTRY — `Alias` newtype, no fourth spelling survives (21-17 t3) | ✓ VERIFIED | Both `add_project` (:140) and `add_project_unchecked` (:186) take `&Alias`; private field, one fallible constructor, every clause delegating. My own predicate census of `src/registry.rs` finds no fourth spelling in executable code. The mechanism is genuinely right; its *reach* is scored at truths 6/7. |
| 9 | Every argv alias entry point classified — guard ten, 8-row census, planted-ninth control (21-17 t4) | ✓ VERIFIED | I counted eight `alias:` declarations in `src/cli.rs` myself (30, 35, 50, 306, 319, 337, 361, 371); the table is `[(&str, &str); 8]`; the control consumes `argv_alias_fields`, the same fn. (IN-02 residual: no row-variant existence check — Warning.) |
| 10 | `AliasNotVisible` replaces the borrowed `UnknownAlias` (21-17 t5) | ✓ VERIFIED | Measured: `from_argv(alias = "\u{200b}")` → `Err(AliasNotVisible { alias: "\u{200b}" })`. Same boundary, same purity, no registry claim. |
| 11 | The phase token's look-alike safety is a PROPERTY, not a precondition (21-17 t6) | ✗ **FAILED** | Measured false for the uncovered class: `"2\u{200b}0"` → refused (the claimed property), but `"2\u{202e}0"`, `"2\u{ad}0"`, `"2\u{e0041}0"`, `"2\u{fe0f}0"` → all accepted, all still relying on roadmap membership. |
| 12 | Disclosed residual: `from_argv` accepts look-alikes by design; identity judged at the seams (21-17 t7) | ✓ VERIFIED | The disclosure is accurate — I measured `from_argv` accepting a look-alike in every position, exactly as documented. (The "each of those seams now refuses look-alikes" half is scored at truth 7.) |
| 13 | **backstop** — no registry key, envelope root, credential scope, run directory or phase token differs from another only by invisible formatting (21-17 t8) | ✗ **FAILED** | Directly falsified, end to end against the built binary: nine registry keys all rendering as `demo`, two envelope roots, two run directories, an accepted look-alike phase token. |
| 14 | Guard nine reports every measured silent spelling; an allowlist entry cannot be silently repurposed (21-18 t1) | ✗ **FAILED** | The nine *fixed* spellings are genuinely fixed (I read `is_field_opener`, `names_token`, `without_trailing_comment` and the flush). The allowlist claim is false: `judge_declaration`'s `return` fires before `names_string_payload`, and the integrity pin is a `contains`, so `claude_args: (Vec<OsString>, String)` is silent AND passes the pin. |
| 15 | Guard nine's limits block claims exactly what its committed controls measure (21-18 t2) | ✗ **FAILED** | `is_field_opener` accepts only `pub `/`pub(`, so a field with no visibility modifier is skipped by BOTH the scan and the floor — `field_lines=12, protected=6, offenders=[]`, pass 6's exact signature, in a spelling named in neither list and bounded only by thirteen fixture crates the SUMMARY reports as retired. |
| 16 | `one_of_each` claims exactly what it delivers (21-18 t3) | ✓ VERIFIED | Read the whole doc (`:2077-2101`): the false "two ways / TWO places" claim is deleted, the residual is named in the artifact, and no pseudo-mechanism was substituted. Closed by honesty, which was the correct move. |
| 17 | The acceptance matrix sweeps all seven positions × four padded payloads, exemption deleted (21-18 t4) | ✓ VERIFIED | `src/driver/mod.rs:2608-2621` — one nested loop, no narrowing, no exemption comment. |
| 18 | `DEGENERATE` spelled in exactly one place tree-wide and the guard's message finally true (21-18 t5) | ✗ **FAILED** | The **consumption** half is closed and I verified it (cfg gate dropped, three pins consuming the const, one additive literal tree-wide). The **message** half is false: the scan detects a hand copy only if it carries one particular witness literal, while the message claims every blank-shape pin consumes the const. No standing violation exists — this is an overclaiming message, not a hole. |
| 19 | The record is corrected where round 6 inherited falsehoods (21-18 t6) | ✓ VERIFIED | `21-18-SUMMARY.md:260-290` names 21-16 truths 1 and 4 FALSE as shipped, with the measurement and the now-true statement for each. This is the single best thing the round did. |
| 20 | The CR-01 tracer's blind half is either seen or named (21-18 t7) | ✓ VERIFIED | `run.rs:3840-3852` asserts the real resolved envelope root received no write. I agree with the reviewer's D-18-6 adjudication: reading the real resolution is strictly stronger than the plan's proposed injected root. (IN-01 residual: nothing asserts the `Some` branch was taken — Info.) |

*(21-18's four probe truths restate ROADMAP criteria 3 and 4 and are scored there rather than double-counted.)*

**Score: 11/20 must-haves verified; 1 present-but-behavior-unverified; 8 failed.**

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/text.rs` (`is_invisible_formatting_char`) | The one production spelling of the invisible class | ✗ **SUBSET** | Three literal ranges, 22 code points. The doc at :90-96 names a wider class than the code implements. Reproduced: 20 of 22 probed invisible/bidi/tag/variation code points accepted. |
| `src/text.rs` (`carries_visible_content`) | The one production emptiness judgment | ⚠️ **HOLLOW** | Correct in shape, wrong in extent — returns `true` for a value of one U+202E / U+00AD / U+E0041 / U+FE0F. This is criterion 1's judgment and its failure is unreported anywhere before this pass. |
| `src/text.rs` (`carries_invisible_formatting`) | The identity judgment, second over the same class | ⚠️ **HOLLOW** | Genuinely a second, different question, genuinely sharing the class fn — and bounded by the same subset. |
| `src/test_support.rs` (`LOOK_ALIKE_PAIRS`) | A fixture that can falsify the predicate | ✗ **TAUTOLOGICAL** | Three pairs, all U+200B/U+FEFF, i.e. drawn from inside the predicate's own ranges. The doc at :49-52 claims the exact opposite property. Round-3 WR-03 one level down. |
| `src/test_support.rs` (`DEGENERATE`) | The one shared blank-shape const, reachable tree-wide | ✓ VERIFIED | `#[cfg(test)]` gate dropped (`src/lib.rs:21-23`); three `tests/` pins now consume it; one additive literal tree-wide. Six payloads, all inside the predicate's ranges — the same sampling limit as `LOOK_ALIKE_PAIRS`. |
| `src/registry.rs` (`Alias`, `add_project`) | Registration closed at the entry; no fourth predicate | ✓ VERIFIED | Newtype with private field and one fallible constructor; both registration fns take `&Alias`; predicate census clean. |
| `src/journal/mod.rs` (`is_plain_path_component`) | Blank + control + identity + structural, composed | ⚠️ **HOLLOW** | All four halves present and I read each one; the identity half delegates correctly. Accepts `"demo\u{202e}"`. |
| `src/error.rs` (`AliasNotVisible`) | A boundary refusal that does not narrate the registry | ✓ VERIFIED | Measured live. |
| `src/driver/mod.rs` (`DriveArgs`, `from_argv`) | Six `NonBlank` fields, twelve-field no-`..` destructure, pure | ✓ VERIFIED | Re-read field by field and line by line; the round's only diff at `from_argv` is the alias refusal variant. |
| `src/driver/mod.rs` (`one_of_each`) | An honest doc naming the residual | ✓ VERIFIED | Read in full. |
| `src/driver/mod.rs` (acceptance matrix) | 7 positions × 4 padded payloads, no exemption | ✓ VERIFIED | `:2608-2621`. |
| `src/driver/run.rs` (`execute_run`, record builders) | Source resolved above every write; builders total | ✓ VERIFIED | :2447 < :2449 < :2493 < :2626; the round's only `run.rs` diff is inside `mod tests`. |
| `src/driver/run.rs` (CR-01 tracer) | Project half AND envelope half asserted | ✓ VERIFIED | :3840-3852, reading the real resolved root. |
| `tests/spawn_seam_guard.rs` (guard nine) | Every measured spelling seen; honest limits block | ⚠️ **REACH OVERCLAIMED** | Nine fixed spellings genuinely fixed. Blind to a field with no visibility modifier (both scan and floor); allowlist integrity pin is a `contains`. |
| `tests/spawn_seam_guard.rs` (guard ten) | 8-row argv-alias census with a planted-ninth control | ✓ VERIFIED | Count independently confirmed at 8; control consumes the same extracted fn. |
| `tests/spawn_seam_guard.rs` (`…spelled_in_exactly_one_place`) | Tree-wide uniqueness, message true | ⚠️ **SCAN NARROWER THAN ITS MESSAGE** | One witness literal; under-detection direction undisclosed. No standing violation. |
| `.planning/REQUIREMENTS.md` | Accurate, untouched by the round | ✓ VERIFIED | Last touch `0c4f712`; all five read `[ ]` / `Gaps Found`. Fourth round holding. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `main.rs` (Add / five envelope arms) | `registry::Alias::new` | The argv-to-identity conversion before any identity-creating call | ✓ WIRED | Read the Add arm and `judged_alias_or_exit`; `Alias::new` runs above `load_config`. |
| `registry::Alias::new` | `text::` + `journal::` predicates | Every clause delegates; clause 4 carries the invariant | ✓ WIRED | Four clauses read individually; no clause judges locally except the documented whitespace usability rule. |
| `journal::is_plain_path_component` | `text::carries_invisible_formatting` | The identity clause, after `is_control` | ✓ WIRED | `journal/mod.rs:309-311`. |
| `payload::NonBlank::new` | `text::carries_visible_content` | Delegation, not re-implementation | ✓ WIRED | Confirmed; and this is exactly the link that carries the subset through to criterion 1. |
| `text::carries_visible_content` | the class of invisible characters | The `is_invisible_formatting_char` clause | ✗ **PARTIAL — 22 of the class** | The link exists and is single; its target is a hand-enumerated subset. |
| `test_support::LOOK_ALIKE_PAIRS` | a falsification of `carries_invisible_formatting` | Independent sampling | ✗ **NOT WIRED** | Sampled from inside the predicate's ranges; cannot falsify it. |
| `guard nine's scan` | every `DriveArgs` field declaration | `is_field_opener` token match | ⚠️ PARTIAL | Silent on a bare (no-`pub`) declaration, in both the scan and the floor. |
| `OSSTRING_ALLOWED` integrity pin | the declared type | Equality on the declaration | ✗ **NOT WIRED** | A `contains`, so a compound type inherits the suppression. |
| `guard ten's census` | `src/cli.rs` alias declarations | Exact-count scan + planted-ninth control | ✓ WIRED | 8 = 8, independently counted. |
| `driver_injection_corpus` arms | the real model boundary | Live `claude` spawn | ⚠️ **IGNORED** | All ten, including the arms' own non-vacuity meta-check. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `run.json` | `gsd_command` | `recorded_command(&IterationSource)`, total over two variants | ⚠️ | ⚠️ AMBIGUOUS — cannot be `""`; **can be a single U+202E**, which renders as nothing and reorders the surrounding text |
| `run.json` | `goal` | `args.goal.as_ref().map(NonBlank::as_str)` | ⚠️ | ⚠️ AMBIGUOUS — same class; `"   "` and `"\u{200b}"` refused, `"\u{202e}"` accepted |
| `run.json` / run directory | `run_id` | `NonBlank` + `is_plain_path_component` | ✗ | ✗ **DISCONNECTED** — `run_paths(planning, "\u{202e}")` → a real directory; `"…-aaaa"` and `"…-aaaa\u{202e}"` → two |
| `config.json` | registry key (alias) | `Alias::new` | ✗ | ✗ **DISCONNECTED** — nine keys rendering as `demo`, measured against the binary |
| `<envelope>/<alias>/` | envelope root | `envelope_dir_in` → `is_plain_path_component` | ✗ | ✗ **DISCONNECTED** — two roots for two look-alike aliases |
| `run.json` | `target_phase` | `Option<NonBlank>` + `is_plain_path_component` + roadmap membership | ⚠️ | ⚠️ AMBIGUOUS — safe by roadmap contents, not by validation |
| TUI / `list` ALIAS column | alias | `Display` on the raw key | ✗ | ✗ **SPOOFABLE** — `gsd-\u{202e}nur` renders as `gsd-run`; column padding also miscounts |

### Behavioral Spot-Checks

Every count- or presence-bearing check ran under `rtk proxy`; each negative grep
carries a positive control. Scratch integration crates were created, run, and
deleted — `git status` afterwards shows only the pre-existing untracked `.gsd/`
and `.planning/milestone.lock`.

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace suite (run once) | `cargo test --workspace --no-fail-fast -- --test-threads=2` | **1346 passed, 0 failed, 13 ignored** | ✓ PASS |
| Gate lint | `cargo clippy --lib -- -D warnings` | exit 0 | ✓ PASS |
| Unfiltered severity | `cargo clippy --all-targets` | 4 warnings, all pre-existing (`browser.rs:131-133`, `project_creator.rs:146`) | ✓ PASS (info) |
| **CR-01, library level** | scratch crate over `Alias::new` / `carries_*` / `is_plain_path_component` | **20 of 22** probed invisible/bidi/tag/VS code points ACCEPTED; only U+200B and U+FEFF refused | ✗ CONFIRMS CR-01 |
| **CR-01, binary level** | 9 × `gsd-meta-manager --config <scratch> add <fixture> "demo<X>"` | exit 0 ×9 (U+202E, U+202D, U+2066, U+061C, U+FE0F, U+E0001, U+00AD, U+13430, baseline); exit 1 only for U+200B. `config.json` → **9 distinct keys, all rendering `demo`** | ✗ CONFIRMS CR-01 |
| **Trojan Source spoofing** | `add … "gsd-\u{202e}nur"` then `list \| cat -v` | exit 0; `list` prints `gsd-M-bM-^@M-.nur` — **renders as `gsd-run`** in any bidi-aware terminal | ✗ CONFIRMS CR-01 (bidi half) |
| **Criterion 1, the emptiness half** | `DriveArgs::from_argv` over 4 argv positions × 6 payloads | U+202E / U+00AD / U+E0041 / U+FE0F → **`Ok` in all four positions**; U+200B and `"   "` → correctly typed `Err` | ✗ **FALSIFIES CRITERION 1** |
| **A run directory named by one invisible character** | `journal::run_paths(planning, p)` | `Some(".../runs/\u{202e}")`, and for U+2066/U+00AD/U+034F/U+E0041/U+FE0F/U+13430/U+180E/U+FFF9; `None` only for U+200B and whitespace | ✗ **FALSIFIES CRITERION 1** |
| **Phase-token identity (21-17 t6)** | `is_plain_path_component` | `"2\u{200b}0"` → false; `"2\u{202e}0"`, `"2\u{ad}0"`, `"2\u{e0041}0"`, `"2\u{fe0f}0"` → **all true** | ✗ CONFIRMS the precondition survives |
| **`LOOK_ALIKE_PAIRS` provenance** | printed every code point at runtime | 3 pairs, exactly two distinct invisibles (U+200B, U+FEFF), **both inside the predicate's ranges** | ✗ CONFIRMS WR-03 |
| **WR-05, the ignored set** | `cargo test --test driver_injection_corpus -- --list --ignored` | **10 tests: SEVEN class arms** (not eight), 2 suppression controls, and `both_arms_of_every_class_comparison_were_really_executed` | ✗ CONFIRMS WR-05 |
| Guard-ten census | `grep -nE 'alias: (String\|Option<String>),' src/cli.rs` | **8** (30, 35, 50, 306, 319, 337, 361, 371) vs an 8-row table | ✓ CONFIRMS the count |
| Registry predicate census | `grep -n 'is_empty()\|is_whitespace\|trim()' src/registry.rs` | Executable hits: `Alias::new` clause 3 (:107), `opt_in.prompt_inputs.is_empty()` (:389), `derive_alias` filter (:494) — **no fourth predicate** | ✓ CONFIRMS Claim B |
| `DEGENERATE` hand copies | `grep -rn 'for blank in \[' src/ tests/` | **1** hit (`src/text.rs:122`), additive not a subset | ✓ PASS |
| Round-6 diff scope at `run.rs` | `git diff -U0 0352dda..HEAD -- src/driver/run.rs` | 5 hunks, all ≥ :3766, all inside `mod tests` | ✓ CONFIRMS non-regression |
| Class unchanged since pass 6 | `git show 5b24022:src/text.rs` | Character class **byte-identical** to HEAD | ✓ CONFIRMS not a regression |
| Debt markers in the round-6 diff | `git diff 0352dda..HEAD -- src/ tests/` filtered on `^+` | **0** `TBD/FIXME/XXX`, **0** `TODO/HACK/PLACEHOLDER`; control needle `Alias` → **78** added lines, so neither zero is vacuous | ✓ PASS |
| REQUIREMENTS.md untouched | `git log -- .planning/REQUIREMENTS.md` | Last commit `0c4f712`, four rounds ago | ✓ PASS |
| Documented flakes | `driver_reattach`, `envelope_tracer` | Neither fired in my full-workspace run | ✓ NOT A GAP |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 04, 07, 09, 11, 13, 15, 17 | User states a goal once; driver pursues it without further input | ✗ BLOCKED | Criterion 1 FAILED — a `--goal` of one invisible character is accepted at the boundary and written verbatim into `run.json`. The alias that selects *which* project the goal is pursued against can also name nine projects that render identically. |
| DRIVE-03 | 21-01, 03, 04, 07, 08, 09, 11, 13, 15, 17 | Goal decomposed into a structured, machine-checkable, reviewable plan | ✗ BLOCKED | Criterion 1 FAILED. The record that is supposed to be evidence of the plan can carry `goal` and `gsd_command` values that render as nothing; for U+202E it also reorders the text around them. |
| DRIVE-04 | 21-02, 04, 06, 10, 12, 14, 16, 18 | Escalation capped per run; exceeding it parks | ✓ SATISFIED | Criterion 3 verified; both cap directions pinned and green in my own full run; untouched by the round-6 diff. |
| SAFE-07 | 21-01, 03, 05, 06, 08, 10, 12, 14, 16, 18 | `.planning/` content passed inside an explicit untrusted boundary | ? NEEDS HUMAN | Criterion 4 behavior-unverified. The structural pins execute and pass; all seven comparison arms, both suppression controls and the arms' own non-vacuity meta-check are `#[ignore]`d and have never run under verification. |
| SAFE-08 | 21-01, 05, 06, 12, 14, 16, 17 | Model's action constrained to a fixed enum; no free-form shell strings | ⚠️ PARTIAL | Criterion 5 verified — the enum refusal is sound and no shell string is constructed (I re-checked the `sh_quote` path the reviewer chased; it is clean). Held back because the phase-token identity check remains a precondition on roadmap contents for the uncovered class, and because the tag block U+E0000–U+E007F — the canonical LLM smuggling carrier — passes every identity seam. |

**No orphaned requirements.** The union of `requirements:` across all eighteen
plans is exactly {DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}, matching
REQUIREMENTS.md's phase-21 mapping and the ROADMAP `Requirements:` line.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/text.rs` | `:75-77` | The invisible-character class is a hand-enumerated subset of the class its own doc names | 🛑 Blocker | Falsifies ROADMAP criterion 1 and 21-17 backstop truth 8. Reproduced at library level and end to end against the built binary. Reaches every identity seam and the emptiness judgment at once, because the sharing this round built is real. |
| `src/test_support.rs` | `:49-57` | `LOOK_ALIKE_PAIRS` is sampled from inside the predicate's own ranges while its doc claims the anti-tautology property | 🛑 Blocker | The systemic cause. No pair in the const can be mishandled, so every LOOK_ALIKE assertion at every seam passes for any implementation covering U+200B and U+FEFF. Round-3 WR-03 re-entering one level down; it is why 1346/0 sits on a reproduced harm. |
| `tests/spawn_seam_guard.rs` | `:2992-2994`, `:3305-3312` | `is_field_opener` cannot see a field declared with no visibility modifier; the scan and the floor share the blindness | 🛑 Blocker (to 21-18 t1/t2) | Reproduces pass 6's exact signature (`field_lines=12 protected=6 offenders=[]`) in a spelling in neither the SEEN nor the SILENT list. Bounded only by thirteen fixture crates the SUMMARY reports as retired. |
| `tests/spawn_seam_guard.rs` | `:2853-2869`, `:3355-3366` | `judge_declaration` returns early on `OsString`; the allowlist integrity pin is a `contains`, not an equality | 🛑 Blocker (to 21-18 t1) | `claude_args: (Vec<OsString>, String)` is silent AND passes the pin that exists to catch exactly that. Falsifies "an allowlist entry cannot be silently repurposed" in truth 1 and the prohibition-3 table. |
| `src/main.rs` | `:126-131`, `:33-47` | The alias is rendered raw through `Display` in `list`, the TUI table and the refusal path | 🛑 Blocker (consequence of the class) | `gsd-\u{202e}nur` renders as `gsd-run`. Terminal spoofing in the tool's primary identity display. The `{:<20}` padding also miscounts, so the table misaligns — a visible symptom nobody would read as a security signal. |
| `tests/spawn_seam_guard.rs` | `:3446-3455`, `:3519-3549` | The `DEGENERATE` uniqueness scan detects a hand copy only via one witness literal while its message claims tree-wide coverage | ⚠️ Warning | Falsifies "the guard's message is finally true" (21-18 t5). No standing violation exists — I censused for one. Under-detection direction undisclosed, which is prohibition 3's own prohibition. |
| `.planning/…/21-18-SUMMARY.md` | `:389-401` | The SAFE-07 qualification says eight class arms; there are seven, and it omits the arms' own non-vacuity meta-check from the ignored set | ⚠️ Warning | The qualification itself is a real honesty improvement over 21-16, which carried the claim with no caveat at all. The enumeration backing it is wrong, and the omitted test is the only one that speaks to the comparison's non-vacuity. |
| `tests/spawn_seam_guard.rs` | `:3594-3629`, `:3747-3760` | Guard ten never checks that a census row's variant still exists in `cli.rs` | ⚠️ Warning | Renaming a variant, or removing one while adding another, leaves the count at 8 with a stale row describing a variant that is gone. The limits block names two approximations but not this one. |
| `src/driver/run.rs` | `:3840` | The CR-01 tracer's envelope half skips silently when `envelope_root()` is `None` | ℹ️ Info | I agree with the reviewer's G.2 adjudication: `establish_envelope`'s first line is `hooks::install(alias)?`, which cannot resolve a root either, so there is genuinely nothing to assert. The residual is that nothing observes the `Some` branch was taken — in a test that exists because a silently-vacuous assertion went unnoticed. |
| `src/main.rs` | `:77-90` | The `Add` arm judges the alias above `load_config`, inverting the ordering round 5's IN-04 documented | ℹ️ Info | Message ordering only; neither path writes and the new order is the better one (a pure refusal costs no file read). Recorded because this phase treats ordering facts as load-bearing and the change is undocumented at the site. |

**No unreferenced debt markers were introduced.** `git diff 0352dda..HEAD -- src/ tests/`
adds zero `TBD`/`FIXME`/`XXX` and zero `TODO`/`HACK`/`PLACEHOLDER`; the control
needle `Alias` returns 78 added lines, so neither zero is vacuous.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|---|---|---|---|---|---|---|
| `tests/driver_injection_corpus.rs` | SAFE-07 | 12 | **10** | No | Behavioral (in the skipped arms) | ⚠️ The requirement's behavioural proof is entirely in the ignored set, including the meta-check that both comparison arms ran |
| `tests/driver_escalation_cap.rs` | DRIVE-04 | 8 | 0 | No | Behavioral, both directions | ✓ |
| `tests/driver_goal_seam.rs` | DRIVE-01, DRIVE-03 | all | 0 | No | Value + behavioral | ✓ |
| `tests/spawn_seam_guard.rs` | SAFE-07, SAFE-08 | 35 | 0 | No | Value, with planted-defect controls | ⚠️ Two guards assert properties their scans cannot see (WR-01, WR-02) |
| `src/text.rs` `mod tests` | DRIVE-01, DRIVE-03 | 4 | 0 | **Effectively yes** | Value | 🛑 The corpus (`DEGENERATE`, `LOOK_ALIKE_PAIRS`) is sampled from inside the predicate's own ranges, so it compares the predicate against itself |

**Disabled tests on requirements:** 10, all on SAFE-07 → the requirement's only
behavioural arms. **Circular patterns detected:** 1 — not the classic
"generate expected values by running the system" shape, but the same epistemics:
a fixture drawn from the implementation's coverage cannot falsify the
implementation, and its doc claims it can. **Insufficient assertions:** 0.

### Decision Coverage

`21-CONTEXT.md`'s trackable decisions are honoured across the round-6 plans and
diff: D-17-1 (the identity clause), D-17-2 (registration closed at the entry),
D-17-3 (Remove raw-by-design, recorded at the site and in guard ten's table),
D-17-4 (`AliasNotVisible`), D-17-5 (`test_support` unconditional), D-17-6
(`envelope_dir_in`'s signature), D-18-1 (`one_of_each` closed by honesty), D-18-2
(`OsString` deny-by-default) and D-18-6 (the tracer reading the real envelope
root). Each appears in code, in a SUMMARY disclosure list, or in both. No
decision vanished during execution. Non-blocking, as this gate always is.

### Human Verification Required

One item. Everything else in this report is a code-level fact — reproduced
against the built library, reproduced end to end against
`target/debug/gsd-meta-manager` at HEAD, or read directly from the source.
Nothing is taken from a SUMMARY or from `21-REVIEW.md`.

#### 1. Execute SAFE-07's boundary live

**Test:** With an authenticated `claude` CLI available, run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
repository root. Record the CLI version beside the result.

**Expected:** 10 passed, 0 failed. Each `corpus_*_arrives_and_leaves_the_command_unchanged`
arm asserts the hostile payload ARRIVED at the model before asserting the command
was unchanged; the two suppression controls show the positive/negative
`CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
`both_arms_of_every_class_comparison_were_really_executed` confirms both arms
really ran.

**Why human:** All ten spawn the real model binary and require an authenticated
subscription, so they cannot execute inside verification. The twelve structural
pins that do execute prove the channel is the only channel — they cannot prove
the model's behaviour on it. The last recorded live run is an executor claim from
thirteen commits ago that no verification pass has reproduced.

### Gaps Summary

**What genuinely closed** — verified by direct reading and my own measurements,
not by trusting either SUMMARY: the `Commands` domain (eight argv alias fields,
one 8-row census, a planted-ninth control consuming the same fn); the
`registry::Alias` newtype, which is a real newtype that makes an unjudged alias
unrepresentable at both registration signatures and deletes the fourth predicate
rather than relocating it; `AliasNotVisible` replacing a refusal that told a
falsehood about the registry; `one_of_each` closed by honesty with no
pseudo-mechanism substituted; the acceptance matrix's hand-exemption deleted;
`DEGENERATE`'s three hand copies retired by dropping a `#[cfg(test)]` gate; the
CR-01 tracer's envelope half asserted against the real resolved root; and — the
best thing the round did — explicit Record corrections naming two of the previous
round's truths FALSE as shipped. REQUIREMENTS.md untouched for the fourth round.
Criterion 1's structural machinery is intact and I re-attacked it and could not
break the *structure*.

**Three gaps remain, and the first one is a correction to my predecessor.**

**Gap 1 — criterion 1 was never verified.** Pass 6's headline finding does not
survive. `carries_visible_content` — the emptiness judgment `NonBlank` rests on
and therefore the judgment criterion 1 rests on — reads a class of 22
hand-written code points, so a payload of one U+202E, U+00AD, U+E0041 or U+FE0F
is judged *visible*, accepted in all four argv string positions, and can name a
run directory or land verbatim in a committed `run.json`. This is pass 5's
reproduction with a different character. Nothing in the round-6 diff caused it —
the class is byte-identical to pass 6's — and nothing in `21-REVIEW.md` names it
either, because the review frames CR-01 entirely as an identity problem. **The
emptiness half is the half nobody has said out loud, and a fix scoped to the
identity seams would leave it standing.**

**Gap 2 — the identity class, reproduced end to end.** Nine registry keys that
all render as `demo`, two envelope roots, two run directories, an accepted
look-alike phase token, and an alias that prints as `gsd-run` in the tool's own
`list`. The reviewer's CR-01 is correct in every particular I checked. The
systemic cause is the one the reviewer names and it is the important sentence in
this report: `LOOK_ALIKE_PAIRS` is built from the two characters the predicate
already covers, while its doc claims it is built to falsify the predicate.
Literal spelling is not independence. **Six rounds have now sampled their test
corpus from inside the thing under test, at three successive levels.**

**Gap 3 — the anti-recurrence machinery, fifth round running.** Guard nine cannot
see a field declared without `pub`, in both its scan and its floor, producing
pass 6's exact measurement signature in a spelling its own SEEN and SILENT lists
both omit — and what actually prevents it is the thirteen-fixture coincidental
bound the SUMMARY reports as retired. Its allowlist integrity pin is a `contains`
rather than an equality, so the one thing it exists to catch passes it. The
`DEGENERATE` uniqueness guard's message is one literal wide. And the SAFE-07
qualification, which is otherwise the round's most honest paragraph, miscounts
its own ignored set and omits the arms' own non-vacuity check from it.

### Recommendation

**Round 7 is warranted, here in phase 21, and it should be small.** Three of the
four things it must do are one module each.

**1. Derive the character class instead of enumerating it, and fix BOTH
judgments.** `General_Category=Cf` + `Default_Ignorable_Code_Point` + the
variation selectors + U+034F, from a checked-in table generated from UCD
`DerivedCoreProperties.txt` or from `unicode-security`/`unicode-properties`.
Record the maintenance obligation honestly: a pinned Unicode version that goes
stale, and a test that fails loudly when it does. Fix `carries_visible_content`
in the same commit as `carries_invisible_formatting` — the emptiness half is
what criterion 1 needs and it is the half nobody has named.

**2. Give the class a corpus that can falsify it.** A property test iterating a
standard-derived corpus, not a fixture. Adding two hand-chosen pairs to
`LOOK_ALIKE_PAIRS` is necessary and nowhere near sufficient — it is one more
hand-chosen sample, which is the move that has now failed six times. And correct
the const's doc, which currently asserts a property it does not have.

**3. Consider inverting the identity judgment to an allow-list, separately.**
`[A-Za-z0-9._-]` for aliases, run ids and phase tokens closes bidi, tags,
variation selectors and homoglyphs in one clause with no table and no dependency,
and it cannot be one item short because the accepted set is finite. Free text
keeps the derived deny-list. If this is adopted, the cost — a user cannot name a
project in a non-Latin script — must be recorded as a chosen trade at the site,
not discovered by whoever hits it.

**4. Fix the two guards in the same commits.** Widen `is_field_opener` to a bare
identifier before `:` and plant that spelling in the control; stop returning
early from the `OsString` branch and make the integrity pin an equality; give the
`DEGENERATE` scan two witnesses and name the under-detection direction. Then
correct the SAFE-07 qualification's arithmetic and move "SAFE-07's boundary has
never been executed under verification" into `deferred-items.md`.

**What is NOT in scope:** TR39 confusables. The carve-out at `src/text.rs:90-96`
is correct, none of the twenty values I reproduced is a homoglyph, and letting
round 7 grow into confusables is how a bounded fix becomes another six rounds.
Open it as its own roadmap item.

**One thing to carry forward without re-litigating, and one to stop carrying.**
The type-level mechanism — `NonBlank`, `Alias`, `from_argv`'s no-`..` destructure
— has now survived three consecutive rounds of direct attack and I could not
break it either. It is right. What must stop being carried forward is the belief
that a hand-written literal fixture is independent of the predicate it tests.
Rounds 2–4 hand-enumerated arms; round 5 hand-enumerated fields; round 6
hand-enumerated the character class; and every one of those rounds wrote its
verifying fixture out of the same head that wrote the implementation. The
enumeration keeps moving down a level, and the *sampling* has never moved at all.
That is the actual recurrence, and step 2 above is the only item on this list
that addresses it.

---

_Verified: 2026-08-23T03:23:33Z_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `f1faa3e` · seventh verification pass · pass 6 preserved at `253de44`_
