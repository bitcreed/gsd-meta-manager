//! Run templates: what `driver new` writes and what `driver check` expects
//! (Phase 29; D-08 .. D-12, PRMT-02).
//!
//! A new run is one directory, `<project>/.planning/meta-manager/drivers/<run-id>/`,
//! holding exactly three files:
//!
//! * `.gitignore` containing `*`, written FIRST so the run dir ignores itself and
//!   its goal text and LOG can never be staged, with no edit to the project's own
//!   `.gitignore` (D-09, non-intrusive);
//! * `LOG.md`, the append-only event log, starting with its grammar as a comment;
//! * `DIRECTION.md`: the tool-owned never-launched frontmatter (the Phase 27
//!   emitter, never hand-written) followed by the human's template body.
//!
//! No `state.json` is written: the run store opens a missing one as a fresh run.
//!
//! The filesystem half is Linux-only for the reason the run store is (it builds on
//! `driverctl::store` and `driverctl::serve`); the templates are plain constants
//! and compile everywhere.
//!
//! # Safety properties
//!
//! * Every refusal happens before the first directory is created: the slug shape
//!   and the composed id are checked first, then `.planning`,
//!   `.planning/meta-manager` and `.planning/meta-manager/drivers` are refused if
//!   any is a symlink (a cloned repository can plant one where the tool writes,
//!   Pitfall 7), then the canonical drivers dir must sit under the canonical
//!   project root.
//! * The run dir is made with `create_dir`, so an existing run is `EEXIST` and a
//!   refusal naming the path (never a suffix, never an overwrite), and every file
//!   is opened `create_new`.

/// The DIRECTION.md body template (no frontmatter). `{run-id}` is filled by
/// `driver new`; every other `{...}` outside an HTML comment is a placeholder the
/// human must replace (the lint reports each one).
pub const DIRECTION_TEMPLATE: &str = include_str!("assets/DIRECTION.md");

/// The LOG.md template: a title and the event grammar as a comment, no events.
pub const LOG_TEMPLATE: &str = include_str!("assets/LOG.md");

/// The token `driver new` replaces with the minted run id.
pub const RUN_ID_TOKEN: &str = "{run-id}";

/// What a slug must look like [inferred - audit]: ASCII, one path component, no
/// leading `.` or `-`. The composed `<yyyymmdd>-<slug>` is bounded separately by
/// the run-id rule (80 bytes).
pub const SLUG_RE: &str = r"^[A-Za-z0-9][A-Za-z0-9._-]*$";

#[cfg(target_os = "linux")]
pub use create::{create_run_dir, mint_run_id, NewRun, NewRunError};

#[cfg(target_os = "linux")]
mod create {
    use std::fs::{self, OpenOptions};
    use std::io::Write as _;
    use std::path::{Path, PathBuf};
    use std::sync::LazyLock;

    use chrono::{DateTime, Utc};
    use regex::Regex;

    use super::{DIRECTION_TEMPLATE, LOG_TEMPLATE, RUN_ID_TOKEN, SLUG_RE};
    use crate::driver::direction::parse_direction;
    use crate::driverctl::log::utc_string;
    use crate::driverctl::serve::{valid_run_id, DRIVERS_REL};
    use crate::driverctl::store::{frontmatter_for, RunState};
    use crate::text::render_for_terminal;

    static SLUG: LazyLock<Regex> = LazyLock::new(|| Regex::new(SLUG_RE).expect("static regex"));

