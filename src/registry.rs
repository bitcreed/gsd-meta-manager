use crate::config::{Config, DriverOptIn, PromptInput, RegisteredProject};
use crate::session_detector::ClaudeSession;
use anyhow::{bail, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A string that has been judged fit to be a registry key.
///
/// **Registration is closed at the ENTRY, not at the seam** (D-17-2). Before
/// this type existed, `add_project` carried its own `is_empty` +
/// `contains(char::is_whitespace)` pair — a **fourth** spelling of "is this a
/// name?", beside `text::carries_visible_content`,
/// `journal::is_plain_path_component` and `driver::payload::NonBlank`. Pass 6
/// measured what that cost: `add_project("demo")` and `add_project("demo\u{200b}")`
/// both returned `Ok`, so two aliases that render identically named two
/// different projects, two opt-ins and two envelope roots.
///
/// The private field plus a single fallible constructor makes an unjudged alias
/// **unrepresentable** at the registration functions' signatures, the same move
/// [`crate::driver::payload::NonBlank`] makes at the argv boundary. The
/// constructor judges nothing itself: every clause delegates, so there is no
/// fourth spelling left to drift.
///
/// **The invariant, in one sentence: an `Alias` always satisfies
/// [`crate::journal::is_plain_path_component`]**, so every alias this build
/// admits into `config.json` can name its own envelope root. The final clause
/// is what carries that invariant; the clauses above it exist to give the
/// likely refusals their own honest message. As of D-19-2 the set is also
/// bounded from the other direction — an `Alias` is drawn entirely from
/// [`crate::text::is_identity_char`]'s finite alphabet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alias(String);

/// Why a candidate alias is not one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasRefusal {
    /// Nothing in it can be seen.
    NotVisible,
    /// It carries a character that renders as nothing, so it is
    /// indistinguishable on screen from an alias that does not.
    InvisibleFormatting {
        /// The candidate, verbatim.
        alias: String,
    },
    /// It carries whitespace.
    Whitespace,
    /// It carries a character outside the identity alphabet `[A-Za-z0-9._-]`.
    ///
    /// **The only refusal in this enum that owes the user a product decision
    /// rather than a defect report** (D-19-2). The others describe values that
    /// were never legitimate; this one describes a value that an older build
    /// accepted and this one will not, so its message carries the trade and the
    /// route rather than only the rule.
    OutsideIdentityAlphabet {
        /// The candidate, verbatim.
        alias: String,
    },
    /// It could not name a directory of its own.
    NotPlainComponent {
        /// The candidate, verbatim.
        alias: String,
    },
}

/// **The ONE producer, so no consumer has to be named** (D-21-3, CR-01).
///
/// Every variant that embeds the refused candidate routes it through
/// [`crate::text::display_identity`] first. That closes `src/main.rs:91` and
/// `src/main.rs:359` — and every refusal echo added after this one — without
/// this file, or that one, naming any of them. One producer beats three
/// consumers, and beats a list of three consumers that will be four next round.
///
/// # The measurement that made this different from what the plan predicted
///
/// The plan expected these echoes to be emitting RAW invisible bytes. Measured
/// with a scratch program against this toolchain, they were not:
///
/// ```text
/// U+202E Cf                                  -> "demo\u{202e}"   survivors: []
/// U+00AD Cf                                  -> "demo\u{ad}"     survivors: []
/// U+E0041 Cf (tag)                           -> "demo\u{e0041}"  survivors: []
/// U+180E Cf                                  -> "demo\u{180e}"   survivors: []
/// U+FE0F Mn + Default_Ignorable              -> "demo\u{fe0f}"   survivors: []
/// U+034F Mn + Default_Ignorable              -> "demo\u{34f}"    survivors: []
/// U+E0100 Mn + Default_Ignorable (VS17)      -> "demo\u{e0100}"  survivors: []
/// ```
///
/// `{alias:?}` — `str`'s `Debug` — already escaped every member of the class it
/// was handed. So the harm here was never "an invisible character reaches the
/// terminal through a refusal". It was subtler and is worth naming, because it
/// is this phase's own recurring shape one level down: **the guarantee rested on
/// `core::char::is_printable`, a standard-library table that no test in this
/// tree pins, no doc in this tree names, that can move with a toolchain
/// upgrade, and that is a SECOND spelling of a class this project already
/// derives for itself in [`crate::text`].** Two spellings of one judgment is
/// exactly the defect D-19-2 removed from `Alias::new`.
///
/// Escaping here replaces that accident with the project's own derived class, in
/// the project's own notation (`U+202E`, not `\u{202e}`), and `{:?}` then adds
/// nothing because the escaped form is ASCII. What a reader gets is one
/// notation, produced by one predicate, certified by one test.
impl std::fmt::Display for AliasRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Bound once, above the match, so a variant added tomorrow cannot embed
        // the candidate without going through it.
        let escaped = |alias: &String| crate::text::display_identity(alias);
        match self {
            Self::NotVisible => write!(
                f,
                "an alias must carry a visible name — this one is made entirely \
                 of whitespace, control or zero-width characters, so nothing in \
                 the project list would identify it"
            ),
            Self::InvisibleFormatting { alias } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the alias {alias:?} carries a character that renders as \
                     nothing, so on screen it is indistinguishable from an alias \
                     that does not. Two aliases that render identically would name \
                     two different projects, two separate driver opt-ins and two \
                     separate envelope roots — and nothing in the interface would \
                     show you which one you were acting on. Choose an alias whose \
                     written form is what you see"
                )
            }
            Self::Whitespace => write!(f, "an alias may not contain whitespace"),
            Self::OutsideIdentityAlphabet { alias } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the alias {alias:?} uses characters outside A-Z a-z 0-9 . _ - \
                     An alias is how this tool names a project to you and to \
                     itself, so the set it accepts is deliberately small and \
                     finite: outside it, two aliases can render identically while \
                     naming different projects — through a bidi override, a tag \
                     character, a variation selector or a look-alike letter from \
                     another script — and nothing in the interface would show you \
                     which one you were acting on. The project folder itself may be \
                     named anything, in any script; only the alias is restricted. \
                     Register it under an ASCII alias of your choosing. If this \
                     alias is an existing entry an older build accepted, \
                     `remove {alias:?}` still accepts it — remove it and re-add \
                     under an alias from this set"
                )
            }
            Self::NotPlainComponent { alias } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the alias {alias:?} is not a single plain directory name, so it \
                     could never name its own envelope root; it may not contain a \
                     path separator, `..`, a leading `/`, or an embedded control \
                     character"
                )
            }
        }
    }
}

