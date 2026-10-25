//! The journal module's surviving helpers.
//!
//! **Phase 31 trimmed this module to what live code still reads.** The run
//! journal itself - the append-only `journal.jsonl` writer, `run.json`, the
//! `active` pointer, `RunPaths`, `JournalEvent`, `RunRecord`, `JournalRun` and the
//! tolerant record reader - was the legacy `drive` run directory's storage and
//! went with the envelope guard that was its last live writer. Runs a previous
//! version left under `.planning/meta-manager/runs/` stay on disk, ignored. What
//! stays:
//!
//! - [`is_plain_path_component`]: the one identity-seam predicate (aliases, run
//!   ids and phase tokens are interpolated into filesystem paths through it);
//! - [`classify_change`] and [`ChangeKind`]: the change classifier the watcher
//!   uses;
//! - [`sha256_digest`] and [`argv_digest`]: the digests the opt-in
//!   re-confirmation and the launcher use;
//! - [`redact`]: the SAFE-04 capture-path redactor the launcher's push step
//!   routes child output through;
//! - [`reader::tail_file`] and [`reader::TailCursor`]: the byte-offset tail the
//!   Driver tab uses to follow a launcher run's `LOG.md`.
//!
//! **Parsing is tolerant by construction and never fatal (D-30).** Serde's
//! strict unknown-field rejection attribute (`deny_unknown_fields`) is never
//! opted into anywhere under `src/`, and that absence is enforced by a test
//! rather than asserted by this paragraph:
//! `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection`
//! walks every non-comment line in the tree and fails if the attribute appears.

pub mod reader;
pub mod redact;

use std::path::{Component, Path};

/// Upper bound on any single string leaf the redactor will pass through (D-31).
///
/// 8 KiB. Phase 15's `tests/fixtures/transcripts/README.md` records base64
/// thinking-block signatures at **4-8 KB**, so this admits one full signature and
/// truncates a pathological one. A defensible starting value with **no tuning
/// data behind it**; it is a named constant so tuning is a one-line change.
/// [`redact`] is its one consumer.
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 8 * 1024;

/// Upper bound on the bytes one tail read will consume (D-13).
///
/// 4 MiB, a wide margin above [`MAX_EVENT_PAYLOAD_BYTES`] because a single tail
/// legitimately batches many lines;
/// the branch that handles it is pure defence against a corrupted file, and
/// without it a line that can never complete would stall the cursor forever and
/// make every later line invisible.
///
/// A defensible starting value with **no tuning data behind it** - it is a named
/// constant so tuning is a one-line change.
pub const MAX_TAIL_BYTES: u64 = 4 * 1024 * 1024;

