//! A line-indexed port of gsd-core 1.21.0's `markdown-sectionizer.cjs` and
//! `markdown-table.cjs`: the fence grammar, ATX heading tokens, sections and table
//! rows the audit parsers share.
//!
//! Upstream works on UTF-16 string offsets; Rust byte offsets differ from those on
//! any non-ASCII text, while line indices are identical. Everything here therefore
//! takes `&[&str]` lines (split on `\n`, a trailing `\r` tolerated) and returns line
//! indices. Every pass is linear in the line count, with no recursion and no regex.
//!
//! The helpers are shared by the UAT, VERIFICATION and deferred-items parsers. Some
//! of them have no caller until those parsers land, hence the module-wide
//! `allow(dead_code)`: the later plans must be able to build on this file without
//! editing it.

#![allow(dead_code)]

use super::uat::{js_space, js_trim, line_terminator};

/// A line that is a fence delimiter: at most three spaces of indent, then a run of
/// at least three backticks or tildes, then trailing text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FenceDelimiter<'a> {
    pub ch: char,
    pub len: usize,
    pub trailing: &'a str,
    /// CommonMark 4.5: a backtick fence's info string must not hold a backtick.
    pub can_open: bool,
}

impl FenceDelimiter<'_> {
    /// Whether this delimiter closes a fence opened with `opener`.
    pub fn closes(&self, opener: &Opener) -> bool {
        self.ch == opener.ch && self.len >= opener.len && self.trailing.chars().all(js_space)
    }
}

/// An open fence: its character and the length of its opening run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Opener {
    pub ch: char,
    pub len: usize,
}

/// `matchFenceDelimiter`: `^( {0,3})(`{3,}|~{3,})(.*)$` on the line without a
/// trailing `\r`.
pub(super) fn match_fence_delimiter(line: &str) -> Option<FenceDelimiter<'_>> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let ch = match rest.as_bytes().first()? {
        b'`' => '`',
        b'~' => '~',
        _ => return None,
    };
    let len = rest.chars().take_while(|c| *c == ch).count();
    if len < 3 {
        return None;
    }
    let trailing = &rest[len..];
    // `.` never matches a line terminator, so the `$` then fails.
    if trailing.chars().any(line_terminator) {
        return None;
    }
    Some(FenceDelimiter {
        ch,
        len,
        trailing,
        can_open: !(ch == '`' && trailing.contains('`')),
    })
}

/// The lines outside every fenced block (delimiter lines included in what is
/// dropped) and whether the text ended inside an open fence.
pub(super) fn strip_fenced_code<'a>(lines: &[&'a str]) -> (Vec<&'a str>, bool) {
    let mut kept = Vec::new();
    let mut open: Option<Opener> = None;
    for line in lines {
        if let Some(delimiter) = match_fence_delimiter(line) {
            match open {
                None => {
                    if delimiter.can_open {
                        open = Some(Opener {
                            ch: delimiter.ch,
                            len: delimiter.len,
                        });
                    } else {
                        kept.push(*line);
                    }
                }
                Some(opener) => {
                    if delimiter.closes(&opener) {
                        open = None;
                    }
                }
            }
            continue;
        }
        if open.is_none() {
            kept.push(*line);
        }
    }
    (kept, open.is_some())
}

/// One fenced block, by line index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FencedBlock {
    pub ch: char,
    pub len: usize,
    pub info: String,
    pub open_line: usize,
    /// `None` when the block is never closed.
    pub close_line: Option<usize>,
}

/// `scanFencedBlocks`: every fenced block with the same delimiter rules as
/// [`strip_fenced_code`].
pub(super) fn scan_fenced_blocks(lines: &[&str]) -> Vec<FencedBlock> {
    let mut blocks = Vec::new();
    let mut open: Option<(Opener, String, usize)> = None;
    for (index, line) in lines.iter().enumerate() {
        let Some(delimiter) = match_fence_delimiter(line) else {
            continue;
        };
        match &open {
            None => {
                if delimiter.can_open {
                    open = Some((
                        Opener {
                            ch: delimiter.ch,
                            len: delimiter.len,
                        },
                        js_trim(delimiter.trailing).to_owned(),
                        index,
                    ));
                }
            }
            Some((opener, info, open_line)) => {
                if delimiter.closes(opener) {
                    blocks.push(FencedBlock {
                        ch: opener.ch,
                        len: opener.len,
                        info: info.clone(),
                        open_line: *open_line,
                        close_line: Some(index),
                    });
                    open = None;
                }
            }
        }
    }
    if let Some((opener, info, open_line)) = open {
        blocks.push(FencedBlock {
            ch: opener.ch,
            len: opener.len,
            info,
            open_line,
            close_line: None,
        });
    }
    blocks
}

/// An ATX heading outside any fence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Heading {
    pub level: usize,
    pub text: String,
    /// Zero-based index of the heading's line.
    pub line: usize,
}

