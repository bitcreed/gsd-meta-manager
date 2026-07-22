use std::path::{Path, PathBuf};

use crate::state_reader::disk_status::find_phase_dir;
use crate::state_reader::ProjectState;

/// Two-level state for the docs browser: listing a directory or viewing a file.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum BrowserDepth {
    #[default]
    List,
    View,
}

/// A single entry shown in the docs browser listing.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// List the contents of `dir` filtered for the docs browser: directories first
/// (alphabetical), then `.md` files (alphabetical, case-insensitive extension
/// check). Hidden entries (starting with `.`) are skipped.
///
/// Returns an empty vec on read errors — the caller renders that as an empty
/// listing, which is the right UX when the path is missing or unreadable.
pub fn list_dir(dir: &Path) -> Vec<BrowserEntry> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut dirs: Vec<BrowserEntry> = Vec::new();
    let mut files: Vec<BrowserEntry> = Vec::new();

    for entry in entries.flatten() {
        let file_name_os = entry.file_name();
        let name = match file_name_os.to_str() {
            Some(s) => s.to_string(),
            None => continue,
        };

        if name.starts_with('.') {
            continue;
        }

        let path = entry.path();
        let is_dir = entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or_else(|_| path.is_dir());

        if is_dir {
            dirs.push(BrowserEntry {
                name,
                path,
                is_dir: true,
            });
        } else if name.to_lowercase().ends_with(".md") {
            files.push(BrowserEntry {
                name,
                path,
                is_dir: false,
            });
        }
    }

    dirs.sort_by_key(|e| e.name.to_lowercase());
    files.sort_by_key(|e| e.name.to_lowercase());
    dirs.extend(files);
    dirs
}

/// Compute the entry directory for the docs browser given the current project
/// state.
///
/// - If the milestone is complete (`completed_phases >= total_phases > 0`),
///   return the `.planning/` root — there is no active phase to open into.
/// - Otherwise, find the active phase (number = completed_phases + 1) and
///   resolve its directory via `find_phase_dir`. If lookup fails, fall back
///   to the planning root.
pub fn resolve_active_phase_dir(planning_dir: &Path, state: &ProjectState) -> PathBuf {
    if state.total_phases > 0 && state.completed_phases >= state.total_phases {
        return planning_dir.to_path_buf();
    }

    let active_num = state.completed_phases + 1;
    let phase_number = state
        .phases
        .iter()
        .find(|p| p.number.parse::<u32>().ok() == Some(active_num))
        .map(|p| p.number.clone());

    match phase_number {
        Some(num) => find_phase_dir(planning_dir, &num).unwrap_or_else(|| planning_dir.to_path_buf()),
        None => planning_dir.to_path_buf(),
    }
}

/// Read a markdown file as a string. On any I/O error, return a human-readable
/// fallback so the viewer renders the error instead of leaving an empty pane.
pub fn read_md_file(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| {
        format!(
            "Could not read file: {}\n\nError: {}",
            path.display(),
            e
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::roadmap_md::RoadmapPhase;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn list_dir_sorts_dirs_first_then_md_files() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("zeta-dir")).unwrap();
        fs::create_dir(dir.path().join("alpha-dir")).unwrap();
        fs::write(dir.path().join("beta.md"), "x").unwrap();
        fs::write(dir.path().join("apple.md"), "x").unwrap();

        let entries = list_dir(dir.path());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["alpha-dir", "zeta-dir", "apple.md", "beta.md"]);
        assert_eq!(entries[0].is_dir, true);
        assert_eq!(entries[1].is_dir, true);
        assert_eq!(entries[2].is_dir, false);
    }

    #[test]
    fn list_dir_hides_non_md_files() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config.json"), "{}").unwrap();
        fs::write(dir.path().join("notes.md"), "x").unwrap();
        fs::write(dir.path().join("script.sh"), "#!/bin/sh").unwrap();

        let entries = list_dir(dir.path());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["notes.md"]);
    }

    #[test]
    fn list_dir_hides_dotfiles() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join(".hidden-dir")).unwrap();
        fs::write(dir.path().join(".env"), "x").unwrap();
        fs::write(dir.path().join("visible.md"), "x").unwrap();

        let entries = list_dir(dir.path());
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["visible.md"]);
    }

    #[test]
    fn list_dir_empty_on_missing_path() {
        let entries = list_dir(Path::new("/nonexistent/xyz123/path"));
        assert!(entries.is_empty());
    }

    #[test]
    fn resolve_active_phase_dir_returns_root_when_milestone_complete() {
        let dir = tempdir().unwrap();
        let planning = dir.path();
        let state = ProjectState {
            total_phases: 3,
            completed_phases: 3,
            ..Default::default()
        };

        assert_eq!(resolve_active_phase_dir(planning, &state), planning);
    }

    #[test]
    fn resolve_active_phase_dir_finds_active_phase() {
        let dir = tempdir().unwrap();
        let planning = dir.path();
        let phase_dir = planning.join("phases").join("02-dashboard");
        fs::create_dir_all(&phase_dir).unwrap();

        let state = ProjectState {
            total_phases: 4,
            completed_phases: 1,
            phases: vec![
                RoadmapPhase {
                    number: "01".to_string(),
                    name: "Foundation".to_string(),
                    description: String::new(),
                    completed: true,
                    total_plans: 0,
                    completed_plans: 0,
                },
                RoadmapPhase {
                    number: "02".to_string(),
                    name: "Dashboard".to_string(),
                    description: String::new(),
                    completed: false,
                    total_plans: 0,
                    completed_plans: 0,
                },
            ],
            ..Default::default()
        };

        assert_eq!(resolve_active_phase_dir(planning, &state), phase_dir);
    }

    #[test]
    fn resolve_active_phase_dir_falls_back_when_phase_dir_missing() {
        let dir = tempdir().unwrap();
        let planning = dir.path();
        let state = ProjectState {
            total_phases: 4,
            completed_phases: 1,
            phases: vec![RoadmapPhase {
                number: "02".to_string(),
                name: "Dashboard".to_string(),
                description: String::new(),
                completed: false,
                total_plans: 0,
                completed_plans: 0,
            }],
            ..Default::default()
        };
        assert_eq!(resolve_active_phase_dir(planning, &state), planning);
    }
}
