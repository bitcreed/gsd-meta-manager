use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::state_reader::phase_num::PhaseNum;
use crate::text::Untrusted;

/// Depth levels for archive drill-down navigation.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ArchiveDepth {
    /// Level 0: List of milestones (v1.0, v1.1, ...)
    #[default]
    MilestoneList,
    /// Level 1: Phases within a selected milestone
    PhaseList { milestone: String },
    /// Level 2: Artifact files within a selected phase
    FileList { milestone: String, phase_idx: usize },
    /// Level 3: Viewing file content with markdown styling
    /// `phase_idx`: `None` = top-level file (entered from PhaseList),
    /// `Some(idx)` = phase file (entered from FileList)
    FileView {
        milestone: String,
        phase_idx: Option<usize>,
        file_idx: usize,
    },
}

/// Parsed milestone archive with top-level files and phase subdirectories.
#[derive(Debug, Clone)]
pub struct MilestoneArchive {
    pub version: String,
    pub top_level_files: Vec<ArchiveFile>,
    pub phases: Vec<PhaseArchive>,
}

/// A phase directory within a milestone archive.
///
/// **`name` and `display_name` are [`Untrusted`]** (D-21-19). `name` is the
/// slug half of a directory name under `.planning/archive/`, and
/// `display_name` is a `format!` over a Title-Cased rendering of that same
/// slug — so the untrusted bytes are in both, and the authored half of
/// `display_name` (`"Phase {padded}: "`) is escaped harmlessly along with them.
///
/// `number` is a [`PhaseNum`]: it survived [`PhaseNum::parse`] (digits and dots
/// only), which is a stronger guarantee than any carrier could give it. It was a
/// `u32`, which silently dropped every archived inserted phase (`07.1-…`) from
/// the Archive tab.
#[derive(Debug, Clone)]
pub struct PhaseArchive {
    pub number: PhaseNum,
    pub name: Untrusted,
    pub display_name: Untrusted,
    pub files: Vec<ArchiveFile>,
}

/// A single markdown file in the archive.
///
/// **`name` is [`Untrusted`]** (D-21-19): it is a file name read off disk from
/// a repository the user cloned, and `detail.rs`'s Archive tab draws it through
/// a `ListItem` — the widget family measured in 21-23 as PRESERVING the entire
/// invisible class, including `U+202E`.
#[derive(Debug, Clone)]
pub struct ArchiveFile {
    pub name: Untrusted,
    pub path: PathBuf,
}

/// Every parsed milestone archive the TUI holds, keyed **alias first,
/// milestone second**.
///
/// A type rather than a bare map because the bare map's key was misread. It
/// was a `HashMap<String, MilestoneArchive>` keyed by milestone (`"v1.2"`), and
/// debug session `archive-milestone-view-loading` found both of the defects
/// that key allowed: `App::prune_driver_maps` retained it by registered ALIAS,
/// so every entry was dropped on every 20-tick pass and the Archive tab's
/// drill-in flipped back to `Loading...` a few seconds after it loaded; and two
/// projects that both archived a `v1.2` shared one entry, so the second project
/// drew the first one's phases. Every method here names both halves of the key,
/// and the only pruning primitive is [`ArchiveCache::retain_aliases`], so
/// neither mistake can be spelled against this type.
#[derive(Debug, Default)]
pub struct ArchiveCache(
    std::collections::HashMap<String, std::collections::HashMap<String, MilestoneArchive>>,
);

impl ArchiveCache {
    /// `alias`'s archive of `milestone`, if it has been loaded.
    pub fn get(&self, alias: &str, milestone: &str) -> Option<&MilestoneArchive> {
        self.0.get(alias)?.get(milestone)
    }

    /// Store (or replace, on an in-place reload) `alias`'s archive of `milestone`.
    pub fn insert(&mut self, alias: String, milestone: String, data: MilestoneArchive) {
        self.0.entry(alias).or_default().insert(milestone, data);
    }

