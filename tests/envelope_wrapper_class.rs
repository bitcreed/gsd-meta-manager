// ============================================================================
// The control over the CLASS `T-19-60` belongs to — not over the six lines
// `19-SECURITY.md` happened to measure.
//
// **What this file is.** A generative, fixed-seed property that the guard's
// verdict is INVARIANT under anything that changes which token is the effective
// program without appearing as the first token: any chain of wrappers of any
// depth, spelled by basename or by absolute path, prefixed with any leading
// `NAME=VALUE` assignments, quoted any legal way, optionally handed to a shell
// as a `-c` payload. For every generated case the assertion is
// `verdict(wrap(base)) == verdict(base)`, where the right-hand side is MEASURED
// FROM THE UNWRAPPED COMMAND IN THE SAME RUN.
//
// **What this file is NOT.** It is not a row per named wrapper. `19-11` proved
// the fix against six enumerated lines, which is the right RED corpus and the
// wrong final control: a corpus of named instances is structurally incapable of
// failing on the class it is taken to certify. It would be green on the day it
// landed and silent on the day someone closed a new bypass by adding a name to a
// list — the failure mode this project has now shipped three times.
//
// **Three things make this non-vacuous rather than merely large.**
//
// 1. The invariance right-hand side is measured, not written down. A suite that
//    quietly stopped refusing the bases would FAIL here rather than pass more
//    easily, because the unwrapped verdict is asserted to be a refusal carrying
//    a `REASON_*` identifier before the wrapping loop runs at all.
// 2. The wrapper alphabet is deliberately over-broad and contains names that
//    appear NOWHERE in the production logic — including `made-up-wrapper-9000`,
//    which exists only in test code. A mechanical control asserts their absence,
//    so a fix that reverted to an enumerated wrapper list turns this corpus RED
//    instead of green (`T-19-76`).
// 3. The SAME generator runs over a corpus of commands that must be PERMITTED
//    (`T-19-78`, D-32). Without it a guard that refused every tool call would
//    satisfy every assertion in the refusal half. `ProgramResolution::Ungoverned`
//    is a PERMIT and it is the ANSWER, not a fall-through: this guard sees every
//    Bash tool call, and a rule that denied what it did not recognise would deny
//    `ls`.
//
// **Offline, agent-free, clock-free and repository-free** (D-35, `T-19-80`). The
// refused corpus admits only bases that are judged from argv alone: a `git push`
// with NO refspec is deliberately absent, because `policy::push_needs_resolved_dests`
// is true for exactly that shape and makes the guard shell out to `git` in the
// test's own working directory — which at this case count is minutes of runtime
// and a verdict that depends on the repository the test happens to run in. That
// one shape is covered by an enumerated row in `tests/envelope_wrapper_bypass.rs`
// instead. One `TempDir` serves the whole property, because no case here writes
// a ledger line.
//
// **No crate was added** (`T-19-SC`). The generator below is a hand-rolled
// fixed-seed LCG; there is no property-testing crate and no shell-parsing crate,
// and `Cargo.toml`/`Cargo.lock` are untouched by this plan.
// ============================================================================

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use gsd_meta_manager::envelope::{hooks, policy};
use tempfile::TempDir;

/// The alias every case drives.
const ALIAS: &str = "alpha";

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/// One guard answer, reduced to the two things a verdict IS.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Verdict {
    /// The hook protocol's exit code: 0 permits, 2 blocks.
    code: i32,
    /// D-24's taxonomy identifier, extracted from the decision JSON rather than
    /// the whole message — so a detail that legitimately quotes a token out of
    /// the command line it refused is not mistaken for a change of verdict.
    reason_id: String,
}

/// What [`Verdict::reason_id`] carries for a permit. A permit writes nothing at
/// all, by design, so there is no identifier to read.
const PERMIT: &str = "permit(the guard answered nothing)";

/// What [`Verdict::reason_id`] carries for a refusal whose message names no
/// member of D-24's taxonomy.
const UNIDENTIFIED: &str = "refused-without-a-D-24-identifier";

/// Every identifier D-24's taxonomy defines, read from the constants rather than
/// re-spelled here — a second spelling of a fact is a second thing to keep in
/// step.
const REASON_IDENTIFIERS: &[&str] = &[
    policy::REASON_PUSH_OUTSIDE_NAMESPACE,
    policy::REASON_FORCE_PUSH_BLOCKED,
    policy::REASON_HOOK_BYPASS_BLOCKED,
    policy::REASON_SECRET_DETECTED,
    policy::REASON_PR_CAP_EXCEEDED,
    policy::REASON_CREDENTIAL_UNAVAILABLE,
    policy::REASON_ENVELOPE_ASSERTION_FAILED,
];

/// Ask the guard about one shell command and reduce its answer to a [`Verdict`].
///
/// Driven in-process through `hooks::guard_in` against a config path that does
/// not exist, so `resolve_policy` degrades to the TIGHTER defaults — which is
/// what makes this fixture's namespace and caps the documented ones.
fn ask(envelope_root: &Path, command: &str) -> (Verdict, String) {
    let request = serde_json::json!({
        "session_id": "fixture",
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": { "command": command },
    })
    .to_string();

    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = hooks::guard_in(
        envelope_root,
        &envelope_root.join("no-such-config.json"),
        ALIAS,
        None,
        request.as_bytes(),
        &mut out,
        &mut err,
    )
    .expect("the guard answers rather than erroring");

    let stdout = String::from_utf8(out).expect("the guard's decision is UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or(serde_json::Value::Null);
    let reason = value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let reason_id = if reason.is_empty() {
        PERMIT.to_string()
    } else if let Some(id) = reason
        .split_once("(reason: ")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(id, _)| id.to_string())
    {
        id
    } else {
        REASON_IDENTIFIERS
            .iter()
            .find(|id| reason.contains(**id))
            .map(|id| (*id).to_string())
            .unwrap_or_else(|| UNIDENTIFIED.to_string())
    };

    (Verdict { code, reason_id }, stdout)
}

/// The verdict alone, for the common case.
fn verdict(envelope_root: &Path, command: &str) -> Verdict {
    ask(envelope_root, command).0
}

// ---------------------------------------------------------------------------
// The generator — a hand-rolled fixed-seed LCG (`T-19-SC`)
// ---------------------------------------------------------------------------

/// **The named seed.** A failure below is reproducible from the command the
/// message prints verbatim; the seed is what makes the whole corpus reproducible
/// rather than only the case that happened to fail.
const SEED: u64 = 0x1912_C0DE_5EED_0060;

/// Knuth's multiplicative LCG over `u64`. Five lines, no crate.
struct Lcg(u64);

impl Lcg {
    fn new() -> Self {
        Lcg(SEED)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        // The high bits are the well-mixed ones in an LCG; the low bit of a
        // power-of-two-modulus LCG alternates, and picking on it would make the
        // corpus a checkerboard rather than a sample.
        self.0 >> 33
    }

    fn pick(&mut self, len: usize) -> usize {
        (self.next() as usize) % len
    }
}

/// Leading `NAME=VALUE` prefixes, which the shell's own grammar makes part of
/// the command rather than its program.
///
/// **No envelope key appears here, and that is deliberate rather than an
/// oversight.** `GIT_CONFIG_COUNT=0 …` legitimately changes the verdict — it is
/// refused on its own account under `hook_bypass_blocked` for what it does to
/// the git-hook layer, BEFORE resolution reaches the command behind it — so
/// including it here would break invariance by being MORE strict, not less. It
/// is an enumerated row in `tests/envelope_wrapper_bypass.rs`, and its
/// interaction with the `T-19-74` residual is pinned at the bottom of this file.
/// **`T-19-89`, the Rule A half.** Audit 3 read all five alphabets in this file
/// and found that no entry of any of them contains a `$`, a `` ` ``, a `{` or a
/// `(` — so the corpus could not GENERATE, and therefore could not FAIL ON, the
/// class that walked through it. The two entries below are the direct repair of
/// that finding for this alphabet, and both are chosen so the verdict stays
/// genuinely INVARIANT:
///
/// * a value carrying an expansion. `resolve_program` step 2b tests an
///   assignment's value against the envelope keys and step 7 tests it against
///   `GOVERNED_PROGRAMS`; `$BAR` is neither, and the step-5 prefix rule looks
///   only at words AFTER the assignment prefix. So the verdict is the base's
///   own, wrapped or not.
/// * a value that is a **proper prefix** of an envelope key rather than a key.
///   `GIT_CONFIG` matches no entry of `ENVELOPE_ENV_KEYS` exactly and no
///   `_`-terminated entry by prefix, so it is not refused on its own account —
///   which is the whole shape of the measured `C=GIT_CONFIG; env -u ${C}_COUNT`
///   line, drawn as an alphabet entry rather than typed once.
///
/// **What must NOT be added here, and why**, in the shape this file already
/// records for the `GIT_CONFIG_COUNT=0` prefix: a prefix whose value IS an
/// envelope key (step 2b) or IS a governed program (step 7) is refused on its
/// own account, BEFORE resolution reaches the command behind it. Folding one in
/// would break the invariance property by being STRICTER rather than laxer,
/// which is a red for the wrong reason.
const ASSIGNMENT_PREFIXES: &[&str] = &[
    "",
    "FOO=bar ",
    "LC_ALL=C TZ=UTC ",
    "EMPTY= ",
    "FOO=$BAR ",
    "C=GIT_CONFIG ",
    // -----------------------------------------------------------------------
    // `T-19-95`, the round-5 half — a value carrying a PATHNAME-EXPANSION
    // character.
    //
    // **Why a glob and NOT a brace expansion here**, which is the substance of
    // the split this round makes between the alphabets. This alphabet feeds
    // properties that assert the wrapped verdict EQUALS the unwrapped one, and
    // it is shared with `PERMITTED_BASES`. `FOO={a,b} git status` is refused on
    // its own account — `}` is a word-splitting closer, so Rule B refuses the
    // segment behind it, measured exit 2 under `envelope_assertion_failed` — and
    // after `19-17` it is a pinned COST row. Folding one in would make the
    // permitted corpus STRICTER than its base and break invariance in the same
    // direction this file already records for the `GIT_CONFIG_COUNT=0` prefix.
    //
    // A glob in an assignment VALUE is nobody's decision word, sets no expansion
    // bit and severs nothing, so its verdict is genuinely invariant: measured
    // exit 0 with `git status` behind it and exit 0 with `ls -la` behind it.
    "GLOB=*.rs ",
];

/// The wrapper alphabet: thirty whole prefixes, each with its own options.
///
/// **Over-broad on purpose.** Eleven of these names appear nowhere in the
/// production logic, and one of them —`made-up-wrapper-9000` — is not a program
/// that exists. That is the point of the exercise: the resolver must never ask
/// what the wrapper is *called*, so a name it has never heard of and `env` must
/// be indistinguishable to it. `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
/// below is the mechanical control that keeps this true.
const WRAPPERS: &[&str] = &[
    "env",
    "/usr/bin/env",
    "env -i",
    "env -u SOME_VAR",
    "busybox env",
    "command",
    "timeout 60",
    "/usr/bin/timeout 5",
    "nohup",
    "nice -n 10",
    "stdbuf -oL",
    "setsid",
    "ionice -c3",
    "chrt -b 0",
    "taskset -c 0",
    "doas",
    "runuser -u me --",
    "sudo -E",
    "xargs -n 99",
    "time",
    "flock /tmp/lock",
    "strace -f",
    "ltrace",
    "unshare -r",
    "firejail",
    "torsocks",
    "catchsegv",
    "proot",
    "nsenter --",
    // The one that is the whole argument. There is no such program. A fix built
    // on a list of wrapper names cannot possibly contain it, so a corpus that
    // includes it can fail on the class in a way a corpus of real names cannot.
    "made-up-wrapper-9000 --flag",
    // `T-19-95`, the round-5 half — a wrapper whose OPERAND carries a
    // pathname-expansion character. Invariant for the same reason the glob
    // assignment value above is: the operand is nobody's decision word, it sets
    // no expansion bit and it severs nothing. Measured exit 0 with `git status`
    // behind it and exit 0 with `ls -la` behind it.
    //
    // **A brace expansion does NOT belong here** — it would sever the base and
    // make this alphabet stricter than its own bases — and neither does a
    // QUOTED literal brace pair, which is the other spelling that would keep the
    // verdict invariant: the outer `ShellLayer`s quote the whole payload, so a
    // `'` or a `"` inside a wrapper entry produces illegal shell under
    // `ShSingleQuoted` or `BashDashLC` respectively, and a generator that
    // emitted illegal shell would be exercising the splitter's error path while
    // claiming to exercise the class. The literal-brace pair is drawn by
    // `PERMITTED_BASES` instead, where it sits at the END of the line and its
    // closer has nothing left to sever.
    "env -u SOME_VAR*",
];

/// The optional outer shell layer, so the payload half of the class is generated
/// too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellLayer {
    /// No outer shell.
    None,
    /// `sh -c '<cmd>'` — the spelling 19-05 deviation 2 covered.
    ShSingleQuoted,
    /// `bash -lc "<cmd>"` — the BUNDLED short-flag spelling an exact `-c` match
    /// did not cover, and the reason `19-11` deleted `NESTED_SHELLS` rather than
    /// extending it.
    BashDashLC,
    /// `{ <cmd>; }` — a brace GROUP (`T-19-89`, plan 19-15).
    ///
    /// One of the two shapes audit 3 named as absent from every alphabet in this
    /// file, and one of the two that exercise the SPLITTER rather than the
    /// resolver. It preserves invariance because bash's own grammar makes `{` a
    /// reserved WORD — it must be followed by whitespace, and the list inside must
    /// be terminated by `;` or a newline before `}` — so no word is in progress at
    /// either boundary and neither operator is a word-splitting flush. The
    /// enumerated rows in `tests/envelope_expansion_slots.rs` assert that claim
    /// directly; this layer makes the corpus able to DRAW it.
    BraceGroup,
    /// `( <cmd> )` — a SUBSHELL (`T-19-89`, plan 19-15).
    ///
    /// The other absent shape. Invariance is preserved for the same reason: the
    /// spellings written here put whitespace inside the parentheses, so no word is
    /// in progress when either character arrives.
    Subshell,
}

impl ShellLayer {
    /// Which quote character may appear INSIDE this layer.
    ///
    /// A single-quoted outer payload cannot contain a single quote and a
    /// double-quoted one cannot contain a double quote — so the quoting
    /// dimension is constrained by the layer rather than generated blind. A
    /// generator that emitted illegal shell would be exercising the splitter's
    /// error path while claiming to exercise the class.
    fn inner_quotes(self) -> &'static [&'static str] {
        match self {
            ShellLayer::None => &["", "'", "\""],
            ShellLayer::ShSingleQuoted => &["", "\""],
            ShellLayer::BashDashLC => &["", "'"],
            // Neither grouping layer introduces an outer quote of its own, so
            // both quote characters remain legal inside it.
            ShellLayer::BraceGroup | ShellLayer::Subshell => &["", "'", "\""],
        }
    }

    fn apply(self, inner: &str) -> String {
        match self {
            ShellLayer::None => inner.to_string(),
            ShellLayer::ShSingleQuoted => format!("sh -c '{inner}'"),
            ShellLayer::BashDashLC => format!("bash -lc \"{inner}\""),
            // The whitespace and the `;` are REQUIRED by bash, not cosmetic: `{`
            // is a reserved word and `}` must follow a `;` or a newline. Emitting
            // `{git status}` would be illegal shell, and a generator that emitted
            // illegal shell would be exercising the splitter's error path while
            // claiming to exercise the class.
            ShellLayer::BraceGroup => format!("{{ {inner}; }}"),
            ShellLayer::Subshell => format!("( {inner} )"),
        }
    }
}

const SHELL_LAYERS: &[ShellLayer] = &[
    ShellLayer::None,
    ShellLayer::None,
    ShellLayer::ShSingleQuoted,
    ShellLayer::BashDashLC,
    ShellLayer::BraceGroup,
    ShellLayer::Subshell,
];

/// One generated wrapping, and the recipe that produced it.
struct Wrapped {
    command: String,
    /// The wrapper-name chain alone, joined — what "a distinct wrapper chain"
    /// counts.
    chain: String,
    /// The whole recipe, for a failure message that can be reproduced by hand.
    recipe: String,
}

/// Wrap `base` in one randomly drawn assignment prefix, wrapper chain (depth 0
/// to 3), per-wrapper quoting and optional outer shell layer.
fn wrap(rng: &mut Lcg, base: &str) -> Wrapped {
    let prefix = ASSIGNMENT_PREFIXES[rng.pick(ASSIGNMENT_PREFIXES.len())];
    let layer = SHELL_LAYERS[rng.pick(SHELL_LAYERS.len())];
    let quotes = layer.inner_quotes();
    let depth = rng.pick(4);

    let mut chain_names: Vec<&str> = Vec::with_capacity(depth);
    let mut spelled = String::new();
    for _ in 0..depth {
        let wrapper = WRAPPERS[rng.pick(WRAPPERS.len())];
        chain_names.push(wrapper);

        // Quote the wrapper's PROGRAM WORD, which is the meaningful quoting
        // variation: `"env" -i` and `env -i` are the same command, and a
        // resolution that compared raw token text rather than the recovered word
        // would tell them apart.
        let quote = quotes[rng.pick(quotes.len())];
        match wrapper.split_once(' ') {
            Some((program, rest)) => {
                spelled.push_str(&format!("{quote}{program}{quote} {rest} "));
            }
            None => spelled.push_str(&format!("{quote}{wrapper}{quote} ")),
        }
    }

    let inner = format!("{prefix}{spelled}{base}");
    Wrapped {
        command: layer.apply(&inner),
        chain: chain_names.join(" | "),
        recipe: format!("prefix={prefix:?} layer={layer:?} chain=[{}]", chain_names.join(", ")),
    }
}

// ---------------------------------------------------------------------------
// The corpora
// ---------------------------------------------------------------------------

/// Commands that must be REFUSED, every one of them judged from argv alone.
///
/// **A `git push` with no refspec is deliberately absent.**
/// `policy::push_needs_resolved_dests` is true for exactly that shape, and it
/// makes the guard shell out to `git` in the test's own working directory. At
/// this case count that is minutes of runtime and a verdict that depends on the
/// repository the test happens to run in — a property that consults a repository
/// per case is `T-19-80`. Every base below either names a refspec or carries a
/// denied flag, so `push_needs_resolved_dests` answers `false` for all of them.
/// The excluded shape is covered by an enumerated row in
/// `tests/envelope_wrapper_bypass.rs`.
const REFUSED_BASES: &[&str] = &[
    "git push --force origin main",
    "git push -f origin main",
    "git push -fu origin main",
    "git push --force-with-lease origin main",
    "git push origin +refs/heads/main",
    "git push --no-verify origin refs/heads/gsd-auto/alpha/w",
    "git push origin main:main",
    "git push --delete origin main",
    "git stash",
    "git update-ref -d refs/heads/main",
    "git config core.hooksPath /tmp/x",
    "git -c core.hooksPath=/tmp/x status",
    // -----------------------------------------------------------------------
    // `T-19-89`, the Rule A half — an expansion in the base's own VERB SLOT.
    //
    // **These three are RED against the pre-Rule-A tree at the property's own
    // floor 2**, which measures every base refused UNWRAPPED before the
    // wrapping loop runs. That is exactly the non-vacuity this widening is
    // about: the alphabet above could not draw a `$`, so 1680 generated cases
    // certified a fix that the very next audit walked around with two
    // characters — `git $V`.
    //
    // **FORGE bases are deliberately kept out of this alphabet.** The
    // invariance property shares ONE envelope root across all its cases, and a
    // forge base that is permitted before the fix writes ledger lines and
    // exhausts the 3/1 PR cap — which would turn later cases red for a reason
    // that has nothing to do with the class and muddy the RED evidence. The
    // forge slots are covered by
    // `a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot`
    // below, which builds a fresh root per case for exactly that reason.
    "git $V --force origin main",
    "git ${V}push --force origin main",
    "git $(true)push --force origin main",
    // -----------------------------------------------------------------------
    // `T-19-95`, the round-5 half — the classes that make a word UNREADABLE
    // rather than the ones that MARK an expansion.
    //
    // **Every entry below is measured at exit 0 against the pre-`19-17` tree**,
    // so all of them are RED at the invariance property's own floor 2, which
    // measures every base refused UNWRAPPED before the wrapping loop runs. That
    // is the non-vacuity this widening exists for: audit 4 verified by reading
    // every entry of every alphabet in this file that **not one contained a
    // comma inside braces, a `*`, a `?` or a `[`**, so 2100 generated cases
    // certified a fix against a class the corpus could not draw. This is
    // `T-19-76`'s failure mode for the FOURTH consecutive round.
    //
    // **The classes, and why a single "brace expansion" class is not enough.**
    // `{a,b}` standing as its own word satisfies a whole-word splice floor and
    // nothing else. A corpus of whole-word splices cannot fail on
    // `{g..g}it push --force origin main`, and a corpus of SINGLE-BRACE entries
    // cannot fail on `{g..g}{i..i}t push --force origin main` — both measured at
    // exit 0 here, both found in plan-check one round apart. So the classes are
    // split and each is drawn separately.
    //
    // **GENUINE vs. COVERAGE, measured under `bash` against argv-printing
    // shims.** A comma list of N alternatives produces N WORDS, so a
    // concatenated COMMA splice in a program slot always leaves a surplus word
    // in the subcommand slot that real git rejects
    // (`git git push --force …` -> `git: 'git' is not a git command`). Only a
    // RANGE with equal endpoints produces exactly ONE word. The comma spellings
    // are kept because the guard is blind to them identically and one character
    // separates them from the range spellings, but they are CLASS COVERAGE
    // rather than reproduced harms and `tests/envelope_literal_decision.rs`
    // records which is which, row by row, with the printed argv.
    //
    // WHOLE-WORD splice, verb slot and position 0:
    "git {push,--force} origin main",
    "{git,push,--force,origin,main}",
    "{env,git} push --force origin main",
    "git {--no-pager,push} --force origin main",
    "git {stash,-q}",
    "git {update-ref,-d,refs/heads/main}",
    "git {config,core.hooksPath,/tmp/x}",
    // CONCATENATED splice — a non-empty literal run adjacent to the braces in
    // the SAME word, which no whole-word entry above can satisfy:
    "{g..g}it push --force origin main",
    "g{i,i}t push --force origin main",
    "g{it,x} push --force origin main",
    "{g,x}it push --force origin main",
    // CONCATENATED splice in a FLAG slot — the one comma spelling that is
    // GENUINE, because git accepts a repeated `--force` so the surplus word is
    // harmless. The refspec is explicit and in-namespace, so
    // `push_needs_resolved_dests` answers false and no repository is consulted.
    "git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}",
    // RANGE, including the INCREMENT form, which is neither a comma list nor a
    // two-endpoint range and so falls outside every enumerability trigger
    // written for the other spellings:
    "{g..g..1}it push --force origin main",
    // MULTI-EXPANSION — two or more brace expansions in ONE word, which no
    // per-`{` product computation reaches because bash composes ACROSS `{`s:
    "{g..g}{i..i}t push --force origin main",
    "{g,g}{i,i}{t,t} push --force origin main",
    // A NESTED expansion inside an alternative, where a decomposition into
    // TOP-LEVEL alternatives yields `g{i,i}t` and `x` and clears both:
    "{g{i,i}t,x} push --force origin main",
    // PATHNAME expansion in a decision slot — the verb, and `config`'s own key
    // operand, which `19-14` closed for `$` and which a glob reaches through a
    // character that sets no expansion bit at all:
    "git pus? --force origin main",
    "git ?ush --force origin main",
    "git stas?",
    "git config core.hooksPat? /tmp/x",
    // TILDE — class coverage, labelled as such here and in
    // `tests/envelope_literal_decision.rs`: bash leaves `~push` alone when no
    // such user exists, so the shell does not assemble a force push. It is drawn
    // because the guard cannot know whether such a user exists on the machine
    // the command will run on.
    "git ~push --force origin main",
    // A LITERAL brace pair inside a command that is refused BEFORE and AFTER.
    // This is a CONTROL rather than a reproducer, and it is the sharpest one in
    // the file: today the tokenizer fragments `HEAD@{0}` and the guard refuses
    // the fragment `git reflog delete HEAD@`; after `19-17` the word survives
    // whole and the guard refuses `reflog delete HEAD@{0}`. A change that
    // absorbed literal braces while LOSING a refusal turns this red and nothing
    // else in the suite.
    "git reflog delete HEAD@{0}",
];

/// How many wrappings each refused base gets. 37 x 140 = 5180, over the 1500
/// floor the plan sets.
const VARIANTS_PER_REFUSED_BASE: usize = 140;

/// The floors, asserted BEFORE the invariance loop so this file cannot pass
/// having generated nothing that matters (`T-19-77`).
const MIN_REFUSED_CASES: usize = 1500;
const MIN_DISTINCT_CHAINS: usize = 200;
const MIN_DISTINCT_REASONS: usize = 3;

// ---------------------------------------------------------------------------
// 1. The invariance property over the refused class
// ---------------------------------------------------------------------------

#[test]
fn the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // --- floor 1: the corpus exists at all -------------------------------
    assert!(
        !REFUSED_BASES.is_empty(),
        "an empty base corpus makes every assertion below vacuous"
    );

    // --- floor 2: every base is refused UNWRAPPED, with an identifier -----
    //
    // This is the invariance property's right-hand side, and measuring it here
    // rather than writing it down is what makes the property TIGHTEN if the
    // guard ever stops refusing these. A corpus whose bases had quietly become
    // permits would fail here, not pass more easily.
    let mut base_verdicts: Vec<(&str, Verdict)> = Vec::with_capacity(REFUSED_BASES.len());
    for base in REFUSED_BASES {
        let answer = verdict(root, base);
        assert_eq!(
            answer.code, 2,
            "the UNWRAPPED base `{base}` must itself be refused. If this row is red, every \
             invariance assertion below compares a permit against a permit and proves \
             nothing — the harness cannot observe a denial at all. Got reason id: {}",
            answer.reason_id
        );
        assert!(
            REASON_IDENTIFIERS.contains(&answer.reason_id.as_str()),
            "the unwrapped base `{base}` must be refused under a member of D-24's \
             taxonomy, or `reason_id` is not a verdict identity that invariance can be \
             compared on. Got: {}",
            answer.reason_id
        );
        base_verdicts.push((base, answer));
    }

    // --- floor 3: the corpus is not one shape wearing many hats -----------
    let distinct_reasons: BTreeSet<&str> = base_verdicts
        .iter()
        .map(|(_, answer)| answer.reason_id.as_str())
        .collect();
    assert!(
        distinct_reasons.len() >= MIN_DISTINCT_REASONS,
        "the refused corpus must span at least {MIN_DISTINCT_REASONS} DISTINCT D-24 \
         reasons, or it is one command shape wearing many spellings and the invariance \
         it proves is narrower than it looks. Got: {distinct_reasons:?}"
    );

    // --- the property -----------------------------------------------------
    let mut rng = Lcg::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut cases = 0usize;

    for (base, expected) in &base_verdicts {
        for _ in 0..VARIANTS_PER_REFUSED_BASE {
            let wrapped = wrap(&mut rng, base);
            chains.insert(wrapped.chain.clone());
            cases += 1;

            let got = verdict(root, &wrapped.command);
            assert_eq!(
                &got,
                expected,
                "\n\nINVARIANCE VIOLATED.\n\
                 \n  wrapped : {}\
                 \n  base    : {base}\
                 \n  recipe  : {}\
                 \n  got     : exit {} reason {}\
                 \n  expected: exit {} reason {}\
                 \n  seed    : {SEED:#x}\n\
                 \nWrapping a command changed what the envelope decided about it. That is \
                 `T-19-60`: something that changes which token is the effective program \
                 without appearing first.\n\
                 \n**The correct response is a change to the STRUCTURAL resolution in \
                 `src/envelope/policy.rs`, reported as a finding.** It is NOT adding this \
                 wrapper's name to a list anywhere, and it is NOT narrowing this \
                 generator's alphabet so the case stops being drawn. Either of those makes \
                 the corpus green while leaving the class exactly as open as it was — and \
                 a corpus that was narrowed to agree with the code is the one thing this \
                 file exists to make impossible.\n",
                wrapped.command,
                wrapped.recipe,
                got.code,
                got.reason_id,
                expected.code,
                expected.reason_id,
            );
        }
    }

    // --- floor 4: the loop above actually ran on something ----------------
    assert!(
        cases >= MIN_REFUSED_CASES,
        "the property must run over at least {MIN_REFUSED_CASES} generated cases; a \
         property that passed by generating nothing is the exact shape of certification \
         this phase exists to argue against (19-08, `T-19-51`). Got {cases}"
    );
    assert!(
        chains.len() >= MIN_DISTINCT_CHAINS,
        "the property must use at least {MIN_DISTINCT_CHAINS} DISTINCT wrapper chains; a \
         thousand cases that all wrapped in `env` would be one row counted a thousand \
         times. Got {}",
        chains.len()
    );

    // Printed rather than only asserted, so the SUMMARY records numbers that
    // were MEASURED. `cargo test -- --nocapture` shows them.
    println!(
        "refused corpus: {cases} generated cases, {} distinct wrapper chains, \
         {} distinct D-24 reasons, seed {SEED:#x}",
        chains.len(),
        distinct_reasons.len()
    );
}

// ---------------------------------------------------------------------------
// 2. The control that would have caught the failure mode shipped three times
// ---------------------------------------------------------------------------

/// The production logic of `src/envelope/policy.rs`, resolved at compile time.
const POLICY_SOURCE: &str = include_str!("../src/envelope/policy.rs");

/// The production logic of `src/envelope/hooks.rs`, resolved at compile time.
const HOOKS_SOURCE: &str = include_str!("../src/envelope/hooks.rs");

/// A string that is present in `policy.rs` and in NO other included file.
///
/// See `each_included_file_is_proved_to_be_the_file_this_control_thinks_it_is`
/// for why an absence control needs one.
const POLICY_ANCHOR: &str = "fn resolve_program";

/// A string that is present in `hooks.rs` and in NO other included file. The
/// function `19-11` rewired onto the resolver, and the most load-bearing thing
/// in that file for this control's purpose.
const HOOKS_ANCHOR: &str = "fn classify_segments(";

/// The wrapper names this control proves the production logic does not know.
///
/// **Twelve, not thirty, and the omissions are stated rather than silent.** The
/// short names in the wider alphabet — `env`, `command`, `time`, `nice`, `sudo`,
/// `proot`, `flock` — are substrings of ordinary English and of ordinary Rust
/// (`environment`, `command line`, `timestamp`, `is_nice`), so asserting their
/// absence from a source file would be asserting something about prose. The
/// twelve below are not substrings of any word this codebase uses, so their
/// absence is a fact about the LOGIC rather than about the vocabulary.
const NAMES_THE_FIX_MUST_NOT_KNOW: &[&str] = &[
    "stdbuf",
    "setsid",
    "ionice",
    "chrt",
    "taskset",
    "doas",
    "runuser",
    "unshare",
    "firejail",
    "torsocks",
    "catchsegv",
    "made-up-wrapper-9000",
];

/// The four of those names that appear nowhere in the RAW bytes of either file —
/// not in production code, not in a doc comment, not in a test fixture.
///
/// The stricter half of the control, asserted against the unprocessed source so
/// it cannot be weakened by a bug in [`production_code`].
const NAMES_ABSENT_FROM_THE_RAW_SOURCE: &[&str] =
    &["unshare", "firejail", "torsocks", "catchsegv"];

/// Everything in `source` that is neither a comment nor part of the trailing
/// `#[cfg(test)]` module.
///
/// **Why the stripping, stated rather than assumed.** `policy.rs` NAMES seven of
/// the wrappers below — in `GOVERNED_PROGRAMS`' doc comment and in
/// `resolve_program`'s, where they are listed precisely to record that the set of
/// things which can precede a program is open and therefore MUST NOT be
/// enumerated — and it uses `made-up-wrapper-9000` as a unit-test fixture. Those
/// are the disclosure and the control, which is the opposite of the failure mode
/// this test defends against. What must be true is that no wrapper name is
/// LOAD-BEARING: that none appears in code the guard actually executes. That is
/// the same fact `19-11`'s audit measured with `grep -v '^\s*//'`, made
/// mechanical here.
fn production_code(source: &str) -> String {
    let mut kept = String::new();
    for line in source.lines() {
        let trimmed = line.trim_start();
        // The unit-test module is the last item in both of these files, so the
        // attribute is the end of the production logic.
        if trimmed == "#[cfg(test)]" {
            break;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    kept
}

/// The absolute floor on `src/envelope/policy.rs`'s stripped production BYTES.
///
/// **This constant and its sibling REPLACE a ratio assertion — a 25% floor,
/// asserting that four times the stripped length exceeded the raw length — and
/// the replacement is a seam requirement rather than a tidy-up.** It is the one
/// existing assertion plan `19-20` is permitted to delete, and it is deleted
/// rather than supplemented: keeping both would reproduce the collision the
/// change exists to remove. The expression itself is deliberately not reproduced
/// here, so that a grep for it over this file answers zero.
///
/// **The measurement, re-derived with this file's own `production_code` at
/// `ec4c700` and stated in BYTES.** Rust's `str::len()` is a BYTE length, and
/// `policy.rs` carries multi-byte characters in its prose, so a `str`-CHARACTER
/// reading of the same file reports 262,748 and overstates the headroom by more
/// than a factor of two:
///
/// ```text
/// src/envelope/policy.rs   raw 263,360   stripped 65,947   ratio 25.0406%
///                          headroom: 107 stripped bytes / ~428 comment bytes
/// src/envelope/hooks.rs    raw  99,909   stripped 33,460   ratio 33.4905%
/// ```
///
/// **428 bytes of comment is less than plan `19-21`'s own doc additions**, which
/// is the sharpest possible statement of why the ratio has stopped measuring what
/// it names. Audit 6's judgement, quoted rather than paraphrased:
///
/// > the control has stopped measuring what it was written to measure … the floor
/// > now functions as a documentation budget whose red says "the stripper broke"
/// > while meaning "someone wrote comments" … keep the anti-vacuity property,
/// > re-express it as an absolute floor on stripped production bytes rather than a
/// > ratio against a file whose comment volume is itself a deliberate security
/// > artifact.
///
/// **The direction of the margin, stated.** 40,000 is 61% of the 65,947 bytes
/// measured at `ec4c700`. Ordinary refactoring that deletes a THIRD of this
/// file's production logic still passes; a stripper that ATE the logic — kept
/// only comments, broke on the `#[cfg(test)]` sentinel, or truncated the
/// `include_str!` — drops to near zero and turns this red. That is precisely the
/// failure the original ratio named and the only one it should have been able to
/// signal.
///
/// **It is deliberately INDEPENDENT of comment volume**, because this file also
/// carries `resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`,
/// which REQUIRES specific paragraphs of `policy.rs` to EXIST. A control that
/// demands documentation cannot coexist with one that penalises writing it, and
/// with 428 bytes of headroom the two were already in direct conflict.
///
/// A lower PERCENTAGE would not have fixed this: it is the same control with a
/// bigger budget — still red for comment growth, and still green under a
/// proportional truncation that removed most of the logic.
const POLICY_MIN_PRODUCTION_BYTES: usize = 40_000;

/// The absolute floor on `src/envelope/hooks.rs`'s stripped production BYTES.
///
/// 20,000 is 60% of the 33,460 bytes measured at `ec4c700`. See
/// [`POLICY_MIN_PRODUCTION_BYTES`] for the derivation, the direction of the
/// margin, and why this is an absolute floor rather than a ratio.
const HOOKS_MIN_PRODUCTION_BYTES: usize = 20_000;

#[test]
fn wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic() {
    // -----------------------------------------------------------------------
    // FIRST, the positive control: prove each included file is the file this
    // scanner believes it is reading.
    //
    // **An absence assertion cannot tell "the name is not in this file" from
    // "this is not the file I think it is".** `include_str!` fails the build on
    // a MISSING path but not on a WRONG-BUT-EXISTING one: point it at a
    // different source file, or at the same file twice, and every absence
    // assertion below passes having certified nothing. So before anything is
    // concluded from what is missing, each file is proved to contain a string
    // known to be in THAT file and in no other included file.
    // -----------------------------------------------------------------------
    let policy = production_code(POLICY_SOURCE);
    let hooks = production_code(HOOKS_SOURCE);

    for (label, raw, code, anchor, other_anchor, floor) in [
        (
            "src/envelope/policy.rs",
            POLICY_SOURCE,
            policy.as_str(),
            POLICY_ANCHOR,
            HOOKS_ANCHOR,
            POLICY_MIN_PRODUCTION_BYTES,
        ),
        (
            "src/envelope/hooks.rs",
            HOOKS_SOURCE,
            hooks.as_str(),
            HOOKS_ANCHOR,
            POLICY_ANCHOR,
            HOOKS_MIN_PRODUCTION_BYTES,
        ),
    ] {
        assert!(
            raw.contains(anchor),
            "POSITIVE CONTROL FAILED for `{label}`: its `include_str!` content does not \
             contain `{anchor}`, which that file is known to define. A red here means the \
             `include_str!` path is wrong or the file was renamed or the anchor moved — \
             and therefore that EVERY absence assertion in this test is vacuous, because \
             a name is trivially absent from a file that is not the one being examined. \
             Fix the path or the anchor; do not delete this assertion."
        );
        assert!(
            !raw.contains(other_anchor),
            "POSITIVE CONTROL FAILED for `{label}`: it contains `{other_anchor}`, which \
             belongs to the OTHER included file. The two `include_str!` calls are \
             therefore not distinguishable, which is exactly the wrong-but-existing-path \
             mistake this control exists to catch."
        );
        assert!(
            code.contains(anchor),
            "POSITIVE CONTROL FAILED for `{label}`: `{anchor}` survives in the raw source \
             but not in the production-stripped text, so `production_code` has eaten the \
             logic it was supposed to keep and the absence assertions below would pass \
             over an empty or truncated string."
        );
        assert!(
            code.len() >= floor,
            "POSITIVE CONTROL FAILED for `{label}`: stripping comments and the \
             `#[cfg(test)]` module left {} production BYTES, and the floor for this file is \
             {floor}.\n\n\
             This is an ABSOLUTE floor and not a ratio, for the reason \
             `POLICY_MIN_PRODUCTION_BYTES` records. A red here means `production_code` has \
             eaten the logic it was supposed to keep — it broke on the `#[cfg(test)]` \
             sentinel, or the `include_str!` truncated — and the absence assertions below \
             would pass over an empty or near-empty string.\n\n\
             **The correct response is to fix `production_code` or to restore the deleted \
             logic. NEVER to lower the floor.** If a deliberate refactor really did remove \
             more than a third of this file's production bytes, that is a fact worth \
             stating in a commit message before this number moves.",
            code.len()
        );
    }

    // -----------------------------------------------------------------------
    // Only now, the absence floor.
    // -----------------------------------------------------------------------
    assert!(
        NAMES_THE_FIX_MUST_NOT_KNOW.len() >= 10,
        "this control must designate at least ten names, or it can pass over an empty \
         list — an absence assertion made about nothing. Got {}",
        NAMES_THE_FIX_MUST_NOT_KNOW.len()
    );

    for name in NAMES_THE_FIX_MUST_NOT_KNOW {
        for (label, code) in [
            ("src/envelope/policy.rs", policy.as_str()),
            ("src/envelope/hooks.rs", hooks.as_str()),
        ] {
            assert!(
                !code.contains(name),
                "`{name}` appears in the PRODUCTION LOGIC of `{label}`.\n\n\
                 This is the failure mode this project has shipped three times: a new \
                 bypass shape gets closed by adding a name to a list. The moment a wrapper \
                 name is load-bearing, the generative corpus in this file certifies a \
                 claim about the CLASS while only being able to fail on names the fix \
                 already knows — which is to say it can no longer fail at all.\n\n\
                 The correct response is to resolve the effective program STRUCTURALLY, as \
                 `policy::resolve_program` does: consume leading assignment words by the \
                 shell's own grammar, then find the first token whose BASENAME is in the \
                 CLOSED set `policy::GOVERNED_PROGRAMS`. Never enumerate the OPEN set of \
                 things that can precede a program (D-08).\n\n\
                 Deleting this assertion, or dropping `{name}` from the alphabet, is not a \
                 fix."
            );
        }
    }

    // The stricter half: four names that are absent even from the raw bytes,
    // asserted against the unprocessed source so this control does not rest
    // entirely on `production_code` being correct.
    for name in NAMES_ABSENT_FROM_THE_RAW_SOURCE {
        for (label, raw) in [
            ("src/envelope/policy.rs", POLICY_SOURCE),
            ("src/envelope/hooks.rs", HOOKS_SOURCE),
        ] {
            assert!(
                !raw.contains(name),
                "`{name}` appears somewhere in `{label}` — including its comments and its \
                 test module. These four names are asserted against the RAW bytes so the \
                 control does not rest entirely on the comment stripper being right."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3. The paired allow corpus (D-32, `T-19-78`) — what stops the fix from being
//    "deny everything"
// ---------------------------------------------------------------------------

/// Commands that must be PERMITTED, run through the SAME generator and the same
/// wrapper chains as the refusal corpus.
///
/// **This half is what makes the other half mean anything.** Every assertion in
/// `the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command`
/// would also be satisfied by a guard that refused every Bash tool call, and the
/// guard is registered against every Bash tool call — so a resolution that denied
/// what it did not recognise would deny `ls`, `cargo` and `rg`, and a control
/// that fails into unusability is a control that gets switched off.
/// `ProgramResolution::Ungoverned` is a PERMIT and it is the ANSWER, not a
/// fall-through.
///
/// **No PR-CREATING forge command appears here, and that is a correctness
/// requirement rather than a preference.** The ledger records before it permits
/// (D-20), so the second `gh pr create` in a run is parked for
/// `pr_cap_exceeded` REGARDLESS of wrapping — which would make an invariance
/// assertion over it pass for entirely the wrong reason. `gh pr list` is the read
/// that belongs here; `env gh pr create` is an enumerated row in
/// `tests/envelope_wrapper_bypass.rs`, where it is asserted against the ledger
/// rather than against invariance.
const PERMITTED_BASES: &[&str] = &[
    // Ordinary commands the guard has no governance claim over at all.
    "ls -la",
    "echo hi",
    "cargo build --offline",
    "rg -n TODO src/",
    // A `-c` that is not a shell's `-c`: its following word is a search pattern,
    // which names no governed program, so the payload resolves to nothing rather
    // than to a refusal.
    "grep -c fn src/main.rs",
    // Legitimate GOVERNED commands. The wrapper must be transparent in BOTH
    // directions: if `env git status` were refused, transparency would be a
    // denial rather than a classification, and reading git state is the first
    // thing a driven run does.
    "git status",
    "git log --oneline -n 5",
    "git push origin refs/heads/gsd-auto/alpha/w:refs/heads/gsd-auto/alpha/w",
    "gh pr list --limit 5",
    // -----------------------------------------------------------------------
    // `T-19-95`, the round-5 half — the PERMITTED TWINS of the refused brace and
    // glob bases, and the only way this corpus can tell a brace EXPANSION from a
    // brace PAIR, or a decision-slot glob from an operand glob.
    //
    // **`git add src/*.rs` is the operand glob.** `19-17` refuses a glob in a
    // DECISION word; only the decision region is in scope, and this row is what
    // makes that boundary a behaviour rather than an intention.
    //
    // **`git log -1 HEAD@{0}` is the LITERAL brace pair** — braces with no comma
    // and no range, which bash passes through byte-identically. It is the
    // permitted twin of `git reflog delete HEAD@{0}` below, and the pair is what
    // `T-19-93` requires: a rule that absorbed literal braces while losing a
    // refusal, or one that refused every brace it saw, turns exactly one of the
    // two red.
    //
    // It sits in the base rather than in a wrapper because a base is the END of
    // the generated line: its `}` closer has no following word to sever, so its
    // verdict really is invariant. Measured exit 0 unwrapped, under `env`, under
    // `sh -c '…'`, under `{ …; }`, under `( … )`, and with the glob assignment
    // prefix and the glob wrapper operand in front of it.
    "git add src/*.rs",
    "git log -1 HEAD@{0}",
];

/// 11 x 120 = 1320, over the 1000 floor the plan sets.
const VARIANTS_PER_PERMITTED_BASE: usize = 120;

const MIN_PERMITTED_CASES: usize = 1000;

#[test]
fn the_permitted_corpus_survives_every_generated_wrapping_and_still_answers_nothing() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    assert!(
        !PERMITTED_BASES.is_empty(),
        "an empty allow corpus is the same thing as not having one: without it, a guard \
         that refused every tool call would satisfy every assertion in the refusal half of \
         this file (D-32)"
    );

    // Every base permits UNWRAPPED first, for the same reason the refusal half
    // measures its bases first: the right-hand side of an invariance claim has
    // to be observed, not assumed.
    for base in PERMITTED_BASES {
        let (answer, stdout) = ask(root, base);
        assert_eq!(
            answer.code, 0,
            "the UNWRAPPED base `{base}` must be PERMITTED. Reason id: {}",
            answer.reason_id
        );
        assert!(
            stdout.is_empty(),
            "a permit answers nothing at all: emitting `allow` would turn a deny-only \
             control into an approval authority. `{base}` wrote: {stdout}"
        );
    }

    let mut rng = Lcg::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut cases = 0usize;

    for base in PERMITTED_BASES {
        for _ in 0..VARIANTS_PER_PERMITTED_BASE {
            let wrapped = wrap(&mut rng, base);
            chains.insert(wrapped.chain.clone());
            cases += 1;

            let (got, stdout) = ask(root, &wrapped.command);
            assert_eq!(
                got.code,
                0,
                "\n\nA PERMITTED COMMAND WAS REFUSED AFTER WRAPPING.\n\
                 \n  wrapped : {}\
                 \n  base    : {base}\
                 \n  recipe  : {}\
                 \n  reason  : {}\
                 \n  seed    : {SEED:#x}\n\
                 \nThis corpus exists so that the refusal half of this file cannot be \
                 satisfied by a guard that denies everything. A red here is the fix having \
                 become a blanket denial in some wrapping — which would make a driven run \
                 unusable, and an unusable control is a control that gets switched off.\n",
                wrapped.command,
                wrapped.recipe,
                got.reason_id,
            );
            assert!(
                stdout.is_empty(),
                "a permit answers nothing at all, wrapped or not. `{}` wrote: {stdout}",
                wrapped.command
            );
        }
    }

    assert!(
        cases >= MIN_PERMITTED_CASES,
        "the allow corpus must run over at least {MIN_PERMITTED_CASES} wrapped cases, or \
         the pairing is decorative. Got {cases}"
    );

    println!(
        "permitted corpus: {cases} generated cases, {} distinct wrapper chains, \
         seed {SEED:#x}",
        chains.len()
    );
}

// ---------------------------------------------------------------------------
// 4. The discrimination pair (`T-19-75`)
// ---------------------------------------------------------------------------

#[test]
fn a_search_for_an_allowed_git_command_runs_and_a_search_for_a_refused_one_does_not() {
    // **Asserted as a PAIR in one test so the two cannot drift apart.** The
    // quoted-payload rule CLASSIFIES the payload rather than blanket-denying it,
    // and the only way to show that is to exhibit one of each. Split across two
    // tests, someone deleting the permitted half would leave a file that still
    // looked like it proved discrimination.
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    let (allowed, stdout) = ask(root, "rg \"git status\" src/");
    assert_eq!(
        allowed.code, 0,
        "`rg \"git status\" src/` must RUN. The payload is classified, and `git status` \
         classifies as allowed — a rule that refused every quoted payload naming a \
         governed program would stop a driven run from grepping its own source. Reason: {}",
        allowed.reason_id
    );
    assert!(stdout.is_empty(), "a permit answers nothing: {stdout}");

    let refused = verdict(root, "rg \"git push --force\" src/");
    assert_eq!(
        refused.code, 2,
        "`rg \"git push --force\" src/` must be REFUSED. This is `T-19-75`, the ACCEPTED \
         over-refusal disclosed in `resolve_program`'s doc: an argument that literally \
         spells a refused git command is refused. It is pinned rather than merely \
         disclosed so that closing it later is a deliberate edit."
    );
    assert_eq!(
        refused.reason_id, policy::REASON_FORCE_PUSH_BLOCKED,
        "and it is refused for the reason the payload itself carries, not for an unrelated \
         cause such as words that could not be recovered"
    );
}

// ---------------------------------------------------------------------------
// 5. The behaviours `19-11` GENERALISED rather than replaced
// ---------------------------------------------------------------------------
//
// Asserted through `hooks::guard_in` against command STRINGS rather than against
// `resolve_program`'s enum, so they survive any future refactor of how
// resolution is spelled. `19-11` deleted `NESTED_SHELLS` — a five-name list of
// shells — in favour of a structural `-c` rule; these rows are what prove the
// deletion widened coverage rather than merely moving it.

/// Assert one command is refused, and that its refusal carries one specific
/// member of D-24's taxonomy.
fn refuses_under(root: &Path, command: &str, reason_id: &str) {
    let answer = verdict(root, command);
    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. Got reason id: {}",
        answer.reason_id
    );
    assert_eq!(
        answer.reason_id, reason_id,
        "`{command}` must be refused UNDER `{reason_id}`. Asserting the identifier and not \
         merely the exit code is what stops a row from passing because it was refused for \
         an unrelated cause."
    );
}

/// Assert one command is refused and that its message NAMES `needle`.
///
/// Used where the refusal's value is in what it tells the reader — that it was
/// `eval`, or an expansion, or words that could not be recovered — rather than
/// only in which bucket it parked under.
fn refuses_naming(root: &Path, command: &str, needle: &str) {
    let (answer, stdout) = ask(root, command);
    assert_eq!(
        answer.code, 2,
        "`{command}` must be REFUSED. Got reason id: {}",
        answer.reason_id
    );
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or(serde_json::Value::Null);
    let reason = value["hookSpecificOutput"]["permissionDecisionReason"]
        .as_str()
        .unwrap_or_default();
    assert!(
        reason.contains(needle),
        "`{command}`'s refusal must NAME `{needle}`, or a reader meeting it cannot tell \
         which property of the command made it unjudgeable. Got: {reason}"
    );
}

/// Assert one command is permitted and answers nothing.
fn permits(root: &Path, command: &str) {
    let (answer, stdout) = ask(root, command);
    assert_eq!(
        answer.code, 0,
        "`{command}` must be PERMITTED. Got reason id: {}",
        answer.reason_id
    );
    assert!(stdout.is_empty(), "a permit answers nothing: {stdout}");
}

#[test]
fn a_force_push_inside_a_shell_payload_is_still_refused_in_every_spelling_of_the_flag() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // 19-05 deviation 2, preserved: the guard follows a `-c` payload.
    refuses_under(
        root,
        "sh -c \"git push --force origin main\"",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses_under(
        root,
        "bash -c \"echo hi && git push --force origin main\"",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );

    // **The row that proves the generalisation WIDENED coverage rather than
    // moving it.** `bash -lc "…"` bundles the login flag with `-c`; the deleted
    // `NESTED_SHELLS` list matched an exact `-c` and did not cover this. The
    // structural rule — a bundled short option containing `c` hands its
    // following word over as a nested command line — covers it without naming
    // `bash`.
    refuses_under(
        root,
        "bash -lc \"git push --force origin main\"",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
}

#[test]
fn a_command_the_guard_cannot_see_the_program_of_is_refused_naming_why() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // `eval` builds its command at run time, so no classifier can see it.
    refuses_naming(root, "eval \"git push --force\"", "eval");

    // A verb assembled by shell expansion is not knowable before it runs.
    refuses_naming(root, "$TOOL push --force", "expansion");

    // Words that cannot be recovered. Note this refusal carries no `(reason: …)`
    // marker — it is produced before classification, by the splitter — so it is
    // pinned on the sentence rather than on a taxonomy identifier.
    refuses_naming(
        root,
        "git commit -m 'unterminated",
        "words cannot be recovered",
    );

    // A nesting deeper than the guard follows is REFUSED rather than followed.
    // The bound exists so the recursion cannot be turned into a denial of
    // service; refusing at the bound is the direction that fails closed.
    refuses_under(
        root,
        "sh -c \"sh -c \\\"sh -c 'git status'\\\"\"",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn a_dash_c_payload_whose_quoting_cannot_be_recovered_is_permitted_when_it_governs_nothing() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // **The control for the payload rule's own edge.** An unbalanced quote
    // inside a `-c` payload is a payload the shell will not run either, so
    // refusing all of them would deny an ordinary command for nothing. The rule
    // refuses such a payload only when it actually mentions something this
    // envelope governs — which is why `refuses_naming` above catches
    // `git commit -m 'unterminated` while this row runs.
    permits(root, "grep -c \"don't\" src/main.rs");
}

// ===========================================================================
// 6. The residual `T-19-60`'s fix does NOT close, bounded on both sides
// ===========================================================================
//
// **Everything below asserts a DISCLOSED LIMITATION, not a desirable
// behaviour.** `19-11` accepted `T-19-74` and recorded it in
// `resolve_program`'s own doc comment; these rows exist so that the residual has
// measured edges rather than prose ones, and so that a future change which
// closes it does so by DELETING these rows deliberately rather than by
// discovering them failing.
//
// A residual whose edges are untested is a residual nobody can tell has grown.

#[test]
fn the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // **`T-19-74`.** The program is assembled by an expansion of a variable
    // bound OUTSIDE this command line, behind a wrapper. `resolve_program`
    // reaches `Ungoverned` and the guard permits it.
    permits(root, "env $X push --force origin main");

    // **The cost of the alternative, measured rather than asserted about.**
    // Closing the row above would require refusing every `$VAR` in an ungoverned
    // command. These two are what that would also refuse — an ordinary read of
    // git's own output, and `cd`. A control that fails into unusability is a
    // control that gets switched off, which is why `19-11` accepted the residual
    // instead. If a future change closes `T-19-74`, these two rows are the bill.
    permits(root, "echo $(git rev-parse HEAD)");
    permits(root, "cd \"$HOME\"");
}

#[test]
fn the_bounds_of_the_t_19_74_residual_are_refused_which_is_what_makes_it_narrow() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // **Bound 1 — the compound `19-SECURITY.md` calls the reason the threat is
    // high is closed even where the program stays unresolved.** One token that
    // both hides the command from layer 2 and disarms layer 3 is refused on its
    // own account, at step 1, before resolution reaches anything.
    refuses_under(
        root,
        "GIT_CONFIG_COUNT=0 env $X push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );

    // **Bound 2 — binding a governed program name to a variable IN THE SAME
    // COMMAND LINE is refused.** `resolve_program` step 7. This closes the
    // same-command-line route, so only a binding from outside remains.
    //
    // NOTE the spelling: no `;`. See
    // `the_residual_begins_exactly_at_the_command_line_boundary` below for why
    // the semicolon form is a different case and what it measures.
    refuses_under(
        root,
        "X=git env $X push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // **Bound 3 — the BARE-NAME form of the envelope-key rule is live, not only
    // the assignment form.** `unset GIT_CONFIG_COUNT` spells no `=`, and a rule
    // that only matched assignments would let the removal through while
    // refusing the overwrite.
    refuses_under(
        root,
        "unset GIT_CONFIG_COUNT && git push --force origin main",
        policy::REASON_HOOK_BYPASS_BLOCKED,
    );
}

#[test]
fn the_residual_begins_exactly_at_the_command_line_boundary() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // **The two sides of the boundary, in one test so they cannot drift apart.**
    //
    // `resolve_program` step 7 refuses a governed program name bound to a
    // variable in the SAME simple command. A `;` makes the binding a DIFFERENT
    // command, and a binding from a previous command line is precisely what
    // `T-19-74` discloses as out of reach: the guard is answering about one
    // command at a time and has no memory of the last one.
    //
    // Both rows are asserted because the interesting fact is WHERE the closed
    // route stops. Pinning only the refused side would let the boundary move
    // outward unnoticed; pinning only the permitted side would not show that
    // anything is closed at all.
    refuses_under(
        root,
        "X=git env $X push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    permits(root, "X=git; env $X push --force origin main");
}

// ===========================================================================
// 8. `T-19-83` — the shape this corpus was STRUCTURALLY INCAPABLE of generating
// ===========================================================================
//
// **The finding, in the auditor's own terms and not softened.** The `WRAPPERS`
// alphabet above contains wrappers WITH operands — `env -u SOME_VAR`,
// `runuser -u me --`, `nice -n 10`, `flock /tmp/lock` — but no operand whose
// BASENAME is a governed program. So 1680 generated cases certified a fix that
// the very next audit walked around with four characters: `env -u git`.
//
// That is `T-19-76`'s own failure mode one level up — the corpus certifying the
// fix could not fail on the class it certifies — and it is now the **SECOND
// consecutive round** in which that has happened. This section exists because of
// that sentence, which is also why it must never be narrowed: a case that stops
// being drawn is a case that stops being able to fail.
//
// **Why a separate property rather than more entries in `WRAPPERS`.** The
// invariance property above compares the wrapped verdict to the UNWRAPPED base's
// verdict on exit code AND D-24 identifier. A decoy wrapping is legitimately MORE
// strict — it is refused for an unresolvable command position rather than for the
// base's own reason — so folding these entries into `WRAPPERS` would break
// invariance by being STRICTER, exactly as this file already records for the
// `GIT_CONFIG_COUNT=0` assignment prefix. The property here is therefore refusal
// carrying a D-24 identifier, not invariance. `WRAPPERS` is left untouched.

/// Whole wrapper prefixes whose own option operand is spelled as a governed
/// program.
///
/// **Mixing plausible real-world spellings with structurally identical nonsense
/// is the point.** A UNIX account called `git` is an ordinary thing on a git
/// server, and `made-up-wrapper-9000 --flag gh` is not a program at all — and the
/// resolver must not be able to tell them apart, because the moment it can it is
/// asking what the wrapper is CALLED.
const DECOY_OPERANDS: &[&str] = &[
    // The measured line. `env`'s `-u` names a variable to REMOVE; spelled `git`,
    // it is a token in an operand slot that looks exactly like a program.
    "env -u git",
    // The forge name, which reaches `pr_command_label` and the ledger instead of
    // `classify_git` — an entirely different decision path.
    "env -u gh",
    // The third governed name, which has no classifier arm of its own.
    "env -u glab",
    // Basename normalisation on the DECOY rather than on the program: a rule
    // comparing raw token text would count one candidate here instead of two.
    "env -u /usr/bin/git",
    // The plausible real-world spelling, and the reason the disclosed cost of
    // this fix is a cost rather than a curiosity.
    "sudo -u git",
    // The same shape from a wrapper whose name the production logic is
    // mechanically asserted never to know.
    "runuser -u git",
    // The end-of-options form: `--` stands between the governed operand and the
    // real program, so the two candidates are not adjacent.
    "sudo -u gh --",
    // The long-option spelling of the user operand.
    "sudo --user git",
    // A wrapper that does not exist, carrying a governed operand. A fix built on
    // a list of wrapper names cannot possibly contain this one.
    "made-up-wrapper-9000 --flag git",
    // ...and its forge twin, so the made-up shape is exercised on both paths.
    "made-up-wrapper-9000 --flag gh",
    // -----------------------------------------------------------------------
    // `T-19-89`, the Rule A half — a decoy operand in a prefix that ALSO
    // carries an expansion.
    //
    // Audit 3's finding for this alphabet was that every entry is a literal
    // program word, so the corpus could not draw the cell where a decoy and an
    // expansion appear in the same prefix. This property asserts REFUSAL
    // carrying a D-24 identifier rather than invariance — which is why these
    // belong here and not in `WRAPPERS`: they are legitimately refused for a
    // reason of their own (two candidates, or an unreadable prefix word) rather
    // than for the base's reason.
    "env -u $V git",
    "sudo -u $U gh",
    "made-up-wrapper-9000 --flag $F git",
    // -----------------------------------------------------------------------
    // `T-19-95`, the round-5 half — a decoy operand carrying a governed name
    // through a class that sets NO expansion bit.
    //
    // `env -u gi?` is a glob decoy: measured exit 0 unwrapped as a prefix in
    // front of a permitted base, so it is not refused on its own account today,
    // and the case is refused only for the base's own reason.
    //
    // `env -u {git,x}` is a splice whose PRODUCTS include a governed name.
    // **Its verdict arrives by a different route today, and that is disclosed
    // rather than glossed** — exactly as this alphabet's `${K}` sibling already
    // is. `}` is a word-splitting closer, so Rule B refuses the segment behind
    // it and the case is refused at `envelope_assertion_failed` before anything
    // this round adds is reached. It is drawn so the decoy axis can DRAW the
    // class, not because it is a live bypass.
    "env -u gi?",
    "env -u {git,x}",
];

/// Whole wrapper prefixes that carry an expansion and NO decoy operand.
///
/// **A separate alphabet with a refusal property of its own, for the reason
/// `DECOY_OPERANDS` already records.** An expansion anywhere in the wrapper
/// prefix region is refused by `resolve_program` step 5 on its own account —
/// what that prefix does to the environment the program runs in is decided
/// after the guard has answered — so folding these into `WRAPPERS` would break
/// the invariance property by being STRICTER, not laxer. The property below
/// therefore asserts refusal carrying a D-24 identifier.
///
/// The last entry is deliberately a BRACE spelling, and its verdict arrives by
/// a different route: `{` and `}` are `SEPARATORS`, so the flush severs the
/// prefix from the governed program and the refusal comes from the base's own
/// segment instead of from step 5. That is the `19-15` shape drawn here rather
/// than typed once, and it is why this property asserts membership in D-24's
/// taxonomy rather than one specific identifier.
const EXPANSION_WRAPPERS: &[&str] = &[
    "env $E",
    "env -u $K",
    "timeout $T",
    "nice -n $N",
    "sudo -u $U",
    "made-up-wrapper-9000 --flag $F",
    "env -u `true`SOME_VAR",
    "env -u ${K}",
    // -----------------------------------------------------------------------
    // `T-19-95`, the round-5 half — brace-expansion wrapper prefixes, one of
    // them CONCATENATED so the wrapper axis can draw that class too.
    //
    // Both reach their verdict by the `19-15` route the `${K}` entry above
    // already documents — `}` severs the prefix from the governed program and
    // the refusal comes from the base's own segment rather than from
    // `resolve_program` step 5 — so both are refused TODAY, measured at exit 2
    // under `envelope_assertion_failed`. **They are class coverage for this
    // axis, not a red row**, and saying so here is what stops a later reader
    // counting them as evidence that `19-17` closed something.
    //
    // **A GLOB wrapper prefix is deliberately NOT here.** Step 5's prefix rule
    // decides on `Token.expansion`, and `19-17` explicitly does not move steps 3
    // and 5: its new literalness bit is consumed by the DECISION-WORD rule, not
    // by the prefix rule. An entry like `env -u GIT_CONFIG_COUN?` would
    // therefore be pinned at a refusal the rule cannot produce — the one thing a
    // plan that measures pre-fix has no method to catch.
    "env -u GIT_CONFIG{_COUNT,_COUNT}",
    "env -u {GIT_CONFIG_COUNT,GIT_SSH_COMMAND}",
];

/// How many wrappings each (base, decoy) pair gets. 12 x 10 x 6 = 720.
const VARIANTS_PER_DECOY_PAIR: usize = 6;

/// The floors, asserted BEFORE the loop.
const MIN_DECOY_OPERANDS: usize = 8;
const MIN_DECOY_CASES: usize = 600;
const MIN_DISTINCT_DECOY_CHAINS: usize = 100;

/// One generated decoy wrapping.
struct Decoyed {
    command: String,
    /// The whole chain including the decoy and its position — what "a distinct
    /// decoy chain" counts.
    chain: String,
    recipe: String,
    /// Whether an ordinary wrapper stands on BOTH sides of the decoy.
    wrapped_on_both_sides: bool,
}

/// Wrap `base` exactly as [`wrap`] does — one drawn assignment prefix, a drawn
/// depth of 0 to 3 ordinary wrappers, per-wrapper quoting, an optional outer
/// shell layer — and then SPLICE `decoy` into that chain at a drawn index.
///
/// **The splice is specified rather than left to the implementer, and the reason
/// is the same partial coverage this whole section exists to close.** Prepending
/// the decoy to the base and wrapping the result would put it innermost every
/// time and never exercise it *under* an ordinary wrapper. Drawing the index in
/// `0..=depth` puts it outermost, innermost, and sandwiched between ordinary
/// wrappers, and the last of those is the case the `wrapped_on_both_sides` floor
/// proves actually occurs.
fn wrap_with_decoy(rng: &mut Lcg, base: &str, decoy: &str) -> Decoyed {
    let prefix = ASSIGNMENT_PREFIXES[rng.pick(ASSIGNMENT_PREFIXES.len())];
    let layer = SHELL_LAYERS[rng.pick(SHELL_LAYERS.len())];
    let quotes = layer.inner_quotes();
    let depth = rng.pick(4);

    let mut chain: Vec<&str> = Vec::with_capacity(depth + 1);
    for _ in 0..depth {
        chain.push(WRAPPERS[rng.pick(WRAPPERS.len())]);
    }
    let at = rng.pick(depth + 1);
    chain.insert(at, decoy);

    let mut spelled = String::new();
    for entry in &chain {
        // Quote the PROGRAM WORD, as `wrap` does: `"env" -u git` and
        // `env -u git` are the same command, and a resolution that compared raw
        // token text rather than the recovered word would tell them apart.
        let quote = quotes[rng.pick(quotes.len())];
        match entry.split_once(' ') {
            Some((program, rest)) => spelled.push_str(&format!("{quote}{program}{quote} {rest} ")),
            None => spelled.push_str(&format!("{quote}{entry}{quote} ")),
        }
    }

    let inner = format!("{prefix}{spelled}{base}");
    Decoyed {
        command: layer.apply(&inner),
        chain: format!("[{}] decoy@{at}", chain.join(", ")),
        recipe: format!(
            "prefix={prefix:?} layer={layer:?} depth={depth} decoy={decoy:?} at={at}"
        ),
        wrapped_on_both_sides: at > 0 && at < depth,
    }
}

#[test]
fn a_governed_program_in_a_wrappers_operand_slot_is_refused_in_every_generated_position() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // --- floor 1: the alphabet exists and is wide enough ------------------
    assert!(
        !DECOY_OPERANDS.is_empty(),
        "an empty decoy alphabet makes every assertion below vacuous, which is the exact \
         defect (`T-19-83`) this section was added to repair"
    );
    assert!(
        DECOY_OPERANDS.len() >= MIN_DECOY_OPERANDS,
        "the decoy alphabet must carry at least {MIN_DECOY_OPERANDS} entries spanning \
         both plausible and nonsense spellings; a narrower one is a row per named shape \
         wearing a property's clothes. Got {}",
        DECOY_OPERANDS.len()
    );

    // --- floor 2: the bases are still refused unwrapped --------------------
    //
    // Measured rather than written down, for the same reason the invariance
    // property measures its right-hand side: a corpus whose bases had quietly
    // become permits would fail HERE rather than pass more easily.
    for base in REFUSED_BASES {
        let answer = verdict(root, base);
        assert_eq!(
            answer.code, 2,
            "the UNWRAPPED base `{base}` must itself be refused, or every decoy assertion \
             below is comparing a refusal against nothing. Got reason id: {}",
            answer.reason_id
        );
    }

    // --- the property ------------------------------------------------------
    let mut rng = Lcg::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut used: BTreeSet<&str> = BTreeSet::new();
    let mut both_sides = 0usize;
    let mut cases = 0usize;

    for base in REFUSED_BASES {
        for decoy in DECOY_OPERANDS {
            for _ in 0..VARIANTS_PER_DECOY_PAIR {
                let case = wrap_with_decoy(&mut rng, base, decoy);
                chains.insert(case.chain.clone());
                used.insert(decoy);
                if case.wrapped_on_both_sides {
                    both_sides += 1;
                }
                cases += 1;

                let got = verdict(root, &case.command);
                assert_eq!(
                    got.code,
                    2,
                    "\n\nA GOVERNED PROGRAM IN A WRAPPER'S OPERAND SLOT WAS PERMITTED.\n\
                     \n  command : {}\
                     \n  base    : {base}\
                     \n  recipe  : {}\
                     \n  got     : exit {} reason {}\
                     \n  seed    : {SEED:#x}\n\
                     \nA token that is not the effective program captured the resolver's \
                     index because it is spelled as a governed program in an operand slot, \
                     so the real command became `argv[0]` of the classified argv and its \
                     verb was read as `git`/`gh`/`glab` — which the denylist's default arm \
                     allows.\n\
                     \n**The correct response is a change to the structural COMMAND \
                     POSITION rule in `src/envelope/policy.rs`.** It is NOT a wrapper name \
                     added to a list, it is NOT a wrapper FLAG added to a list — a rule \
                     keyed on what `-u` means is a wrapper-name list wearing a flag's \
                     clothes — and it is NOT a narrowing of this alphabet. This corpus \
                     exists because the last one could not fail on its own class; making \
                     it green by shrinking it would be the third time.\n",
                    case.command,
                    case.recipe,
                    got.code,
                    got.reason_id,
                );
                assert!(
                    REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
                    "\n\nREFUSED WITHOUT A D-24 IDENTIFIER.\n\
                     \n  command : {}\
                     \n  recipe  : {}\
                     \n  reason  : {}\n\
                     \nA refusal that parks under no member of D-24's taxonomy cannot be \
                     found by a later reader grepping for a disarmed layer, and a row that \
                     asserted only the exit code would pass on a refusal for an unrelated \
                     cause.\n",
                    case.command,
                    case.recipe,
                    got.reason_id,
                );
            }
        }
    }

    // --- floors 3, 4, 5: the loop ran on something that matters ------------
    assert!(
        cases >= MIN_DECOY_CASES,
        "the decoy property must run over at least {MIN_DECOY_CASES} generated cases. Got \
         {cases}"
    );
    assert!(
        chains.len() >= MIN_DISTINCT_DECOY_CHAINS,
        "the decoy property must use at least {MIN_DISTINCT_DECOY_CHAINS} DISTINCT chains; \
         hundreds of cases that all spelled `env -u git` at the innermost position would be \
         one row counted hundreds of times. Got {}",
        chains.len()
    );
    assert_eq!(
        used.len(),
        DECOY_OPERANDS.len(),
        "every entry of `DECOY_OPERANDS` must appear in at least one generated case, or \
         the alphabet is wider than the corpus and an entry nobody drew is an entry that \
         cannot fail. Unused: {:?}",
        DECOY_OPERANDS
            .iter()
            .filter(|entry| !used.contains(*entry))
            .collect::<Vec<_>>()
    );
    assert!(
        both_sides > 0,
        "at least one generated case must have an ordinary wrapper on BOTH sides of the \
         decoy. This is the floor that proves the SPLICE reaches the middle of a chain \
         rather than only its ends — a decoy that is always innermost is never exercised \
         under a wrapper, which is the same partial coverage this section exists to close."
    );

    println!(
        "decoy corpus: {cases} generated cases, {} distinct decoy chains, \
         {} decoy operands, {both_sides} cases wrapped on both sides, seed {SEED:#x}",
        chains.len(),
        DECOY_OPERANDS.len()
    );
}

#[test]
fn the_decoy_rule_discriminates_rather_than_blanket_denying_the_shape() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // **The row that tells "establishing command position" apart from "denying
    // anything that looks like a decoy".** `env -u git ls` has exactly ONE
    // governed candidate — the operand — and the command that actually runs is
    // `ls`. Refusing it would be blanket-denying the SHAPE.
    permits(root, "env -u git ls");

    // **`T-19-74` / AR-19-10 in its DECOY form — CONVERTED from a permit to a
    // refusal by plan 19-14's Rule A, and disclosed rather than absorbed.**
    //
    // Old verdict: PERMITTED (exit 0). New verdict: REFUSED under
    // `envelope_assertion_failed`.
    //
    // The mechanism, and why the flip is a SIDE EFFECT of closing `T-19-88`
    // rather than a decision to narrow AR-19-10: the segment resolves
    // `Governed { index: 2 }` on the decoy — ONE candidate, so the ambiguity
    // rule does not fire — and the verb `classify_git` would then judge is
    // `$X`, which is the decision word Rule A refuses. Every argv whose verb
    // slot carries an expansion is refused now, and this one is inside that
    // class however it got there.
    //
    // **The CORE of the residual is unmoved and is pinned elsewhere in this
    // file:** `env $X push --force origin main` reaches ZERO candidates and
    // stays permitted, `X=git; env $X push --force origin main` returns
    // `NoProgram` then `Ungoverned` and stays permitted, and
    // `the_residual_begins_exactly_at_the_command_line_boundary` is not edited.
    // This is the ONE pre-existing row plan 19-14 converts, and it is named in
    // `19-14-SUMMARY.md` with its old verdict, its new verdict and this
    // sentence.
    refuses_under(
        root,
        "env -u git $X push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    // **The discriminating pair from `19-SECURITY.md`, in ONE test so they cannot
    // drift apart.** These two commands differ by a single word. Both must be
    // refused; today the second exits 0, and that one word is the whole of
    // `T-19-60`'s surviving wrapper-operand sub-class.
    refuses_under(
        root,
        "env -u SOME_VAR git push --force origin main",
        policy::REASON_FORCE_PUSH_BLOCKED,
    );
    refuses_under(
        root,
        "env -u git git push --force origin main",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
}

#[test]
fn the_disclosed_costs_of_the_command_position_rule_are_pinned_beside_the_spellings_that_work() {
    // **Everything below asserts a COST of this fix, not a desirable
    // behaviour.** A cost that is not pinned is a cost nobody can tell has grown,
    // and each row is paired with the spelling that still works so the boundary
    // of the cost is visible rather than asserted about. A future change that
    // REDUCES one of these costs must delete its row deliberately rather than
    // discover it failing.
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // Cost 1 — a UNIX account called `git` is indistinguishable from a decoy.
    refuses_under(
        root,
        "sudo -u git git status",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    permits(root, "git status");
    permits(root, "sudo -E git status");

    // Cost 2 — the same on the forge path, where the read command is ordinary.
    refuses_under(
        root,
        "env -u gh gh pr list --limit 5",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    permits(root, "gh pr list --limit 5");
    permits(root, "env gh pr list --limit 5");

    // Cost 3 — an expansion anywhere in the WRAPPER PREFIX region makes the
    // prefix unresolvable, because what it does to the environment is decided
    // after the guard has answered. Restricted to that region: an expansion in
    // the assignment prefix or in the program's own arguments is untouched.
    refuses_under(
        root,
        "timeout $T git fetch origin",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );
    permits(root, "git fetch origin");
    permits(root, "timeout 60 git fetch origin");
}

#[test]
fn a_wrapped_forge_command_quoting_a_governed_name_is_refused_and_the_unwrapped_one_is_not() {
    // Cost 4, in its own test with a root per row because each of these touches
    // the PR ledger and a shared root would produce a cap refusal that looks like
    // the verdict under test.
    //
    // This is `T-19-75`'s accepted class extended to governed heads rather than a
    // new kind of cost: WRAPPED, the quoted title is a second candidate and the
    // position cannot be established; UNWRAPPED, the head answers immediately and
    // the title is just a string.
    let wrapped = TempDir::new().expect("a temporary envelope root");
    refuses_under(
        wrapped.path(),
        "env gh pr create --title \"git push --force\"",
        policy::REASON_ENVELOPE_ASSERTION_FAILED,
    );

    let plain = TempDir::new().expect("a temporary envelope root");
    permits(plain.path(), "gh pr create --title \"git push --force\"");
}

// ---------------------------------------------------------------------------
// 9. The disclosure in the code and the disclosure in the test cannot drift
//    apart (`T-19-79`, D-27)
// ---------------------------------------------------------------------------

/// The doc comment immediately above the line that starts with `item`,
/// normalised to one line.
///
/// The `///` markers are stripped and the lines joined with single spaces, so a
/// phrase that happens to wrap in the source still matches — a pin that broke
/// every time someone reflowed a comment would be a pin that gets deleted.
fn doc_comment_above(source: &str, item: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let index = lines
        .iter()
        .position(|line| line.trim_start().starts_with(item))
        .unwrap_or_else(|| {
            panic!(
                "`{item}` was not found in the included source at all. A red here means the \
                 function was renamed or moved, not that its disclosure is intact — every \
                 assertion below would otherwise read an empty string and pass."
            )
        });

    let mut start = index;
    while start > 0 && lines[start - 1].trim_start().starts_with("///") {
        start -= 1;
    }

    lines[start..index]
        .iter()
        .map(|line| line.trim_start().trim_start_matches('/').trim())
        .collect::<Vec<&str>>()
        .join(" ")
}

#[test]
fn resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover() {
    let doc = doc_comment_above(POLICY_SOURCE, "pub fn resolve_program(");

    // -----------------------------------------------------------------------
    // ANTI-VACUITY FIRST. An extraction that silently yielded an empty string
    // would let every `contains` below pass having read nothing — the same
    // shape of certification this whole file exists to argue against.
    // -----------------------------------------------------------------------
    assert!(
        doc.len() > 400,
        "the extracted doc region for `resolve_program` is {} bytes, which is too short to \
         be the disclosure. The extractor has failed, and every substring assertion below \
         would be reading an empty or truncated string. Region was: {doc:?}",
        doc.len()
    );

    for required in [
        // The identifier, so a reader can find the threat register row.
        "T-19-74",
        // The SHAPE, so the disclosure names what is actually open rather than
        // only pointing at a ticket.
        "env $X push --force",
        "bound outside this command line",
        // And that the shape is PERMITTED, which is the fact a reader needs.
        "and is permitted",
        // The paired over-refusal, disclosed in the same doc and pinned in
        // `a_search_for_an_allowed_git_command_runs_and_a_search_for_a_refused_one_does_not`.
        "T-19-75",
    ] {
        assert!(
            doc.contains(required),
            "`resolve_program`'s doc comment no longer contains `{required}`.\n\n\
             This is the honesty statement `19-06`/`19-09` established the discipline for \
             (D-27): an accepted limitation that can be deleted without a test failing is a \
             limitation that gets deleted, and the reader of the code meets the resolver \
             without meeting what it does not cover.\n\n\
             The correct response is to RESTORE the disclosure, or to CLOSE the residual \
             and delete these rows deliberately along with the pins in \
             `the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it`. \
             Deleting this assertion is neither.\n\n\
             Extracted region was: {doc}"
        );
    }
}

// ===========================================================================
// 10. `T-19-89` — the alphabets this corpus could not draw, and the forge
//     slots no enumerated row can generate (plan 19-14, Rule A half)
// ===========================================================================
//
// **The finding, in the auditor's own terms and not softened.** Audit 3 read all
// five alphabets in this file — `ASSIGNMENT_PREFIXES`, `WRAPPERS`,
// `SHELL_LAYERS`, `DECOY_OPERANDS` and `REFUSED_BASES` — and found that **no
// entry of any of them contains a `$`, a `` ` ``, a `{` or a `(`**. The corpus
// therefore cannot generate, and so cannot fail on: an expansion in a base's
// verb (`T-19-88`), an expansion in a wrapper or decoy operand, an assignment
// prefix whose value reaches a rule, or any brace/paren grouping (`T-19-87`).
//
// That is `T-19-76`'s failure mode for the **THIRD consecutive round**, at the
// next radius out — a corpus certifying a fix while being structurally incapable
// of failing on the class the fix is about. This section exists because of that
// sentence, which is also why it must never be narrowed: a case that stops being
// drawn is a case that stops being able to fail.
//
// **This is the Rule A half only.** Plan 19-14 widens `ASSIGNMENT_PREFIXES`,
// `REFUSED_BASES` and `DECOY_OPERANDS`, adds `EXPANSION_WRAPPERS`, and adds the
// forge-slot property below. `19-15` completes the alphabets — `SHELL_LAYERS`
// still offers only `sh -c '…'` and `bash -lc "…"`, never `{ …; }` or `( … )`,
// and there is no severed-prefix alphabet yet. **`T-19-89` is NOT claimed closed
// by plan 19-14.**

/// The characters whose presence in a word makes it an expansion the guard
/// cannot read, or a grouping character that fragments the splitter.
const EXPANSION_METACHARACTERS: &[char] = &['$', '`', '{', '('];

/// Whether an alphabet entry carries one of them.
fn carries_a_metacharacter(entry: &str) -> bool {
    entry.chars().any(|ch| EXPANSION_METACHARACTERS.contains(&ch))
}

/// Whether a generated command line carries a `$` or a `` ` `` **outside single
/// quotes** — i.e. one the shell would actually expand.
///
/// Single quotes are the only construct that suppresses expansion entirely; a
/// `$` inside double quotes still expands, which is why only `'` toggles here.
fn carries_a_live_expansion(command: &str) -> bool {
    let mut in_single = false;
    for ch in command.chars() {
        match ch {
            '\'' => in_single = !in_single,
            '$' | '`' if !in_single => return true,
            _ => {}
        }
    }
    false
}

#[test]
fn every_alphabet_this_phase_widens_can_draw_an_expansion_metacharacter() {
    // **The direct mechanical inverse of audit 3's finding, and the floor that
    // makes the finding un-reintroducible.** The auditor established the defect
    // by READING the alphabets; this asserts the repaired fact so that narrowing
    // one of them back turns this red instead of quietly restoring a corpus that
    // cannot fail on its own class.
    //
    // **Plan 19-15 completes the set.** `SHELL_LAYERS` is checked through the
    // SPELLING each layer produces rather than through its name, because a layer
    // is an enum rather than a string and what audit 3 read was the shell each
    // alphabet can emit. `SEVERED_PREFIXES` is Rule B's own alphabet. With these
    // two the floor now covers every alphabet named in audit 3's axis table, which
    // is what closes `T-19-89`.
    let layer_spellings: Vec<String> = SHELL_LAYERS
        .iter()
        .map(|layer| layer.apply("CMD"))
        .collect();
    let layer_entries: Vec<&str> = layer_spellings.iter().map(String::as_str).collect();

    for (name, entries) in [
        ("ASSIGNMENT_PREFIXES", ASSIGNMENT_PREFIXES),
        ("REFUSED_BASES", REFUSED_BASES),
        ("DECOY_OPERANDS", DECOY_OPERANDS),
        ("EXPANSION_WRAPPERS", EXPANSION_WRAPPERS),
        ("SHELL_LAYERS", layer_entries.as_slice()),
        ("SEVERED_PREFIXES", SEVERED_PREFIXES),
    ] {
        let drawable: Vec<&&str> = entries
            .iter()
            .filter(|entry| carries_a_metacharacter(entry))
            .collect();
        assert!(
            !drawable.is_empty(),
            "`{name}` contains no entry carrying a `$`, a backtick, a `{{` or a `(`.\n\n\
             This is audit 3's `T-19-89` finding restated: an alphabet that cannot DRAW an \
             expansion is an alphabet whose property cannot FAIL on one, and every case \
             generated from it certifies a claim about a class it could never have \
             exercised. It is the same defect as `T-19-76` and `T-19-83`, one radius \
             further out, and plan 19-14 exists partly because it had by then happened \
             three rounds running.\n\n\
             The correct response is to RESTORE the entries, not to delete this floor. \
             Entries were: {entries:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 10a. An expansion in the WRAPPER PREFIX, generated rather than enumerated
// ---------------------------------------------------------------------------

/// One generated splice, including the command BEFORE the outer shell layer is
/// applied.
///
/// **Why the pre-layer string is carried and `Wrapped`/`Decoyed` do not carry
/// it.** The counted expansion floor below asks how many generated cases carry a
/// metacharacter the shell would actually expand. `ShellLayer::ShSingleQuoted`
/// wraps the whole command in single quotes, so a count taken over the FINAL
/// string would answer "none" for every `sh -c '…'` case — while the guard, which
/// splits that payload and classifies it, sees the expansion perfectly well. The
/// count is therefore taken over the command the guard ultimately classifies.
struct Spliced {
    command: String,
    inner: String,
    chain: String,
    recipe: String,
}

/// Splice `entry` into a drawn wrapper chain around `base`, exactly as
/// [`wrap_with_decoy`] does.
fn splice_prefix(rng: &mut Lcg, base: &str, entry: &str) -> Spliced {
    let prefix = ASSIGNMENT_PREFIXES[rng.pick(ASSIGNMENT_PREFIXES.len())];
    let layer = SHELL_LAYERS[rng.pick(SHELL_LAYERS.len())];
    let quotes = layer.inner_quotes();
    let depth = rng.pick(3);

    let mut chain: Vec<&str> = Vec::with_capacity(depth + 1);
    for _ in 0..depth {
        chain.push(WRAPPERS[rng.pick(WRAPPERS.len())]);
    }
    let at = rng.pick(depth + 1);
    chain.insert(at, entry);

    let mut spelled = String::new();
    for item in &chain {
        let quote = quotes[rng.pick(quotes.len())];
        match item.split_once(' ') {
            Some((program, rest)) => spelled.push_str(&format!("{quote}{program}{quote} {rest} ")),
            None => spelled.push_str(&format!("{quote}{item}{quote} ")),
        }
    }

    let inner = format!("{prefix}{spelled}{base}");
    Spliced {
        command: layer.apply(&inner),
        inner,
        chain: format!("[{}] expansion@{at}", chain.join(", ")),
        recipe: format!("prefix={prefix:?} layer={layer:?} depth={depth} entry={entry:?} at={at}"),
    }
}

/// 15 x 8 x 4 = 480, over the 300 counted floor.
const VARIANTS_PER_EXPANSION_WRAPPER: usize = 4;

/// The floors, asserted BEFORE the loop.
const MIN_EXPANSION_WRAPPERS: usize = 6;
const MIN_EXPANSION_WRAPPER_CASES: usize = 400;
const MIN_DISTINCT_EXPANSION_CHAINS: usize = 100;

/// **The counted floor the plan sets**: at least this many GENERATED cases must
/// carry an expansion metacharacter the shell would expand, counted while
/// generating rather than inferred from alphabet sizes.
const MIN_LIVE_EXPANSION_CASES: usize = 300;

#[test]
fn a_wrapper_prefix_carrying_an_expansion_is_refused_in_every_generated_position() {
    let envelope = TempDir::new().expect("a temporary envelope root");
    let root = envelope.path();

    // --- floor 1: the alphabet exists and is wide enough ------------------
    assert!(
        EXPANSION_WRAPPERS.len() >= MIN_EXPANSION_WRAPPERS,
        "the expansion-wrapper alphabet must carry at least {MIN_EXPANSION_WRAPPERS} \
         entries; a narrower one is a row per named shape wearing a property's clothes. \
         Got {}",
        EXPANSION_WRAPPERS.len()
    );

    // --- floor 2: every base is refused UNWRAPPED, with an identifier -----
    //
    // Measured rather than written down, for the same reason the invariance
    // property measures its right-hand side. **This is the floor that is RED
    // against the pre-Rule-A tree**, because `REFUSED_BASES` now carries three
    // bases whose VERB is assembled by expansion.
    for base in REFUSED_BASES {
        let answer = verdict(root, base);
        assert_eq!(
            answer.code, 2,
            "the UNWRAPPED base `{base}` must itself be refused, or every assertion below \
             is comparing a refusal against nothing. Got reason id: {}",
            answer.reason_id
        );
        assert!(
            REASON_IDENTIFIERS.contains(&answer.reason_id.as_str()),
            "the unwrapped base `{base}` must be refused under a member of D-24's \
             taxonomy. Got: {}",
            answer.reason_id
        );
    }

    // --- the property ------------------------------------------------------
    let mut rng = Lcg::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut used: BTreeSet<&str> = BTreeSet::new();
    let mut cases = 0usize;
    let mut live_expansion_cases = 0usize;

    for base in REFUSED_BASES {
        for entry in EXPANSION_WRAPPERS {
            for _ in 0..VARIANTS_PER_EXPANSION_WRAPPER {
                let case = splice_prefix(&mut rng, base, entry);
                chains.insert(case.chain.clone());
                used.insert(entry);
                cases += 1;
                if carries_a_live_expansion(&case.inner) {
                    live_expansion_cases += 1;
                }

                let got = verdict(root, &case.command);
                assert_eq!(
                    got.code,
                    2,
                    "\n\nA WRAPPER PREFIX CARRYING AN EXPANSION WAS PERMITTED.\n\
                     \n  command : {}\
                     \n  base    : {base}\
                     \n  recipe  : {}\
                     \n  got     : exit {} reason {}\
                     \n  seed    : {SEED:#x}\n\
                     \nWhat that prefix does to the environment the program runs in — and \
                     therefore which program runs and with what — is decided AFTER the guard \
                     has answered, so an unreadable word in the prefix is a decision the \
                     guard cannot make.\n\
                     \n**The correct response is a change to the decision-region rule in \
                     `src/envelope/policy.rs`.** It is NOT a wrapper name added to a list, it \
                     is NOT a wrapper FLAG added to a list — a rule keyed on what `-u` means \
                     is a wrapper-name list wearing a flag's clothes — and it is NOT a \
                     narrowing of this alphabet. Audit 3 found that no alphabet in this file \
                     could draw a `$` at all; making this green by removing the entry would \
                     be restoring exactly that defect.\n",
                    case.command,
                    case.recipe,
                    got.code,
                    got.reason_id,
                );
                assert!(
                    REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
                    "\n\nREFUSED WITHOUT A D-24 IDENTIFIER.\n\
                     \n  command : {}\
                     \n  recipe  : {}\
                     \n  reason  : {}\n\
                     \nA refusal that parks under no member of D-24's taxonomy cannot be \
                     found by a later reader grepping for a disarmed layer.\n",
                    case.command,
                    case.recipe,
                    got.reason_id,
                );
            }
        }
    }

    // --- floors 3, 4, 5, 6 -------------------------------------------------
    assert!(
        cases >= MIN_EXPANSION_WRAPPER_CASES,
        "the property must run over at least {MIN_EXPANSION_WRAPPER_CASES} generated \
         cases. Got {cases}"
    );
    assert!(
        chains.len() >= MIN_DISTINCT_EXPANSION_CHAINS,
        "the property must use at least {MIN_DISTINCT_EXPANSION_CHAINS} DISTINCT chains, \
         or hundreds of cases are one row counted hundreds of times. Got {}",
        chains.len()
    );
    assert_eq!(
        used.len(),
        EXPANSION_WRAPPERS.len(),
        "every entry of `EXPANSION_WRAPPERS` must appear in at least one generated case; \
         an entry nobody drew is an entry that cannot fail. Unused: {:?}",
        EXPANSION_WRAPPERS
            .iter()
            .filter(|entry| !used.contains(*entry))
            .collect::<Vec<_>>()
    );
    assert!(
        live_expansion_cases >= MIN_LIVE_EXPANSION_CASES,
        "at least {MIN_LIVE_EXPANSION_CASES} GENERATED cases must carry an expansion \
         metacharacter the shell would actually expand, COUNTED while generating rather \
         than inferred from the alphabet sizes. Audit 3's finding was about what the \
         corpus can draw, and a floor derived from alphabet arithmetic would restate the \
         alphabet rather than measure the corpus. Got {live_expansion_cases} of {cases}"
    );

    println!(
        "expansion-wrapper corpus: {cases} generated cases, {live_expansion_cases} carrying \
         a live expansion, {} distinct chains, {} wrapper entries, seed {SEED:#x}",
        chains.len(),
        EXPANSION_WRAPPERS.len()
    );
}

// ---------------------------------------------------------------------------
// 10b. The FORGE SLOTS, with a fresh envelope root per case
// ---------------------------------------------------------------------------

/// Which subcommand slot of a forge command line carries the expansion.
///
/// **Why this property exists, and why it varies the SLOT.** The three region
/// cells plan 19-14 was corrected for during plan-check were the SECOND
/// subcommand word, the `-`-initial `api` flag, and the endpoint DISPLACED past
/// the first two subcommand words by an option value only the `api` arm's own
/// scan skips. A corpus that varied only the FIRST word would not have failed on
/// any of the three. Every enumerated forge row in
/// `tests/envelope_expansion_slots.rs` fixes one slot; this varies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ForgeSlot {
    /// `gh <EXP> create --title x` — `pr_command_label`'s first matched word.
    First,
    /// `gh pr <EXP> --title x` — the cell a ONE-word region leaves open.
    Second,
    /// `gh api /repos/o/r/<EXP> …` — the endpoint in its ordinary position.
    ApiEndpoint,
    /// `gh api <OPT> <VALUE> repos/o/r/<EXP> …` — the endpoint pushed past the
    /// first two subcommand words by an option value that ONLY
    /// `gh_api_posts_a_pull_request`'s scan skips.
    ApiDisplacedEndpoint,
    /// `gh api … -X <EXP> …` — the method value.
    ApiMethod,
    /// `gh api … <EXP> title=x` — a token that BEGINS with an expansion marker.
    ApiFlagMarker,
    /// `gh api … -<EXP> title=x` — the same hole ONE CHARACTER TO THE LEFT: the
    /// token begins with `-`, so it is neither one of the first two subcommand
    /// words nor the method value nor marker-initial.
    ApiFlagDash,
}

const FORGE_SLOTS: &[ForgeSlot] = &[
    ForgeSlot::First,
    ForgeSlot::Second,
    ForgeSlot::ApiEndpoint,
    ForgeSlot::ApiDisplacedEndpoint,
    ForgeSlot::ApiMethod,
    ForgeSlot::ApiFlagMarker,
    ForgeSlot::ApiFlagDash,
];

/// The `GH_API_VALUE_OPTS` pairs that displace the endpoint out of
/// `subcommand_words`' first two words. Each is a whole option-and-value pair.
///
/// `subcommand_words` skips only `FORGE_VALUE_OPTS` (`-R`, `--repo`,
/// `--hostname`), so every VALUE below is an ordinary non-flag word to it.
const DISPLACING_OPTS: &[(&str, &str)] = &[
    ("-f", "title=x"),
    ("-H", "accept:x"),
    ("-q", ".number"),
    ("-t", "{{.number}}"),
];

/// The three spellings of "an expansion-carrying word standing where `word`
/// should be".
///
/// 0 is a plain variable, 1 is the BRACE form that fragments the splitter, and 2
/// is a backtick command substitution followed by the literal rest — the
/// spelling audit 3 measured separately because it reaches the same slot through
/// a different tokenizer arm.
///
/// **Spellings 3 and 4 are `T-19-95`'s round-5 half and are drawn by a property
/// of their own**, not by
/// `a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot`,
/// whose loop runs `0..3` and is left byte-identical. 3 is a brace-expansion
/// SPLICE and 4 is a PATHNAME expansion — neither sets `Token.expansion`, which
/// is the whole of audit 4's finding about this file.
fn expansion_spelling(spelling: usize, name: &str, word: &str) -> String {
    match spelling {
        0 => format!("${name}"),
        1 => format!("${{{name}}}"),
        2 => format!("`true`{word}"),
        // A whole-word splice whose first alternative is the real word, so bash
        // really does assemble the forge command: `gh {pr,x} create --title x`
        // runs `gh pr x create --title x`. The guard sees `gh` with an empty
        // argv and answers `Allow`.
        3 => format!("{{{word},x}}"),
        // A pathname expansion over the same word — the last character replaced
        // by a `?`, so a matching file in the working directory reassembles it.
        _ => {
            let mut glob: String = word.chars().collect();
            glob.pop();
            glob.push('?');
            glob
        }
    }
}

/// The spellings that make a word UNREADABLE without marking it — the two
/// `19-16` adds. Kept as a named constant so the per-class floor below counts
/// the same thing the property generates.
const UNREADABLE_SPELLINGS: &[usize] = &[3, 4];

/// The command line for one (slot, spelling, displacing option) case.
fn forge_slot_case(slot: ForgeSlot, spelling: usize, displacer: (&str, &str)) -> String {
    match slot {
        ForgeSlot::First => format!(
            "gh {} create --title x",
            expansion_spelling(spelling, "P", "pr")
        ),
        ForgeSlot::Second => format!(
            "gh pr {} --title x",
            expansion_spelling(spelling, "P", "create")
        ),
        ForgeSlot::ApiEndpoint => format!(
            "gh api /repos/o/r/{} -X POST -f title=x",
            expansion_spelling(spelling, "E", "pulls")
        ),
        ForgeSlot::ApiDisplacedEndpoint => format!(
            "gh api {} {} repos/o/r/{} -f title=y",
            displacer.0,
            displacer.1,
            expansion_spelling(spelling, "E", "pulls")
        ),
        ForgeSlot::ApiMethod => format!(
            "gh api repos/o/r/pulls -X {} -f title=x",
            expansion_spelling(spelling, "M", "POST")
        ),
        ForgeSlot::ApiFlagMarker => format!(
            "gh api repos/o/r/pulls {} title=x",
            expansion_spelling(spelling, "F", "-f")
        ),
        ForgeSlot::ApiFlagDash => format!(
            "gh api repos/o/r/pulls -{} title=x",
            expansion_spelling(spelling, "F", "f")
        ),
    }
}

/// Every file under `dir`, recursively — so a "no ledger line" claim is
/// OBSERVED by walking the envelope root rather than derived from a read of one
/// expected path. A row that only read `ledger::ledger_path_in` would pass
/// identically if the line were written somewhere else.
fn files_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// Every pull-request ledger line anywhere under the envelope root.
fn ledger_lines_under(root: &Path) -> Vec<String> {
    let mut lines = Vec::new();
    for path in files_under(root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if line.contains("\"platform\"") {
                lines.push(format!("{}: {line}", path.display()));
            }
        }
    }
    lines
}

/// Two wrapper depths — 0 and 1 — because each case builds an envelope root of
/// its own and the point of the property is the SLOT rather than the chain.
const FORGE_WRAPPER_DEPTHS: usize = 2;

/// The floors, asserted BEFORE the loop.
const MIN_FORGE_SLOT_CASES: usize = 50;

#[test]
fn a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot() {
    // --- floor 0: the POSITIVE control for the walk -----------------------
    //
    // Every case below asserts that the walked envelope root holds NO ledger
    // line. Without this row those assertions could all be passing because the
    // walk cannot see anything: "the line is not there" and "this scanner cannot
    // read the file" are indistinguishable without a positive control.
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "a PERMITTED `gh pr create` writes exactly one ledger line, and the walk must be \
         able to find it. If this is 0 the walk is blind and every no-ledger-line \
         assertion below is vacuous. Files: {:?}",
        files_under(control.path())
    );

    // --- floor 1: the slot alphabet spans what the region checks were about
    assert!(
        FORGE_SLOTS.len() >= 7,
        "the slot alphabet must span the first word, the second word, the `api` endpoint, \
         the DISPLACED `api` endpoint, the method and BOTH flag spellings. Got {}",
        FORGE_SLOTS.len()
    );

    let mut rng = Lcg::new();
    let mut slots_seen: BTreeSet<ForgeSlot> = BTreeSet::new();
    let mut displacers_seen: BTreeSet<&str> = BTreeSet::new();
    let mut commands: BTreeSet<String> = BTreeSet::new();
    let mut cases = 0usize;
    let mut live_expansion_cases = 0usize;

    for slot in FORGE_SLOTS {
        // Only the displaced-endpoint slot varies the displacing option; every
        // other slot draws it once so its case count stays modest.
        let displacers: &[(&str, &str)] = if *slot == ForgeSlot::ApiDisplacedEndpoint {
            DISPLACING_OPTS
        } else {
            &DISPLACING_OPTS[..1]
        };

        for spelling in 0..3 {
            for displacer in displacers {
                for depth in 0..FORGE_WRAPPER_DEPTHS {
                    let base = forge_slot_case(*slot, spelling, *displacer);
                    let command = if depth == 0 {
                        base.clone()
                    } else {
                        format!("{} {base}", WRAPPERS[rng.pick(WRAPPERS.len())])
                    };

                    slots_seen.insert(*slot);
                    if *slot == ForgeSlot::ApiDisplacedEndpoint {
                        displacers_seen.insert(displacer.0);
                    }
                    commands.insert(command.clone());
                    cases += 1;
                    if carries_a_live_expansion(&command) {
                        live_expansion_cases += 1;
                    }

                    // A fresh root per case, because a forge command that is
                    // PERMITTED writes a ledger line and a shared root would
                    // exhaust the 3/1 cap and turn later cases red for a reason
                    // that has nothing to do with the slot under test.
                    let envelope = TempDir::new().expect("a temporary envelope root");
                    let got = verdict(envelope.path(), &command);

                    assert_eq!(
                        got.code,
                        2,
                        "\n\nA FORGE DECISION WORD CARRYING AN EXPANSION WAS PERMITTED.\n\
                         \n  command : {command}\
                         \n  slot    : {slot:?}\
                         \n  spelling: {spelling} (0=$W 1=${{W}} 2=`true`W)\
                         \n  depth   : {depth}\
                         \n  got     : exit {} reason {}\
                         \n  seed    : {SEED:#x}\n\
                         \n`pr_command_label` matches its arms on WORDS. A word it cannot read \
                         matches no arm, so `None` is returned and the command is neither \
                         refused NOR counted — the SAFE-06 cap is BYPASSED rather than \
                         exceeded, and the cap has no second carrier (`T-19-35`).\n\
                         \n**The correct response is a change to the decision-region rule in \
                         `src/envelope/policy.rs`, with every index reported by the scan the \
                         classifier itself runs.** It is NOT a name or a flag added to a list \
                         in `src/`, and it is NOT a narrowing of this slot alphabet: the three \
                         cells this property exists for — the SECOND subcommand word, the \
                         `-`-initial flag and the DISPLACED endpoint — were each missed once \
                         already by a region derived from the wrong scan.\n",
                        got.code,
                        got.reason_id,
                    );
                    assert!(
                        REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
                        "\n\nREFUSED WITHOUT A D-24 IDENTIFIER.\n  command : {command}\n  \
                         reason  : {}\n",
                        got.reason_id
                    );

                    let written = ledger_lines_under(envelope.path());
                    assert!(
                        written.is_empty(),
                        "\n\nA REFUSED FORGE COMMAND STILL WROTE A LEDGER LINE.\n\
                         \n  command : {command}\
                         \n  slot    : {slot:?}\
                         \n  written : {written:?}\n\
                         \nThe refusal must be taken BEFORE the ledger write, or an argv the \
                         guard refuses consumes cap budget for a pull request that never \
                         opens. Observed by WALKING the envelope root, not by reading one \
                         expected path.\n"
                    );
                }
            }
        }
    }

    // --- the SLOT-COVERAGE floor ------------------------------------------
    //
    // The floor that makes "generatable" true rather than claimed. A slot the
    // generator never drew is a slot covered only by the enumerated row that
    // named it, which is the state this property exists to leave behind.
    assert_eq!(
        slots_seen.len(),
        FORGE_SLOTS.len(),
        "every slot this property varies must appear in at least one generated case. \
         Missing: {:?}",
        FORGE_SLOTS
            .iter()
            .filter(|slot| !slots_seen.contains(slot))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        displacers_seen.len(),
        DISPLACING_OPTS.len(),
        "the DISPLACED-endpoint slot must be generated with EVERY option it displaces \
         with. `subcommand_words` skips only `FORGE_VALUE_OPTS`, so each of these option \
         VALUES is an ordinary word to it and pushes the endpoint out of the first two — \
         and an option nobody drew is a displacement nobody tested. Missing: {:?}",
        DISPLACING_OPTS
            .iter()
            .filter(|(opt, _)| !displacers_seen.contains(opt))
            .collect::<Vec<_>>()
    );
    assert!(
        cases >= MIN_FORGE_SLOT_CASES,
        "the forge-slot property must run over at least {MIN_FORGE_SLOT_CASES} generated \
         cases. Got {cases}"
    );
    assert_eq!(
        live_expansion_cases, cases,
        "every generated forge case must carry a live expansion metacharacter, counted \
         while generating. Got {live_expansion_cases} of {cases}"
    );

    println!(
        "forge-slot corpus: {cases} generated cases over {} slots and {} displacing \
         options, {} distinct commands, seed {SEED:#x}",
        slots_seen.len(),
        displacers_seen.len(),
        commands.len()
    );
}

// ---------------------------------------------------------------------------
// 10c. The SEVERED-PREFIX alphabet — WHERE the split falls (plan 19-15, Rule B)
// ---------------------------------------------------------------------------
//
// **This alphabet varies WHERE the split falls, not what the fragment says, and
// that distinction is the whole reason it is written this way.** A generator that
// varied the fragment's TEXT — `_COUNT`, `_COMMAND`, `_GLOBAL` — would be the
// withdrawn textual formulation of Rule B wearing a property's clothes: it would
// certify a rule that reads names, and it would be evaded by the same two
// characters that evaded the rule. What actually distinguishes these cases is the
// POSITION of the cut, so that is what is varied: a trailing literal fragment, no
// literal fragment at all, one- and two-character tails, and the
// command-substitution spelling that assembles the name from two harmless halves.
//
// **Why a property of its own rather than more `WRAPPERS` entries.** A severed
// prefix is legitimately STRICTER than the unwrapped base — `git fetch origin`
// alone is permitted and must stay permitted — so folding these into `WRAPPERS`
// would break the invariance property by comparing a refusal against a permit,
// exactly as this file already records for the `GIT_CONFIG_COUNT=0` assignment
// prefix and for `DECOY_OPERANDS`. The property here is therefore REFUSAL under a
// D-24 identifier.
//
// **The bases are deliberately INNOCUOUS git and forge commands.** The harm this
// class names is what the PREFIX does — `env -u GIT_CONFIG_COUNT` removes layer
// 3's `core.hooksPath` carrier, `env -u GIT_SSH_COMMAND` removes
// `IdentitiesOnly=yes` — and it lands whether or not the git command behind it is
// itself refused. A corpus built on `git push --force` bases would be green
// against the pre-Rule-B tree for the base's own reason and could not fail on this
// class at all.

/// Whole wrapper prefixes whose operand is SEVERED from the program behind it.
///
/// Each entry ends in a word-splitting CLOSER — `}` or `)` — with a word in
/// progress before it, so the tokenizer flushes and everything after the closer
/// becomes a segment whose first token is not a command position.
const SEVERED_PREFIXES: &[&str] = &[
    // A trailing literal fragment: the measured carry-forward shape.
    "env -u ${C}_COUNT",
    // No literal fragment at all — the spelling a substring test cannot see.
    "env -u ${C}",
    // A one-character tail, and a two-character tail: the two spellings that duck
    // under any minimum-length floor a textual rule could set.
    "env -u ${C}T",
    "env -u ${C}NT",
    // The command-substitution spelling, which reaches the same severed shape
    // through `(`/`)` and assembles the name from two harmless halves.
    "env -u $(printf %s%s GIT_CONFIG _COUNT)",
    // An ordinary-shell shape carrying no envelope key name anywhere. It is here
    // because Rule B reads no name: if this row were permitted while the rows
    // above were refused, the rule would be textual after all.
    "sudo -u $(id -un)",
    // A different wrapper and a different fragment position, so the class is not
    // one wrapper counted eight times.
    "timeout ${T}s",
    // A wrapper the production logic is mechanically asserted never to know.
    "made-up-wrapper-9000 --flag ${W}x",
];

/// Rule B's alphabet reached through a BRACE EXPANSION rather than through a
/// parameter expansion — `T-19-95`'s round-5 half for the severed axis.
///
/// **A separate alphabet rather than more entries in [`SEVERED_PREFIXES`], and
/// the reason is a floor rather than a preference.**
/// `a_governed_program_behind_a_severed_prefix_is_refused_wherever_the_split_falls`
/// asserts `live_expansion_cases == cases` — EVERY generated severed case must
/// carry a `$` or a backtick the shell would actually expand. That floor is
/// correct for the class it was written for and this plan must not lower it, so
/// a brace spelling, which carries neither character, gets an alphabet and a
/// property of its own instead.
///
/// The first entry is a WHOLE-WORD splice and the second is CONCATENATED, so
/// both classes are drawn on Rule B's own seam. Both are refused TODAY —
/// measured exit 2 under `envelope_assertion_failed` — because `}` severs a word
/// in progress exactly as `${C}`'s does, and both must STAY refused after
/// `19-17`. That is a real requirement rather than a formality: `19-17`
/// reclassifies `{`, and case 1 of its three-way question exists precisely to
/// keep this flush.
const SEVERED_BRACE_PREFIXES: &[&str] = &[
    "env -u {GIT_CONFIG_COUNT,GIT_SSH_COMMAND}",
    "env -u GIT_CONFIG{_COUNT,_COUNT}",
];

/// Governed commands that are PERMITTED unwrapped, so a refusal below can only
/// come from the severed prefix.
const SEVERED_BASES: &[&str] = &[
    "git fetch origin",
    "git status",
    "git log --oneline",
    "glab mr list",
];

/// The floors, asserted BEFORE the loop.
const MIN_SEVERED_PREFIXES: usize = 6;
const MIN_SEVERED_CASES: usize = 50;
const MIN_DISTINCT_SEVERED_CHAINS: usize = 20;

/// Splice a severed prefix into a drawn wrapper chain, exactly as
/// [`splice_prefix`] does for the expansion-wrapper alphabet.
fn splice_severed(rng: &mut Lcg, base: &str, entry: &str) -> Spliced {
    let prefix = ASSIGNMENT_PREFIXES[rng.pick(ASSIGNMENT_PREFIXES.len())];
    let layer = SHELL_LAYERS[rng.pick(SHELL_LAYERS.len())];
    let quotes = layer.inner_quotes();
    let depth = rng.pick(3);

    let mut chain: Vec<&str> = Vec::with_capacity(depth + 1);
    for _ in 0..depth {
        chain.push(WRAPPERS[rng.pick(WRAPPERS.len())]);
    }
    let at = rng.pick(depth + 1);
    chain.insert(at, entry);

    let mut spelled = String::new();
    for item in &chain {
        let quote = quotes[rng.pick(quotes.len())];
        match item.split_once(' ') {
            Some((program, rest)) => spelled.push_str(&format!("{quote}{program}{quote} {rest} ")),
            None => spelled.push_str(&format!("{quote}{item}{quote} ")),
        }
    }

    let inner = format!("{prefix}{spelled}{base}");
    Spliced {
        command: layer.apply(&inner),
        inner,
        chain: format!("[{}] severed@{at}", chain.join(", ")),
        recipe: format!("prefix={prefix:?} layer={layer:?} depth={depth} entry={entry:?} at={at}"),
    }
}

/// 8 x 4 x 2 = 64, over the 50 floor.
const VARIANTS_PER_SEVERED_PREFIX: usize = 2;

#[test]
fn a_governed_program_behind_a_severed_prefix_is_refused_wherever_the_split_falls() {
    // --- floor 1: the alphabet exists and is wide enough ------------------
    assert!(
        SEVERED_PREFIXES.len() >= MIN_SEVERED_PREFIXES,
        "the severed-prefix alphabet must carry at least {MIN_SEVERED_PREFIXES} entries \
         spanning WHERE the split falls — a trailing fragment, no fragment, a one- and a \
         two-character tail, and the command-substitution spelling. A narrower one is a row \
         per named shape wearing a property's clothes. Got {}",
        SEVERED_PREFIXES.len()
    );

    // --- floor 2: every base is PERMITTED unwrapped -----------------------
    //
    // The mirror image of the invariance property's floor 2, and it is the floor
    // that makes this property non-vacuous. If a base were refused on its own
    // account, every case built on it would be green for the base's reason and the
    // property could not fail on the severed prefix at all — which is exactly the
    // certification failure three rounds of this phase have been about.
    for base in SEVERED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let answer = verdict(envelope.path(), base);
        assert_eq!(
            answer.code,
            0,
            "the UNWRAPPED base `{base}` must be PERMITTED, or every case built on it is \
             refused for the BASE's reason and this property cannot fail on the severed \
             prefix at all. Got reason id: {}",
            answer.reason_id
        );
    }

    // --- the property ------------------------------------------------------
    let mut rng = Lcg::new();
    let mut chains: BTreeSet<String> = BTreeSet::new();
    let mut used: BTreeSet<&str> = BTreeSet::new();
    let mut cases = 0usize;
    let mut live_expansion_cases = 0usize;

    for base in SEVERED_BASES {
        for entry in SEVERED_PREFIXES {
            for _ in 0..VARIANTS_PER_SEVERED_PREFIX {
                let case = splice_severed(&mut rng, base, entry);
                chains.insert(case.chain.clone());
                used.insert(entry);
                cases += 1;
                if carries_a_live_expansion(&case.inner) {
                    live_expansion_cases += 1;
                }

                // A fresh root per case, for the reason the forge-slot property
                // records: a case that is PERMITTED against the pre-fix tree can
                // write a ledger line, and a shared root would exhaust the 3/1 cap
                // and turn later cases red for a reason unrelated to the class.
                let envelope = TempDir::new().expect("a temporary envelope root");
                let got = verdict(envelope.path(), &case.command);

                assert_eq!(
                    got.code,
                    2,
                    "\n\nA GOVERNED PROGRAM BEHIND A SEVERED PREFIX WAS PERMITTED.\n\
                     \n  command : {}\
                     \n  base    : {base}\
                     \n  recipe  : {}\
                     \n  got     : exit {} reason {}\
                     \n  seed    : {SEED:#x}\n\
                     \nThe expansion flushed the tokenizer's current word, so what reached \
                     the resolver was a FRAGMENT continuing an enclosing word — and its \
                     first token was mistaken for a command position. In bash the prefix \
                     removes an environment key the envelope injects, which disarms layer \
                     3, while the git command behind it is innocuous and permitted on its \
                     own account. No verb-slot rule can see this: by the time resolution \
                     runs the shape is gone.\n\
                     \n**The correct response is a change to the COMMAND-POSITION rule in \
                     `src/envelope/policy.rs`.** It is NOT a name, a wrapper flag or an \
                     environment-key substring test added to a list in `src/` — that \
                     formulation was measured EVADABLE (move the split point: `${{C}}NT`, \
                     `${{C}}T`, or `${{C}}` with no fragment at all) and UNSHIPPABLE (it \
                     refuses `ROOT=$(git rev-parse --show-toplevel)` and every other \
                     uppercase assignment whose name sits inside an envelope key). And it \
                     is NOT a narrowing of this alphabet: a case that stops being drawn is \
                     a case that stops being able to fail.\n",
                    case.command,
                    case.recipe,
                    got.code,
                    got.reason_id,
                );
                assert!(
                    REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
                    "\n\nREFUSED WITHOUT A D-24 IDENTIFIER.\n\
                     \n  command : {}\
                     \n  recipe  : {}\
                     \n  reason  : {}\n\
                     \nA refusal that parks under no member of D-24's taxonomy cannot be \
                     found by a later reader grepping for a disarmed layer.\n",
                    case.command,
                    case.recipe,
                    got.reason_id,
                );
            }
        }
    }

    // --- floors 3, 4, 5 ----------------------------------------------------
    assert!(
        cases >= MIN_SEVERED_CASES,
        "the property must run over at least {MIN_SEVERED_CASES} generated cases. Got {cases}"
    );
    assert!(
        chains.len() >= MIN_DISTINCT_SEVERED_CHAINS,
        "the property must use at least {MIN_DISTINCT_SEVERED_CHAINS} DISTINCT chains, or \
         dozens of cases are one row counted dozens of times. Got {}",
        chains.len()
    );
    assert_eq!(
        used.len(),
        SEVERED_PREFIXES.len(),
        "every entry of `SEVERED_PREFIXES` must appear in at least one generated case; an \
         entry nobody drew is a split point nobody tested. Unused: {:?}",
        SEVERED_PREFIXES
            .iter()
            .filter(|entry| !used.contains(*entry))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        live_expansion_cases, cases,
        "every generated severed case must carry an expansion metacharacter the shell would \
         actually expand, COUNTED while generating rather than inferred from the alphabet \
         sizes. Got {live_expansion_cases} of {cases}"
    );

    println!(
        "severed-prefix corpus: {cases} generated cases over {} split points and {} bases, \
         {} distinct chains, seed {SEED:#x}",
        SEVERED_PREFIXES.len(),
        SEVERED_BASES.len(),
        chains.len()
    );
}

// ===========================================================================
// 13. `T-19-95` — the alphabets widened to the SEVEN classes that make a word
//     UNREADABLE, rather than the four characters that MARK an expansion
// ===========================================================================
//
// **Audit 4's finding, in the auditor's own terms and not softened.**
//
// > `EXPANSION_METACHARACTERS` is exactly `['$', '`', '{', '(']`. Verified by
// > reading every entry of `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `REFUSED_BASES`,
// > `DECOY_OPERANDS`, `EXPANSION_WRAPPERS`, `SHELL_LAYERS` and
// > `SEVERED_PREFIXES`: **not one entry anywhere contains a comma inside braces,
// > and not one contains a `*`, a `?` or a `[`.** Every entry satisfying the
// > floor does so through an expansion MARKER. The corpus is therefore
// > structurally incapable of generating, and so of failing on, `T-19-92` and
// > `T-19-94`.
//
// **The alphabets model `$`-shaped assembly and nothing else, so any control
// certified by them is certified against `$`-shaped assembly and nothing else.**
// That sentence belongs in this file because it is the reason this section must
// never be narrowed: a case that stops being drawn is a case that stops being
// able to fail. It is the same finding as `T-19-76`, `T-19-83` and `T-19-89`,
// one radius further out — the FOURTH consecutive round in which a control was
// certified by a corpus that could not draw the cell the next audit walked
// through, and the first three were each found by the NEXT audit rather than by
// the round's own evidence.
//
// **The class split is itself a finding, twice over, and both halves were caught
// in PLAN-CHECK rather than by the next audit.** A single "brace expansion"
// class is satisfied by `{a,b}` standing as its own word, and a corpus of
// whole-word splices can never generate `{g..g}it push --force origin main`. A
// class set without MULTI-EXPANSION is satisfied by any single-brace entry, and
// such a corpus can never generate `{g..g}{i..i}t push --force origin main`.
// Both are measured at exit 0 against the pre-`19-17` tree. Hence the predicates
// below: the concatenated class requires a non-empty literal run adjacent to the
// braces WITHIN the same whitespace-delimited word, the range class requires a
// `..` between them, and the multi-expansion class requires at least TWO
// top-level expansion pairs inside ONE word — none of which `{a,b}` can satisfy
// however it is spelled.

/// One `{`…`}` pair found inside a single whitespace-delimited word.
struct BracePair {
    /// Byte offset of the `{`.
    start: usize,
    /// Byte offset of the `}`.
    end: usize,
    /// A comma at the pair's own nesting level — what makes it an alternative
    /// list rather than a literal pair.
    top_level_comma: bool,
    /// A `..` between the braces — what makes it a RANGE, including the
    /// increment form `{a..b..n}`.
    range: bool,
}

impl BracePair {
    /// A pair bash would EXPAND, as opposed to one it passes through unchanged.
    ///
    /// This is the distinction `T-19-93` turns on: `repos/{owner}/{repo}/pulls`
    /// carries two pairs and neither is an expansion, which is why
    /// `printf "[%s]" repos/{owner}/{repo}/pulls` prints it byte-identically.
    fn is_expansion(&self) -> bool {
        self.top_level_comma || self.range
    }
}

/// Every top-level `{`…`}` pair in one word, in source order.
///
/// A `{` immediately preceded by an unquoted `$` is a PARAMETER expansion rather
/// than a brace pair, and is skipped: `${K}` is round 4's class, already drawn by
/// this file's `$` entries, and counting it here would let the old alphabet
/// satisfy the new floors.
fn brace_pairs(word: &str) -> Vec<BracePair> {
    let bytes: Vec<char> = word.chars().collect();
    let mut pairs = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in bytes.iter().enumerate() {
        match ch {
            '{' => {
                let parameter_expansion = index > 0 && bytes[index - 1] == '$';
                if parameter_expansion {
                    continue;
                }
                if depth == 0 {
                    start = index;
                }
                depth += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    let inner: String = bytes[start + 1..index].iter().collect();
                    let mut nesting = 0usize;
                    let mut top_level_comma = false;
                    for inner_ch in inner.chars() {
                        match inner_ch {
                            '{' => nesting += 1,
                            '}' if nesting > 0 => nesting -= 1,
                            ',' if nesting == 0 => top_level_comma = true,
                            _ => {}
                        }
                    }
                    pairs.push(BracePair {
                        start,
                        end: index,
                        top_level_comma,
                        range: inner.contains(".."),
                    });
                }
            }
            _ => {}
        }
    }
    pairs
}

/// A brace expansion occupying its OWN whitespace-delimited word, with no
/// literal run attached to it.
///
/// `{a,b}` satisfies this and nothing else below, which is the point.
fn draws_a_whole_word_splice(entry: &str) -> bool {
    entry.split_whitespace().any(|word| {
        let chars = word.chars().count();
        let pairs = brace_pairs(word);
        pairs.len() == 1
            && pairs[0].is_expansion()
            && pairs[0].start == 0
            && pairs[0].end + 1 == chars
    })
}

/// A brace expansion with a NON-EMPTY LITERAL RUN adjacent to it inside the same
/// whitespace-delimited word.
///
/// **`{a,b}` cannot satisfy this however it is spelled**, and that is the whole
/// reason the class exists separately: bash joins the literal run to every
/// alternative, which is how `g{i,i}t` produces `git` and `{g..g}it` produces
/// `git` while no alternative names anything at all.
fn draws_a_concatenated_splice(entry: &str) -> bool {
    entry.split_whitespace().any(|word| {
        let chars = word.chars().count();
        let pairs = brace_pairs(word);
        if !pairs.iter().any(BracePair::is_expansion) {
            return false;
        }
        // A literal run exists if any character of the word lies OUTSIDE every
        // pair — a prefix, a suffix, or a run between two pairs.
        (0..chars).any(|index| {
            !pairs
                .iter()
                .any(|pair| index >= pair.start && index <= pair.end)
        })
    })
}

/// A `..` between the braces, in either its whole-word or its concatenated
/// spelling, and including the increment form `{a..b..n}`.
fn draws_a_range(entry: &str) -> bool {
    entry
        .split_whitespace()
        .any(|word| brace_pairs(word).iter().any(|pair| pair.range))
}

/// TWO OR MORE brace expansions inside ONE whitespace-delimited word.
///
/// **No SINGLE-BRACE entry can satisfy this**, which is the second half of the
/// class split. Bash composes ACROSS `{`s, so a product computed per-`{` answers
/// `g` for the first pair of `{g..g}{i..i}t` and `it` for the second — neither
/// governed, both sets enumerating cleanly — and permits the command. Only a
/// product composed over the WHOLE WORD reaches it, and only a corpus that can
/// draw two pairs in one word can fail on it.
fn draws_a_multi_expansion_word(entry: &str) -> bool {
    entry.split_whitespace().any(|word| {
        brace_pairs(word)
            .iter()
            .filter(|pair| pair.is_expansion())
            .count()
            >= 2
    })
}

/// A `{`…`}` pair with NEITHER a top-level comma NOR a range — the pair bash
/// passes through byte-identically.
///
/// This is the class `T-19-93` requires to keep working AND be counted, and a
/// corpus that can only draw `{a,b}` cannot fail on the difference between a
/// brace expansion and a brace pair.
fn draws_a_literal_brace_pair(entry: &str) -> bool {
    entry
        .split_whitespace()
        .any(|word| brace_pairs(word).iter().any(|pair| !pair.is_expansion()))
}

/// An unquoted `*`, `?` or `[` — pathname expansion, whose result depends on the
/// working directory and is therefore unknowable at guard time WHETHER OR NOT a
/// file matches today.
///
/// Single quotes are the only construct that suppresses it entirely, so only `'`
/// toggles here — the same discipline `carries_a_live_expansion` uses.
fn draws_a_glob(entry: &str) -> bool {
    let mut in_single = false;
    for ch in entry.chars() {
        match ch {
            '\'' => in_single = !in_single,
            '*' | '?' | '[' if !in_single => return true,
            _ => {}
        }
    }
    false
}

/// An unquoted `~` — tilde expansion, whose result depends on the passwd
/// database of the machine the command will run on.
fn draws_a_tilde(entry: &str) -> bool {
    let mut in_single = false;
    for ch in entry.chars() {
        match ch {
            '\'' => in_single = !in_single,
            '~' if !in_single => return true,
            _ => {}
        }
    }
    false
}

/// One named class and the predicate that decides whether an entry draws it.
type UnreadableClass = (&'static str, fn(&str) -> bool);

/// The seven classes, named once so the per-alphabet floor, the per-class floor
/// and the counted floor all count the same thing.
const UNREADABLE_CLASSES: &[UnreadableClass] = &[
    ("whole-word splice", draws_a_whole_word_splice),
    ("concatenated splice", draws_a_concatenated_splice),
    ("range", draws_a_range),
    ("multi-expansion word", draws_a_multi_expansion_word),
    ("literal brace pair", draws_a_literal_brace_pair),
    ("glob", draws_a_glob),
    ("tilde", draws_a_tilde),
];

/// Whether an entry draws ANY of the seven.
fn carries_an_unreadable_class(entry: &str) -> bool {
    UNREADABLE_CLASSES
        .iter()
        .any(|(_, predicate)| predicate(entry))
}

// ---------------------------------------------------------------------------
// 13a. The floors — per ALPHABET, per CLASS, and COUNTED over generated cases
// ---------------------------------------------------------------------------

#[test]
fn every_alphabet_this_round_widens_can_draw_a_word_the_guard_cannot_read() {
    // **The direct mechanical inverse of audit 4's finding.** The auditor
    // established the defect by READING the alphabets; this asserts the repaired
    // fact, so narrowing one of them back turns this red instead of quietly
    // restoring a corpus that cannot fail on its own class.
    //
    // `SHELL_LAYERS` is deliberately absent: it is NOT widened this round. It
    // already carries `BraceGroup` and `Subshell`, and a brace-EXPANSION layer
    // would be a layer whose verdict is a refusal — which would break the
    // invariance property by being stricter than its base rather than laxer.
    for (name, entries) in [
        ("ASSIGNMENT_PREFIXES", ASSIGNMENT_PREFIXES),
        ("WRAPPERS", WRAPPERS),
        ("REFUSED_BASES", REFUSED_BASES),
        ("PERMITTED_BASES", PERMITTED_BASES),
        ("DECOY_OPERANDS", DECOY_OPERANDS),
        ("EXPANSION_WRAPPERS", EXPANSION_WRAPPERS),
        ("SEVERED_BRACE_PREFIXES", SEVERED_BRACE_PREFIXES),
    ] {
        let drawable: Vec<&&str> = entries
            .iter()
            .filter(|entry| carries_an_unreadable_class(entry))
            .collect();
        assert!(
            !drawable.is_empty(),
            "`{name}` contains no entry carrying a brace expansion, a literal brace pair, a \
             glob character or a tilde.\n\n\
             This is audit 4's `T-19-95` finding restated: an alphabet that cannot DRAW a \
             word the guard cannot read is an alphabet whose property cannot FAIL on one, \
             and every case generated from it certifies a claim about a class it could \
             never have exercised. It is `T-19-76`'s failure mode for the FOURTH \
             consecutive round, after `T-19-83` and `T-19-89`.\n\n\
             The correct response is to RESTORE the entries, never to delete this floor. \
             Entries were: {entries:?}"
        );
    }
}

#[test]
fn the_corpus_can_draw_every_one_of_the_seven_unreadable_classes() {
    // **Per CLASS rather than per alphabet, because one "brace expansion" class
    // is what let two live cells through in two consecutive plan-check rounds.**
    //
    // A floor a WHOLE-WORD splice satisfies is a floor
    // `{g..g}it push --force origin main` walks around. A floor a SINGLE-BRACE
    // entry satisfies is a floor `{g..g}{i..i}t push --force origin main` walks
    // around. Both are measured at exit 0 against the pre-`19-17` tree, and
    // shipping either would be another round of a corpus incapable of failing on
    // the class that got through it.
    let corpus: Vec<&str> = ASSIGNMENT_PREFIXES
        .iter()
        .chain(WRAPPERS.iter())
        .chain(REFUSED_BASES.iter())
        .chain(PERMITTED_BASES.iter())
        .chain(DECOY_OPERANDS.iter())
        .chain(EXPANSION_WRAPPERS.iter())
        .chain(SEVERED_PREFIXES.iter())
        .chain(SEVERED_BRACE_PREFIXES.iter())
        .copied()
        .collect();

    for (class, predicate) in UNREADABLE_CLASSES {
        let drawable: Vec<&&str> = corpus.iter().filter(|entry| predicate(entry)).collect();
        assert!(
            !drawable.is_empty(),
            "no entry in ANY alphabet draws the class `{class}`.\n\n\
             The seven classes are the ways bash makes a word that the guard cannot read: \
             a whole-word splice, a CONCATENATED splice, a RANGE (including the increment \
             form), a MULTI-EXPANSION word, a literal brace pair, a pathname-expansion \
             character and a tilde. They are separate classes and not spellings of one, \
             because the predicates are written so that `{{a,b}}` satisfies only the \
             first: the concatenated predicate requires a literal run adjacent to the \
             braces inside the same word, the range predicate requires a `..` between \
             them, and the multi-expansion predicate requires at least TWO top-level \
             expansion pairs inside one word.\n\n\
             The correct response is to RESTORE the entries, never to relax this \
             predicate or delete this floor."
        );
    }

    // The degenerate-proofing, asserted rather than described. If any of these
    // three ever became true, the floors above would be satisfiable by an
    // alphabet that cannot generate the cells this round is about — which is
    // exactly how the last two plan-check rounds each found a live cell.
    assert!(
        draws_a_whole_word_splice("{a,b}"),
        "`{{a,b}}` IS a whole-word splice — if this is false the first predicate is broken"
    );
    assert!(
        !draws_a_concatenated_splice("{a,b}"),
        "`{{a,b}}` must NOT satisfy the concatenated class: a corpus of whole-word splices \
         cannot fail on `{{g..g}}it push --force origin main`"
    );
    assert!(
        !draws_a_range("{a,b}"),
        "`{{a,b}}` must NOT satisfy the range class: it carries no `..`"
    );
    assert!(
        !draws_a_multi_expansion_word("{a,b}"),
        "`{{a,b}}` must NOT satisfy the multi-expansion class: one pair is not two"
    );
    assert!(
        !draws_a_multi_expansion_word("{a,b} {c,d}"),
        "two pairs in two WORDS must NOT satisfy the multi-expansion class — bash composes \
         across `{{`s only inside ONE word, which is the whole mechanism of \
         `{{g..g}}{{i..i}}t`"
    );
    assert!(
        !draws_a_multi_expansion_word("{a,b}{x}"),
        "a second pair that is LITERAL must NOT satisfy the multi-expansion class: only \
         EXPANSION pairs compose"
    );
    assert!(
        draws_a_multi_expansion_word("{g..g}{i..i}t"),
        "`{{g..g}}{{i..i}}t` IS a multi-expansion word — measured at exit 0 today"
    );
    assert!(
        draws_a_literal_brace_pair("git log -1 HEAD@{0}") && !draws_a_range("HEAD@{0}"),
        "`HEAD@{{0}}` IS a literal brace pair and is NOT a range — the distinction \
         `T-19-93` turns on"
    );
}

/// The counted floors over GENERATED cases, in the shape `MIN_LIVE_EXPANSION_CASES`
/// already establishes: counted while generating rather than inferred from
/// alphabet sizes.
const MIN_UNREADABLE_GENERATED_CASES: usize = 400;
const MIN_GENERATED_CASES_PER_UNREADABLE_CLASS: usize = 20;

#[test]
fn the_generated_corpus_really_produces_each_unreadable_class_in_quantity() {
    // **An alphabet floor is not a generation floor.** An entry can sit in an
    // alphabet and be drawn by nothing, or be drawn once out of thousands of
    // cases — which is a corpus that can technically fail on the class and
    // practically never does. This counts what the generator ACTUALLY emits,
    // over the same recipe shape the invariance property uses.
    let mut rng = Lcg::new();
    let mut total = 0usize;
    let mut unreadable = 0usize;
    let mut per_class: BTreeMap<&str, usize> = BTreeMap::new();

    for base in REFUSED_BASES {
        for _ in 0..VARIANTS_PER_REFUSED_BASE {
            let prefix = ASSIGNMENT_PREFIXES[rng.pick(ASSIGNMENT_PREFIXES.len())];
            let layer = SHELL_LAYERS[rng.pick(SHELL_LAYERS.len())];
            let depth = rng.pick(4);
            let mut chain = String::new();
            for _ in 0..depth {
                chain.push_str(WRAPPERS[rng.pick(WRAPPERS.len())]);
                chain.push(' ');
            }
            let command = layer.apply(&format!("{prefix}{chain}{base}"));

            total += 1;
            if carries_an_unreadable_class(&command) {
                unreadable += 1;
            }
            for (class, predicate) in UNREADABLE_CLASSES {
                if predicate(&command) {
                    *per_class.entry(class).or_default() += 1;
                }
            }
        }
    }

    assert!(
        unreadable >= MIN_UNREADABLE_GENERATED_CASES,
        "the generator emitted only {unreadable} of {total} cases carrying a word the guard \
         cannot read, under the floor of {MIN_UNREADABLE_GENERATED_CASES}.\n\n\
         An entry that sits in an alphabet and is drawn by nothing is an entry that cannot \
         fail on anything. The correct response is to RESTORE the entries, never to lower \
         this floor. Per class: {per_class:?}"
    );

    for (class, _) in UNREADABLE_CLASSES {
        let count = per_class.get(class).copied().unwrap_or(0);
        assert!(
            count >= MIN_GENERATED_CASES_PER_UNREADABLE_CLASS,
            "the generator emitted only {count} cases of the class `{class}`, under the \
             floor of {MIN_GENERATED_CASES_PER_UNREADABLE_CLASS}.\n\n\
             This is `T-19-95` counted rather than read: a class the generator emits a \
             handful of times is a class the property is not really testing. Seed: \
             {SEED:#x}. Full counts: {per_class:?}"
        );
    }

    // Recorded so the SUMMARY carries measured counts rather than described
    // ones. `cargo test -- --nocapture` shows them.
    println!(
        "generated {total} cases from {} refused bases; {unreadable} carry an unreadable \
         class.\nper class: {per_class:?}",
        REFUSED_BASES.len()
    );
}

// ---------------------------------------------------------------------------
// 13b. The forge slots, carrying a SPLICE and a GLOB
// ---------------------------------------------------------------------------

/// **The arithmetic, recounted by plan 19-17 because `19-16`'s was wrong and the
/// floor it produced was unreachable by construction.**
///
/// `19-16` wrote "7 slots x 2 spellings x (1 or 4 displacers) x 2 depths = 68"
/// and set the floor at 50. The loop below gives ONE displacer to six slots and
/// four to `ApiDisplacedEndpoint`, so the real count is
/// `(6 x 1 + 1 x 4) = 10` slot-displacer pairs `x 2 spellings x 2 depths = 40`.
///
/// **This was invisible until the rule landed**, and that is the only reason it
/// is corrected here rather than in the round that wrote it: the property failed
/// earlier, at its per-case refusal assertion, so the floor was never reached.
/// With `19-17`'s rule in place all 40 cases are refused with an empty walk and
/// execution reached this line for the first time.
///
/// **No assertion about a REFUSAL was touched and no alphabet was narrowed.**
/// This is a self-consistency check on the generator's own count, and no
/// production change can move it — it is a pure function of the alphabet sizes
/// in this file. Raising the generated count instead by giving every slot all
/// four displacers would emit duplicates, because `forge_slot_case` ignores the
/// displacer for every slot but `ApiDisplacedEndpoint`, and a floor satisfied by
/// duplicate cases is the dishonest counting this file's floors exist against.
const MIN_UNREADABLE_FORGE_SLOT_CASES: usize = 40;

#[test]
fn a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot() {
    // **A property of its own rather than three more spellings in
    // `a_forge_subcommand_slot_carrying_an_expansion_is_refused_in_every_generated_slot`.**
    // That property is round 4's evidence and its loop runs `0..3`; leaving it
    // byte-identical is what keeps round-4 and round-5 evidence attributable to
    // the round that produced each. `expansion_spelling` gains spellings 3 and 4,
    // which only this property draws.
    //
    // **RED against the pre-`19-17` tree, every case.** Measured, one fresh root
    // per row and the root walked afterwards: `gh {pr,x} create --title x` → 0,
    // `gh pr {create,x} --title x` → 0, `gh api /repos/o/r/{pulls,x} …` → 0,
    // `gh p? create --title x` → 0, `gh pr creat? --title x` → 0,
    // `gh api /repos/o/r/pull? …` → 0, `gh api … -X POS? …` → 0,
    // `gh api … -? title=x` → 0 — every one with an EMPTY WALK, so the SAFE-06
    // cap is bypassed rather than exceeded and the cap has no second carrier
    // (`T-19-35`).
    //
    // Derivations: the splice spelling by clause 2(a) — the segment `gh`
    // resolves `Governed` and its simple command is brace-spliced — and the glob
    // spelling by clause 1, every slot being a decision word `19-14` already
    // named through the scan the classifier itself runs.

    // --- floor 0: the POSITIVE control for the walk -----------------------
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "a PERMITTED `gh pr create` writes exactly one ledger line, and the walk must be \
         able to find it. If this is 0 the walk is blind and every no-ledger-line \
         assertion below is vacuous. Files: {:?}",
        files_under(control.path())
    );

    let mut rng = Lcg::new();
    let mut slots_seen: BTreeSet<ForgeSlot> = BTreeSet::new();
    let mut spellings_seen: BTreeSet<usize> = BTreeSet::new();
    let mut cases = 0usize;
    let mut splice_cases = 0usize;
    let mut glob_cases = 0usize;

    for slot in FORGE_SLOTS {
        let displacers: &[(&str, &str)] = if *slot == ForgeSlot::ApiDisplacedEndpoint {
            DISPLACING_OPTS
        } else {
            &DISPLACING_OPTS[..1]
        };

        for spelling in UNREADABLE_SPELLINGS {
            for displacer in displacers {
                for depth in 0..FORGE_WRAPPER_DEPTHS {
                    let base = forge_slot_case(*slot, *spelling, *displacer);
                    let command = if depth == 0 {
                        base.clone()
                    } else {
                        format!("{} {base}", WRAPPERS[rng.pick(WRAPPERS.len())])
                    };

                    slots_seen.insert(*slot);
                    spellings_seen.insert(*spelling);
                    cases += 1;
                    if draws_a_whole_word_splice(&command) || draws_a_concatenated_splice(&command)
                    {
                        splice_cases += 1;
                    }
                    if draws_a_glob(&command) {
                        glob_cases += 1;
                    }

                    // A fresh root per case, because a forge command that is
                    // PERMITTED writes a ledger line and a shared root would
                    // exhaust the cap and turn later cases red for a reason that
                    // has nothing to do with the slot under test.
                    let envelope = TempDir::new().expect("a temporary envelope root");
                    let got = verdict(envelope.path(), &command);

                    assert_eq!(
                        got.code,
                        2,
                        "\n\nA FORGE DECISION WORD THE SHELL ASSEMBLES WITHOUT MARKING IT WAS \
                         PERMITTED.\n\
                         \n  command : {command}\
                         \n  slot    : {slot:?}\
                         \n  spelling: {spelling} (3=splice `{{word,x}}` 4=glob `wor?`)\
                         \n  depth   : {depth}\
                         \n  got     : exit {} reason {}\
                         \n  seed    : {SEED:#x}\n\
                         \nNeither spelling sets `Token.expansion`. A brace expansion is \
                         spliced back into the same simple command by the shell, and a glob \
                         is resolved from the working directory — so `pr_command_label` \
                         matches no arm, `None` is returned, and the creation is neither \
                         refused NOR counted. The SAFE-06 cap is BYPASSED rather than \
                         exceeded, and it has no second carrier (`T-19-35`).\n\
                         \n**The correct response is a change to the tokenizer's brace \
                         classification or to the decision-word rule in \
                         `src/envelope/policy.rs`.** It is NOT a name or a character added \
                         to a list in `src/`, and it is NOT a narrowing of this slot \
                         alphabet or of `UNREADABLE_SPELLINGS`.",
                        got.code,
                        got.reason_id
                    );

                    let written = ledger_lines_under(envelope.path());
                    assert!(
                        written.is_empty(),
                        "`{command}` was refused, but a pull-request ledger line was written \
                         somewhere under the envelope root — cap budget consumed for a \
                         command that never opens a pull request. Found: {written:?} Files: \
                         {:?}",
                        files_under(envelope.path())
                    );
                }
            }
        }
    }

    assert_eq!(
        slots_seen.len(),
        FORGE_SLOTS.len(),
        "every slot must have been generated. Seen: {slots_seen:?}"
    );
    assert_eq!(
        spellings_seen.len(),
        UNREADABLE_SPELLINGS.len(),
        "both unreadable spellings must have been generated. Seen: {spellings_seen:?}"
    );
    assert!(
        cases >= MIN_UNREADABLE_FORGE_SLOT_CASES,
        "only {cases} cases were generated, under the floor of \
         {MIN_UNREADABLE_FORGE_SLOT_CASES}"
    );
    assert!(
        splice_cases > 0 && glob_cases > 0,
        "the forge axis must draw BOTH classes — a property that could only draw one would \
         certify the fix for half of `T-19-92`/`T-19-94` while staying silent on the other. \
         splice={splice_cases} glob={glob_cases}"
    );
}

// ---------------------------------------------------------------------------
// 13c. `T-19-93` on the forge axis — the class whose bar is COUNT, not refuse
// ---------------------------------------------------------------------------

/// The placements of `gh`'s own documented placeholders that all name the pulls
/// COLLECTION, so every one of them is a pull-request creation and every one
/// must be counted.
const PLACEHOLDER_ENDPOINTS: &[&str] = &[
    "gh api repos/{owner}/{repo}/pulls -f title=x",
    "gh api repos/{owner}/r/pulls -f title=x",
    "gh api repos/o/{repo}/pulls -f title=x",
    "gh api /repos/{owner}/{repo}/pulls -X POST -f title=x",
    "gh api -f title=x repos/{owner}/{repo}/pulls -f title=y",
];

#[test]
fn a_gh_placeholder_endpoint_is_permitted_and_counted_in_every_generated_placement() {
    // **The forge property above could only draw the REFUSED half.** A corpus
    // that generated only refusals would certify the fix for `T-19-92` and
    // `T-19-94` while remaining silent on `T-19-93` — the same defect one class
    // over, and the reason this property exists beside it.
    //
    // `{owner}` and `{repo}` are `gh`'s OWN documented placeholders. Bash passes
    // them through byte-identically because there is no comma between the braces
    // — `printf "[%s]" repos/{owner}/{repo}/pulls` prints it unchanged, measured
    // — and `gh` is what substitutes them. **Refusing these is not the fix**: an
    // agent following `gh`'s manual would be denied, which is how a safety
    // control gets switched off (AR-19-11).
    //
    // So the assertion is a LEDGER LINE, which no implementation can satisfy by
    // refusing. Every placement below measured exit 0 with an EMPTY WALK against
    // the pre-`19-17` tree, while the quoted spelling
    // `gh api "repos/o/r/pulls" -f title=x` already exits 0 WITH a ledger line —
    // the positive control that proves the walk and the cap are both live.
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh api \"repos/o/r/pulls\" -f title=x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "the QUOTED endpoint is counted TODAY — this is the behaviour every placement below \
         must be brought up to, and it is what proves this walk is not blind. Files: {:?}",
        files_under(control.path())
    );

    let mut rng = Lcg::new();
    let mut cases = 0usize;

    for base in PLACEHOLDER_ENDPOINTS {
        // Every placement carries a literal brace pair and no expansion pair,
        // asserted rather than assumed: if one of these ever became a brace
        // EXPANSION the row would be testing a different class entirely.
        assert!(
            draws_a_literal_brace_pair(base) && !draws_a_range(base),
            "`{base}` must carry a LITERAL brace pair — no comma between the braces and no \
             range. That is the whole distinction `T-19-93` turns on."
        );

        for depth in 0..FORGE_WRAPPER_DEPTHS {
            let command = if depth == 0 {
                (*base).to_string()
            } else {
                format!("{} {base}", WRAPPERS[rng.pick(WRAPPERS.len())])
            };
            cases += 1;

            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code,
                0,
                "\n\nA LEGITIMATE `gh api` PLACEHOLDER ENDPOINT WAS REFUSED.\n\
                 \n  command : {command}\
                 \n  got     : exit {} reason {}\
                 \n  seed    : {SEED:#x}\n\
                 \nThis is `gh`'s own documented syntax. The correctness bar for `T-19-93` \
                 is COUNT, not refuse: a rule that denied it would deny an agent following \
                 the manual, which is how a safety control gets switched off (AR-19-11).",
                got.code, got.reason_id
            );

            let written = ledger_lines_under(envelope.path());
            assert_eq!(
                written.len(),
                1,
                "\n\nA PULL-REQUEST CREATION WAS PERMITTED WITHOUT BEING COUNTED.\n\
                 \n  command : {command}\
                 \n  ledger  : {written:?}\
                 \n  files   : {:?}\
                 \n  seed    : {SEED:#x}\n\
                 \nBecause `{{` and `}}` are `SEPARATORS`, the endpoint is fragmented before \
                 either forge scan sees it, `pr_command_label` matches no arm, and the \
                 creation is never counted. The SAFE-06 cap is BYPASSED rather than \
                 exceeded and it has NO second carrier (`T-19-35`).\n\
                 \n**The correct response is to stop the tokenizer fragmenting a brace pair \
                 that contains no comma and no range**, or to teach the forge scans the \
                 placeholders — not to refuse the command, and not to count every endpoint \
                 that merely contains a brace.",
                files_under(envelope.path())
            );
        }
    }

    assert!(
        cases >= PLACEHOLDER_ENDPOINTS.len(),
        "every placement must have been generated. Got {cases}"
    );
}

// ---------------------------------------------------------------------------
// 13d. Rule B's seam reached through a BRACE EXPANSION
// ---------------------------------------------------------------------------

/// 2 prefixes x 4 bases x 2 depths = 16.
const MIN_SEVERED_BRACE_CASES: usize = 12;

#[test]
fn a_governed_program_behind_a_brace_severed_prefix_is_refused_wherever_the_split_falls() {
    // **This property is GREEN today and that is stated first, because a control
    // nobody can tell is already satisfied is exactly this phase's failure
    // mode.** `}` is a word-splitting closer whether the pair it closes is a
    // parameter expansion or a brace expansion, so Rule B already reaches these
    // spellings: both entries measured exit 2 under `envelope_assertion_failed`
    // at this file's base commit. They are drawn so Rule B's seam can be
    // exercised through the OTHER character class, not because they are live
    // bypasses.
    //
    // **What it is load-bearing for is `19-17`, not `19-16`.** `19-17`
    // reclassifies `{` three ways, and case 1 — a `{` preceded in-word by an
    // unquoted `$` — exists solely to keep this flush. If a later change folded
    // the brace-expansion case into the literal-brace case, these lines would
    // stop being severed and this property would turn red where a verdict pin
    // alone might not.

    // --- floor 1: every base is PERMITTED unwrapped -----------------------
    //
    // The mirror image of the invariance property's floor 2. If a base were
    // refused on its own account, every case built on it would be green for the
    // base's reason and the property could not fail on the severed prefix at all.
    for base in SEVERED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let answer = verdict(envelope.path(), base);
        assert_eq!(
            answer.code, 0,
            "the UNWRAPPED base `{base}` must be PERMITTED, or every case built on it is \
             green for the base's own reason. Got reason id: {}",
            answer.reason_id
        );
    }

    let mut rng = Lcg::new();
    let mut cases = 0usize;
    let mut splice_cases = 0usize;
    let mut concatenated_cases = 0usize;

    for entry in SEVERED_BRACE_PREFIXES {
        for base in SEVERED_BASES {
            for depth in 0..2usize {
                let mut chain = String::new();
                for _ in 0..depth {
                    chain.push_str(WRAPPERS[rng.pick(WRAPPERS.len())]);
                    chain.push(' ');
                }
                let command = format!("{chain}{entry} {base}");
                cases += 1;
                if draws_a_whole_word_splice(&command) {
                    splice_cases += 1;
                }
                if draws_a_concatenated_splice(&command) {
                    concatenated_cases += 1;
                }

                let envelope = TempDir::new().expect("a temporary envelope root");
                let got = verdict(envelope.path(), &command);
                assert_eq!(
                    got.code,
                    2,
                    "\n\nA GOVERNED PROGRAM BEHIND A BRACE-SEVERED PREFIX WAS PERMITTED.\n\
                     \n  command : {command}\
                     \n  entry   : {entry}\
                     \n  base    : {base}\
                     \n  depth   : {depth}\
                     \n  got     : exit {} reason {}\
                     \n  seed    : {SEED:#x}\n\
                     \nRule B is POSITIONAL and reads no name: a segment whose immediately \
                     preceding operator is a `}}` that severed a word in progress is a \
                     fragment continuing an enclosing word, so its first token is not a \
                     command position. That must hold for a brace EXPANSION exactly as it \
                     holds for a parameter expansion.\n\
                     \n**The correct response is to restore the word-splitting flush for \
                     `}}`, never to narrow this alphabet.** If a brace-expansion `{{` was \
                     folded into the literal-brace case, this is the property that says so.",
                    got.code, got.reason_id
                );
                assert!(
                    got.reason_id
                        .contains(policy::REASON_ENVELOPE_ASSERTION_FAILED),
                    "and refused under `{}` — the identifier Rule B answers with. A refusal \
                     for another cause would leave the class exactly as open. Got: {}",
                    policy::REASON_ENVELOPE_ASSERTION_FAILED,
                    got.reason_id
                );
            }
        }
    }

    assert!(
        cases >= MIN_SEVERED_BRACE_CASES,
        "only {cases} cases were generated, under the floor of {MIN_SEVERED_BRACE_CASES}"
    );
    assert!(
        splice_cases > 0 && concatenated_cases > 0,
        "Rule B's brace alphabet must draw BOTH a whole-word splice and a CONCATENATED \
         one, counted while generating. A floor a whole-word `{{a,b}}` satisfies is a floor \
         `env -u GIT_CONFIG{{_COUNT,_COUNT}}` walks around. \
         whole-word={splice_cases} concatenated={concatenated_cases}"
    );
}

// ===========================================================================
// 14. `T-19-99` — the SECOND AXIS: the words the shell DELETES
// ===========================================================================
//
// **The gap moved AXIS, not one slot over, and this section must be legible as a
// second axis rather than as two more entries in section 13.**
//
// Audit 5 enumerated bash's word expansions one at a time against round 5's
// inverted rule and found **no gap in word ASSEMBLY** — `T-19-92` … `T-19-95`
// closed, `T-19-93` on the harder COUNT bar. What that inversion answers is
// *"is this word handed over as written"*. What it does not answer is *"is this
// word handed over at all"*.
//
// So `UNREADABLE_CLASSES` and its seven predicates are **word-ASSEMBLY classes**,
// and audit 5's `T-19-99` is that every one of them is:
//
// > `grep -rnE '"(git|gh|glab)[^"]*[<>][^"]*"' tests/ src/` finds **no
// > guard-driven row carrying a redirection anywhere in the repository**. The
// > corpus is therefore structurally incapable of generating, and so of failing
// > on, `T-19-97` and `T-19-98`.
//
// **The seven assembly classes are NOT touched, NOT widened and NOT relaxed.**
// `UNREADABLE_CLASSES`, `brace_pairs`, the seven predicates, the
// degenerate-proofing block, `MIN_UNREADABLE_GENERATED_CASES`,
// `MIN_GENERATED_CASES_PER_UNREADABLE_CLASS` and `MIN_UNREADABLE_FORGE_SLOT_CASES`
// are byte-identical. Smuggling a `>` into them would model DELETION as if it
// were ASSEMBLY and lose exactly the distinction this round is about — which is
// how a corpus comes to model the control it certifies and nothing beside it.
//
// **The invariant this axis is written against, stated once.** The words the
// guard classifies must be exactly the words the program receives, in the same
// order — no more and no fewer. Two mechanisms sit outside the literalness bit,
// and both words are perfectly LITERAL by its own test, correctly so: a
// REDIRECTION is a word the guard reads and the shell deletes from argv,
// displacing every decision word one slot right (`T-19-97`); a BACKSLASH-NEWLINE
// is two characters bash deletes before the word is assembled and `tokenize`
// keeps (`T-19-98`).
//
// **A deletion entry is VERDICT-PRESERVING after the fix, which is the opposite
// of the brace axis and is why this section has a permitted arm at all.** A brace
// expansion spliced into a permitted governed command is REFUSED after `19-17`,
// so `19-16` had to keep it out of the invariance-preserving alphabets. A
// redirection changes no verdict: after `19-19` the surviving argv is what the
// classifier already answers about. `git >/dev/null status` is permitted today
// and must stay permitted; `git >/dev/null push --force origin main` is permitted
// today and must become refused. Both halves are drawn below, or the property
// could not fail on an implementation that simply refuses everything carrying
// a `>`.
//
// **THIS SECTION IS RED AT PLAN 19-18'S END, BY DESIGN**, except the permitted
// arm, the degenerate-proofing block and the counting floors. The refusal
// assertions cannot pass until `19-19`'s rule exists.

/// One whitespace-delimited word of a command, with a per-character record of
/// whether the character was inside quotes.
///
/// **Quoting is tracked because a quoted `>` is an ordinary character.**
/// `git log --grep='>'` is pinned PERMITTED in
/// `tests/envelope_argv_deletion.rs`, and a predicate that counted it would make
/// the floors below satisfiable by a row the rule must never touch — the same
/// discipline `draws_a_glob` and `draws_a_tilde` already use.
struct DeletionWord {
    chars: Vec<char>,
    quoted: Vec<bool>,
}

/// Split an entry into words on UNQUOTED whitespace, carrying the quote mask.
fn deletion_words(entry: &str) -> Vec<DeletionWord> {
    let mut words: Vec<DeletionWord> = Vec::new();
    let mut chars: Vec<char> = Vec::new();
    let mut quoted: Vec<bool> = Vec::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut started = false;

    for ch in entry.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                started = true;
            }
            '"' if !in_single => {
                in_double = !in_double;
                started = true;
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if started {
                    words.push(DeletionWord {
                        chars: std::mem::take(&mut chars),
                        quoted: std::mem::take(&mut quoted),
                    });
                    started = false;
                }
            }
            c => {
                chars.push(c);
                quoted.push(in_single || in_double);
                started = true;
            }
        }
    }
    if started {
        words.push(DeletionWord { chars, quoted });
    }
    words
}

/// Bash's redirection operators, LONGEST FIRST so the match is the longest one.
///
/// `<<-` and `&>>` are named here and DRAWN by `DISPLACING_REDIRECTIONS` below,
/// because they are the two spellings `19-19`'s production uniquely adds: a class
/// list that named them without an entry drawing them would be a floor nothing
/// satisfies.
const REDIRECTION_OPERATORS: &[&str] = &[
    "&>>", "<<<", "<<-", "&>", ">>", "<<", ">|", "<>", ">&", "<&", ">", "<",
];

/// The longest redirection operator starting at `index`, if the whole operator is
/// UNQUOTED.
fn operator_at(word: &DeletionWord, index: usize) -> Option<&'static str> {
    for operator in REDIRECTION_OPERATORS {
        let width = operator.chars().count();
        if index + width > word.chars.len() {
            continue;
        }
        if (index..index + width).any(|k| word.quoted[k]) {
            continue;
        }
        if word.chars[index..index + width]
            .iter()
            .copied()
            .eq(operator.chars())
        {
            return Some(operator);
        }
    }
    None
}

/// The length of the leading all-digits run of a word — bash's IO_NUMBER.
///
/// **An IO_NUMBER is a digits-only run since the START of the word.** That single
/// rule is what makes `git 2>/dev/null push …` a redirection and
/// `git x2>/tmp/o push …` a command with the real argv word `x2` — the
/// OVER-DELETION control pinned PERMITTED in `tests/envelope_argv_deletion.rs`.
fn io_number_width(word: &DeletionWord) -> usize {
    word.chars
        .iter()
        .zip(word.quoted.iter())
        .take_while(|(c, q)| !**q && c.is_ascii_digit())
        .count()
}

/// **Class 1** — an unquoted redirection operator standing as its OWN
/// whitespace-delimited word or LEADING one, with a following word.
///
/// `git >/dev/null push …`, `git > /tmp/o push …` and `git 2>/dev/null push …`
/// satisfy this. `git push>/dev/null …` does NOT: the operator does not lead the
/// word, which is exactly why class 2 is separate.
fn draws_a_separate_word_redirection(entry: &str) -> bool {
    let words = deletion_words(entry);
    if words.len() < 2 {
        return false;
    }
    words.iter().enumerate().any(|(index, word)| {
        index + 1 < words.len() && operator_at(word, io_number_width(word)).is_some()
    })
}

/// **Class 2** — an unquoted redirection operator with a NON-EMPTY literal run
/// before it INSIDE the same whitespace-delimited word, where that run is not an
/// IO_NUMBER.
///
/// **Class 1 cannot satisfy this, and that is the whole reason the class is
/// separate**: the operator need not be its own word, and a whitespace-delimited
/// predicate walks straight around it. `git push>/dev/null --force …` is measured
/// at exit 0 today and prints `ARGV[git]: [push] [--force] [origin] [main]`.
///
/// The OVER-DELETION control `git x2>/tmp/o push …` draws this class too, which
/// is what makes the class drawable in BOTH directions: a rule that deleted the
/// whole word rather than the redirection would lose the real argv word `x2`.
///
/// **Only the FIRST operator occurrence in the word is considered, and getting
/// that wrong over-counted the class by 90 cases when this predicate was first
/// written.** A naive "some operator has a non-empty run before it" reads the
/// second `>` of `>>/tmp/x` as attached to a literal run `>`, and likewise for
/// `<<<x`, `<>/tmp/o`, `&>/tmp/o`, `&>>/tmp/o` and `<<-EOF` — six of the eleven
/// entries, every one of which is a SEPARATE-WORD redirection. Bash takes the
/// LONGEST operator match at the start of the redirection and everything after it
/// is the TARGET, never a literal run, which is exactly what this now encodes.
fn draws_an_attached_redirection(entry: &str) -> bool {
    deletion_words(entry).iter().any(|word| {
        let fd = io_number_width(word);
        (0..word.chars.len())
            .find(|index| operator_at(word, *index).is_some())
            .is_some_and(|first| first > fd)
    })
}

/// **Class 3** — a MULTI-CHARACTER or FD-CARRYING operator: `>>`, `2>`, `1>`,
/// `<<<`, `<<`, `<<-`, `>|`, `<>`, `>&`, `<&`, `&>`, `&>>`.
///
/// **A bare `>` cannot satisfy this**, and the class exists because five of the
/// seven cells found while planning round 6 are exactly here — `&>`, `>|`, `<>`,
/// `<<EOF` and `{v}>`.
fn draws_a_multi_character_or_fd_operator(entry: &str) -> bool {
    deletion_words(entry).iter().any(|word| {
        let fd = io_number_width(word);
        if fd > 0 && operator_at(word, fd).is_some() {
            return true;
        }
        (0..word.chars.len())
            .any(|index| operator_at(word, index).is_some_and(|op| op.chars().count() > 1))
    })
}

/// Every unquoted backslash-newline in an entry, as `(index of the backslash,
/// total length)`.
///
/// Only SINGLE quotes suppress a line continuation — bash performs the
/// continuation inside double quotes, which is why
/// `git "pu\`+NL+`sh" --force origin main` is a row in
/// `tests/envelope_argv_deletion.rs` and the single-quoted spelling is one of
/// audit 5's three DISCARDED measurements.
fn line_continuations(entry: &str) -> (Vec<usize>, Vec<char>) {
    let chars: Vec<char> = entry.chars().collect();
    let mut found = Vec::new();
    let mut in_single = false;
    let mut index = 0usize;
    while index < chars.len() {
        match chars[index] {
            '\'' => in_single = !in_single,
            '\\' if !in_single && chars.get(index + 1) == Some(&'\n') => {
                found.push(index);
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    (found, chars)
}

/// **Class 4** — character-pair deletion INSIDE a word: a backslash-newline with
/// a non-empty literal run on BOTH sides inside one word.
///
/// `git pu\`+NL+`sh --force origin main` satisfies this. **Class 5 cannot.**
fn draws_a_continuation_inside_a_word(entry: &str) -> bool {
    let (found, chars) = line_continuations(entry);
    found.iter().any(|&index| {
        let left = index > 0 && !chars[index - 1].is_whitespace();
        let right = chars
            .get(index + 2)
            .is_some_and(|c| !c.is_whitespace());
        left && right
    })
}

/// **Class 5** — character-pair deletion at a word BOUNDARY: a backslash-newline
/// with whitespace or a word boundary on at least one side.
///
/// `git \`+NL+`push --force origin main` satisfies this, and so does the second
/// pre-existing FALSE REFUSAL `git push \`+NL+` origin refs/heads/gsd-auto/alpha/w`,
/// where the whitespace AFTER the continuation flushes it into its own word in
/// the REMOTE slot. **Class 4 cannot satisfy this and class 5 cannot satisfy
/// class 4**, which is the split that makes the pair non-degenerate.
fn draws_a_continuation_at_a_word_boundary(entry: &str) -> bool {
    let (found, chars) = line_continuations(entry);
    found.iter().any(|&index| {
        let left_boundary = index == 0 || chars[index - 1].is_whitespace();
        let right_boundary = chars
            .get(index + 2)
            .is_none_or(|c| c.is_whitespace());
        left_boundary || right_boundary
    })
}

/// One named class and the predicate that decides whether an entry draws it.
type DeletionClass = (&'static str, fn(&str) -> bool);

/// The FIVE classes, named once so the per-alphabet floor, the per-class floor
/// and the counted floor all count the same thing.
///
/// **A SECOND axis standing beside `UNREADABLE_CLASSES`, not two more entries in
/// it.** The seven above are ways bash ASSEMBLES a word the guard cannot read;
/// the five below are ways bash DELETES something the guard counted.
const DELETION_CLASSES: &[DeletionClass] = &[
    ("separate-word redirection", draws_a_separate_word_redirection),
    ("attached redirection", draws_an_attached_redirection),
    (
        "multi-character or fd operator",
        draws_a_multi_character_or_fd_operator,
    ),
    ("continuation inside a word", draws_a_continuation_inside_a_word),
    (
        "continuation at a word boundary",
        draws_a_continuation_at_a_word_boundary,
    ),
];

/// Whether an entry draws ANY of the five.
fn carries_a_deletion_class(entry: &str) -> bool {
    DELETION_CLASSES
        .iter()
        .any(|(_, predicate)| predicate(entry))
}

#[test]
fn the_corpus_can_draw_every_one_of_the_five_deletion_classes() {
    // **The degenerate-proofing, asserted rather than described**, in the shape
    // `the_corpus_can_draw_every_one_of_the_seven_unreadable_classes` already
    // uses. If any pair below collapsed, the floors would be satisfiable by an
    // alphabet that cannot generate the cells this round is about — which is
    // exactly how the last two plan-check rounds each found a live cell.
    assert!(
        draws_a_separate_word_redirection("git >/dev/null push --force origin main"),
        "`git >/dev/null push …` IS a separate-word redirection"
    );
    assert!(
        !draws_an_attached_redirection("git >/dev/null push --force origin main"),
        "`git >/dev/null push …` must NOT satisfy the ATTACHED class: a corpus of \
         separate-word redirections cannot fail on `git push>/dev/null --force …`, which is \
         measured at exit 0 today"
    );
    assert!(
        draws_an_attached_redirection("git push>/dev/null --force origin main"),
        "`git push>/dev/null --force …` IS an attached redirection — the operator need not \
         be its own word, and a whitespace-delimited predicate walks straight around it"
    );

    assert!(
        draws_a_separate_word_redirection("git > /tmp/o push --force origin main"),
        "the operator as its OWN word is still a separate-word redirection"
    );
    assert!(
        !draws_a_multi_character_or_fd_operator("git > /tmp/o push --force origin main"),
        "a bare `>` must NOT satisfy the multi-character-or-fd class: a corpus of bare `>` \
         entries cannot fail on `&>`, `>|`, `<>`, `<<` or `<<-`, five of which are cells \
         measured at exit 0 today"
    );
    assert!(
        draws_a_multi_character_or_fd_operator("git 2>/dev/null push --force origin main"),
        "`2>` IS fd-carrying"
    );
    for spelling in [
        "git >>/tmp/x push --force origin main",
        "git <<<x push --force origin main",
        "git >|/tmp/o push --force origin main",
        "git <>/tmp/o push --force origin main",
        "git &>/tmp/o push --force origin main",
        "git &>>/tmp/o push --force origin main",
        "git <<-EOF push --force origin main",
    ] {
        assert!(
            draws_a_multi_character_or_fd_operator(spelling),
            "`{spelling}` carries a MULTI-CHARACTER operator and must draw class 3"
        );
        // **And none of them is ATTACHED.** This is the degenerate-proofing for a
        // real bug in the first draft of `draws_an_attached_redirection`, caught
        // by the exact per-class count below: reading the second `>` of
        // `>>/tmp/x` as an operator attached to a literal run `>` made six of the
        // eleven entries satisfy class 2 as well, over-counting it by 90 cases
        // and collapsing the class-1/class-2 split this axis turns on. Bash takes
        // the LONGEST operator match at the start of a redirection; everything
        // after it is the TARGET.
        assert!(
            !draws_an_attached_redirection(spelling),
            "`{spelling}` is a SEPARATE-WORD redirection, not an attached one. If this is \
             true the class-1/class-2 split has collapsed and a corpus of separate-word \
             redirections would satisfy the attached floor — which is a corpus that cannot \
             fail on `git push>/dev/null --force …`."
        );
    }

    assert!(
        draws_a_continuation_inside_a_word("git pu\\\nsh --force origin main"),
        "`pu\\`+NL+`sh` IS a continuation inside a word"
    );
    assert!(
        !draws_a_continuation_at_a_word_boundary("git pu\\\nsh --force origin main"),
        "`pu\\`+NL+`sh` must NOT satisfy the BOUNDARY class: a corpus of in-word \
         continuations cannot fail on `git \\`+NL+`push …`, nor on the second pre-existing \
         FALSE REFUSAL, whose whole mechanism is the flush the trailing whitespace causes"
    );
    assert!(
        draws_a_continuation_at_a_word_boundary("git \\\npush --force origin main"),
        "`git \\`+NL+`push …` IS a boundary continuation"
    );
    assert!(
        !draws_a_continuation_inside_a_word("git \\\npush --force origin main"),
        "`git \\`+NL+`push …` must NOT satisfy the IN-WORD class: the two are different \
         mechanisms and the split is what makes the pair non-degenerate"
    );

    // The QUOTING control. A quoted `>` is an ordinary character, and
    // `git log --grep='>'` is pinned PERMITTED in
    // `tests/envelope_argv_deletion.rs`. A predicate that counted it would make
    // every floor below satisfiable by a row the rule must never touch.
    for (class, predicate) in DELETION_CLASSES {
        assert!(
            !predicate("git log --grep='>'"),
            "`git log --grep='>'` must satisfy NO deletion class, and it satisfies \
             `{class}`. A quoted redirection character is an ordinary character."
        );
        assert!(
            !predicate("git commit -m \"a > b\""),
            "`git commit -m \"a > b\"` must satisfy NO deletion class, and it satisfies \
             `{class}`."
        );
    }

    // And the OVER-DELETION control DOES draw a class — the class must be
    // drawable in both directions, or the corpus cannot fail on a rule that
    // deletes too much.
    assert!(
        draws_an_attached_redirection("git x2>/tmp/o push --force origin main"),
        "`git x2>/tmp/o push …` draws the ATTACHED class. It is pinned PERMITTED in \
         `tests/envelope_argv_deletion.rs` because bash gives git `[x2] [push] [--force] \
         [origin] [main]` — an IO_NUMBER is a digits-only run since the START of the word, \
         so `x2` IS argv. This is the row that tells a deletion MODEL apart from a rule \
         that deletes any word part before a `>`."
    );
    assert!(
        !draws_an_attached_redirection("git 2>/dev/null push --force origin main"),
        "`2>` is an IO_NUMBER and NOT a literal run: it must not satisfy the attached class, \
         or the over-deletion control's whole distinction is lost"
    );
}

// ---------------------------------------------------------------------------
// 14a. The alphabets — spliced where the hole ACTUALLY is
// ---------------------------------------------------------------------------

/// The displacement alphabet, spliced **between the governed program and its
/// decision words**.
///
/// **Not a wrapper prefix and not a trailing operand.** Measured at this file's
/// base commit: `>/dev/null git push --force origin main`,
/// `2>/dev/null git …`, `env >/dev/null git …` and
/// `git push --force origin main >/dev/null` are ALL already refused at exit 2
/// `force_push_blocked` — `resolve_program` walks past a leading redirection as
/// it would any wrapper operand and finds `git` — and
/// `>out gh pr create --title x` is already correctly COUNTED. An alphabet
/// drawing those positions would give a property that is GREEN before the fix and
/// certifies nothing.
///
/// **Every entry here is VERDICT-PRESERVING after the fix**, which is why the
/// property below asserts refusal over refused bases and PERMISSION over
/// permitted ones. This is the opposite of `19-16`'s brace entries, which had to
/// be kept out of the invariance-preserving alphabets because they are STRICTER
/// than their bases after `19-17`.
///
/// **`&>>/tmp/o` and `<<-EOF` are minimum entries because they are the two
/// spellings `19-19`'s production uniquely adds**, and both are measured at this
/// file's base commit: `git &>>/tmp/o push --force origin main` and
/// `git <<-EOF push --force origin main` are at exit 0 with
/// `ARGV[git]: [push] [--force] [origin] [main]` under the shims, and
/// `git &>>/tmp/o status` and `git <<-EOF status` are at exit 0 with
/// `ARGV[git]: [status]` — live on the refused half and verdict-preserving on the
/// permitted half, so both arms can draw them.
///
/// **`{v}>/tmp/o` is DELIBERATELY NOT an entry here.** Every entry must be
/// verdict-PRESERVING, because the permitted arm asserts the spliced verdict
/// EQUALS the unspliced one. A `{name}` fd-allocation prefix is the one spelling
/// `19-19` does **not** model — it is marked unresolvable and refuses — so
/// `git {v}>/tmp/o status` would be STRICTER than its base and would break
/// invariance in exactly the direction `19-16` recorded for the
/// `GIT_CONFIG_COUNT=0` prefix, turning this property permanently red in a file
/// `19-19` may not edit. It is RECORDED instead, unasserted, in
/// `tests/envelope_argv_deletion.rs` section 9. This is the same call `19-16`
/// made when it gave `SEVERED_BRACE_PREFIXES` its own alphabet rather than
/// lowering a floor its class could not satisfy.
const DISPLACING_REDIRECTIONS: &[&str] = &[
    ">/dev/null",
    "2>/dev/null",
    "1>/dev/null",
    ">>/tmp/x",
    "> /tmp/o",
    "<<<x",
    ">|/tmp/o",
    "<>/tmp/o",
    "&>/tmp/o",
    "&>>/tmp/o",
    "<<-EOF",
];

/// Where a redirection entry is spliced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RedirectionSplice {
    /// Its own whitespace-delimited word — class 1.
    AsItsOwnWord,
    /// Appended to the decision word at the slot — class 2.
    AttachedToTheDecisionWord,
}

const REDIRECTION_SPLICES: &[RedirectionSplice] = &[
    RedirectionSplice::AsItsOwnWord,
    RedirectionSplice::AttachedToTheDecisionWord,
];

/// The backslash-newline splice points — the second alphabet of this axis.
///
/// Two points rather than two strings, because what distinguishes `T-19-98`'s two
/// classes is WHERE the pair sits, not what it spells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ContinuationSplice {
    /// Between two words — class 5, the mechanism of the second pre-existing
    /// FALSE REFUSAL.
    AtTheWordBoundary,
    /// Halfway through the decision word — class 4, `git pu\`+NL+`sh …`.
    InsideTheWord,
}

const CONTINUATION_SPLICES: &[ContinuationSplice] = &[
    ContinuationSplice::AtTheWordBoundary,
    ContinuationSplice::InsideTheWord,
];

/// Bases whose UNWRAPPED form is REFUSED — measured in the property's own floor
/// before the splice loop runs.
const DELETION_REFUSED_BASES: &[&str] = &[
    "git push --force origin main",
    "git push -f origin main",
    // No second carrier at all: for these the guard is the only control.
    "git stash",
    "git update-ref -d refs/heads/main",
    // Disarms layer 3, which is the loss of the second carrier.
    "git config core.hooksPath /tmp/x",
];

/// Bases whose UNWRAPPED form is PERMITTED. **Without this half the property
/// cannot fail on an implementation that refuses everything carrying a `>`**, and
/// reading git state is the first thing a driven run does.
const DELETION_PERMITTED_BASES: &[&str] = &[
    "git status",
    "git log --oneline",
    "git diff",
    "gh pr list --limit 5",
];

/// The splice slots of a base: strictly BETWEEN the governed program (word 0) and
/// its decision words, capped at two so the generated corpus stays bounded.
fn deletion_slots(base: &str) -> std::ops::Range<usize> {
    1..base.split_whitespace().count().min(3)
}

/// Splice one redirection entry into `base` at `slot`.
fn redirection_case(base: &str, slot: usize, entry: &str, splice: RedirectionSplice) -> String {
    let mut words: Vec<String> = base.split_whitespace().map(str::to_string).collect();
    match splice {
        RedirectionSplice::AsItsOwnWord => words.insert(slot, entry.to_string()),
        RedirectionSplice::AttachedToTheDecisionWord => {
            words[slot] = format!("{}{entry}", words[slot]);
        }
    }
    words.join(" ")
}

/// Whether an entry may be ATTACHED to a decision word.
///
/// **An entry beginning with an fd digit may not.** Appending `2>/dev/null` to
/// `push` spells `push2>/dev/null`, which bash reads as the argv word `push2`
/// plus a redirection — a DIFFERENT decision word, not a displaced one, and a
/// verdict `19-19` could not restore. That is the over-deletion control's rule
/// running in the other direction, and it is why the class-2 predicate excludes
/// the IO_NUMBER run.
fn is_attachable(entry: &str) -> bool {
    !entry.starts_with(|c: char| c.is_ascii_digit())
}

/// Splice a backslash-newline into `base` at `slot`.
fn continuation_case(base: &str, slot: usize, splice: ContinuationSplice) -> String {
    let mut words: Vec<String> = base.split_whitespace().map(str::to_string).collect();
    match splice {
        ContinuationSplice::AtTheWordBoundary => words.insert(slot, "\\\n".to_string()),
        ContinuationSplice::InsideTheWord => {
            let word: Vec<char> = words[slot].chars().collect();
            let half = (word.len() / 2).max(1);
            let head: String = word[..half].iter().collect();
            let tail: String = word[half..].iter().collect();
            words[slot] = format!("{head}\\\n{tail}");
        }
    }
    words.join(" ")
}

/// Every case this axis generates, as `(base, slot, label, command)`.
///
/// One function so the counting floor below and the guard-driven property count
/// exactly the same thing — a second enumeration would be a second thing to keep
/// in step.
fn deletion_cases(bases: &[&'static str]) -> Vec<(&'static str, usize, String, String)> {
    let mut cases = Vec::new();
    for base in bases.iter().copied() {
        for slot in deletion_slots(base) {
            for entry in DISPLACING_REDIRECTIONS {
                for splice in REDIRECTION_SPLICES {
                    if *splice == RedirectionSplice::AttachedToTheDecisionWord
                        && !is_attachable(entry)
                    {
                        continue;
                    }
                    cases.push((
                        base,
                        slot,
                        format!("{splice:?}({entry})"),
                        redirection_case(base, slot, entry, *splice),
                    ));
                }
            }
            for splice in CONTINUATION_SPLICES {
                cases.push((
                    base,
                    slot,
                    format!("{splice:?}"),
                    continuation_case(base, slot, *splice),
                ));
            }
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// 14b. The floors — per ALPHABET, per CLASS, and COUNTED over generated cases
// ---------------------------------------------------------------------------

/// The arithmetic, STATED rather than guessed, because audit 5 found `19-16` set
/// a floor of 50 against a maximum of 40 by construction.
///
/// `DISPLACING_REDIRECTIONS` has 11 entries. Nine of them are attachable (all but
/// `2>/dev/null` and `1>/dev/null`, which would fuse an fd digit onto the decision
/// word). `CONTINUATION_SPLICES` has 2. So each SLOT emits
/// `11 + 9 + 2 = 22` cases.
///
/// Slots are `1..min(words, 3)`, so a two-word base has one and every longer base
/// has two:
///
/// * refused  — 2 + 2 + 1 + 2 + 2 = **9 slots** -> 9 x 22 = **198** cases
/// * permitted — 1 + 2 + 1 + 2 = **6 slots** -> 6 x 22 = **132** cases
/// * total = **330** cases over **15** slots
///
/// The floors are EXACT equalities, so losing one case turns them red. Nothing in
/// `src/` can move them: they are a pure function of the alphabets in this file.
const DELETION_CASES: usize = 330;
const DELETION_REFUSED_CASES: usize = 198;
const DELETION_PERMITTED_CASES: usize = 132;
const DELETION_SLOTS: usize = 15;
const MIN_DELETION_CLASSES: usize = 5;
const MIN_DISPLACING_REDIRECTIONS: usize = 11;

/// The per-class counts over all 330 generated commands, derived the same way:
///
/// * class 1 (separate-word) — the 11 own-word cases at each of 15 slots = **165**
/// * class 2 (attached) — the 9 attached cases at each of 15 slots = **135**
/// * class 3 (multi-char or fd) — own-word: every entry but `>/dev/null` and
///   `> /tmp/o` (9) = 135; attached: every ATTACHABLE entry but those same two
///   (7) = 105; total **240**
/// * class 4 (continuation in a word) — one per slot = **15**
/// * class 5 (continuation at a boundary) — one per slot = **15**
const DELETION_CLASS_COUNTS: &[(&str, usize)] = &[
    ("separate-word redirection", 165),
    ("attached redirection", 135),
    ("multi-character or fd operator", 240),
    ("continuation inside a word", 15),
    ("continuation at a word boundary", 15),
];

#[test]
fn every_alphabet_this_round_widens_can_draw_a_word_the_shell_deletes() {
    // **The direct mechanical inverse of audit 5's `T-19-99`.** The auditor
    // established the defect by grepping for a guard-driven row carrying a
    // redirection and finding NONE anywhere in the repository; this asserts the
    // repaired fact, so narrowing the alphabet back turns this red instead of
    // quietly restoring a corpus that cannot fail on its own class.
    //
    // The predicate is evaluated on a REPRESENTATIVE SPLICED COMMAND rather than
    // on the bare entry, because these entries are splice FRAGMENTS: `>/dev/null`
    // standing alone has no following word and no decision word to displace.
    assert!(
        DELETION_CLASSES.len() >= MIN_DELETION_CLASSES,
        "the deletion axis must name at least {MIN_DELETION_CLASSES} classes"
    );
    assert!(
        DISPLACING_REDIRECTIONS.len() >= MIN_DISPLACING_REDIRECTIONS,
        "`DISPLACING_REDIRECTIONS` must carry at least {MIN_DISPLACING_REDIRECTIONS} entries. \
         The correct response to a red here is to RESTORE entries, never to lower this floor."
    );

    for entry in DISPLACING_REDIRECTIONS {
        let spliced = redirection_case(
            "git push --force origin main",
            1,
            entry,
            RedirectionSplice::AsItsOwnWord,
        );
        assert!(
            carries_a_deletion_class(&spliced),
            "`DISPLACING_REDIRECTIONS` entry `{entry}` draws NO deletion class when spliced \
             between the program and its decision words (`{spliced}`).\n\n\
             An alphabet entry that cannot DRAW a class is an entry whose property cannot \
             FAIL on one, and every case generated from it certifies a claim about a class \
             it could never have exercised. It is `T-19-76`'s failure mode for the FIFTH \
             consecutive round, after `T-19-83`, `T-19-89` and `T-19-95`.\n\n\
             The correct response is to RESTORE the entry, never to delete this floor."
        );
    }

    // The two operators `19-19`'s production uniquely adds must be DRAWN, not
    // merely named in class 3's doc. A class list naming a spelling no entry
    // draws is a floor nothing satisfies.
    for required in ["&>>/tmp/o", "<<-EOF"] {
        assert!(
            DISPLACING_REDIRECTIONS.contains(&required),
            "`{required}` must be an entry: it is one of the two spellings `19-19`'s \
             production uniquely adds, and a corpus without it cannot fail on the operator \
             the fix adds beyond what any other row exercises"
        );
    }

    // And `{v}>` must NOT be one, for the reason the alphabet's doc gives.
    assert!(
        !DISPLACING_REDIRECTIONS
            .iter()
            .any(|entry| entry.contains("{v}")),
        "`{{v}}>/tmp/o` must NOT be an entry. Every entry here is asserted VERDICT-PRESERVING \
         by the permitted arm below, and `19-19` deliberately does not model a `{{name}}` fd \
         prefix — so it would be STRICTER than its base and would turn this property \
         permanently red in a file `19-19` may not edit. It is RECORDED unasserted in \
         `tests/envelope_argv_deletion.rs` section 9 instead."
    );

    for splice in CONTINUATION_SPLICES {
        let spliced = continuation_case("git push --force origin main", 1, *splice);
        assert!(
            carries_a_deletion_class(&spliced),
            "`CONTINUATION_SPLICES` point {splice:?} draws NO deletion class (`{spliced:?}`)"
        );
    }
}

#[test]
fn the_generated_corpus_really_produces_each_deletion_class_in_quantity() {
    // **An alphabet floor is not a generation floor.** An entry can sit in an
    // alphabet and be drawn by nothing, or be drawn once out of hundreds of
    // cases — which is a corpus that can technically fail on the class and
    // practically never does. This counts what the generator ACTUALLY emits.
    //
    // **Green today and after**: it drives no guard call at all, and no
    // production change can move it. The counts are a pure function of the
    // alphabets above, which is exactly what makes the exact equalities safe.
    let refused = deletion_cases(DELETION_REFUSED_BASES);
    let permitted = deletion_cases(DELETION_PERMITTED_BASES);

    assert_eq!(
        refused.len(),
        DELETION_REFUSED_CASES,
        "the refused arm's generation count must equal the stated arithmetic exactly, so \
         losing one case turns this red. `19-16` set a floor of 50 against a maximum of 40 \
         by construction and it was invisible until the rule landed."
    );
    assert_eq!(
        permitted.len(),
        DELETION_PERMITTED_CASES,
        "the permitted arm's generation count must equal the stated arithmetic exactly"
    );

    let all: Vec<&(&str, usize, String, String)> = refused.iter().chain(permitted.iter()).collect();
    assert_eq!(all.len(), DELETION_CASES);

    let slots: BTreeSet<(&str, usize)> = all.iter().map(|(base, slot, _, _)| (*base, *slot)).collect();
    assert_eq!(
        slots.len(),
        DELETION_SLOTS,
        "every splice slot must have been generated. Seen: {slots:?}"
    );

    let mut per_class: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, _, command) in &all {
        for (class, predicate) in DELETION_CLASSES {
            if predicate(command) {
                *per_class.entry(class).or_default() += 1;
            }
        }
    }

    for (class, expected) in DELETION_CLASS_COUNTS {
        let got = per_class.get(class).copied().unwrap_or(0);
        assert_eq!(
            got, *expected,
            "the generator emitted {got} cases of the deletion class `{class}`, and the \
             stated arithmetic derives {expected}.\n\n\
             This is `T-19-99` counted rather than read. The correct response to a shortfall \
             is to RESTORE entries, never to lower the number. Full counts: {per_class:?}"
        );
    }

    // Recorded so the SUMMARY carries measured counts rather than described ones.
    println!(
        "deletion axis: {} cases ({} refused, {} permitted) over {} slots from {} \
         redirection entries and {} continuation splice points.\nper class: {per_class:?}",
        all.len(),
        refused.len(),
        permitted.len(),
        slots.len(),
        DISPLACING_REDIRECTIONS.len(),
        CONTINUATION_SPLICES.len(),
    );
}

// ---------------------------------------------------------------------------
// 14c. The generative property — RED against the pre-fix tree
// ---------------------------------------------------------------------------

#[test]
fn a_word_the_shell_deletes_between_the_program_and_its_decision_words_is_not_a_decision_word() {
    // **Modelled directly on
    // `a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot`**,
    // with the PERMITTED arm running FIRST so that today's failure output is
    // itself evidence the permitted half passed. A property whose permitted arm
    // is unobservable until the fix lands cannot be said to fail in both
    // directions.
    //
    // Measured against the built binary while this plan was written: of the 198
    // refused-arm cases, **124 are at exit 0 today** and 74 are already refused
    // for a position the splice did not reach. All 132 permitted-arm cases are at
    // exit 0 today, with no anomalies.
    //
    // **The refusal arm asserts the exit code and an EMPTY WALK, not a reason
    // identifier.** After `19-19` the surviving argv is the dangerous one, and
    // which D-24 identifier it lands on legitimately differs by base and by
    // splice — `git push&>/tmp/o --force origin main`, for example, is refused
    // today under `push_outside_namespace` and must be refused after under
    // `force_push_blocked`. Pinning the identifier would be pinning a verdict
    // this plan cannot derive.

    // --- floor 0: the POSITIVE control for the walk -----------------------
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "a PERMITTED `gh pr create` writes exactly one ledger line, and the walk must be \
         able to find it. If this is 0 the walk is blind and every empty-walk assertion \
         below is vacuous. Files: {:?}",
        files_under(control.path())
    );

    // --- floor 1: every base answers what the arm it is in claims ---------
    let mut base_verdicts: BTreeMap<&str, Verdict> = BTreeMap::new();
    for base in DELETION_PERMITTED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 0,
            "the UNWRAPPED base `{base}` must be PERMITTED, or every case built on it is \
             green or red for the base's own reason. Got reason id: {}",
            got.reason_id
        );
        base_verdicts.insert(base, got);
    }
    for base in DELETION_REFUSED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 2,
            "the UNWRAPPED base `{base}` must be REFUSED, or the splice cannot be what makes \
             the difference. Got reason id: {}",
            got.reason_id
        );
        assert!(
            REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
            "and refused under a D-24 identifier. Got: {}",
            got.reason_id
        );
    }

    // --- the PERMITTED arm: a deletion entry is VERDICT-PRESERVING --------
    //
    // Runs first, and is GREEN today. Without it, an implementation that simply
    // refused every governed simple command containing a token the shell deletes
    // would satisfy the whole refusal arm below — while refusing `git log > out`,
    // `git status > /tmp/s.txt` and `gh pr create --title x > /tmp/o`, whose
    // measured cost `tests/envelope_argv_deletion.rs` pins row by row.
    let mut permitted_cases = 0usize;
    for (base, slot, label, command) in deletion_cases(DELETION_PERMITTED_BASES) {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), &command);
        let expected = base_verdicts.get(base).expect("the base was measured");
        assert_eq!(
            got,
            *expected,
            "\n\nA DELETION ENTRY CHANGED A PERMITTED VERDICT.\n\
             \n  command : {command:?}\
             \n  base    : {base}\
             \n  slot    : {slot}\
             \n  splice  : {label}\
             \n  got     : exit {} reason {}\n\
             \nA redirection and a line continuation are VERDICT-PRESERVING: after the fix \
             the surviving argv is exactly the base's argv, so the classifier must reach the \
             base's verdict. Confirmed under bash shims — `git >/dev/null status` prints \
             `ARGV[git]: [status]` and `git status<<-EOF` prints `ARGV[git]: [status]`.\n\
             \n**A red here is the fix having become a blanket refusal of anything carrying \
             a `>`**, which would refuse the ordinary-redirection corpus pinned in \
             `tests/envelope_argv_deletion.rs` and turn a correct COUNT into a false \
             positive — the trade `T-19-93`'s bar forbids. It is NOT a reason to narrow \
             this alphabet.",
            got.code,
            got.reason_id,
        );
        permitted_cases += 1;
    }

    // --- the REFUSED arm: RED against the pre-fix tree --------------------
    let mut refused_cases = 0usize;
    let mut slots_seen: BTreeSet<(&str, usize)> = BTreeSet::new();
    let mut classes_seen: BTreeSet<&str> = BTreeSet::new();

    for (base, slot, label, command) in deletion_cases(DELETION_REFUSED_BASES) {
        slots_seen.insert((base, slot));
        for (class, predicate) in DELETION_CLASSES {
            if predicate(&command) {
                classes_seen.insert(class);
            }
        }

        // A fresh root per case, because a permitted forge command writes a
        // ledger line and a shared root would exhaust the cap and turn later
        // cases red for a reason that has nothing to do with the splice.
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), &command);

        assert_eq!(
            got.code,
            2,
            "\n\nA WORD THE SHELL DELETES DISPLACED THE DECISION WORD AND THE COMMAND WAS \
             PERMITTED.\n\
             \n  command : {command:?}\
             \n  base    : {base}\
             \n  slot    : {slot}\
             \n  splice  : {label}\
             \n  got     : exit {} reason {}\
             \n  seed    : {SEED:#x}\n\
             \nA redirection is a word the guard reads and `execve` NEVER SEES: bash removes \
             the operator and its target before the program runs, displacing every decision \
             word one slot right. A backslash-newline is two characters bash DELETES before \
             the word is assembled and `tokenize` keeps. Either way the argv the classifier \
             answers about is not the argv that runs.\n\
             \n`SEPARATORS` contains neither `>` nor `<` — correctly, because a redirection \
             does not start a new command — and `scan_leading` skips only tokens beginning \
             with `-`, so `>/dev/null` becomes the git VERB and `classify_git` finds it in \
             no denylist arm.\n\
             \n**The correct response is to MODEL THE DELETION in `src/envelope/policy.rs` \
             so the argv the classifier reads is the argv that runs.** It is NOT `>` added \
             to `SEPARATORS` — the doc there records why, and \
             `a_redirection_operator_is_not_a_separator_and_this_round_must_not_make_it_one` \
             pins it. It is NOT a blanket refusal of anything carrying a `>` — the permitted \
             arm above and the ordinary-redirection corpus in \
             `tests/envelope_argv_deletion.rs` measure that option's cost. And it is NOT a \
             narrowing of `DISPLACING_REDIRECTIONS`, `CONTINUATION_SPLICES` or the five \
             `DELETION_CLASSES`.",
            got.code,
            got.reason_id,
        );

        let written = ledger_lines_under(envelope.path());
        assert!(
            written.is_empty(),
            "`{command:?}` was refused, but a pull-request ledger line was written somewhere \
             under the envelope root — cap budget consumed for a command that never opens a \
             pull request. Found: {written:?} Files: {:?}",
            files_under(envelope.path())
        );

        refused_cases += 1;
    }

    // --- the floors, reachable only once the arm above is green -----------
    assert_eq!(
        permitted_cases, DELETION_PERMITTED_CASES,
        "the permitted arm must run every generated case"
    );
    assert_eq!(
        refused_cases, DELETION_REFUSED_CASES,
        "the refused arm must run every generated case"
    );
    assert_eq!(
        slots_seen.len(),
        DELETION_REFUSED_BASES
            .iter()
            .map(|base| deletion_slots(base).len())
            .sum::<usize>(),
        "every splice slot of every refused base must have been generated. Seen: {slots_seen:?}"
    );
    assert_eq!(
        classes_seen.len(),
        DELETION_CLASSES.len(),
        "the refused arm must draw ALL FIVE deletion classes — a property that could only \
         draw some would certify the fix for part of `T-19-97`/`T-19-98` while staying \
         silent on the rest. Seen: {classes_seen:?}"
    );

    println!(
        "deletion axis drove {permitted_cases} permitted and {refused_cases} refused cases \
         over {} slots, drawing {} classes.",
        slots_seen.len(),
        classes_seen.len()
    );
}

// ===========================================================================
// 15. THE THIRD AXIS — the CALLEE's grammar, which is not the shell's
//
// **Why a THIRD named axis rather than more entries in an existing one.** Audit 6
// enumerated bash's transformations between the command string and `execve` one
// at a time — quote removal, every expansion, redirection, line continuation,
// assignment-prefix removal, control-operator splitting, here-document
// delimiters, pipeline and group nesting — and found every one modelled or
// failing closed, with **no reordering step in a simple command** for a third
// rule to miss. The command-line-to-argv boundary is CLOSED.
//
// So the gap moved AXIS rather than one cell over. `UNREADABLE_CLASSES` (section
// 13) names seven ways bash ASSEMBLES a word the guard cannot read.
// `DELETION_CLASSES` (section 14) names five ways bash DELETES something the
// guard counted. **Both are axes of the SHELL's grammar.** This axis is about
// something the shell has no opinion on at all:
//
//   HAVING RECONSTRUCTED ARGV CORRECTLY, WHOSE GRAMMAR DECIDES WHICH ARRIVING
//   WORD IS THE VERB?
//
// `scan_leading` answers that by asking `GIT_GLOBAL_VALUE_OPTS` — a
// hand-maintained, unpinned enumeration of GIT's own global-option grammar which
// **fails OPEN** when it is silent (`policy.rs:488`, `(None, 1)`). Audit 6
// verified mechanically that nothing anywhere in `tests/` modelled this:
// `grep -rn "attr-source\|shallow-file\|GIT_GLOBAL_VALUE_OPTS" tests/` returned
// nothing at all, and no alphabet carried a leading git option that consumes a
// separate word. That is `T-19-101` — `T-19-76`'s failure mode for the SIXTH
// consecutive round.
//
// **Smuggling a leading git option into either shell axis would model the
// callee's grammar as if it were the shell's and lose exactly the distinction
// this round is about.** `UNREADABLE_CLASSES` and `DELETION_CLASSES`, their
// predicates, their degenerate-proofing blocks and every one of their floors are
// BYTE-IDENTICAL across this change.
//
// The row-by-row evidence lives in `tests/envelope_callee_grammar.rs`, which is
// this round's sixth evidence file. This section is the GENERATIVE half.
// ---------------------------------------------------------------------------

/// Options the two-sided real-git probe classified as CONSUMING A SEPARATE WORD.
///
/// **Measured, not copied.** For each option, `git <opt> version` versus
/// `git <opt> XVALUE version` on `git version 2.43.0`: the 1-word form prints
/// git's usage and the 2-word form prints `git version 2.43.0`, so the option
/// swallowed the following word. `-C` and `--config-env` fail for their own
/// reasons in both forms and are classified by the variant probe the plan names —
/// `git -C version` answers `cannot change to 'version'` and
/// `git --config-env version` answers `invalid config format: version`, each of
/// which is ITSELF the proof that the word was consumed.
///
/// **This table is the test file's own model of git's grammar, and it is what
/// makes the classes below descriptions of the GRAMMAR rather than of the
/// alphabet.** `--attr-source` and `--shallow-file` are here and are ABSENT from
/// `GIT_GLOBAL_VALUE_OPTS` in `policy.rs`, which is `T-19-100`.
const PROBED_VALUE_TAKING: &[&str] = &[
    "-c",
    "-C",
    "--config-env",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--attr-source",
    "--shallow-file",
];

/// Options the same probe classified as NOT consuming a following word.
///
/// **The TERMINATING family is folded in here deliberately, and the doc says so
/// rather than leaving it to be inferred.** `--exec-path`, `--html-path`,
/// `--man-path`, `--info-path` and `--version` never print a version at all — for
/// them the probe is that the 1-word and 2-word forms produce IDENTICAL first
/// lines, which is the proof that no following word can reach a verb slot. They
/// differ from the booleans in what git does NEXT, not in the one bit this axis
/// is about: does the option consume the following word. It does not.
///
/// `--help` and `-h` satisfy no probe of this shape — `git --help XVALUE version`
/// answers `No manual entry for gitXVALUE` — and are recorded UNPROBED rather
/// than guessed into either list.
const PROBED_SELF_CONTAINED: &[&str] = &[
    "--no-pager",
    "-p",
    "--paginate",
    "-P",
    "--bare",
    "--no-replace-objects",
    "--literal-pathspecs",
    "--glob-pathspecs",
    "--noglob-pathspecs",
    "--icase-pathspecs",
    "--no-optional-locks",
    "--exec-path",
    "--html-path",
    "--man-path",
    "--info-path",
    "--version",
];

/// Whether a token carries its value ATTACHED, which needs no knowledge of git.
///
/// A `--`-prefixed token containing `=`, or a `-c`-prefixed short token with a
/// non-empty remainder. Git's own grammar makes an attached value self-contained
/// whatever the option is, which is why `git --attr-source=HEAD push --force
/// origin main` is ALREADY correctly refused at this file's base commit.
fn carries_an_attached_value(token: &str) -> bool {
    if token.starts_with("--") {
        return token.contains('=');
    }
    token.starts_with("-c") && token.len() > 2
}

/// The leading option tokens of a governed simple command, walked exactly as
/// `scan_leading` walks them.
///
/// Word 0 is the program and the scan begins at word 1. It stops at the first
/// word that is not `-`-initial (that word is the VERB), and it stops at a bare
/// `-` and at `--` before any option check runs. An option this file's probe
/// classified as value-taking advances the index by TWO; everything else by one.
///
/// Each entry is `(token, a following word exists)`.
fn callee_leading_tokens(command: &str) -> Vec<(&str, bool)> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut out = Vec::new();
    let mut index = 1;
    while index < words.len() {
        let token = words[index];
        if !token.starts_with('-') {
            break;
        }
        if token == "-" || token == "--" {
            out.push((token, false));
            break;
        }
        out.push((token, index + 1 < words.len()));
        index += if PROBED_VALUE_TAKING.contains(&token) {
            2
        } else {
            1
        };
    }
    out
}

/// **Class 1** — a leading option that consumes a SEPARATE word: the word after
/// it is the option's VALUE and not the verb.
///
/// `git --attr-source HEAD push …` and `git -C /tmp status` satisfy this. **This
/// is the class `T-19-100` lives in**, and the whole of the grammar question.
fn draws_a_leading_option_that_consumes_a_separate_word(command: &str) -> bool {
    callee_leading_tokens(command)
        .iter()
        .any(|(token, has_next)| *has_next && PROBED_VALUE_TAKING.contains(token))
}

/// **Class 2** — a leading option that consumes NO word: the word after it IS the
/// verb.
///
/// `git --no-pager status` satisfies this. **Class 1 cannot satisfy this and
/// class 2 cannot satisfy class 1**, which is the split that makes the pair
/// non-degenerate — and it is the entire grammar question, because a guard that
/// could not tell the two apart would either miss `T-19-100` or refuse every
/// leading option.
fn draws_a_leading_option_that_consumes_no_word(command: &str) -> bool {
    callee_leading_tokens(command)
        .iter()
        .any(|(token, _)| PROBED_SELF_CONTAINED.contains(token))
}

/// **Class 3** — a leading option carrying an ATTACHED value.
///
/// **This class needs no knowledge of git at all**, and that is why it is named
/// separately: git's own grammar makes an attached value self-contained whatever
/// the option is. `git --attr-source=HEAD push --force origin main` is already
/// correctly refused today for exactly this reason, which is why an alphabet
/// drawn in the attached position would be green before the fix and would certify
/// nothing. The structural rule is applied FIRST by `19-21` so the constants have
/// less to know.
fn draws_a_leading_option_with_an_attached_value(command: &str) -> bool {
    callee_leading_tokens(command)
        .iter()
        .any(|(token, _)| carries_an_attached_value(token))
}

/// **Class 4** — a leading option the installed git does NOT accept.
///
/// The unknown class: a `-`-prefixed token that is neither `-` nor `--`, carries
/// no attached value, and appears in NEITHER probed list. Drawn from
/// [`GIT_GLOBAL_UNKNOWN_OPTIONS`] and never from [`GIT_GLOBAL_OPTIONS`], for the
/// verdict-preservation reason that alphabet's doc gives.
fn draws_a_leading_option_the_installed_git_rejects(command: &str) -> bool {
    callee_leading_tokens(command).iter().any(|(token, _)| {
        *token != "-"
            && *token != "--"
            && !carries_an_attached_value(token)
            && !PROBED_VALUE_TAKING.contains(token)
            && !PROBED_SELF_CONTAINED.contains(token)
    })
}

/// **Class 5** — the end-of-options marker `--`, or a bare `-`: the two tokens
/// `scan_leading` breaks on before any option check runs.
///
/// **The CLASS covers both; the ALPHABET carries only `--`, and the distinction
/// is load-bearing rather than tidy.** See [`GIT_GLOBAL_OPTIONS`] for why a bare
/// `-` must not be an entry. This predicate deliberately still RECOGNISES a bare
/// `-`: the class is about the tokens the scan breaks on, and narrowing the
/// predicate to hide the alphabet's exclusion would make the class a description
/// of the alphabet instead of a description of the grammar.
fn draws_the_end_of_options_marker_or_a_bare_dash(command: &str) -> bool {
    callee_leading_tokens(command)
        .iter()
        .any(|(token, _)| *token == "--" || *token == "-")
}

/// One named class and the predicate that decides whether a spliced command draws
/// it.
type CalleeGrammarClass = (&'static str, fn(&str) -> bool);

/// The FIVE classes of the callee axis, named once so the per-alphabet floor, the
/// per-class floor and the counted floor all count the same thing.
///
/// **A THIRD axis standing beside `UNREADABLE_CLASSES` and `DELETION_CLASSES`,
/// not more entries in either.** The seven above are ways bash ASSEMBLES a word;
/// the five above that are ways bash DELETES one; **the five below are ways GIT's
/// own option grammar decides which arriving word is the verb.**
const CALLEE_GRAMMAR_CLASSES: &[CalleeGrammarClass] = &[
    (
        "a leading option that consumes a separate word",
        draws_a_leading_option_that_consumes_a_separate_word,
    ),
    (
        "a leading option that consumes no word",
        draws_a_leading_option_that_consumes_no_word,
    ),
    (
        "a leading option carrying an attached value",
        draws_a_leading_option_with_an_attached_value,
    ),
    (
        "a leading option the installed git does not accept",
        draws_a_leading_option_the_installed_git_rejects,
    ),
    (
        "the end-of-options marker or a bare dash",
        draws_the_end_of_options_marker_or_a_bare_dash,
    ),
];

/// Whether a spliced command draws ANY of the five.
fn carries_a_callee_grammar_class(command: &str) -> bool {
    CALLEE_GRAMMAR_CLASSES
        .iter()
        .any(|(_, predicate)| predicate(command))
}

#[test]
fn the_corpus_can_draw_every_one_of_the_five_callee_grammar_classes() {
    // **The degenerate-proofing, asserted rather than described**, in the shape
    // `the_corpus_can_draw_every_one_of_the_seven_unreadable_classes` and
    // `the_corpus_can_draw_every_one_of_the_five_deletion_classes` already use.
    // If any pair below collapsed, the floors would be satisfiable by an alphabet
    // that cannot generate the cells this round is about — which is exactly how
    // the last three plan-check rounds each found a live cell.
    //
    // **GREEN today and after.** It drives no guard call and no production change
    // can move it.

    // -- class 1 versus class 2. This pair IS the grammar question.
    assert!(
        draws_a_leading_option_that_consumes_a_separate_word("git --attr-source HEAD push"),
        "`git --attr-source HEAD push` IS a leading option consuming a separate word — the \
         real-git probe classified it so, two-sided, on git 2.43.0"
    );
    assert!(
        !draws_a_leading_option_that_consumes_no_word("git --attr-source HEAD push"),
        "`git --attr-source HEAD push` must NOT satisfy the CONSUMES-NO-WORD class: a corpus \
         of self-contained options cannot fail on `T-19-100`, whose entire mechanism is the \
         word the guard did not know was a VALUE"
    );
    assert!(
        draws_a_leading_option_that_consumes_no_word("git --no-pager push"),
        "`git --no-pager push` IS a self-contained leading option"
    );
    assert!(
        !draws_a_leading_option_that_consumes_a_separate_word("git --no-pager push"),
        "`git --no-pager push` must NOT satisfy the CONSUMES-A-WORD class. **This is the \
         split that tells a grammar MODEL apart from a blanket refusal of anything beginning \
         with `-`**, and a corpus that collapsed it could not fail on a rule that refused \
         `git --no-pager status` — the row pinned PERMITTED in \
         `tests/envelope_callee_grammar.rs` as this axis's `ls {{git,svn}}-repo`"
    );

    // -- class 3 satisfies NEITHER 1 nor 2, because it needs no git knowledge.
    assert!(
        draws_a_leading_option_with_an_attached_value("git --attr-source=HEAD push"),
        "`--attr-source=HEAD` IS an attached value"
    );
    assert!(
        !draws_a_leading_option_that_consumes_a_separate_word("git --attr-source=HEAD push"),
        "the ATTACHED spelling must not satisfy class 1: it is already correctly refused at \
         this file's base commit, so a corpus that conflated the two would be green before \
         the fix and would certify nothing"
    );
    assert!(
        !draws_a_leading_option_that_consumes_no_word("git --attr-source=HEAD push"),
        "nor class 2 — an attached value is a THIRD structural shape, not a boolean"
    );
    assert!(
        draws_a_leading_option_with_an_attached_value("git -cuser.name=x status"),
        "git's short-option parser accepts `-ckey=value` with no space, and the guard's own \
         `leading_git_option` already reads it — so the attached class must draw it too"
    );

    // -- class 4, the unknown class.
    assert!(
        draws_a_leading_option_the_installed_git_rejects("git --bogus-opt push"),
        "`--bogus-opt` is in neither probed list and is therefore UNKNOWN"
    );
    for known in ["git --attr-source HEAD push", "git --no-pager push"] {
        assert!(
            !draws_a_leading_option_the_installed_git_rejects(known),
            "`{known}` carries an option the real-git probe CLASSIFIED, so it must not draw \
             the unknown class. If it does, the unknown alphabet and the known alphabet are \
             not distinguishable and the invariance arm below would draw entries whose \
             verdict the fix CHANGES."
        );
    }

    // -- class 5, and the bare-dash distinction that is load-bearing.
    assert!(
        draws_the_end_of_options_marker_or_a_bare_dash("git -- push"),
        "`--` IS the end-of-options marker"
    );
    assert!(
        draws_the_end_of_options_marker_or_a_bare_dash("git - push --force origin main"),
        "**a bare `-` IS in the CLASS, even though it is deliberately NOT in the alphabet.** \
         The class describes the tokens `scan_leading` breaks on; narrowing this predicate to \
         match the alphabet would make the class a description of the alphabet rather than of \
         the grammar, which is the whole distinction this axis exists to hold."
    );
    assert!(
        !draws_a_leading_option_the_installed_git_rejects("git -- push"),
        "`--` must not ALSO draw the unknown class, or class 4's floor would be satisfiable \
         by an end-of-options marker"
    );
    assert!(
        !draws_a_leading_option_the_installed_git_rejects("git - push --force origin main"),
        "nor must a bare `-`"
    );

    // -- THE QUOTING-AND-POSITION CONTROL. An option-shaped word that is neither
    //    leading nor in the scan's view must satisfy NO class, or the floors
    //    become satisfiable by a row the rule must never touch.
    for (class, predicate) in CALLEE_GRAMMAR_CLASSES {
        for control in [
            "git commit -m \"--attr-source x\"",
            "git push --attr-source HEAD --force origin main",
            "git log --grep='--no-pager'",
        ] {
            assert!(
                !predicate(control),
                "`{control}` must satisfy NO callee-grammar class, and it satisfies \
                 `{class}`.\n\n\
                 The first is an option-shaped word inside a quoted OPERAND. The second sits \
                 AFTER the verb, which `scan_leading` does not read at all. The third is \
                 quoted. A predicate that counted any of them would make every floor below \
                 satisfiable by a row the rule must never touch, and would put the fix's \
                 blast radius outside the region it is allowed to decide in."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 15a. The alphabets — spliced where `scan_leading` ACTUALLY reads
// ---------------------------------------------------------------------------

/// One leading-option alphabet entry: its spelling, its class, and how many words
/// it occupies.
///
/// One alphabet feeds every floor, in the shape `CONTINUATION_SPLICES` uses, so
/// the per-alphabet floor and the per-class floor cannot drift apart.
#[derive(Debug, Clone, Copy)]
struct GitGlobalOption {
    spelling: &'static str,
    class: &'static str,
    words: usize,
}

/// The leading-option alphabet, tagged by class, spliced **between the governed
/// program and its decision words**.
///
/// **That position is the only one `scan_leading` reads**, and both alternatives
/// would certify nothing. An ATTACHED-value entry (`--attr-source=HEAD`) is
/// already refused at this file's base commit — it is a CONTROL, which is why
/// class 3 is present in the alphabet but is not what makes the property red. An
/// option AFTER the verb (`git push --attr-source HEAD --force origin main`) is
/// not read by this scan at all.
///
/// **Every entry here is one the two-sided real-git probe CLASSIFIED**, recorded
/// in `tests/envelope_callee_grammar.rs`'s header, rather than one copied from a
/// plan's text.
///
/// **`--attr-source HEAD` and `--shallow-file /tmp/s` are MINIMUM entries because
/// they are the two spellings the rule's new knowledge uniquely adds** — exactly
/// as `&>>` and `<<-` were for round 6 — and a corpus without them cannot fail on
/// the two options `T-19-100` is about. Measured against the built binary at this
/// file's base commit, they are also the ONLY two entries whose refused-arm cases
/// are at exit 0 today: 8 cases each, 16 of the refused arm's 120. Their presence
/// is asserted by name below.
///
/// **Every entry here must be VERDICT-PRESERVING**, because the property's
/// permitted arm asserts that the spliced verdict EQUALS the unspliced one. An
/// option the installed git does NOT accept is refused after `19-21` even on a
/// PERMITTED base, so it would be STRICTER than its base — see
/// [`GIT_GLOBAL_UNKNOWN_OPTIONS`], which is a separate alphabet for exactly that
/// reason.
///
/// **A bare `-` is DELIBERATELY NOT AN ENTRY, and this is the last latent
/// instance of the seam defect that blocked the previous three rounds.** Every
/// entry here is spliced into REFUSED bases and asserted refused; but
/// `git - push --force origin main` is pinned PERMITTED in
/// `tests/envelope_callee_grammar.rs`, at exit 0 both before and after, because
/// `scan_leading` breaks on `-` before any option check and `19-21` is FORBIDDEN
/// to widen the rule into non-`-`-prefixed words. A `-` entry would therefore
/// make the refused arm **permanently red in a file `19-21` may not edit**, which
/// is exactly `19-18`'s `{v}>` blocker one axis over. The exclusion is asserted
/// mechanically below.
const GIT_GLOBAL_OPTIONS: &[GitGlobalOption] = &[
    // class 1 — consumes a separate word
    GitGlobalOption { spelling: "--attr-source HEAD", class: "a leading option that consumes a separate word", words: 2 },
    GitGlobalOption { spelling: "--shallow-file /tmp/s", class: "a leading option that consumes a separate word", words: 2 },
    GitGlobalOption { spelling: "-C /tmp", class: "a leading option that consumes a separate word", words: 2 },
    GitGlobalOption { spelling: "--git-dir /tmp/g", class: "a leading option that consumes a separate word", words: 2 },
    GitGlobalOption { spelling: "--work-tree /tmp/w", class: "a leading option that consumes a separate word", words: 2 },
    GitGlobalOption { spelling: "--namespace n", class: "a leading option that consumes a separate word", words: 2 },
    // class 2 — consumes no word
    GitGlobalOption { spelling: "--no-pager", class: "a leading option that consumes no word", words: 1 },
    GitGlobalOption { spelling: "--bare", class: "a leading option that consumes no word", words: 1 },
    GitGlobalOption { spelling: "--literal-pathspecs", class: "a leading option that consumes no word", words: 1 },
    GitGlobalOption { spelling: "--no-optional-locks", class: "a leading option that consumes no word", words: 1 },
    GitGlobalOption { spelling: "-p", class: "a leading option that consumes no word", words: 1 },
    // class 3 — attached value, which needs no knowledge of git
    GitGlobalOption { spelling: "--git-dir=/tmp/g", class: "a leading option carrying an attached value", words: 1 },
    GitGlobalOption { spelling: "--attr-source=HEAD", class: "a leading option carrying an attached value", words: 1 },
    GitGlobalOption { spelling: "--namespace=n", class: "a leading option carrying an attached value", words: 1 },
    // class 5 — EXACTLY `--`, and a bare `-` is deliberately absent
    GitGlobalOption { spelling: "--", class: "the end-of-options marker or a bare dash", words: 1 },
];

/// The SEPARATE alphabet of leading options the installed git does NOT accept.
///
/// **Kept out of `GIT_GLOBAL_OPTIONS`, and out of the invariance arm, because its
/// entries are NOT verdict-preserving — this is `19-18`'s `{v}>` blocker one axis
/// over and it is the single most likely way this seam breaks.** The generative
/// property's permitted arm asserts that a spliced verdict EQUALS the unspliced
/// one. After `19-21`, an option in neither of the guard's two constants falls to
/// *grammar not established* and is REFUSED — including on a PERMITTED base. So
/// `git --bogus-opt status`, measured at exit 0 today, is refused after: STRICTER
/// than its base, and it would turn the invariance arm permanently red in a file
/// `19-21` may not edit.
///
/// The unknown class gets its OWN fail-closed property instead
/// (`an_option_the_installed_git_rejects_fails_closed_on_every_base`), spliced
/// into both refused and permitted bases and asserting refusal and an empty walk
/// in BOTH.
///
/// **`--no-advice` and `--no-lazy-fetch` are the measured stand-ins for the
/// future-git over-refusal cost this design accepts**: both are real git global
/// options in releases after 2.43, and both print `unknown option:` on the
/// installed git. A future cost argued only in prose is a cost nobody can check.
/// `--super-prefix x` is here because the installed git rejects it while
/// `GIT_GLOBAL_VALUE_OPTS` carries it — the measurement that names the LIST as
/// the defect rather than a missing row.
const GIT_GLOBAL_UNKNOWN_OPTIONS: &[&str] = &[
    "--bogus-opt",
    "--no-advice",
    "--no-lazy-fetch",
    "--super-prefix x",
];

/// Where an alphabet entry is spliced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CalleeSplice {
    /// Immediately after the governed program — the first thing `scan_leading`
    /// reads.
    ImmediatelyAfterTheProgram,
    /// After a leading option the guard ALREADY knows.
    ///
    /// **The scan is a LOOP, so the gap is not confined to the first slot.** This
    /// is the position the cell `git -c a=b --attr-source HEAD push --force
    /// origin main` reaches, measured at exit 0 at this file's base commit.
    AfterAKnownLeadingOption,
}

const CALLEE_SPLICES: &[CalleeSplice] = &[
    CalleeSplice::ImmediatelyAfterTheProgram,
    CalleeSplice::AfterAKnownLeadingOption,
];

/// The known leading option the `AfterAKnownLeadingOption` splice puts ahead of
/// the entry.
///
/// `-c a=b` and not `-c core.hooksPath=…`: the hooks-path KEY CHECK inside
/// `scan_leading` fires as soon as the scan reaches it, before any verb is
/// identified, and would refuse every case in the permitted arm for a reason that
/// has nothing to do with this axis. `a=b` is an inert assignment.
const CALLEE_KNOWN_LEADING_PREFIX: &str = "-c a=b";

/// Bases whose UNWRAPPED form is REFUSED.
///
/// Three of the four have NO SECOND CARRIER at all — `stash`, `update-ref -d`
/// and `config core.hooksPath`, where disarming the hook IS the loss of the
/// carrier — so for them this guard is the only control.
const CALLEE_REFUSED_BASES: &[&str] = &[
    "git push --force origin main",
    "git stash",
    "git update-ref -d refs/heads/main",
    "git config core.hooksPath /tmp/x",
];

/// Bases whose UNWRAPPED form is PERMITTED.
///
/// **Without this half the property cannot fail on an implementation that refuses
/// every leading option**, which is the shape this fix is ONE WRONG STEP away
/// from — and reading git state is the first thing a driven run does.
const CALLEE_PERMITTED_BASES: &[&str] = &["git status", "git log --oneline", "git diff"];

/// Splice one alphabet entry into `base`.
fn callee_case(base: &str, entry: &str, splice: CalleeSplice) -> String {
    let words: Vec<&str> = base.split_whitespace().collect();
    let program = words[0];
    let rest = words[1..].join(" ");
    match splice {
        CalleeSplice::ImmediatelyAfterTheProgram => format!("{program} {entry} {rest}"),
        CalleeSplice::AfterAKnownLeadingOption => {
            format!("{program} {CALLEE_KNOWN_LEADING_PREFIX} {entry} {rest}")
        }
    }
}

/// Every case this axis generates, as `(base, splice, label, command)`.
///
/// One function so the counting floor and the guard-driven property count exactly
/// the same thing — a second enumeration would be a second thing to keep in step.
fn callee_cases(
    bases: &[&'static str],
    alphabet: &[&str],
) -> Vec<(&'static str, CalleeSplice, String, String)> {
    let mut cases = Vec::new();
    for base in bases.iter().copied() {
        for splice in CALLEE_SPLICES {
            for entry in alphabet {
                cases.push((
                    base,
                    *splice,
                    format!("{splice:?}({entry})"),
                    callee_case(base, entry, *splice),
                ));
            }
        }
    }
    cases
}

/// The known alphabet's spellings, so `callee_cases` can be shared by both arms.
fn git_global_option_spellings() -> Vec<&'static str> {
    GIT_GLOBAL_OPTIONS.iter().map(|o| o.spelling).collect()
}

// ---------------------------------------------------------------------------
// 15b. The floors — per ALPHABET, per CLASS, and COUNTED over generated cases
// ---------------------------------------------------------------------------

/// The arithmetic, STATED rather than guessed, because audit 5 found `19-16` set
/// a floor of 50 against a maximum of 40 by construction.
///
/// `GIT_GLOBAL_OPTIONS` has 15 entries and `CALLEE_SPLICES` has 2, so every base
/// emits `15 x 2 = 30` cases over 2 slots:
///
/// * refused  — 4 bases -> 8 slots -> 4 x 30 = **120** cases
/// * permitted — 3 bases -> 6 slots -> 3 x 30 = **90** cases
/// * total = **210** cases over **14** slots
///
/// The floors are EXACT equalities, so losing one case turns them red. Nothing in
/// `src/` can move them: they are a pure function of the alphabets in this file.
const CALLEE_GRAMMAR_CASES: usize = 210;
const CALLEE_GRAMMAR_REFUSED_CASES: usize = 120;
const CALLEE_GRAMMAR_PERMITTED_CASES: usize = 90;
const CALLEE_GRAMMAR_SLOTS: usize = 14;
const MIN_CALLEE_GRAMMAR_CLASSES: usize = 5;
const MIN_GIT_GLOBAL_OPTIONS: usize = 15;
const MIN_GIT_GLOBAL_UNKNOWN_OPTIONS: usize = 4;

/// The unknown alphabet's own arithmetic: 4 entries x 2 splices x 7 bases = **56**
/// cases, of which 32 sit on refused bases and 24 on permitted ones. **All 24
/// permitted-base cases are at exit 0 today and must be REFUSED after `19-21`**,
/// which is what makes the fail-closed property red; the 32 refused-base cases
/// are already refused, for their verb rather than for their option.
const CALLEE_UNKNOWN_CASES: usize = 56;

/// The per-class counts over all 210 generated commands of the KNOWN alphabet,
/// derived from the alphabet and the splice set:
///
/// * class 1 (consumes a separate word) — the 6 class-1 entries at each of 14
///   slots = 84; plus the `-c` of `CALLEE_KNOWN_LEADING_PREFIX`, which is itself
///   value-taking, in every one of the 7 `AfterAKnownLeadingOption` slots x 15
///   entries = 105, of which 6 x 7 = 42 are already counted. 84 + 63 = **147**
/// * class 2 (consumes no word) — the 5 class-2 entries at each of 14 slots = **70**
/// * class 3 (attached value) — the 3 class-3 entries at each of 14 slots = **42**
/// * class 4 (unknown) — **0**, and that zero IS the disjointness assertion:
///   `GIT_GLOBAL_OPTIONS` must never draw the class that is not verdict-preserving
/// * class 5 (`--` or a bare `-`) — the 1 class-5 entry at each of 14 slots = **14**
const CALLEE_GRAMMAR_CLASS_COUNTS: &[(&str, usize)] = &[
    ("a leading option that consumes a separate word", 147),
    ("a leading option that consumes no word", 70),
    ("a leading option carrying an attached value", 42),
    ("a leading option the installed git does not accept", 0),
    ("the end-of-options marker or a bare dash", 14),
];

#[test]
fn every_alphabet_this_round_widens_can_draw_a_word_whose_grammar_is_gits_and_not_the_shells() {
    // **The direct mechanical inverse of audit 6's `T-19-101`.** The auditor
    // established the defect by grepping `tests/` for `attr-source`,
    // `shallow-file` and `GIT_GLOBAL_VALUE_OPTS` and finding NOTHING AT ALL; this
    // asserts the repaired fact, so narrowing the alphabet back turns this red
    // instead of quietly restoring a corpus that cannot fail on its own class.
    //
    // The predicate is evaluated on a REPRESENTATIVE SPLICED COMMAND rather than
    // on the bare entry, the way section 14's is, because these entries are
    // splice FRAGMENTS: `--attr-source HEAD` standing alone has no program before
    // it and no verb after it.
    //
    // **GREEN today and after.**
    assert!(
        CALLEE_GRAMMAR_CLASSES.len() >= MIN_CALLEE_GRAMMAR_CLASSES,
        "the callee-grammar axis must name at least {MIN_CALLEE_GRAMMAR_CLASSES} classes"
    );
    assert!(
        GIT_GLOBAL_OPTIONS.len() >= MIN_GIT_GLOBAL_OPTIONS,
        "`GIT_GLOBAL_OPTIONS` must carry at least {MIN_GIT_GLOBAL_OPTIONS} entries. The \
         correct response to a red here is to RESTORE entries, never to lower this floor."
    );
    assert!(
        GIT_GLOBAL_UNKNOWN_OPTIONS.len() >= MIN_GIT_GLOBAL_UNKNOWN_OPTIONS,
        "`GIT_GLOBAL_UNKNOWN_OPTIONS` must carry at least \
         {MIN_GIT_GLOBAL_UNKNOWN_OPTIONS} entries"
    );

    for option in GIT_GLOBAL_OPTIONS {
        let spliced = callee_case(
            "git push --force origin main",
            option.spelling,
            CalleeSplice::ImmediatelyAfterTheProgram,
        );
        assert!(
            carries_a_callee_grammar_class(&spliced),
            "`GIT_GLOBAL_OPTIONS` entry `{}` draws NO callee-grammar class when spliced \
             between the program and its decision words (`{spliced}`).\n\n\
             An alphabet entry that cannot DRAW a class is an entry whose property cannot \
             FAIL on one, and every case generated from it certifies a claim about a class it \
             could never have exercised. It is `T-19-76`'s failure mode for the SIXTH \
             consecutive round, after `T-19-83`, `T-19-89`, `T-19-95` and `T-19-99` — and \
             this time the gap moved AXIS rather than one cell over.\n\n\
             The correct response is to RESTORE the entry, never to delete this floor.",
            option.spelling
        );

        // The entry's TAG must agree with the predicate that actually fires, or
        // the per-class counts below are counting something the alphabet does not
        // claim.
        let (_, predicate) = CALLEE_GRAMMAR_CLASSES
            .iter()
            .find(|(class, _)| *class == option.class)
            .unwrap_or_else(|| panic!("entry `{}` names a class that exists", option.spelling));
        assert!(
            predicate(&spliced),
            "`{}` is TAGGED `{}` but the predicate for that class does not fire on \
             `{spliced}`. An alphabet whose tags and predicates disagree makes the per-class \
             floors meaningless.",
            option.spelling,
            option.class
        );

        // And the word count in the tag must match the spelling, so a two-word
        // entry cannot be silently rewritten to one.
        assert_eq!(
            option.spelling.split_whitespace().count(),
            option.words,
            "`{}` claims to occupy {} words",
            option.spelling,
            option.words
        );
    }

    // **The two spellings the rule's new knowledge uniquely adds must be DRAWN,
    // not merely named in a doc.** A class list naming a spelling no entry draws
    // is a floor nothing satisfies, and these two are the only entries whose
    // refused-arm cases are at exit 0 today.
    for required in ["--attr-source HEAD", "--shallow-file /tmp/s"] {
        assert!(
            git_global_option_spellings().contains(&required),
            "`{required}` must be an entry: it is one of the two spellings `19-21`'s \
             production uniquely adds, it is measured at exit 0 today on every refused base, \
             and a corpus without it cannot fail on the option `T-19-100` is about"
        );
    }

    // **And a bare `-` must NOT be one**, for the reason the alphabet's doc gives.
    assert!(
        !GIT_GLOBAL_OPTIONS
            .iter()
            .any(|option| option.spelling == "-" || option.spelling.starts_with("- ")),
        "a bare `-` must NOT be an entry of `GIT_GLOBAL_OPTIONS`. Every entry here is \
         spliced into REFUSED bases and asserted REFUSED, but \
         `git - push --force origin main` is pinned PERMITTED in \
         `tests/envelope_callee_grammar.rs` at exit 0 both before and after — `scan_leading` \
         breaks on `-` before any option check runs, and `19-21` is FORBIDDEN to widen the \
         rule into non-`-`-prefixed words. A `-` entry would make the refused arm PERMANENTLY \
         RED in a file `19-21` may not edit, which is `19-18`'s `{{v}}>` blocker one axis \
         over.\n\n\
         The class-5 PREDICATE still recognises a bare `-`, and must: the class describes the \
         tokens the scan breaks on, and narrowing it to match the alphabet would make the \
         class a description of the alphabet rather than of the grammar."
    );

    // **THE DISJOINTNESS ASSERTION.** The two alphabets must not overlap, or the
    // invariance arm would draw an entry whose verdict the fix CHANGES.
    for unknown in GIT_GLOBAL_UNKNOWN_OPTIONS {
        assert!(
            !git_global_option_spellings().contains(unknown),
            "`{unknown}` appears in BOTH alphabets. `GIT_GLOBAL_OPTIONS` entries are asserted \
             VERDICT-PRESERVING by the invariance arm; an option the installed git does not \
             accept is REFUSED after `19-21` even on a PERMITTED base, so it is STRICTER than \
             its base. Mixing the two would turn the invariance arm permanently red in a file \
             `19-21` may not edit."
        );
        let spliced = callee_case("git status", unknown, CalleeSplice::ImmediatelyAfterTheProgram);
        assert!(
            draws_a_leading_option_the_installed_git_rejects(&spliced),
            "`{unknown}` must draw the UNKNOWN class when spliced (`{spliced}`), or the \
             fail-closed property below is asserting refusal about something else"
        );
    }
}

#[test]
fn the_generated_corpus_really_produces_each_callee_grammar_class_in_quantity() {
    // **An alphabet floor is not a generation floor.** An entry can sit in an
    // alphabet and be drawn by nothing, or be drawn once out of hundreds of cases
    // — which is a corpus that can technically fail on the class and practically
    // never does. This counts what the generator ACTUALLY emits.
    //
    // **Green today and after**: it drives no guard call at all, and no production
    // change can move it. The counts are a pure function of the alphabets above,
    // which is exactly what makes the exact equalities safe.
    let spellings = git_global_option_spellings();
    let refused = callee_cases(CALLEE_REFUSED_BASES, &spellings);
    let permitted = callee_cases(CALLEE_PERMITTED_BASES, &spellings);

    assert_eq!(
        refused.len(),
        CALLEE_GRAMMAR_REFUSED_CASES,
        "the refused arm's generation count must equal the stated arithmetic exactly, so \
         losing one case turns this red. `19-16` set a floor of 50 against a maximum of 40 by \
         construction and it was invisible until the rule landed."
    );
    assert_eq!(
        permitted.len(),
        CALLEE_GRAMMAR_PERMITTED_CASES,
        "the permitted arm's generation count must equal the stated arithmetic exactly"
    );

    let all: Vec<&(&str, CalleeSplice, String, String)> =
        refused.iter().chain(permitted.iter()).collect();
    assert_eq!(all.len(), CALLEE_GRAMMAR_CASES);

    let slots: BTreeSet<(&str, CalleeSplice)> =
        all.iter().map(|(base, splice, _, _)| (*base, *splice)).collect();
    assert_eq!(
        slots.len(),
        CALLEE_GRAMMAR_SLOTS,
        "every splice slot must have been generated. Seen: {slots:?}"
    );

    let mut per_class: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, _, _, command) in &all {
        for (class, predicate) in CALLEE_GRAMMAR_CLASSES {
            if predicate(command) {
                *per_class.entry(class).or_default() += 1;
            }
        }
    }

    for (class, expected) in CALLEE_GRAMMAR_CLASS_COUNTS {
        let got = per_class.get(class).copied().unwrap_or(0);
        assert_eq!(
            got, *expected,
            "the generator emitted {got} cases of the callee-grammar class `{class}`, and the \
             stated arithmetic derives {expected}.\n\n\
             This is `T-19-101` counted rather than read. The correct response to a shortfall \
             is to RESTORE entries, never to lower the number. Full counts: {per_class:?}"
        );
    }

    // The unknown alphabet's own count.
    let unknown_all = callee_cases(CALLEE_REFUSED_BASES, GIT_GLOBAL_UNKNOWN_OPTIONS).len()
        + callee_cases(CALLEE_PERMITTED_BASES, GIT_GLOBAL_UNKNOWN_OPTIONS).len();
    assert_eq!(
        unknown_all, CALLEE_UNKNOWN_CASES,
        "the unknown alphabet must generate exactly {CALLEE_UNKNOWN_CASES} cases"
    );

    // Recorded so the SUMMARY carries measured counts rather than described ones.
    println!(
        "callee-grammar axis: {} cases ({} refused, {} permitted) over {} slots from {} \
         known entries and {} splice positions, plus {} unknown-alphabet cases from {} \
         entries.\nper class: {per_class:?}",
        all.len(),
        refused.len(),
        permitted.len(),
        slots.len(),
        GIT_GLOBAL_OPTIONS.len(),
        CALLEE_SPLICES.len(),
        unknown_all,
        GIT_GLOBAL_UNKNOWN_OPTIONS.len(),
    );
}

// ---------------------------------------------------------------------------
// 15c. The generative properties — RED against the pre-fix tree
// ---------------------------------------------------------------------------

#[test]
fn a_leading_option_whose_grammar_the_guard_does_not_know_is_not_a_verb() {
    // **Modelled directly on
    // `a_word_the_shell_deletes_between_the_program_and_its_decision_words_is_not_a_decision_word`**,
    // with the PERMITTED arm running FIRST so that today's failure output is
    // itself evidence the permitted half passed. A property whose permitted arm
    // is unobservable until the fix lands cannot be said to fail in both
    // directions.
    //
    // Measured against the built binary while this plan was written: of the 120
    // refused-arm cases, **16 are at exit 0 today** — the 8 `--attr-source HEAD`
    // and 8 `--shallow-file /tmp/s` cases — and 104 are already refused for a
    // grammar the guard happens to know. All 90 permitted-arm cases are at exit 0
    // today, with no anomalies and no ledger lines.
    //
    // **The refusal arm asserts the exit code and an EMPTY WALK, not a reason
    // identifier.** A class-3 or class-5 case on a refused base is refused TODAY
    // for its verb and after the fix for the same verb, while a class-1 case moves
    // from permitted to refused — so which D-24 identifier a case lands on
    // legitimately differs by entry. Pinning it here would be pinning a verdict
    // this plan cannot derive per case; the identifiers live in the NAMED per-row
    // pins of `tests/envelope_callee_grammar.rs`, where each carries its own
    // written derivation.

    // --- floor 0: the POSITIVE control for the walk -----------------------
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "a PERMITTED `gh pr create` writes exactly one ledger line, and the walk must be able \
         to find it. If this is 0 the walk is blind and every empty-walk assertion below is \
         vacuous. Files: {:?}",
        files_under(control.path())
    );

    // --- floor 1: every base answers what the arm it is in claims ---------
    let mut base_verdicts: BTreeMap<&str, Verdict> = BTreeMap::new();
    for base in CALLEE_PERMITTED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 0,
            "the UNWRAPPED base `{base}` must be PERMITTED, or every case built on it is \
             green or red for the base's own reason. Got reason id: {}",
            got.reason_id
        );
        base_verdicts.insert(base, got);
    }
    for base in CALLEE_REFUSED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 2,
            "the UNWRAPPED base `{base}` must be REFUSED, or the leading option cannot be \
             what makes the difference. Got reason id: {}",
            got.reason_id
        );
        assert!(
            REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
            "and refused under a D-24 identifier. Got: {}",
            got.reason_id
        );
    }

    let spellings = git_global_option_spellings();

    // --- the PERMITTED arm: a KNOWN leading option is VERDICT-PRESERVING ---
    //
    // Runs first, and is GREEN today. Without it, an implementation that simply
    // refused every governed command carrying a leading `-` would satisfy the
    // whole refusal arm below — while refusing `git --no-pager status`,
    // `git -C /tmp status` and `git --version`, whose measured cost
    // `tests/envelope_callee_grammar.rs` pins row by row. **That is the shape
    // this fix is ONE WRONG STEP away from.**
    let mut permitted_cases = 0usize;
    for (base, splice, label, command) in callee_cases(CALLEE_PERMITTED_BASES, &spellings) {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), &command);
        let expected = base_verdicts.get(base).expect("the base was measured");
        assert_eq!(
            got,
            *expected,
            "\n\nA KNOWN LEADING GIT OPTION CHANGED A PERMITTED VERDICT.\n\
             \n  command : {command:?}\
             \n  base    : {base}\
             \n  splice  : {splice:?} / {label}\
             \n  got     : exit {} reason {}\n\
             \nEvery entry of `GIT_GLOBAL_OPTIONS` was CLASSIFIED by a two-sided probe of the \
             real git binary, so after the fix the guard knows exactly how many words each \
             occupies and reaches the base's own verb. Confirmed by running real git: \
             `git --attr-source HEAD status` prints `On branch gsd-auto/alpha/w` and \
             `git --shallow-file /tmp/s log --oneline` prints `7347130 init` in a fixture.\n\
             \n**A red here is the fix having become a blanket refusal of anything beginning \
             with `-`**, which is a guard nobody can use and therefore a control that gets \
             switched off (AR-19-11). It is NOT a reason to narrow this alphabet, and it is \
             NOT a reason to move an entry into `GIT_GLOBAL_UNKNOWN_OPTIONS` — that alphabet \
             is for options the installed git REJECTS, and its membership is a measurement.",
            got.code,
            got.reason_id,
        );
        permitted_cases += 1;
    }

    // --- the REFUSED arm: RED against the pre-fix tree --------------------
    let mut refused_cases = 0usize;
    let mut slots_seen: BTreeSet<(&str, CalleeSplice)> = BTreeSet::new();
    let mut classes_seen: BTreeSet<&str> = BTreeSet::new();

    for (base, splice, label, command) in callee_cases(CALLEE_REFUSED_BASES, &spellings) {
        slots_seen.insert((base, splice));
        for (class, predicate) in CALLEE_GRAMMAR_CLASSES {
            if predicate(&command) {
                classes_seen.insert(class);
            }
        }

        // A fresh root per case, because a permitted forge command writes a
        // ledger line and a shared root would exhaust the cap and turn later
        // cases red for a reason that has nothing to do with the option.
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), &command);

        assert_eq!(
            got.code,
            2,
            "\n\nA LEADING GIT OPTION CONSUMED THE VERB AND THE COMMAND WAS PERMITTED.\n\
             \n  command : {command:?}\
             \n  base    : {base}\
             \n  splice  : {splice:?} / {label}\
             \n  got     : exit {} reason {}\
             \n  seed    : {SEED:#x}\n\
             \n`scan_leading` walks leading `-`-initial tokens and asks `leading_git_option` \
             how many words each occupies. Its answer for an option it does not recognise is \
             `(None, 1)` at `policy.rs:488` — it ASSUMES one word. The loop then advances one \
             word, lands on the option's VALUE, sees it does not start with `-`, and BREAKS \
             with that value as the verb. `classify_git` finds that value in no denylist arm \
             and answers `Allow`.\n\
             \n**Every word here is literal and every word arrives, in order.** Round 5's bit \
             is RIGHT about this line and round 6's model is RIGHT about it — both are pinned \
             non-vacuous in `tests/envelope_callee_grammar.rs`. What is wrong is the verb \
             INDEX, and it is wrong because of a fact about GIT rather than about bash.\n\
             \n**The correct response is to make the ABSENCE of that one bit a REFUSAL \
             instead of a guess**, the same fail-closed treatment `resolve_program`'s wrapper \
             axis already has. It is NOT a blanket refusal of anything beginning with `-` — \
             the permitted arm above and the twelve ordinary invocations in \
             `tests/envelope_callee_grammar.rs` measure that option's cost. It is NOT a \
             widening of the rule into non-`-`-prefixed words — `git - push --force origin \
             main` is pinned PERMITTED. And it is NOT a narrowing of `GIT_GLOBAL_OPTIONS`, \
             `GIT_GLOBAL_UNKNOWN_OPTIONS` or the five `CALLEE_GRAMMAR_CLASSES`.",
            got.code,
            got.reason_id,
        );

        let written = ledger_lines_under(envelope.path());
        assert!(
            written.is_empty(),
            "`{command:?}` was refused, but a pull-request ledger line was written somewhere \
             under the envelope root. Found: {written:?} Files: {:?}",
            files_under(envelope.path())
        );

        refused_cases += 1;
    }

    // --- the floors, reachable only once the arm above is green -----------
    assert_eq!(
        permitted_cases, CALLEE_GRAMMAR_PERMITTED_CASES,
        "the permitted arm must run every generated case"
    );
    assert_eq!(
        refused_cases, CALLEE_GRAMMAR_REFUSED_CASES,
        "the refused arm must run every generated case"
    );
    assert_eq!(
        slots_seen.len(),
        CALLEE_REFUSED_BASES.len() * CALLEE_SPLICES.len(),
        "every splice slot of every refused base must have been generated. Seen: {slots_seen:?}"
    );
    assert!(
        classes_seen.len() >= 4,
        "the refused arm must draw at least FOUR callee-grammar classes — every class the \
         KNOWN alphabet can express. It cannot draw the fifth (the unknown class), and that \
         is deliberate: `GIT_GLOBAL_UNKNOWN_OPTIONS` is not verdict-preserving and has its \
         own fail-closed property. Seen: {classes_seen:?}"
    );

    println!(
        "callee-grammar axis drove {permitted_cases} permitted and {refused_cases} refused \
         cases over {} slots, drawing {} classes.",
        slots_seen.len(),
        classes_seen.len()
    );
}

#[test]
fn an_option_the_installed_git_rejects_fails_closed_on_every_base() {
    // **The unknown class's OWN property, separate because its entries are not
    // verdict-preserving.** An option the installed git does not accept is
    // REFUSED after `19-21` on BOTH kinds of base — that is the whole point of
    // inverting the failure direction — so it cannot be drawn by the invariance
    // arm above without turning that property permanently red in a file `19-21`
    // may not edit. This is `19-18`'s `{v}>` blocker one axis over, prevented by
    // construction rather than by care.
    //
    // Measured against the built binary at this file's base commit: of the 56
    // cases, the 32 on refused bases are already refused (for their VERB, which
    // the one-word assumption happens to reach), and **all 24 on permitted bases
    // are at exit 0**. Those 24 are what makes this property RED.
    //
    // **This is the measured over-refusal cost of the round, and it is asserted
    // rather than argued.** On the installed git 2.43.0 the cost is ZERO in
    // practice: every option this git ACCEPTS is classified by the probe and
    // enumerated, so the only commands moving permitted -> refused are ones git
    // itself rejects — refusals of commands that already do nothing.
    // `--no-advice` and `--no-lazy-fetch` are the measured stand-ins for the
    // FUTURE-git cost: real global options in later releases, rejected by this
    // git, one refusal each until the constant learns them.
    let mut cases = 0usize;
    for bases in [CALLEE_REFUSED_BASES, CALLEE_PERMITTED_BASES] {
        for (base, splice, label, command) in callee_cases(bases, GIT_GLOBAL_UNKNOWN_OPTIONS) {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code,
                2,
                "\n\nAN OPTION THE INSTALLED GIT DOES NOT ACCEPT WAS PERMITTED.\n\
                 \n  command : {command:?}\
                 \n  base    : {base}\
                 \n  splice  : {splice:?} / {label}\
                 \n  got     : exit {} reason {}\n\
                 \nThe guard has NO BIT for this spelling: it is in neither of the two \
                 constants, carries no attached `=`, and is not `-`, `--` or a non-option \
                 word. **The absence of that bit must be a REFUSAL and not a guess**, which \
                 is the inversion this round is about.\n\
                 \nA refusal here is cheap and honest: on git 2.43.0 every one of these \
                 spellings prints `unknown option:`, so the command does nothing anyway. On a \
                 FUTURE git the cost is one refusal per newly added global option until the \
                 constant learns it — `--no-advice` and `--no-lazy-fetch` are the measured \
                 stand-ins, and the recovery is to spell the option's value ATTACHED where \
                 git accepts that form, to drop the option, or to add it to the constant.\n\
                 \n**The correct response is NOT to move this entry into \
                 `GIT_GLOBAL_OPTIONS`.** That alphabet's membership is a MEASUREMENT — a \
                 two-sided probe of the real git binary — and an entry there is asserted \
                 verdict-preserving.",
                got.code,
                got.reason_id,
            );

            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command:?}` was refused, but a pull-request ledger line was written \
                 somewhere under the envelope root. Found: {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }

    assert_eq!(
        cases, CALLEE_UNKNOWN_CASES,
        "the fail-closed arm must run every generated unknown-alphabet case"
    );
}

// ---------------------------------------------------------------------------
// 16. The FOURTH named axis — `CONFIG_RESOLUTION_CLASSES`
//
// **A FOURTH axis standing beside three BYTE-IDENTICAL command-line axes, not
// more entries in any of them.** `UNREADABLE_CLASSES` names seven ways bash
// ASSEMBLES a word; `DELETION_CLASSES` names five ways bash DELETES one;
// `CALLEE_GRAMMAR_CLASSES` names five ways GIT's own option grammar decides which
// arriving word is the verb. **All three are axes of how a COMMAND LINE becomes
// an argv and which word in it is the verb.** The five below are about something
// else entirely: **what that verb RUNS UNDER.**
//
// Audit 7's framing is the reason there is a fourth axis rather than more entries
// in the third: git turns a command line into behaviour in THREE stages, and this
// phase now models one. The gap moved off the command line — from *how a word is
// written* (round 5), to *which words arrive* (round 6), to *which arriving word
// is the verb* (round 7), to **what that verb runs under** (round 8). Audit 7
// verified mechanically that nothing anywhere modelled it:
// `grep -rn "include\.path\|includeIf" src/ tests/` returned **nothing at all**
// and `grep -rn "GIT_CONFIG_PARAMETERS" src/ tests/` returned **nothing at all**.
//
// **An axis smuggled into an existing one stops being legible as one** — which is
// how a corpus comes to model exactly the control it certifies and nothing beside
// it. Nothing in sections 13, 14 or 15 is touched by this section, and neither
// are `POLICY_MIN_PRODUCTION_BYTES`, `HOOKS_MIN_PRODUCTION_BYTES` or
// `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`.
// **This round is granted no deletion of any kind**; `19-20`'s ratio deletion was
// a named one-off and there is no equivalent here.
//
// Every per-row verdict, every derivation and every real-git probe behind this
// section lives in `tests/envelope_config_resolution.rs`, the round's own
// evidence file.
//
// ---------------------------------------------------------------------------
// **ROUND 9 EXTENDS THIS AXIS AND DOES NOT CREATE A FIFTH — THE AXIS DID NOT
// MOVE, THE REGION DID.**
//
// Rounds 1-5 were about how a word is WRITTEN; round 6 about which words ARRIVE;
// round 7 about which arriving word is the VERB; round 8 about what the verb RUNS
// UNDER. **Round 9 is on the SAME axis as round 8, one REGION over**, and that is
// audit 8's own framing, adopted.
//
// The confinement clause is the right SHAPE of rule: it asks whether an assignment
// can be BOUNDED, not whether it spells a name, and audit 8 verified every one of
// its rows. **But the guard reads config assignments in TWO regions — the
// leading-option region `scan_leading` walks, and the environment — while git
// resolves configuration from a THIRD: a value the guard itself CONFINED and
// handed on.** `alias.q` names the section `alias`, which is not an indirection,
// so `config_key_names_an_indirection_section` correctly answers `false`, the
// assignment is CONFINED, and the carrier rides inside its VALUE into a position
// `scan_leading` never reads.
//
// **Standing that up as a FIFTH AXIS would model a REGION as if it were a STAGE
// and lose exactly the distinction audit 8 drew.** The axis is still *what the verb
// runs under*; the class is the region the round-8 rule does not reach. So
// `CONFIG_RESOLUTION_CLASSES` gains a SIXTH class and
// `MIN_CONFIG_RESOLUTION_CLASSES` rises from 5 to 6, while `UNREADABLE_CLASSES`,
// `DELETION_CLASSES`, `CALLEE_GRAMMAR_CLASSES`, all of their predicates, all of
// their degenerate-proofing and all of their floors stay BYTE-IDENTICAL.
//
// Every per-row verdict, every derivation, every real-git probe and the rebuilt
// bare-remote fixture behind round 9's half live in
// `tests/envelope_reparsed_value.rs`, the EIGHTH evidence file.
// ---------------------------------------------------------------------------

/// One config assignment a `-c` / `--config-env` carrier delivers in the LEADING
/// region of a governed command: the carrier that delivered it, the KEY half and
/// the VALUE half.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigAssignment {
    carrier: String,
    key: String,
    #[allow(dead_code)]
    value: String,
}

/// Split a command into (the `GIT_CONFIG*`-shaped names assigned in the
/// ASSIGNMENT-PREFIX region, the index of the governed program, the words).
///
/// **The prefix region is where `resolve_program`'s step-1 check reads**, and it
/// is a different place from where `scan_leading` reads. Recognising the two
/// separately is what lets class 3 exist at all: an environment carrier puts no
/// `-c` on the line, so no predicate that only walks leading options could ever
/// draw it.
fn config_prefix_split(command: &str) -> (Vec<String>, usize, Vec<&str>) {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut names = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let token = words[index].trim_end_matches(';');
        if token == "env" || token == "export" {
            index += 1;
            continue;
        }
        if token.starts_with('-') {
            break;
        }
        let Some((name, _)) = token.split_once('=') else {
            break;
        };
        let plausible = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        if !plausible {
            break;
        }
        names.push(name.to_string());
        index += 1;
    }
    (names, index, words)
}

/// Every config assignment carried in the leading region of the governed command.
///
/// Walked exactly as `scan_leading` walks it: from the word AFTER the program,
/// stopping at the first word that is not `-`-initial (that word is the VERB) and
/// at a bare `-` or `--`. Arm (a) of `leading_git_option` returns the NEXT TOKEN
/// as the assignment for a bare `-c` / `--config-env` and arm (b) returns the
/// attached one, so both spellings are read here for the same reason the guard
/// reads both.
///
/// **An option AFTER the verb is not read at all**, which is why
/// `git push -c include.path=… --force origin main` satisfies NO class — see the
/// quoting-and-position control below.
fn config_assignments(command: &str) -> Vec<ConfigAssignment> {
    let (_, program, words) = config_prefix_split(command);
    let mut out = Vec::new();
    let mut index = program + 1;
    let push = |out: &mut Vec<ConfigAssignment>, carrier: &str, text: &str| {
        let (key, value) = text.split_once('=').unwrap_or((text, ""));
        out.push(ConfigAssignment {
            carrier: carrier.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        });
    };
    while index < words.len() {
        let token = words[index];
        if !token.starts_with('-') || token == "-" || token == "--" {
            break;
        }
        if token == "-c" || token == "--config-env" {
            if let Some(next) = words.get(index + 1) {
                push(&mut out, token, next);
            }
            index += 2;
        } else if let Some(rest) = token.strip_prefix("--config-env=") {
            push(&mut out, "--config-env", rest);
            index += 1;
        } else if token.len() > 2 && token.starts_with("-c") {
            push(&mut out, "-c", &token[2..]);
            index += 1;
        } else {
            index += 1;
        }
    }
    out
}

/// The SECTION half of a config key — the text before the FIRST `.`, or the empty
/// string for a key with no `.` at all.
///
/// **A dotless key has no section**, and real git says so: `git -c a=b config
/// --get a` answers `error: key does not contain a section: a` while
/// `git -c a=b version` RUNS at rc 0. That measurement is asserted in
/// `tests/envelope_config_resolution.rs`.
fn config_section(key: &str) -> &str {
    key.split_once('.').map(|(section, _)| section).unwrap_or("")
}

/// The VARIABLE half — the text after the LAST `.`.
///
/// Git folds the SECTION and the VARIABLE to lower case and leaves the SUBSECTION
/// case-sensitive, measured against `git version 2.43.0` with the envelope's own
/// injection as the control.
fn config_variable(key: &str) -> &str {
    key.rsplit_once('.')
        .map(|(_, variable)| variable)
        .unwrap_or("")
}

/// Whether a config key names a section that pulls FURTHER configuration in.
///
/// `include` and `includeIf` are the two, ASCII-case-folded because git folds the
/// section. **Only the SECTION is read** — not the subsection and not the
/// variable — which is the whole design: `[include]` honours exactly one variable
/// and `includeIf`'s subsection is an open condition family, so not reading either
/// covers both by construction.
fn config_key_is_an_indirection(key: &str) -> bool {
    let section = config_section(key).to_ascii_lowercase();
    section == "include" || section == "includeif"
}

/// **Class 1** — a command-line carrier of a config INDIRECTION.
///
/// A `-c` or `--config-env` assignment whose key names a section that pulls
/// further configuration in **at the precedence of the directive that named it**,
/// i.e. at command-line precedence, without the string `core.hooksPath` appearing
/// anywhere. **This is the class `T-19-103` lives in.**
fn draws_a_config_indirection_carrier(command: &str) -> bool {
    config_assignments(command)
        .iter()
        .any(|a| config_key_is_an_indirection(&a.key))
}

/// **Class 2** — a command-line carrier of a CONFINED assignment.
///
/// A `-c` assignment whose effect is BOUNDED to the key it names.
/// **Class 1 cannot satisfy this and class 2 cannot satisfy class 1**, and that
/// split IS the resolution question: a guard that could not tell the two apart
/// would either miss `T-19-103` or refuse every `-c` — and
/// `git -c user.name="$NAME" commit -m x`, pinned PERMITTED in
/// `tests/envelope_config_resolution.rs`, is this axis's `ls {git,svn}-repo`.
fn draws_a_confined_config_carrier(command: &str) -> bool {
    config_assignments(command)
        .iter()
        .any(|a| !config_key_is_an_indirection(&a.key))
}

/// **Class 3** — an ENVIRONMENT carrier of configuration.
///
/// A word that sets configuration git reads with **no `-c` on the line at all**.
/// `GIT_CONFIG_PARAMETERS` is git's own internal carrier for `-c`, it OUTRANKS
/// the envelope's injected triplet (measured `/PARAM_WINS` against the control's
/// `/ENV_WINS`), and git EXPORTS it, so one prefix disarms every git SUBPROCESS
/// of the command.
///
/// **The command line is not involved**, which is why this class cannot live in
/// any of the three existing axes: all three walk argv words, and this one is
/// read by `resolve_program`'s step-1 check over the assignment-prefix region.
fn draws_an_environment_config_carrier(command: &str) -> bool {
    config_prefix_split(command)
        .0
        .iter()
        .any(|name| name.to_ascii_uppercase().starts_with("GIT_CONFIG"))
}

/// **Class 4** — a CASE-VARIED spelling of an indirection.
///
/// Git folds the SECTION and the VARIABLE to lower case while leaving the
/// SUBSECTION case-sensitive. Measured in both halves of the key against
/// `git version 2.43.0` with the envelope's own injection as the control:
/// `-c INCLUDE.PATH=<f>` resolves to `/INCLUDE_WINS`, and so does
/// `-c INCLUDEIF.gitdir:<p>.PATH=<f>`.
///
/// **`git -c include.path=…` must NOT satisfy this**, or class 4 would be a
/// second name for class 1 and its floor would be satisfiable without a single
/// case-varied spelling being drawn.
fn draws_a_case_varied_indirection(command: &str) -> bool {
    config_assignments(command).iter().any(|a| {
        config_key_is_an_indirection(&a.key)
            && (config_section(&a.key) != config_section(&a.key).to_ascii_lowercase()
                || config_variable(&a.key) != config_variable(&a.key).to_ascii_lowercase())
    })
}

/// **Class 5** — an OPTION-CARRIER delivery of an indirection.
///
/// `--config-env=include.path=<VAR>` and its separate-word form
/// `--config-env include.path=<VAR>`: the SECOND carrier `scan_leading` reads,
/// and the one whose value is an environment variable NAME rather than a value.
/// Real git resolves it identically — measured `/INCLUDE_WINS` through both
/// spellings — which is why one clause covers both.
fn draws_an_option_carrier_indirection(command: &str) -> bool {
    config_assignments(command)
        .iter()
        .any(|a| a.carrier == "--config-env" && config_key_is_an_indirection(&a.key))
}

// ---------------------------------------------------------------------------
// Round 9's additions — the SIXTH class on this SAME axis
// ---------------------------------------------------------------------------

/// Split a command into words with SHELL QUOTING applied, so a quoted config VALUE
/// arrives as the bytes the program would receive rather than as three
/// whitespace-separated fragments.
///
/// **Why this exists beside `config_prefix_split`'s `split_whitespace`, rather than
/// replacing it.** The five existing classes decide on the KEY half, which never
/// contains a space, so whitespace splitting is exact for them and
/// `config_assignments` is left BYTE-IDENTICAL. Round 9's class decides on the
/// VALUE half, which is a whole command line — and the distinction between
/// `-c alias.q="-c include.path=<f> status"` (value `-c include.path=<f> status`,
/// no quote byte) and `-c alias.q='"!git …"'` (value `"!git …"`, first byte a
/// literal `"`) is *exactly* the distinction the one-byte fence turns on. A
/// whitespace splitter conflates the two and the fence stops meaning anything.
fn shell_words(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut started = false;
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) if c == q => quote = None,
            Some('"') if c == '\\' => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            Some(_) => current.push(c),
            None if c == '\'' || c == '"' => {
                started = true;
                quote = Some(c);
            }
            None if c.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            None => {
                started = true;
                current.push(c);
            }
        }
    }
    if started {
        words.push(current);
    }
    words
}

/// Every config assignment in the leading region, read with SHELL QUOTING applied
/// so the VALUE half is the bytes git would receive.
///
/// Walked exactly as `config_assignments` walks it — and for the same reason the
/// guard reads both: arm (a) of `leading_git_option` returns the NEXT token for a
/// bare `-c` / `--config-env` and arm (b) the attached one.
fn quoted_config_assignments(command: &str) -> Vec<ConfigAssignment> {
    let words = shell_words(command);
    let mut out = Vec::new();
    let mut index = 1;
    let mut push = |out: &mut Vec<ConfigAssignment>, carrier: &str, text: &str| {
        let (key, value) = text.split_once('=').unwrap_or((text, ""));
        out.push(ConfigAssignment {
            carrier: carrier.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        });
    };
    while index < words.len() {
        let token = words[index].as_str();
        if !token.starts_with('-') || token == "-" || token == "--" {
            break;
        }
        if token == "-c" || token == "--config-env" {
            if let Some(next) = words.get(index + 1) {
                push(&mut out, token, next);
            }
            index += 2;
        } else if let Some(rest) = token.strip_prefix("--config-env=") {
            push(&mut out, "--config-env", rest);
            index += 1;
        } else if token.len() > 2 && token.starts_with("-c") {
            push(&mut out, "-c", &token[2..]);
            index += 1;
        } else {
            index += 1;
        }
    }
    out
}

/// The `(key, value)` a `git config <key> <value>` WRITE names, if the command is
/// one — the PERSISTED delivery's decision region.
///
/// **This is the region `classify_config` already reads with `is_hooks_path_key`**,
/// which is why asking the alias question there is the SAME reading site and not a
/// second one. Level options are skipped, and the three that take a value of their
/// own consume it.
fn persisted_config_write(command: &str) -> Option<(String, String)> {
    let words = shell_words(command);
    if words.len() < 4 || words[1] != "config" {
        return None;
    }
    let mut index = 2;
    while index < words.len() && words[index].starts_with('-') {
        if matches!(words[index].as_str(), "--file" | "-f" | "--blob") {
            index += 1;
        }
        index += 1;
    }
    let key = words.get(index)?.clone();
    let value = words.get(index + 1)?.clone();
    Some((key, value))
}

/// Whether a config VALUE is a SHELL alias body — git's rule, which is the FIRST
/// BYTE and nothing else.
///
/// **THE FENCE THIS WHOLE ROUND TURNS ON, and it is a MEASUREMENT.** Against
/// `git version 2.43.0` with the envelope's own injection as the control
/// (`/ENV_WINS`): `-c alias.b='!git config --get core.hooksPath' b` prints
/// `/ENV_WINS` — a `!` body is handed to a SHELL and run as a CHILD that INHERITS
/// the triplet, so layer 3 is intact — while
/// `-c alias.a='-c include.path=<f> config --get core.hooksPath' a` prints
/// `/INCLUDE_WINS` — a non-`!` body is re-parsed by git IN-PROCESS at command-line
/// precedence.
///
/// The boundary is the FIRST BYTE and nothing else, measured in nine spellings in
/// `tests/envelope_reparsed_value.rs`: a `!` that is not first is not a shell body;
/// a SPACE or a TAB before `!` makes git refuse to expand at all; and a QUOTED body
/// whose first byte is `"` is re-parsed IN-PROCESS, so refusing it is CORRECT.
/// **No spelling was found in which the first byte IS `!` and git nonetheless
/// re-parses in-process**, and a counterexample would be a FINDING rather than a
/// row.
fn config_value_is_a_shell_alias_body(value: &str) -> bool {
    value.starts_with('!')
}

/// Whether a config key names the ALIAS section — ASCII-case-folded, because git
/// folds the SECTION.
///
/// **Only the SECTION is read.** A rule written as `key.starts_with("alias")` turns
/// `-c aliasx.q=…` red and a rule written as `key.contains("alias")` turns
/// `-c notalias.q=…` red; both are pinned PERMITTED before and after in
/// `tests/envelope_reparsed_value.rs` as this round's `--signed no`.
fn config_key_names_the_alias_section(key: &str) -> bool {
    config_section(key).to_ascii_lowercase() == "alias"
}

/// **Class 6** — a carrier delivered inside a config VALUE THE GUARD CONFINES.
///
/// A `-c` / `--config-env` assignment, or a persisted `git config` write, whose KEY
/// names an ordinary BOUNDED section (`alias` is not an indirection, so round 8's
/// clause correctly CONFINES it) and whose VALUE git RE-PARSES as a git command
/// line **including its own leading options**.
///
/// **This is the class `T-19-108` lives in, and it is a REGION rather than an
/// axis.** Both deliveries are live: measured against real git, a `-c alias.<n>=`
/// carrier and a persisted `git config alias.<n>` both resolve `/INCLUDE_WINS`
/// against the control's `/ENV_WINS`.
///
/// **A `!`-bodied value must NOT satisfy this**, or the class would name
/// `T-19-86`'s subject too and a rule written to satisfy it would turn
/// `tests/envelope_command_position.rs:550` and
/// `tests/envelope_config_resolution.rs:1539-1543` permanently red.
fn draws_a_reparsed_value_carrier(command: &str) -> bool {
    quoted_config_assignments(command).iter().any(|a| {
        config_key_names_the_alias_section(&a.key)
            && !a.value.is_empty()
            && !config_value_is_a_shell_alias_body(&a.value)
    }) || persisted_config_write(command).is_some_and(|(key, value)| {
        config_key_names_the_alias_section(&key)
            && !value.is_empty()
            && !config_value_is_a_shell_alias_body(&value)
    })
}

/// One named class and the predicate that decides whether a spliced command draws
/// it.
type ConfigResolutionClass = (&'static str, fn(&str) -> bool);

/// The SIX classes of the CONFIG-RESOLUTION axis, named once so the per-alphabet
/// floor, the per-class floor and the counted floor all count the same thing.
///
/// **The sixth joined this axis rather than founding a fifth**, because audit 8's
/// finding is that the region moved and not the axis — see this section's header.
const CONFIG_RESOLUTION_CLASSES: &[ConfigResolutionClass] = &[
    (
        "a command-line carrier of a config indirection",
        draws_a_config_indirection_carrier,
    ),
    (
        "a command-line carrier of a confined assignment",
        draws_a_confined_config_carrier,
    ),
    (
        "an environment carrier of configuration",
        draws_an_environment_config_carrier,
    ),
    (
        "a case-varied spelling of an indirection",
        draws_a_case_varied_indirection,
    ),
    (
        "an option-carrier delivery of an indirection",
        draws_an_option_carrier_indirection,
    ),
    (
        "a carrier delivered inside a config value the guard confines",
        draws_a_reparsed_value_carrier,
    ),
];

/// Whether a spliced command draws ANY of the six.
fn carries_a_config_resolution_class(command: &str) -> bool {
    CONFIG_RESOLUTION_CLASSES
        .iter()
        .any(|(_, predicate)| predicate(command))
}

#[test]
fn the_corpus_can_draw_every_one_of_the_five_config_resolution_classes() {
    // **The degenerate-proofing, ASSERTED rather than described**, in the shape
    // `the_corpus_can_draw_every_one_of_the_seven_unreadable_classes` and
    // `the_corpus_can_draw_every_one_of_the_five_callee_grammar_classes` already
    // use. If any pair below collapsed, the floors would be satisfiable by an
    // alphabet that cannot generate the cells this round is about — which is
    // exactly how the last four plan-check rounds each found a live cell.
    //
    // **GREEN today and after.** It drives no guard call and no production change
    // can move it.

    // -- class 1 versus class 2. **This pair IS the resolution question.**
    assert!(
        draws_a_config_indirection_carrier("git -c include.path=/tmp/e status"),
        "`git -c include.path=/tmp/e status` IS a config indirection: measured against real \
         git with the envelope's own injection as the control, it makes \
         `git config --get core.hooksPath` print `/INCLUDE_WINS` where the injection alone \
         prints `/ENV_WINS`"
    );
    assert!(
        !draws_a_confined_config_carrier("git -c include.path=/tmp/e status"),
        "`git -c include.path=/tmp/e status` must NOT satisfy the CONFINED class. A corpus \
         that conflated the two could not fail on `T-19-103`, whose entire mechanism is an \
         assignment whose effect is NOT bounded to the key it names."
    );
    assert!(
        draws_a_confined_config_carrier("git -c user.name=x status"),
        "`git -c user.name=x status` IS a confined assignment — its effect is bounded to the \
         key it names"
    );
    assert!(
        !draws_a_config_indirection_carrier("git -c user.name=x status"),
        "`git -c user.name=x status` must NOT satisfy the INDIRECTION class. **This is the \
         split that tells a resolution MODEL apart from a blanket refusal of anything spelled \
         `-c`**, and a corpus that collapsed it could not fail on a rule that refused \
         `git -c user.name=\"$NAME\" commit -m x` — the row pinned PERMITTED in \
         `tests/envelope_config_resolution.rs` as this axis's `ls {{git,svn}}-repo`."
    );

    // -- class 3, which involves NO command line at all.
    assert!(
        draws_an_environment_config_carrier(
            "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/x'\" git status"
        ),
        "`GIT_CONFIG_PARAMETERS=… git status` IS an environment carrier"
    );
    for not_env in [
        "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/x'\" git status",
        "env GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/x'\" git status",
    ] {
        assert!(
            !draws_a_config_indirection_carrier(not_env),
            "`{not_env}` must satisfy NEITHER command-line class: there is no `-c` on the \
             line at all. **That is why this class cannot live in any of the three existing \
             axes** — all three walk argv words and this one is read from the \
             assignment-prefix region."
        );
        assert!(!draws_a_confined_config_carrier(not_env));
        assert!(draws_an_environment_config_carrier(not_env));
    }

    // -- class 4, and the case distinction that is load-bearing.
    assert!(
        draws_a_case_varied_indirection("git -c INCLUDE.PATH=/tmp/e status"),
        "`INCLUDE.PATH` IS a case-varied spelling — git folds the SECTION and the VARIABLE, \
         measured `/INCLUDE_WINS`"
    );
    assert!(
        !draws_a_case_varied_indirection("git -c include.path=/tmp/e status"),
        "the already-lower-case spelling must NOT satisfy class 4, or class 4 is a second \
         name for class 1 and its floor is satisfiable without a single case-varied spelling \
         being drawn"
    );

    // -- class 5, the second carrier, in both spellings.
    for spelling in [
        "git --config-env=include.path=V status",
        "git --config-env include.path=V status",
    ] {
        assert!(
            draws_an_option_carrier_indirection(spelling),
            "`{spelling}` IS an option-carrier delivery of an indirection"
        );
    }
    assert!(
        !draws_an_option_carrier_indirection("git -c include.path=/tmp/e status"),
        "a `-c` delivery must NOT satisfy class 5, or class 5 is a second name for class 1"
    );

    // -- **class 6, and the REGION-NOT-AXIS finding asserted rather than
    //    described.** Both deliveries satisfy it.
    for delivery in [
        "git -c alias.q=\"-c include.path=/tmp/e status\" q",
        "git config alias.q \"-c include.path=/tmp/e status\"",
    ] {
        assert!(
            draws_a_reparsed_value_carrier(delivery),
            "`{delivery}` IS a carrier delivered inside a config VALUE the guard confines. \
             Measured against real git with the envelope's own injection as the control, it \
             makes `git config --get core.hooksPath` print `/INCLUDE_WINS` where the injection \
             alone prints `/ENV_WINS` — through a key that names the ordinary bounded section \
             `alias`."
        );

        // It must satisfy NONE of the four INDIRECTION-shaped classes, or class 6
        // would be a second name for one of them and its floor would be satisfiable
        // without a single re-parsed value being drawn.
        for (class, predicate) in [
            ("class 1 (indirection)", draws_a_config_indirection_carrier as fn(&str) -> bool),
            ("class 3 (environment)", draws_an_environment_config_carrier),
            ("class 4 (case-varied)", draws_a_case_varied_indirection),
            ("class 5 (option-carrier)", draws_an_option_carrier_indirection),
        ] {
            assert!(
                !predicate(delivery),
                "`{delivery}` must NOT satisfy {class}. Its key names `alias`, not `include` or \
                 `includeIf`, and there is no `GIT_CONFIG*` assignment prefix on the line. If it \
                 did, class 6 would be a second name for an existing class."
            );
        }
    }

    // -- **THE FINDING ITSELF, ASSERTED POSITIVELY — AND THE TWO DELIVERIES FALL
    //    DIFFERENTLY, WHICH IS SHARPER THAN THE OVERLAP ALONE.**
    //
    //    In the `-c` DELIVERY the assignment IS read by round 8's region and IS
    //    confined: `alias.q` correctly answers `false` to
    //    `config_key_names_an_indirection_section`, so class 2 draws it. **That
    //    overlap IS `T-19-108`** — the guard read the assignment, bounded it
    //    correctly, and handed the value on.
    assert!(
        draws_a_confined_config_carrier("git -c alias.q=\"-c include.path=/tmp/e status\" q"),
        "\n\n**THE REGION-NOT-AXIS FINDING HAS BEEN ERASED.**\n\n\
         The `-c` delivery of a re-parsed-value carrier MUST satisfy the CONFINED class. \
         `alias.q` names the section `alias`, which is not an indirection, so round 8's clause \
         correctly CONFINES the assignment and lets it through — and the carrier rides inside \
         the VALUE into a position `scan_leading` never reads. **That overlap IS `T-19-108`**, \
         and a corpus in which class 2 stopped drawing it would have quietly re-modelled the \
         region as an axis and lost the distinction audit 8 drew."
    );

    //    In the PERSISTED delivery the assignment is not read by that region AT ALL
    //    — `scan_leading` walks leading OPTIONS and stops at the verb, and `config`
    //    is the verb. So class 2 correctly does NOT draw it. **The two deliveries
    //    are therefore open for two different reasons**: one was confined and let
    //    through, the other was never in the confining region. `19-25` must reach
    //    BOTH, and a rule written only inside `scan_leading` closes only the first.
    assert!(
        !draws_a_confined_config_carrier("git config alias.q \"-c include.path=/tmp/e status\""),
        "\n\n**THE TWO DELIVERIES HAVE BEEN CONFLATED.**\n\n\
         The PERSISTED delivery must NOT satisfy the confined class: `scan_leading` walks \
         leading OPTIONS and stops at the verb, and here `config` IS the verb — so round 8's \
         region never sees this assignment at all, not even to confine it.\n\n\
         **That is why the two deliveries are open for two DIFFERENT reasons**, and why \
         `19-25` needs the question asked at the `git config` KEY OPERAND (where \
         `classify_config` already reads with `is_hooks_path_key`) as well as at the `-c` \
         carrier. A rule written only inside `scan_leading` closes only the first delivery and \
         leaves every row of the persisted arm at exit 0."
    );

    // -- and NO existing class's representative may satisfy class 6.
    for representative in [
        "git -c include.path=/tmp/e status",
        "git -c user.name=x status",
        "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/x'\" git status",
        "git -c INCLUDE.PATH=/tmp/e status",
        "git --config-env=include.path=V status",
    ] {
        assert!(
            !draws_a_reparsed_value_carrier(representative),
            "`{representative}` must NOT satisfy class 6. None of these keys names the `alias` \
             section, and git re-parses nothing from their values. If one did, class 6's floor \
             would be satisfiable without a single re-parsed value being generated — which is \
             `T-19-105`'s failure mode reproduced inside the repair for it."
        );
    }

    // -- **THE `!` NEGATIVE CONTROL, and it is the most load-bearing line in this
    //    block.** A `!`-bodied alias is a whole command line handed to a governed
    //    program as DATA and run in a CHILD that inherits the envelope's injection
    //    (measured `/ENV_WINS`). That is `T-19-86`'s class, OPEN at `high` and out
    //    of scope. **If class 6 ever drew it, a rule written to satisfy class 6
    //    would turn `tests/envelope_command_position.rs:550` and
    //    `tests/envelope_config_resolution.rs:1539-1543` PERMANENTLY RED in files
    //    `19-25` may not edit.**
    for shell_bodied in [
        "git -c alias.q='!git push' q",
        "git -c alias.p='!git push --force origin main' p",
        "git config alias.p \"!git push --force origin HEAD:refs/heads/main\"",
    ] {
        assert!(
            !draws_a_reparsed_value_carrier(shell_bodied),
            "\n\n**A `!`-BODIED ALIAS SATISFIES CLASS 6.**\n\n`{shell_bodied}`\n\n\
             Git's rule is the FIRST BYTE and nothing else, measured in nine spellings in \
             `tests/envelope_reparsed_value.rs` against `git version 2.43.0`: a `!` body is \
             handed to a SHELL and run in a CHILD that INHERITS the envelope's \
             `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet (`/ENV_WINS`), while a non-`!` body is \
             re-parsed by git IN-PROCESS at command-line precedence (`/INCLUDE_WINS`).\n\n\
             A `!` body is `T-19-86` — OPEN at `high` by explicit user scoping decision — and a \
             rule written to satisfy this class would turn \
             `tests/envelope_command_position.rs:550` and \
             `tests/envelope_config_resolution.rs:1539-1543` PERMANENTLY RED in files `19-25` \
             may not edit. **This is `19-18`'s `{{v}}>` blocker and `19-22`'s `-c a=b` seam one \
             REGION over.**"
        );
    }

    // -- THE QUOTING-AND-POSITION CONTROL. A config-shaped word that is neither a
    //    carrier nor in the scan's view must satisfy NO class, or the floors
    //    become satisfiable by a row the rules must NEVER touch.
    for (class, predicate) in CONFIG_RESOLUTION_CLASSES {
        for control in [
            "git commit -m \"include.path=/tmp/e\"",
            "git log --grep='include.path=/tmp/e'",
            "git push -c include.path=/tmp/e --force origin main",
        ] {
            assert!(
                !predicate(control),
                "`{control}` must satisfy NO config-resolution class, and it satisfies \
                 `{class}`.\n\n\
                 The first is a config-shaped word inside a quoted OPERAND — measured at exit \
                 0 today and it must stay there. The second is quoted. The third sits AFTER \
                 the verb, which `scan_leading` does not read at all. A predicate that \
                 counted any of them would make every floor below satisfiable by a row the \
                 rules must never touch, and would put the fix's blast radius outside the \
                 region it is allowed to decide in."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 16a. The THREE alphabets — and the split between them is the SEAM FENCE
//      rather than a tidy-up
// ---------------------------------------------------------------------------

/// The ONLY alphabet the invariance-preserving arm draws.
///
/// Entries are spliced **between the governed program and its decision words**,
/// because that is the position `scan_leading` reads. **Every entry is
/// VERDICT-PRESERVING** — measured permitted on a permitted base and measured at
/// the base's own identifier on a refused base, before AND after. Measured at
/// this section's base commit over all 70 generated cases: 40 permits and 30
/// `force_push_blocked`, walks EMPTY throughout.
///
/// **`-c a=b` is a MINIMUM entry and its presence is asserted BY NAME below.**
/// `tests/envelope_wrapper_class.rs:5197` defines
/// `CALLEE_KNOWN_LEADING_PREFIX: &str = "-c a=b"` and round 7's ENTIRE
/// callee-grammar generative property is spliced behind it; real git RUNS
/// `git -c a=b version` at rc 0 and errors only when something READS the key. A
/// rule that refused a key it cannot decompose into a section would refuse
/// `-c a=b` and turn round 7's whole property **permanently red in a file `19-23`
/// may not edit**, which is `19-18`'s `{v}>` blocker one axis over.
///
/// **`-c notinclude.path=…` and `-c includepath=…` are the DISCRIMINATION
/// CONTROLS** — this round's `--signed no`. A rule written as
/// `key.contains("include")` turns both red; only a rule that compares the
/// SECTION keeps them green.
/// **Round 9's two additions are `!`-BODIED ALIASES, and they belong HERE because
/// their verdict does NOT move.** Measured at round 9's base commit over all 28
/// generated cases: verdict-preserving on every base, in both splices, with ZERO
/// mismatches. A `!` body is handed to a SHELL and run as a CHILD that INHERITS the
/// envelope's `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet — measured `/ENV_WINS`
/// against the control — so layer 3 is intact for it and `19-25` must not refuse
/// it. **This is the alphabet the invariance arm draws, so putting them here is
/// what makes the corpus able to FAIL on a rule that refuses them**, which is
/// exactly the rule that would turn `tests/envelope_command_position.rs:550` and
/// `tests/envelope_config_resolution.rs:1539-1543` permanently red.
const CONFIG_CONFINED_CARRIERS: &[&str] = &[
    "-c user.name=x",
    "-c core.pager=cat",
    "-c a=b",
    "-c notinclude.path=/tmp/e",
    "-c includepath=/tmp/e",
    "-c alias.p='!git push --force origin main'",
    "-c alias.b='!git status'",
];

/// A SEPARATE alphabet with its OWN fail-closed property, spliced at the same
/// position.
///
/// **Kept out of `CONFIG_CONFINED_CARRIERS`, and out of the invariance arm,
/// because its entries are NOT verdict-preserving — this is `19-18`'s `{v}>`
/// blocker and `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split, one axis over, and it
/// is the single most likely way this seam breaks.** The invariance arm asserts
/// that a spliced verdict EQUALS the unspliced one. **An indirection carrier is
/// refused after `19-23` even on a PERMITTED base** — `git -c include.path=…
/// status` is measured at exit 0 today and is refused after — so it would be
/// STRICTER than its base and would turn that property permanently red in a file
/// `19-23` may not edit.
///
/// Every entry was measured against the REAL git binary and OUTRANKS the
/// envelope's own `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection: control
/// `/ENV_WINS`, each of these `/INCLUDE_WINS`. **A carrier that does not outrank
/// the injection is not an indirection and is not a reproducer.**
const CONFIG_INDIRECTION_CARRIERS: &[&str] = &[
    "-c include.path=/tmp/evil.cfg",
    "-c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg",
    "-c INCLUDE.PATH=/tmp/evil.cfg",
    "--config-env=include.path=EVILVAR",
    "--config-env include.path=EVILVAR",
];

/// A SEPARATE alphabet with the same fail-closed property, spliced as an
/// ASSIGNMENT PREFIX rather than between the program and its decision words —
/// because that is where `resolve_program`'s step-1 check reads.
///
/// The three spellings mirror the three spellings of the paired discriminator
/// `GIT_CONFIG_COUNT=0` exactly, and that is what makes every post-fix verdict on
/// this alphabet DERIVABLE: all three of the discriminator's spellings are
/// measured at exit 2 `hook_bypass_blocked` TODAY, on both a refused and a
/// permitted base, for a key that IS in `ENVELOPE_ENV_KEYS`.
///
/// **`GIT_CONFIG_PARAMETERS` is absent from `ENVELOPE_ENV_KEYS`, git EXPORTS it,
/// and it OUTRANKS the injection** (measured `/PARAM_WINS`). Not
/// verdict-preserving, for the same reason as the alphabet above.
const CONFIG_ENV_CARRIERS: &[&str] = &[
    "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\"",
    "env GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\"",
    "export GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\";",
];

/// A FOURTH alphabet with its OWN fail-closed property, spliced at the same
/// position as the first two — **round 9's, and the one `T-19-108` lives in.**
///
/// Every entry names an ordinary BOUNDED section (`alias`) whose VALUE git
/// RE-PARSES as a git command line **including its own leading options**. Round 8's
/// clause correctly answers `false` to `config_key_names_an_indirection_section`
/// for every one of these keys, CONFINES the assignment, and lets the carrier
/// through into a region `scan_leading` never reads.
///
/// **Kept OUT of `CONFIG_CONFINED_CARRIERS` and out of the invariance arm** for the
/// reason this section's doc already gives for the other two fail-closed alphabets:
/// a re-parsed-value carrier is REFUSED after `19-25` even on a PERMITTED base —
/// measured exit 0 today on all 40 permitted-base cases — so it would be STRICTER
/// than its base and would turn the invariance property permanently red in a file
/// `19-25` may not edit. **This is `19-18`'s `{v}>` blocker, `19-20`'s
/// `GIT_GLOBAL_UNKNOWN_OPTIONS` split and `19-22`'s indirection/confined split, one
/// REGION over.**
///
/// **NO ENTRY MAY HAVE A `!` FIRST BYTE**, asserted mechanically by the SHELL-ALIAS
/// FENCE below. Every entry was measured against the REAL git binary and outranks
/// the envelope's injection: control `/ENV_WINS`, each of these `/INCLUDE_WINS` or
/// `/ALIAS_WINS`.
const CONFIG_REPARSED_VALUE_CARRIERS: &[&str] = &[
    "-c alias.q=\"-c include.path=/tmp/evil.cfg status\"",
    "-c alias.z=\"-c core.hooksPath=/dev/null status\"",
    "-c ALIAS.q=\"-c include.path=/tmp/evil.cfg status\"",
    "--config-env=alias.q=EVILBODY",
    "-c alias.st=status",
];

/// The PERSISTED delivery of the same class, as WHOLE COMMANDS rather than splice
/// fragments — because `git config alias.<n> "<body>"` has no base to be spliced
/// into.
///
/// **This is a SECOND REGION of the SAME class, not a second class.**
/// `classify_config` already reads this exact operand with `is_hooks_path_key` and
/// refuses `git config core.hooksPath /dev/null` at `hook_bypass_blocked` today, so
/// asking the alias question there is the SAME decision region.
///
/// **The persistence asymmetry is why this arm must exist at all.** A persisted
/// `git config include.path <evil>` is INERT — measured at repo-local and at
/// GLOBAL under the envelope's own config posture, it resolves `/ENV_WINS` because
/// the injection OUTRANKS every repository-side level. **An alias is different in
/// kind: it does not have to WIN a precedence contest, it only has to EXIST**, and
/// both persistence levels were measured live at `/INCLUDE_WINS`.
const CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS: &[&str] = &[
    "git config alias.q \"-c include.path=/tmp/evil.cfg status\"",
    "git config alias.z \"-c core.hooksPath=/dev/null status\"",
    "git config ALIAS.q \"-c include.path=/tmp/evil.cfg status\"",
    "git config --global alias.q \"-c include.path=/tmp/evil.cfg status\"",
    "git config alias.st status",
];

/// Where a command-line alphabet entry is spliced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConfigSplice {
    /// Immediately after the governed program — the first thing `scan_leading`
    /// reads.
    ImmediatelyAfterTheProgram,
    /// After a leading option the guard ALREADY knows.
    ///
    /// **The scan is a LOOP, so the gap is not confined to the first slot.** The
    /// known prefix is `CALLEE_KNOWN_LEADING_PREFIX` — `-c a=b` — which makes the
    /// dotless fence below directly load-bearing for this axis too: every case at
    /// this splice begins `git -c a=b …`.
    AfterAKnownLeadingOption,
}

const CONFIG_SPLICES: &[ConfigSplice] = &[
    ConfigSplice::ImmediatelyAfterTheProgram,
    ConfigSplice::AfterAKnownLeadingOption,
];

/// Bases whose UNWRAPPED form is REFUSED. Measured: all three at exit 2
/// `force_push_blocked`, walks EMPTY.
const CONFIG_REFUSED_BASES: &[&str] = &[
    "git push --force origin main",
    "git stash",
    "git update-ref -d refs/heads/main",
];

/// Bases whose UNWRAPPED form is PERMITTED.
///
/// **THE PERMITTED-BASE FENCE'S SUBJECT, and this axis's whole constraint.** The
/// entire content of `T-19-103` is that an indirection disarms layer 3 on a line
/// layer 2 LETS THROUGH, so a property drawing only refused bases would be GREEN
/// before `19-23` and would certify nothing. Measured at this section's base
/// commit: `git -c include.path=/tmp/evil.cfg push --force origin main` is
/// ALREADY exit 2 `force_push_blocked` for its VERB, as are its `bash -lc`,
/// `echo hi &&`, `>/dev/null` and `--attr-source HEAD` compositions — five
/// CONTROLS, not reproducers.
///
/// The three named bases are the ones audit 7 used. `git push origin
/// refs/heads/gsd-auto/alpha/w` names its refspec, so
/// `push_needs_resolved_dests` answers FALSE for it and no repository is consulted
/// — this section stays repository-free (D-35, `T-19-80`).
const CONFIG_PERMITTED_BASES: &[&str] = &[
    "git status",
    "git commit -m x",
    "git push origin refs/heads/gsd-auto/alpha/w",
    "git log --oneline",
];

/// Splice one command-line alphabet entry into `base`.
fn config_case(base: &str, entry: &str, splice: ConfigSplice) -> String {
    let words: Vec<&str> = base.split_whitespace().collect();
    let program = words[0];
    let rest = words[1..].join(" ");
    match splice {
        ConfigSplice::ImmediatelyAfterTheProgram => format!("{program} {entry} {rest}"),
        ConfigSplice::AfterAKnownLeadingOption => {
            format!("{program} {CALLEE_KNOWN_LEADING_PREFIX} {entry} {rest}")
        }
    }
}

/// Splice one ENVIRONMENT alphabet entry into `base` — as an assignment PREFIX,
/// which is a different position and therefore a different generator.
fn config_env_case(base: &str, entry: &str) -> String {
    format!("{entry} {base}")
}

/// Every command-line case this axis generates, as `(base, splice, label,
/// command)`.
///
/// One function so the counting floor and the guard-driven properties count
/// exactly the same thing — a second enumeration would be a second thing to keep
/// in step.
fn config_cases(
    bases: &[&'static str],
    alphabet: &[&str],
) -> Vec<(&'static str, ConfigSplice, String, String)> {
    let mut cases = Vec::new();
    for base in bases.iter().copied() {
        for splice in CONFIG_SPLICES {
            for entry in alphabet {
                cases.push((
                    base,
                    *splice,
                    format!("{splice:?}({entry})"),
                    config_case(base, entry, *splice),
                ));
            }
        }
    }
    cases
}

/// Every ENVIRONMENT case, as `(base, label, command)`.
fn config_env_cases(bases: &[&'static str]) -> Vec<(&'static str, String, String)> {
    let mut cases = Vec::new();
    for base in bases.iter().copied() {
        for entry in CONFIG_ENV_CARRIERS {
            cases.push((
                base,
                format!("AssignmentPrefix({entry})"),
                config_env_case(base, entry),
            ));
        }
    }
    cases
}

// ---------------------------------------------------------------------------
// 16b. The floors — per ALPHABET, per CLASS, and COUNTED over generated cases
// ---------------------------------------------------------------------------

/// The arithmetic, STATED rather than guessed and RE-DERIVED for round 9, because
/// audit 5 found `19-16` set a floor of 50 against a maximum of 40 by construction.
///
/// There are 3 refused bases and 4 permitted bases = **7 bases**, and 2 splices.
///
/// * confined       — 7 entries x 2 splices x 7 bases = **98** cases
/// * indirection    — 5 entries x 2 splices x 7 bases = **70** cases
/// * reparsed value — 5 entries x 2 splices x 7 bases = **70** cases
/// * environment    — 3 entries x 1 prefix position x 7 bases = **21** cases
/// * persisted      — 5 WHOLE COMMANDS, which have no base and no splice = **5**
/// * total = 98 + 70 + 70 + 21 + 5 = **264** cases
///
/// Split by the arm the base is in (the persisted arm is in NEITHER, because its
/// entries are whole commands rather than splices):
///
/// * refused bases   — (7 x 2 + 5 x 2 + 5 x 2) x 3 + 3 x 3 = 102 + 9 = **111**
/// * permitted bases — (7 x 2 + 5 x 2 + 5 x 2) x 4 + 3 x 4 = 136 + 12 = **148**
/// * 111 + 148 + 5 persisted = **264**
///
/// Slots are `(base, splice position)` pairs: 7 x 2 command-line slots plus 7
/// assignment-prefix slots = **21**. The persisted arm contributes NO slot, and
/// that is deliberate rather than an omission — it is not spliced anywhere.
///
/// The floors are EXACT equalities, so losing one case turns them red. Nothing in
/// `src/` can move them: they are a pure function of the alphabets in this file.
const CONFIG_RESOLUTION_CASES: usize = 264;
const CONFIG_RESOLUTION_REFUSED_CASES: usize = 111;
const CONFIG_RESOLUTION_PERMITTED_CASES: usize = 148;
const CONFIG_RESOLUTION_PERSISTED_CASES: usize = 5;
const CONFIG_RESOLUTION_SLOTS: usize = 21;
const MIN_CONFIG_RESOLUTION_CLASSES: usize = 6;
const MIN_CONFIG_CONFINED_CARRIERS: usize = 7;
const MIN_CONFIG_INDIRECTION_CARRIERS: usize = 5;
const MIN_CONFIG_ENV_CARRIERS: usize = 3;
const MIN_CONFIG_REPARSED_VALUE_CARRIERS: usize = 5;
const MIN_CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS: usize = 5;

/// The per-class counts over all 264 generated commands, derived from the
/// alphabets and the splice sets:
///
/// * class 1 (indirection) — the 5 indirection entries at each of 14 command-line
///   slots = **70**. No confined, reparsed, environment or persisted case names an
///   `include`/`includeIf` section.
/// * class 2 (confined) — the 7 confined entries at each of 14 slots = 98; PLUS
///   the `-c a=b` of `CALLEE_KNOWN_LEADING_PREFIX`, which is itself a confined
///   assignment, in every `AfterAKnownLeadingOption` case of the INDIRECTION
///   alphabet: 5 entries x 7 bases = 35; PLUS all 70 REPARSED-value cases, whose
///   `alias.*` keys are precisely keys round 8's clause CONFINES — **which is the
///   whole of `T-19-108` and is asserted positively in the degenerate-proofing
///   block above.** 98 + 35 + 70 = **203**
/// * class 3 (environment) — the 3 environment entries at each of 7 prefix slots
///   = **21**
/// * class 4 (case-varied) — the 2 case-varied INDIRECTION entries
///   (`-c INCLUDE.PATH=…` and `-c includeIf.gitdir:…`, whose SECTION is not
///   already lower case) at each of 14 slots = **28**. `-c ALIAS.q=…` is
///   case-varied but is not an INDIRECTION, so it correctly does not count here.
/// * class 5 (option-carrier) — the 2 `--config-env` INDIRECTION entries at each of
///   14 slots = **28**. `--config-env=alias.q=EVILBODY` is an option-carrier
///   delivery but not of an indirection, so it correctly does not count here.
/// * class 6 (re-parsed value) — the 5 reparsed entries at each of 14 slots = 70,
///   PLUS the 5 persisted whole commands = **75**. The two `!`-bodied confined
///   entries do NOT count, by the one-byte fence.
const CONFIG_RESOLUTION_CLASS_COUNTS: &[(&str, usize)] = &[
    ("a command-line carrier of a config indirection", 70),
    ("a command-line carrier of a confined assignment", 203),
    ("an environment carrier of configuration", 21),
    ("a case-varied spelling of an indirection", 28),
    ("an option-carrier delivery of an indirection", 28),
    ("a carrier delivered inside a config value the guard confines", 75),
];

#[test]
fn every_alphabet_this_round_widens_can_draw_a_fact_about_what_the_verb_runs_under() {
    // **The direct mechanical inverse of audit 7's `T-19-105`.** The auditor
    // established the defect by grepping `src/` and `tests/` for `include.path`,
    // `includeIf` and `GIT_CONFIG_PARAMETERS` and finding NOTHING AT ALL; this
    // asserts the repaired fact, so narrowing an alphabet back turns this red
    // instead of quietly restoring a corpus that cannot fail on its own class.
    //
    // The predicate is evaluated on a REPRESENTATIVE SPLICED COMMAND rather than
    // on the bare entry, the way sections 14 and 15's are, because these entries
    // are splice FRAGMENTS.
    //
    // **GREEN today and after.**
    assert!(
        CONFIG_RESOLUTION_CLASSES.len() >= MIN_CONFIG_RESOLUTION_CLASSES,
        "the config-resolution axis must name at least {MIN_CONFIG_RESOLUTION_CLASSES} classes"
    );
    assert!(
        CONFIG_CONFINED_CARRIERS.len() >= MIN_CONFIG_CONFINED_CARRIERS,
        "`CONFIG_CONFINED_CARRIERS` must carry at least {MIN_CONFIG_CONFINED_CARRIERS} \
         entries. **The correct response to a red here is to RESTORE entries, never to lower \
         this floor.** `T-19-105` is `T-19-76`'s failure mode for the EIGHTH consecutive \
         round, and for the THIRD round running the gap moved AXIS rather than one cell over."
    );
    assert!(
        CONFIG_INDIRECTION_CARRIERS.len() >= MIN_CONFIG_INDIRECTION_CARRIERS,
        "`CONFIG_INDIRECTION_CARRIERS` must carry at least \
         {MIN_CONFIG_INDIRECTION_CARRIERS} entries"
    );
    assert!(
        CONFIG_ENV_CARRIERS.len() >= MIN_CONFIG_ENV_CARRIERS,
        "`CONFIG_ENV_CARRIERS` must carry at least {MIN_CONFIG_ENV_CARRIERS} entries — one \
         per environment spelling the paired `GIT_CONFIG_COUNT` discriminator is measured in"
    );
    assert!(
        CONFIG_REPARSED_VALUE_CARRIERS.len() >= MIN_CONFIG_REPARSED_VALUE_CARRIERS,
        "`CONFIG_REPARSED_VALUE_CARRIERS` must carry at least \
         {MIN_CONFIG_REPARSED_VALUE_CARRIERS} entries. **The correct response to a red here is \
         to RESTORE entries, never to lower this floor.** `T-19-105` is `T-19-76`'s failure \
         mode for the NINTH consecutive round, and for the SECOND round running the gap was one \
         REGION over on the SAME axis rather than one axis further out."
    );
    assert!(
        CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS.len()
            >= MIN_CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS,
        "`CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS` must carry at least \
         {MIN_CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS} entries — the SECOND REGION of the same \
         class. A persisted alias does not have to WIN a precedence contest, it only has to \
         EXIST, and both persistence levels were measured live at `/INCLUDE_WINS`."
    );

    // -- every entry of every alphabet must DRAW a class when spliced.
    for entry in CONFIG_CONFINED_CARRIERS
        .iter()
        .chain(CONFIG_INDIRECTION_CARRIERS.iter())
        .chain(CONFIG_REPARSED_VALUE_CARRIERS.iter())
    {
        let spliced = config_case(
            "git push --force origin main",
            entry,
            ConfigSplice::ImmediatelyAfterTheProgram,
        );
        assert!(
            carries_a_config_resolution_class(&spliced),
            "alphabet entry `{entry}` draws NO config-resolution class when spliced between \
             the program and its decision words (`{spliced}`).\n\n\
             An alphabet entry that cannot DRAW a class is an entry whose property cannot \
             FAIL on one, and every case generated from it certifies a claim about a class it \
             could never have exercised. It is `T-19-76`'s failure mode for the EIGHTH \
             consecutive round, after `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99` and \
             `T-19-101`.\n\n\
             The correct response is to RESTORE the entry, never to delete this floor."
        );
    }
    for entry in CONFIG_ENV_CARRIERS {
        let spliced = config_env_case("git status", entry);
        assert!(
            draws_an_environment_config_carrier(&spliced),
            "`CONFIG_ENV_CARRIERS` entry `{entry}` draws NO environment class when spliced as \
             an assignment prefix (`{spliced}`)"
        );
    }
    for entry in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        assert!(
            draws_a_reparsed_value_carrier(entry),
            "`CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS` entry `{entry}` draws NO \
             re-parsed-value class. It is a WHOLE COMMAND rather than a splice fragment, and \
             `persisted_config_write` must be able to read its key operand — the same operand \
             `classify_config` reads with `is_hooks_path_key`. An entry that draws no class is \
             an entry whose property cannot FAIL on one."
        );
    }

    // ---------------------------------------------------------------------
    // **THE SHELL-ALIAS FENCE** — the mechanical assertion round 9 exists to
    // add, written in the shape the DOTLESS fence below already uses.
    // ---------------------------------------------------------------------
    for entry in CONFIG_REPARSED_VALUE_CARRIERS {
        for assignment in quoted_config_assignments(&config_case(
            "git status",
            entry,
            ConfigSplice::ImmediatelyAfterTheProgram,
        )) {
            assert!(
                !config_value_is_a_shell_alias_body(&assignment.value),
                "\n\nA `!`-BODIED ALIAS IS IN `CONFIG_REPARSED_VALUE_CARRIERS`: `{entry}` \
                 delivers the value `{}`.\n\n\
                 **GIT'S RULE IS THE FIRST BYTE AND NOTHING ELSE**, measured in nine spellings \
                 in `tests/envelope_reparsed_value.rs` against `git version 2.43.0` with the \
                 envelope's own injection as the control: a body whose first byte is `!` is \
                 handed to a SHELL and run as a CHILD that INHERITS the \
                 `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet (`/ENV_WINS`, so layer 3 is \
                 INTACT), while a non-`!` body is re-parsed by git IN-PROCESS at command-line \
                 precedence (`/INCLUDE_WINS`).\n\n\
                 **`tests/envelope_command_position.rs:550` and \
                 `tests/envelope_config_resolution.rs:1539-1543` BOTH pin a `!`-bodied alias \
                 PERMITTED as a registered `T-19-86` row, and `19-25` may edit NEITHER FILE.** \
                 A rule written to refuse this entry would turn both PERMANENTLY RED, and \
                 `T-19-86` is OPEN at `high` by explicit user scoping decision.\n\n\
                 The correct response is to keep `!`-bodied aliases in \
                 `CONFIG_CONFINED_CARRIERS`, where their verdict-preserving behaviour is \
                 measured, never to edit either pinning file. **This is `19-18`'s `{{v}}>` \
                 blocker and `19-22`'s `-c a=b` seam one REGION over.**",
                assignment.value
            );
        }
    }
    for entry in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        let (_, value) = persisted_config_write(entry)
            .unwrap_or_else(|| panic!("`{entry}` must parse as a `git config <key> <value>` write"));
        assert!(
            !config_value_is_a_shell_alias_body(&value),
            "a `!`-bodied alias is in `CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS`: `{entry}`. \
             The same one-byte fence applies to the persisted region, and \
             `tests/envelope_config_resolution.rs:1539-1543` pins exactly this shape PERMITTED."
        );
    }
    assert!(
        CONFIG_CONFINED_CARRIERS.iter().any(|entry| {
            quoted_config_assignments(&config_case(
                "git status",
                entry,
                ConfigSplice::ImmediatelyAfterTheProgram,
            ))
            .iter()
            .any(|a| config_value_is_a_shell_alias_body(&a.value))
        }),
        "\n\n**AT LEAST ONE `!`-BODIED ALIAS MUST BE AN ENTRY OF \
         `CONFIG_CONFINED_CARRIERS`**, and must therefore be asserted VERDICT-PRESERVING by the \
         invariance arm below.\n\n\
         A `!` body runs in a CHILD that inherits the envelope's injection, so its verdict does \
         NOT move — measured at round 9's base commit over all 28 generated cases with ZERO \
         mismatches. **Asserting its presence here BY MECHANISM is what makes the corpus able \
         to FAIL on a rule that refuses it**, which is exactly the rule that would turn \
         `tests/envelope_command_position.rs:550` and \
         `tests/envelope_config_resolution.rs:1539-1543` permanently red in files `19-25` may \
         not edit. Without this assertion the fence above could be satisfied by removing every \
         `!`-bodied entry from the file entirely — a corpus that cannot say what it means."
    );

    // ---------------------------------------------------------------------
    // **THE DOTLESS FENCE** — the mechanical assertion that would otherwise
    // have blocked this round, written in the shape section 14b's `{v}>`
    // exclusion already uses.
    // ---------------------------------------------------------------------
    for entry in CONFIG_INDIRECTION_CARRIERS {
        for assignment in config_assignments(&config_case(
            "git status",
            entry,
            ConfigSplice::ImmediatelyAfterTheProgram,
        )) {
            assert!(
                assignment.key.contains('.'),
                "\n\nA DOTLESS KEY IS IN `CONFIG_INDIRECTION_CARRIERS`: `{entry}` delivers \
                 the key `{}`.\n\n\
                 A `-c` key with no `.` at all names NO CONFIG SECTION and therefore cannot \
                 be an indirection — real git says so: `git -c a=b config --get a` answers \
                 `error: key does not contain a section: a`, while `git -c a=b version` RUNS \
                 at rc 0.\n\n\
                 **`CALLEE_KNOWN_LEADING_PREFIX` at tests/envelope_wrapper_class.rs:5197 is \
                 `\"-c a=b\"`, and round 7's ENTIRE callee-grammar generative property is \
                 spliced behind it.** A rule that refused a key it cannot decompose into a \
                 section would refuse `-c a=b`, turn that whole property PERMANENTLY RED in a \
                 file `19-23` may not edit, and reproduce `19-18`'s `{{v}}>` blocker one axis \
                 over.\n\n\
                 The correct response is to CONFINE dotless keys, never to edit round 7's \
                 property.",
                assignment.key
            );
        }
    }
    // The same dotless reasoning, restated for the FOURTH alphabet: a key with no
    // `.` names NO SECTION, so it can never name `alias` either.
    for entry in CONFIG_REPARSED_VALUE_CARRIERS {
        for assignment in quoted_config_assignments(&config_case(
            "git status",
            entry,
            ConfigSplice::ImmediatelyAfterTheProgram,
        )) {
            assert!(
                assignment.key.contains('.'),
                "\n\nA DOTLESS KEY IS IN `CONFIG_REPARSED_VALUE_CARRIERS`: `{entry}` delivers \
                 the key `{}`.\n\n\
                 A `-c` key with no `.` at all names NO CONFIG SECTION and therefore cannot name \
                 `alias`. **`CALLEE_KNOWN_LEADING_PREFIX` at \
                 tests/envelope_wrapper_class.rs:5197 is `\"-c a=b\"`, and round 7's ENTIRE \
                 callee-grammar generative property is spliced behind it.** A rule that refused \
                 a key it cannot decompose into a section would refuse `-c a=b` and turn that \
                 whole property PERMANENTLY RED in a file `19-25` may not edit.",
                assignment.key
            );
        }
    }
    for entry in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        let (key, _) = persisted_config_write(entry).expect("a `git config <key> <value>` write");
        assert!(
            key.contains('.'),
            "a DOTLESS key is in `CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS`: `{entry}` names \
             `{key}`. The same reasoning holds at the persisted operand."
        );
    }

    assert!(
        CONFIG_CONFINED_CARRIERS.contains(&CALLEE_KNOWN_LEADING_PREFIX),
        "\n\n`{CALLEE_KNOWN_LEADING_PREFIX}` — `CALLEE_KNOWN_LEADING_PREFIX` at \
         tests/envelope_wrapper_class.rs:5197 — must be an entry of \
         `CONFIG_CONFINED_CARRIERS`, and must be asserted VERDICT-PRESERVING by the \
         invariance arm below.\n\n\
         Round 7's ENTIRE callee-grammar generative property is spliced behind it, and real \
         git RUNS `git -c a=b version` at rc 0. Confining it is CORRECT BY DESIGN, not a \
         concession. Asserting it here by name is what stops a later round from quietly \
         moving it into the indirection alphabet."
    );

    // ---------------------------------------------------------------------
    // **THE DISJOINTNESS ASSERTION.** The three alphabets must not overlap, or
    // the invariance arm would draw an entry whose verdict the fix CHANGES.
    // ---------------------------------------------------------------------
    for entry in CONFIG_INDIRECTION_CARRIERS
        .iter()
        .chain(CONFIG_ENV_CARRIERS)
        .chain(CONFIG_REPARSED_VALUE_CARRIERS)
    {
        assert!(
            !CONFIG_CONFINED_CARRIERS.contains(entry),
            "`{entry}` appears in BOTH `CONFIG_CONFINED_CARRIERS` and one of the fail-closed \
             alphabets.\n\n\
             `CONFIG_CONFINED_CARRIERS` entries are asserted VERDICT-PRESERVING by the \
             invariance arm. **An indirection carrier and an environment carrier are both \
             REFUSED after `19-23` even on a PERMITTED base** — `git -c include.path=… \
             status` and `GIT_CONFIG_PARAMETERS=… git status` are both measured at exit 0 \
             today and both refused after — so either would be STRICTER than its base and \
             would turn the invariance arm permanently red in a file `19-23` may not edit. \
             **This is `19-18`'s `{{v}}>` blocker and `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` \
             split, one axis over, and it is the single most likely way this seam breaks.**"
        );
    }
    for entry in CONFIG_ENV_CARRIERS {
        assert!(
            !CONFIG_INDIRECTION_CARRIERS.contains(entry),
            "`{entry}` appears in BOTH fail-closed alphabets. They are spliced at DIFFERENT \
             positions — between the program and its decision words versus as an assignment \
             prefix — and the counting floors would double-count an entry in both."
        );
    }
    for entry in CONFIG_CONFINED_CARRIERS {
        let spliced = config_case(
            "git status",
            entry,
            ConfigSplice::ImmediatelyAfterTheProgram,
        );
        assert!(
            !draws_a_config_indirection_carrier(&spliced),
            "`{entry}` is in the CONFINED alphabet but DRAWS the indirection class. \
             Membership of the confined alphabet is an assertion that the entry is \
             verdict-preserving, and an indirection is not."
        );
        assert!(
            !draws_a_reparsed_value_carrier(&spliced),
            "\n\n`{entry}` is in the CONFINED alphabet but DRAWS the RE-PARSED-VALUE class.\n\n\
             Membership of the confined alphabet is an assertion that the entry is \
             VERDICT-PRESERVING, and a re-parsed-value carrier is REFUSED after `19-25` even on \
             a PERMITTED base — so it would be STRICTER than its base and would turn the \
             invariance arm permanently red in a file `19-25` may not edit.\n\n\
             **The two `!`-bodied entries are the exception the fence exists to protect**: git \
             hands a `!` body to a SHELL child that inherits the injection, so class 6 \
             correctly does not draw them and their verdict does not move."
        );
    }
    for entry in CONFIG_REPARSED_VALUE_CARRIERS {
        assert!(
            !CONFIG_INDIRECTION_CARRIERS.contains(entry)
                && !CONFIG_ENV_CARRIERS.contains(entry),
            "`{entry}` appears in `CONFIG_REPARSED_VALUE_CARRIERS` AND in another fail-closed \
             alphabet. The counting floors would double-count it, and the three alphabets model \
             three different REGIONS."
        );
    }

    // ---------------------------------------------------------------------
    // **THE PERMITTED-BASE FENCE** — this axis's own version of the `{v}>`
    // lesson, and the constraint the whole corpus is shaped by.
    // ---------------------------------------------------------------------
    for required in [
        "git status",
        "git commit -m x",
        "git push origin refs/heads/gsd-auto/alpha/w",
    ] {
        assert!(
            CONFIG_PERMITTED_BASES.contains(&required),
            "\n\n`{required}` must be one of `CONFIG_PERMITTED_BASES`.\n\n\
             **A `T-19-103` reproducer must be built on a base LAYER 2 PERMITS.** The whole \
             content of an indirection is that it disarms layer 3 on a line layer 2 lets \
             THROUGH. Measured at this section's base commit, fresh root per row, walk EMPTY: \
             `git -c include.path=/tmp/evil.cfg push --force origin main` is ALREADY exit 2 \
             `force_push_blocked` for its VERB, and so are its `bash -lc`, `echo hi &&`, \
             `>/dev/null` and `--attr-source HEAD` compositions — **five CONTROLS, not \
             reproducers**.\n\n\
             A property drawing only refused bases is GREEN before the fix and certifies \
             nothing about config resolution: the eighth consecutive instance of `T-19-76`'s \
             failure mode, produced by the corpus rather than found by the next audit. These \
             three are the bases audit 7 itself used."
        );
    }
    assert!(
        !CONFIG_PERMITTED_BASES
            .iter()
            .any(|base| base.contains("--force")),
        "no `CONFIG_PERMITTED_BASES` entry may carry `--force`: every `--force` base is \
         already refused for its VERB and certifies nothing about config resolution"
    );

    // -- **THE PERMITTED-BASE FENCE, EXTENDED TO ROUND 9'S PROPERTY.** The
    //    re-parsed-value arm draws from the SAME base set, so the fence above
    //    already covers it — but a later round could give it a base set of its own,
    //    and this asserts BY NAME that at least one base it draws is PERMITTED.
    assert!(
        CONFIG_PERMITTED_BASES.iter().any(|base| {
            let envelope = TempDir::new().expect("a temporary envelope root");
            verdict(envelope.path(), base).code == 0
        }),
        "\n\n**THE RE-PARSED-VALUE PROPERTY MUST DRAW AT LEAST ONE LAYER-2-PERMITTED BASE.**\n\n\
         The whole content of `T-19-108` is that a carrier inside a confined VALUE disarms \
         layer 3 on a line layer 2 LETS THROUGH. Measured at round 9's base commit, fresh root \
         per row, walk EMPTY: `git -c include.path=/tmp/evil.cfg push --force origin main` is \
         ALREADY exit 2 `envelope_assertion_failed` for its CARRIER, and every `--force` \
         composition is refused for its VERB — **CONTROLS, not reproducers**.\n\n\
         A property drawing only refused bases is GREEN before the fix and certifies nothing: \
         **the NINTH consecutive instance of `T-19-76`'s failure mode**, produced by the corpus \
         rather than found by the next audit. Measured: all 40 permitted-base re-parsed-value \
         cases are at exit 0 today, and those 40 are what makes the property RED."
    );

    // -- and the PERSISTED arm's own permitted-base fence: its entries are whole
    //    commands, so the constraint is that each is PERMITTED today. `git config`
    //    writes are layer-2-permitted, which is exactly why the class is live.
    for entry in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        let (_, value) = persisted_config_write(entry).expect("a `git config <key> <value>` write");
        assert!(
            !value.contains("--force"),
            "\n\n`{entry}` carries `--force` in its alias BODY.\n\n\
             That would make the row's refusal derivable from the VERB rather than from the \
             carrier — the persisted equivalent of building a reproducer on a refused base. \
             Measured: `git config alias.p '<non-shell body>'` is exit 0 today in every \
             spelling, and that is what makes this arm RED."
        );
    }
}

#[test]
fn the_generated_corpus_really_produces_each_config_resolution_class_in_quantity() {
    // **An alphabet floor is not a generation floor.** An entry can sit in an
    // alphabet and be drawn by nothing, or be drawn once out of hundreds of cases
    // — which is a corpus that can technically fail on the class and practically
    // never does. This counts what the generator ACTUALLY emits.
    //
    // **Green today and after**: it drives no guard call at all, and no production
    // change can move it. The counts are a pure function of the alphabets above,
    // which is exactly what makes the exact equalities safe.
    let mut all: Vec<String> = Vec::new();
    let mut slots: BTreeSet<(&str, &str)> = BTreeSet::new();

    let mut refused_count = 0usize;
    let mut permitted_count = 0usize;

    for (bases, counter) in [
        (CONFIG_REFUSED_BASES, &mut refused_count),
        (CONFIG_PERMITTED_BASES, &mut permitted_count),
    ] {
        for alphabet in [
            CONFIG_CONFINED_CARRIERS,
            CONFIG_INDIRECTION_CARRIERS,
            CONFIG_REPARSED_VALUE_CARRIERS,
        ] {
            for (base, splice, _, command) in config_cases(bases, alphabet) {
                slots.insert((
                    base,
                    match splice {
                        ConfigSplice::ImmediatelyAfterTheProgram => "ImmediatelyAfterTheProgram",
                        ConfigSplice::AfterAKnownLeadingOption => "AfterAKnownLeadingOption",
                    },
                ));
                all.push(command);
                *counter += 1;
            }
        }
        for (base, _, command) in config_env_cases(bases) {
            slots.insert((base, "AssignmentPrefix"));
            all.push(command);
            *counter += 1;
        }
    }

    // The PERSISTED arm: whole commands, no base and no splice, so they belong to
    // neither the refused nor the permitted counter and contribute no slot.
    for entry in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        all.push((*entry).to_string());
    }
    assert_eq!(
        CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS.len(),
        CONFIG_RESOLUTION_PERSISTED_CASES,
        "the persisted arm's generation count must equal the stated arithmetic exactly"
    );

    assert_eq!(
        refused_count, CONFIG_RESOLUTION_REFUSED_CASES,
        "the refused-base generation count must equal the stated arithmetic exactly, so \
         losing one case turns this red. `19-16` set a floor of 50 against a maximum of 40 by \
         construction and it was invisible until the rule landed."
    );
    assert_eq!(
        permitted_count, CONFIG_RESOLUTION_PERMITTED_CASES,
        "the permitted-base generation count must equal the stated arithmetic exactly"
    );
    assert_eq!(all.len(), CONFIG_RESOLUTION_CASES);
    assert_eq!(
        slots.len(),
        CONFIG_RESOLUTION_SLOTS,
        "every splice slot must have been generated — 7 bases x 2 command-line positions plus \
         7 assignment-prefix positions. Seen: {slots:?}"
    );

    let mut per_class: BTreeMap<&str, usize> = BTreeMap::new();
    for command in &all {
        for (class, predicate) in CONFIG_RESOLUTION_CLASSES {
            if predicate(command) {
                *per_class.entry(class).or_default() += 1;
            }
        }
    }

    for (class, expected) in CONFIG_RESOLUTION_CLASS_COUNTS {
        let got = per_class.get(class).copied().unwrap_or(0);
        assert_eq!(
            got, *expected,
            "the generator emitted {got} cases of the config-resolution class `{class}`, and \
             the stated arithmetic derives {expected}.\n\n\
             This is `T-19-105` counted rather than read. The correct response to a shortfall \
             is to RESTORE entries, never to lower the number. Full counts: {per_class:?}"
        );
    }

    // Recorded so the SUMMARY carries measured counts rather than described ones.
    println!(
        "config-resolution axis: {} cases ({refused_count} on refused bases, \
         {permitted_count} on permitted bases, {} persisted) over {} slots from {} confined, \
         {} indirection, {} reparsed-value and {} environment entries.\nper class: {per_class:?}",
        all.len(),
        CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS.len(),
        slots.len(),
        CONFIG_CONFINED_CARRIERS.len(),
        CONFIG_INDIRECTION_CARRIERS.len(),
        CONFIG_REPARSED_VALUE_CARRIERS.len(),
        CONFIG_ENV_CARRIERS.len(),
    );
}

#[test]
fn a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries() {
    // **ROUND 9'S OWN FAIL-CLOSED PROPERTY, in BOTH deliveries, and RED against the
    // pre-fix tree.**
    //
    // Kept out of the invariance arm because its entries are NOT verdict-preserving:
    // a re-parsed-value carrier is REFUSED after `19-25` even on a PERMITTED base —
    // measured at round 9's base commit, **all 40 permitted-base cases at exit 0
    // and all 30 refused-base cases already refused for their VERB, walks EMPTY
    // throughout**. Those 40, plus the 5 persisted whole commands, are what makes
    // this property RED.
    //
    // **The assertion is the exit code and an EMPTY WALK, never the reason
    // identifier.** A re-parsed-value carrier on a refused base is refused TODAY for
    // its verb and after the fix for its carrier — a reason-identifier change on a
    // row whose verdict never moves. Identifiers belong in the NAMED per-row pins of
    // `tests/envelope_reparsed_value.rs`, where each carries its own written
    // derivation, and in the two ordering pins there that fix the scan order.
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "the walk's POSITIVE control must find a line, or every empty-walk assertion below is \
         vacuous. Files: {:?}",
        files_under(control.path())
    );

    let failure = "\n\nA CARRIER INSIDE A CONFINED CONFIG VALUE WAS PERMITTED.\n\
         \n`scan_leading` reads config assignments in the LEADING-OPTION region and \
         `resolve_program`'s step-1 check reads them in the ENVIRONMENT. **Git resolves \
         configuration from a THIRD place: a value the guard itself CONFINED and handed on.** \
         Git re-parses a non-`!` `alias.<name>` body as a git command line IN-PROCESS \
         INCLUDING ITS LEADING OPTIONS, so a `-c include.path=<f>` inside the VALUE is applied \
         by git at command-line precedence — measured `/INCLUDE_WINS` against the control's \
         `/ENV_WINS` — while `alias.q` correctly answers `false` to \
         `config_key_names_an_indirection_section` and is CONFINED.\n\
         \n**Round 8's clause is the RIGHT SHAPE of rule and this is not a reason to replace \
         it** — it asks whether an assignment can be BOUNDED rather than whether it spells a \
         name, and audit 8 verified every one of its rows. The gap is in REACH, one REGION \
         over.\n\
         \n**The correct response is NOT a blanket `alias.*` refusal.** A `!`-bodied body is \
         handed to a SHELL child that INHERITS the envelope's injection (measured `/ENV_WINS`), \
         and `tests/envelope_command_position.rs:550` and \
         `tests/envelope_config_resolution.rs:1539-1543` both pin one PERMITTED as a registered \
         `T-19-86` row in files `19-25` may not edit. It is NOT a prefix match on `alias` — \
         `-c aliasx.q=…` and `-c notalias.q=…` are pinned PERMITTED before and after. And it is \
         NOT a refusal of a key that cannot be decomposed into a section — `-c a=b` is \
         `CALLEE_KNOWN_LEADING_PREFIX` and round 7's entire property is spliced behind it.";

    // --- ARM 1: the `-c` / `--config-env` DELIVERY, spliced --------------
    let mut cases = 0usize;
    let mut classes_seen: BTreeSet<&str> = BTreeSet::new();
    for bases in [CONFIG_REFUSED_BASES, CONFIG_PERMITTED_BASES] {
        for (base, splice, label, command) in config_cases(bases, CONFIG_REPARSED_VALUE_CARRIERS) {
            for (class, predicate) in CONFIG_RESOLUTION_CLASSES {
                if predicate(&command) {
                    classes_seen.insert(class);
                }
            }
            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code,
                2,
                "{failure}\
                 \n\n  command : {command:?}\
                 \n  base    : {base}\
                 \n  splice  : {splice:?} / {label}\
                 \n  got     : exit {} reason {}\
                 \n  seed    : {SEED:#x}",
                got.code,
                got.reason_id,
            );
            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command:?}` was refused, but a pull-request ledger line was written \
                 somewhere under the envelope root. Found: {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }
    assert_eq!(
        cases,
        CONFIG_REPARSED_VALUE_CARRIERS.len() * CONFIG_SPLICES.len() * 7,
        "the `-c` arm must run every generated re-parsed-value case"
    );

    // --- ARM 2: the PERSISTED delivery, as whole commands ----------------
    //
    // **This is a SECOND REGION of the same class, and it must be able to fail
    // separately.** `classify_config` already reads this exact operand with
    // `is_hooks_path_key`, so asking the alias question there is the same decision
    // region — but a rule written only inside `scan_leading` would leave every row
    // below at exit 0.
    let mut persisted_cases = 0usize;
    for command in CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), command);
        assert_eq!(
            got.code,
            2,
            "{failure}\
             \n\n  command : {command:?}\
             \n  region  : the PERSISTED `git config <key> <value>` write site\
             \n  got     : exit {} reason {}\n\
             \n**AND THE PERSISTENCE ASYMMETRY, which `19-25` must not get backwards.** A \
             persisted `git config include.path <evil>` is INERT — measured at repo-local and \
             at GLOBAL under the envelope's own config posture, it resolves `/ENV_WINS` because \
             the injection OUTRANKS every repository-side level — so a clause for the INCLUDE \
             family at this write site would be INERT and must NOT be added. **An alias is \
             different in kind: it does not have to WIN a precedence contest, it only has to \
             EXIST**, and both persistence levels were measured live at `/INCLUDE_WINS`.",
            got.code,
            got.reason_id,
        );
        let written = ledger_lines_under(envelope.path());
        assert!(
            written.is_empty(),
            "`{command:?}` was refused, but a ledger line was written. Found: {written:?}"
        );
        persisted_cases += 1;
    }
    assert_eq!(
        persisted_cases,
        CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS.len(),
        "the persisted arm must run every entry"
    );

    assert!(
        classes_seen.contains("a carrier delivered inside a config value the guard confines"),
        "the re-parsed-value arm must DRAW its own class. Seen: {classes_seen:?}"
    );
    assert!(
        classes_seen.len() >= 2,
        "and it must also draw the CONFINED class, because that overlap IS `T-19-108`: round \
         8's clause confines an `alias.*` key and lets the carrier through. Seen: {classes_seen:?}"
    );
}

// ---------------------------------------------------------------------------
// 16c. The generative properties — the invariance arm GREEN today, the two
//      fail-closed arms RED against the pre-fix tree
// ---------------------------------------------------------------------------

#[test]
fn a_confined_config_assignment_never_changes_a_verdict() {
    // **The INVARIANCE arm, and the ONLY alphabet it draws is
    // `CONFIG_CONFINED_CARRIERS`.** It runs as its own test and is GREEN TODAY, so
    // this round's failure output is itself evidence that the permitted half
    // passed. A property whose permitted arm is unobservable until the fix lands
    // cannot be said to fail in both directions.
    //
    // Measured against the built binary at this section's base commit over all 70
    // generated cases: **40 permits and 30 `force_push_blocked`**, walks EMPTY
    // throughout, no anomalies.
    //
    // **Without this arm, an implementation that simply refused every governed
    // command carrying a `-c` would satisfy both fail-closed arms below** — while
    // refusing `git -c user.name="$NAME" commit -m x`, `git -c core.pager=cat log`
    // and `git -c a=b status`, whose measured cost
    // `tests/envelope_config_resolution.rs` pins row by row. **That is the shape
    // this fix is ONE WRONG STEP away from** (AR-19-11), and setting `user.name`
    // on the command line is exactly what a driven run does to make its commits
    // attributable.

    // --- floor 0: the POSITIVE control for the walk -----------------------
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "a PERMITTED `gh pr create` writes exactly one ledger line, and the walk must be able \
         to find it. If this is 0 the walk is blind and every empty-walk assertion below is \
         vacuous. Files: {:?}",
        files_under(control.path())
    );

    // --- floor 1: every base answers what the arm it is in claims ---------
    let mut base_verdicts: BTreeMap<&str, Verdict> = BTreeMap::new();
    for base in CONFIG_PERMITTED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 0,
            "the UNWRAPPED base `{base}` must be PERMITTED, or every case built on it is \
             green or red for the base's own reason — and the whole permitted-base constraint \
             of this axis rests on it. Got reason id: {}",
            got.reason_id
        );
        base_verdicts.insert(base, got);
    }
    for base in CONFIG_REFUSED_BASES {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), base);
        assert_eq!(
            got.code, 2,
            "the UNWRAPPED base `{base}` must be REFUSED. Got reason id: {}",
            got.reason_id
        );
        assert!(
            REASON_IDENTIFIERS.contains(&got.reason_id.as_str()),
            "and refused under a D-24 identifier. Got: {}",
            got.reason_id
        );
        base_verdicts.insert(base, got);
    }

    // --- the invariance assertion -----------------------------------------
    let mut cases = 0usize;
    for bases in [CONFIG_PERMITTED_BASES, CONFIG_REFUSED_BASES] {
        for (base, splice, label, command) in config_cases(bases, CONFIG_CONFINED_CARRIERS) {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);
            let expected = base_verdicts.get(base).expect("the base was measured");
            assert_eq!(
                got,
                *expected,
                "\n\nA CONFINED CONFIG ASSIGNMENT CHANGED A VERDICT.\n\
                 \n  command : {command:?}\
                 \n  base    : {base}\
                 \n  splice  : {splice:?} / {label}\
                 \n  got     : exit {} reason {}\
                 \n  seed    : {SEED:#x}\n\
                 \nEvery entry of `CONFIG_CONFINED_CARRIERS` names a config key whose effect \
                 is BOUNDED to the key it names: measured against real git with the \
                 envelope's own injection as the control, none of them changes what \
                 `git config --get core.hooksPath` resolves to.\n\
                 \n**A red here is the fix having become a blanket refusal of anything \
                 spelled `-c`**, which is a guard nobody can use and therefore a control that \
                 gets switched off (AR-19-11). It is NOT a reason to narrow this alphabet, \
                 and it is NOT a reason to move an entry into `CONFIG_INDIRECTION_CARRIERS` \
                 — that alphabet's membership is a MEASUREMENT against the real git binary, \
                 and an entry there is asserted to OUTRANK the envelope's injection.\n\
                 \n**If the red entry is `-c a=b`, the damage is wider than this file**: it \
                 is `CALLEE_KNOWN_LEADING_PREFIX` (line 5197) and round 7's entire \
                 callee-grammar property is spliced behind it.\n\
                 \n**If the red entry is `-c notinclude.path=…` or `-c includepath=…`, the \
                 rule became a substring match on `include`** rather than a comparison of the \
                 SECTION. Those two are this round's `--signed no`.",
                got.code,
                got.reason_id,
            );
            cases += 1;
        }
    }

    assert_eq!(
        cases,
        CONFIG_CONFINED_CARRIERS.len() * CONFIG_SPLICES.len() * 7,
        "the invariance arm must run every generated confined case"
    );
    println!("config-resolution invariance arm drove {cases} verdict-preserving cases.");
}

#[test]
fn a_config_indirection_the_guard_cannot_bound_fails_closed_on_every_base() {
    // **The indirection alphabet's OWN property, separate because its entries are
    // NOT verdict-preserving.** An indirection carrier is REFUSED after `19-23` on
    // BOTH kinds of base — that is the whole point of the inversion — so it cannot
    // be drawn by the invariance arm without turning that property permanently red
    // in a file `19-23` may not edit. This is `19-18`'s `{v}>` blocker one axis
    // over, prevented by CONSTRUCTION rather than by care.
    //
    // Measured against the built binary at this section's base commit: of the 70
    // cases, the **30 on refused bases are already refused** (for their VERB,
    // which the missing key check happens not to prevent) and **all 40 on
    // permitted bases are at exit 0**. Those 40 are what makes this property RED.
    //
    // **The assertion is the exit code and an EMPTY WALK, never the reason
    // identifier.** An indirection carrier on a refused base is refused TODAY for
    // its verb and after the fix for its carrier — a reason-identifier change on a
    // row whose verdict never moves. Identifiers belong in the NAMED per-row pins
    // of `tests/envelope_config_resolution.rs`, where each carries its own written
    // derivation, and in the two ordering pins there that fix the scan order.
    let control = TempDir::new().expect("a temporary envelope root");
    permits(control.path(), "gh pr create --title x");
    assert_eq!(
        ledger_lines_under(control.path()).len(),
        1,
        "the walk's POSITIVE control must find a line, or every empty-walk assertion below is \
         vacuous. Files: {:?}",
        files_under(control.path())
    );

    let mut cases = 0usize;
    let mut classes_seen: BTreeSet<&str> = BTreeSet::new();
    for bases in [CONFIG_REFUSED_BASES, CONFIG_PERMITTED_BASES] {
        for (base, splice, label, command) in config_cases(bases, CONFIG_INDIRECTION_CARRIERS) {
            for (class, predicate) in CONFIG_RESOLUTION_CLASSES {
                if predicate(&command) {
                    classes_seen.insert(class);
                }
            }

            // A fresh root per case, because a permitted forge command writes a
            // ledger line and a shared root would exhaust the cap and turn later
            // cases red for a reason that has nothing to do with the carrier.
            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);

            assert_eq!(
                got.code,
                2,
                "\n\nA CONFIG INDIRECTION DISARMED THE HOOK AND THE COMMAND WAS PERMITTED.\n\
                 \n  command : {command:?}\
                 \n  base    : {base}\
                 \n  splice  : {splice:?} / {label}\
                 \n  got     : exit {} reason {}\
                 \n  seed    : {SEED:#x}\n\
                 \n`scan_leading` decides exactly ONE question about a `-c` assignment: is the \
                 KEY half `core.hooksPath` (D-09, `is_hooks_path_key` at `policy.rs:750`)? \
                 Git's config resolution has an INDIRECTION that check does not model. A key \
                 in the `include` or `includeIf` section names a FILE whose contents are \
                 spliced in **at the precedence of the directive that named them** — i.e. at \
                 command-line precedence, outranking the envelope's env-injected triplet — \
                 **without the string `core.hooksPath` appearing anywhere on the line.** \
                 Measured against real git with the envelope's own \
                 `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection as the control: the control \
                 alone prints `/ENV_WINS`, and every entry of this alphabet makes the same \
                 command print `/INCLUDE_WINS`.\n\
                 \n**Every word here is literal, every word arrives in order, and the word \
                 the guard calls the verb IS the word git calls the verb.** Rounds 5, 6 and 7 \
                 are all RIGHT about this line and all three are pinned non-vacuous in \
                 `tests/envelope_config_resolution.rs`. What is wrong is one layer further \
                 out: the guard reasoned correctly about the command line and then assumed a \
                 CONFIGURATION git will not resolve.\n\
                 \n**The correct response is to make the guard ask `can I establish what \
                 core.hooksPath will be` instead of `does this assignment spell \
                 core.hooksPath`.** It is NOT a blanket refusal of every `-c` — the invariance \
                 arm above and the eight ordinary invocations in \
                 `tests/envelope_config_resolution.rs` measure that option's cost. It is NOT \
                 a substring match on `include` — `-c notinclude.path=…` and \
                 `-c includepath=…` are pinned PERMITTED before and after. And it is NOT a \
                 refusal of a key that cannot be decomposed into a section — `-c a=b` is \
                 `CALLEE_KNOWN_LEADING_PREFIX` and round 7's entire property is spliced \
                 behind it.",
                got.code,
                got.reason_id,
            );

            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command:?}` was refused, but a pull-request ledger line was written \
                 somewhere under the envelope root. Found: {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }

    assert_eq!(
        cases,
        CONFIG_INDIRECTION_CARRIERS.len() * CONFIG_SPLICES.len() * 7,
        "the fail-closed arm must run every generated indirection case"
    );
    assert!(
        classes_seen.len() >= 4,
        "the indirection arm must draw at least FOUR config-resolution classes — the \
         indirection class, the case-varied class, the option-carrier class, and the confined \
         class through `CALLEE_KNOWN_LEADING_PREFIX` at the second splice. It cannot draw the \
         environment class, and that is deliberate: `CONFIG_ENV_CARRIERS` is spliced at a \
         different position and has its own property. Seen: {classes_seen:?}"
    );
}

#[test]
fn an_environment_carrier_of_configuration_fails_closed_on_every_base() {
    // **The environment alphabet's OWN property**, separate for two reasons: its
    // entries are not verdict-preserving, and they are spliced at a DIFFERENT
    // POSITION — as an assignment PREFIX, which is where `resolve_program`'s
    // step-1 check reads rather than where `scan_leading` reads.
    //
    // Measured at this section's base commit: of the 21 cases, the 9 on refused
    // bases are already refused for their verb and **all 12 on permitted bases are
    // at exit 0**. Those 12 are what makes this property RED.
    //
    // **The post-fix verdict is DERIVED, not guessed.** All three environment
    // spellings of the paired discriminator `GIT_CONFIG_COUNT=0` — the bare
    // prefix, `export …;` and `env …` — are measured at exit 2
    // `hook_bypass_blocked` TODAY on BOTH a refused and a permitted base, for a
    // key that IS in `ENVELOPE_ENV_KEYS`. One list entry therefore derives every
    // case here.
    //
    // **Why the existing drift pin STRUCTURALLY CANNOT SEE THIS.** It is sourced
    // from `cred::EnvelopeEnv::with_run_id(build_env_in(…))` — from the keys the
    // envelope SETS or REMOVES — and `GIT_CONFIG_PARAMETERS` is a key the envelope
    // neither sets nor removes but which DEFEATS one it sets. The fix is a SECOND
    // SOURCE, not a wider filter.
    let mut cases = 0usize;
    for bases in [CONFIG_REFUSED_BASES, CONFIG_PERMITTED_BASES] {
        for (base, label, command) in config_env_cases(bases) {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code,
                2,
                "\n\nAN ENVIRONMENT CARRIER OF CONFIGURATION WAS PERMITTED.\n\
                 \n  command : {command:?}\
                 \n  base    : {base}\
                 \n  splice  : {label}\
                 \n  got     : exit {} reason {}\n\
                 \n`GIT_CONFIG_PARAMETERS` is git's OWN internal carrier for `-c`. Measured \
                 against real git with the envelope's own injection as the control, it prints \
                 `/PARAM_WINS` where the injection alone prints `/ENV_WINS` — **it OUTRANKS \
                 the triplet the envelope delivers `core.hooksPath` through.** Git EXPORTS \
                 it, so ONE PREFIX DISARMS EVERY GIT SUBPROCESS of the command, which is how \
                 it composes with `T-19-86`. Confirmed end to end in \
                 `tests/envelope_config_resolution.rs`: an in-namespace push the `pre-push` \
                 hook refuses completed and MOVED a bare remote's ref under this carrier, \
                 with the SHAs recorded before and after.\n\
                 \n**This is a gap in the LIST, not in the mechanism**, and the paired \
                 discriminator is what proves it: all three spellings of \
                 `GIT_CONFIG_COUNT=0` are refused TODAY at `hook_bypass_blocked`, on both a \
                 refused and a permitted base. The correct response is to add \
                 `GIT_CONFIG_PARAMETERS` to `ENVELOPE_ENV_KEYS` — one entry, whose disclosed \
                 bare-word cost (`echo GIT_CONFIG_PARAMETERS`) is pinned in \
                 `tests/envelope_config_resolution.rs` beside its measured twin.",
                got.code,
                got.reason_id,
            );

            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command:?}` was refused, but a pull-request ledger line was written \
                 somewhere under the envelope root. Found: {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }

    assert_eq!(
        cases,
        CONFIG_ENV_CARRIERS.len() * 7,
        "the environment fail-closed arm must run every generated case"
    );
}

// ===========================================================================
// 17. THE FIFTH AXIS — `CONTROL_CARRIER_CLASSES`
//
// **ROUND 10 CREATES A FIFTH AXIS, AND THAT IS THE OPPOSITE OF ROUND 9's
// FINDING. THE REASON IS STATED RATHER THAN ASSUMED.**
//
// Section 16's header is right and this section EXTENDS it rather than
// contradicting it. Rounds 1-5 were about how a word is WRITTEN; round 6 about
// which words ARRIVE; round 7 about which arriving word is the VERB; round 8
// about what the verb RUNS UNDER; round 9 about what git does with a VALUE the
// verb was handed. **Round 9 correctly refused a fifth axis because its gap was
// one REGION over on round 8's axis, and standing a region up as a stage would
// have lost exactly the distinction audit 8 drew.**
//
// **ALL FIVE READ AN ARGV. Round 10's gap is not a region and not a stage — it
// is a different KIND of carrier.** Every control this envelope installs has a
// FILE carrier that no command line has to touch: `.git/config` carries the
// alias, `pr-ledger.ndjson` carries SAFE-06, `hooks/pre-push` carries SAFE-05
// and `AR-19-03`'s backstop, `gitconfig` carries D-16/D-18, `settings.json`
// carries layer 2's own registration — and the guard's `Ungoverned` arm
// (`hooks.rs:992-994`) permits every command that touches them.
//
// **THE MECHANICAL PROOF THAT THE AXIS IS NEW, ASSERTED IN
// `the_control_carrier_axis_is_new_because_no_existing_axis_reaches_a_command_
// with_no_governed_program` RATHER THAN CLAIMED IN PROSE:** every generative
// property on all four existing axes splices its alphabet into a GOVERNED BASE
// and asserts a verdict about the GOVERNED PROGRAM'S ARGV. **A control-carrier
// command reaches NO governed program at all** — `resolve_program_with_head`
// answers `Ungoverned` or `NoProgram` for every entry below — so no existing
// axis's property can be satisfied by one, and no control-carrier entry can be
// generated by an existing axis.
//
// **WHAT IS *NOT* CLAIMED, AND THE REASON.** A TEXT-LEVEL disjointness between
// this axis's class predicates and the four existing axes' would be FALSE, and
// asserting it would be asserting something untrue. `draws_a_separate_word_
// redirection` (`DELETION_CLASSES` class 1) is a pure text function and it fires
// on `: > <ENV>/alpha/pr-ledger.ndjson`, because that command really does carry a
// separate-word redirection; `draws_a_tilde` fires on any entry naming `~`. Those
// are INCIDENTAL TEXTUAL MATCHES on commands the existing axes' properties can
// never generate, because those properties only ever splice into `git …`. **The
// disjointness that is TRUE is the one about REACHABILITY, and that is the one
// asserted.**
//
// `UNREADABLE_CLASSES` (`:3079`), `DELETION_CLASSES` (`:3967`),
// `CALLEE_GRAMMAR_CLASSES` (`:4915`) and `CONFIG_RESOLUTION_CLASSES` (`:6253`),
// all of their predicates, all of their degenerate-proofing and all of their
// floors are BYTE-IDENTICAL, and `MIN_CONFIG_RESOLUTION_CLASSES` stays at **6**.
//
// Every per-row verdict, every derivation, the four-call cap-reset drive and the
// rebuilt bare-remote fixture behind round 10's half live in
// `tests/envelope_control_carrier.rs`, the NINTH evidence file.
// ---------------------------------------------------------------------------

/// The envelope root every representative and every class predicate resolves
/// against.
///
/// A SENTINEL rather than a real temporary directory, because the class
/// predicates are pure text functions evaluated at compile-time-known strings,
/// exactly as sections 13-16's are. The GUARD-DRIVEN properties substitute a
/// fresh `TempDir` for [`ENVELOPE_ROOT_PLACEHOLDER`] instead.
const REPRESENTATIVE_ENVELOPE_ROOT: &str = "/tmp/gsd-envelope-root";

/// The same root with ONE CHARACTER CHANGED — the near-miss control's subject.
const REPRESENTATIVE_NEAR_MISS_ROOT: &str = "/tmp/gsd-envelope-rooX";

/// The placeholder each alphabet entry carries where the envelope root goes.
const ENVELOPE_ROOT_PLACEHOLDER: &str = "<ENV>";

/// The placeholder for the one-character-changed root.
const NEAR_MISS_ROOT_PLACEHOLDER: &str = "<ENVX>";

/// The path the corpus NAMES as a symlink into the envelope root.
///
/// **A symlink is not textually distinguishable from an ordinary path, and that
/// is EXACTLY why direction (iii) fails open.** The class is therefore defined by
/// a NAMED representative path, and the predicate recognises it by that name.
/// Stating this is the point: a predicate that could tell a link from a file
/// would be doing filesystem I/O, which is what rule (a) must not do.
const SYMLINK_MARKER: &str = "/tmp/link-into-the-envelope-root";

/// **`C-10` — THE BINARY EVERY HOOK STUB AND THE GUARD REGISTRATION EXEC.**
///
/// **This is an EXACT-PATH constant and not a directory, and the difference is
/// MEASURED rather than aesthetic.** The envelope DIRECTORY is a PREFIX
/// boundary because this envelope owns every byte under it — `rm -rf
/// <ENV>/alpha` takes NINE carriers in one call. **The binary's directory is
/// shared with everything else the user installed**, so a prefix over it would
/// refuse `ls ~/.cargo/bin` and every `cargo install`. The two near-miss
/// controls in [`CONTROL_CARRIER_BINARY_NEAR_MISSES`] pin that from both sides.
///
/// A SENTINEL rather than a real path, for the same reason
/// [`REPRESENTATIVE_ENVELOPE_ROOT`] is one: the class predicates are pure text
/// functions. The GUARD-DRIVEN properties substitute `std::env::current_exe()`
/// for [`BINARY_PLACEHOLDER`] instead — the same call `hooks::install`
/// (`hooks.rs:92`) makes to decide what to BAKE INTO the stub.
const REPRESENTATIVE_BINARY_PATH: &str = "/tmp/gsd-binary-dir/gsd-meta-manager";

/// The DIRECTORY the binary lives in — deliberately NOT a boundary.
const REPRESENTATIVE_BINARY_PARENT: &str = "/tmp/gsd-binary-dir";

/// The placeholder each alphabet entry carries where the guard's own binary goes.
const BINARY_PLACEHOLDER: &str = "<BIN>";

/// The placeholder for the binary's PARENT DIRECTORY, which is not a carrier.
const BINARY_PARENT_PLACEHOLDER: &str = "<BINPAR>";

/// The envelope ROOT's own PARENT — **the first path ABOVE the ancestor clause's
/// STOP, and the row that turns red if a clause walks the whole chain to `/`.**
///
/// The clause `19-33` writes stops at `<root>` on OWNERSHIP grounds: `<root>` is
/// created by this tool and holds only alias directories it created, while
/// `~/.local/share`, `$HOME`, `/tmp` and `/` are shared with everything the user
/// has. **A boundary reaching them would refuse `ls /`, `df /`, `du -sh $HOME`
/// and `ls /tmp` — an OUTAGE rather than a boundary (AR-19-11).** That residue
/// is registered, disclosed and UNACCEPTED.
const REPRESENTATIVE_ENVELOPE_PARENT: &str = "/tmp";

/// The placeholder for the envelope root's PARENT, which is **above the stop**.
///
/// **It is substituted BEFORE [`ENVELOPE_ROOT_PLACEHOLDER`]**, because `<ENV>`
/// is a prefix of `<ENVPAR>` and a naive order would leave `<ENVPAR>` half
/// replaced — the same ordering hazard `<BINPAR>` and `<BIN>` already carry.
const ENVELOPE_PARENT_PLACEHOLDER: &str = "<ENVPAR>";

/// The envelope root spelled RELATIVE TO `$HOME`, which is what a `~` spelling
/// carries.
///
/// `dirs::data_local_dir()` is `$HOME/.local/share` on Linux and `envelope_root`
/// joins `gsd-meta-manager/envelope` onto it (`mod.rs:175-181`).
const TILDE_ENVELOPE_RELATIVE: &str = ".local/share/gsd-meta-manager/envelope";

/// Whether a word carries a PATHNAME-EXPANSION metacharacter.
///
/// **Round 5's `Token.literal` table names EXPANSION, PATHNAME, TILDE and BRACE
/// as the four classes that clear the bit, and rule (a) requires the bit.** This
/// is the PATHNAME half. `REWRITING_CHARACTERS` (`policy.rs:2535`) is
/// `$ ` ` * ? [ ~`; the three characters here are the ones that make a word a
/// GLOB rather than a substitution or a tilde spelling.
fn word_has_pathname_metacharacter(word: &str) -> bool {
    word.contains('*') || word.contains('?') || word.contains('[')
}

/// Whether a word carries a BRACE LIST — `{a,b}`.
///
/// A brace LIST rather than a bare brace: `{` alone is a `SEPARATORS` entry and
/// a word containing one without a comma and a closing brace is not brace
/// expansion.
fn word_has_brace_list(word: &str) -> bool {
    word.contains('{') && word.contains(',') && word.contains('}')
}

/// Whether a word is TILDE-BORNE and names a path under the envelope root.
fn word_is_tilde_borne_carrier(word: &str) -> bool {
    word.starts_with('~') && word.contains(TILDE_ENVELOPE_RELATIVE)
}

/// The basenames of the files the envelope's own controls live in.
///
/// `config` is deliberately ABSENT — it is `.git/config`'s basename and belongs
/// to the REPO-SIDE class, and a shared basename would collapse two classes.
const CARRIER_BASENAMES: &[&str] = &[
    "pr-ledger.ndjson",
    "pre-push",
    "pre-commit",
    "askpass",
    "gitconfig",
    "settings.json",
    "hosts.yml",
];

/// The repo-side control carriers, by the path fragment that names them.
const REPO_SIDE_CARRIER_PATHS: &[&str] = &[
    ".git/config",
    ".claude/settings.json",
    ".git/info/exclude",
    ".planning/meta-manager/runs",
];

/// The redirection operators this axis reads a TARGET after.
///
/// **`REDIRECTION_OPERATORS` (`:3794`) is reused rather than re-spelled**, for
/// the reason this file records elsewhere about second spellings: a second list
/// is a second thing to keep in step. Bash DELETES a redirection's operator AND
/// its target before `execve` (`policy.rs:2264-2279`, `T-19-97`), which is why
/// class 2 exists at all.
/// **BASH'S TWELVE-OPERATOR SPLIT, READ RATHER THAN INVENTED — NEW IN ROUND
/// 13.**
///
/// `redirection_operator` (`policy.rs:3193-3229`) derives the split in the same
/// match that computes each operator's length: SEVEN take a WORD the shell
/// resolves as a FILENAME, and FIVE take something that is not a file at all —
/// `<<` and `<<-` a heredoc DELIMITER, `<<<` a here-STRING, `>&` and `<&` an fd
/// NUMBER.
///
/// **A CORPUS-SIDE CLASS PREDICATE MAY NAME OPERATORS AND A PRODUCTION RULE MAY
/// NOT**, and the distinction is not a loophole: this decides which CLASS an
/// entry draws, where `redirection_operator`'s own match decides a guard
/// VERDICT. [`DISPLACING_REDIRECTIONS`] already exercises exactly that licence
/// by naming all twelve. **`19-33` reads the split from
/// `RedirectionOperator::pathname_target` and never re-spells it**, which
/// `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
/// is the mechanical control for.
///
/// The two lists are asserted to partition [`REDIRECTION_OPERATORS`] below, so a
/// spelling that appeared in one list and not the other could not hide.
const PATHNAME_REDIRECTION_OPERATORS: &[&str] = &["<", ">", ">>", ">|", "<>", "&>", "&>>"];

/// The FIVE operators whose target is not a pathname — **the word class
/// `protected_carrier_named` reads in NEITHER of its two loops today.**
///
/// **ORDERED LONGEST FIRST**, because these are matched against an ATTACHED
/// spelling (`>&PATH`) as well as a separated one (`<<< PATH`), and a short
/// match would leave the remainder to be read as the target's first character —
/// the same reason `redirection_operator` matches longest-first.
const NON_PATHNAME_REDIRECTION_OPERATORS: &[&str] = &["<<<", "<<-", "<<", ">&", "<&"];

/// The text a NON-PATHNAME redirection operator carries as its target, whether
/// the operator stands as a WORD OF ITS OWN (`<<< PATH`) or is ATTACHED to the
/// target (`>&PATH`), or `None` when this word is not such a target.
///
/// **BOTH SPELLINGS ARE DRAWN BECAUSE BOTH WERE MEASURED.** `19-32` drove
/// `xargs rm -rf <<< <ENV>/alpha` (separated) and `: >&<ledger>` (attached) and
/// both are exit 0 with a REACH demonstrated under real `bash` — the here-string
/// deleted the envelope directory, and `>&` truncated the ledger to zero bytes,
/// because bash falls back to `&>word` when the operand is not an fd number.
fn non_pathname_redirection_target<'a>(previous: &str, text: &'a str) -> Option<&'a str> {
    if NON_PATHNAME_REDIRECTION_OPERATORS.contains(&previous) {
        return Some(text);
    }
    NON_PATHNAME_REDIRECTION_OPERATORS
        .iter()
        .find(|operator| text.starts_with(**operator))
        .map(|operator| &text[operator.len()..])
}

/// Whether a string NAMES the envelope ROOT exactly — **the text fact class 14
/// is about, and the one class 1 now hands to it.**
///
/// A trailing `/` is trimmed because `lexical_absolute_components` collapses it
/// to the empty component and drops it, so `<ENV>` and `<ENV>/` normalise to the
/// SAME component vector. Treating them differently here would let one spelling
/// of the same path fall into a different class from the other.
fn word_names_the_envelope_root_exactly(text: &str) -> bool {
    text.trim_end_matches('/') == REPRESENTATIVE_ENVELOPE_ROOT
}

/// Whether a word carries an ANCESTOR of the envelope directory — at index ZERO
/// or at a NON-ZERO index — with the same four literalness exclusions class 12
/// applies, and for the same reason.
///
/// **THE ANCESTOR SET IS EXACTLY `{<ENV>}` AND THE PREDICATE SAYS SO BY
/// CONSTRUCTION.** `envelope_dir_in(root, alias)` is `<root>/<alias>` and the
/// alias is a PLAIN SINGLE PATH COMPONENT — `ledger_path_in` refuses anything
/// else — so the only proper ancestor of the envelope directory that lies at or
/// under the root is the root itself.
fn word_carries_an_ancestor_of_the_envelope_directory(text: &str) -> bool {
    if text.contains('$')
        || text.starts_with('~')
        || word_has_pathname_metacharacter(text)
        || word_has_brace_list(text)
    {
        return false;
    }
    std::iter::once(0)
        .chain(
            text.char_indices()
                .filter(|(index, character)| *index > 0 && *character == '/')
                .map(|(index, _)| index),
        )
        .any(|index| word_names_the_envelope_root_exactly(&text[index..]))
}

/// One word of a control-carrier command, with the two facts these predicates
/// need: whether it is a redirection TARGET, and whether it was QUOTED.
struct CarrierWord {
    text: String,
    is_redirection_target: bool,
    /// **NEW IN ROUND 13 — which SIDE of bash's twelve-operator split the
    /// preceding operator falls on.** Class 2 requires a PATHNAME operator and
    /// class 13 requires a NON-PATHNAME one, and without this field the two
    /// cannot be told apart: `is_redirection_target` is TRUE for all twelve.
    redirection_is_pathname: bool,
    /// **NEW IN ROUND 13 — the target text of a NON-PATHNAME redirection
    /// operator, in EITHER spelling**, or `None`.
    non_pathname_target: Option<String>,
    quoted: bool,
}

/// Split a control-carrier command into [`CarrierWord`]s.
///
/// A whitespace split rather than a re-implementation of `tokenize`: this is a
/// CORPUS predicate deciding which CLASS an entry draws, not a guard decision.
/// The guard's own tokenizer is `policy::split_segments_with_heads`, and the
/// reachability proof below uses it directly rather than this.
fn carrier_words(command: &str) -> Vec<CarrierWord> {
    let raw: Vec<&str> = command.split_whitespace().collect();
    let mut words = Vec::new();
    for (index, word) in raw.iter().enumerate() {
        let previous = index.checked_sub(1).map(|i| raw[i]).unwrap_or("");
        words.push(CarrierWord {
            text: (*word).to_string(),
            is_redirection_target: REDIRECTION_OPERATORS.contains(&previous),
            redirection_is_pathname: PATHNAME_REDIRECTION_OPERATORS.contains(&previous),
            non_pathname_target: non_pathname_redirection_target(previous, word)
                .map(|target| target.to_string()),
            quoted: word.starts_with('\'') || word.starts_with('"'),
        });
    }
    words
}

/// The final path component of `word`, for a carrier-basename test.
fn carrier_basename(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// **CLASS 1** — a non-governed command whose ABSOLUTE LITERAL word OPERAND
/// resolves under the envelope root. **The class the rule closes.**
///
/// A redirection TARGET is deliberately excluded: bash deletes it before
/// `execve`, so it is not an operand at all. That exclusion IS class 2.
///
/// **REFINED BY ROUND 11: a word carrying a PATHNAME-EXPANSION metacharacter or
/// a BRACE LIST is excluded and handed to classes 9 and 10.** The word
/// `<ENV>/alph?` starts with the root and satisfied this predicate before the
/// refinement — but it is NOT `Token.literal`, so rule (a) does not reach it and
/// putting it here would count a PERMITTED entry inside a fail-closed class.
/// The refinement is what lets the two live on the same axis without collapsing.
///
/// **EXTENDED BY ROUND 13: A WORD NAMING THE ROOT *EXACTLY* IS EXCLUDED AND
/// HANDED TO CLASS 14, AND THE REASON IS A MISATTRIBUTION RATHER THAN TIDINESS.**
/// `REPRESENTATIVE_ENVELOPE_ROOT` is the ROOT, and a word that IS the root
/// `starts_with` itself — so without this refinement every `CONTROL_CARRIER_ANCESTOR`
/// entry would be filed under **the class round 10 CLOSED**, which attributes a
/// live, unruled bypass to a control the record says is closed. That is worse
/// than a wrong arm. This is the same shape of refinement round 11 made when it
/// excluded globs and braces from this class, one silence over, and
/// `19-32` RE-DERIVED MECHANICALLY that **no existing entry names the root
/// exactly** before making it: `grep -rnE '<ENV>([ "]|$)' tests/` returns exactly
/// ONE hit, [`ENVELOPE_ROOT_PLACEHOLDER`]'s own definition.
///
/// **THE ONE DELIBERATE OVERLAP THIS CLASS NOW CARRIES, RECORDED RATHER THAN
/// SMOOTHED AWAY.** Round 13 adds three ANCESTOR-STOP controls to
/// [`CONTROL_CARRIER_ORDINARY_OPERANDS`] — `ls <ENV>/unrelated-sibling`,
/// `rm -f <ENV>/unrelated-sibling` and `rm -rf <ENV>/beta` — which are
/// PERMITTED before and after and which this predicate DOES draw, because they
/// really are absolute literal operands under the envelope ROOT. **That is
/// exactly the distinction `T-19-123` is about**: this class's predicate is over
/// the ROOT while rule (a)'s prefix is over `<root>/<alias>`, and the three rows
/// sit in the gap between them. They are driven in the INVARIANCE arm, no
/// fail-closed property touches them, and the count below states the overlap.
fn draws_an_envelope_root_operand(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        !word.is_redirection_target
            && !word_names_the_envelope_root_exactly(&word.text)
            && !word.quoted
            && word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
            && !word_has_pathname_metacharacter(&word.text)
            && !word_has_brace_list(&word.text)
    })
}

/// **CLASS 2** — a non-governed command whose ABSOLUTE LITERAL REDIRECTION
/// TARGET is a control carrier. **Fail-open direction (i) TODAY, and the class
/// `19-29`'s widened SIGHT closes.**
///
/// **REFINED BY ROUND 11, AND THIS IS THE REFINEMENT WHOSE ABSENCE WOULD HAVE
/// DONE MORE THAN FIRE A FENCE.** The predicate was
/// `is_redirection_target && starts_with(ROOT)` with **no literalness guard at
/// all**, so this round's own `: > <ENV>/alpha/pr-ledger.ndjso?` and
/// `: > <ENV>/alpha/{pr-ledger.ndjson,x}` entries satisfied it — **and class 2's
/// alphabet is simultaneously the one MOVING to the fail-closed arm**, so the
/// collision would have placed PERMITTED glob and brace targets inside a
/// property that asserts REFUSAL. The metacharacter and brace-list spellings are
/// handed to classes 9 and 10, which are verdict-PRESERVING.
///
/// **EXTENDED BY ROUND 13 TO REQUIRE A *PATHNAME* OPERATOR, AND THIS IS THE
/// SECOND TIME THE COLLISION HAS BEEN FORCED THROUGH CLASS 2 SPECIFICALLY.**
/// The predicate asked only `is_redirection_target && starts_with(ROOT)`, and
/// `is_redirection_target` is TRUE for ALL TWELVE of bash's operators because
/// [`REDIRECTION_OPERATORS`] holds all twelve. So a here-string carrier such as
/// `xargs rm -rf <<< <ENV>/alpha` satisfied it — **and it would have been filed
/// under the class ROUND 11 CLOSED**, whose alphabet is
/// [`CONTROL_CARRIER_REDIRECTION_TARGETS`] and whose property is GREEN.
/// **Attributing a live, unruled bypass to a closed control is worse than a
/// wrong arm**, because a later reader checking whether the class still fails
/// would find it green and conclude the spelling was covered.
///
/// **THIS IS `19-18`'s `{v}>` BLOCKER, `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS`
/// SPLIT, `19-22`'s INDIRECTION/CONFINED SPLIT, `19-24`'s
/// RE-PARSED/CONFINED SPLIT, `19-28`'s OWN CLASS-2 COLLISION AND `19-30`'s
/// COMPLEMENT EXTENSION — A SEVENTH TIME.** It is stated here as a reason and
/// not only in the record, because the record is not what a future editor reads
/// before changing this predicate.
///
/// **`19-32` RE-DERIVED MECHANICALLY THAT NO EXISTING ENTRY FALLS OUT**, and it
/// recorded the CHECK rather than the conclusion: every entry of
/// [`CONTROL_CARRIER_REDIRECTION_TARGETS`] uses `>` or `>>` and every entry of
/// [`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING`] uses `>`, all of which are
/// PATHNAME operators, so the count below is unchanged at 4.
fn draws_a_redirection_target_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        word.is_redirection_target
            && word.redirection_is_pathname
            && word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
            && !word_has_pathname_metacharacter(&word.text)
            && !word_has_brace_list(&word.text)
    })
}

/// **CLASS 3** — a non-governed command whose carrier operand is
/// EXPANSION-BORNE. **Fail-open direction (ii), NOT closed.**
fn draws_an_expansion_borne_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        word.text.contains('$')
            && CARRIER_BASENAMES.contains(&carrier_basename(&word.text))
            && !word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
    })
}

/// **CLASS 4** — a non-governed command whose carrier operand is a SYMLINK into
/// the envelope root. **Fail-open direction (iii), NARROWED but NOT closed.**
fn draws_a_symlinked_carrier(command: &str) -> bool {
    carrier_words(command)
        .iter()
        .any(|word| word.text.starts_with(SYMLINK_MARKER))
}

/// **CLASS 5** — a non-governed command whose carrier operand is RELATIVE.
/// **Fail-open direction (iv), NARROWED but NOT closed.**
///
/// **REFINED BY ROUND 11: a word beginning with a TILDE is excluded and handed
/// to class 8.** A `~` spelling does not begin with `/`, so it satisfied this
/// predicate before the refinement — but it is not a relative path, it is an
/// ABSOLUTE path the shell has not expanded yet, and the two fail open for
/// completely different reasons: class 5 because the guard has no cwd, class 8
/// because expanding a `~` means reading the ENVIRONMENT the command will run
/// in.
/// **REFINED AGAIN BY ROUND 12: a word carrying a carrier path at a NON-ZERO
/// index is excluded and handed to class 12.** `of=<ENV>/alpha/pr-ledger.ndjson`
/// does not begin with `/`, carries a carrier BASENAME, and satisfied this
/// predicate before the refinement — **but it is not a relative path at all, it
/// is an ABSOLUTE path carried INSIDE a token**, and the two fail open for
/// completely different reasons: class 5 because the guard has no cwd, class 12
/// because the normaliser requires the WORD itself to start with `/`. This is
/// the same kind of refinement round 11 made for the tilde, one silence over.
/// **REFINED A THIRD TIME BY ROUND 13: a word that is a NON-PATHNAME redirection
/// target is excluded and handed to class 13.** `>&<ENV>/alpha/askpass` is ONE
/// whitespace word whose BASENAME is a carrier filename and which does not begin
/// with `/`, so it satisfied this predicate — **but it is not a relative path at
/// all, it is an ABSOLUTE path after a redirection operator**, and the two fail
/// open for completely different reasons: class 5 because the guard has no cwd,
/// class 13 because `consume_redirection` records the word on no `Segment` field.
/// This is the same kind of refinement round 11 made for the tilde and round 12
/// for the interior path, one silence over. **`19-32` re-derived that neither
/// existing entry falls out:** both carry no redirection operator at all.
fn draws_a_relative_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        !word.is_redirection_target
            && word.non_pathname_target.is_none()
            && !word.quoted
            && !word.text.starts_with('/')
            && !word.text.starts_with('~')
            && !word.text.contains('$')
            && !word_carries_an_interior_carrier_path(&word.text)
            && !REPO_SIDE_CARRIER_PATHS
                .iter()
                .any(|path| word.text.contains(path))
            && CARRIER_BASENAMES.contains(&carrier_basename(&word.text))
    })
}

/// **CLASS 6** — a non-governed command whose operand is a REPO-SIDE control
/// carrier. **NO RULE AT ALL — control (e).**
fn draws_a_repo_side_carrier(command: &str) -> bool {
    REPO_SIDE_CARRIER_PATHS
        .iter()
        .any(|path| command.contains(path))
}

/// **CLASS 7** — a non-governed command whose operand is an ORDINARY path.
/// **The permitted control that stops the rule being "refuse every `rm`".**
///
/// Defined as the COMPLEMENT of the TEN carrier classes, and that is correct
/// rather than degenerate: its whole content is "no control carrier is named in
/// any of the ten ways". It is proved non-vacuous below by asserting that its own
/// representative satisfies it and that each of the other ten representatives
/// does not.
///
/// **EXTENDED BY ROUND 11 to exclude classes 8 … 11 as well.** Without that, a
/// tilde, glob, brace or BINARY entry would satisfy class 7 too and the counts
/// would call a live carrier "an ordinary path operand", which is untrue.
///
/// **EXTENDED AGAIN BY ROUND 12 TO EXCLUDE CLASS 12, AND THE REASON IS THE
/// STRUCTURAL COLLISION THIS SPLIT HAS BEEN FORCED BY SIX TIMES.** An
/// option-attached carrier word such as `dd of=<ENV>/alpha/pr-ledger.ndjson`
/// satisfies class 1 NOT (it does not `starts_with` the root), class 2 NOT (it
/// is not a redirection target) and class 11 NOT (that is an EQUALITY over the
/// whole word) — **and therefore satisfies class 7, the COMPLEMENT, which is in
/// the INVARIANCE arm.** That would assert PERMITTED a row `19-31` REFUSES,
/// landing permanently red in a file `19-31` may only ADD to. This is `19-18`'s
/// `{v}>` blocker, `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split, `19-22`'s
/// indirection/confined split, `19-24`'s re-parsed/confined split and `19-28`'s
/// own class-2 collision, **a SIXTH time — and the SECOND time through the
/// COMPLEMENT specifically.**
///
/// **The arithmetic proof that the extension took effect is class 7's own
/// COUNT**: it is 24 over the 84 generated commands, where the naive count
/// without this clause would be 36. See [`CONTROL_CARRIER_CLASS_COUNTS`].
///
/// **EXTENDED AGAIN BY ROUND 13 TO EXCLUDE CLASSES 13 AND 14, AND THE REASON IS
/// THE SAME STRUCTURAL COLLISION FOR A SEVENTH TIME.** A here-string carrier
/// satisfies class 1 NOT (its word is a redirection target), class 2 NOT (with
/// round 13's pathname requirement) and class 11 NOT (with round 13's
/// exclusion); an ANCESTOR word satisfies class 1 NOT (with round 13's
/// exact-root exclusion) and class 12 NOT (with round 13's ancestor guard).
/// **So without this clause both would fall into the COMPLEMENT, which is in the
/// INVARIANCE arm** — asserting PERMITTED a row `19-33` REFUSES, landing
/// permanently red in a file `19-33` may only ADD to.
///
/// **THE EXCLUSION IS WRITTEN SO IT DOES NOT DEPEND ON THE ORDER THE OTHER
/// NARROWINGS WERE APPLIED.** Classes 13 and 14 are named DIRECTLY here rather
/// than left to fall out of classes 1, 2, 11 and 12 having been refined — so a
/// future round that widened one of those four back could not silently return an
/// entry to the invariance arm, and the property does not have to be re-derived
/// every round.
///
/// **The arithmetic proof that THIS extension took effect is class 7's own
/// count**: it is 29 over the 107 generated commands, where the naive count
/// without the two new clauses would be 43. **That fall of FOURTEEN is the
/// proof**, and it is exactly the seven class-13 and seven class-14 entries.
fn draws_an_ordinary_operand(command: &str) -> bool {
    !draws_an_envelope_root_operand(command)
        && !draws_a_redirection_target_carrier(command)
        && !draws_an_expansion_borne_carrier(command)
        && !draws_a_symlinked_carrier(command)
        && !draws_a_relative_carrier(command)
        && !draws_a_repo_side_carrier(command)
        && !draws_a_tilde_borne_carrier(command)
        && !draws_a_glob_borne_carrier(command)
        && !draws_a_brace_borne_carrier(command)
        && !draws_the_guards_own_binary(command)
        && !draws_an_interior_carrier_path(command)
        && !draws_a_non_pathname_target_carrier(command)
        && !draws_an_ancestor_of_the_envelope_directory(command)
}

/// **CLASS 8** — a carrier word spelled with a TILDE, in EITHER word position.
/// **Fail-open direction (v) — one of the THREE the docs do not name.**
///
/// **`19-29` WRITES NO RULE FOR THIS CLASS, AND THE REASON IS MECHANICAL.**
/// Expanding a `~` means reading the ENVIRONMENT the command will run in, and
/// the guard is forbidden that (`hooks.rs:771-816`). The fix is the residue's
/// ARITHMETIC: the docs say FOUR directions and there are SEVEN.
///
/// **It draws in BOTH word positions — the cell nobody had drawn.**
fn draws_a_tilde_borne_carrier(command: &str) -> bool {
    carrier_words(command)
        .iter()
        .any(|word| word_is_tilde_borne_carrier(&word.text))
}

/// **CLASS 9** — a carrier word spelled with a PATHNAME-EXPANSION
/// metacharacter, in EITHER word position. **Fail-open direction (vi).**
///
/// **`19-29` WRITES NO RULE FOR THIS CLASS EITHER**: expanding a glob means
/// reading the FILESYSTEM on the guard's critical path, which is the I/O
/// `hooks.rs:771-816` forbids and the TOCTOU `mod.rs:196-203` forbids.
fn draws_a_glob_borne_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
            && word_has_pathname_metacharacter(&word.text)
    })
}

/// **CLASS 10** — a carrier word spelled as a BRACE LIST, in EITHER word
/// position. **Fail-open direction (vii).**
///
/// Round 5's literalness table names BRACE as its fourth class and round 10's
/// corpus carried not one.
fn draws_a_brace_borne_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT) && word_has_brace_list(&word.text)
    })
}

/// **CLASS 11** — a command naming the GUARD'S OWN BINARY, in EITHER word
/// position. **`C-10`, `T-19-116`, `high` — and the class `19-29`'s widened
/// PATH SET closes.**
///
/// **THE COMPARISON IS EXACT-PATH AND NOT A PREFIX**, and the predicate says so
/// mechanically: `REPRESENTATIVE_BINARY_PARENT` is a prefix of
/// `REPRESENTATIVE_BINARY_PATH`, so a `starts_with` here would draw the
/// near-miss controls too and this class would silently assert them refused.
///
/// **This class EXISTS because class 1 does not draw the binary — it is not
/// under the envelope root — and class 7, the complement, WOULD.** An entry
/// asserted PERMITTED by class 7's arm is an entry `19-29` refuses, landing
/// permanently red in a file `19-29` may not edit.
/// **EXTENDED BY ROUND 13 TO EXCLUDE A COMMAND CLASS 13 DRAWS, FOR THE SAME
/// MISATTRIBUTION REASON AS CLASSES 1 AND 2.** `xargs cp /bin/true <<< <BIN>` is
/// a here-string whose target word IS the binary, so it satisfied this EQUALITY
/// — and this class's alphabet, [`CONTROL_CARRIER_BINARY`], is the one **round
/// 11 CLOSED**. **`19-32` RE-DERIVED MECHANICALLY that no existing entry falls
/// out**: none of `CONTROL_CARRIER_BINARY`'s five entries follows a NON-PATHNAME
/// operator (four are operands and the fifth is a `>` target), so the count
/// below is unchanged at 5 where the naive count would be 6.
fn draws_the_guards_own_binary(command: &str) -> bool {
    // **The clause is written ABOVE the existing expression rather than inside
    // it, so the three lines below stay BYTE-IDENTICAL** — `19-32` may add lines
    // under `tests/` and may delete none, and re-indenting an existing line is a
    // deletion.
    !draws_a_non_pathname_target_carrier(command) &&
    carrier_words(command)
        .iter()
        .any(|word| word.text == REPRESENTATIVE_BINARY_PATH)
}

/// Whether a word carries a carrier path as a `/`-ANCHORED SUBSTRING at a
/// **NON-ZERO** index — the text fact **CLASS 12** is about.
///
/// **IT ASKS NOTHING ABOUT WHAT PRECEDES THE `/`, AND THAT IS THE WHOLE POINT.**
/// `19-31`'s mandated boundary is `/`-anchored substrings of a LITERAL word, not
/// a list of attachment characters and not a list of option spellings. An
/// `=`-split would miss `--opt=a=/p` (a second `=`), `tar -C/p` and `cp -t/p`
/// (attached SHORT options with no `=` at all) and `host:/p` (a `:`); **and a
/// character list would be a PROGRAM-GRAMMAR ENUMERATION — `dd`'s `of=` is an
/// operand grammar, not an option syntax — which is D-08's defect one level
/// over**, the same shape
/// `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
/// exists to forbid. So this predicate walks every `/` and asks only whether the
/// substring from it is a carrier path.
///
/// **THE FOUR EXCLUSIONS ARE WHAT KEEP CLASS 12 DISJOINT FROM CLASSES 3, 8, 9
/// AND 10.** A word carrying `$`, a leading `~`, a pathname-expansion
/// metacharacter or a brace list is NOT `Token.literal`, rule (a) requires that
/// bit, and `19-31` applies the LITERAL FILTER BEFORE ANY CANDIDATE IS PRODUCED.
/// Those spellings stay verdict-PRESERVING and belong to the classes that
/// already draw them.
fn word_carries_an_interior_carrier_path(text: &str) -> bool {
    if text.contains('$')
        || text.starts_with('~')
        || word_has_pathname_metacharacter(text)
        || word_has_brace_list(text)
    {
        return false;
    }
    // **EXTENDED BY ROUND 13: A WORD WHOSE INTERIOR PATH IS THE ROOT ITSELF IS
    // AN *ANCESTOR* AND IS HANDED TO CLASS 14.** `-C<ENV>` carries
    // `/tmp/gsd-envelope-root` at a non-zero index and `starts_with(ROOT)` is
    // true of the root itself, so without this it would be filed under **class
    // 12 — the class `19-31` CLOSED** — while being a live, unruled bypass. That
    // is the same misattribution classes 1, 2 and 11 are refined against, and it
    // is the fourth and last of them. **`19-32` re-derived that no existing
    // entry falls out:** every `CONTROL_CARRIER_INTERIOR_PATH` entry carries a
    // path STRICTLY UNDER the root (`<ENV>/alpha` or deeper) or the BINARY, so
    // the count below is unchanged except for the one entry round 13 ADDS.
    if word_carries_an_ancestor_of_the_envelope_directory(text) {
        return false;
    }
    // **AND A WORD THAT IS ITSELF A NON-PATHNAME REDIRECTION TARGET IN THE
    // *ATTACHED* SPELLING BELONGS TO CLASS 13.** `>&<ENV>/alpha/pr-ledger.ndjson`
    // is ONE whitespace word, so `carrier_words`' `is_redirection_target` — which
    // reads the PREVIOUS word — is FALSE for it, and the `/` at index 2 makes it
    // look like an interior carrier path. **It is not: the path is a redirection
    // TARGET and the operator is one bash answers `pathname_target = false`
    // for.** Filing it under class 12 would attribute a live, unruled bypass to
    // **the class `19-31` CLOSED**, which is the same misattribution classes 1,
    // 2 and 11 are refined against. **`19-32` re-derived that no existing entry
    // falls out:** no `CONTROL_CARRIER_INTERIOR_PATH` entry begins with `<<<`,
    // `<<-`, `<<`, `>&` or `<&`.
    if non_pathname_redirection_target("", text).is_some() {
        return false;
    }
    text.char_indices()
        .filter(|(index, character)| *index > 0 && *character == '/')
        .any(|(index, _)| {
            let candidate = &text[index..];
            candidate.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                || candidate == REPRESENTATIVE_BINARY_PATH
        })
}

/// **CLASS 12** — a non-governed command carrying a carrier path INSIDE a
/// literal word, at a NON-ZERO index. **`T-19-119`, `high` — and the class
/// `19-31`'s widened READING closes.**
///
/// **DISJOINT FROM CLASS 1 BY CONSTRUCTION:** class 1 is
/// `starts_with(REPRESENTATIVE_ENVELOPE_ROOT)`, which is index ZERO, and this
/// predicate only looks at indices ABOVE zero. **DISJOINT FROM CLASS 11 BY
/// CONSTRUCTION:** class 11 is an EQUALITY over the WHOLE word, and a word whose
/// carrier path starts above index zero is longer than the path it carries.
///
/// **DISJOINT FROM CLASS 2 BY EXCLUSION, AND THE EXCLUSION IS STATED RATHER THAN
/// LEFT AS SYMMETRY.** A redirection target is a WHOLE WORD by construction —
/// bash parses `> WORD` and the word IS the pathname — so an interior path in a
/// redirection target is not a shape bash produces. `19-30`'s attachment sweep
/// is the evidence, and this is the same kind of deliberate absence
/// [`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING`] already records for a
/// relative target.
fn draws_an_interior_carrier_path(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        !word.is_redirection_target && word_carries_an_interior_carrier_path(&word.text)
    })
}

/// **CLASS 13** — a non-governed command whose carrier path is carried by a word
/// after one of bash's FIVE **NON-PATHNAME** redirection operators.
/// **`T-19-122`, `high` — and the class `19-33`'s widened WORD SET closes.**
///
/// # WHY IT NEEDS A CLASS OF ITS OWN: THE WORD IS IN *NEITHER* WORD CLASS
///
/// `consume_redirection` (`policy.rs:3532-3609`) consumes the target word and
/// emits **no `Token`** — that is round 6's deletion model, unchanged and
/// correct — and pushes it onto `Segment::redirection_targets` only under
/// `if pathname_target && target.literal` (`:3581`). `redirection_operator`
/// (`:3205`) answers `pathname_target = false` for `<<<`, `<<`, `<<-`, `>&` and
/// `<&`. **So the word is in NEITHER of the two classes
/// `protected_carrier_named` iterates at `policy.rs:6118-6128`**, while being
/// fully `Token.literal`, ABSOLUTE at index zero, needing no cwd and no link,
/// and naming the directory EXACTLY — **none of the three declared silences.**
///
/// # THE PREDICATE DECIDES ON THE WORD'S TEXT, ON THE CARRIER PATHS, AND ON
/// WHICH OPERATOR PRECEDES IT
///
/// It reads bash's own seven/five split (`policy.rs:3193-3229`) rather than
/// inventing a second list — the licence [`DISPLACING_REDIRECTIONS`] already
/// exercises for a corpus-side classifier — and it accepts the path at index
/// ZERO **or** at a NON-ZERO index, **so it COMPOSES with class 12's text fact
/// rather than duplicating it.**
///
/// # THE FOUR EXCLUSIONS KEEP IT DISJOINT FROM CLASSES 3, 8, 9 AND 10
///
/// A word carrying `$`, a leading `~`, a pathname-expansion metacharacter or a
/// brace list is NOT `Token.literal`; rule (a) requires that bit, and `19-33`
/// applies the literal filter BEFORE any candidate is produced — indeed
/// `skip_redirection_target` computes the SAME bit for a redirection target by
/// the SAME `REWRITING_CHARACTERS` classification, which is why the third word
/// class inherits the exclusion by construction.
fn draws_a_non_pathname_target_carrier(command: &str) -> bool {
    carrier_words(command).iter().any(|word| {
        let Some(target) = word.non_pathname_target.as_deref() else {
            return false;
        };
        if target.contains('$')
            || target.starts_with('~')
            || word_has_pathname_metacharacter(target)
            || word_has_brace_list(target)
        {
            return false;
        }
        std::iter::once(0)
            .chain(
                target
                    .char_indices()
                    .filter(|(_, character)| *character == '/')
                    .map(|(index, _)| index),
            )
            .any(|index| {
                let candidate = &target[index..];
                candidate.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                    || candidate == REPRESENTATIVE_BINARY_PATH
            })
    })
}

/// **CLASS 14** — a non-governed command naming a proper **ANCESTOR** of the
/// envelope directory that is itself at or under the envelope root.
/// **`T-19-123`, `high` — and the class `19-33`'s widened PATH SET closes.**
///
/// # WHY IT NEEDS A CLASS OF ITS OWN
///
/// `word_is_within` (`policy.rs:5685-5711`) is a component-wise PREFIX over
/// `<root>/<alias>` and returns early on `word.len() < dir.len()` (`:5706`), so
/// **every ANCESTOR — being SHORTER — answers `false`**, while
/// `rm -rf <root>` takes the same nine carriers in ONE call that the prefix
/// boundary exists to protect.
///
/// **THE CLASS IS DISJOINT FROM CLASS 12 BY REFINEMENT AND FROM CLASS 11 BY
/// CONSTRUCTION.** Class 12 now excludes a word whose interior path IS the root
/// (see [`word_carries_an_interior_carrier_path`]); class 11 is an EQUALITY over
/// the binary, which is not under the envelope root at all. It is disjoint from
/// class 1 because class 1 now excludes a word naming the root exactly.
///
/// **IT DRAWS AT INDEX ZERO *AND* AT A NON-ZERO INDEX**, so `tar -C<ENV> …`
/// composes the ancestor with round 12's interior scan — which is the row that
/// proves the two mechanisms meet at one site rather than being two rules.
fn draws_an_ancestor_of_the_envelope_directory(command: &str) -> bool {
    carrier_words(command)
        .iter()
        .any(|word| !word.quoted && word_carries_an_ancestor_of_the_envelope_directory(&word.text))
}

/// One named class and the predicate that decides whether a command draws it.
type ControlCarrierClass = (&'static str, fn(&str) -> bool);

/// The ELEVEN classes of the CONTROL-CARRIER axis — **the FIFTH axis** — named
/// once so the per-alphabet floor, the per-class floor and the counted floor all
/// count the same thing.
///
/// **ROUND 11 TOOK IT FROM SEVEN TO ELEVEN, AND FOUR OF THE CHANGES WERE FORCED
/// BY MEASUREMENT RATHER THAN CHOSEN FOR SYMMETRY.** Classes 8, 9 and 10 are the
/// three spellings that clear `Token.literal` without a `$` — the classes round
/// 5's own literalness table names and round 10's corpus could not draw. Class
/// 11 is `C-10`, the carrier OUTSIDE the envelope root that class 1 cannot draw
/// and class 7 would have.
///
/// **Class 7 stays at index 6**, because it is the COMPLEMENT and the
/// existing-axis fence at (C) below skips it by index with its reason stated.
///
/// **ROUND 12 TOOK IT TO TWELVE AND ROUND 13 TAKES IT TO FOURTEEN, AND BOTH NEW
/// CLASSES ARE ON *THIS* AXIS RATHER THAN A SIXTH ONE.** Rule (a) decides on a
/// word's PATH — so **which words reach the rule** (class 13) and **which paths
/// it protects** (class 14) are the same axis's own alphabet, not a new stage.
/// Standing either up as a sixth axis would lose exactly the distinction audit 8
/// drew when round 9 correctly refused one, and folding them onto
/// `CONFIG_RESOLUTION_CLASSES`, `UNREADABLE_CLASSES`, `DELETION_CLASSES` or
/// `CALLEE_GRAMMAR_CLASSES` would put a command that reaches NO governed program
/// on an axis every one of whose properties splices into a governed base.
///
/// **The three silences these fourteen classes now span, stated once:**
///
/// ```text
/// AXIS 1  WHICH WORDS REACH THE READER      classes 2 and 13
/// AXIS 2  WHAT THE READER SEES IN A WORD    classes 3, 5, 8, 9, 10, 12
/// AXIS 3  WHICH PATHS THE COMPARISON        classes 1, 4, 6, 11, 14
///         PROTECTS
/// ```
const CONTROL_CARRIER_CLASSES: &[ControlCarrierClass] = &[
    (
        "an absolute literal operand under the envelope root",
        draws_an_envelope_root_operand,
    ),
    (
        "a redirection target under the envelope root",
        draws_a_redirection_target_carrier,
    ),
    (
        "an expansion-borne carrier operand",
        draws_an_expansion_borne_carrier,
    ),
    (
        "a symlinked carrier operand",
        draws_a_symlinked_carrier,
    ),
    ("a relative carrier operand", draws_a_relative_carrier),
    ("a repo-side control carrier", draws_a_repo_side_carrier),
    ("an ordinary path operand", draws_an_ordinary_operand),
    ("a tilde-borne carrier word", draws_a_tilde_borne_carrier),
    ("a glob-borne carrier word", draws_a_glob_borne_carrier),
    ("a brace-list-borne carrier word", draws_a_brace_borne_carrier),
    ("the guard's own binary", draws_the_guards_own_binary),
    (
        "a carrier path INSIDE a literal word, at a non-zero index",
        draws_an_interior_carrier_path,
    ),
    (
        "a carrier path after a NON-PATHNAME redirection operator",
        draws_a_non_pathname_target_carrier,
    ),
    (
        "an ANCESTOR of the envelope directory, at or under the root",
        draws_an_ancestor_of_the_envelope_directory,
    ),
];

/// Whether a command draws ANY of the eleven.
fn carries_a_control_carrier_class(command: &str) -> bool {
    CONTROL_CARRIER_CLASSES
        .iter()
        .any(|(_, predicate)| predicate(command))
}

/// One representative command per class, in class order.
const CONTROL_CARRIER_REPRESENTATIVES: &[&str] = &[
    "rm -f <ENV>/alpha/pr-ledger.ndjson",
    ": > <ENV>/alpha/pr-ledger.ndjson",
    "D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson",
    "rm -f /tmp/link-into-the-envelope-root",
    "rm -f pr-ledger.ndjson",
    "sed -i s/x/y/ .git/config",
    "rm -f /tmp/x",
    "rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha",
    "rm -rf <ENV>/alph?",
    "rm -f <ENV>/alpha/{pr-ledger.ndjson,x}",
    "cp /bin/true <BIN>",
    "dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson",
    "xargs rm -rf <<< <ENV>/alpha",
    "rm -rf <ENV>",
];

/// **THE FAIL-CLOSED ALPHABET, WITH ITS OWN PROPERTY.**
///
/// Entries are WHOLE COMMANDS rather than splice fragments, and **that splice
/// difference is itself the axis**: sections 13-16 splice their alphabets into a
/// GOVERNED base because their claim is about the governed program's argv, while
/// a control-carrier command reaches no governed program and has no base to be
/// spliced into.
///
/// **KEPT OUT OF THE INVARIANCE ARM**, for the reason section 16's doc already
/// gives for `CONFIG_INDIRECTION_CARRIERS` and `CONFIG_REPARSED_VALUE_CARRIERS`:
/// these entries are REFUSED after `19-27` **even where their ORDINARY TWIN is
/// PERMITTED** — the twin fence below asserts every twin permitted today — so an
/// entry here would be STRICTER than its base and would turn the invariance
/// property permanently red in a file `19-27` may not edit. This is `19-18`'s
/// `{v}>` blocker, `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split, `19-22`'s
/// indirection/confined split and `19-24`'s re-parsed/confined split, one KIND
/// over.
///
/// The last two entries are directions (iii) and (iv)'s **MEASURED PARTIAL
/// MITIGATIONS**: both name an envelope path as their OWN operand, so class 1
/// catches them and the two directions are NARROWED rather than open in every
/// spelling.
const ENVELOPE_ROOT_OPERAND_CARRIERS: &[&str] = &[
    "rm -f <ENV>/alpha/pr-ledger.ndjson",
    "truncate -s 0 <ENV>/alpha/pr-ledger.ndjson",
    "cp /dev/null <ENV>/alpha/pr-ledger.ndjson",
    "shred -u <ENV>/alpha/pr-ledger.ndjson",
    "rm -rf <ENV>/alpha",
    "cp /bin/true <ENV>/alpha/hooks/pre-push",
    "cp /dev/null <ENV>/alpha/askpass",
    "cat <ENV>/alpha/pr-ledger.ndjson",
    "ls <ENV>/alpha/hooks",
    "ln -s <ENV>/alpha/pr-ledger.ndjson /tmp/link-into-the-envelope-root",
    "cd <ENV>/alpha",
];

/// **FAIL-OPEN DIRECTION (i) — AND ROUND 11 MOVED THIS ALPHABET OUT OF THE
/// INVARIANCE ARM AND INTO A FAIL-CLOSED ONE.**
///
/// **WHY THE ARM CHANGED, STATED RATHER THAN LEFT TO BE INFERRED.** All four
/// entries carry ABSOLUTE LITERAL targets under the envelope root, and `19-28`
/// measured them REFUSED after `19-29` — the widened SIGHT reaches a pathname
/// redirection target. **Leaving them here would put entries whose verdict the
/// fix CHANGES into the invariance-preserving arm**, which is `19-18`'s `{v}>`
/// blocker, `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split, `19-22`'s
/// indirection/confined split, `19-24`'s re-parsed/confined split and `19-26`'s
/// own envelope-root split, **a SIXTH time**. The verdict-PRESERVING targets —
/// expansion-borne, tilde, glob and brace — live in
/// [`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING`] instead.
///
/// **What has NOT changed is the reason the direction was real.** `tokenize`
/// (`policy.rs:2264-2279`, `T-19-97`) consumes bash's redirection production and
/// emits NO TOKEN for the operator OR ITS TARGET, so the carrier path is not a
/// word at all — and `19-29` does NOT change that. It carries the target on the
/// SEGMENT the way `Segment::redirection_unresolvable` already travels.
/// `SEPARATORS` (`policy.rs:2422` — the constant is BYTE-IDENTICAL and the
/// CITATION was stale, re-derived by `19-33`: `git log -L 2422,2422` returns
/// exactly `84a9b05`, and the old line 2297 now returns `af72137` and says
/// nothing about `SEPARATORS`) still has ONE commit in the whole phase
/// (`84a9b05`), `is_separator(">") == false` by construction, and round 6's
/// over-deletion control still pins `git x2>/tmp/o push --force origin main`
/// PERMITTED. **The SEGMENT-COUNT pins in `tests/envelope_carrier_reach.rs` are
/// what make that falsifiable.**
const CONTROL_CARRIER_REDIRECTION_TARGETS: &[&str] = &[
    ": > <ENV>/alpha/pr-ledger.ndjson",
    "printf 'exit 0' > <ENV>/alpha/hooks/pre-push",
    "echo evil > <ENV>/alpha/askpass",
    "cat /tmp/evil >> <ENV>/alpha/gitconfig",
];

/// **`C-10` — THE BINARY. A FAIL-CLOSED ALPHABET, NEW IN ROUND 11.**
///
/// **WHY IT NEEDS ITS OWN ALPHABET AND CLASS.** `C-10` is OUTSIDE the envelope
/// root, so class 1 does not draw it — and class 7, the complement, WOULD.
/// Leaving it there would assert PERMITTED a row `19-29` refuses, landing
/// permanently red in a file `19-29` may not edit. That is the same shape as
/// putting an entry whose verdict the fix changes into the invariance arm, one
/// carrier over.
///
/// **Its near-miss controls live in [`CONTROL_CARRIER_ORDINARY_OPERANDS`] and
/// are asserted BY NAME by the PATH-PREFIX-NOT-BASENAME fence**, the way
/// `CONTROL_CARRIER_NEAR_MISSES` already is: a clause written as a DIRECTORY
/// PREFIX over the binary's parent turns them red, and only an EXACT-PATH
/// comparison keeps them green.
///
/// The last entry is the REDIRECTION spelling, which needs BOTH of `19-29`'s
/// widenings at once — the widened path set to know the binary is a carrier, and
/// the widened sight to see a redirection target at all.
const CONTROL_CARRIER_BINARY: &[&str] = &[
    "cp /bin/true <BIN>",
    "mv /bin/true <BIN>",
    "install -m 0755 /bin/true <BIN>",
    "ln -f /bin/true <BIN>",
    "printf 'x' > <BIN>",
];

/// The binary's EXACT-PATH-not-PREFIX near-miss controls, asserted BY NAME.
const CONTROL_CARRIER_BINARY_NEAR_MISSES: &[&str] =
    &["cp /bin/true <BINPAR>/some-other-file", "ls <BINPAR>"];

/// **`T-19-119` — THE CARRIER PATH INSIDE THE WORD. A FAIL-CLOSED ALPHABET, NEW
/// IN ROUND 12.**
///
/// **WHY IT NEEDS ITS OWN ALPHABET AND CLASS.** Every entry here satisfies class
/// 1 NOT (no word `starts_with` the root), class 2 NOT (no redirection target)
/// and class 11 NOT (that is an EQUALITY over the whole word) — **so without
/// class 12 every one of them would land in class 7, the COMPLEMENT, which is in
/// the INVARIANCE arm.** That would assert PERMITTED a row `19-31` REFUSES,
/// landing permanently red in a file `19-31` may only ADD to. Same shape as
/// `19-18`'s `{v}>` blocker, `19-20`'s split, `19-22`'s, `19-24`'s and `19-28`'s
/// — the SIXTH time, and the SECOND through the complement.
///
/// **THE ENTRIES ARE DRAWN FROM `19-30`'s MEASURED SWEEP AND FROM NOTHING
/// ELSE.** Every one was driven against the built binary at exit 0 with an empty
/// walk, and its outside-the-path-set twin was driven at exit 0 too, so this arm
/// is not merely drawing commands that were already refused.
///
/// **THE ALPHABET IS DELIBERATELY NOT AN `=` ALPHABET, and the NO-INTERIOR-PATH
/// fence counts that rather than trusting this sentence.** Entries 7, 8 and 9
/// attach the path by something OTHER than `=` — two attached SHORT options with
/// no `=` anywhere, and a `:` inside an assignment prefix — because **an
/// alphabet that only ever drew `=` is an alphabet that certifies an `=`-keyed
/// rule**, and an `=`-keyed rule would be one character short in exactly the way
/// the current one is.
///
/// **Entry 11 carries TWO `=` before the path and is drawn under a program name
/// absent from BOTH production halves.** `19-30` recorded that no real program
/// grammar was found which consumes a two-`=` word as a path — the two reaching
/// two-`=` spellings it did find (`GIT_CONFIG_PARAMETERS=…` and a `-c alias.q=`
/// body) are ALREADY refused today by other clauses, so neither discriminates on
/// this axis — **so this entry fences the rule SHAPE rather than a measured
/// bypass: a first-`=`-split rule misses it and the mandated `/`-anchored scan
/// does not.**
///
/// **CLASS 12 IS NOT DRAWN IN REDIRECTION-TARGET POSITION, and that is stated
/// rather than forced for symmetry.** A redirection target is a WHOLE WORD by
/// construction — bash parses `> WORD` and the word IS the pathname — so an
/// interior path in one is not a shape bash produces. This is the same kind of
/// deliberate absence [`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING`] already
/// records for a relative target.
const CONTROL_CARRIER_INTERIOR_PATH: &[&str] = &[
    // 1-2 — the `=`-attached spellings over BOTH protected paths.
    "dd if=/bin/true of=<BIN>",
    "dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson",
    // 3-6 — the remaining `=`-attached spellings whose reach real `bash`
    //       confirmed, including a READ (`--reference=`).
    "tar --directory=<ENV>/alpha --create --file /tmp/t .",
    "cp --target-directory=<ENV>/alpha /bin/true",
    "chmod --reference=<ENV>/alpha/pr-ledger.ndjson /tmp/x",
    "rsync --temp-dir=<ENV>/alpha /bin/true /tmp/x",
    // 7-8 — ATTACHED SHORT OPTIONS WITH NO `=` AT ALL.
    "tar --create --file /tmp/t -C<ENV>/alpha",
    "cp /bin/true -t<ENV>/alpha",
    // 9 — a `:` attachment INSIDE an assignment prefix; bash execs out of it.
    "PATH=/usr/bin:<ENV>/alpha mytool",
    // 10 — a BARE assignment prefix, which closes `R=<ENV>/alpha; rm -rf $R`.
    "R=<ENV>/alpha",
    // 11 — TWO `=` before the path, under an unknown program name.
    "mytool --opt=a=<ENV>/alpha/pr-ledger.ndjson",
    // 12 — the BINARY behind a DIFFERENT `=` spelling, so the EXACT-PATH half is
    //      reached by more than one option.
    "tar --directory=<BIN> --create --file /tmp/t .",
    // 13 — **ADDED BY ROUND 13, `T-19-126`(ii). See the WR-02 note below.** The
    //      same attached short option as entry 7, WITH the `.` member, which is
    //      what makes it REACH the file under real GNU tar.
    "tar --create --file /tmp/t -C<ENV>/alpha .",
];

/// **WR-02 — APPENDED BY ROUND 13. ENTRY 7 DOES NOT REACH THE FILE, AND THE
/// ALPHABET'S DOC ABOVE SAYS EVERY ENTRY WAS DRAWN FROM `19-30`'s MEASURED
/// SWEEP.**
///
/// Audit 12 measured `tar --create --file /tmp/t -C<ENV>/alpha` — entry 7 as
/// re-spelled — answering *"Cowardly refusing to create an empty archive"* under
/// real GNU tar, because the re-spelling **moved `-C` last and dropped the `.`
/// member**. `19-32` re-measured it and it reproduces: tar exits non-zero and no
/// archive is created.
///
/// **THE CLASS IS NOT VACUOUS** — entries 8 and 9 reach, verified — so this is
/// ONE ROW'S EVIDENTIARY VALUE rather than a hole in the class.
///
/// **THE REPAIR IS AN *ADDED* REACHING ENTRY AND THIS *APPENDED* NOTE, NEVER A
/// DELETION AND NEVER A RE-SPELLING.** Entry 7 stays exactly as it is: **a case
/// that stops being drawn is a case that stops being able to fail**, and entry 7
/// still draws class 12 and still fences the rule shape. Entry 13 is the same
/// attachment WITH the `.` member, measured REACHING — `tar -tf` lists
/// `./pr-ledger.ndjson` out of the archive.
///
/// **AND THE RE-SPELLING'S ORIGINAL MOTIVE IS MOOT, WHICH AUDIT 12 MEASURED.**
/// With a `c`-containing envelope root BOTH the re-spelled and the original
/// forms answer exit 2, because the carrier clause is raised at the top of
/// `classify_segments`' loop, ABOVE program resolution. The re-spelling bought
/// nothing and cost the reach.
const CONTROL_CARRIER_INTERIOR_PATH_WR_02: &str = "\
entry 7 (`tar --create --file /tmp/t -C<ENV>/alpha`) does NOT reach the file \
under real GNU tar — `Cowardly refusing to create an empty archive` — because \
the re-spelling moved `-C` last and dropped the `.` member. Entry 13 is the \
ADDED reaching form. Entry 7 is NOT deleted and NOT re-worded.";

/// **`T-19-122` — THE WORD IN NEITHER WORD CLASS. A FAIL-CLOSED ALPHABET, NEW
/// IN ROUND 13.**
///
/// **WHY IT NEEDS ITS OWN ALPHABET AND CLASS.** Every entry here satisfies
/// class 1 NOT (its carrier word is a redirection target), class 2 NOT (round
/// 13 requires a PATHNAME operator there), class 11 NOT (round 13 excludes a
/// command this class draws) and class 12 NOT (that requires a non-redirection
/// word) — **so without class 13 every one of them would land in class 7, the
/// COMPLEMENT, which is in the INVARIANCE arm.** That would assert PERMITTED a
/// row `19-33` REFUSES, landing permanently red in a file `19-33` may only ADD
/// to. Same shape as `19-18`'s `{v}>` blocker, `19-20`'s split, `19-22`'s,
/// `19-24`'s, `19-28`'s and `19-30`'s — **the SEVENTH time, and the SECOND
/// through CLASS 2 specifically.**
///
/// **THE ENTRIES ARE DRAWN FROM `19-32`'s MEASURED SWEEP AND FROM NOTHING
/// ELSE.** Every one was driven against the built binary at exit 0 with an empty
/// walk, its OUTSIDE-THE-PATH-SET twin was driven at exit 0 too, and — except
/// where stated — its REACH was demonstrated under real `bash`.
///
/// **THE ALPHABET IS DELIBERATELY NOT A HERE-STRING ALPHABET, and the WORD-SET
/// fence COUNTS that rather than trusting this sentence.** Entries 5 and 6 use
/// `>&`, which is a different one of bash's five non-pathname operators —
/// because **an alphabet that only ever drew `<<<` is an alphabet that certifies
/// a here-string-only rule**, one operator short in exactly the way the current
/// rule is one word class short.
///
/// **ENTRY 7 CARRIES THE PATH AT A NON-ZERO INDEX INSIDE THE HERE-STRING WORD,
/// AND ITS REACH IS NOT CLAIMED.** `xargs` hands `of=<ENV>/alpha` to `rm -rf` as
/// a literal filename, so no file under the envelope directory is opened. **It
/// fences the rule SHAPE rather than a measured bypass** — a rule that read only
/// the WHOLE here-string word would miss it and the mandated design does not —
/// exactly as entry 11 of [`CONTROL_CARRIER_INTERIOR_PATH`] already does.
///
/// **TWO OPERATORS ARE A RECORDED DELIBERATE ABSENCE, with their reason**, the
/// way [`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING`] already records one:
/// `<<` and `<<-` WITH THE PATH AS THE DELIMITER are permitted by the guard and
/// **REACH NOTHING** (bash warns *here-document at line 1 delimited by
/// end-of-file* and the body is empty), and `<&` with a path answers *ambiguous
/// redirect*. **A spelling the guard permits but the shell does not reach is not
/// a bypass**, so all three are RECORDED in `tests/envelope_word_set.rs`'s
/// section 14 rather than drawn here.
///
/// **THE HEREDOC *BODY* FORM IS ABSENT FOR THE OPPOSITE REASON:** it REACHES,
/// and the guard **already refuses it** at exit 2, because `\n` is a `SEPARATORS`
/// entry and the body is an ordinary segment of its own. That is round 13's
/// CORRECTION to audit 12, and a verdict-PRESERVING row does not belong in a
/// fail-closed arm.
const CONTROL_CARRIER_NON_PATHNAME_TARGET: &[&str] = &[
    // 1-3 — the here-string over the envelope DIRECTORY, the LEDGER and the BINARY.
    "xargs rm -rf <<< <ENV>/alpha",
    "xargs rm -f <<< <ENV>/alpha/pr-ledger.ndjson",
    "xargs cp /bin/true <<< <BIN>",
    // 4 — a SECOND here-string spelling, under a different `xargs` invocation and
    //     over the hook-stub directory, SAFE-05's only carrier.
    "xargs -n1 rm -rf <<< <ENV>/alpha/hooks",
    // 5-6 — **AN OPERATOR OTHER THAN `<<<`.** `>&` with a path is not an fd
    //       duplication at all: bash falls back to `&>word` and OPENS THE FILE.
    //       Measured REACHING — the ledger was truncated to ZERO bytes.
    ": >&<ENV>/alpha/pr-ledger.ndjson",
    "echo evil >&<ENV>/alpha/askpass",
    // 7 — the carrier path at a NON-ZERO index inside the here-string word, so
    //     the third word class and round 12's interior scan are drawn COMPOSED.
    //     Reach NOT claimed; see the doc above.
    "xargs rm -rf <<< of=<ENV>/alpha",
];

/// **`T-19-123` — THE PATH THE COMPARISON NEVER PROTECTS. A FAIL-CLOSED
/// ALPHABET, NEW IN ROUND 13.**
///
/// **WHY IT NEEDS ITS OWN ALPHABET AND CLASS.** A word that IS the envelope root
/// `starts_with` the root, so every entry here satisfied **class 1 — the class
/// ROUND 10 CLOSED** — until round 13 refined class 1 to exclude an exact-root
/// word. Filing a live, unruled bypass under a closed control is worse than a
/// wrong arm.
///
/// **THE ENTRIES ARE DRAWN FROM `19-32`'s MEASURED SWEEP AND EVERY ONE REACHES**,
/// verified under real `bash`: `rm -rf` and `rm -rf …/` deleted the root and the
/// envelope directory with it; `mv` moved the whole root away; `find -delete`
/// emptied it; `chmod 000` made the envelope directory inaccessible; `tar -C`
/// archived `./alpha/pr-ledger.ndjson`; and `cd <ENV> && rm -rf alpha` deleted
/// the directory from a cwd the guard has no way to resolve.
///
/// **ENTRY 6 CARRIES THE ANCESTOR AT A NON-ZERO INDEX**, so the ancestor clause
/// and round 12's interior scan are drawn COMPOSED rather than assumed — which
/// is what would turn this file red if `19-33` wrote the ancestor comparison
/// OUTSIDE the per-candidate walk.
///
/// **ENTRY 7 IS THE ROW THAT DEFEATS THE NARROWING `protected_carrier_named`'s
/// OWN DOC CLAIMS.** The doc says `cd <env>/<alias> && rm -f pr-ledger.ndjson`
/// *"is refused BY SEGMENT ONE"* — measured TRUE — but one component up, segment
/// one names the ANCESTOR and nothing refuses it.
///
/// **THE STOP IS PINNED FROM THE OTHER SIDE IN
/// [`CONTROL_CARRIER_ORDINARY_OPERANDS`]**, by `ls <ENV>/unrelated-sibling`,
/// `rm -rf <ENV>/beta` and `ls <ENVPAR>` — **so a PREFIX widened to the root
/// instead of an ANCESTOR clause beside it turns THIS FILE red rather than
/// turning a driven run unusable.** The ANCESTOR-STOP fence asserts the pair by
/// name.
const CONTROL_CARRIER_ANCESTOR: &[&str] = &[
    // 1-2 — the root, and its trailing-slash spelling, which normalises identically.
    "rm -rf <ENV>",
    "rm -rf <ENV>/",
    // 3-5 — THREE spellings that are not `rm`, so the class cannot certify a
    //       rule that read a program name. All three REACH.
    "mv <ENV> /tmp/gone",
    "find <ENV> -delete",
    "chmod 000 <ENV>",
    // 6 — the ancestor at a NON-ZERO index, composed with the interior scan.
    "tar -C<ENV> --create --file /tmp/t .",
    // 7 — the TWO-SEGMENT composite that defeats the doc's own claimed narrowing.
    "cd <ENV> && rm -rf alpha",
];

/// **THE VERDICT-PRESERVING REDIRECTION TARGETS — NEW IN ROUND 11, in the
/// INVARIANCE arm.**
///
/// `19-29` widens what rule (a) SEES to pathname redirection targets, but the
/// four conditions apply unchanged over the widened sight: **a target that is
/// not `Token.literal`, or does not resolve under a carrier path, stays
/// PERMITTED.** These four are exactly that residue, one per remaining
/// direction, and pinning them at exit 0 BEFORE AND AFTER is what makes a rule
/// that quietly widened past its stated boundary turn this file RED.
///
/// **A RELATIVE redirection target is deliberately ABSENT, and the reason is
/// recorded rather than left as a gap.** Class 5 is an OPERAND class — round 11
/// refined it to exclude tildes and did not widen it to redirection targets — so
/// `: > pr-ledger.ndjson` would draw NO carrier class and be counted as class 7,
/// "an ordinary path operand", which is untrue. The relative direction is drawn
/// in operand position by [`CONTROL_CARRIER_RELATIVE`].
const CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING: &[&str] = &[
    "H=$(git config --get core.hooksPath); : > $H/../pr-ledger.ndjson",
    ": > ~/.local/share/gsd-meta-manager/envelope/alpha/pr-ledger.ndjson",
    ": > <ENV>/alpha/pr-ledger.ndjso?",
    ": > <ENV>/alpha/{pr-ledger.ndjson,x}",
];

/// **FAIL-OPEN DIRECTION (v) — TILDE. NEW IN ROUND 11, verdict-PRESERVING.**
///
/// **One of the THREE directions the docs do not name.** Round 5's
/// `Token.literal` table names EXPANSION, PATHNAME, TILDE and BRACE as the four
/// classes that clear the bit; rule (a) requires the bit; and round 10's corpus
/// drew only the first. `19-29` writes NO rule here — expanding a `~` means
/// reading the ENVIRONMENT the command will run in — so the fix is the residue's
/// ARITHMETIC: the docs say FOUR directions and there are SEVEN.
///
/// **It draws in BOTH word positions**, which is the cell nobody had drawn. The
/// third entry is under `shred`, a program name absent from both production
/// halves.
const CONTROL_CARRIER_TILDE_BORNE: &[&str] = &[
    "rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha",
    "cp /bin/true ~/.local/share/gsd-meta-manager/envelope/alpha/hooks/pre-push",
    "shred -u ~/.local/share/gsd-meta-manager/envelope/alpha/pr-ledger.ndjson",
    "echo evil > ~/.local/share/gsd-meta-manager/envelope/alpha/askpass",
];

/// **FAIL-OPEN DIRECTION (vi) — PATHNAME EXPANSION. NEW IN ROUND 11,
/// verdict-PRESERVING.**
///
/// `19-29` writes no rule here either: expanding a glob means reading the
/// FILESYSTEM on the guard's critical path. Measured live: `rm -rf <ENV>/alph?`
/// takes NINE carriers in one call at exit 0 while its absolute-literal twin is
/// exit 2. The third entry is under `unlink`, absent from both production halves.
const CONTROL_CARRIER_GLOB_BORNE: &[&str] = &[
    "rm -rf <ENV>/alph?",
    "rm -f <ENV>/alpha/*",
    "unlink <ENV>/alpha/pr-ledger.ndjso?",
    "printf 'exit 0' > <ENV>/alpha/hooks/pre-pus?",
];

/// **FAIL-OPEN DIRECTION (vii) — BRACE LIST. NEW IN ROUND 11,
/// verdict-PRESERVING.**
///
/// **AND THE RECORD THAT ONE OF THESE COSTS NOTHING, WHICH IS THE HONEST HALF.**
/// `tests/envelope_carrier_reach.rs` drove each spelling under a real `bash`:
/// the OPERAND spellings all REACH the file, and so does a glob in
/// redirection-target position when it matches exactly one path — but
/// `: > <ENV>/alpha/{pr-ledger.ndjson,x}` does **NOT**, because brace expansion
/// produces TWO words and a redirection target must be ONE, so bash answers
/// `ambiguous redirect`. **The entry stays — it is verdict-preserving either way
/// and the axis must be able to DRAW the spelling — and the record says which
/// kind of permit it is.** The third entry is under `shred`.
const CONTROL_CARRIER_BRACE_BORNE: &[&str] = &[
    "rm -f <ENV>/alpha/{pr-ledger.ndjson,x}",
    "rm -rf <ENV>/{alpha,beta}",
    "shred -u <ENV>/alpha/{askpass,x}",
    "echo evil > <ENV>/alpha/{askpass,x}",
];

/// **FAIL-OPEN DIRECTION (ii)** — verdict-PRESERVING, in the invariance arm.
///
/// **Rule (a)'s silence is a permit here too.** The operand's value comes from a
/// command substitution in a previous segment. Refusing every non-literal operand
/// of an UNGOVERNED command would deny `rm $TMPDIR/x` and `cp "$SRC" "$DST"`,
/// which is how a safety control gets switched off (AR-19-11).
const CONTROL_CARRIER_EXPANSION_BORNE: &[&str] = &[
    "D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson",
    "H=$(git config --get core.hooksPath); cp /bin/true $H/pre-push",
];

/// **FAIL-OPEN DIRECTION (iii)** — verdict-PRESERVING, in the invariance arm.
///
/// **NARROWED, not closed.** Rule (a) resolves LEXICALLY: `..` is collapsed
/// textually, but a symlink is not followed, because following one means
/// `readlink`/`canonicalize` on the guard's critical path — the I/O
/// `hooks.rs:771-816` forbids and the TOCTOU `mod.rs:196-203` forbids. The
/// MEASURED partial mitigation is `ln -s <ENV>/… <LINK>`, which names an envelope
/// path as its OWN operand and lives in [`ENVELOPE_ROOT_OPERAND_CARRIERS`]. A
/// link must PREDATE the run, or be made by a means naming no envelope path.
const CONTROL_CARRIER_SYMLINKED: &[&str] = &[
    "rm -f /tmp/link-into-the-envelope-root",
    ": > /tmp/link-into-the-envelope-root",
];

/// **FAIL-OPEN DIRECTION (iv)** — verdict-PRESERVING, in the invariance arm.
///
/// **NARROWED, not closed.** The guard has no cwd: `guard_in` takes a
/// `project_root: Option<&Path>` from `PROJECT_ROOT_ENV`, which is the JOURNAL
/// locator and not the shell's working directory. The MEASURED partial mitigation
/// is `cd <ENV>/alpha`, which lives in [`ENVELOPE_ROOT_OPERAND_CARRIERS`]; the
/// two-segment composite `cd <ENV>/alpha && rm -f pr-ledger.ndjson` is refused by
/// SEGMENT 1, and segment 2 stays unresolvable.
const CONTROL_CARRIER_RELATIVE: &[&str] =
    &["rm -f pr-ledger.ndjson", "cp /bin/true pre-push"];

/// **THE REPO-SIDE ALPHABET — control (e), NO RULE AT ALL.**
///
/// It is part of this axis's class structure and of the disjointness fence, and
/// **its guard verdicts are RECORDED rather than ASSERTED**, in either direction.
/// `19-27` writes no rule for `C-11` … `C-15`, so a row asserted REFUSED lands
/// permanently red in a file `19-27` may not edit — and **asserting only that
/// they are PERMITTED is equally forbidden, because that would pin a live bypass
/// as correct.** `19-22` asserted such a row against its own comment, its SUMMARY
/// and its plan-check, and it halted `19-23` mid-plan.
const CONTROL_CARRIER_REPO_SIDE: &[&str] = &[
    "printf '[alias]\\n\\tfp = -c include.path=/tmp/evil.cfg push --force origin \
     HEAD:refs/heads/main\\n' >> .git/config",
    "sed -i s/x/y/ .git/config",
    "cp /tmp/evil .git/config",
    "printf x >> .claude/settings.json",
    "rm -f /tmp/proj/.git/info/exclude",
    "rm -rf /tmp/proj/.planning/meta-manager/runs/run-1",
];

/// **THE ORDINARY-OPERAND ALPHABET** — verdict-PRESERVING, in the invariance arm.
///
/// **This is what stops the rule becoming "refuse every `rm`".** The last two
/// entries are the NEAR-MISS controls — this round's `--signed no` — asserted BY
/// NAME by the PATH-PREFIX-NOT-BASENAME fence below.
const CONTROL_CARRIER_ORDINARY_OPERANDS: &[&str] = &[
    "rm -f /tmp/x",
    "rm -rf /tmp/scratch",
    "cp /bin/true /tmp/t",
    "truncate -s 0 /tmp/f",
    "cat /tmp/x",
    "ls",
    "cargo test",
    "rg 'pr-ledger.ndjson' src/",
    "git config --get core.hooksPath",
    "rm -f /tmp/pr-ledger.ndjson",
    "cat <ENVX>/alpha/pr-ledger.ndjson",
    // **ROUND 11's `--signed no`, FROM THE OTHER SIDE.** The envelope DIRECTORY
    // is a PREFIX boundary; the BINARY is an EXACT-PATH one, because its
    // directory is shared with everything else the user installed. A clause
    // written as a directory prefix over the binary's parent turns both of these
    // red — and would refuse `ls ~/.cargo/bin` and every `cargo install`.
    "cp /bin/true <BINPAR>/some-other-file",
    "ls <BINPAR>",
    // **ROUND 12's COST CONTROLS — THE OTHER SIDE OF `T-19-119`, AND WHAT STOPS
    // THE RULE BEING WRITTEN WRONG.** Every one carries an `=`, a `:`, an
    // attached short option or a `/` — and NONE of them can reach the path set,
    // because a candidate is refused only if it PREFIXES a directory of at least
    // three components rooted at `<root>/<alias>` or EQUALS a full binary path.
    // `word_is_within` returns early on `word.len() < dir.len()` over COMPONENT
    // VECTORS, so a short suffix can never match a deep directory at all.
    //
    // **Each of the first four is the OUTSIDE-THE-PATH-SET TWIN of a
    // `CONTROL_CARRIER_INTERIOR_PATH` entry in the SAME attachment**, which is
    // what makes the fail-closed arm a measurement of the PATH SET rather than
    // of the attachment character.
    "dd if=/tmp/g/x of=/tmp/stolen",
    "tar --create --file /tmp/t -C/tmp/g",
    "cp /bin/true -t/tmp/g",
    "chmod --reference=/tmp/g/x /tmp/y",
    "PATH=/usr/bin:/tmp/g mytool",
    "R=/tmp/g mytool",
    // A word carrying TWO `=` before an UNPROTECTED absolute path.
    "mytool --opt=a=/tmp/g/x",
    // A `%`-formatted value, which carries no `/` at all.
    "printf %H /tmp/x",
    // A substitution whose candidates are `[x, y]`, `[y]` and the EMPTY
    // component list — the degenerate case, handled by the same early return.
    "sed s/x/y/ /tmp/f",
    // A URL operand: `[github.com, o, r]`, `[o, r]`, `[r]` — every one rooted at
    // a component no envelope directory carries.
    "curl https://github.com/o/r",
    // A `:`-attached path in its rsync spelling.
    "rsync host:/tmp/g /tmp/x",
    // **ROUND 13's ANCESTOR-STOP CONTROLS — THE OTHER SIDE OF `T-19-123`, AND
    // WHAT TELLS AN ANCESTOR CLAUSE FROM A PREFIX WIDENED TO THE ROOT.**
    //
    // `19-33`'s mandated clause refuses a candidate that is a proper ANCESTOR of
    // `<root>/<alias>` and is itself at or under `<root>`. The ancestor set is
    // EXACTLY `{<root>}`, because the alias is a plain single path component and
    // `ledger_path_in` refuses anything else — **so the protected set grows by
    // EXACTLY ONE PATH.**
    //
    // **A PREFIX WIDENED TO `<root>` REFUSES ALL THREE OF THESE AND THE
    // ANCESTOR CLAUSE REFUSES NONE.** That is the whole difference between the
    // two designs, and it is drawn here rather than argued. And because
    // `GSD_MM_ENVELOPE_ROOT` is USER-SETTABLE, the prefix design would refuse the
    // whole `/tmp` subtree for a user who set the root there — reachable
    // configuration, not a thought experiment.
    "ls <ENV>/unrelated-sibling",
    "rm -f <ENV>/unrelated-sibling",
    "rm -rf <ENV>/beta",
    // **THE STOP ITSELF.** The root's own PARENT is ABOVE the clause's boundary
    // and NO RULE IS WRITTEN FOR IT. A clause that walked the whole ancestor
    // chain to `/` turns this row red — and refusing `ls /tmp` is an OUTAGE
    // rather than a boundary (AR-19-11). **That residue is registered,
    // disclosed and UNACCEPTED.**
    "ls <ENVPAR>",
    // **ROUND 13's WORD-SET CONTROLS — what keeps class 13 a PATH class rather
    // than a HERE-STRING BAN.** The rejected third design (*treat every
    // deleted-and-unrecorded word as unresolvable*) turns all four of these red,
    // which is why it is rejected in writing rather than left unconsidered.
    "cat <<<x",
    "cat <<< /tmp/x",
    "echo x >&/tmp/plain-outside",
    "xargs rm -rf <<< of=/tmp/g/x",
];

/// The near-miss entries the PATH-PREFIX-NOT-BASENAME fence asserts BY NAME.
const CONTROL_CARRIER_NEAR_MISSES: &[&str] = &[
    "rm -f /tmp/pr-ledger.ndjson",
    "cat <ENVX>/alpha/pr-ledger.ndjson",
];

/// The ONE deliberately GOVERNED entry in the ordinary alphabet, named so the
/// reachability proof states its exception rather than hiding it.
///
/// It is the PERMITTED READ that keeps the disclosed over-refusal family legible:
/// a run cannot inspect its own envelope directory after `19-27`, but it can
/// still NAME it, so a human debugging the run loses nothing the guard's own
/// refusal message does not tell them. Pinned UNCHANGED.
const CONTROL_CARRIER_GOVERNED_TWIN: &str = "git config --get core.hooksPath";

/// Every alphabet on this axis, for the disjointness and class fences.
///
/// **TWELVE after round 11, THIRTEEN after round 12**, and the SLOT count below
/// counts these.
const CONTROL_CARRIER_ALPHABETS: &[(&str, &[&str])] = &[
    ("CONTROL_CARRIER_INTERIOR_PATH", CONTROL_CARRIER_INTERIOR_PATH),
    ("ENVELOPE_ROOT_OPERAND_CARRIERS", ENVELOPE_ROOT_OPERAND_CARRIERS),
    ("CONTROL_CARRIER_REDIRECTION_TARGETS", CONTROL_CARRIER_REDIRECTION_TARGETS),
    ("CONTROL_CARRIER_BINARY", CONTROL_CARRIER_BINARY),
    ("CONTROL_CARRIER_EXPANSION_BORNE", CONTROL_CARRIER_EXPANSION_BORNE),
    ("CONTROL_CARRIER_SYMLINKED", CONTROL_CARRIER_SYMLINKED),
    ("CONTROL_CARRIER_RELATIVE", CONTROL_CARRIER_RELATIVE),
    ("CONTROL_CARRIER_REPO_SIDE", CONTROL_CARRIER_REPO_SIDE),
    ("CONTROL_CARRIER_ORDINARY_OPERANDS", CONTROL_CARRIER_ORDINARY_OPERANDS),
    (
        "CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING",
        CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING,
    ),
    ("CONTROL_CARRIER_TILDE_BORNE", CONTROL_CARRIER_TILDE_BORNE),
    ("CONTROL_CARRIER_GLOB_BORNE", CONTROL_CARRIER_GLOB_BORNE),
    ("CONTROL_CARRIER_BRACE_BORNE", CONTROL_CARRIER_BRACE_BORNE),
    (
        "CONTROL_CARRIER_NON_PATHNAME_TARGET",
        CONTROL_CARRIER_NON_PATHNAME_TARGET,
    ),
    ("CONTROL_CARRIER_ANCESTOR", CONTROL_CARRIER_ANCESTOR),
];

/// **THE FAIL-CLOSED ARM — THREE alphabets after round 11, FOUR after round
/// 12.**
///
/// Every entry here is REFUSED after `19-29` and PERMITTED today, which is what
/// makes the fail-closed property RED before the fix and green after.
/// `CONTROL_CARRIER_REDIRECTION_TARGETS` MOVED here from the invariance arm and
/// `CONTROL_CARRIER_BINARY` is new; `ENVELOPE_ROOT_OPERAND_CARRIERS` was already
/// fail-closed and is unchanged.
const CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS: &[(&str, &[&str])] = &[
    ("ENVELOPE_ROOT_OPERAND_CARRIERS", ENVELOPE_ROOT_OPERAND_CARRIERS),
    ("CONTROL_CARRIER_REDIRECTION_TARGETS", CONTROL_CARRIER_REDIRECTION_TARGETS),
    ("CONTROL_CARRIER_BINARY", CONTROL_CARRIER_BINARY),
    ("CONTROL_CARRIER_INTERIOR_PATH", CONTROL_CARRIER_INTERIOR_PATH),
    // **ROUND 13's TWO. Both are RED today and green only after `19-33`.**
    (
        "CONTROL_CARRIER_NON_PATHNAME_TARGET",
        CONTROL_CARRIER_NON_PATHNAME_TARGET,
    ),
    ("CONTROL_CARRIER_ANCESTOR", CONTROL_CARRIER_ANCESTOR),
];

/// The EIGHT verdict-PRESERVING alphabets the invariance arm DRIVES.
///
/// The ninth verdict-preserving alphabet — `CONTROL_CARRIER_REPO_SIDE` — is
/// RECORDED instead, for the reason stated on its own constant.
///
/// **`CONTROL_CARRIER_REDIRECTION_TARGETS` IS NO LONGER HERE**, and its own doc
/// states why: its four entries' verdict the fix CHANGES, so leaving them in
/// this arm would be `19-18`'s `{v}>` blocker for a sixth time.
const CONTROL_CARRIER_INVARIANCE_ALPHABETS: &[(&str, &[&str])] = &[
    (
        "CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING",
        CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING,
    ),
    ("CONTROL_CARRIER_TILDE_BORNE", CONTROL_CARRIER_TILDE_BORNE),
    ("CONTROL_CARRIER_GLOB_BORNE", CONTROL_CARRIER_GLOB_BORNE),
    ("CONTROL_CARRIER_BRACE_BORNE", CONTROL_CARRIER_BRACE_BORNE),
    ("CONTROL_CARRIER_EXPANSION_BORNE", CONTROL_CARRIER_EXPANSION_BORNE),
    ("CONTROL_CARRIER_SYMLINKED", CONTROL_CARRIER_SYMLINKED),
    ("CONTROL_CARRIER_RELATIVE", CONTROL_CARRIER_RELATIVE),
    ("CONTROL_CARRIER_ORDINARY_OPERANDS", CONTROL_CARRIER_ORDINARY_OPERANDS),
];

/// Substitute the representative roots into an alphabet entry, for the CLASS
/// predicates.
fn control_carrier_representative(entry: &str) -> String {
    entry
        .replace(ENVELOPE_PARENT_PLACEHOLDER, REPRESENTATIVE_ENVELOPE_PARENT)
        .replace(ENVELOPE_ROOT_PLACEHOLDER, REPRESENTATIVE_ENVELOPE_ROOT)
        .replace(NEAR_MISS_ROOT_PLACEHOLDER, REPRESENTATIVE_NEAR_MISS_ROOT)
        .replace(BINARY_PARENT_PLACEHOLDER, REPRESENTATIVE_BINARY_PARENT)
        .replace(BINARY_PLACEHOLDER, REPRESENTATIVE_BINARY_PATH)
}

/// **THE BINARY THIS GUARD IS RUNNING AS**, for the guard-driven properties.
///
/// `std::env::current_exe()` is the same call `hooks::install` (`hooks.rs:92`)
/// makes to decide what to BAKE INTO the stub. Under `cargo test` it is this
/// test binary, which is the right subject: the claim is about *the binary the
/// process answering the guard call is running as*.
fn control_carrier_binary() -> PathBuf {
    std::env::current_exe().expect("the running test binary has a path")
}

/// Substitute a REAL envelope root and the REAL binary into an alphabet entry,
/// for the GUARD-DRIVEN properties.
///
/// **`<BINPAR>` is substituted BEFORE `<BIN>`**, because `<BIN>` is a prefix of
/// neither but the two placeholders share their first four characters and a
/// naive order would leave `<BINPAR>` half-replaced.
fn control_carrier_command(entry: &str, root: &Path) -> String {
    let root = root.display().to_string();
    let mut near_miss = root.clone().into_bytes();
    if let Some(last) = near_miss.last_mut() {
        *last = if *last == b'z' { b'y' } else { b'z' };
    }
    let near_miss = String::from_utf8(near_miss).expect("a temporary path is ASCII");
    let binary = control_carrier_binary();
    let binary_parent = binary
        .parent()
        .expect("the running binary has a parent directory")
        .display()
        .to_string();
    let envelope_parent = Path::new(&root)
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_else(|| "/".to_string());
    entry
        .replace(ENVELOPE_PARENT_PLACEHOLDER, &envelope_parent)
        .replace(ENVELOPE_ROOT_PLACEHOLDER, &root)
        .replace(NEAR_MISS_ROOT_PLACEHOLDER, &near_miss)
        .replace(BINARY_PARENT_PLACEHOLDER, &binary_parent)
        .replace(BINARY_PLACEHOLDER, &binary.display().to_string())
}

/// The ORDINARY TWIN of a fail-closed entry: the same command with a directory
/// that is NOT the envelope root.
///
/// **The permitted-base fence's subject on this axis.** Sections 13-16 assert
/// that at least one BASE their fail-closed property draws is PERMITTED; a
/// control-carrier entry has no base, so the analogue is its twin. A property
/// drawing only already-refused commands is GREEN before the fix and certifies
/// nothing.
const NOT_AN_ENVELOPE_ROOT: &str = "/tmp/not-an-envelope-root";

/// The BINARY entries' ordinary twin: a file in the same directory that is not
/// the binary.
///
/// **This is the same discrimination the near-miss controls make, applied as a
/// twin**: the entry and its twin differ in exactly the property the clause is
/// supposed to read — the identity of the path — and in nothing else.
const NOT_THE_GUARDS_BINARY: &str = "<BINPAR>/some-other-file";

fn control_carrier_ordinary_twin(entry: &str) -> String {
    entry
        .replace(ENVELOPE_ROOT_PLACEHOLDER, NOT_AN_ENVELOPE_ROOT)
        .replace(BINARY_PLACEHOLDER, NOT_THE_GUARDS_BINARY)
}

// ---------------------------------------------------------------------------
// 17a. The floors — the arithmetic STATED and CHECKED
// ---------------------------------------------------------------------------

/// The arithmetic, STATED rather than guessed, because audit 5 found `19-16` set
/// a floor of 50 against a maximum of 40 BY CONSTRUCTION.
///
/// **A control-carrier entry is a WHOLE COMMAND**, so there is no base and no
/// splice: the generation arithmetic is `entries`, and the SLOT is the ALPHABET
/// itself. Stating that is the axis rather than an omission — sections 13-16
/// splice into a governed base precisely because their claim is about a governed
/// program's argv, and this axis has none.
///
/// **RE-DERIVED IN ROUND 11 FROM THE THREE REFINEMENTS AS WELL AS THE FOUR NEW
/// CLASSES.** The counts move because three PREDICATES changed, not only because
/// four classes were added, and re-deriving them from the alphabets alone would
/// miss that.
///
/// * envelope-root operands       — **11** entries (9 rows + 2 measured mitigations)
/// * redirection targets          — **4**  (MOVED to the fail-closed arm)
/// * binary                       — **5**  (NEW, fail-closed)
/// * expansion-borne              — **2**
/// * symlinked                    — **2**
/// * relative                     — **2**
/// * repo-side                    — **6**
/// * ordinary operands            — **13** (11 + the 2 binary near-miss controls)
/// * redirection targets, preserving — **4** (NEW)
/// * tilde-borne                  — **4**  (NEW)
/// * glob-borne                   — **4**  (NEW)
/// * brace-list-borne             — **4**  (NEW)
/// * total after round 11 = 11 + 4 + 5 + 2 + 2 + 2 + 6 + 13 + 4 + 4 + 4 + 4 =
///   **61** cases over **12** slots
///
/// **RE-DERIVED IN ROUND 12 FROM THE NEW CLASS, THE NEW ALPHABET, THE GROWN
/// ORDINARY ALPHABET *AND* THE COMPLEMENT EXTENSION.** Two predicates changed —
/// class 7 to exclude class 12, class 5 to exclude an interior carrier path —
/// so re-deriving from the alphabets alone would miss half of it.
///
/// * interior-path              — **12** entries (NEW, fail-closed)
/// * ordinary operands          — **24** (13 + 11 measured cost controls)
/// * every other alphabet       — unchanged
/// * total = 61 + 12 + 11 = **84** cases
/// * slots = one per alphabet = **13**
///
/// The floors are EXACT equalities, so losing one case turns them red. Nothing in
/// `src/` can move them: they are a pure function of the alphabets in this file.
/// **Audit 5 found `19-16` set a floor of 50 against a maximum of 40 BY
/// CONSTRUCTION, so the arithmetic is stated and CHECKED against the alphabets
/// rather than against the prose.**
/// **RE-DERIVED IN ROUND 13 FROM THE TWO NEW CLASSES, THE TWO NEW ALPHABETS, THE
/// GROWN ORDINARY ALPHABET, THE ONE ADDED INTERIOR ENTRY *AND* ALL SIX PREDICATE
/// EXTENSIONS.** SIX predicates changed — **so re-deriving from the alphabets
/// alone would miss most of it:**
///
/// * **class 1** excludes a word naming the root EXACTLY (→ class 14);
/// * **class 2** requires a PATHNAME operator (→ class 13);
/// * **class 5** excludes a NON-PATHNAME redirection target (→ class 13);
/// * **class 7**, the COMPLEMENT, excludes classes 13 and 14;
/// * **class 11** excludes a command class 13 draws (→ class 13);
/// * **class 12** excludes a word whose interior path IS the root (→ class 14)
///   and a word that is itself a non-pathname target in the ATTACHED spelling
///   (→ class 13).
///
/// **FOUR of the six were forced by MEASUREMENT rather than chosen for
/// symmetry**, and two of those four — class 5's and class 12's second clause —
/// were found by the class-count fence going red at 15 against a derived 13
/// while this plan was being executed, which is exactly what that fence exists
/// to do.
///
/// * non-pathname target       — **7**  entries (NEW, fail-closed)
/// * ancestor                  — **7**  entries (NEW, fail-closed)
/// * interior-path             — 12 + **1** = **13** (the `T-19-126`(ii) ADDITION)
/// * ordinary operands         — 24 + **8** = **32** (4 ANCESTOR-STOP + 4 WORD-SET controls)
/// * every other alphabet      — unchanged
/// * total = 84 + 7 + 7 + 1 + 8 = **107** cases
/// * slots = one per alphabet = 13 + 2 = **15**
const CONTROL_CARRIER_CASES: usize = 107;
const CONTROL_CARRIER_SLOTS: usize = 15;
const MIN_CONTROL_CARRIER_CLASSES: usize = 14;
const MIN_CONTROL_CARRIER_NON_PATHNAME_TARGET: usize = 7;
const MIN_CONTROL_CARRIER_ANCESTOR: usize = 7;
const MIN_CONTROL_CARRIER_INTERIOR_PATH: usize = 13;
const MIN_ENVELOPE_ROOT_OPERAND_CARRIERS: usize = 11;
const MIN_CONTROL_CARRIER_REDIRECTION_TARGETS: usize = 4;
const MIN_CONTROL_CARRIER_BINARY: usize = 5;
const MIN_CONTROL_CARRIER_EXPANSION_BORNE: usize = 2;
const MIN_CONTROL_CARRIER_SYMLINKED: usize = 2;
const MIN_CONTROL_CARRIER_RELATIVE: usize = 2;
const MIN_CONTROL_CARRIER_REPO_SIDE: usize = 6;
const MIN_CONTROL_CARRIER_ORDINARY_OPERANDS: usize = 32;
const MIN_CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING: usize = 4;
const MIN_CONTROL_CARRIER_TILDE_BORNE: usize = 4;
const MIN_CONTROL_CARRIER_GLOB_BORNE: usize = 4;
const MIN_CONTROL_CARRIER_BRACE_BORNE: usize = 4;

/// The per-class counts over all 61 generated commands, DERIVED from the
/// alphabets and from the three refinements:
///
/// * class 1 (envelope-root operand) — the **11** fail-closed entries. **The
///   metacharacter and brace-list refinement is what keeps the four glob and
///   brace OPERAND entries out**; without it this would read 17 and three
///   classes would have collapsed.
/// * class 2 (redirection target)    — the **4** absolute-literal redirection
///   entries. **The same refinement keeps out the glob and brace TARGETS** —
///   `: > <ENV>/alpha/pr-ledger.ndjso?`, `: > <ENV>/alpha/{pr-ledger.ndjson,x}`,
///   `printf 'exit 0' > <ENV>/alpha/hooks/pre-pus?` and
///   `echo evil > <ENV>/alpha/{askpass,x}` — which would otherwise have been
///   counted inside a FAIL-CLOSED class while being PERMITTED.
/// * class 3 (expansion-borne)       — the 2 expansion entries PLUS the
///   expansion-borne redirection TARGET in the preserving alphabet = **3**.
///   That overlap is RECORDED: direction (ii) and direction (i) compound, and
///   the compounded row is drawn once in each position.
/// * class 4 (symlinked)             — the 2 symlink entries PLUS the `ln -s`
///   MITIGATION in the fail-closed alphabet, which names the marker as its second
///   operand = **3**. That overlap is the RECORD that direction (iii) is
///   NARROWED.
/// * class 5 (relative)              — the **2** relative entries. `rg
///   'pr-ledger.ndjson' src/` is QUOTED and correctly does not count, and the
///   tilde refinement keeps the four tilde entries out.
/// * class 6 (repo-side)             — the **6** repo-side entries.
/// * class 7 (ordinary)              — the **13** ordinary entries, and only
///   those, because class 7 is the complement of the ten. **The two binary
///   near-miss controls count HERE and not in class 11**, which is the
///   exact-path boundary asserted at the predicate level.
/// * class 8 (tilde-borne)           — the 4 tilde entries PLUS the tilde
///   redirection TARGET in the preserving alphabet = **5**.
/// * class 9 (glob-borne)            — the 4 glob entries PLUS the glob
///   redirection TARGET in the preserving alphabet = **5**.
/// * class 10 (brace-list-borne)     — the 4 brace entries PLUS the brace
///   redirection TARGET in the preserving alphabet = **5**.
/// * class 11 (the guard's own binary) — the **5** binary entries, and NOT the
///   two near-miss controls.
///
/// **ROUND 12 RE-DERIVES ALL OF THEM OVER THE 84 GENERATED COMMANDS, AND TWO
/// COUNTS MOVE:**
///
/// * **class 7 (ordinary) — 13 → 24, and the number is what proves the
///   COMPLEMENT EXTENSION took effect.** The 24 are the grown ordinary
///   alphabet's own entries and ONLY those. **The NAIVE count — class 7 without
///   the `!draws_an_interior_carrier_path` clause — would be 24 + 12 = 36**,
///   because every interior-path entry satisfies classes 1, 2 and 11 NOT and
///   would fall into the complement. **That fall of TWELVE is the arithmetic
///   proof**, and it is the reason the count is stated here rather than left to
///   be inferred from the alphabet's length.
/// * **class 12 (interior path) — the 12 new entries, and NOTHING else.** No
///   existing entry draws it: the eleven envelope-root operands carry the root
///   at index ZERO; the redirection entries carry it in TARGET position, which
///   class 12 excludes; the expansion, tilde, glob and brace entries are
///   excluded by the four literalness guards; `cat <ENVX>/…` is the
///   one-character-changed root; and `cp /bin/true <BINPAR>/some-other-file`
///   carries the binary's PARENT, which is not an EQUALITY match.
///
/// **Class 5 stays at 2 despite its refinement**, because both relative entries
/// carry no `/` at all and cannot have an interior path; the refinement exists
/// to keep the twelve NEW entries out, not to move an old one.
/// **ROUND 13 RE-DERIVES ALL OF THEM OVER THE 107 GENERATED COMMANDS, AND FOUR
/// COUNTS MOVE. Each fall is the ARITHMETIC PROOF that one predicate extension
/// took effect, which is why the naive number is written beside it.**
///
/// * **class 1 (envelope-root operand) — 11 → 14.** The three ANCESTOR-STOP
///   controls `ls <ENV>/unrelated-sibling`, `rm -f <ENV>/unrelated-sibling` and
///   `rm -rf <ENV>/beta` are absolute literal operands under the ROOT, so this
///   class draws them. **That OVERLAP is RECORDED rather than smoothed away, and
///   it is exactly the distinction `T-19-123` is about**: this class's predicate
///   is over the ROOT while rule (a)'s prefix is over `<root>/<alias>`. The
///   three are PERMITTED before and after and sit in the INVARIANCE arm; no
///   fail-closed property touches them. **The NAIVE count — class 1 without the
///   exact-root exclusion — would be 14 + 6 = 20**, because six of the seven
///   `CONTROL_CARRIER_ANCESTOR` entries name the root as a whole word. **That
///   fall of SIX is the proof the class-1 refinement took effect.**
/// * **class 2 (redirection target) — 4, UNCHANGED, and the unchangedness is
///   itself the check.** Every entry of `CONTROL_CARRIER_REDIRECTION_TARGETS`
///   and `CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING` uses `>` or `>>`, both
///   PATHNAME operators, so requiring one drops nothing. **The NAIVE count —
///   class 2 without the pathname requirement — would be 4 + 3 = 7**, because
///   three `CONTROL_CARRIER_NON_PATHNAME_TARGET` entries put a word starting
///   with the root after a `<<<`. **That fall of THREE is the proof.**
/// * **class 7 (ordinary) — 24 → 29.** The ordinary alphabet is now 32, of which
///   the three sibling controls draw class 1 instead. **The NAIVE count — class
///   7 without the two new exclusion clauses — would be 29 + 7 + 7 = 43.** That
///   fall of FOURTEEN is the proof the COMPLEMENT extension took effect, and it
///   is exactly the seven class-13 and seven class-14 entries.
/// * **class 11 (the guard's own binary) — 5, UNCHANGED.** **The NAIVE count
///   would be 6**, because `xargs cp /bin/true <<< <BIN>` puts the binary's own
///   path in a here-string target. That fall of ONE is the proof.
/// * **class 12 (interior path) — 12 → 13**, the one entry round 13 ADDS. **The
///   NAIVE count — class 12 without EITHER of its two new guards — would be
///   13 + 1 + 2 = 16:** `tar -C<ENV> …` carries the ROOT at a non-zero index
///   (the ancestor guard), and `: >&<ledger>` and `echo evil >&<ENV>/alpha/askpass`
///   are ONE whitespace word each whose `/` sits at index 2 (the attached-target
///   guard). **That fall of THREE is the proof**, and the second guard was found
///   by this very fence going red at 15 against a derived 13.
/// * **class 5 (relative) — 2, UNCHANGED. The NAIVE count would be 4**, because
///   the two `>&` entries do not begin with `/` and their BASENAMES —
///   `pr-ledger.ndjson` and `askpass` — are carrier filenames. That fall of TWO
///   is the proof class 5's third refinement took effect.
/// * **class 13 (non-pathname target) — the 7 new entries and NOTHING else.** No
///   existing entry draws it: every other redirection in every alphabet uses `>`
///   or `>>`, and the four new ordinary WORD-SET controls carry no protected
///   path in their target.
/// * **class 14 (ancestor) — the 7 new entries and NOTHING else.** `ls <ENVPAR>`
///   resolves to `/tmp`, which is ABOVE the root and not an ancestor AT OR UNDER
///   it; every other entry names the root with at least one component after it.
///
/// **Classes 3, 4, 5, 6, 8, 9 and 10 are unchanged**, and class 5 in particular
/// stays at 2 despite class 12's refinement, because both relative entries carry
/// no `/` at all.
const CONTROL_CARRIER_CLASS_COUNTS: &[(&str, usize)] = &[
    ("an absolute literal operand under the envelope root", 14),
    ("a redirection target under the envelope root", 4),
    ("an expansion-borne carrier operand", 3),
    ("a symlinked carrier operand", 3),
    ("a relative carrier operand", 2),
    ("a repo-side control carrier", 6),
    ("an ordinary path operand", 29),
    ("a tilde-borne carrier word", 5),
    ("a glob-borne carrier word", 5),
    ("a brace-list-borne carrier word", 5),
    ("the guard's own binary", 5),
    (
        "a carrier path INSIDE a literal word, at a non-zero index",
        13,
    ),
    ("a carrier path after a NON-PATHNAME redirection operator", 7),
    ("an ANCESTOR of the envelope directory, at or under the root", 7),
];

#[test]
fn the_corpus_can_draw_every_one_of_the_eleven_control_carrier_classes() {
    // **THE DEGENERATE-PROOFING, ASSERTED RATHER THAN DESCRIBED**, in the shape
    // `the_corpus_can_draw_every_one_of_the_seven_unreadable_classes`,
    // `..._five_deletion_classes`, `..._five_callee_grammar_classes` and
    // `..._five_config_resolution_classes` already use.
    //
    // **GREEN today and after.** It drives no guard call and no production change
    // can move it.
    assert_eq!(
        CONTROL_CARRIER_CLASSES.len(),
        MIN_CONTROL_CARRIER_CLASSES,
        "the control-carrier axis names exactly {MIN_CONTROL_CARRIER_CLASSES} classes"
    );
    assert_eq!(
        CONTROL_CARRIER_REPRESENTATIVES.len(),
        CONTROL_CARRIER_CLASSES.len(),
        "one representative per class, in class order"
    );

    // -- each representative satisfies its OWN class and NONE of the other TEN.
    for (index, (name, predicate)) in CONTROL_CARRIER_CLASSES.iter().enumerate() {
        let mine = control_carrier_representative(CONTROL_CARRIER_REPRESENTATIVES[index]);
        assert!(
            predicate(&mine),
            "class {index} `{name}` must be satisfied by its own representative `{mine}`. A \
             class no representative can draw is a class the corpus cannot fail on."
        );
        for (other, (other_name, other_predicate)) in CONTROL_CARRIER_CLASSES.iter().enumerate() {
            if other == index {
                continue;
            }
            assert!(
                !other_predicate(&mine),
                "\n\n**TWO CONTROL-CARRIER CLASSES HAVE COLLAPSED.**\n\
                 \n  representative of class {index} (`{name}`) : {mine}\
                 \n  also satisfies class {other} (`{other_name}`)\n\n\
                 If a pair collapses, the floors below become satisfiable by an alphabet that \
                 cannot generate the cells this round is about — which is exactly how the last \
                 five plan-check rounds each found a live cell.\n\n\
                 **ROUND 11 REFINED THREE PREDICATES FOR EXACTLY THIS REASON** — class 1 and \
                 class 2 to exclude a pathname-expansion metacharacter or a brace list, class \
                 5 to exclude a tilde — and class 2's was the one that would have done more \
                 than fire this fence: its alphabet is simultaneously the one MOVING to the \
                 fail-closed arm, so an unrefined class 2 would have placed PERMITTED glob and \
                 brace targets inside a property that asserts REFUSAL."
            );
        }
    }

    // -- **THE QUOTED-SEARCH NEGATIVE CONTROL.** A command that merely MENTIONS a
    //    carrier filename inside a quoted search pattern satisfies class 1 NOT AT
    //    ALL, because it is an operand of a SEARCH and not a path the command
    //    acts on. The corpus needs the distinction STATED rather than assumed.
    let search = "rg 'pr-ledger.ndjson' src/";
    assert!(
        !draws_an_envelope_root_operand(search),
        "`{search}` must not satisfy class 1: it names no path under the envelope root."
    );
    assert!(
        !draws_a_relative_carrier(search),
        "`{search}` must not satisfy class 5 either: the carrier filename is QUOTED, so it is \
         a search PATTERN and not an operand the command acts on. A rule that refused this \
         would be reading TEXT rather than OPERANDS."
    );
    assert!(
        draws_an_ordinary_operand(search),
        "`{search}` is an ORDINARY command and must satisfy class 7."
    );
}

#[test]
fn the_control_carrier_axis_is_new_because_no_existing_axis_reaches_a_command_with_no_governed_program(
) {
    // **THE MECHANICAL PROOF THAT THE AXIS IS A FIFTH ONE**, asserted in both
    // directions rather than argued in prose.
    //
    // **GREEN today and after.**

    // -- (A) **THE SEGMENT THAT NAMES THE CARRIER REACHES NO GOVERNED PROGRAM.**
    //    This is the axis-defining property, and it is read from the guard's OWN
    //    resolver rather than from a corpus predicate.
    //
    //    **It is the CARRIER-BEARING segment and not "every segment", and the
    //    difference was found by MEASUREMENT rather than assumed.** Direction
    //    (ii)'s entries are two segments: `D=$(git config --get core.hooksPath)`
    //    resolves `Governed { index: 0 }` — the substitution really does run a
    //    governed `git config --get`, and that read is PERMITTED (it is the same
    //    permitted read `CONTROL_CARRIER_GOVERNED_TWIN` names) — while `rm -f
    //    $D/../pr-ledger.ndjson`, the segment that acts on the carrier, reaches
    //    nothing this envelope governs. **That composition IS direction (ii): the
    //    carrier's location is fetched by a permitted governed READ and then acted
    //    on by an ungoverned command.** Asserting over every segment would have
    //    asserted something false; asserting over the carrier-bearing one asserts
    //    the fact the axis is about.
    for (alphabet, entries) in CONTROL_CARRIER_ALPHABETS {
        for entry in *entries {
            if *entry == CONTROL_CARRIER_GOVERNED_TWIN {
                continue;
            }
            let command = control_carrier_representative(entry);
            let segments = policy::split_segments_with_heads(&command)
                .unwrap_or_else(|| panic!("`{command}` splits into segments"));
            let carrier_segment = segments
                .last()
                .unwrap_or_else(|| panic!("`{command}` has at least one segment"));
            let resolution = policy::resolve_program_with_head(carrier_segment);
            assert!(
                matches!(
                    resolution,
                    policy::ProgramResolution::NoProgram
                        | policy::ProgramResolution::Ungoverned
                ),
                "\n\n**A CONTROL-CARRIER ENTRY'S CARRIER-BEARING SEGMENT REACHES A GOVERNED \
                 PROGRAM.**\n\
                 \n  alphabet : {alphabet}\
                 \n  entry    : {entry}\
                 \n  command  : {command}\
                 \n  resolved : {resolution:?}\n\n\
                 The whole content of this axis is that the carrier is NOT a command line \
                 the envelope governs: every predicate on `UNREADABLE_CLASSES`, \
                 `DELETION_CLASSES`, `CALLEE_GRAMMAR_CLASSES` and \
                 `CONFIG_RESOLUTION_CLASSES` is exercised by a property that SPLICES into a \
                 governed base, so an entry whose carrier-bearing segment reaches a governed \
                 program belongs on one of those axes and not here.\n\n\
                 The ONE deliberate exception is `{CONTROL_CARRIER_GOVERNED_TWIN}` — the \
                 permitted READ that keeps the disclosed over-refusal family legible — and \
                 it is named by constant rather than skipped silently."
            );
        }
    }

    // -- (B) EVERY existing-axis representative DOES reach a governed program, so
    //    the two sets are disjoint by the property that defines them.
    const EXISTING_AXIS_REPRESENTATIVES: &[&str] = &[
        "git pus? --force origin main",
        "git >/dev/null push --force origin main",
        "git --attr-source HEAD push --force origin main",
        "git --bogus-opt status",
        "git -c include.path=/tmp/evil.cfg status",
        "git -c alias.q=\"-c include.path=/tmp/evil.cfg status\" q",
        "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git status",
    ];
    for command in EXISTING_AXIS_REPRESENTATIVES {
        let segments = policy::split_segments_with_heads(command)
            .unwrap_or_else(|| panic!("`{command}` splits into segments"));
        assert!(
            segments.iter().any(|segment| matches!(
                policy::resolve_program_with_head(segment),
                policy::ProgramResolution::Governed { .. }
            )) || segments.iter().any(|segment| !matches!(
                policy::resolve_program_with_head(segment),
                policy::ProgramResolution::NoProgram
                    | policy::ProgramResolution::Ungoverned
            )),
            "`{command}` is a representative of one of the four EXISTING axes and must reach a \
             governed program (or be refused before resolution). If it reaches none, the \
             existing axes and this one are not separated by the property that defines them."
        );
    }

    // -- (C) NO existing-axis representative satisfies any of the SIX CARRIER
    //    classes. **Class 7 is deliberately excluded and the reason is stated
    //    rather than left to be inferred: it is the COMPLEMENT of the six, so a
    //    `git` command that names no carrier satisfies it trivially — which is
    //    correct and is the point.**
    for command in EXISTING_AXIS_REPRESENTATIVES {
        for (index, (name, predicate)) in CONTROL_CARRIER_CLASSES.iter().enumerate() {
            if index == 6 {
                continue;
            }
            assert!(
                !predicate(command),
                "the existing-axis representative `{command}` must not satisfy the \
                 control-carrier class `{name}`. If it does, the fifth axis has been given a \
                 predicate that names an ARGV fact rather than a CARRIER fact."
            );
        }
    }

    // -- (D) **WHAT IS NOT CLAIMED, RECORDED RATHER THAN ASSERTED.** The reverse
    //    direction — that no control-carrier entry satisfies any predicate on the
    //    four existing axes — is FALSE, and asserting it would be asserting
    //    something untrue. Those predicates are pure TEXT functions:
    //    `draws_a_separate_word_redirection` fires on `: > <ENV>/…` because that
    //    command really does carry a separate-word redirection. **The overlap is
    //    textual and incidental; the properties that USE those predicates can
    //    never generate these commands, because they only ever splice into
    //    `git …`.** The disjointness that is TRUE is (A)+(B) above.
    let mut incidental = Vec::new();
    for (alphabet, entries) in CONTROL_CARRIER_ALPHABETS {
        for entry in *entries {
            let command = control_carrier_representative(entry);
            let mut hits = Vec::new();
            if carries_an_unreadable_class(&command) {
                hits.push("UNREADABLE_CLASSES");
            }
            if carries_a_deletion_class(&command) {
                hits.push("DELETION_CLASSES");
            }
            if carries_a_callee_grammar_class(&command) {
                hits.push("CALLEE_GRAMMAR_CLASSES");
            }
            if carries_a_config_resolution_class(&command) {
                hits.push("CONFIG_RESOLUTION_CLASSES");
            }
            if !hits.is_empty() {
                incidental.push(format!("  {alphabet}: {entry}  -> {hits:?}"));
            }
        }
    }
    println!(
        "RECORDED (not asserted): INCIDENTAL TEXTUAL OVERLAPS between control-carrier entries \
         and the four existing axes' pure-text class predicates. Every one of these commands \
         reaches NO governed program, so no existing axis's PROPERTY can generate or judge \
         it.\n{}",
        if incidental.is_empty() {
            "  (none)".to_string()
        } else {
            incidental.join("\n")
        }
    );
}

#[test]
fn every_alphabet_this_round_widens_can_draw_a_fact_about_the_file_a_control_lives_in() {
    // **The direct mechanical inverse of `T-19-114`.** Audit 9 established the
    // defect by grepping the three test files for `pr-ledger`, `hooks/pre-push`,
    // `.git/config`, `askpass` and `settings.json` and finding NOTHING AT ALL
    // (re-verified while planning: 0 / 0, and the single `pr-ledger` hit in
    // `tests/envelope_reparsed_value.rs:580` is a COMMENT about the positive walk
    // control). This asserts the repaired fact, so narrowing an alphabet back
    // turns this red instead of quietly restoring a corpus that cannot fail on
    // its own class.
    //
    // **GREEN today and after.**
    assert!(
        CONTROL_CARRIER_CLASSES.len() >= MIN_CONTROL_CARRIER_CLASSES,
        "the control-carrier axis must name at least {MIN_CONTROL_CARRIER_CLASSES} classes"
    );
    for (name, entries, floor) in [
        (
            "ENVELOPE_ROOT_OPERAND_CARRIERS",
            ENVELOPE_ROOT_OPERAND_CARRIERS,
            MIN_ENVELOPE_ROOT_OPERAND_CARRIERS,
        ),
        (
            "CONTROL_CARRIER_REDIRECTION_TARGETS",
            CONTROL_CARRIER_REDIRECTION_TARGETS,
            MIN_CONTROL_CARRIER_REDIRECTION_TARGETS,
        ),
        (
            "CONTROL_CARRIER_EXPANSION_BORNE",
            CONTROL_CARRIER_EXPANSION_BORNE,
            MIN_CONTROL_CARRIER_EXPANSION_BORNE,
        ),
        (
            "CONTROL_CARRIER_SYMLINKED",
            CONTROL_CARRIER_SYMLINKED,
            MIN_CONTROL_CARRIER_SYMLINKED,
        ),
        (
            "CONTROL_CARRIER_RELATIVE",
            CONTROL_CARRIER_RELATIVE,
            MIN_CONTROL_CARRIER_RELATIVE,
        ),
        (
            "CONTROL_CARRIER_REPO_SIDE",
            CONTROL_CARRIER_REPO_SIDE,
            MIN_CONTROL_CARRIER_REPO_SIDE,
        ),
        (
            "CONTROL_CARRIER_ORDINARY_OPERANDS",
            CONTROL_CARRIER_ORDINARY_OPERANDS,
            MIN_CONTROL_CARRIER_ORDINARY_OPERANDS,
        ),
        ("CONTROL_CARRIER_BINARY", CONTROL_CARRIER_BINARY, MIN_CONTROL_CARRIER_BINARY),
        (
            "CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING",
            CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING,
            MIN_CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING,
        ),
        (
            "CONTROL_CARRIER_TILDE_BORNE",
            CONTROL_CARRIER_TILDE_BORNE,
            MIN_CONTROL_CARRIER_TILDE_BORNE,
        ),
        (
            "CONTROL_CARRIER_GLOB_BORNE",
            CONTROL_CARRIER_GLOB_BORNE,
            MIN_CONTROL_CARRIER_GLOB_BORNE,
        ),
        (
            "CONTROL_CARRIER_BRACE_BORNE",
            CONTROL_CARRIER_BRACE_BORNE,
            MIN_CONTROL_CARRIER_BRACE_BORNE,
        ),
        (
            "CONTROL_CARRIER_INTERIOR_PATH",
            CONTROL_CARRIER_INTERIOR_PATH,
            MIN_CONTROL_CARRIER_INTERIOR_PATH,
        ),
        (
            "CONTROL_CARRIER_NON_PATHNAME_TARGET",
            CONTROL_CARRIER_NON_PATHNAME_TARGET,
            MIN_CONTROL_CARRIER_NON_PATHNAME_TARGET,
        ),
        (
            "CONTROL_CARRIER_ANCESTOR",
            CONTROL_CARRIER_ANCESTOR,
            MIN_CONTROL_CARRIER_ANCESTOR,
        ),
    ] {
        assert!(
            entries.len() >= floor,
            "\n\n`{name}` must carry at least {floor} entries; it has {}.\n\n\
             **THE CORRECT RESPONSE TO A RED HERE IS TO RESTORE ENTRIES, NEVER TO LOWER THIS \
             FLOOR.** `T-19-114` is `T-19-76`'s failure mode for the TENTH consecutive round, \
             after `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99`, `T-19-101` and `T-19-105` — and \
             **the FIRST in which the gap was a different KIND of carrier rather than a wider \
             alphabet of the same one.** A case that stops being drawn is a case that stops \
             being able to fail.",
            entries.len()
        );
        for entry in entries {
            let command = control_carrier_representative(entry);
            assert!(
                carries_a_control_carrier_class(&command),
                "`{name}` entry `{entry}` draws NO control-carrier class (`{command}`).\n\n\
                 An alphabet entry that cannot DRAW a class is an entry whose property cannot \
                 FAIL on one, and every case generated from it certifies a claim about a class \
                 it could never have exercised."
            );
        }
    }

    // -- the counted floors, as EXACT equalities.
    let mut all: Vec<String> = Vec::new();
    let mut slots: BTreeSet<&str> = BTreeSet::new();
    for (name, entries) in CONTROL_CARRIER_ALPHABETS {
        slots.insert(name);
        for entry in *entries {
            all.push(control_carrier_representative(entry));
        }
    }
    assert_eq!(
        all.len(),
        CONTROL_CARRIER_CASES,
        "the generation count must equal the stated arithmetic exactly, so losing one case \
         turns this red. `19-16` set a floor of 50 against a maximum of 40 by construction and \
         it was invisible until the rule landed."
    );
    assert_eq!(
        slots.len(),
        CONTROL_CARRIER_SLOTS,
        "every slot must have been generated. A control-carrier command has no base and no \
         splice, so the SLOT is the ALPHABET — which is itself the axis. Seen: {slots:?}"
    );

    let mut per_class: BTreeMap<&str, usize> = BTreeMap::new();
    for command in &all {
        for (class, predicate) in CONTROL_CARRIER_CLASSES {
            if predicate(command) {
                *per_class.entry(class).or_default() += 1;
            }
        }
    }
    for (class, expected) in CONTROL_CARRIER_CLASS_COUNTS {
        let got = per_class.get(class).copied().unwrap_or(0);
        assert_eq!(
            got, *expected,
            "class `{class}` must be drawn by exactly {expected} of the \
             {CONTROL_CARRIER_CASES} generated commands, got {got}. The counts are DERIVED from \
             the alphabets and stated in `CONTROL_CARRIER_CLASS_COUNTS`' doc as exact \
             equalities."
        );
    }
}

#[test]
fn the_control_carrier_alphabets_are_pairwise_disjoint_and_the_fail_closed_one_is_never_drawn_by_the_invariance_arm(
) {
    // **THE DISJOINTNESS AND PERMITTED-BASE FENCES, EXTENDED TO THE NEW AXIS.**
    //
    // **GREEN today and after.**
    for (i, (left_name, left)) in CONTROL_CARRIER_ALPHABETS.iter().enumerate() {
        for (right_name, right) in CONTROL_CARRIER_ALPHABETS.iter().skip(i + 1) {
            for entry in *left {
                assert!(
                    !right.contains(entry),
                    "`{entry}` is in BOTH `{left_name}` and `{right_name}`. The seven alphabets \
                     must be pairwise disjoint, or the class counts above count one command \
                     twice and the floors stop measuring what they name."
                );
            }
        }
    }

    // -- **THE FAIL-CLOSED ALPHABET IS NEVER DRAWN BY THE INVARIANCE ARM.** An
    //    envelope-root operand entry is REFUSED after `19-27` even where its
    //    ordinary twin is PERMITTED, so it would be STRICTER than its base and
    //    would turn the invariance property permanently red in a file `19-27` may
    //    not edit.
    for (name, entries) in CONTROL_CARRIER_INVARIANCE_ALPHABETS {
        for entry in *entries {
            for (closed_name, closed) in CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS {
                assert!(
                    !closed.contains(entry),
                    "\n\n`{entry}` is in the invariance-arm alphabet `{name}` AND in the \
                     fail-closed alphabet `{closed_name}`.\n\n\
                     **MUST NOT PUT AN ENTRY WHOSE VERDICT THE FIX CHANGES INTO THE \
                     INVARIANCE-PRESERVING ARM.** This is `19-18`'s `{{v}}>` blocker, \
                     `19-20`'s `GIT_GLOBAL_UNKNOWN_OPTIONS` split, `19-22`'s \
                     indirection/confined split, `19-24`'s re-parsed/confined split and \
                     `19-26`'s own envelope-root split — **and round 11 is the SIXTH time it \
                     has been forced by measurement**, when \
                     `CONTROL_CARRIER_REDIRECTION_TARGETS` moved arm because all four of its \
                     entries are refused after `19-29`."
                );
            }
        }
    }

    // -- **THE THREE FAIL-CLOSED ALPHABETS ARE PAIRWISE DISJOINT FROM EACH
    //    OTHER TOO**, so the fail-closed property below drives each entry once
    //    and the class counts do not double-count a command.
    for (i, (left_name, left)) in CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS.iter().enumerate() {
        for (right_name, right) in CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS.iter().skip(i + 1) {
            for entry in *left {
                assert!(
                    !right.contains(entry),
                    "`{entry}` is in BOTH fail-closed alphabets `{left_name}` and `{right_name}`"
                );
            }
        }
    }

    // -- **THE PERMITTED-BASE FENCE, IN THIS AXIS'S TERMS.** A control-carrier
    //    entry has no base, so the analogue is its ORDINARY TWIN: the same
    //    command with a directory that is not the envelope root. **Every twin
    //    must be PERMITTED**, because a property drawing only already-refused
    //    commands is GREEN before the fix and certifies nothing — the TENTH
    //    consecutive instance of `T-19-76`'s failure mode, produced by the corpus
    //    rather than found by the next audit.
    for (_, entries) in CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS {
      for entry in *entries {
        let twin = control_carrier_command(&control_carrier_ordinary_twin(entry), Path::new("/tmp"));
        let envelope = TempDir::new().expect("a temporary envelope root");
        let got = verdict(envelope.path(), &twin);
        assert_eq!(
            got.code, 0,
            "\n\n**THE ORDINARY TWIN OF A FAIL-CLOSED ENTRY MUST BE PERMITTED.**\n\
             \n  entry : {entry}\
             \n  twin  : {twin}\
             \n  got   : exit {} reason {}\n\n\
             The entire content of `T-19-112`, `T-19-113`, `T-19-116` and `T-19-118` is \
             that a command layer 2 LETS THROUGH removes a control. If the twin were already \
             refused, this axis's fail-closed property would be green before the fix and would \
             certify nothing.",
            got.code, got.reason_id,
        );
      }
    }
}

#[test]
fn the_carrier_rule_may_not_be_a_denylist_of_program_names() {
    // **THE NO-PROGRAM-NAMES FENCE — the first mechanical assertion this section
    // exists to add**, written in the shape the DOTLESS fence already uses.
    //
    // **GREEN today and after.**
    let policy_production = production_code(POLICY_SOURCE);
    let hooks_production = production_code(HOOKS_SOURCE);

    // The positive control, for the reason
    // `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
    // states: an absence assertion cannot tell "the name is not in this file"
    // from "this is not the file I think it is".
    assert!(
        policy_production.contains(POLICY_ANCHOR),
        "the stripped `policy.rs` must contain `{POLICY_ANCHOR}`"
    );
    assert!(
        hooks_production.contains(HOOKS_ANCHOR),
        "the stripped `hooks.rs` must contain `{HOOKS_ANCHOR}`"
    );

    let unknown: Vec<&str> = ENVELOPE_ROOT_OPERAND_CARRIERS
        .iter()
        .filter_map(|entry| entry.split_whitespace().next())
        .filter(|program| {
            !policy_production.contains(*program) && !hooks_production.contains(*program)
        })
        .collect();

    assert!(
        !unknown.is_empty(),
        "\n\n**AT LEAST ONE `ENVELOPE_ROOT_OPERAND_CARRIERS` ENTRY MUST NAME A PROGRAM THE \
         GUARD'S PRODUCTION LOGIC DOES NOT KNOW.**\n\n\
         `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` is the \
         control that forbids a denylist of dangerous commands, and D-08's whole argument is \
         that `resolve_program` NEVER ASKS WHAT THE WRAPPER IS CALLED. A rule reading program \
         names would be red there.\n\n\
         **This fence is the corpus's own proof that the rule reads a PATH.** If it goes red, \
         either every carrier entry now names a program the guard knows — in which case the \
         corpus can no longer distinguish a path rule from a denylist — or `19-27` taught the \
         production logic a program name, which is the defect itself. The correct response is \
         to DELETE the program name from `src/`, never to delete this assertion.\n\n\
         Programs named by the alphabet: {:?}",
        ENVELOPE_ROOT_OPERAND_CARRIERS
            .iter()
            .filter_map(|entry| entry.split_whitespace().next())
            .collect::<Vec<_>>()
    );
    println!("the NO-PROGRAM-NAMES fence is carried by: {unknown:?}");

    // -- **EXTENDED BY ROUND 11 TO THIS ROUND'S OWN CLASSES.** A fence that only
    //    covered class 1 would say nothing about whether the BINARY clause or
    //    the three spelling classes had been written as a denylist. **At least
    //    one entry of each must name a program the guard's production logic does
    //    not know.**
    for (name, entries) in [
        ("CONTROL_CARRIER_BINARY", CONTROL_CARRIER_BINARY),
        ("CONTROL_CARRIER_TILDE_BORNE", CONTROL_CARRIER_TILDE_BORNE),
        ("CONTROL_CARRIER_GLOB_BORNE", CONTROL_CARRIER_GLOB_BORNE),
        ("CONTROL_CARRIER_BRACE_BORNE", CONTROL_CARRIER_BRACE_BORNE),
        // **EXTENDED BY ROUND 12.** `19-31` widens what a LITERAL word is READ
        // FOR, and that widening may not teach the guard a program name either.
        // `dd`, `tar`, `cp`, `chmod`, `rsync` and `mytool` are all absent from
        // both production halves, and the fence counts that rather than
        // trusting this comment.
        ("CONTROL_CARRIER_INTERIOR_PATH", CONTROL_CARRIER_INTERIOR_PATH),
        // **EXTENDED BY ROUND 13 TO BOTH NEW CLASSES.** `19-33` widens the WORD
        // SET and the PATH SET, and neither widening may teach the guard a
        // program name. `xargs` is absent from both production halves (measured
        // zero occurrences of the quoted literal in each), and so are `mv`,
        // `find`, `chmod` and `tar`.
        (
            "CONTROL_CARRIER_NON_PATHNAME_TARGET",
            CONTROL_CARRIER_NON_PATHNAME_TARGET,
        ),
        ("CONTROL_CARRIER_ANCESTOR", CONTROL_CARRIER_ANCESTOR),
    ] {
        let unknown: Vec<&str> = entries
            .iter()
            .filter_map(|entry| entry.split_whitespace().next())
            .filter(|program| {
                !policy_production.contains(*program) && !hooks_production.contains(*program)
            })
            .collect();
        assert!(
            !unknown.is_empty(),
            "\n\n**`{name}` MUST NAME AT LEAST ONE PROGRAM THE GUARD'S PRODUCTION LOGIC DOES \
             NOT KNOW.**\n\n\
             `19-29` widens rule (a)'s PATH SET and what it SEES, and neither widening may \
             teach the guard a program name. D-08's whole argument is that `resolve_program` \
             never asks what the wrapper is CALLED. **The correct response is to DELETE the \
             program name from `src/`, never to delete this assertion.**\n\n\
             Programs named: {:?}",
            entries
                .iter()
                .filter_map(|entry| entry.split_whitespace().next())
                .collect::<Vec<_>>()
        );
        println!("  {name} carries the fence with: {unknown:?}");
    }
}

#[test]
fn the_carrier_rule_must_resolve_a_path_prefix_and_not_a_basename() {
    // **THE PATH-PREFIX-NOT-BASENAME FENCE — the second mechanical assertion.**
    //
    // **GREEN today and after.**
    for near_miss in CONTROL_CARRIER_NEAR_MISSES {
        assert!(
            CONTROL_CARRIER_ORDINARY_OPERANDS.contains(near_miss),
            "\n\n**`{near_miss}` MUST BE IN `CONTROL_CARRIER_ORDINARY_OPERANDS`, BY NAME.**\n\n\
             These two are ROUND 10's `--signed no`, one from each side:\n\
             \x20 * `rm -f /tmp/pr-ledger.ndjson` — the BASENAME is a carrier filename and the \
             DIRECTORY is not the envelope root;\n\
             \x20 * `cat <ENVX>/alpha/pr-ledger.ndjson` — an envelope-root spelling with ONE \
             CHARACTER CHANGED.\n\n\
             A rule written as `path.ends_with(\"pr-ledger.ndjson\")`, or as a substring test \
             for the filename, or as a string-prefix test that does not compare resolved \
             COMPONENTS, turns BOTH of these RED. **Only a rule that resolves a PATH PREFIX \
             against the root the guard was GIVEN keeps them green.**"
        );
        let command = control_carrier_representative(near_miss);
        assert!(
            !draws_an_envelope_root_operand(&command),
            "`{command}` must NOT draw class 1: it names no path under the envelope root."
        );
    }

    // -- **THE BINARY'S EXACT-PATH-NOT-PREFIX CONTROLS, ASSERTED BY NAME.**
    //    **The two boundary KINDS differ and the reason is MEASURED rather than
    //    aesthetic.** The envelope DIRECTORY is a PREFIX boundary because this
    //    envelope owns every byte under it — `rm -rf <ENV>/alpha` takes NINE
    //    carriers in one call. **The BINARY is an EXACT-PATH boundary because
    //    its directory is shared with everything else the user installed**: a
    //    prefix over `~/.cargo/bin` would refuse `ls ~/.cargo/bin` and every
    //    `cargo install`, and under `cargo test` it would refuse
    //    `ls target/debug/deps`.
    for near_miss in CONTROL_CARRIER_BINARY_NEAR_MISSES {
        assert!(
            CONTROL_CARRIER_ORDINARY_OPERANDS.contains(near_miss),
            "\n\n**`{near_miss}` MUST BE IN `CONTROL_CARRIER_ORDINARY_OPERANDS`, BY NAME.**\n\n\
             These two are ROUND 11's `--signed no`, one from each side:\n\
             \x20 * `cp /bin/true <BINPAR>/some-other-file` — a SIBLING of the binary in the \
             same directory;\n\
             \x20 * `ls <BINPAR>` — a READ of the binary's own directory.\n\n\
             **A clause written as a DIRECTORY PREFIX over the binary's parent turns BOTH of \
             these RED. Only an EXACT-PATH comparison keeps them green.**"
        );
        let command = control_carrier_representative(near_miss);
        assert!(
            !draws_the_guards_own_binary(&command),
            "`{command}` must NOT draw class 11. The predicate compares the path's IDENTITY \
             and not its neighbourhood, and `REPRESENTATIVE_BINARY_PARENT` is a PREFIX of \
             `REPRESENTATIVE_BINARY_PATH` — so a `starts_with` in that predicate would draw \
             these two and silently assert them refused."
        );
        assert!(
            draws_an_ordinary_operand(&command),
            "`{command}` must draw class 7, the ordinary complement."
        );
    }

    // -- **EXTENDED BY ROUND 12: THE MECHANICAL REASON THE INTERIOR SCAN COSTS
    //    NOTHING, ASSERTED RATHER THAN ARGUED.** Splitting a word produces more
    //    CANDIDATES; a candidate is refused only if it PREFIXES a directory of
    //    at least three components or EQUALS a full binary path. **A candidate
    //    SHORTER than the envelope directory can never match it**, and
    //    `word_is_within`'s `word.len() < dir.len()` early return over COMPONENT
    //    VECTORS is what makes that structural rather than lucky.
    for (word, why) in [
        ("s/x/y/", "a `sed` substitution: `[x, y]`, `[y]`, `[]`"),
        ("src/", "a trailing slash: the single candidate `/` is the EMPTY list"),
        (
            "https://github.com/o/r",
            "a URL: `[github.com, o, r]`, `[o, r]`, `[r]` — wrong FIRST component",
        ),
        (
            "HEAD:refs/heads/gsd-auto/alpha/w",
            "a `:`-attached ref carrying the word `alpha`, which fails on its FIRST component \
             rather than on its length",
        ),
        ("--git-dir=/tmp/g", "an `=`-attached absolute path OUTSIDE the set"),
        ("--format=%H", "a `%`-formatted value with no `/` at all"),
    ] {
        assert!(
            !word_carries_an_interior_carrier_path(word),
            "\n\n**`{word}` MUST NOT BE READ AS AN INTERIOR CARRIER PATH: {why}.**\n\n\
             The cost of reading every `/`-anchored substring is bounded by the PATH SET rather \
             than by the split. If this fires, the rule widened past the boundary it declares \
             — and `--git-dir=/tmp/g` alone is pinned PERMITTED in FOUR files and named in FIVE \
             `policy.rs` doc sites, so five files would turn red at once."
        );
    }
    assert!(
        word_carries_an_interior_carrier_path(&format!("of={REPRESENTATIVE_ENVELOPE_ROOT}/alpha")),
        "the POSITIVE control: without it the six absences above pass because the predicate \
         answers `false` for everything."
    );

    // -- **EXTENDED BY ROUND 13: THE ANCESTOR COMPARISON IS THE SAME COMPONENT
    //    VECTORS THE OTHER WAY ROUND, AND IT IS BOUNDED AT THE ROOT — WHICH IS
    //    THE MECHANICAL REASON IT ADDS EXACTLY ONE PATH.**
    //
    //    `word_is_within` asks whether the CANDIDATE's component vector has the
    //    DIRECTORY's as a prefix. The ancestor clause asks the converse — whether
    //    the DIRECTORY's has the CANDIDATE's as a proper prefix — and then bounds
    //    the answer by requiring the candidate to be at or under the ROOT.
    //    Because the alias is a plain single path component (`ledger_path_in`
    //    refuses anything else), the set of candidates satisfying BOTH halves is
    //    exactly `{<root>}`.
    //
    //    **A CLAUSE THAT DROPPED THE SECOND HALF WOULD REACH `/tmp` AND `/`**,
    //    and these rows are what turns that red.
    for (word, why) in [
        (
            REPRESENTATIVE_ENVELOPE_PARENT,
            "**THE STOP.** The root's own PARENT is a proper ancestor of the envelope \
             directory and is NOT at or under the root, so the clause does not reach it",
        ),
        ("/", "the filesystem root, one further up"),
        (
            REPRESENTATIVE_BINARY_PARENT,
            "**the BINARY's parent. NO ANCESTOR CLAUSE IS WRITTEN FOR THE BINARY** — \
             `word_is_exactly` stays an EQUALITY because its directory is shared with \
             everything the user installed",
        ),
        (
            "/tmp/gsd-envelope-root/alpha",
            "the envelope DIRECTORY itself is not a PROPER ancestor of itself",
        ),
        (
            "/tmp/gsd-envelope-root/unrelated-sibling",
            "**THE ANCESTOR-vs-PREFIX DISCRIMINATOR.** It is UNDER the root and is NOT an \
             ancestor of the envelope directory, so the ancestor clause leaves it alone — \
             where a PREFIX widened to the root refuses it",
        ),
    ] {
        assert!(
            !word_carries_an_ancestor_of_the_envelope_directory(word),
            "\n\n**`{word}` MUST NOT BE READ AS AN ANCESTOR: {why}.**\n\n\
             The ancestor clause is bounded at `<root>` on OWNERSHIP grounds and adds EXACTLY \
             ONE path. If this fires, the clause reached past its stop — and refusing `ls /`, \
             `df /`, `du -sh $HOME` or `ls /tmp` is an OUTAGE rather than a boundary \
             (AR-19-11)."
        );
    }
    for (word, why) in [
        (REPRESENTATIVE_ENVELOPE_ROOT, "the root, named exactly"),
        (
            "/tmp/gsd-envelope-root/",
            "the trailing-slash spelling, which `lexical_absolute_components` normalises \
             identically",
        ),
        (
            "-C/tmp/gsd-envelope-root",
            "**the ancestor COMPOSED with round 12's interior scan**, at a non-zero index",
        ),
    ] {
        assert!(
            word_carries_an_ancestor_of_the_envelope_directory(word),
            "the ANCESTOR clause's positive control `{word}` must answer `true`: {why}. \
             Without these the five absences above pass because the predicate answers `false` \
             for everything."
        );
    }
}

#[test]
fn no_control_carrier_floor_or_class_count_fell_when_round_13_re_derived_them() {
    // **THE MONOTONICITY PIN — NEW IN ROUND 13, AND IT EXISTS BECAUSE OF A
    // CONTRADICTION IN `19-32`'s OWN GATE THAT IS REPORTED RATHER THAN
    // SMOOTHED.**
    //
    // `19-32` mandates BOTH that `git diff --numstat` over `tests/` show ZERO
    // deletions AND that `MIN_CONTROL_CARRIER_CLASSES` rise from 12 to 14,
    // `CONTROL_CARRIER_SLOTS` become 15, `CONTROL_CARRIER_CASES` be recomputed
    // and the per-class counts be re-derived. **A raised floor cannot be written
    // without changing the line that carries it**, and `git` counts a changed
    // line as one deletion and one addition. The two requirements cannot both be
    // met, and this round met the one that carries the meaning.
    //
    // **THE INTENT THE ZERO-DELETIONS RULE CARRIES IS STATED IN `19-32` ITSELF:**
    // *"Entries may be ADDED, a predicate may be EXTENDED with its reason
    // stated, and a doc may gain an APPENDED WR-02 note; nothing may be removed,
    // reworded away or narrowed … A case that stops being drawn is a case that
    // stops being able to fail."* **A floor that RISES is the opposite of
    // narrowing.** So the intent is asserted here MECHANICALLY, where the line
    // count cannot express it: every floor and every class count is at or above
    // the value round 12 left it at, and every alphabet is at or above its round
    // 12 length.
    //
    // **GREEN today and after.** Its correct response to a red is to restore the
    // floor, never to lower this table.
    for (name, round_12, now) in [
        ("CONTROL_CARRIER_CASES", 84usize, CONTROL_CARRIER_CASES),
        ("CONTROL_CARRIER_SLOTS", 13, CONTROL_CARRIER_SLOTS),
        ("MIN_CONTROL_CARRIER_CLASSES", 12, MIN_CONTROL_CARRIER_CLASSES),
        (
            "MIN_CONTROL_CARRIER_INTERIOR_PATH",
            12,
            MIN_CONTROL_CARRIER_INTERIOR_PATH,
        ),
        (
            "MIN_ENVELOPE_ROOT_OPERAND_CARRIERS",
            11,
            MIN_ENVELOPE_ROOT_OPERAND_CARRIERS,
        ),
        (
            "MIN_CONTROL_CARRIER_REDIRECTION_TARGETS",
            4,
            MIN_CONTROL_CARRIER_REDIRECTION_TARGETS,
        ),
        ("MIN_CONTROL_CARRIER_BINARY", 5, MIN_CONTROL_CARRIER_BINARY),
        (
            "MIN_CONTROL_CARRIER_EXPANSION_BORNE",
            2,
            MIN_CONTROL_CARRIER_EXPANSION_BORNE,
        ),
        ("MIN_CONTROL_CARRIER_SYMLINKED", 2, MIN_CONTROL_CARRIER_SYMLINKED),
        ("MIN_CONTROL_CARRIER_RELATIVE", 2, MIN_CONTROL_CARRIER_RELATIVE),
        ("MIN_CONTROL_CARRIER_REPO_SIDE", 6, MIN_CONTROL_CARRIER_REPO_SIDE),
        (
            "MIN_CONTROL_CARRIER_ORDINARY_OPERANDS",
            24,
            MIN_CONTROL_CARRIER_ORDINARY_OPERANDS,
        ),
        (
            "MIN_CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING",
            4,
            MIN_CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING,
        ),
        ("MIN_CONTROL_CARRIER_TILDE_BORNE", 4, MIN_CONTROL_CARRIER_TILDE_BORNE),
        ("MIN_CONTROL_CARRIER_GLOB_BORNE", 4, MIN_CONTROL_CARRIER_GLOB_BORNE),
        ("MIN_CONTROL_CARRIER_BRACE_BORNE", 4, MIN_CONTROL_CARRIER_BRACE_BORNE),
        ("MIN_CONFIG_RESOLUTION_CLASSES", 6, MIN_CONFIG_RESOLUTION_CLASSES),
    ] {
        assert!(
            now >= round_12,
            "\n\n**A FLOOR FELL: `{name}` was {round_12} after round 12 and is {now} now.**\n\n\
             `19-32` may RAISE a floor and may not lower one. **The correct response to a red \
             here is to restore the entries the floor counts, never to lower the floor.**"
        );
    }
    // -- `MIN_CONFIG_RESOLUTION_CLASSES` is asserted EXACT rather than
    //    monotone, because `19-32` may not open that axis at all.
    assert_eq!(
        MIN_CONFIG_RESOLUTION_CLASSES, 6,
        "**`MIN_CONFIG_RESOLUTION_CLASSES` STAYS AT 6.** The four earlier axes, their \
         predicates, their degenerate-proofing and their floors are BYTE-IDENTICAL, and \
         round 13 stands neither new class up as a sixth axis."
    );

    // -- **AND NO CLASS COUNT FELL EITHER**, which is the half a floor table
    //    cannot see: a predicate extension moves entries BETWEEN classes, and
    //    round 13's six extensions are all in the direction of a NEW class.
    for (class, round_12, now) in [
        ("a redirection target under the envelope root", 4usize, 4),
        ("an expansion-borne carrier operand", 3, 3),
        ("a symlinked carrier operand", 3, 3),
        ("a relative carrier operand", 2, 2),
        ("a repo-side control carrier", 6, 6),
        ("a tilde-borne carrier word", 5, 5),
        ("a glob-borne carrier word", 5, 5),
        ("a brace-list-borne carrier word", 5, 5),
        ("the guard's own binary", 5, 5),
    ] {
        let stated = CONTROL_CARRIER_CLASS_COUNTS
            .iter()
            .find(|(name, _)| *name == class)
            .map(|(_, count)| *count)
            .unwrap_or_else(|| panic!("class `{class}` must still be counted"));
        assert_eq!(
            stated, now,
            "the stated count for `{class}` must match this table"
        );
        assert!(
            stated >= round_12,
            "\n\n**A CLASS COUNT FELL: `{class}` was {round_12} after round 12 and is {stated} \
             now.**\n\n\
             **Every one of round 13's SIX predicate extensions must move entries INTO a new \
             class and never OUT of an existing one.** A fall here means a refinement emptied \
             a cell — which is how `19-16` set a floor of 50 against a maximum of 40 by \
             construction, invisible until the rule landed."
        );
    }
    println!(
        "the MONOTONICITY pin: {} floors and {} class counts checked; \
         CONTROL_CARRIER_CASES 84 -> {CONTROL_CARRIER_CASES}, SLOTS 13 -> \
         {CONTROL_CARRIER_SLOTS}, CLASSES 12 -> {MIN_CONTROL_CARRIER_CLASSES}. **The eight \
         changed lines under `tests/` are ALL floor and count RAISES; not one alphabet entry, \
         property or assertion was removed, reworded away or narrowed.**",
        17, 9
    );
}

#[test]
fn bashs_twelve_operator_split_is_partitioned_and_the_axis_reads_it_rather_than_inventing_it() {
    // **THE SPLIT IS BASH'S OWN GRAMMAR, AND THE AXIS READS IT RATHER THAN
    // RE-ENUMERATING IT.** `redirection_operator` (`policy.rs:3193-3229`) derives
    // it in the same match that computes each operator's length; this fence
    // asserts the corpus's two lists PARTITION `REDIRECTION_OPERATORS` exactly,
    // so a spelling that appeared in one and not the other could not hide.
    //
    // **GREEN today and after.**
    assert_eq!(
        PATHNAME_REDIRECTION_OPERATORS.len() + NON_PATHNAME_REDIRECTION_OPERATORS.len(),
        REDIRECTION_OPERATORS.len(),
        "bash's redirection production has TWELVE operators and the corpus splits them SEVEN \
         and FIVE. If this arithmetic is wrong the grammar was re-enumerated rather than \
         read, which is the sixth enumeration `redirection_operator`'s doc exists to refuse."
    );
    for operator in PATHNAME_REDIRECTION_OPERATORS
        .iter()
        .chain(NON_PATHNAME_REDIRECTION_OPERATORS)
    {
        assert!(
            REDIRECTION_OPERATORS.contains(operator),
            "`{operator}` is not one of the twelve `REDIRECTION_OPERATORS`"
        );
    }
    for operator in PATHNAME_REDIRECTION_OPERATORS {
        assert!(
            !NON_PATHNAME_REDIRECTION_OPERATORS.contains(operator),
            "`{operator}` is in BOTH lists; the split must be a PARTITION"
        );
    }
    // -- **THE LONGEST-MATCH-FIRST ORDER, ASSERTED rather than assumed.** These
    //    are matched against an ATTACHED spelling, so `<<<` must be tried before
    //    `<<-` and both before `<<`, exactly as `redirection_operator` matches.
    assert_eq!(
        non_pathname_redirection_target("", "<<<PATH"),
        Some("PATH"),
        "`<<<` must be matched at its FULL length; a short match would leave `<` to be read \
         as the target's first character"
    );
    assert_eq!(
        non_pathname_redirection_target("", "<<-PATH"),
        Some("PATH"),
        "`<<-` likewise"
    );
    assert_eq!(
        non_pathname_redirection_target("", ">&PATH"),
        Some("PATH"),
        "the ATTACHED spelling, which is the one `>&` is measured REACHING in"
    );
    assert_eq!(
        non_pathname_redirection_target("<<<", "PATH"),
        Some("PATH"),
        "the SEPARATED spelling, which is the one `<<<` is measured REACHING in"
    );
    assert_eq!(
        non_pathname_redirection_target(">", "PATH"),
        None,
        "**THE NEGATIVE CONTROL: a PATHNAME operator is not one of these.** Without it the \
         four rows above would pass against a function that answered `Some` for everything, \
         and class 13 would draw class 2's whole alphabet."
    );
}

#[test]
fn the_control_carrier_axis_can_draw_a_carrier_after_an_operator_other_than_a_here_string() {
    // **THE WORD-SET FENCE — ROUND 13's FIRST NEW MECHANICAL ASSERTION, AND THE
    // ONE THAT WOULD HAVE CAUGHT `T-19-122` TWELVE AUDITS EARLIER.**
    //
    // `consume_redirection` records a target on the `Segment` only under
    // `pathname_target && target.literal`, so the target of any of bash's FIVE
    // non-pathname operators is in NEITHER of the two word classes
    // `protected_carrier_named` iterates. Round 12's corpus drew not one such
    // word: audit 12 measured it directly — *"the corpus that certifies the round
    // draws eighty-four rows across twelve classes and not one of them is a
    // here-string or an ancestor"*. **The axis was STRUCTURALLY INCAPABLE of
    // failing on this cell.**
    //
    // **THIS COUNTS ENTRIES RATHER THAN READING PROSE, AND IT COUNTS TWO THINGS
    // RATHER THAN ONE.** The second count is the important one: **an alphabet
    // that only ever drew `<<<` would certify a HERE-STRING-ONLY rule**, which is
    // one operator short in exactly the way the current rule is one word class
    // short — the same trap round 12 refused when it declined an `=`-keyed rule.
    //
    // **GREEN today and after.**
    let mut targets = 0usize;
    let mut non_here_string: Vec<String> = Vec::new();
    for (_, entries) in CONTROL_CARRIER_ALPHABETS {
        for entry in *entries {
            let command = control_carrier_representative(entry);
            let raw: Vec<&str> = command.split_whitespace().collect();
            for (index, word) in raw.iter().enumerate() {
                let previous = index.checked_sub(1).map(|i| raw[i]).unwrap_or("");
                let Some(target) = non_pathname_redirection_target(previous, word) else {
                    continue;
                };
                let carries = std::iter::once(0)
                    .chain(
                        target
                            .char_indices()
                            .filter(|(_, character)| *character == '/')
                            .map(|(i, _)| i),
                    )
                    .any(|i| {
                        target[i..].starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                            || &target[i..] == REPRESENTATIVE_BINARY_PATH
                    });
                if !carries {
                    continue;
                }
                targets += 1;
                let operator = if NON_PATHNAME_REDIRECTION_OPERATORS.contains(&previous) {
                    previous.to_string()
                } else {
                    word[..word.len() - target.len()].to_string()
                };
                if operator != "<<<" {
                    non_here_string.push(format!("{operator} in `{entry}`"));
                }
            }
        }
    }

    assert!(
        targets > 0,
        "\n\n**THE AXIS MUST BE ABLE TO DRAW A CARRIER PATH AFTER A NON-PATHNAME REDIRECTION \
         OPERATOR, AND IT DRAWS {targets}.**\n\n\
         `consume_redirection` (`policy.rs:3581`) pushes a target onto \
         `Segment::redirection_targets` only under `pathname_target && target.literal`, so \
         the target of `<<<`, `<<`, `<<-`, `>&` or `<&` is in NEITHER word class while being \
         fully `Token.literal` and ABSOLUTE. An axis that cannot DRAW that shape cannot FAIL \
         on it, and every floor it satisfies certifies a claim about a class it could never \
         have exercised.\n\n\
         **THE CORRECT RESPONSE IS TO ADD THE SPELLING, NEVER TO RELAX THIS FENCE.**"
    );
    assert!(
        !non_here_string.is_empty(),
        "\n\n**THE AXIS DRAWS {targets} NON-PATHNAME REDIRECTION TARGETS AND EVERY ONE OF THEM \
         IS A HERE-STRING.**\n\n\
         **THAT IS THE `<<<`-ONLY TRAP, AND IT IS THE EXACT SHAPE OF THE DEFECT THIS ROUND \
         EXISTS TO CLOSE.** An alphabet that only ever draws `<<<` certifies a \
         HERE-STRING-ONLY rule — and bash has FIVE non-pathname operators. `>&` with a path \
         is not an fd duplication at all: bash falls back to `&>word` and OPENS THE FILE, \
         which `19-32` measured TRUNCATING the ledger to zero bytes. A rule keyed to one \
         operator would be one spelling short in exactly the way \
         `RedirectionOperator::pathname_target` is one word class short.\n\n\
         **THE CORRECT RESPONSE IS TO ADD A NON-`<<<` SPELLING TO \
         `CONTROL_CARRIER_NON_PATHNAME_TARGET`, NEVER TO RELAX THIS FENCE.**"
    );
    println!(
        "the WORD-SET fence counts {targets} non-pathname redirection targets carrying a \
         carrier path across the fifteen alphabets, of which {} are under an operator OTHER \
         than `<<<`: {:?}",
        non_here_string.len(),
        non_here_string
    );
}

#[test]
fn the_control_carrier_axis_draws_the_ancestor_and_pins_the_stop_from_the_other_side() {
    // **THE ANCESTOR-STOP FENCE — ROUND 13's SECOND NEW MECHANICAL ASSERTION.**
    //
    // It counts TWO things, and the second is what makes the design a BOUNDARY
    // rather than an ESCALATION: **at least one ANCESTOR entry in the FAIL-CLOSED
    // arm, and at least one word AT OR ABOVE the root's own parent in the
    // INVARIANCE arm.** An implementation that walked the whole ancestor chain to
    // `/` turns the second count's rows red.
    //
    // **GREEN today and after.**
    let mut ancestors = 0usize;
    for (_, entries) in CONTROL_CARRIER_FAIL_CLOSED_ALPHABETS {
        for entry in *entries {
            if draws_an_ancestor_of_the_envelope_directory(&control_carrier_representative(entry)) {
                ancestors += 1;
            }
        }
    }
    assert!(
        ancestors > 0,
        "\n\n**THE AXIS MUST DRAW AT LEAST ONE ANCESTOR ENTRY IN A FAIL-CLOSED ARM, AND IT \
         DRAWS {ancestors}.**\n\n\
         `word_is_within` returns early on `word.len() < dir.len()` over COMPONENT VECTORS \
         (`policy.rs:5706`), so every ANCESTOR — being SHORTER — answers `false`, while \
         `rm -rf <root>` takes the same nine carriers in ONE call that the prefix boundary \
         exists to protect. **THE CORRECT RESPONSE IS TO ADD THE SPELLING, NEVER TO RELAX \
         THIS FENCE.**"
    );

    let mut above_the_stop = 0usize;
    for (_, entries) in CONTROL_CARRIER_INVARIANCE_ALPHABETS {
        for entry in *entries {
            if entry.contains(ENVELOPE_PARENT_PLACEHOLDER) {
                above_the_stop += 1;
            }
        }
    }
    assert!(
        above_the_stop > 0,
        "\n\n**THE AXIS MUST DRAW AT LEAST ONE WORD AT OR ABOVE THE ROOT'S OWN PARENT IN THE \
         INVARIANCE ARM, AND IT DRAWS {above_the_stop}.**\n\n\
         **THE CLAUSE STOPS AT `<root>` AND THE REASON IS OWNERSHIP.** `<root>` is created by \
         this tool and holds only alias directories it created; above it the chain is \
         `~/.local/share`, `$HOME`, `/tmp` and `/`, shared with everything the user has. A \
         boundary reaching them would refuse `ls /`, `df /`, `du -sh $HOME` and `ls /tmp` — \
         **not a boundary but an OUTAGE** (AR-19-11: a refusal a user cannot act on is a \
         control that gets switched off). Without an entry above the stop, an implementation \
         that walked the whole chain would pass this axis.\n\n\
         **THE CORRECT RESPONSE IS TO ADD THE SPELLING, NEVER TO RELAX THIS FENCE.**"
    );

    // -- **THE PAIR, ASSERTED BY NAME**, because a pair differing in ONE PATH
    //    COMPONENT is what proves the clause is an ANCESTOR clause rather than a
    //    prefix widened to the root.
    assert!(
        CONTROL_CARRIER_ANCESTOR.contains(&"rm -rf <ENV>"),
        "**`rm -rf <ENV>` MUST BE IN `CONTROL_CARRIER_ANCESTOR`, BY NAME.**"
    );
    assert!(
        CONTROL_CARRIER_ORDINARY_OPERANDS.contains(&"ls <ENV>/unrelated-sibling"),
        "\n\n**`ls <ENV>/unrelated-sibling` MUST BE IN `CONTROL_CARRIER_ORDINARY_OPERANDS`, \
         BY NAME.**\n\n\
         It is round 13's `--signed no`: a PREFIX widened from `<ENV>/alpha` to `<ENV>` \
         refuses it and the mandated ANCESTOR clause does not. **The pair differing in ONE \
         PATH COMPONENT is what tells the two designs apart**, and because \
         `GSD_MM_ENVELOPE_ROOT` is USER-SETTABLE the prefix design would refuse the whole \
         `/tmp` subtree for a user who set the root there — reachable configuration, not a \
         thought experiment."
    );
    let inside = control_carrier_representative("rm -rf <ENV>");
    let outside = control_carrier_representative("ls <ENV>/unrelated-sibling");
    assert!(
        draws_an_ancestor_of_the_envelope_directory(&inside),
        "`{inside}` must draw class 14"
    );
    assert!(
        !draws_an_ancestor_of_the_envelope_directory(&outside),
        "`{outside}` must NOT draw class 14: it is UNDER the root and is not an ancestor of \
         the envelope directory. **That is what makes the pair discriminating.**"
    );
    assert!(
        !draws_an_ancestor_of_the_envelope_directory(&control_carrier_representative("ls <ENVPAR>")),
        "and `ls <ENVPAR>` must not draw it either: the root's own PARENT is ABOVE the stop."
    );
    println!(
        "the ANCESTOR-STOP fence counts {ancestors} ancestor entries in the fail-closed arm \
         and {above_the_stop} word(s) at or above the root's parent in the invariance arm."
    );
    println!("the `T-19-126`(ii) WR-02 note reads: {CONTROL_CARRIER_INTERIOR_PATH_WR_02}");
}

#[test]
fn the_control_carrier_axis_can_draw_a_tilde_a_glob_and_a_brace_list_inside_a_carrier_path() {
    // **THE NO-EXPANSION-SPELLING FENCE — ROUND 11's NEW MECHANICAL ASSERTION,
    // AND THE ONE THAT WOULD HAVE CAUGHT `T-19-115` A ROUND EARLIER.**
    //
    // Rule (a) decides on `Token.literal` — round 5's own bit — and therefore
    // inherits round 5's whole class list by construction. Round 5's literalness
    // table names EXPANSION, PATHNAME, TILDE and BRACE as the four classes that
    // clear the bit. **Round 10's corpus drew ONE of the four**: every one of
    // `ENVELOPE_ROOT_OPERAND_CARRIERS`' eleven entries is an absolute literal
    // path and `CONTROL_CARRIER_EXPANSION_BORNE` is exactly two `$(…)` entries,
    // so the axis was STRUCTURALLY INCAPABLE of failing on three of the rule's
    // own silences.
    //
    // **This counts ENTRIES rather than reading prose**, because the absence of
    // exactly that is `T-19-115`, and a floor that cannot fail on a class is how
    // this failure mode reached its tenth consecutive round.
    //
    // **GREEN today and after.**
    let mut tilde = 0usize;
    let mut glob = 0usize;
    let mut brace = 0usize;
    for (_, entries) in CONTROL_CARRIER_ALPHABETS {
        for entry in *entries {
            let command = control_carrier_representative(entry);
            for word in carrier_words(&command) {
                if word_is_tilde_borne_carrier(&word.text) {
                    tilde += 1;
                }
                if word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                    && word_has_pathname_metacharacter(&word.text)
                {
                    glob += 1;
                }
                if word.text.starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                    && word_has_brace_list(&word.text)
                {
                    brace += 1;
                }
            }
        }
    }
    for (what, count) in [("a TILDE", tilde), ("a GLOB", glob), ("a BRACE LIST", brace)] {
        assert!(
            count > 0,
            "\n\n**THE AXIS MUST BE ABLE TO DRAW {what} INSIDE A CARRIER PATH, AND IT DRAWS \
             {count}.**\n\n\
             Round 5's `Token.literal` table names EXPANSION, PATHNAME, TILDE and BRACE as the \
             four classes that clear the bit rule (a) requires. An axis that cannot DRAW one \
             of them cannot FAIL on it, and every floor it satisfies certifies a claim about a \
             class it could never have exercised.\n\n\
             **THE CORRECT RESPONSE IS TO ADD THE SPELLING, NEVER TO RELAX THIS FENCE.**"
        );
    }
    println!(
        "the NO-EXPANSION-SPELLING fence counts {tilde} tilde, {glob} glob and {brace} \
         brace-list carrier words across the twelve alphabets."
    );
}

#[test]
fn the_control_carrier_axis_can_draw_a_carrier_path_inside_a_word_and_not_only_after_an_equals_sign(
) {
    // **THE NO-INTERIOR-PATH FENCE — ROUND 12's NEW MECHANICAL ASSERTION, AND
    // THE ONE THAT WOULD HAVE CAUGHT `T-19-119` FOUR ROUNDS EARLIER.**
    //
    // Rule (a) decides on `lexical_absolute_components`, which opens with
    // `if !word.starts_with('/') { return None; }` — so **a word can CARRY an
    // absolute path without BEING one, and the whole rule is blind to it.**
    // Round 11's corpus drew not one such word: every entry of the eleven
    // alphabets carries its carrier path at index ZERO of some word, or not at
    // all. **The axis was STRUCTURALLY INCAPABLE of failing on this cell**, and
    // `19-28` observed the cell and classified it "not a defect" — which is how
    // it survived a round that had already found it.
    //
    // **THIS COUNTS ENTRIES RATHER THAN READING PROSE, AND IT COUNTS TWO THINGS
    // RATHER THAN ONE.** The second count is the important one: an alphabet that
    // only ever drew `=` would certify an `=`-KEYED RULE, and an `=`-keyed rule
    // is one character short in exactly the way the current one is —
    // `--opt=a=/p` carries a second `=`, `tar -C/p` and `cp -t/p` carry none at
    // all, and `host:/p` attaches with a `:`. **A list of attachment characters
    // is a PROGRAM-GRAMMAR ENUMERATION and is D-08's defect one level over.**
    //
    // **GREEN today and after.**
    let mut interior = 0usize;
    let mut non_equals_attached: Vec<String> = Vec::new();
    for (_, entries) in CONTROL_CARRIER_ALPHABETS {
        for entry in *entries {
            let command = control_carrier_representative(entry);
            for word in carrier_words(&command) {
                if word.is_redirection_target || !word_carries_an_interior_carrier_path(&word.text)
                {
                    continue;
                }
                interior += 1;
                // The character IMMEDIATELY BEFORE the matching `/`. If it is
                // anything other than `=`, this entry is one an `=`-keyed rule
                // would miss.
                let attached_by_something_else =
                    word.text
                        .char_indices()
                        .any(|(index, character)| {
                            index > 0
                                && character == '/'
                                && (word.text[index..].starts_with(REPRESENTATIVE_ENVELOPE_ROOT)
                                    || &word.text[index..] == REPRESENTATIVE_BINARY_PATH)
                                && word.text[..index].chars().next_back() != Some('=')
                        });
                if attached_by_something_else {
                    non_equals_attached.push(word.text.clone());
                }
            }
        }
    }

    assert!(
        interior > 0,
        "\n\n**THE AXIS MUST BE ABLE TO DRAW A CARRIER PATH AT A NON-ZERO INDEX INSIDE A WORD, \
         AND IT DRAWS {interior}.**\n\n\
         `lexical_absolute_components` requires the WORD to start with `/`, so a path carried \
         inside a token is invisible to BOTH halves of the path set while `Token.literal` is \
         TRUE. An axis that cannot DRAW that shape cannot FAIL on it, and every floor it \
         satisfies certifies a claim about a class it could never have exercised.\n\n\
         **THE CORRECT RESPONSE IS TO ADD THE SPELLING, NEVER TO RELAX THIS FENCE.**"
    );
    assert!(
        !non_equals_attached.is_empty(),
        "\n\n**THE AXIS DRAWS {interior} INTERIOR CARRIER PATHS AND EVERY ONE OF THEM IS \
         ATTACHED BY AN `=`.**\n\n\
         **THAT IS THE `=`-ONLY TRAP, AND IT IS THE EXACT SHAPE OF THE DEFECT THIS ROUND \
         EXISTS TO CLOSE.** An alphabet that only ever draws `=` certifies an `=`-KEYED RULE — \
         and an `=`-keyed rule would be one character short in exactly the way \
         `lexical_absolute_components` is today: it would miss `--opt=a=/p` (a second `=`), \
         `tar -C/p` and `cp -t/p` (ATTACHED SHORT OPTIONS with no `=` anywhere) and `host:/p` \
         (a `:`). **A list of attachment characters is a PROGRAM-GRAMMAR ENUMERATION — `dd`'s \
         `of=` is an operand grammar, not an option syntax — and that is D-08's defect one \
         level over.**\n\n\
         **THE CORRECT RESPONSE IS TO ADD A NON-`=` SPELLING TO \
         `CONTROL_CARRIER_INTERIOR_PATH`, NEVER TO RELAX THIS FENCE.**"
    );
    println!(
        "the NO-INTERIOR-PATH fence counts {interior} interior carrier words across the \
         thirteen alphabets, of which {} are attached by something OTHER than `=`: {:?}",
        non_equals_attached.len(),
        non_equals_attached
    );
}

// ---------------------------------------------------------------------------
// 17b. The generative properties — the invariance arm GREEN today, the
// fail-closed arm RED until `19-27`
// ---------------------------------------------------------------------------

#[test]
fn a_command_whose_operand_resolves_under_the_envelope_root_is_refused_after_19_27() {
    // **THE FAIL-CLOSED ARM. RED at this plan's end BY DESIGN.**
    //
    // Measured at this section's base commit: all 11 entries are at exit 0 with
    // an EMPTY walk. **Those 11 are what makes this property RED.**
    //
    // **The post-fix verdict is DERIVED, not guessed.** `19-27`'s clause is
    // raised on the segment's own literal words against `envelope_dir_in(root,
    // alias)` — a validate-then-`join` with no filesystem probe — BEFORE the
    // resolution match at `hooks.rs:978-994`, so it fires on a segment that
    // reaches no governed program and the reason identifier is the GENERAL
    // unresolvable one rather than one a classifier would have earned.
    // `classify_segments` (`hooks.rs:954-961`) already carries `root` and
    // `alias`, so no new input is introduced.
    let mut cases = 0usize;
    for (alphabet, entries) in [("ENVELOPE_ROOT_OPERAND_CARRIERS", ENVELOPE_ROOT_OPERAND_CARRIERS)] {
      for entry in entries {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let command = control_carrier_command(entry, envelope.path());
        let got = verdict(envelope.path(), &command);
        assert_eq!(
            got.code,
            2,
            "\n\nA COMMAND THAT NAMES ONE OF THIS ENVELOPE'S OWN CONTROL CARRIERS WAS \
             PERMITTED.\n\
             \n  alphabet: {alphabet}\
             \n  entry   : {entry}\
             \n  command : {command}\
             \n  got     : exit {} reason {}\n\
             \n**Every control this envelope installs has a FILE carrier that no command line \
             has to touch.** `rm -f <ledger>` resets the pull-request cap `SECTION_ENVELOPE` \
             lists under **Guaranteed**; `cp /bin/true <hooks>/pre-push` removes layer 3 — \
             SAFE-05's only carrier and `AR-19-03`'s backstop — without touching \
             `GIT_CONFIG_COUNT` at all; and `rm -rf <ENV>/alpha` takes NINE carriers in ONE \
             call, which is why the boundary is the ROOT and not eight filenames.\n\
             \nDriven end to end in `tests/envelope_control_carrier.rs`: the four-call cap \
             reset in ONE persistent root (permitted, refused, refused, deletion, PERMITTED \
             AGAIN with a fresh one-line ledger), and a bare-remote force push that COMPLETED \
             and MOVED `main` with the stub replaced and `GIT_CONFIG_COUNT` untouched.\n\
             \n**This rule reads an OPERAND PATH and never a PROGRAM NAME** — the \
             NO-PROGRAM-NAMES fence above is the corpus's own proof — and it FAILS OPEN in \
             FOUR named directions whose entries are pinned PERMITTED in the invariance arm \
             below.\n\
             \n**ROUND 11 ADDED TWO MORE FAIL-CLOSED ALPHABETS TO THIS ARM.** \
             `CONTROL_CARRIER_REDIRECTION_TARGETS` MOVED here from the invariance arm — all \
             four of its entries carry ABSOLUTE LITERAL targets under the envelope root, and \
             `19-29`'s widened SIGHT reaches them. `CONTROL_CARRIER_BINARY` is new: `C-10`, \
             the binary every hook stub and the guard registration exec, which takes LAYER 2 \
             AND LAYER 3 in one permitted call and which `19-29`'s widened PATH SET reaches as \
             an EXACT PATH.",
            got.code,
            got.reason_id,
        );
        assert_eq!(
            got.reason_id,
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            "`{command}` must be refused UNDER `{}`, the GENERAL unresolvable identifier, and \
             not under one a classifier would have earned — the clause fires before any \
             classifier has looked at the command. Got: {}",
            policy::REASON_ENVELOPE_ASSERTION_FAILED,
            got.reason_id,
        );

        let written = ledger_lines_under(envelope.path());
        assert!(
            written.is_empty(),
            "`{command}` was refused, but a pull-request ledger line was written somewhere \
             under the envelope root. Found: {written:?} Files: {:?}",
            files_under(envelope.path())
        );
        cases += 1;
      }
    }
    assert_eq!(
        cases,
        ENVELOPE_ROOT_OPERAND_CARRIERS.len(),
        "round 10's fail-closed arm must run every generated case"
    );
}

/// The TWO fail-closed alphabets round 11 ADDS — the ones `19-29` turns green.
///
/// **They are a SEPARATE property from round 10's, and the split is honest
/// rather than cosmetic.** `ENVELOPE_ROOT_OPERAND_CARRIERS` is GREEN today:
/// `19-27` landed its rule. These two are RED today and green only after
/// `19-29`. Folding them into one property would make round 10's landed rule
/// indistinguishable from round 11's unlanded one, and a later reader could not
/// tell which half a red belonged to.
const CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_29: &[(&str, &[&str])] = &[
    ("CONTROL_CARRIER_REDIRECTION_TARGETS", CONTROL_CARRIER_REDIRECTION_TARGETS),
    ("CONTROL_CARRIER_BINARY", CONTROL_CARRIER_BINARY),
];

#[test]
fn a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29() {
    // **ROUND 11's FAIL-CLOSED ARM. RED at this plan's end BY DESIGN.**
    //
    // Two alphabets, two widenings, and neither is a widening of the other:
    //
    // * **`CONTROL_CARRIER_REDIRECTION_TARGETS`** — the carrier is INSIDE the
    //   envelope root and the write arrives as a REDIRECTION TARGET, which rule
    //   (a) cannot SEE. This alphabet MOVED here out of the invariance arm,
    //   because all four of its entries' verdict the fix CHANGES. `19-29` widens
    //   what the rule sees to pathname redirection targets, carried on the
    //   SEGMENT the way `Segment::redirection_unresolvable` already travels —
    //   **never as a token in the stream**, which would split a redirected simple
    //   command and turn round 6's headline refusal into a permit. The
    //   SEGMENT-COUNT pins in `tests/envelope_carrier_reach.rs` make that
    //   falsifiable.
    // * **`CONTROL_CARRIER_BINARY`** — `C-10`, `T-19-116`, `high`. The carrier is
    //   OUTSIDE the envelope root entirely, so no amount of widened SIGHT reaches
    //   it; `19-29` widens the PATH SET to
    //   `{envelope_dir_in(root, alias)} ∪ {the binary this guard is running as}`,
    //   the second as an **EXACT PATH** and not a prefix. Driven end to end in
    //   `tests/envelope_carrier_reach.rs`: with the binary replaced by
    //   `/bin/true`, a force push the hook had REFUSED COMPLETED and MOVED a bare
    //   remote's `main`, and the replaced binary answered exit 0 to a guard call
    //   the real binary answered exit 2 to. **Layer 2 and layer 3 in one
    //   permitted call.**
    //
    // The reason identifier is the GENERAL unresolvable one,
    // `envelope_assertion_failed`, for both — the clause fires before any
    // classifier has looked at the command, so it is not one a classifier would
    // have earned (D-24).
    let mut cases = 0usize;
    for (alphabet, entries) in CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_29 {
        for entry in *entries {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let command = control_carrier_command(entry, envelope.path());
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code, 2,
                "\n\nA COMMAND NAMING A CARRIER RULE (a) CANNOT SEE WAS PERMITTED.\n\
                 \n  alphabet: {alphabet}\
                 \n  entry   : {entry}\
                 \n  command : {command}\
                 \n  got     : exit {} reason {}\n\
                 \n**MEASURED AT THIS PLAN'S BASE: every entry here is exit 0 with an EMPTY \
                 walk.** That is what makes this property RED before the fix and able to \
                 certify something after it. Its ORDINARY TWINS are asserted PERMITTED by the \
                 permitted-base fence above, so this arm is not merely drawing commands that \
                 were already refused.",
                got.code, got.reason_id,
            );
            assert_eq!(
                got.reason_id,
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                "`{command}` must be refused UNDER `{}`, the GENERAL unresolvable identifier. \
                 Got: {}",
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                got.reason_id,
            );
            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command}` was refused, but a pull-request ledger line was written. Found: \
                 {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }
    assert_eq!(
        cases,
        CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_29
            .iter()
            .map(|(_, entries)| entries.len())
            .sum::<usize>(),
        "round 11's fail-closed arm must run every generated case"
    );
}

#[test]
fn a_command_carrying_a_carrier_path_inside_a_word_is_refused_after_19_31() {
    // **ROUND 12's FAIL-CLOSED ARM. RED at this plan's end BY DESIGN.**
    //
    // **A SEPARATE PROPERTY FROM ROUNDS 10's AND 11's, AND THE SPLIT IS HONEST
    // RATHER THAN COSMETIC.** `ENVELOPE_ROOT_OPERAND_CARRIERS` is GREEN today
    // (`19-27` landed its rule); round 11's two alphabets are green once `19-29`
    // landed; this one is RED until `19-31`. Folding them into one property
    // would make three rounds' rules indistinguishable and a later reader could
    // not tell which half a red belonged to.
    //
    // **THE DERIVATION.** `19-31` widens what rule (a) READS from THE WORD to
    // EVERY `/`-ANCHORED SUBSTRING of a LITERAL word, each fed through the
    // EXISTING `lexical_absolute_components` normaliser and the EXISTING two
    // comparisons. **It is raised at the SAME ONE SITE, on the segment the walk
    // already holds, BEFORE the resolution match**, so the reason identifier is
    // the GENERAL unresolvable one and not one a classifier would have earned
    // (D-24). **CONTAINMENT is of the NORMALISED STRING rather than of the
    // index**: `lexical_absolute_components` strips leading `./`s BEFORE testing
    // `starts_with('/')`, so the exact claim is that the string today's rule
    // normalises is itself one of the candidates and re-normalises to the same
    // component list — the `./` strip can only ADD answers, never remove one.
    //
    // **MEASURED AT THIS PLAN'S BASE: every entry here is exit 0 with an EMPTY
    // walk, and every ORDINARY TWIN is exit 0 too** (asserted by the
    // permitted-base fence above), so this arm is not merely drawing commands
    // that were already refused.
    let mut cases = 0usize;
    for (alphabet, entries) in [("CONTROL_CARRIER_INTERIOR_PATH", CONTROL_CARRIER_INTERIOR_PATH)] {
        for entry in entries {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let command = control_carrier_command(entry, envelope.path());
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code, 2,
                "\n\nA COMMAND CARRYING A CARRIER PATH INSIDE A LITERAL WORD WAS PERMITTED.\n\
                 \n  alphabet: {alphabet}\
                 \n  entry   : {entry}\
                 \n  command : {command}\
                 \n  got     : exit {} reason {}\n\
                 \n**`T-19-119`, `high`.** `lexical_absolute_components` (`policy.rs:5457`) \
                 opens with `if !word.starts_with('/') {{ return None; }}`, and BOTH halves of \
                 `protected_carrier_named`'s path set are built on it — so a path carried \
                 INSIDE a token is invisible to the whole rule while `Token.literal` is TRUE. \
                 Driven end to end in `tests/envelope_interior_path.rs`: \
                 `dd if=/bin/true of=<BINARY>` is exit 0 against its twin \
                 `cp /bin/true <BINARY>` at exit 2, and the hook-refused force push then MOVED \
                 a bare remote's `main`; the same spelling RESET A FIRED PR CAP.\n\
                 \n**THE BOUNDARY IS `/`-ANCHORED SUBSTRINGS OF A LITERAL WORD AND NOT A LIST \
                 OF ATTACHMENT CHARACTERS.** Entries 7, 8 and 9 attach the path by something \
                 other than `=`, and the NO-INTERIOR-PATH fence counts that — because an \
                 `=`-keyed rule would be one character short in exactly the way the current \
                 one is.",
                got.code, got.reason_id,
            );
            assert_eq!(
                got.reason_id,
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                "`{command}` must be refused UNDER `{}`, the GENERAL unresolvable identifier. \
                 Got: {}",
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                got.reason_id,
            );
            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command}` was refused, but a pull-request ledger line was written. Found: \
                 {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }
    assert_eq!(
        cases,
        CONTROL_CARRIER_INTERIOR_PATH.len(),
        "round 12's fail-closed arm must run every generated case"
    );
}

/// The TWO fail-closed alphabets round 13 ADDS — the ones `19-33` turns green.
///
/// **They are a SEPARATE property from rounds 10's, 11's and 12's, and the split
/// is honest rather than cosmetic.** `ENVELOPE_ROOT_OPERAND_CARRIERS` is GREEN
/// today (`19-27` landed its rule); round 11's two are green (`19-29` landed);
/// `CONTROL_CARRIER_INTERIOR_PATH` is green (`19-31` landed). These two are RED
/// today and green only after `19-33`. Folding them into one property would make
/// four rounds' rules indistinguishable and a later reader could not tell which
/// half a red belonged to.
const CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_33: &[(&str, &[&str])] = &[
    (
        "CONTROL_CARRIER_NON_PATHNAME_TARGET",
        CONTROL_CARRIER_NON_PATHNAME_TARGET,
    ),
    ("CONTROL_CARRIER_ANCESTOR", CONTROL_CARRIER_ANCESTOR),
];

#[test]
fn a_command_naming_a_carrier_in_a_word_class_or_a_path_set_rule_a_has_no_clause_for_is_refused_after_19_33(
) {
    // **ROUND 13's FAIL-CLOSED ARM. RED at this plan's end BY DESIGN.**
    //
    // Two alphabets, two axes, and neither is a widening of the other:
    //
    // * **`CONTROL_CARRIER_NON_PATHNAME_TARGET` — AXIS 1, THE WORD SET.** The
    //   carrier path is INSIDE the envelope root (or IS the binary) and the word
    //   arrives after one of bash's FIVE NON-PATHNAME redirection operators.
    //   `consume_redirection` (`policy.rs:3532-3609`) consumes the word, emits
    //   NO `Token`, and records it on `Segment::redirection_targets` only under
    //   `pathname_target && target.literal` (`:3581`) — so it is in NEITHER of
    //   the two classes `protected_carrier_named` iterates at `:6118-6128`.
    //   **`19-33` carries LITERAL NON-PATHNAME targets on a SECOND `Segment`
    //   FIELD and chains them as a THIRD word class at the SAME reading site.**
    //   **DESIGN (A) — widening `RedirectionOperator::pathname_target` — IS
    //   FORBIDDEN**, and the reason is a MEASURED FENCE rather than a preference:
    //   it turns
    //   `only_a_literal_pathname_target_reaches_the_segment_and_the_tokens_do_not_move`
    //   (`policy.rs:10693-10727`) and the twelve-operator grammar pin (`:10586`,
    //   `:10603`) RED, and design (B) turns NEITHER red.
    //
    // * **`CONTROL_CARRIER_ANCESTOR` — AXIS 3, THE PATH SET.** The carrier path
    //   is an ANCESTOR of the envelope directory. `word_is_within`
    //   (`policy.rs:5685-5711`) is a component-wise PREFIX and returns early on
    //   `word.len() < dir.len()` (`:5706`), so every ancestor — being SHORTER —
    //   answers `false`. **`19-33` adds an ANCESTOR clause BESIDE the prefix,
    //   bounded at `<root>`**, which grows the protected set by EXACTLY ONE path
    //   because the alias is a plain single component. **It is NOT the existing
    //   prefix widened to the root**, and `ls <ENV>/unrelated-sibling` in the
    //   invariance arm is what tells them apart.
    //
    // The reason identifier is the GENERAL unresolvable one,
    // `envelope_assertion_failed`, for both — the clause fires before any
    // classifier has looked at the command, so it is not one a classifier would
    // have earned (D-24).
    //
    // **MEASURED AT THIS PLAN'S BASE: every entry here is exit 0 with an EMPTY
    // walk, and every ORDINARY TWIN is exit 0 too** (asserted by the
    // permitted-base fence above), so this arm is not merely drawing commands
    // that were already refused.
    let mut cases = 0usize;
    for (alphabet, entries) in CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_33 {
        for entry in *entries {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let command = control_carrier_command(entry, envelope.path());
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code, 2,
                "\n\nA COMMAND NAMING A CARRIER IN A WORD CLASS OR A PATH SET RULE (a) HAS NO \
                 CLAUSE FOR WAS PERMITTED.\n\
                 \n  alphabet: {alphabet}\
                 \n  entry   : {entry}\
                 \n  command : {command}\
                 \n  got     : exit {} reason {}\n\
                 \n**`T-19-122` and `T-19-123`, both `high`, both BLOCKING.** Driven end to \
                 end in `tests/envelope_word_set.rs`: a here-string deletion removed the hook \
                 stubs and a force push the hook had REFUSED then COMPLETED and MOVED a bare \
                 remote's `main`; the same spelling RESET A FIRED PR CAP; and `rm -rf <ENV>` \
                 did both one component up.\n\
                 \n**THE RESIDUE CONDITION ENUMERATED THE SILENCES OF A *READING*, AND A \
                 BOUNDARY HAS THREE WAYS TO BE SILENT** — which words reach the reader, what \
                 the reader sees in a word, and which paths the comparison protects. Round 12 \
                 got the second right and inherited two holes it never named.",
                got.code, got.reason_id,
            );
            assert_eq!(
                got.reason_id,
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                "`{command}` must be refused UNDER `{}`, the GENERAL unresolvable identifier. \
                 Got: {}",
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                got.reason_id,
            );
            let written = ledger_lines_under(envelope.path());
            assert!(
                written.is_empty(),
                "`{command}` was refused, but a pull-request ledger line was written. Found: \
                 {written:?} Files: {:?}",
                files_under(envelope.path())
            );
            cases += 1;
        }
    }
    assert_eq!(
        cases,
        CONTROL_CARRIER_FAIL_CLOSED_AFTER_19_33
            .iter()
            .map(|(_, entries)| entries.len())
            .sum::<usize>(),
        "round 13's fail-closed arm must run every generated case"
    );
}

#[test]
fn the_two_git_dir_spellings_are_pinned_by_name_on_opposite_sides_of_the_boundary() {
    // **THE ONE-CHARACTER PAIR, ASSERTED BY NAME — because the pair differing in
    // exactly one character is what proves the rule reads the PATH SET rather
    // than the `=`.**
    //
    // **GREEN today and after.**
    assert!(
        CONTROL_CARRIER_ORDINARY_OPERANDS
            .iter()
            .any(|entry| entry.contains("--reference=/tmp/g/x")),
        "**`chmod --reference=/tmp/g/x /tmp/y` MUST BE IN \
         `CONTROL_CARRIER_ORDINARY_OPERANDS`.** It is the outside-the-path-set twin of \
         `chmod --reference=<ENV>/alpha/pr-ledger.ndjson /tmp/x`, in the SAME attachment, and \
         a rule keyed to `=` rather than to the path set turns it RED."
    );
    assert!(
        CONTROL_CARRIER_INTERIOR_PATH
            .iter()
            .any(|entry| entry.contains("-C<ENV>/alpha")),
        "**AN ATTACHED SHORT OPTION WITH NO `=` MUST BE IN \
         `CONTROL_CARRIER_INTERIOR_PATH`.** Without it the alphabet certifies an `=`-keyed \
         rule, which is D-08's defect one level over."
    );
    assert!(
        CONTROL_CARRIER_ORDINARY_OPERANDS
            .iter()
            .any(|entry| *entry == "tar --create --file /tmp/t -C/tmp/g"),
        "**AND ITS OUTSIDE-THE-PATH-SET TWIN MUST BE IN THE ORDINARY ALPHABET.** A fail-closed \
         entry whose twin is not pinned permitted certifies nothing about WHY it is refused."
    );

    // -- And the pair itself, at the predicate level.
    let inside = control_carrier_representative("tar --create --file /tmp/t -C<ENV>/alpha");
    let outside = "tar --create --file /tmp/t -C/tmp/g";
    assert!(
        draws_an_interior_carrier_path(&inside),
        "`{inside}` must draw class 12"
    );
    assert!(
        !draws_an_interior_carrier_path(outside),
        "`{outside}` must NOT draw class 12: the attachment is IDENTICAL and only the path \
         differs. **That is what makes the pair discriminating.**"
    );
    assert!(
        draws_an_ordinary_operand(outside),
        "`{outside}` must draw class 7, the ordinary complement."
    );
}

#[test]
fn the_verdict_preserving_control_carrier_alphabets_stay_permitted_before_and_after() {
    // **THE INVARIANCE ARM. GREEN today and after.**
    //
    // **THIS IS WHAT MAKES THE CORPUS ABLE TO FAIL ON A RULE THAT QUIETLY WIDENED
    // PAST ITS STATED BOUNDARY.** Rule (a) reads ABSOLUTE LITERAL operands and
    // resolves them LEXICALLY. **Its SILENCE IS A PERMIT in FOUR directions:**
    //
    //   (i)   a REDIRECTION TARGET is not an operand — `tokenize` deletes the
    //         operator AND its target, and reading `>` would re-open a model five
    //         rounds have pinned shut (`SEPARATORS`, one commit, `84a9b05`);
    //   (ii)  an EXPANSION-BORNE operand cannot be resolved — refusing every
    //         non-literal operand of an ungoverned command would deny
    //         `rm $TMPDIR/x`;
    //   (iii) a SYMLINK is not followed — following one means `readlink` /
    //         `canonicalize` on the guard's critical path, which is the I/O and
    //         the TOCTOU both prohibited;
    //   (iv)  a RELATIVE path is not resolved — the guard has no cwd.
    //
    // **Directions (iii) and (iv) are NARROWED by a MEASURED partial mitigation
    // rather than open in every spelling**: `ln -s <ENV>/… <LINK>` and
    // `cd <ENV>/alpha` both name an envelope path as their OWN operand and live in
    // `ENVELOPE_ROOT_OPERAND_CARRIERS`, where they are asserted REFUSED.
    let mut cases = 0usize;
    for (name, entries) in CONTROL_CARRIER_INVARIANCE_ALPHABETS {
        for entry in *entries {
            let envelope = TempDir::new().expect("a temporary envelope root");
            let command = control_carrier_command(entry, envelope.path());
            let got = verdict(envelope.path(), &command);
            assert_eq!(
                got.code,
                0,
                "\n\nA VERDICT-PRESERVING CONTROL-CARRIER ENTRY WAS REFUSED.\n\
                 \n  alphabet : {name}\
                 \n  entry    : {entry}\
                 \n  command  : {command}\
                 \n  got      : exit {} reason {}\n\
                 \n**These five alphabets are pinned exit 0 BEFORE AND AFTER `19-27`.** Four of \
                 them carry rule (a)'s four disclosed FAIL-OPEN directions and the fifth is the \
                 ordinary-operand half that stops the rule becoming \"refuse every `rm`\".\n\
                 \nA red here means the rule reached PAST its stated boundary — into \
                 redirection targets, into expansions, into symlinks, into relative paths, or \
                 into ordinary commands. **Every one of those is a widening that would ship \
                 uncosted if this arm did not exist**, and two of them would break assertions \
                 in files `19-27` may not edit: `git x2>/tmp/o push --force origin main` is \
                 pinned PERMITTED by round 6's over-deletion control, and \
                 `policy::is_separator(\">\")` is pinned `false`.",
                got.code,
                got.reason_id,
            );
            cases += 1;
        }
    }
    assert_eq!(
        cases,
        CONTROL_CARRIER_INVARIANCE_ALPHABETS
            .iter()
            .map(|(_, entries)| entries.len())
            .sum::<usize>(),
        "the invariance arm must run every generated verdict-preserving case"
    );
    println!("control-carrier invariance arm drove {cases} verdict-preserving cases.");
}

#[test]
fn the_repo_side_control_carrier_alphabet_is_recorded_and_never_asserted() {
    // **CONTROL (e) — NO RULE AT ALL, and the verdicts are RECORDED.**
    //
    // `19-27` writes no rule for `C-11` … `C-15`: none is under the envelope root,
    // so rule (a)'s boundary does not reach any of them, and widening it to the
    // repository would mean the guard deciding about `.git/**` on every tool call
    // with `project_root` an `Option`. Their headline delivery is a REDIRECTION
    // TARGET the deletion model deliberately does not read, so a rule naming them
    // would be silent at exactly the measured row. And layer 3 — the only place
    // `.git/config` could be observed without TOCTOU — **is disabled by the very
    // carrier in question**, because the alias sets `core.hooksPath` away before
    // the hook could run.
    //
    // **An assertion in EITHER direction is wrong here.** REFUSED lands
    // permanently red in a file `19-27` may not edit; PERMITTED pins a live bypass
    // as correct. `19-22` asserted such a row against its own comment, its SUMMARY
    // and its plan-check, and it halted `19-23` mid-plan.
    for entry in CONTROL_CARRIER_REPO_SIDE {
        let envelope = TempDir::new().expect("a temporary envelope root");
        let command = control_carrier_command(entry, envelope.path());
        let got = verdict(envelope.path(), &command);
        println!(
            "RECORDED (not asserted) [CONTROL_CARRIER_REPO_SIDE]\n  command : {command}\n  \
             exit    : {}\n  reason  : {}\n  walk    : {:?}",
            got.code,
            got.reason_id,
            files_under(envelope.path())
        );
    }
}
