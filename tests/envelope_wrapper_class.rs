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

use std::collections::BTreeSet;
use std::path::Path;

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
        }
    }

    fn apply(self, inner: &str) -> String {
        match self {
            ShellLayer::None => inner.to_string(),
            ShellLayer::ShSingleQuoted => format!("sh -c '{inner}'"),
            ShellLayer::BashDashLC => format!("bash -lc \"{inner}\""),
        }
    }
}

const SHELL_LAYERS: &[ShellLayer] = &[
    ShellLayer::None,
    ShellLayer::None,
    ShellLayer::ShSingleQuoted,
    ShellLayer::BashDashLC,
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
];

/// How many wrappings each refused base gets. 15 x 140 = 2100, over the 1500
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

    for (label, raw, code, anchor, other_anchor) in [
        (
            "src/envelope/policy.rs",
            POLICY_SOURCE,
            policy.as_str(),
            POLICY_ANCHOR,
            HOOKS_ANCHOR,
        ),
        (
            "src/envelope/hooks.rs",
            HOOKS_SOURCE,
            hooks.as_str(),
            HOOKS_ANCHOR,
            POLICY_ANCHOR,
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
            code.len() * 4 > raw.len(),
            "POSITIVE CONTROL FAILED for `{label}`: stripping comments and the \
             `#[cfg(test)]` module left {} of {} bytes — under a quarter, which means the \
             stripper has removed most of the file and an absence assertion over what \
             remains proves almost nothing.",
            code.len(),
            raw.len()
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
];

/// 9 x 120 = 1080, over the 1000 floor the plan sets.
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
fn every_alphabet_this_plan_widens_can_draw_an_expansion_metacharacter() {
    // **The direct mechanical inverse of audit 3's finding, and the floor that
    // makes the finding un-reintroducible.** The auditor established the defect
    // by READING the alphabets; this asserts the repaired fact so that narrowing
    // one of them back turns this red instead of quietly restoring a corpus that
    // cannot fail on its own class.
    //
    // `19-15` extends the same floor to `SHELL_LAYERS` — which still cannot draw
    // `{ …; }` or `( … )` — and to the severed-prefix alphabet Rule B needs.
    for (name, entries) in [
        ("ASSIGNMENT_PREFIXES", ASSIGNMENT_PREFIXES),
        ("REFUSED_BASES", REFUSED_BASES),
        ("DECOY_OPERANDS", DECOY_OPERANDS),
        ("EXPANSION_WRAPPERS", EXPANSION_WRAPPERS),
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
fn expansion_spelling(spelling: usize, name: &str, word: &str) -> String {
    match spelling {
        0 => format!("${name}"),
        1 => format!("${{{name}}}"),
        _ => format!("`true`{word}"),
    }
}

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