/// The heading of one line: `^( {0,3})(#{1,6})([ \t]+.*|[ \t]*)?$`, its text without
/// the optional closing `#` sequence.
fn atx_heading(line: &str) -> Option<(usize, String)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    if indent > 3 {
        return None;
    }
    let rest = &line[indent..];
    let hashes = rest.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after = &rest[hashes..];
    let blank_only = after.bytes().all(|b| b == b' ' || b == b'\t');
    let starts_blank = after.starts_with([' ', '\t']);
    if !(blank_only || starts_blank) || after.chars().any(line_terminator) {
        return None;
    }
    let text = after.trim_start_matches([' ', '\t']);
    let trimmed = text.trim_end_matches([' ', '\t']);
    let head = trimmed.trim_end_matches('#');
    let text = if head.len() < trimmed.len() && head.ends_with([' ', '\t']) {
        head
    } else if head.is_empty() && !trimmed.is_empty() {
        // `^#+[ \t]*$`: only a closing sequence is left.
        ""
    } else {
        text
    };
    Some((hashes, js_trim(text).to_owned()))
}

/// `tokenizeHeadings`: the ATX headings outside fenced code, in document order.
pub(super) fn tokenize_headings(lines: &[&str]) -> Vec<Heading> {
    let mut tokens = Vec::new();
    let mut open: Option<Opener> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(delimiter) = match_fence_delimiter(line) {
            match open {
                None if delimiter.can_open => {
                    open = Some(Opener {
                        ch: delimiter.ch,
                        len: delimiter.len,
                    });
                    continue;
                }
                None => {}
                Some(opener) => {
                    if delimiter.closes(&opener) {
                        open = None;
                    }
                    continue;
                }
            }
        }
        if open.is_none() {
            if let Some((level, text)) = atx_heading(line) {
                tokens.push(Heading {
                    level,
                    text,
                    line: index,
                });
            }
        }
    }
    tokens
}

/// A section found by [`collect_section`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Section {
    pub heading: Heading,
    /// The body lines joined with `\n`, trailing whitespace removed.
    pub body: String,
}

/// `collectSection`: the section of the first heading `predicate` accepts. A
/// level-bounded section ends at the next heading of the same or a higher level,
/// an unbounded one at the next heading of any level.
pub(super) fn collect_section(
    lines: &[&str],
    predicate: impl Fn(&Heading) -> bool,
    level_bounded: bool,
) -> Option<Section> {
    let headings = tokenize_headings(lines);
    let target = headings.iter().position(predicate)?;
    let level = headings[target].level;
    let end = headings[target + 1..]
        .iter()
        .find(|next| !level_bounded || next.level <= level)
        .map_or(lines.len(), |next| next.line);
    Some(section_of(lines, &headings[target], end))
}

/// `collectSection` with `stopAtLevel`: the section ends at the next heading whose
/// level is at most `stop_at_level`, whatever the opener's level.
pub(super) fn collect_section_stopping_at(
    lines: &[&str],
    predicate: impl Fn(&Heading) -> bool,
    stop_at_level: usize,
) -> Option<Section> {
    let headings = tokenize_headings(lines);
    let target = headings.iter().position(predicate)?;
    let end = headings[target + 1..]
        .iter()
        .find(|next| next.level <= stop_at_level)
        .map_or(lines.len(), |next| next.line);
    Some(section_of(lines, &headings[target], end))
}

fn section_of(lines: &[&str], heading: &Heading, end: usize) -> Section {
    let start = (heading.line + 1).min(end);
    let body = lines[start..end].join("\n");
    Section {
        heading: heading.clone(),
        body: body.trim_end_matches(js_space).to_owned(),
    }
}

/// `splitTableRow`: the trimmed cells of a GFM row. One leading and one trailing `|`
/// are dropped, the rest is split on a `|` that no backslash precedes, and `\\` and
/// `\|` inside a cell become `\` and `|`.
pub(super) fn split_table_row(line: &str) -> Vec<String> {
    let mut stripped = js_trim(line);
    stripped = stripped.strip_prefix('|').unwrap_or(stripped);
    stripped = stripped.strip_suffix('|').unwrap_or(stripped);
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut previous: Option<char> = None;
    for c in stripped.chars() {
        if c == '|' && previous != Some('\\') {
            cells.push(std::mem::take(&mut cell));
        } else {
            cell.push(c);
        }
        previous = Some(c);
    }
    cells.push(cell);
    cells
        .iter()
        .map(|cell| unescape_cell(js_trim(cell)))
        .collect()
}

/// `replace(/\\([\\|])/g, '$1')`.
fn unescape_cell(cell: &str) -> String {
    let mut out = String::with_capacity(cell.len());
    let mut chars = cell.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && matches!(chars.peek(), Some('\\' | '|')) {
            out.extend(chars.next());
        } else {
            out.push(c);
        }
    }
    out
}