    /// The milestones loaded for `alias` — what an in-place refresh re-reads.
    pub fn loaded_milestones(&self, alias: &str) -> Vec<String> {
        self.0
            .get(alias)
            .map(|by_milestone| by_milestone.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Whether anything is held for `alias`.
    pub fn has_alias(&self, alias: &str) -> bool {
        self.0.contains_key(alias)
    }

    /// Keep only the aliases `keep` accepts, dropping each rejected alias's
    /// archives whole. The prune's primitive: it is handed an ALIAS, never a
    /// milestone.
    pub fn retain_aliases(&mut self, mut keep: impl FnMut(&str) -> bool) {
        self.0.retain(|alias, _| keep(alias));
    }
}

/// Discover milestones by scanning for `v*-ROADMAP.md` files.
///
/// Returns sorted version strings like `["v1.0", "v1.1"]`.
/// Returns empty vec if the directory does not exist.
pub fn discover_milestones(milestones_dir: &Path) -> Vec<String> {
    if !milestones_dir.is_dir() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(milestones_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut versions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('v') && name.ends_with("-ROADMAP.md") {
                Some(name.trim_end_matches("-ROADMAP.md").to_string())
            } else {
                None
            }
        })
        .collect();

    // Natural version sort: split on '.', compare numeric parts
    versions.sort_by(|a, b| {
        let parse_parts = |s: &str| -> Vec<u64> {
            s.trim_start_matches('v')
                .split('.')
                .filter_map(|p| p.parse::<u64>().ok())
                .collect()
        };
        let pa = parse_parts(a);
        let pb = parse_parts(b);
        pa.cmp(&pb)
    });

    versions
}

/// Load a milestone archive for a given version string.
///
/// Scans for top-level files matching `{version}-*` and phase directories
/// under `{version}-phases/`.
pub fn load_milestone_archive(milestones_dir: &Path, version: &str) -> MilestoneArchive {
    // Top-level files: files starting with "{version}-"
    let prefix = format!("{}-", version);
    let mut top_level_files: Vec<ArchiveFile> = std::fs::read_dir(milestones_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with(&prefix) && e.path().is_file()
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // One of the two places an ArchiveFile is created, so one of the
            // two places the name is wrapped.
            ArchiveFile {
                name: Untrusted::from_untrusted_source(name),
                path: e.path().canonicalize().unwrap_or_else(|_| e.path()),
            }
        })
        .collect();
    // A sort ORDER, not something a human reads.
    top_level_files.sort_by(|a, b| {
        a.name
            .as_raw_for_logic_only()
            .cmp(b.name.as_raw_for_logic_only())
    });

    // Phases: scan {version}-phases/ directory
    let phases_dir = milestones_dir.join(format!("{}-phases", version));
    let mut phases: Vec<PhaseArchive> = if phases_dir.is_dir() {
        std::fs::read_dir(&phases_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| {
                let dir_name = e.file_name().to_string_lossy().to_string();
                parse_phase_dir(&dir_name, &e.path())
            })
            .collect()
    } else {
        Vec::new()
    };
    phases.sort_by(|a, b| a.number.cmp(&b.number));

    MilestoneArchive {
        version: version.to_string(),
        top_level_files,
        phases,
    }
}