/// A registry key an OLDER build accepted, on its way to being removed.
///
/// **This type judges NOTHING, and that is its entire content** (D-17-3). It
/// must accept exactly what an older build registered — invisible bytes,
/// look-alike scripts, anything — because removal is the documented recovery
/// route for precisely those entries and a removal that could not name them
/// would make a bad entry permanent. [`Alias`] is the type for a value being
/// CREATED; this is the type for a value being LOOKED UP and thrown away.
///
/// **What it buys is that ACCEPTING and ECHOING become two different questions
/// the compiler asks separately** (21-21, T-21-21-03). Before it, the `Remove`
/// arm bound a bare `String` and `println!("Removed project '{}'", alias)`
/// compiled without anyone deciding anything; a bidi spoof through that line was
/// measured by verification pass 8. The type has:
///
/// * **no `Display`** — so it cannot be interpolated at all,
/// * **no `Into<Cow<str>>`, no `Deref`, no `AsRef<str>`** — so it cannot be
///   coerced into one either,
/// * exactly two accessors, each named after the question it answers:
///   [`as_raw_for_lookup_only`](Self::as_raw_for_lookup_only) and
///   [`escaped_for_display`](Self::escaped_for_display).
///
/// The raw accessor is deliberately unattractive to type. Reaching for it is a
/// choice a reviewer can see in a diff, which is what a bare `String` never was.
#[derive(Debug)]
pub struct LegacyRegistryKey(String);

impl LegacyRegistryKey {
    /// Wrap an argv string. **No judgment is applied and none may be added** —
    /// see the type's own doc for why (D-17-3).
    pub fn from_argv(raw: String) -> Self {
        Self(raw)
    }

    /// The raw bytes, for the membership check and the removal ONLY.
    ///
    /// `config.projects` is keyed by exactly these bytes. An escaped key would
    /// miss every legacy entry, which is the regression this accessor's name
    /// exists to make visible at the call site.
    pub fn as_raw_for_lookup_only(&self) -> &str {
        &self.0
    }

    /// The form a human READS, with every invisible-class character replaced by
    /// its visible `U+XXXX` spelling.
    pub fn escaped_for_display(&self) -> String {
        crate::text::display_identity(&self.0)
    }
}

impl Alias {
    /// Judge a candidate alias, delegating every clause.
    ///
    /// **This function judges nothing itself, on purpose.** Four independent
    /// spellings of "is this a name?" is how `add_project` came to accept a
    /// value the envelope seam refuses. The order is chosen so the most
    /// specific true statement is the one the user reads.
    pub fn new(raw: &str) -> Result<Self, AliasRefusal> {
        // 1. Emptiness, by the one production spelling of it.
        if !crate::text::carries_visible_content(raw) {
            return Err(AliasRefusal::NotVisible);
        }
        // 2. Identity — a DIFFERENT question from 1, and the one that lost.
        //    `"demo\u{200b}"` passes clause 1 by construction.
        if crate::text::carries_invisible_formatting(raw) {
            return Err(AliasRefusal::InvisibleFormatting {
                alias: raw.to_string(),
            });
        }
        // 3. The one rule kept from the deleted predicate, and it is kept
        //    because it is STRICTER than the path-component question below:
        //    `"my app"` is a perfectly good directory name and a bad alias to
        //    type at a shell. This is a usability rule, not a blankness
        //    judgment — clause 1 is the blankness judgment.
        if raw.contains(char::is_whitespace) {
            return Err(AliasRefusal::Whitespace);
        }
        // 4. The identity ALPHABET — the finite direction (D-19-2). It sits
        //    above the path-component clause so a non-ASCII alias reads the
        //    message that carries the product trade and the recovery route,
        //    rather than "not a single plain directory name", which would be
        //    true and unhelpful. Like every other clause here it judges nothing
        //    itself: `text::is_identity_char` is the one spelling.
        if !raw.chars().all(crate::text::is_identity_char) {
            return Err(AliasRefusal::OutsideIdentityAlphabet {
                alias: raw.to_string(),
            });
        }
        // 5. The clause that carries the invariant: separators, `..`,
        //    traversal tokens, embedded control characters. An alias that
        //    cannot name an envelope root must not become a registry key —
        //    that mismatch is WR-06's falsehood generator.
        if !crate::journal::is_plain_path_component(raw) {
            return Err(AliasRefusal::NotPlainComponent {
                alias: raw.to_string(),
            });
        }
        Ok(Self(raw.to_string()))
    }

    /// The judged alias.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `impl Display for Alias` is WITHDRAWN (D-21-2, CR-01).
//
// **What the withdrawal buys, and it is not the four sites it named.** With the
// impl in place, `format!("{alias}")` compiles anywhere and renders the raw
// bytes, so every render site is a site somebody has to REMEMBER. Without it,
// the compiler names every interpolation and each one has to be resolved
// deliberately: `as_str()` where the value is a lookup, a comparison, a path
// segment or a tracing field, and `crate::text::display_identity` where a human
// reads it. The point is not the four sites that existed — it is that the fifth,
// added tomorrow, is a compile error rather than a bug nobody sees.
//
// The reach is small and saying so is the point: it does NOT catch
// `Removed project '{}'` in `main.rs`, whose arm binds a raw argv `String` and
// never an `Alias` (that is `LegacyRegistryKey`'s job), and it does not catch
// the TUI, where no identity type reaches at all (that is the `Screen` census's
// job). Three mechanisms, three measured reaches, none doing another's work.

/// Add a project to the registry with the given alias and path.
/// Validates that:
/// - alias is unique in the config (the alias itself was judged by
///   [`Alias::new`] — this function no longer carries a predicate of its own)
/// - path exists on disk
/// - path contains a `.planning/` directory
pub fn add_project(config: &mut Config, alias: &Alias, path: &Path) -> Result<()> {
    let alias = alias.as_str();

    if config.projects.contains_key(alias) {
        bail!("Alias already exists");
    }

    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }

    let planning_dir = path.join(".planning");
    if !planning_dir.is_dir() {
        bail!("No .planning/ directory found at: {}", path.display());
    }