/// Whether `value` names **exactly one plain path component** (D-27, WR-02).
///
/// Non-empty, `Path::new(value).components()` yields exactly one item, that
/// item is a [`Component::Normal`], and its text is the whole of `value`.
///
/// **The property is general, and it has several consumers** (D-03): a
/// **registry alias** joined under the envelope directory by
/// [`crate::envelope::envelope_dir`], a launcher **run id**, and a **target
/// phase** token. All are user-typed strings interpolated into a filesystem
/// path, which is the whole of what this predicate is about - the name says the
/// property rather than the first caller that needed it. (The legacy journal
/// writer that first needed it was deleted in Phase 31.)
///
/// **The failure it exists to prevent was reproduced, not imagined.**
/// `--run-id '../../../../escaped'` created `run.json` and `journal.jsonl`
/// outside the project, in a directory with no `.gitignore`, **with exit 0**
/// (WR-02, against the legacy journal writer).
///
/// **The alternative D-03 declined: a second, alias-specific validator.**
/// Keeping this one named for run ids and adding another for aliases is exactly
/// how the next caller escapes validation — the argument the (since deleted)
/// `run_paths` signature made about infallible variants. Two predicates for one
/// property is one predicate plus a hole. So the predicate was **promoted** and
/// every caller swept in the same commit; the compiler enumerated the set.
///
/// **The final equality is not redundant belt-and-braces.** `components()`
/// silently normalises a leading `./` and a trailing `/` away, so `"./escape"`
/// and `"escape/"` both yield one `Normal` — comparing the component back
/// against the original string is what refuses a value whose written form is not
/// the plain name it resolves to.
///
/// Modelled on [`classify_change`], which already rejects every non-`Normal`
/// component and, like this, **refuses to normalise**: normalising means asking
/// the filesystem, and asking the filesystem is precisely what a traversal check
/// must not depend on. `..` is rejected as a *token*, not resolved.
///
/// The alternative that was declined: canonicalising the joined path and
/// checking it is a descendant of the runs root. That reads the filesystem
/// (so it answers differently for a path that does not exist yet, which is
/// every new run), it follows symlinks (so a symlink planted by the driven
/// agent decides the answer), and it cannot run on the `notify` callback
/// thread. A token check has none of those properties.
///
/// **Two shapes it used to accept, and the correction rides the commit that
/// falsified the old text** (a precedent the retired dry-run preview set):
///
/// * **Whitespace-only.** Only the *empty* string was short-circuited, so
///   `"   "` yielded exactly one [`Component::Normal`] whose text was the whole
///   of the value and the predicate returned `true`. `--run-id '   '` therefore
///   passed the seam and named a run directory made of spaces. Worse, the
///   round-3 review cited this predicate as the reason a blank `--target-phase`
///   needed no check of its own — a justification that was simply false, and
///   the exemption it bought is what let round-4's Critical through.
/// * **Embedded control characters.** `"a\nEVIL"` is one `Normal` component, so
///   it passed, and the dry-run render then printed the newline verbatim inside
///   what reads as a pasteable command line.
/// * **Zero-width and format characters only**, which `str::trim` cannot see. A
///   value of one `U+200B` passed every check above and was reproduced end to
///   end by pass 5: a **complete run** whose directory name was one invisible
///   character, and two visually identical registry aliases resolving to two
///   different envelope paths. The blank half of this predicate now delegates to
///   [`crate::text::carries_visible_content`] — the one production spelling of
///   the judgment — instead of carrying a second, weaker one of its own. Refused
///   as of that commit; the correction rides it.
/// * **EMBEDDED invisible formatting** — a value that renders exactly like a
///   sibling and carries different bytes. The shape above closes values that are
///   *wholly* invisible; this one carries visible content too, so
///   [`crate::text::carries_visible_content`] accepts it **by construction** and
///   the blank half above can never see it. Pass 5 named the harm ("two visually
///   identical aliases resolve to two different envelope and credential paths"),
///   round 5 claimed to close it with the emptiness judgment alone, and pass 6
///   reproduced it unchanged: `"demo"` and `"demo\u{200b}"` both accepted, two
///   envelope roots rendering as one; `"…-aaaa"` and `"…-aaaa\u{200b}"` both
///   accepted, two run directories, one rendering — a terminal record no human
///   can tell from its sibling. Refused as of D-17-1 by delegating to
///   [`crate::text::carries_invisible_formatting`], the second judgment over the
///   same class spelling; the correction rides that commit.
///
/// * **ANY value outside the identity alphabet** — the fifth shape, and the one
///   that subsumes every character shape above it by construction (D-19-2,
///   Premise 8). The four corrections above were all deny-list widenings, and
///   this phase widened the deny-list at three successive levels while each new
///   level stayed a subset: pass 7 measured `"demo\u{202e}"`, `"2\u{202e}0"`,
///   `"2\u{e0041}0"` and `"2\u{ad}0"` all ACCEPTED here, and `run_paths` (since
///   deleted) handing back a directory for them. An identity needs no script at all, so the
///   direction inverts: the accepted set is the finite, printable
///   `[A-Za-z0-9._-]` ([`crate::text::is_identity_char`]), which cannot be one
///   code point short. Bidi controls, the tag block, variation selectors and
///   homoglyphs all fall out of one clause. The recorded product trade lives on
///   `is_identity_char`.
///
/// All five are now refused. None was ever a legitimate run id, alias, or phase
/// name; every value the suite pins as acceptable (`"20"`, `"2.1"`, real
/// timestamped run ids, registered aliases) still passes.
///
/// **The clause layering, stated honestly rather than tidied.** The blank,
/// control and invisible-formatting clauses below are now REDUNDANT for
/// refusal — nothing they reject survives the alphabet clause. They are kept
/// anyway, and not out of caution: message-bearing consumers read them
/// (`crate::registry::Alias::new` gives `NotVisible` and `InvisibleFormatting`
/// their own honest wording by asking the same questions in the same order), and
/// the two-judgments drift pins in `crate::text` assert on them directly.
/// Deleting them would couple those consumers to this function's internals,
/// which is the coupling this module was split up to remove.
///
/// **The named trade (D-17-1).** An identity may not carry ZWJ/ZWNJ, which are
/// load-bearing in some scripts. Free text — goals, commands — is unaffected,
/// because only identity seams consult this predicate; a directory whose *name*
/// carries format characters registers under an explicitly chosen alias.
pub fn is_plain_path_component(value: &str) -> bool {
    // Blank rather than merely empty, and judged by the ONE production spelling
    // of blankness rather than by a `trim` of this function's own. A component
    // with nothing visible in it names nothing a reader could identify, and it
    // reached a run directory.
    if !crate::text::carries_visible_content(value) {
        return false;
    }
    // A control character has no business in a path component, and one that
    // reaches a rendered preview breaks the line the preview appears to be.
    //
    // **This is a SEPARATE question from the one above and is kept.** Visibility
    // asks whether anything can be seen at all; this asks whether a control
    // character is EMBEDDED in otherwise-visible text — `"a\nEVIL"` carries
    // plenty of visible content and is still not a name.
    if value.chars().any(|c| c.is_control()) {
        return false;
    }
    // Identity, not emptiness — a THIRD question, and the one the two above
    // structurally cannot ask. Both of them are satisfied by `"demo\u{200b}"`:
    // it carries visible content and no control character. What it also carries
    // is a byte that renders as nothing, so it names the same thing on screen as
    // `"demo"` while naming a different directory on disk (D-17-1).
    if crate::text::carries_invisible_formatting(value) {
        return false;
    }
    // The FINITE direction, and the one that structurally cannot be one code
    // point short (D-19-2). Everything above this line is a deny-list; this is
    // the allow-list. It subsumes them all for refusal purposes — see the doc
    // above for why they are nonetheless kept.
    if !value.chars().all(crate::text::is_identity_char) {
        return false;
    }
    let mut components = Path::new(value).components();
    let Some(Component::Normal(name)) = components.next() else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }
    name == std::ffi::OsStr::new(value)
}