/// Where a milestone key points, as parsed by [`parse_milestone_key`].
///
/// Docs › Milestones used to read ONE planning directory (`.planning/`, one
/// `milestones/`), so a bare version was a unique identity. It now reads the
/// project's plus each parallel workstream's, and `v0.13` can be archived in
/// both, so the identity is (source planning directory, version). The key stays
/// a `String` because that is what `Action::ArchiveLoaded`, [`ArchiveDepth`] and
/// [`ArchiveCache`] carry; this type is the one way back from the string to a
/// path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneSource<'a> {
    /// An archived milestone of the project itself: `<planning>/milestones/`.
    /// The key is the bare version (`v1.2`), exactly as before workstreams.
    Root { version: &'a str },
    /// A workstream's ACTIVE milestone: its own planning directory,
    /// `<planning>/workstreams/<dir>/`. The key is `<dir>/`.
    Workstream { workstream: &'a str },
    /// A milestone a workstream archived:
    /// `<planning>/workstreams/<dir>/milestones/`. The key is `<dir>/<version>`.
    WorkstreamArchive {
        workstream: &'a str,
        version: &'a str,
    },
}

/// The key of a workstream's milestone: `<ws>/` for its active milestone,
/// `<ws>/<version>` for an archived one. The inverse of [`parse_milestone_key`].
pub fn workstream_milestone_key(workstream: &str, version: Option<&str>) -> String {
    format!("{workstream}/{}", version.unwrap_or(""))
}

/// One plain path component: non-empty, not `.` or `..`, and no separator.
fn is_plain_component(part: &str) -> bool {
    !part.is_empty() && part != "." && part != ".." && !part.contains('/')
}

/// Parse a milestone key into where it points; `None` when it names no
/// directory this module may read.
///
/// The grammar follows the `scoped_phase_key` precedent (`/` cannot occur in a
/// directory name, so it is a safe separator):
///
/// - `v1.2` (no `/`): [`MilestoneSource::Root`];
/// - `<ws>/`: [`MilestoneSource::Workstream`];
/// - `<ws>/<version>`: [`MilestoneSource::WorkstreamArchive`].
///
/// Every component must be a plain name: empty, `.`, `..` and extra segments are
/// rejected, so a key can never walk out of the project (T-05h-01). Every
/// loader resolves a key through this function before it builds a path.
pub fn parse_milestone_key(key: &str) -> Option<MilestoneSource<'_>> {
    match key.split_once('/') {
        None => is_plain_component(key).then_some(MilestoneSource::Root { version: key }),
        Some((workstream, rest)) => {
            if !is_plain_component(workstream) {
                return None;
            }
            if rest.is_empty() {
                Some(MilestoneSource::Workstream { workstream })
            } else if is_plain_component(rest) {
                Some(MilestoneSource::WorkstreamArchive {
                    workstream,
                    version: rest,
                })
            } else {
                None
            }
        }
    }
}

/// The names of `planning_dir/workstreams/*` subdirectories, in name order.
/// Hidden and non-UTF-8 names are skipped; so is anything that is not a
/// directory. Empty when there is no `workstreams/` directory.
fn workstream_dir_names(planning_dir: &Path) -> Vec<String> {
    let Ok(read) = std::fs::read_dir(planning_dir.join("workstreams")) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort();
    names
}

/// Every milestone Docs › Milestones lists for the project whose planning
/// directory is `planning_dir` (`<project>/.planning`), as keys for
/// [`load_milestone_by_key`].
///
/// The root's archived milestones come first, unchanged
/// ([`discover_milestones`] over `milestones/`). Then, for each workstream in
/// name order, its active milestone (`<ws>/`) and any milestones it archived
/// (`<ws>/<version>`). A project without `workstreams/` yields exactly
/// `discover_milestones(planning_dir/milestones)`.
pub fn discover_all_milestones(planning_dir: &Path) -> Vec<String> {
    let mut keys = discover_milestones(&planning_dir.join("milestones"));
    for workstream in workstream_dir_names(planning_dir) {
        let archived = discover_milestones(
            &planning_dir
                .join("workstreams")
                .join(&workstream)
                .join("milestones"),
        );
        keys.push(workstream_milestone_key(&workstream, None));
        keys.extend(
            archived
                .iter()
                .map(|version| workstream_milestone_key(&workstream, Some(version))),
        );
    }
    keys
}

/// Load the milestone a key from [`discover_all_milestones`] names.
///
/// The key is resolved through [`parse_milestone_key`] first; one that does not
/// parse loads nothing (an empty archive, no filesystem access).
pub fn load_milestone_by_key(planning_dir: &Path, key: &str) -> MilestoneArchive {
    match parse_milestone_key(key) {
        Some(MilestoneSource::Root { version }) => {
            load_milestone_archive(&planning_dir.join("milestones"), version)
        }
        Some(MilestoneSource::WorkstreamArchive {
            workstream,
            version,
        }) => load_milestone_archive(
            &planning_dir
                .join("workstreams")
                .join(workstream)
                .join("milestones"),
            version,
        ),
        Some(MilestoneSource::Workstream { workstream }) => {
            load_workstream_milestone(&planning_dir.join("workstreams").join(workstream), key)
        }
        None => MilestoneArchive {
            version: key.to_string(),
            top_level_files: Vec::new(),
            phases: Vec::new(),
        },
    }
}