    let now = chrono::Utc::now().to_rfc3339();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: path.to_path_buf(),
            added: now,
            // Written out rather than defaulted: registering a project lets the
            // dashboard read it, and driving it is a separate deliberate act.
            // `auto_register_from_sessions` reaches this line, so this explicit
            // `None` is what proves discovery can never enrol a project into
            // being driven (D-14, D-15).
            //
            // Written out at the site rather than with a struct-update
            // shorthand on purpose: `..Default::default()` would absorb the next
            // field silently, and the compile error that forced this line to
            // exist is the whole mechanism.
            driver_opt_in: None,
            // A fresh entry carries no unknown fields. Also explicit, for the
            // same reason.
            extra: Default::default(),
        },
    );

    Ok(())
}

/// Add a project to the registry without checking for `.planning/` directory.
/// Used for freshly created projects that don't yet have a `.planning/` folder.
/// "Unchecked" refers to the `.planning/` directory only: the alias is judged by
/// [`Alias::new`] before it can reach this signature.
pub fn add_project_unchecked(config: &mut Config, alias: &Alias, path: &Path) -> Result<()> {
    let alias = alias.as_str();

    if config.projects.contains_key(alias) {
        bail!("Alias already exists");
    }

    let now = chrono::Utc::now().to_rfc3339();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: path.to_path_buf(),
            added: now,
            // See `add_project`: never set by any registration path (D-14).
            driver_opt_in: None,
            extra: Default::default(),
        },
    );

    Ok(())
}

/// Record the user's deliberate opt-in for `alias`.
///
/// **This is the only function outside tests that constructs a [`DriverOptIn`],
/// and that uniqueness is the point (D-14).** Because a record can come into
/// existence in exactly one place, a `Some(record)` sitting in a `config.json` is
/// proof of a deliberate user action rather than a bit that could have been
/// flipped from anywhere. `tests/spawn_seam_guard.rs`-style greps can check
/// "constructed once"; they cannot check "every branch remembered to ask".
///
/// The timestamp is
/// `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)` — the same
/// call `JournalRun::finish` makes, deliberately not this module's older bare
/// `to_rfc3339()`, so an opt-in stamp and a run's `ended_at` compare directly
/// without normalising.
///
/// **Does not persist.** The caller calls `save_config`, consistently with every
/// other function in this module.
pub fn record_opt_in(config: &mut Config, alias: &str) -> Result<()> {
    let Some(entry) = config.projects.get_mut(alias) else {
        bail!("Project not found: {}", alias);
    };

    let prompt_inputs = current_prompt_inputs(&entry.path);
    entry.driver_opt_in = Some(DriverOptIn {
        opted_in_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        // Deliberately `None`: this build does not write the legacy key. It is
        // kept on the struct so an older binary that loads and re-saves this
        // config still finds the field it expects, which is what makes the
        // upgrade migration-free. `prompt_inputs` is where the real digest
        // lives, and it covers `CLAUDE.md` along with everything else that
        // reaches a prompt.
        claude_md_digest: None,
        // Named explicitly rather than absorbed through `..Default::default()`,
        // for the reason the comment below records: the next field added must
        // break this line.
        prompt_inputs,
        // A fresh record carries no fields this build does not model.
        extra: Default::default(),
        // Every envelope field is named explicitly as `None` rather than
        // reached for through `..Default::default()`, for the reason
        // `config.rs:41-45` records about the opt-in field itself: the next
        // field added must break this line, not be silently absorbed by it.
        // `None` here means "the envelope's own defaults apply", resolved in
        // the one place they are decided — `EnvelopePolicy::resolve` (D-30).
        branch_namespace: None,
        credential: None,
        pr_cap_per_24h: None,
        pr_cap_per_run: None,
    });

    Ok(())
}

/// Which spawn profile reads a disclosed file.
///
/// A **disclosure label**, deliberately not [`crate::executor::SpawnProfile`]:
/// that type carries a JSON schema and exists to shape an argv, while this one
/// exists to tell a user which of the two spawns will read a file. Keeping them
/// separate stops the disclosure surface from acquiring a dependency on the
/// seam's payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptProfile {
    /// The bounded model seam: an empty tool set and a suppressed `CLAUDE.md`.
    /// Only the enumerated third-party *strings* from these files reach it, and
    /// only inside the labelled untrusted boundary — never the whole file.
    ModelSeam,
    /// The spawn that actually runs a GSD command. It loads the target
    /// repository's `CLAUDE.md` itself, and this phase does **not** change that.
    Executor,
}

/// Every file whose bytes can reach a model prompt, and which profile reads it.
///
/// **This list is the disclosure, and its membership is a correctness property.**
/// The plan's own prohibition rates an under-broad list as exactly as dishonest
/// as an over-broad one, and under-broad is the more dangerous direction: an
/// over-broad list annoys, an under-broad one misleads a user about what reaches
/// the model.
///
/// `HANDOFF` is here in **both** its spellings because `state_reader` reads
/// either (`HANDOFF.json` first, then `HANDOFF.md`), and a list naming only one
/// would be under-broad by exactly one file. `ProjectState` is parsed from
/// `STATE.md` *and* the `HANDOFF` file; `RoadmapPhase` from `ROADMAP.md` — see
/// `crate::driver::untrusted`'s census, which is authoritative for that.
///
/// Absent files are still listed. A file that does not exist at opt-in is
/// recorded with a `None` digest rather than omitted, so that its later
/// appearance is a mismatch — see [`crate::config::PromptInput::digest`].
pub const DISCLOSED_PROMPT_INPUTS: &[(&str, PromptProfile)] = &[
    ("CLAUDE.md", PromptProfile::Executor),
    (".planning/STATE.md", PromptProfile::ModelSeam),
    (".planning/ROADMAP.md", PromptProfile::ModelSeam),
    (".planning/HANDOFF.json", PromptProfile::ModelSeam),
    (".planning/HANDOFF.md", PromptProfile::ModelSeam),
];

/// A SHA-256 digest of one file under `project_root`, or `None` if it is absent
/// or unreadable.
///
/// Absent and unreadable are ordinary states for a project, not failures — and
/// they are **not** silently equivalent to "unchanged": `None` is recorded and
/// compared like any other value, so a file appearing or vanishing after opt-in
/// is drift.
fn file_digest(project_root: &Path, relative: &str) -> Option<String> {
    let bytes = std::fs::read(project_root.join(relative)).ok()?;
    Some(crate::journal::sha256_digest(&bytes))
}

/// The disclosed file set with each file's digest as it stands right now.
///
/// Reads bytes rather than a `String` so a file that is not valid UTF-8 still
/// digests instead of silently reading as absent — "unreadable" should mean the
/// filesystem refused, not that the content surprised us.
pub fn current_prompt_inputs(project_root: &Path) -> Vec<PromptInput> {
    DISCLOSED_PROMPT_INPUTS
        .iter()
        .map(|(relative, _profile)| PromptInput {
            path: (*relative).to_string(),
            digest: file_digest(project_root, relative),
            extra: Default::default(),
        })
        .collect()
}