/// `isDelimiterRow`: every cell, whitespace removed, is `:?-+:?`.
pub(super) fn is_delimiter_row(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let squeezed: String = cell.chars().filter(|c| !js_space(*c)).collect();
        let body = squeezed.strip_prefix(':').unwrap_or(&squeezed);
        let body = body.strip_suffix(':').unwrap_or(body);
        !body.is_empty() && body.bytes().all(|b| b == b'-')
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.split('\n').collect()
    }

    fn texts(text: &str) -> Vec<(usize, String)> {
        tokenize_headings(&lines(text))
            .into_iter()
            .map(|h| (h.level, h.text))
            .collect()
    }

    #[test]
    fn a_heading_inside_a_backtick_or_tilde_fence_is_not_a_heading() {
        let text = "# A\n```\n### 3. Fake\n```\n## B\n~~~\n## Fake\n~~~\n### 4. Real\n";
        assert_eq!(
            texts(text),
            [
                (1, "A".to_owned()),
                (2, "B".to_owned()),
                (3, "4. Real".to_owned())
            ]
        );
    }

    #[test]
    fn a_tilde_inside_a_backtick_fence_is_content_and_a_short_closer_does_not_close() {
        let text = "````\n~~~\n```\n# Still inside\n````\n# Out\n";
        assert_eq!(texts(text), [(1, "Out".to_owned())]);
    }

    #[test]
    fn an_unterminated_fence_is_reported_and_hides_the_rest() {
        let l = lines("# A\n```\n# B\n");
        assert_eq!(tokenize_headings(&l).len(), 1);
        let (kept, unterminated) = strip_fenced_code(&l);
        assert!(unterminated);
        assert_eq!(kept, ["# A"]);
        let (_, closed) = strip_fenced_code(&lines("```\nx\n```\n"));
        assert!(!closed);
    }

    #[test]
    fn a_backtick_fence_with_a_backtick_in_its_info_string_is_not_an_opener() {
        assert!(!match_fence_delimiter("```a`b").unwrap().can_open);
        assert!(match_fence_delimiter("~~~a`b").unwrap().can_open);
        assert_eq!(texts("```a`b\n# Heading\n"), [(1, "Heading".to_owned())]);
    }

    #[test]
    fn the_atx_rules_hold() {
        assert_eq!(texts("    # four spaces"), []);
        assert_eq!(texts("   # three spaces"), [(1, "three spaces".to_owned())]);
        assert_eq!(texts("####### seven"), []);
        assert_eq!(texts("#nospace"), []);
        assert_eq!(texts("##"), [(2, String::new())]);
        assert_eq!(texts("## Title ##"), [(2, "Title".to_owned())]);
        assert_eq!(texts("## Title##"), [(2, "Title##".to_owned())]);
        assert_eq!(texts("### ###"), [(3, String::new())]);
        assert_eq!(texts("# CRLF\r"), [(1, "CRLF".to_owned())]);
    }

    #[test]
    fn collect_section_is_level_bounded_or_runs_to_any_heading() {
        let l = lines("## Gaps\nfirst\n### Sub\nsub body\n## Next\nlast\n");
        let bounded = collect_section(&l, |h| h.text == "Gaps", true).unwrap();
        assert_eq!(bounded.body, "first\n### Sub\nsub body");
        let unbounded = collect_section(&l, |h| h.text == "Gaps", false).unwrap();
        assert_eq!(unbounded.body, "first");
        let stopped = collect_section_stopping_at(&l, |h| h.text == "Gaps", 3).unwrap();
        assert_eq!(stopped.body, "first");
        assert!(collect_section(&l, |h| h.text == "Absent", true).is_none());
        let eof = collect_section(&l, |h| h.text == "Next", true).unwrap();
        assert_eq!(eof.body, "last");
    }

    #[test]
    fn fenced_blocks_carry_their_info_string_and_unterminated_flag() {
        let blocks = scan_fenced_blocks(&lines("```rust\nx\n```\n~~~\ny\n"));
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].info, "rust");
        assert_eq!(blocks[0].close_line, Some(2));
        assert_eq!(blocks[1].close_line, None);
        assert_eq!(blocks[1].open_line, 3);
    }

    #[test]
    fn table_cells_split_on_unescaped_pipes_and_unescape() {
        assert_eq!(split_table_row("| a | b \\| c | d |"), ["a", "b | c", "d"]);
        assert_eq!(split_table_row("a|b"), ["a", "b"]);
        assert_eq!(split_table_row("| x \\\\ y |"), ["x \\ y"]);
    }

    #[test]
    fn a_delimiter_row_is_dashes_with_optional_colons() {
        assert!(is_delimiter_row(&split_table_row("|---|:--:|")));
        assert!(is_delimiter_row(&split_table_row("| --- | ---: |")));
        assert!(!is_delimiter_row(&split_table_row("| a | --- |")));
        assert!(!is_delimiter_row(&split_table_row("| - | :: |")));
    }
}