/// A workstream's ACTIVE milestone: the docs sitting directly in its planning
/// directory (`*.md`, plus `HANDOFF.json` when present; `config.json`,
/// `state.json` and `milestone.lock` are machine state, not docs) and its
/// `phases/`. Opened in place, so these are the files GSD itself edits.
fn load_workstream_milestone(ws_dir: &Path, key: &str) -> MilestoneArchive {
    let mut top_level_files: Vec<ArchiveFile> = std::fs::read_dir(ws_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            (name.ends_with(".md") || name == "HANDOFF.json") && e.path().is_file()
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            ArchiveFile {
                name: Untrusted::from_untrusted_source(name),
                path: e.path().canonicalize().unwrap_or_else(|_| e.path()),
            }
        })
        .collect();
    // A sort ORDER, not something a human reads.
    top_level_files.sort_by(|a, b| {
        a.name
            .as_raw_for_logic_only()
            .cmp(b.name.as_raw_for_logic_only())
    });

    let mut phases: Vec<PhaseArchive> = std::fs::read_dir(ws_dir.join("phases"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let dir_name = e.file_name().to_string_lossy().to_string();
            parse_phase_dir(&dir_name, &e.path())
        })
        .collect();
    phases.sort_by(|a, b| a.number.cmp(&b.number));

    MilestoneArchive {
        version: key.to_string(),
        top_level_files,
        phases,
    }
}

/// Parse a phase directory name like "01-core-infrastructure" into a PhaseArchive.
fn parse_phase_dir(dir_name: &str, dir_path: &Path) -> Option<PhaseArchive> {
    let parts: Vec<&str> = dir_name.splitn(2, '-').collect();
    if parts.len() < 2 {
        return None;
    }

    let number = PhaseNum::parse(parts[0])?;
    let slug = parts[1];

    // Convert kebab-case to Title Case
    let title = slug
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    upper + &chars.collect::<String>()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let display_name = format!("Phase {}: {}", number.padded(), title);

    // List .md files in the phase directory
    let mut files: Vec<ArchiveFile> = std::fs::read_dir(dir_path)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md") && e.path().is_file()
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            ArchiveFile {
                name: Untrusted::from_untrusted_source(name),
                path: e.path().canonicalize().unwrap_or_else(|_| e.path()),
            }
        })
        .collect();
    files.sort_by(|a, b| {
        a.name
            .as_raw_for_logic_only()
            .cmp(b.name.as_raw_for_logic_only())
    });

    Some(PhaseArchive {
        number,
        name: Untrusted::from_untrusted_source(slug.to_string()),
        // Wrapped at the END of the construction, once, rather than wrapping
        // `title` and re-interpolating: `display_name` is a single string whose
        // untrusted half is `title` and whose authored half is `"Phase {padded}: "`,
        // and escaping the whole thing leaves the authored half unchanged
        // (the invisible class contains no ASCII).
        display_name: Untrusted::from_untrusted_source(display_name),
        files,
    })
}

/// Read archive file content. Returns error message on failure.
pub fn read_archive_file(path: &Path) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => {
            let filename = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("Could not read file: {}", filename)
        }
    }
}