/// Why a recorded opt-in no longer covers what would reach a prompt.
///
/// Every variant means the same thing to the gate — **re-confirm** — and the
/// variants exist only so the reason can be shown to the user. A gate with one
/// answer for every unclear state has no unclear state left to get wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptInDrift {
    /// The record predates the disclosure entirely (an empty `prompt_inputs`).
    /// A record written before the field existed never showed the user a list,
    /// so it cannot stand as an informed approval of what reaches a prompt.
    NothingDisclosed,
    /// A recorded digest does not carry the `sha256:` prefix.
    ///
    /// Matched on "is not `sha256:`" rather than on "is `fnv1a64:`", which is
    /// load-bearing: the pre-Phase-19 fixture in `config.rs` carries `fnv1a:`,
    /// so an exact legacy-prefix match would sail past a real legacy value
    /// already present in this tree.
    LegacyDigest { path: String, digest: String },
    /// A file this build would read has no entry in the record at all — the
    /// user never saw it, so they never approved it.
    Undisclosed { path: String },
    /// The bytes changed, appeared, or vanished since opt-in.
    Changed { path: String },
}

impl OptInDrift {
    /// A single line naming the file and what happened, for the surface that
    /// tells the user why they are being asked again.
    pub fn describe(&self) -> String {
        match self {
            OptInDrift::NothingDisclosed => "this opt-in predates the prompt-input disclosure, \
                 so it never listed which files' bytes reach a prompt"
                .to_string(),
            OptInDrift::LegacyDigest { path, .. } => format!(
                "`{path}` was recorded with a legacy digest that is not a security control, \
                 so it cannot be compared across hash families"
            ),
            OptInDrift::Undisclosed { path } => format!(
                "`{path}` can reach a prompt but was not disclosed when this opt-in was recorded"
            ),
            OptInDrift::Changed { path } => {
                format!("`{path}` no longer matches the bytes recorded at opt-in")
            }
        }
    }
}

/// Whether `opt_in` still covers the bytes that would reach a prompt right now.
///
/// `None` means the recorded approval is still good. `Some(drift)` means
/// re-confirm.
///
/// **Called at the spawn gate, not at render time.** A confirmation screen left
/// open while a `git pull` rewrites `CLAUDE.md` must not be able to approve
/// bytes that changed underneath it, so the question is asked where the opt-in
/// is consulted before a spawn — `crate::executor::DrivableProject::from_registry`.
pub fn check_prompt_input_drift(project_root: &Path, opt_in: &DriverOptIn) -> Option<OptInDrift> {
    if opt_in.prompt_inputs.is_empty() {
        return Some(OptInDrift::NothingDisclosed);
    }

    // A legacy digest anywhere in the record is checked before any comparison,
    // so a value from a hash family this binary does not compute can never
    // reach the equality test and read as "matches".
    for recorded in &opt_in.prompt_inputs {
        if let Some(digest) = &recorded.digest {
            if !digest.starts_with("sha256:") {
                return Some(OptInDrift::LegacyDigest {
                    path: recorded.path.clone(),
                    digest: digest.clone(),
                });
            }
        }
    }

    for (relative, _profile) in DISCLOSED_PROMPT_INPUTS {
        let Some(recorded) = opt_in
            .prompt_inputs
            .iter()
            .find(|entry| entry.path == *relative)
        else {
            return Some(OptInDrift::Undisclosed {
                path: (*relative).to_string(),
            });
        };

        if recorded.digest != file_digest(project_root, relative) {
            return Some(OptInDrift::Changed {
                path: (*relative).to_string(),
            });
        }
    }

    None
}

/// Withdraw the opt-in for `alias`, removing the record entirely.
///
/// Setting the field back to `None` rather than storing a "revoked" marker: the
/// gate asks one question — is there a record? — and a second representation of
/// "no" is a second thing that can be got wrong. Like [`record_opt_in`], this
/// does not persist.
pub fn clear_opt_in(config: &mut Config, alias: &str) -> Result<()> {
    let Some(entry) = config.projects.get_mut(alias) else {
        bail!("Project not found: {}", alias);
    };
    entry.driver_opt_in = None;
    Ok(())
}

/// Whether `alias` carries an opt-in record.
///
/// A cheap read for the UI (plan 17-07). **Not a gate**: the gate is
/// `DrivableProject::from_registry`, which lives in the driver process so a
/// hand-typed `drive` is refused by the same code as a TUI-initiated one (D-16).
/// An unregistered alias answers `false`.
pub fn is_opted_in(config: &Config, alias: &str) -> bool {
    config
        .projects
        .get(alias)
        .is_some_and(|entry| entry.driver_opt_in.is_some())
}

/// Remove a project from the registry by alias.
///
/// **It deliberately touches no UI-side map, and the omission is structural
/// rather than an oversight** (D-27): this module is free functions over a
/// `&mut Config` and has no access to `AppContext`, where the driver state lives
/// — `run_states`, `observed_runs` and `journal_cursors`. Those are cleaned in
/// exactly two places, and this sentence exists so the next reader finds them
/// instead of concluding the leak is still open:
///
/// * `ui::screens::delete_confirm::do_remove_project` — the interactive path,
///   which drops them in the same block as `project_states` and `last_refresh`.
/// * `App::prune_driver_maps` — the backstop, on the existing 20-tick block, for
///   every removal that does not go through that screen.
pub fn remove_project(config: &mut Config, alias: &str) -> Result<()> {
    if config.projects.remove(alias).is_none() {
        bail!("Project not found: {}", alias);
    }
    Ok(())
}

/// List all registered projects sorted by alias.
pub fn list_projects(config: &Config) -> Vec<(&String, &RegisteredProject)> {
    let mut projects: Vec<_> = config.projects.iter().collect();
    projects.sort_by_key(|(alias, _)| alias.to_lowercase());
    projects
}

