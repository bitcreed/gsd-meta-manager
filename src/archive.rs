use std::path::{Path, PathBuf};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

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
    FileView { milestone: String, phase_idx: Option<usize>, file_idx: usize },
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
/// `display_name` (`"Phase {:02}: "`) is escaped harmlessly along with them.
///
/// `number` stays a `u32`: it survived a `parse::<u32>()`, which is a stronger
/// guarantee than any carrier could give it.
#[derive(Debug, Clone)]
pub struct PhaseArchive {
    pub number: u32,
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
    phases.sort_by_key(|p| p.number);

    MilestoneArchive {
        version: version.to_string(),
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

    let number: u32 = parts[0].parse().ok()?;
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

    let display_name = format!("Phase {:02}: {}", number, title);

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
        // untrusted half is `title` and whose authored half is `"Phase {:02}: "`,
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
