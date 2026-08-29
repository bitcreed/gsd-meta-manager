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
const ASSIGNMENT_PREFIXES: &[&str] = &["", "FOO=bar ", "LC_ALL=C TZ=UTC ", "EMPTY= "];

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
];

/// How many wrappings each refused base gets. 12 x 140 = 1680, over the 1500
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