/// Canonicalize a path, falling back to the raw path if canonicalization fails
/// (e.g. broken symlink, permission denied). Equivalent canonical paths are
/// what we use to deduplicate against the registry.
fn canon_or_raw(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// Derive an alias from a path's file name (lowercased), SANITIZED to the
/// identity alphabet. Falls back to `"project"`.
///
/// **Sanitizing rather than refusing is the right direction here, and the
/// reason is that a folder name is not an identity** (D-19-2). [`Alias::new`]
/// refuses a value outside the alphabet because the user TYPED it and can type
/// another; this function is handed whatever the filesystem happens to hold, by
/// auto-registration the user did not ask a question about. Refusing there would
/// drop a real project on the floor.
///
/// So a disallowed character maps to `'-'`, repeats collapse, and the result is
/// trimmed of leading/trailing separators. A name that sanitizes to nothing —
/// every character outside the alphabet, e.g. an all-Cyrillic folder — falls
/// back to `"project"` rather than to `""` or to a bare run of separators.
/// Without that last step `unique_alias` would grow `"-"` into `"-2"`, `"-3"`,
/// queueing every such folder under one meaningless prefix; `"project"` at least
/// says what it is and collides into `project-2` like any other duplicate.
/// Pinned by
/// `a_folder_named_in_a_non_latin_script_derives_a_usable_alias_or_refuses_with_the_hint`.
///
/// The result still passes through [`Alias::new`] at the call site — this
/// sanitizer is a convenience, not a second judgment.
fn derive_alias(path: &Path) -> String {
    let sanitized = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .map(|s| {
            let mut out = String::with_capacity(s.len());
            for c in s.chars() {
                if crate::text::is_identity_char(c) {
                    out.push(c);
                } else if !out.ends_with('-') {
                    out.push('-');
                }
            }
            out.trim_matches('-').to_string()
        })
        .filter(|s| !s.is_empty())
        .filter(|s| s.chars().any(|c| c.is_ascii_alphanumeric()));

    sanitized.unwrap_or_else(|| "project".to_string())
}

/// Pick an alias that doesn't collide with existing keys. If `base` is free,
/// returns it as-is; otherwise tries `base-2`, `base-3`, … up to `base-999`.
fn unique_alias(config: &Config, base: &str) -> String {
    if !config.projects.contains_key(base) {
        return base.to_string();
    }
    for n in 2..1000 {
        let candidate = format!("{}-{}", base, n);
        if !config.projects.contains_key(&candidate) {
            return candidate;
        }
    }
    // Pathological fallback — should never happen in practice
    format!("{}-{}", base, chrono::Utc::now().timestamp())
}

/// Scan active Claude sessions and auto-register any whose `working_dir` is a
/// GSD project (contains `.planning/`) and is not already in the registry.
///
/// Returns the list of `(alias, canonical_path)` pairs that were newly added.
/// Callers are responsible for persisting the config and starting file
/// watchers on the new entries.
///
/// Dedup is canonical-path based — paths that resolve to the same target
/// (e.g. symlinked variants, trailing slashes) register at most once per call
/// and won't double-register if already present in the config.
pub fn auto_register_from_sessions(
    config: &mut Config,
    sessions: &[ClaudeSession],
) -> Vec<(String, PathBuf)> {
    let registered: HashSet<PathBuf> = config
        .projects
        .values()
        .map(|p| canon_or_raw(&p.path))
        .collect();

    let mut seen_this_pass: HashSet<PathBuf> = HashSet::new();
    let mut added: Vec<(String, PathBuf)> = Vec::new();

    for session in sessions {
        let canonical = canon_or_raw(&session.working_dir);

        if !canonical.join(".planning").is_dir() {
            continue;
        }
        if registered.contains(&canonical) || !seen_this_pass.insert(canonical.clone()) {
            continue;
        }

        let base = derive_alias(&canonical);
        let alias = unique_alias(config, &base);

        // The derived alias goes through the same judgment as a typed one
        // (D-17-2). A discovered folder whose NAME carries format characters —
        // real for some scripts — is skipped here and registers only under an
        // alias the user chooses explicitly; auto-registration must not be the
        // one route that admits a key the envelope seam would refuse.
        let alias = match Alias::new(&alias) {
            Ok(alias) => alias,
            Err(refusal) => {
                tracing::warn!(
                    alias = %alias,
                    path = %canonical.display(),
                    error = %refusal,
                    "auto-register: alias refused",
                );
                continue;
            }
        };

        if let Err(e) = add_project(config, &alias, &canonical) {
            // RAW (`as_str`). A log line is a machine record read by grep and by
            // whoever is debugging a registration that did not happen, and it
            // has to carry the bytes that were actually used as the key. It is
            // not a terminal cell an operator reads a name off, which is the
            // question `display_identity` answers.
            tracing::warn!(
                alias = %alias.as_str(),
                path = %canonical.display(),
                error = %e,
                "auto-register: add_project failed",
            );
            continue;
        }
        let alias = alias.as_str().to_string();
        added.push((alias, canonical));
    }

    added
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn make_session(working_dir: PathBuf) -> ClaudeSession {
        ClaudeSession {
            pid: 1,
            session_id: None,
            working_dir,
            start_time: None,
            tty: None,
        }
    }

    fn empty_config() -> Config {
        Config::new()
    }

    /// A fixture alias, judged the way a real one is.
    fn visible(raw: &str) -> Alias {
        Alias::new(raw).expect("a visible test alias")
    }

    /// `display_identity` applied twice equals `display_identity` applied once.
    ///
    /// **This is what licenses removing the outer escape at `src/main.rs`'s
    /// `judged_alias_or_exit`** (D-21-3, 21-21). That site used to wrap
    /// `refusal.to_string()` in a second `display_identity` call while
    /// `Display for AliasRefusal` now escapes at the producer. Deleting the
    /// outer call is only behaviour-preserving if the function is idempotent
    /// over its own output — so that is ASSERTED here rather than assumed, over
    /// the imported hostile fixtures rather than over a literal chosen to
    /// agree with it.
    ///
    /// It also states the property that makes `shown()` safe to apply at a
    /// render site whose input may already have passed through another one.
    #[test]
    fn escaping_an_already_escaped_identity_changes_nothing() {
        use crate::test_support::{DEGENERATE, LOOK_ALIKE_PAIRS};
        use crate::text::display_identity;

        let corpus: Vec<String> = DEGENERATE
            .iter()
            .map(|s| (*s).to_string())
            .chain(
                LOOK_ALIKE_PAIRS
                    .iter()
                    .flat_map(|(clean, hostile)| [(*clean).to_string(), (*hostile).to_string()]),
            )
            .collect();

        assert!(
            !corpus.is_empty(),
            "the imported fixtures are empty, so this pin asserts nothing"
        );

        // Non-vacuity: at least one fixture must actually BE escaped, or
        // idempotence would be the trivial identity over unchanged strings.
        assert!(
            corpus.iter().any(|raw| display_identity(raw) != *raw),
            "no imported fixture is changed by display_identity, so an \
             idempotence pin over them would hold for a function that did \
             nothing at all"
        );

        for raw in &corpus {
            let once = display_identity(raw);
            let twice = display_identity(&once);
            assert_eq!(
                once, twice,
                "display_identity is not idempotent over {raw:?}: one pass gives \
                 {once:?} and a second gives {twice:?}. Removing the duplicate \
                 outer escape at main.rs's judged_alias_or_exit would then be a \
                 behaviour CHANGE rather than a de-duplication."
            );
        }
    }

    /// `..` and `.` still reach [`AliasRefusal::NotPlainComponent`].
    ///
    /// **The pin certifies more than the message** (WR-04, D-21-5). `Alias::new`
    /// places the identity-ALPHABET clause (clause 4, D-19-2) ABOVE the
    /// structural path-component clause (clause 5). `.` and `-` and `_` are all
    /// inside that alphabet, so `".."` and `"."` pass clause 4 and fall through
    /// to clause 5 — which is what makes them the ONLY two values that still
    /// reach this variant, and what makes this test the certificate that the
    /// alphabet clause did NOT subsume the structural one. That non-subsumption
    /// is 21-19 truth 3's load-bearing claim, and before this test it had zero
    /// consumers: measured with
    /// `rtk proxy grep -rn "NotPlainComponent" src/ tests/`, the
    /// `AliasRefusal` variant had exactly three occurrences — its declaration,
    /// its `Display` arm and its construction — all in this file, and none in a
    /// test.
    ///
    /// Kept and pinned rather than retired: it is the only place the traversal
    /// invariant is stated in a refusal a user can read.
    #[test]
    fn the_two_values_that_still_reach_not_plain_component_still_reach_it() {
        for raw in ["..", "."] {
            let refusal = Alias::new(raw).expect_err(
                "a value that cannot name a directory of its own must not become \
                 a registry key — that mismatch is WR-06's falsehood generator",
            );
            assert!(
                matches!(refusal, AliasRefusal::NotPlainComponent { .. }),
                "{raw:?} must be refused as NOT-A-PLAIN-COMPONENT and not by some \
                 clause above it. If this now reports OutsideIdentityAlphabet, \
                 D-19-2's alphabet clause has SUBSUMED the structural clause — \
                 and 21-19 truth 3, which claims the two are independent, is \
                 false. Got: {refusal:?}"
            );
            let message = refusal.to_string();
            assert!(
                message.contains(raw),
                "the refusal must name the value it refused, or the user is told \
                 no without being told about what. Got: {message}"
            );
        }

        // The other direction, so "reaches it" is not mistaken for "reaches it
        // for everything": a value outside the alphabet must be refused by
        // clause 4, ABOVE this one, because that message carries the product
        // trade and the recovery route.
        let outside = Alias::new("d\u{e9}mo").expect_err("a non-ASCII alias is refused");
        assert!(
            matches!(outside, AliasRefusal::OutsideIdentityAlphabet { .. }),
            "a non-ASCII alias must read the message that names the trade and the \
             `remove` recovery route, not the structural one. Got: {outside:?}"
        );
    }

    /// The one producer escapes, so no consumer has to (D-21-3, CR-01).
    ///
    /// `src/main.rs`'s two remaining refusal echoes — `Commands::Add` and
    /// `EnvelopeAction::Scan` — do nothing but `eprintln!("Error: {refusal}")`.
    /// This is what makes that safe, and it is asserted against the type rather
    /// than against those two call sites, which is the whole point: an echo
    /// added tomorrow inherits it without being named here.
    #[test]
    fn a_refusal_never_carries_an_invisible_character_into_its_own_message() {
        use crate::test_support::LOOK_ALIKE_PAIRS;

        let mut refusals_seen = 0;
        for (_, hostile) in LOOK_ALIKE_PAIRS {
            let refusal = Alias::new(hostile).expect_err("a look-alike is refused");
            refusals_seen += 1;
            let message = refusal.to_string();
            let invisible: Vec<char> = message
                .chars()
                .filter(|c| crate::text::is_invisible_formatting_char(*c))
                .collect();
            assert!(
                invisible.is_empty(),
                "the refusal for {hostile:?} carried {invisible:?} into its own \
                 message. A refusal that reports an invisible character by \
                 emitting one lets the rejected value edit the sentence \
                 explaining why it was rejected. Message: {message}"
            );
            assert!(
                message.contains("U+"),
                "the refusal for {hostile:?} must name the offending code point in \
                 THIS project's notation, produced by THIS project's derived \
                 class — not by `str`'s Debug impl, whose escaping is a \
                 standard-library table no test here pins. Message: {message}"
            );
        }
        assert!(
            refusals_seen > 0,
            "no look-alike fixture produced a refusal, so this test asserted nothing"
        );
    }

    /// Two aliases that render identically cannot both name a project.
    ///
    /// **The pass-6 CR-01 registry half, which is the genuinely new surface.**
    /// Measured against the unfixed tree: `add_project("demo")` → `Ok`,
    /// `add_project("demo\u{200b}")` → `Ok`, two entries in `config.json`, two
    /// driver opt-ins, two envelope roots, one rendering. The refusal now lands
    /// at the constructor, so the second call cannot be made at all.
    #[test]
    fn registering_a_look_alike_beside_its_visible_twin_is_refused() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();

        add_project(&mut config, &visible("demo"), dir.path()).expect("the visible twin registers");

        let refused = Alias::new("demo\u{200b}").expect_err(
            "an alias that renders exactly like `demo` must not be constructible, or a \
             second project can exist that no reader can tell from the first",
        );
        assert!(
            matches!(refused, AliasRefusal::InvisibleFormatting { .. }),
            "the refusal must name the look-alike harm rather than borrowing a \
             blankness message: got {refused:?}"
        );
        assert_eq!(
            config.projects.len(),
            1,
            "exactly one project may exist for a name that renders one way"
        );

        // Every look-alike member, and every wholly-blank shape, refused
        // through the ONE constructor — no hand-picked subset.
        for (visible_member, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            assert!(
                Alias::new(visible_member).is_ok(),
                "{visible_member:?} must remain registrable — the refusal is \
                 about the invisible bytes, not about the pair"
            );
            assert!(
                Alias::new(look_alike).is_err(),
                "{look_alike:?} renders exactly as {visible_member:?} and must \
                 not become a registry key"
            );
        }
        for blank in crate::test_support::DEGENERATE {
            assert!(
                Alias::new(blank).is_err(),
                "{blank:?} carries no visible name and must not become a \
                 registry key"
            );
        }

        // The accepting direction, so the constructor is not a predicate that
        // refuses everything.
        for legitimate in ["demo", "myproj-2", "a.b"] {
            assert!(
                Alias::new(legitimate).is_ok(),
                "{legitimate:?} is a shape the tool registers and must keep \
                 being registrable"
            );
        }
    }

    /// **The finite direction at the registration seam** (D-19-2).
    ///
    /// The look-alike sweep above refuses by DENY-list, and this phase spent six
    /// rounds proving a deny-list can always be one code point short: pass 7
    /// registered nine keys all rendering as `demo`. This pins the other
    /// direction — a value outside `[A-Za-z0-9._-]` is refused because it is not
    /// in the alphabet, not because someone remembered its code point.
    ///
    /// The Cyrillic case is the one the deny-list could NEVER have caught: it
    /// carries nothing invisible at all. `"\u{434}\u{435}\u{43c}\u{43e}"` renders
    /// as `демо`, which is not `demo` — but `carries_invisible_formatting` is
    /// explicitly not a homoglyph defence (TR39 is carved out and stays carved
    /// out), and the alphabet closes the identity half of that harm for free by
    /// admitting no non-ASCII letter at all.
    #[test]
    fn an_alias_outside_the_identity_alphabet_is_refused_with_the_product_trade() {
        for outside in [
            "\u{434}\u{435}\u{43c}\u{43e}", // Cyrillic
            "demo\u{202e}",                 // bidi override
            "demo\u{e0041}",                // tag character
            "d\u{e9}mo",                    // Latin-1 accented — visible, and still not an identity
            "\u{65e5}\u{672c}",             // CJK
        ] {
            let refused = Alias::new(outside)
                .expect_err("a value outside the identity alphabet is not an alias");
            assert!(
                matches!(
                    refused,
                    AliasRefusal::OutsideIdentityAlphabet { .. }
                        | AliasRefusal::InvisibleFormatting { .. }
                ),
                "{outside:?} must be refused by the alphabet (or by the earlier, \
                 more specific invisible-formatting clause), not by a generic \
                 path-component message: got {refused:?}"
            );
        }

        // The message carries the product trade and the recovery route — this is
        // the one refusal in the enum that reports a CHOSEN narrowing rather
        // than a value that was never legitimate, so the user is owed both.
        let refused = Alias::new("\u{434}\u{435}\u{43c}\u{43e}").unwrap_err();
        let message = refused.to_string();
        for owed in ["A-Z a-z 0-9 . _ -", "may be named anything", "remove"] {
            assert!(
                message.contains(owed),
                "the refusal message must carry {owed:?} — it reports a trade \
                 this build chose, so it owes the user the rule, the fact that \
                 the FOLDER is unrestricted, and the recovery route. Got: \
                 {message}"
            );
        }

        // The accepting direction, so the alphabet is not a predicate that
        // refuses everything.
        for legitimate in ["demo", "myproj-2", "a.b", "A_1", "20", "RID"] {
            assert!(
                Alias::new(legitimate).is_ok(),
                "{legitimate:?} is drawn entirely from the alphabet and must \
                 remain registrable"
            );
        }
    }

    /// **`derive_alias`'s residual, pinned rather than left to the sanitizer's
    /// ordering.**
    ///
    /// Mapping disallowed characters INTO the alphabet is the right direction
    /// for a DERIVED alias — a folder name is not an identity, and refusing
    /// would drop a real project on the floor during auto-registration. But an
    /// all-non-ASCII folder name has a failure the refusal path does not: it
    /// sanitizes to a run of separators, so `"-"` registers as a legal-but-
    /// meaningless alias and [`unique_alias`] then grows it into `"-2"`, `"-3"`,
    /// queueing every such folder under one prefix. `""` would fall through the
    /// emptiness filter instead.
    ///
    /// What this forbids is that silent third outcome. Either branch of the
    /// contract is acceptable and the pin names whichever the sanitizer's
    /// ordering produces.
    ///
    /// It lives in this module rather than in `tests/registry_test.rs` because
    /// `derive_alias` is private.
    #[test]
    fn a_folder_named_in_a_non_latin_script_derives_a_usable_alias_or_refuses_with_the_hint() {
        for folder in [
            "\u{43f}\u{440}\u{43e}\u{435}\u{43a}\u{442}", // проект
            "\u{65e5}\u{672c}\u{8a9e}",                   // 日本語
            "\u{645}\u{634}\u{631}\u{648}\u{639}",        // مشروع
        ] {
            let derived = derive_alias(Path::new("/tmp").join(folder).as_path());

            assert!(
                !derived.is_empty() && derived != "-" && derived.chars().any(|c| c != '-'),
                "a folder named {folder:?} must not derive the empty string, \
                 {:?}, or an all-separator run — `unique_alias` would then queue \
                 every such folder under one meaningless prefix. Got: {derived:?}",
                "-"
            );

            let usable = derived.chars().all(crate::text::is_identity_char)
                && derived.chars().any(|c| c.is_ascii_alphanumeric());
            if usable {
                assert!(
                    Alias::new(&derived).is_ok(),
                    "a derived value inside the alphabet must construct: \
                     {derived:?}"
                );
            } else {
                let refused = Alias::new(&derived).expect_err(
                    "if the sanitizer did not produce a usable alias, the \
                     downstream judgment must refuse it rather than admit a key \
                     the envelope seam would reject",
                );
                assert!(
                    refused.to_string().contains("Register it under an ASCII alias"),
                    "and the refusal must name the explicit-alias route: \
                     {refused}"
                );
            }
        }

        // The ASCII path is unchanged, including the separator mapping that
        // used to leave a space in and get refused as `Whitespace` downstream.
        assert_eq!(derive_alias(Path::new("/tmp/MyApp")), "myapp");
        assert_eq!(derive_alias(Path::new("/tmp/my app")), "my-app");
    }

    /// The invariant, asserted rather than documented: an `Alias` can always
    /// name its own envelope root. A registration that admitted a value the
    /// envelope seam refuses is WR-06's falsehood generator.
    #[test]
    fn every_constructible_alias_can_name_its_own_envelope_root() {
        for legitimate in ["demo", "myproj-2", "a.b", "20", "RID"] {
            let alias = Alias::new(legitimate).expect("a registrable alias");
            assert!(
                crate::envelope::envelope_dir_in(Path::new("/data/envelope"), alias.as_str())
                    .is_some(),
                "{legitimate:?} registers, so it must be able to name its \
                 envelope root — registration must never be looser than the seam"
            );
        }
    }

    #[test]
    fn auto_register_adds_new_gsd_project() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(config.projects.len(), 1);
        let alias = &added[0].0;
        assert!(config.projects.contains_key(alias));
    }

    #[test]
    fn auto_register_skips_existing_canonical_path() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let mut config = empty_config();
        add_project(&mut config, &visible("existing"), &canonical).unwrap();
        let sessions = vec![make_session(canonical.clone())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert!(added.is_empty());
        assert_eq!(config.projects.len(), 1);
    }

    #[test]
    fn auto_register_skips_non_gsd_project() {
        let dir = tempdir().unwrap();
        // Intentionally NO .planning/ directory
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert!(added.is_empty());
        assert!(config.projects.is_empty());
    }

    #[test]
    fn auto_register_dedupes_within_single_pass() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![
            make_session(dir.path().to_path_buf()),
            make_session(dir.path().to_path_buf()),
        ];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(config.projects.len(), 1);
    }

    #[test]
    fn auto_register_uses_suffix_on_alias_collision() {
        // Two different directories sharing the same basename.
        let parent_a = tempdir().unwrap();
        let parent_b = tempdir().unwrap();
        std::fs::create_dir_all(parent_a.path().join("myproj/.planning")).unwrap();
        std::fs::create_dir_all(parent_b.path().join("myproj/.planning")).unwrap();
        let path_a = parent_a.path().join("myproj");
        let path_b = parent_b.path().join("myproj");

        let mut config = empty_config();
        // Pre-register the first one under its natural alias.
        add_project(&mut config, &visible("myproj"), &path_a.canonicalize().unwrap()).unwrap();

        let sessions = vec![make_session(path_b)];
        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 1);
        assert_eq!(added[0].0, "myproj-2");
        assert!(config.projects.contains_key("myproj"));
        assert!(config.projects.contains_key("myproj-2"));
    }

    #[test]
    fn record_opt_in_stamps_a_record_with_a_second_precision_timestamp() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        std::fs::write(dir.path().join("CLAUDE.md"), "# Project\n").unwrap();
        let mut config = empty_config();
        add_project(&mut config, &visible("opted"), dir.path()).unwrap();

        assert!(
            !is_opted_in(&config, "opted"),
            "registration alone never opts a project in (D-14)"
        );

        record_opt_in(&mut config, "opted").unwrap();

        let record = config.projects["opted"]
            .driver_opt_in
            .as_ref()
            .expect("record_opt_in constructs the record");

        // Second precision, RFC3339, `Z` — the shape `JournalRun::finish` writes,
        // so an opt-in stamp and a run's `ended_at` compare without normalising.
        assert!(
            record.opted_in_at.ends_with('Z'),
            "the stamp is UTC with a Z suffix, got: {}",
            record.opted_in_at
        );
        assert!(
            !record.opted_in_at.contains('.'),
            "second precision means no fractional part, got: {}",
            record.opted_in_at
        );
        assert_eq!(
            record.opted_in_at.len(),
            20,
            "`YYYY-MM-DDTHH:MM:SSZ` is 20 characters, got: {}",
            record.opted_in_at
        );

        // **Rewritten in the commit that falsified it** (the `dry_run.rs:78-83`
        // precedent). This previously asserted `claude_md_digest` was `Some`
        // with an `fnv1a64:` prefix. Phase 21 stopped writing that key — the
        // digest moved to `prompt_inputs` and became SHA-256, because a
        // re-confirmation prompt is a security affordance and FNV-1a says in
        // its own documentation that it is not a security control (C-4).
        assert_eq!(
            record.claude_md_digest, None,
            "this build must not write the legacy key; it is kept on the struct \
             only so an older binary's load-and-save stays non-destructive"
        );

        let claude_md = record
            .prompt_inputs
            .iter()
            .find(|entry| entry.path == "CLAUDE.md")
            .expect("`CLAUDE.md` is one of the disclosed prompt inputs");
        let digest = claude_md
            .digest
            .as_ref()
            .expect("a project that has a CLAUDE.md records its digest");
        assert!(
            digest.starts_with("sha256:"),
            "the re-confirmation digest must be collision resistant, got: {digest}"
        );

        // The whole disclosed set is recorded, not just the files that exist —
        // an absent file is `None`, so its later appearance is drift.
        assert_eq!(
            record.prompt_inputs.len(),
            DISCLOSED_PROMPT_INPUTS.len(),
            "every disclosed file gets an entry, present or not"
        );

        assert!(is_opted_in(&config, "opted"));
    }

    #[test]
    fn clear_opt_in_removes_the_record_entirely() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        add_project(&mut config, &visible("opted"), dir.path()).unwrap();
        record_opt_in(&mut config, "opted").unwrap();

        clear_opt_in(&mut config, "opted").unwrap();

        assert!(
            config.projects["opted"].driver_opt_in.is_none(),
            "withdrawal removes the record; there is no second representation of 'no'"
        );
        assert!(!is_opted_in(&config, "opted"));
        assert!(
            config.projects.contains_key("opted"),
            "withdrawing the opt-in never unregisters the project"
        );
    }

    #[test]
    fn auto_registration_leaves_every_discovered_project_not_opted_in() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        std::fs::create_dir(dir_a.path().join(".planning")).unwrap();
        std::fs::create_dir(dir_b.path().join(".planning")).unwrap();
        // A CLAUDE.md present at discovery time must make no difference at all.
        std::fs::write(dir_a.path().join("CLAUDE.md"), "# A\n").unwrap();

        let mut config = empty_config();
        let sessions = vec![
            make_session(dir_a.path().to_path_buf()),
            make_session(dir_b.path().to_path_buf()),
        ];

        let added = auto_register_from_sessions(&mut config, &sessions);

        assert_eq!(added.len(), 2, "both discovered projects register");
        for (alias, entry) in &config.projects {
            assert!(
                entry.driver_opt_in.is_none(),
                "discovery may REGISTER; driving requires a separate, explicit, \
                 persisted opt-in — but '{alias}' came back opted in (D-15)"
            );
        }
    }

    #[test]
    fn auto_register_returns_canonical_paths() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join(".planning")).unwrap();
        let mut config = empty_config();
        let sessions = vec![make_session(dir.path().to_path_buf())];

        let added = auto_register_from_sessions(&mut config, &sessions);

        let canonical = dir.path().canonicalize().unwrap();
        assert_eq!(added[0].1, canonical);
    }
}