/// A non-cryptographic identity digest of a spawned command line.
///
/// FNV-1a 64 over `argv` joined by the ASCII unit separator, rendered as
/// `fnv1a64:` plus 16 lowercase hex digits. Two facts about it are deliberate:
///
/// - **It is not a security control.** It exists so two runs can be compared for
///   "same command line" in a `run.json`, nothing more. Nothing authenticates,
///   authorises or trusts anything on the strength of this value.
/// - **It stays inline even though a hashing crate is now present.** Phase 21
///   added `sha2` to `Cargo.toml` for the driver opt-in's re-confirmation
///   digest, so the second half of this bullet's original claim — that no
///   hashing crate existed and none was warranted — stopped being true. This
///   function deliberately does **not** switch: a command-line identity
///   fingerprint has no adversary, and rewriting it would churn every
///   `fnv1a64:` value already sitting in a `run.json` for no gain. The function
///   that does need collision resistance is [`sha256_digest`], and it is a
///   sibling rather than a replacement so the two cannot be confused.
pub fn argv_digest(argv: &[String]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let joined = argv.join("\u{1f}");
    let mut hash = OFFSET_BASIS;
    for byte in joined.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}

/// A collision-resistant digest of arbitrary bytes, rendered as `sha256:` plus
/// 64 lowercase hex digits.
///
/// **A sibling of [`argv_digest`], never a replacement.** The two exist for
/// opposite reasons and the prefixes are what keep them apart on disk:
///
/// - `argv_digest` answers "same command line?" for a human reading a
///   `run.json`. It has no adversary and says so.
/// - This one backs the driver opt-in's re-confirmation prompt, which **is** a
///   security affordance. The tool drives other people's cloned repositories,
///   whose `CLAUDE.md` can change under a `git pull` the user never read, so
///   the question "did these bytes change?" has to survive someone who wants
///   the answer to be no.
///
/// The prefix is not decoration. A recorded digest is compared as a whole
/// string, so a legacy `fnv1a64:` value can never accidentally compare equal to
/// a freshly computed `sha256:` one — the upgrade cannot be defeated by leaving
/// an old record in place, and no user's `config.json` needs a migration.
/// The hex rendering is written out by hand rather than through a `hex` crate:
/// `sha2` 0.11 returns a `hybrid_array::Array`, which — unlike 0.10's
/// `GenericArray` — does not implement `LowerHex`, and one `write!` per byte is
/// a smaller thing to own than another dependency.
pub fn sha256_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    hasher.update(bytes);

    let mut out = String::from("sha256:");
    for byte in hasher.finalize() {
        // `write!` to a String is infallible; the result is discarded rather
        // than unwrapped so this cannot panic on a path with no error to report.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// What kind of write a watched path change represents (D-11).
///
/// `Hash` is load-bearing, not decorative: the watcher's debounce dedup becomes
/// per-`(root, classification)` rather than per-root, and this value is half of
/// that key (D-10).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// A change to a launcher run's own files under
    /// `.planning/meta-manager/drivers/<run-id>/` (30-01, D-24): the run directory
    /// itself appearing or disappearing, its `DIRECTION.md`, or its `LOG.md`.
    /// Routes to a launcher-run refresh for that run, never to a project re-parse
    /// (OBS-06): the launcher writes these files often, and none of them is
    /// planning state.
    DriverRun {
        /// The run directory's name, which is the run id.
        run_id: String,
    },
    /// A path under `.planning/meta-manager/drivers/` that no consumer reads: the
    /// splice temps `.DIRECTION.md.fm.*`, `.DIRECTION.md.edit`, `state.json`,
    /// `launcher.lock`, `DIRECTION.lock`, editor swap files, anything deeper, and
    /// the bare `drivers` directory itself. Dropped before an action is built
    /// (T-30-03): the splice alone rewrites its temp and `state.json` on every
    /// heartbeat, and a re-parse per heartbeat is the denial of service this
    /// variant exists to stop.
    ///
    /// **Also every path at or below `.planning/meta-manager/runs/`** (Phase 31, D-03):
    /// the legacy `drive` journal-run directory, which nothing in the TUI reads any
    /// more. Runs a previous version left on disk stay there (they are not migrated or
    /// deleted) and are inert: they trigger neither a journal tail nor a re-parse.
    Ignored,
    /// Everything else. Keeps today's re-parse behaviour.
    Planning,
}