    /// A run directory `driver new` created.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NewRun {
        /// `<yyyymmdd>-<slug>`.
        pub run_id: String,
        /// The (canonical) run directory.
        pub run_dir: PathBuf,
        /// `<run_dir>/DIRECTION.md`.
        pub direction: PathBuf,
    }

    /// Why `driver new` created nothing (or, for [`NewRunError::Io`], could not
    /// finish). `Display` is one line each; paths and the slug are escaped for the
    /// terminal.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum NewRunError {
        /// The slug (or the id it composes) is not usable; carries the slug.
        BadSlug(String),
        /// `.planning` is missing or not a directory.
        NotGsdProject(PathBuf),
        /// A component the tool would write through is a symlink.
        Symlink(PathBuf),
        /// The drivers dir resolves outside the project root.
        OutsideProject(PathBuf),
        /// The run directory already exists.
        Exists(PathBuf),
        /// A filesystem fault.
        Io(String),
    }

    fn shown(path: &Path) -> String {
        render_for_terminal(&path.display().to_string()).to_string()
    }

    impl std::fmt::Display for NewRunError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                NewRunError::BadSlug(slug) => write!(
                    f,
                    "slug {:?} is not usable: it must match {SLUG_RE} (ASCII letters, digits, \
                     '.', '_', '-'; not empty, no leading '.' or '-') and the run id \
                     <yyyymmdd>-<slug> must be at most 80 bytes",
                    render_for_terminal(slug).to_string()
                ),
                NewRunError::NotGsdProject(p) => write!(
                    f,
                    "{} is not a directory: this is not a GSD project (no .planning)",
                    shown(p)
                ),
                NewRunError::Symlink(p) => write!(
                    f,
                    "refusing to write through the symlink {}; nothing was created",
                    shown(p)
                ),
                NewRunError::OutsideProject(p) => write!(
                    f,
                    "{} resolves outside the project; nothing was created",
                    shown(p)
                ),
                NewRunError::Exists(p) => {
                    write!(f, "the run directory {} already exists", shown(p))
                }
                NewRunError::Io(detail) => f.write_str(detail),
            }
        }
    }

    impl std::error::Error for NewRunError {}

    /// Compose `<yyyymmdd>-<slug>` from the UTC date and check it.
    ///
    /// The slug shape ([`SLUG_RE`]) is checked first, then the composed id with
    /// [`valid_run_id`], which also bounds it at 80 bytes (Pitfall 8).
    pub fn mint_run_id(slug: &str, now: DateTime<Utc>) -> Result<String, NewRunError> {
        if !SLUG.is_match(slug) {
            return Err(NewRunError::BadSlug(slug.to_owned()));
        }
        let run_id = format!("{}-{slug}", now.format("%Y%m%d"));
        if !valid_run_id(&run_id) {
            return Err(NewRunError::BadSlug(slug.to_owned()));
        }
        Ok(run_id)
    }

    fn io(path: &Path, e: &std::io::Error) -> NewRunError {
        NewRunError::Io(format!("{}: {e}", shown(path)))
    }

    /// Refuse a symlink at `path`; `Ok(true)` when it exists as a real directory,
    /// `Ok(false)` when it does not exist yet.
    fn real_dir_or_absent(path: &Path) -> Result<bool, NewRunError> {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                Err(NewRunError::Symlink(path.to_path_buf()))
            }
            Ok(meta) if meta.is_dir() => Ok(true),
            Ok(_) => Err(NewRunError::Io(format!(
                "{} exists and is not a directory",
                shown(path)
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(io(path, &e)),
        }
    }

    /// Write one new file; an existing path is an error, never an overwrite.
    fn write_new(path: &Path, content: &str) -> Result<(), NewRunError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| io(path, &e))?;
        file.write_all(content.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|e| io(path, &e))
    }

    /// Create `<project>/.planning/meta-manager/drivers/<yyyymmdd>-<slug>/` with
    /// `.gitignore`, `LOG.md` and `DIRECTION.md`, in that order.
    ///
    /// Nothing outside the new run dir is created or changed, apart from the
    /// missing `meta-manager/` and `drivers/` parents. See the module docs for the
    /// refusals, all of which precede the first `create_dir`.
    pub fn create_run_dir(
        project_root: &Path,
        slug: &str,
        now: DateTime<Utc>,
    ) -> Result<NewRun, NewRunError> {
        let run_id = mint_run_id(slug, now)?;

        let planning = project_root.join(".planning");
        let meta = project_root.join(".planning/meta-manager");
        let drivers = project_root.join(DRIVERS_REL);
        match fs::symlink_metadata(&planning) {
            Ok(m) if m.file_type().is_symlink() => return Err(NewRunError::Symlink(planning)),
            Ok(m) if m.is_dir() => {}
            Ok(_) => return Err(NewRunError::NotGsdProject(planning)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(NewRunError::NotGsdProject(planning))
            }
            Err(e) => return Err(io(&planning, &e)),
        }
        let meta_exists = real_dir_or_absent(&meta)?;
        let drivers_exists = meta_exists && real_dir_or_absent(&drivers)?;

        let canonical_project = fs::canonicalize(project_root).map_err(|e| io(project_root, &e))?;
        // The deepest component that exists must already sit under the project.
        let deepest = if drivers_exists {
            &drivers
        } else if meta_exists {
            &meta
        } else {
            &planning
        };
        let canonical_deepest = fs::canonicalize(deepest).map_err(|e| io(deepest, &e))?;
        if !canonical_deepest.starts_with(&canonical_project) {
            return Err(NewRunError::OutsideProject(canonical_deepest));
        }

        if !drivers_exists {
            fs::create_dir_all(&drivers).map_err(|e| io(&drivers, &e))?;
        }
        let canonical_drivers = fs::canonicalize(&drivers).map_err(|e| io(&drivers, &e))?;
        if !canonical_drivers.starts_with(&canonical_project) {
            return Err(NewRunError::OutsideProject(canonical_drivers));
        }

        let run_dir = canonical_drivers.join(&run_id);
        match fs::create_dir(&run_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(NewRunError::Exists(run_dir));
            }
            Err(e) => return Err(io(&run_dir, &e)),
        }

        let direction = run_dir.join("DIRECTION.md");
        let body = DIRECTION_TEMPLATE.replace(RUN_ID_TOKEN, &run_id);
        let state = RunState::new(
            &run_id,
            &canonical_project.display().to_string(),
            &utc_string(now),
        );
        let block = frontmatter_for(&state, &parse_direction(&body)).emit();
        let written = write_new(&run_dir.join(".gitignore"), "*\n")
            .and_then(|()| {
                write_new(
                    &run_dir.join("LOG.md"),
                    &LOG_TEMPLATE.replace(RUN_ID_TOKEN, &run_id),
                )
            })
            .and_then(|()| write_new(&direction, &format!("{block}{body}")));
        if let Err(e) = written {
            // The directory was created by this call (EEXIST would have refused), so
            // its three known files are ours to take back; nothing else is touched.
            for name in [".gitignore", "LOG.md", "DIRECTION.md"] {
                let _ = fs::remove_file(run_dir.join(name));
            }
            let _ = fs::remove_dir(&run_dir);
            return Err(e);
        }
        Ok(NewRun {
            run_id,
            run_dir,
            direction,
        })
    }
}