/// Render markdown text into styled ratatui Lines.
///
/// Supports: `#`/`##`/`###` headings, `**bold**` inline, triple-backtick
/// code blocks (DarkGray), and `- ` list items (rendered as-is).
///
/// # Every line is escaped here, and this is the ONE place it happens
///
/// This function is the single render for BOTH file viewers — the Archive tab's
/// `FileView` depth and the Browse tab's `View` depth — and what it draws is the
/// BODY of a markdown file read off disk from a repository the user cloned.
/// `ProjectViewCache::{archive_file_content, browser_file_content}` are
/// deliberately still `String` rather than `crate::text::Untrusted`, because a
/// file body is not a name and the carrier's accessors do not fit a value that
/// is split into lines and pattern-matched for markdown prefixes. **What closes
/// the gap that leaves is this function**, which every byte of both bodies flows
/// through.
///
/// The Backlog tab's content pane is a third consumer (debug
/// backlog-content-empty): its `BacklogItem::content` IS an `Untrusted`, and
/// the pane hands this function `as_raw_for_logic_only()` precisely so the
/// escape happens here, per line, rather than through `shown()` over the whole
/// body — which turns every `\n` into a visible `·` and draws one row.
///
/// Found by POPULATING `browser_file_content` in the render probe (21-25 T2),
/// not by reading. Verbatim, before this escape landed:
///
/// ```text
/// thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (719875) panicked at src/ui/screens/render_escape_guard.rs:1670:17:
/// DetailScreen (src/ui/screens/detail.rs) [Browse tab, file view] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
/// ```
///
/// **Escaped per LINE, never over the whole document**, because
/// `crate::text::strip_terminal_controls` replaces every C0 control with a
/// visible marker and `\n` is `0x0A` — escaping first and splitting second
/// would collapse the entire file into one row. Splitting first and escaping
/// each line leaves the markdown prefixes (`#`, backticks, `-`) untouched:
/// none of them is in either class.
pub fn render_markdown_lines(content: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;

    for raw_line in content.lines() {
        let escaped = crate::text::render_for_terminal(raw_line).to_string();
        let line = escaped.as_str();
        if line.starts_with("```") {
            in_code_block = !in_code_block;
            // Render the backtick delimiter itself in code block style
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(Color::DarkGray),
            )));
            continue;
        }

        // Check headings (### before ## before #)
        if line.starts_with("### ") {
            let text = line.trim_start_matches("### ").to_string();
            lines.push(Line::from(Span::styled(
                text,
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::DIM)
                    .add_modifier(Modifier::UNDERLINED),
            )));
        } else if line.starts_with("## ") {
            let text = line.trim_start_matches("## ").to_string();
            lines.push(Line::from(Span::styled(
                text,
                Style::default().add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("# ") {
            let text = line.trim_start_matches("# ").to_string();
            lines.push(Line::from(Span::styled(
                text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(parse_inline_styles(line));
        }
    }

    lines
}

/// Generate right-aligned line number Lines for a visible window of content.
///
/// `total` is the total number of lines in the content.
/// `scroll` is the current scroll offset (0-based).
/// `visible` is the number of visible rows in the viewport.
pub fn line_number_lines(total: usize, scroll: u16, visible: u16) -> Vec<Line<'static>> {
    let start = scroll as usize;
    let end = (start + visible as usize).min(total);
    let width = total.max(1).to_string().len();
    (start..end)
        .map(|i| {
            Line::from(Span::styled(
                format!("{:>width$} ", i + 1, width = width),
                Style::default().fg(Color::DarkGray),
            ))
        })
        .collect()
}

/// Parse inline `**bold**` markers into mixed Span sequences.
fn parse_inline_styles(line: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut remaining = line;
    let bold_style = Style::default().add_modifier(Modifier::BOLD);

    loop {
        match remaining.find("**") {
            None => {
                spans.push(Span::raw(remaining.to_string()));
                break;
            }
            Some(start) => {
                if start > 0 {
                    spans.push(Span::raw(remaining[..start].to_string()));
                }
                let after_open = &remaining[start + 2..];
                match after_open.find("**") {
                    None => {
                        // Unmatched **, render as-is
                        spans.push(Span::raw(remaining[start..].to_string()));
                        break;
                    }
                    Some(end) => {
                        let bold_text = &after_open[..end];
                        spans.push(Span::styled(bold_text.to_string(), bold_style));
                        remaining = &after_open[end + 2..];
                    }
                }
            }
        }
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An archived inserted phase (`07.1-…`) is listed, ordered between its
    /// neighbours, and labelled with GSD's padded spelling. A `u32` parse
    /// dropped it from the Archive tab entirely.
    #[test]
    fn archived_decimal_phases_are_listed_in_numeric_order() {
        let dir = tempfile::tempdir().unwrap();
        let phases = dir.path().join("v1.0-phases");
        for name in [
            "08-next",
            "07.1-inserted-fix",
            "07-consolidation",
            "10-later",
        ] {
            std::fs::create_dir_all(phases.join(name)).unwrap();
        }
        let archive = load_milestone_archive(dir.path(), "v1.0");
        let labels: Vec<&str> = archive
            .phases
            .iter()
            .map(|p| p.display_name.as_raw_for_logic_only())
            .collect();
        assert_eq!(
            labels,
            [
                "Phase 07: Consolidation",
                "Phase 07.1: Inserted Fix",
                "Phase 08: Next",
                "Phase 10: Later",
            ]
        );
    }

    // ── quick 261023-05h: workstream milestones ─────────────────────────────

    use std::path::PathBuf;

    fn touch(path: PathBuf) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "# x\n").unwrap();
    }

    /// The sentriq tree plus an archived root milestone `v0.11` and an archived
    /// `v0.13` under the v0.13 workstream (so `v0.13` exists only there).
    fn sentriq_planning() -> (tempfile::TempDir, PathBuf) {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path().join(".planning");
        crate::state_reader::workstreams::write_sentriq_workstreams_fixture(
            &planning,
            chrono::Utc::now(),
        );
        touch(planning.join("milestones/v0.11-ROADMAP.md"));
        touch(planning.join("milestones/v0.11-phases/14-old/14-01-PLAN.md"));
        let ws13 = planning.join("workstreams/v0-13-public-beta/milestones");
        touch(ws13.join("v0.13-ROADMAP.md"));
        touch(ws13.join("v0.13-phases/16-beta-build-channel/16-01-PLAN.md"));
        (td, planning)
    }

    /// A field-by-field rendering of an archive, for equality (the type has no
    /// `PartialEq`).
    fn flatten(a: &MilestoneArchive) -> Vec<String> {
        let mut out = vec![format!("version={}", a.version)];
        for f in &a.top_level_files {
            out.push(format!(
                "top {} {}",
                f.name.as_raw_for_logic_only(),
                f.path.display()
            ));
        }
        for p in &a.phases {
            out.push(format!(
                "phase {} {} {}",
                p.number.padded(),
                p.name.as_raw_for_logic_only(),
                p.display_name.as_raw_for_logic_only()
            ));
            for f in &p.files {
                out.push(format!(
                    "  file {} {}",
                    f.name.as_raw_for_logic_only(),
                    f.path.display()
                ));
            }
        }
        out
    }

    fn top_names(a: &MilestoneArchive) -> Vec<&str> {
        a.top_level_files
            .iter()
            .map(|f| f.name.as_raw_for_logic_only())
            .collect()
    }

    #[test]
    fn discover_all_milestones_lists_root_archives_then_each_workstream() {
        let (_td, planning) = sentriq_planning();
        // v0.14's empty `milestones/archived-20261022-phases/` yields nothing.
        assert_eq!(
            discover_all_milestones(&planning),
            [
                "v0.11",
                "v0-13-public-beta/",
                "v0-13-public-beta/v0.13",
                "v0-14-voice-assistant/"
            ]
        );
    }

    #[test]
    fn load_a_workstream_active_milestone_lists_its_docs_and_phases_in_place() {
        let (_td, planning) = sentriq_planning();
        let ws14 =
            std::fs::canonicalize(planning.join("workstreams/v0-14-voice-assistant")).unwrap();
        let a = load_milestone_by_key(&planning, "v0-14-voice-assistant/");
        assert_eq!(a.version, "v0-14-voice-assistant/");
        // No HANDOFF.json (absent), and config.json / state.json / milestone.lock
        // are machine state, not docs.
        assert_eq!(
            top_names(&a),
            [
                "MILESTONE-CONTEXT.md",
                "REQUIREMENTS.md",
                "ROADMAP.md",
                "STATE.md"
            ]
        );
        let numbers: Vec<String> = a.phases.iter().map(|p| p.number.padded()).collect();
        assert_eq!(numbers, ["23", "24", "25", "26", "28"]);
        let p26 = a.phases.iter().find(|p| p.number.padded() == "26").unwrap();
        let files: Vec<&str> = p26
            .files
            .iter()
            .map(|f| f.name.as_raw_for_logic_only())
            .collect();
        for n in 1..=14 {
            assert!(
                files.contains(&format!("26-{n:02}-PLAN.md").as_str()),
                "{files:?}"
            );
        }
        assert!(files.contains(&"26-VERIFICATION.md"));
        for f in a
            .top_level_files
            .iter()
            .chain(a.phases.iter().flat_map(|p| p.files.iter()))
        {
            assert!(f.path.starts_with(&ws14), "{:?} escapes {ws14:?}", f.path);
        }
    }

    #[test]
    fn load_other_keys_resolve_under_their_own_directories() {
        let (_td, planning) = sentriq_planning();
        let a = load_milestone_by_key(&planning, "v0-13-public-beta/");
        assert!(
            top_names(&a).contains(&"HANDOFF.json"),
            "{:?}",
            top_names(&a)
        );
        let numbers: Vec<String> = a.phases.iter().map(|p| p.number.padded()).collect();
        assert_eq!(numbers, ["16", "17", "18", "19", "20", "21", "22"]);

        let archived = load_milestone_by_key(&planning, "v0-13-public-beta/v0.13");
        assert_eq!(top_names(&archived), ["v0.13-ROADMAP.md"]);
        assert_eq!(archived.phases.len(), 1);
        assert_eq!(archived.phases[0].number.padded(), "16");
        let base = std::fs::canonicalize(planning.join("workstreams/v0-13-public-beta/milestones"))
            .unwrap();
        for f in archived
            .top_level_files
            .iter()
            .chain(archived.phases[0].files.iter())
        {
            assert!(f.path.starts_with(&base), "{:?}", f.path);
        }

        // A bare version is the root's, unchanged.
        let root = load_milestone_by_key(&planning, "v0.11");
        assert_eq!(
            flatten(&root),
            flatten(&load_milestone_archive(
                &planning.join("milestones"),
                "v0.11"
            ))
        );
    }

    /// Every key discovery yields parses, and its load stays inside the
    /// directory the key names.
    #[test]
    fn every_discovered_key_parses_and_loads_only_from_the_directory_it_names() {
        let (_td, planning) = sentriq_planning();
        for key in discover_all_milestones(&planning) {
            let source = parse_milestone_key(&key).unwrap_or_else(|| panic!("{key} parses"));
            let base = match source {
                MilestoneSource::Root { .. } => planning.join("milestones"),
                MilestoneSource::Workstream { workstream } => {
                    planning.join("workstreams").join(workstream)
                }
                MilestoneSource::WorkstreamArchive { workstream, .. } => planning
                    .join("workstreams")
                    .join(workstream)
                    .join("milestones"),
            };
            let base = std::fs::canonicalize(base).unwrap();
            let a = load_milestone_by_key(&planning, &key);
            assert!(!a.top_level_files.is_empty(), "{key} loads something");
            for f in a
                .top_level_files
                .iter()
                .chain(a.phases.iter().flat_map(|p| p.files.iter()))
            {
                assert!(
                    f.path.starts_with(&base),
                    "{key}: {:?} not under {base:?}",
                    f.path
                );
            }
        }
    }

    #[test]
    fn a_key_that_could_leave_the_project_parses_to_nothing_and_loads_nothing() {
        let (_td, planning) = sentriq_planning();
        for key in ["", ".", "..", "../x/", "./", "x/..", "x/.", "a/b/c", "/v1"] {
            assert!(parse_milestone_key(key).is_none(), "{key:?} must not parse");
            let a = load_milestone_by_key(&planning, key);
            assert!(a.top_level_files.is_empty(), "{key:?}");
            assert!(a.phases.is_empty(), "{key:?}");
        }
    }

    #[test]
    fn milestone_keys_round_trip_through_the_grammar() {
        assert_eq!(workstream_milestone_key("ws", None), "ws/");
        assert_eq!(workstream_milestone_key("ws", Some("v1.0")), "ws/v1.0");
        assert!(matches!(
            parse_milestone_key("v1.2"),
            Some(MilestoneSource::Root { version: "v1.2" })
        ));
        assert!(matches!(
            parse_milestone_key("ws/"),
            Some(MilestoneSource::Workstream { workstream: "ws" })
        ));
        assert!(matches!(
            parse_milestone_key("ws/v1.0"),
            Some(MilestoneSource::WorkstreamArchive {
                workstream: "ws",
                version: "v1.0"
            })
        ));
    }

    #[test]
    fn a_flat_project_discovers_exactly_what_it_did_before() {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path().join(".planning");
        touch(planning.join("milestones/v1.0-ROADMAP.md"));
        touch(planning.join("milestones/v1.1-ROADMAP.md"));
        touch(planning.join("milestones/v1.10-ROADMAP.md"));
        assert_eq!(
            discover_all_milestones(&planning),
            discover_milestones(&planning.join("milestones"))
        );
        // And none at all.
        let empty = td.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(discover_all_milestones(&empty).is_empty());
    }
}