/// Classify a changed path as a launcher-run write, an ignorable write or a planning
/// write.
///
/// **A pure function with no filesystem access (D-11).** It must not stat,
/// canonicalise or read anything, because it runs on the `notify` debouncer's
/// callback thread, which has no runtime and no `App`. Purity is also what lets
/// it be tested exhaustively against path shapes the way `extract_project_root`
/// already is at `src/watcher.rs:97-116`.
///
/// Two aspects of the rule are deliberate:
///
/// - **`Planning` is the default**, so any path shape this function does not
///   recognise keeps today's behaviour rather than silently losing a re-parse.
/// - **It anchors on the full three-component `.planning/meta-manager/runs/`
///   prefix, never on a bare `runs` component.** A project that happens to keep
///   `.planning/runs/` for its own purposes must not have its writes
///   misclassified as ignorable, because a misclassified planning write is a
///   dashboard that silently stops updating (RESEARCH Security Domain, tampering
///   row; T-16-07).
///
/// **`.planning/meta-manager/runs/` is inert (Phase 31, D-03).** It held the legacy
/// `drive` journal runs, which nothing in the TUI reads any more, and previous
/// versions' runs stay on disk. Every path at or below it is [`ChangeKind::Ignored`]:
/// the shared `.gitignore`, the `active` pointer, `run.lock`, the run directory
/// itself and everything deeper. Before this, writes under it routed to a journal
/// tail, and the shallower ones to a full re-parse.
///
/// **`.planning/meta-manager/drivers/` is an ALLOWLIST (30-01, D-24).** The
/// run directory (`drivers/<id>`), `drivers/<id>/DIRECTION.md` and
/// `drivers/<id>/LOG.md` are [`ChangeKind::DriverRun`]; every other shape under
/// `drivers/` is [`ChangeKind::Ignored`], so a file added to a run directory later
/// is silent until someone decides it should not be. The same three-component
/// anchor applies: a project's own `.planning/drivers/` stays `Planning`.
pub fn classify_change(project_root: &Path, changed_path: &Path) -> ChangeKind {
    let Ok(relative) = changed_path.strip_prefix(project_root) else {
        // Not under this project at all.
        return ChangeKind::Planning;
    };

    let mut parts: Vec<&str> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(name) => parts.push(name),
                // Non-UTF-8 component: unrecognised, so it takes the default.
                None => return ChangeKind::Planning,
            },
            // `..`, a root, or a prefix inside the relative remainder means the
            // path is not a plain descendant. Do not try to normalise it here —
            // normalising is exactly the filesystem access this function forbids.
            _ => return ChangeKind::Planning,
        }
    }

    if parts.len() >= 3
        && parts[0] == ".planning"
        && parts[1] == "meta-manager"
        && parts[2] == "runs"
    {
        return ChangeKind::Ignored;
    }

    if parts.len() >= 3
        && parts[0] == ".planning"
        && parts[1] == "meta-manager"
        && parts[2] == "drivers"
    {
        return match parts.as_slice() {
            [_, _, _, run_id] | [_, _, _, run_id, "DIRECTION.md" | "LOG.md"] => {
                ChangeKind::DriverRun {
                    run_id: (*run_id).to_string(),
                }
            }
            // The bare `drivers` directory, a splice temp, `state.json`, a lock,
            // an editor swap file, a deeper path: nothing reads them.
            _ => ChangeKind::Ignored,
        };
    }

    ChangeKind::Planning
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_change_ignores_every_legacy_runs_path() {
        let root = Path::new("/home/user/projects/myapp");
        let run = ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9";
        let cases = [
            ".planning/meta-manager/runs".to_string(),
            ".planning/meta-manager/runs/.gitignore".to_string(),
            ".planning/meta-manager/runs/active".to_string(),
            ".planning/meta-manager/runs/run.lock".to_string(),
            run.to_string(),
            format!("{run}/journal.jsonl"),
            format!("{run}/run.json"),
            format!("{run}/inbox/x"),
            format!("{run}/inbox.jsonl"),
            format!("{run}/nested/run.json"),
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(&case)),
                ChangeKind::Ignored,
                "{case} is a legacy run path and must be ignored"
            );
        }
    }

    #[test]
    fn classify_change_names_state_md_as_planning() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            ".planning/STATE.md",
            ".planning/ROADMAP.md",
            ".planning/phases/16-run-journal-state-substrate/16-01-PLAN.md",
            ".planning/meta-manager/QUEUE.md",
            "src/main.rs",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} must classify as a planning write"
            );
        }
    }

    #[test]
    fn classify_change_does_not_treat_a_bare_runs_directory_as_inert() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            // A project that keeps its own `runs/` under `.planning/`. The inert
            // prefix is `meta-manager/runs/`, never a bare `runs`.
            ".planning/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
            ".planning/runs/anything",
            ".planning/runs/x",
            "meta-manager/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} must keep re-parsing the project"
            );
        }
    }

    #[test]
    fn classify_change_routes_driver_run_paths() {
        let root = Path::new("/home/user/projects/myapp");
        let run_id = "20261020-driver-tab";
        let cases = [
            // The run directory appearing or disappearing.
            ".planning/meta-manager/drivers/20261020-driver-tab",
            ".planning/meta-manager/drivers/20261020-driver-tab/DIRECTION.md",
            ".planning/meta-manager/drivers/20261020-driver-tab/LOG.md",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::DriverRun {
                    run_id: run_id.to_string()
                },
                "{case} must classify as a driver run change"
            );
        }
    }

    #[test]
    fn classify_change_ignores_every_other_drivers_path() {
        let root = Path::new("/home/user/projects/myapp");
        let run = ".planning/meta-manager/drivers/20261020-driver-tab";
        let cases = [
            format!("{run}/state.json"),
            format!("{run}/.DIRECTION.md.fm.abc"),
            format!("{run}/.DIRECTION.md.edit"),
            format!("{run}/launcher.lock"),
            format!("{run}/DIRECTION.lock"),
            format!("{run}/.DIRECTION.md.edit.swp"),
            format!("{run}/a/b/c"),
            // Deeper paths named like the allowed files are still deeper paths.
            format!("{run}/a/DIRECTION.md"),
            format!("{run}/LOG.md/x"),
            // The bare drivers directory itself (inferred: it names no run).
            ".planning/meta-manager/drivers".to_string(),
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(&case)),
                ChangeKind::Ignored,
                "{case} must be ignored"
            );
        }
    }

    #[test]
    fn classify_change_keeps_a_project_owned_drivers_dir_as_planning() {
        let root = Path::new("/home/user/projects/myapp");
        // No `meta-manager` component: the project's own directory, whose writes
        // must keep re-parsing the project.
        for case in [
            ".planning/drivers/x/DIRECTION.md",
            ".planning/drivers/x",
            ".planning/runs/x",
            "drivers/x/DIRECTION.md",
            ".planning/meta-manager/QUEUE.md",
        ] {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} is not the launcher's drivers directory"
            );
        }
    }

    #[test]
    fn classify_change_returns_planning_for_a_path_outside_the_project() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            "/home/user/projects/other/.planning/meta-manager/runs/RID/journal.jsonl",
            "/etc/passwd",
            "/home/user/projects/myapp-sibling/.planning/STATE.md",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, Path::new(case)),
                ChangeKind::Planning,
                "{case} is outside the project and must classify as planning"
            );
        }
    }

    #[test]
    fn argv_digest_is_stable_and_separator_safe() {
        let a = vec!["claude".to_string(), "-p".to_string(), "hello".to_string()];
        assert_eq!(argv_digest(&a), argv_digest(&a));
        assert!(argv_digest(&a).starts_with("fnv1a64:"));
        assert_eq!(argv_digest(&a).len(), "fnv1a64:".len() + 16);

        // The unit separator means an argv split cannot collide with an argv
        // that merely contains the joined text.
        let b = vec!["claude".to_string(), "-p hello".to_string()];
        assert_ne!(argv_digest(&a), argv_digest(&b));
    }

    // ========================================================================
    // The approval binding: the plan AND the bytes, together
    // ========================================================================

    // -----------------------------------------------------------------------
    // The approval TOKEN: one copy-pasteable value carrying both halves
    // -----------------------------------------------------------------------

    #[test]
    fn sha256_digest_is_prefixed_and_is_sixty_four_lowercase_hex_digits() {
        // A known vector, so this fails if the function ever stops being
        // SHA-256 rather than merely stops being stable against itself.
        let digest = sha256_digest(b"abc");
        assert_eq!(
            digest, "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "the published SHA-256 vector for `abc`; comparing the function with \
             itself would not detect a swap to a different hash"
        );

        let hex = digest
            .strip_prefix("sha256:")
            .expect("the prefix is what keeps a legacy value from comparing equal");
        assert_eq!(hex.len(), 64, "SHA-256 renders as 64 hex digits");
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "lowercase hex only, so two records of the same bytes compare as \
             strings without normalising. Got: {hex}"
        );
    }

    #[test]
    fn two_different_inputs_produce_different_sha256_digests() {
        assert_ne!(sha256_digest(b"one"), sha256_digest(b"two"));
        // A single flipped byte must move the digest — the whole point of the
        // upgrade is that a crafted near-identical file does not slip through.
        assert_ne!(
            sha256_digest(b"CLAUDE.md v1"),
            sha256_digest(b"CLAUDE.md v2")
        );
    }

    #[test]
    fn the_two_digest_functions_cannot_be_confused_for_the_same_input() {
        // Same logical input through both functions. If a caller ever reaches
        // for the wrong one, the prefix is what makes it visible on disk rather
        // than a silent downgrade to a non-security hash.
        let argv = vec!["abc".to_string()];
        let weak = argv_digest(&argv);
        let strong = sha256_digest(b"abc");

        assert_ne!(weak, strong);
        assert!(weak.starts_with("fnv1a64:"));
        assert!(strong.starts_with("sha256:"));
        assert!(
            !weak.starts_with("sha256:"),
            "a value that is not collision resistant must never wear the prefix \
             the re-confirmation gate treats as trustworthy"
        );
    }

    #[test]
    fn only_a_single_plain_component_is_accepted_as_a_run_id() {
        // WR-02's write side, at the predicate. Every rejected case below is a
        // shape the reproduction in `17-REVIEW.md` reached or a near neighbour
        // of it.
        assert!(is_plain_path_component("2026-07-28T14-03-11Z-a3f9"));
        assert!(is_plain_path_component("RID"));

        // **Both directions, and the acceptances come first on purpose.** The
        // predicate was tightened twice in this commit; a tightening that
        // regressed a legitimate value would be a worse defect than the two it
        // closes, so every shape the tree actually passes through this seam —
        // a bare phase number, a decimal phase, a timestamped run id, a
        // registered alias — is pinned as accepted.
        for legitimate in ["20", "2.1", "2026-08-19T12-00-00Z-aaaa", "demo", "99"] {
            assert!(
                is_plain_path_component(legitimate),
                "{legitimate:?} is a shape the tree passes through this seam and \
                 must keep passing"
            );
        }

        // **The blank half consumes the SHARED const, and the structural half
        // stays literal.** Pass 5 found this pin carrying a hand-copied
        // `["   ", "\t", "\n  \n"]` — three of the six blank shapes defined in
        // the very commit that defined six — so the two zero-width shapes were
        // never asserted at the one seam where they were reachable end to end,
        // and `--run-id '\u{200b}'` drove a complete run. Every blank-shape pin
        // in the tree now reads `test_support::DEGENERATE`, so a NEW shape lands
        // here without anybody remembering to copy it — which is how D-19-4's
        // four outside-the-old-ranges payloads arrived at this seam. The
        // traversal,
        // separator and embedded-control hostiles below are NOT blank shapes and
        // stay written out.
        let structural_hostiles = [
            ".",
            "..",
            "../escaped",
            "../../../../escaped",
            "a/b",
            "/etc/passwd",
            "/",
            "./escaped",
            "escaped/",
            "a/../b",
            // A control character embedded in an otherwise plain component.
            // `"a\nEVIL"` reached the dry-run render verbatim, breaking the line
            // the preview appears to be. It carries plenty of VISIBLE content,
            // so it is refused by the control-character half rather than by the
            // blankness half — which is why that half is composed with the
            // shared predicate rather than replaced by it.
            "a\nEVIL",
        ];
        for hostile in crate::test_support::DEGENERATE
            .iter()
            .chain(structural_hostiles.iter())
            .copied()
        {
            assert!(
                !is_plain_path_component(hostile),
                "{hostile:?} must not be accepted as a run id"
            );
        }
    }

    /// Two identities that render identically must not both resolve.
    ///
    /// **The tracer for pass-6 gap 1.** Pass 5 wrote the sentence "two visually
    /// identical aliases resolve to two different envelope and credential
    /// paths", 21-15 truth 5 claimed to close it, and the fix structurally could
    /// not: [`crate::text::carries_visible_content`] asks "is anything visible?",
    /// which refuses `"\u{200b}"` and accepts `"demo\u{200b}"`. Every payload in
    /// `DEGENERATE` is wholly invisible, so no fixture in the tree could express
    /// the harm — hence [`crate::test_support::LOOK_ALIKE_PAIRS`], consumed here
    /// rather than hand-spelled.
    ///
    /// The four seams asserted are the four a look-alike reached at pass 6: the
    /// predicate itself, the run directory, the envelope root, and (through the
    /// envelope root) the credential scope.
    ///
    /// **Red arm, observed verbatim against the unfixed tree** (this commit;
    /// the `#[ignore]` comes off in the fix commit, so the committed tree stays
    /// green while the red evidence is in history):
    ///
    /// ```text
    /// running 1 test
    /// test journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin ... FAILED
    ///
    /// ---- journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin stdout ----
    ///
    /// thread 'journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin' (123442) panicked at src/journal/mod.rs:2801:13:
    /// "demo\u{200b}" renders exactly as "demo" and carries different bytes; accepting both is how two identities that no reader can distinguish both resolve
    ///
    /// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1031 filtered out; finished in 0.00s
    /// ```
    #[test]
    fn a_look_alike_identity_never_resolves_beside_its_visible_twin() {
        let envelope_root = Path::new("/data/envelope");

        for (visible, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            assert!(
                is_plain_path_component(visible),
                "{visible:?} is the visible member of a look-alike pair and must \
                 keep being accepted — a refusal that swallowed both members \
                 would close the harm by making the tool unusable"
            );
            assert!(
                !is_plain_path_component(look_alike),
                "{look_alike:?} renders exactly as {visible:?} and carries \
                 different bytes; accepting both is how two identities that no \
                 reader can distinguish both resolve"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, look_alike).is_none(),
                "{look_alike:?} must not name an envelope root beside \
                 {visible:?}'s — pass 6 reproduced exactly two roots rendering \
                 as one"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, visible).is_some(),
                "{visible:?} must still name its envelope root"
            );
        }
    }

    /// **The finite direction at this seam, and every value pass 7 measured
    /// ACCEPTED here** (D-19-2).
    ///
    /// The look-alike sweep above refuses by deny-list, and pass 7 measured what
    /// that costs: with the class spelled as three literal ranges, this
    /// predicate returned `true` for `"demo\u{202e}"`, `"2\u{202e}0"`,
    /// `"2\u{e0041}0"` and `"2\u{ad}0"`, and `run_paths` handed back a real
    /// directory for each. Every one of those is outside the ranges and inside
    /// the class the doc claimed. The alphabet ends that regress: the accepted
    /// set is finite, so there is no "next code point" for a later pass to find.
    ///
    /// The phase-token case is the sharpest. `"2\u{202e}0"` was safe only
    /// because no roadmap happens to declare it — a fact about roadmap
    /// *contents* that the next roadmap could falsify. Here it is a property of
    /// the value (21-17 truth 6, SAFE-08).
    #[test]
    fn an_identity_outside_the_alphabet_names_nothing_at_any_seam() {
        let envelope_root = Path::new("/data/envelope");

        for outside in [
            "demo\u{202e}",
            "2\u{202e}0",
            "2\u{e0041}0",
            "2\u{ad}0",
            "\u{434}\u{435}\u{43c}\u{43e}", // Cyrillic — no invisible byte at all
            "d\u{e9}mo",                    // visible accented Latin
        ] {
            assert!(
                !is_plain_path_component(outside),
                "{outside:?} is outside the identity alphabet and must not name \
                 a run directory, envelope root, credential scope or phase token"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, outside).is_none(),
                "{outside:?} must not name an envelope root"
            );
        }

        // The accepting direction, unchanged. Every value the suite pins as a
        // legitimate identity is drawn from the alphabet already, which is why
        // the inversion costs the tool nothing it was using.
        for accepted in [
            "20",
            "2.1",
            "2026-08-19T12-00-00Z-aaaa",
            "demo",
            "99",
            "RID",
        ] {
            assert!(
                is_plain_path_component(accepted),
                "{accepted:?} is a pinned legitimate identity and must keep \
                 being accepted — a narrowing that swallowed these would close \
                 the harm by making the tool unusable"
            );
        }
    }
}
