//! Workstreams data layer (GSD 1.8.0).
//!
//! GSD 1.8.0 supports parallel workstreams. Each lives under
//! `.planning/workstreams/<ws>/` and holds its own `ROADMAP.md`, `STATE.md`,
//! `phases/`, and `milestones/` — exactly the shape [`super::parse_project_state`]
//! already parses. The active workstream is named in `.planning/active-workstream`
//! (the file contains the workstream name, e.g. `alpha\n`).
//!
//! Recursion note: a workstream dir is itself a planning-shaped directory, so
//! parsing one via [`super::parse_project_state`] could in principle recurse into
//! another `workstreams/` layer. [`super::parse_project_state`] carries a
//! recursion guard (added in plan 8 task 2) that skips workstream loading when the
//! dir is already inside a `workstreams/` parent, bounding depth to one level.

use std::path::Path;

/// One workstream discovered under `.planning/workstreams/`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkstreamState {
    /// The workstream directory name (e.g. `alpha`).
    pub name: String,
    /// True when this workstream is named in `.planning/active-workstream`.
    pub active: bool,
    /// The parsed project state of the workstream's own planning directory.
    pub state: super::ProjectState,
}

/// Load every workstream under `<planning_dir>/workstreams/`.
///
/// Returns one [`WorkstreamState`] per subdirectory, each parsed with
/// [`super::parse_project_state`]. The workstream named in
/// `<planning_dir>/active-workstream` (trimmed) is marked `active`; when that file
/// is absent no workstream is active. Results are sorted by name for stability.
///
/// Never panics: a missing or unreadable `workstreams/` directory yields an empty
/// `Vec`.
pub fn load_workstreams(planning_dir: &Path) -> Vec<WorkstreamState> {
    let ws_root = planning_dir.join("workstreams");

    let active_name = std::fs::read_to_string(planning_dir.join("active-workstream"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let entries = match std::fs::read_dir(&ws_root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut workstreams: Vec<WorkstreamState> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let active = active_name.as_deref() == Some(name.as_str());
            let state = super::parse_project_state(&e.path());
            Some(WorkstreamState {
                name,
                active,
                state,
            })
        })
        .collect();

    workstreams.sort_by(|a, b| a.name.cmp(&b.name));
    workstreams
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a `.planning/` dir with the given files (relative paths under
    /// `.planning/`). Returns the TempDir (keep it alive).
    fn make_planning(files: &[(&str, &str)]) -> TempDir {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        fs::create_dir_all(&planning).unwrap();
        for (rel, content) in files {
            let path = planning.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
        td
    }

    #[test]
    fn test_two_workstreams_both_returned() {
        let td = make_planning(&[
            (
                "workstreams/alpha/STATE.md",
                "---\nstatus: executing\ncurrent_phase: 1\n---\n",
            ),
            ("workstreams/alpha/ROADMAP.md", "# Roadmap\n"),
            (
                "workstreams/beta/STATE.md",
                "---\nstatus: planning\ncurrent_phase: 2\n---\n",
            ),
            ("workstreams/beta/ROADMAP.md", "# Roadmap\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert_eq!(wss.len(), 2);
        // Sorted by name.
        assert_eq!(wss[0].name, "alpha");
        assert_eq!(wss[1].name, "beta");
        // Each carries its own parsed state.
        assert_eq!(wss[0].state.status, "executing");
        assert_eq!(wss[1].state.status, "planning");
    }

    #[test]
    fn test_active_workstream_flag_set_from_pointer() {
        let td = make_planning(&[
            ("workstreams/alpha/STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
            ("active-workstream", "alpha\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert_eq!(wss.len(), 2);
        assert!(wss[0].active, "alpha should be active");
        assert!(!wss[1].active, "beta should be inactive");
    }

    #[test]
    fn test_no_active_pointer_means_none_active() {
        let td = make_planning(&[
            ("workstreams/alpha/STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert!(wss.iter().all(|w| !w.active));
    }

    #[test]
    fn test_no_workstreams_dir_returns_empty() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert!(wss.is_empty());
    }
}
