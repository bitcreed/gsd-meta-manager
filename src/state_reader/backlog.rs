use std::path::{Path, PathBuf};

use crate::text::Untrusted;

/// One `999.*` directory under a project's `.planning/phases/`.
///
/// **Every text field is [`Untrusted`]** (D-21-19). Each of them is read off
/// disk from a repository the user cloned: `dir_name` is a directory name,
/// `number` and `description` are parsed out of it and out of the first
/// heading of a `.md` file inside it, and `content` is the item's ROADMAP.md
/// entry plus any `.md` bodies ([`load_backlog_content`]). None of it was
/// authored by this build, which is SAFE-07's own trust boundary.
///
/// The carrier is what makes that checkable rather than remembered: it
/// implements no `Display`, no `AsRef<str>`, no `Into<Cow<str>>`, so a render
/// site cannot interpolate one of these fields or hand it to a ratatui sink at
/// all. Retyping the struct is therefore what makes the COMPILER name every
/// consumer, instead of a reader working through a list of sites — which is how
/// `detail.rs:2882`'s raw `format!` survived nine rounds of review.
///
/// `path` stays a `PathBuf`: a path is not display text, and what may be done
/// with one is a different question with a different answer.
#[derive(Debug, Clone)]
pub struct BacklogItem {
    pub dir_name: Untrusted,
    pub number: Untrusted,
    pub description: Untrusted,
    pub content: Option<Untrusted>,
    /// Path to the first .md file in the backlog item directory.
    pub path: Option<PathBuf>,
}

/// Parse a backlog directory name like "999.3-queue-editor-and-reorder" into (number, slug).
/// Skips entries containing `{` or newlines (malformed slugs from broken JSON output).
pub fn parse_backlog_dir_name(name: &str) -> Option<(String, String)> {
    if name.contains('{') || name.contains('\n') {
        return None;
    }
    // Expected format: "999.N-slug-text"
    let rest = name.strip_prefix("999.")?;
    let dash_pos = rest.find('-')?;
    let number = format!("999.{}", &rest[..dash_pos]);
    let slug = rest[dash_pos + 1..].to_string();
    Some((number, slug))
}

/// **The SOLE definition of "which directories under `.planning/phases/` are
/// backlog items"** — returning `(dir_name, number, slug)` for each.
///
/// # Why this exists as one function (260916-vr0)
///
/// It used to be two rules. The overview's count kept any entry whose name
/// merely `starts_with("999")`; the Backlog tab's parser additionally required a
/// `.md` file INSIDE the directory. Two rules kept in agreement by care are two
/// rules that disagree, and these disagreed totally: measured across all six
/// registered projects, 12 of 12 `999.*` directories hold exactly one entry —
/// `.gitkeep` — and ZERO hold a `.md` file. So the overview advertised 4 while
/// the tab drew 0, for every project that had a backlog at all.
///
/// Reconciled toward the STRICT rule (D-INF-02): the count's only job is to say
/// how many rows the tab will draw, so the tab's rule is the correct one. A name
/// [`parse_backlog_dir_name`] rejects is one from which no number and no slug
/// can be extracted, so it could never have been DISPLAYED — and must therefore
/// not be counted either.
///
/// # No file reads
///
/// One `read_dir` and nothing more. This runs on the dashboard refresh path for
/// every registered project; opening files per entry is what the count must
/// never start doing.
pub fn backlog_dirs(planning_dir: &Path) -> Vec<(String, String, String)> {
    let phases_dir = planning_dir.join("phases");
    let entries = match std::fs::read_dir(&phases_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("999"))
                .unwrap_or(false)
                && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
        })
        .filter_map(|e| {
            let dir_name = e.file_name().to_string_lossy().to_string();
            let (number, slug) = parse_backlog_dir_name(&dir_name)?;
            Some((dir_name, number, slug))
        })
        .collect()
}

/// How many backlog items a project has — the SAME rule, and therefore the same
/// number, as the list [`parse_backlog_items`] returns.
pub fn count_backlog_dirs(planning_dir: &Path) -> usize {
    backlog_dirs(planning_dir).len()
}

/// Parse all backlog items from the `.planning/phases/` directory.
/// Matches via [`backlog_dirs`], so the list cannot diverge from the count.
/// Does NOT load content (leaves it as None).
pub fn parse_backlog_items(planning_dir: &Path) -> Vec<BacklogItem> {
    let phases_dir = planning_dir.join("phases");

    let mut items: Vec<BacklogItem> = backlog_dirs(planning_dir)
        .into_iter()
        .map(|(dir_name, number, slug)| {
            let item_dir = phases_dir.join(&dir_name);

            // MEASURED: a backlog item's payload is its DIRECTORY NAME, and a
            // markdown body is an optional enrichment that in practice is never
            // present — 12 of 12 real `999.*` directories contain only
            // `.gitkeep`. So an absent `.md` resolves to `path: None` rather
            // than deciding whether the item exists at all.
            let md_path = find_first_md_file(&item_dir);

            // Description: the first heading when there IS a `.md`, the
            // humanized slug when there is not.
            let description = find_first_heading(&item_dir).unwrap_or_else(|| humanize_slug(&slug));

            // The ONE place these four values are created, so the ONE place
            // they are wrapped. Everything downstream inherits the carrier.
            // This change raises the number of untrusted directory names that
            // reach the renderer from zero to ALL of them (T-vr0-01), which
            // makes the wrapping more load-bearing here, not less.
            BacklogItem {
                dir_name: Untrusted::from_untrusted_source(dir_name),
                number: Untrusted::from_untrusted_source(number),
                description: Untrusted::from_untrusted_source(description),
                content: None,
                path: md_path,
            }
        })
        .collect();

    // Sort by number ascending (999.1, 999.2, etc.)
    items.sort_by(|a, b| {
        // A sort key is a COMPARISON, not something a human reads, so the raw
        // bytes are the right answer here.
        backlog_number_ordering(
            a.number.as_raw_for_logic_only(),
            b.number.as_raw_for_logic_only(),
        )
    });

    items
}

/// The numeric sort key for a backlog item's `999.N` number.
///
/// **The finite filter is not defensive programming — it is the boundary**
/// (WR-07, T-21-29-04). This number is parsed straight out of a
/// `.planning/phases/` directory name, which a third party chose and which is
/// exactly SAFE-07's declared trust boundary. `999.NaN-anything` reaches
/// [`parse_backlog_dir_name`] intact, `"NaN".parse::<f64>()` succeeds, and a
/// `NaN` key reaches the comparator. "Nobody would name a directory that" is
/// not an argument that is available here.
///
/// A non-finite parse therefore falls back to the SAME default a failed parse
/// already used, so the two unrepresentable cases are one case.
fn backlog_sort_key(number: &str) -> f64 {
    number
        .strip_prefix("999.")
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

/// Order two backlog numbers. **A total order by construction.**
///
/// # What was wrong, and what was NOT wrong
///
/// This used to be `partial_cmp(..).unwrap_or(Ordering::Equal)` over an
/// unfiltered `f64`. `NaN` compares `None` against everything, so the fallback
/// made it compare `Equal` to everything — and a relation where `NaN == 1` and
/// `NaN == 2` while `1 < 2` is not transitive. A comparator that is not a total
/// order is outside `slice::sort_by`'s contract, and what comes out is
/// whatever the sort's internal state happened to do.
///
/// **The round-10 review called this a panic. It is not, and that was
/// MEASURED** — verification pass 10, quoted verbatim from
/// `21-VERIFICATION.md`:
///
/// > "I built and ran a standalone Rust program (rustc 1.97.1, matching this
/// > toolchain) sorting a `Vec<f64>` containing multiple `NaN` values with the
/// > exact comparator shape used in `backlog.rs:88-106`
/// > (`partial_cmp(...).unwrap_or(Equal)`), at both small (5-element) and larger
/// > (2000-element, 1/3 NaN) sizes. Neither run panicked; both produced a
/// > silently-wrong order with NaNs interspersed. Rust's stable `slice::sort_by`
/// > does NOT panic on a non-total-order comparator on this toolchain — WR-07's
/// > specific claim ('Rust's current slice::sort_by detects total-order
/// > violations and panics') is not reproducible and is likely incorrect,
/// > possibly confusing Rust with Java's TimSort."
///
/// So the defect is a silently wrong ORDER, and
/// [`a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation`](tests::a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation)
/// asserts the order rather than the absence of a panic. A test written to
/// expect a panic would have failed, and a record repeating the panic claim
/// would have propagated a measured-false statement.
///
/// [`f64::total_cmp`] is a total order over ALL `f64` values including `NaN`, so
/// there is no fallback arm on the KEY comparison. A fallback on a total order
/// is dead code that tells the next reader the order might not be total.
///
/// # What was STILL wrong after that, and what the tiebreak fixes (WR-05)
///
/// The comparison above is total over **keys**. It was not total over
/// **elements**, and that distinction is not academic here: [`backlog_sort_key`]
/// maps every unusable suffix — `999.NaN`, `999.nan`, `999.inf`, `999.x`,
/// `999.` — onto the SAME fallback key. `slice::sort_by` is stable, so a whole
/// group of tied elements kept whatever relative order it was handed, and what
/// hands it that order is `read_dir`. The Backlog tab's display order was
/// therefore still a function of the filesystem for exactly those entries —
/// **which is the defect this function's own failure message describes**, still
/// present in the function that describes it.
///
/// Appending a tiebreak on the ELEMENT closes it: when two keys compare equal,
/// the numbers themselves decide. The order is now total over elements, it is
/// reproducible from any input permutation, and
/// [`tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order`](tests::tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order)
/// asserts it — observed RED against the pre-tiebreak body, whose red is quoted
/// on that test.
///
/// **Scope: display ordering only.** No persisted artifact and no on-disk
/// format depends on this order; it decides the sequence of rows the Backlog
/// tab draws. Entries with distinct numeric keys are unaffected, which
/// [`well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix`](tests::well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix)
/// pins.
fn backlog_number_ordering(a: &str, b: &str) -> std::cmp::Ordering {
    backlog_sort_key(a)
        .total_cmp(&backlog_sort_key(b))
        .then_with(|| a.cmp(b))
}

/// Load what the Backlog tab's content pane shows for one item: its
/// ROADMAP.md entry, followed by every `.md` file in its directory.
///
/// # Where a backlog item's content actually lives (debug backlog-content-empty)
///
/// GSD's backlog capture (`gsd-core/workflows/add-backlog.md`, run by
/// `/gsd-capture --backlog`) writes the item ONLY into ROADMAP.md — a
/// `### Phase 999.N: <description> (BACKLOG)` section under `## Backlog`
/// carrying `**Goal:**`, `**Requirements:**`, `**Plans:**` and any prose — and
/// creates `.planning/phases/999.N-<slug>/` holding nothing but `.gitkeep`, so
/// that `/gsd-discuss-phase` and `/gsd-plan-phase` have somewhere to write.
/// Measured 2026-09-24 over every registered project: 11 of 11 `999.*`
/// directories hold only `.gitkeep`. This used to read the directory alone, so
/// every item in every project drew "Empty — no .md files".
///
/// So the ROADMAP.md section ([`super::roadmap_md::phase_section`], matched on
/// the item's `999.N`) comes first; then each `.md` file the directory has
/// accumulated (a `/gsd-discuss-phase` CONTEXT.md, a RESEARCH.md, a
/// hand-written note), in name order, each under a `── <file name> ──` label
/// so the reader can tell the sources apart. `None` only when there is neither.
///
/// The result is raw text read off disk; the caller wraps it as untrusted.
pub fn load_backlog_content(planning_dir: &Path, dir_name: &str) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    let roadmap_entry = parse_backlog_dir_name(dir_name).and_then(|(number, _)| {
        let roadmap = std::fs::read_to_string(planning_dir.join("ROADMAP.md")).ok()?;
        super::roadmap_md::phase_section(&roadmap, &number)
    });
    parts.extend(roadmap_entry);

    let item_dir = planning_dir.join("phases").join(dir_name);
    for md_path in md_files(&item_dir) {
        let Ok(body) = std::fs::read_to_string(&md_path) else {
            continue;
        };
        let name = md_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        parts.push(format!("── {name} ──\n\n{}", body.trim_end()));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// The file the Backlog tab's edit key opens for one item, and the 1-based
/// line to open it at (quick-260924-drx).
///
/// **Where the item actually lives wins:** GSD's backlog capture writes the
/// item into ROADMAP.md (see [`load_backlog_content`]), so when that section
/// exists the target is `ROADMAP.md` at the section's heading line. Only an
/// item with no ROADMAP.md entry falls back to `md_path` — the first `.md` in
/// its directory — opened at the top. `None` when there is neither.
///
/// INFERRED: ROADMAP.md is preferred even when a directory `.md` also exists;
/// those files are secondary `/gsd-discuss-phase` / research artifacts, and the
/// content pane shows the ROADMAP.md section first.
pub fn backlog_edit_target(
    planning_dir: &Path,
    dir_name: &str,
    md_path: Option<&Path>,
) -> Option<(PathBuf, Option<usize>)> {
    let roadmap_path = planning_dir.join("ROADMAP.md");
    let roadmap_line = parse_backlog_dir_name(dir_name).and_then(|(number, _)| {
        let roadmap = std::fs::read_to_string(&roadmap_path).ok()?;
        super::roadmap_md::phase_section_line(&roadmap, &number)
    });
    match roadmap_line {
        Some(line) => Some((roadmap_path, Some(line))),
        None => md_path.map(|p| (p.to_path_buf(), None)),
    }
}

/// Find the first .md file in a directory and extract its first `# heading`.
fn find_first_heading(dir: &Path) -> Option<String> {
    let md_path = find_first_md_file(dir)?;
    let content = std::fs::read_to_string(md_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

/// Find the first .md file in a directory (alphabetically).
fn find_first_md_file(dir: &Path) -> Option<std::path::PathBuf> {
    md_files(dir).into_iter().next()
}

/// Every `.md` file in a directory, sorted by name. Empty when the directory
/// cannot be read.
fn md_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = read
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries.into_iter().map(|e| e.path()).collect()
}

/// Convert a slug like "queue-editor-and-reorder" to "Queue editor and reorder".
fn humanize_slug(slug: &str) -> String {
    let mut s = slug.replace('-', " ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backlog_dir_name_valid() {
        let result = parse_backlog_dir_name("999.3-queue-editor-and-reorder");
        assert_eq!(
            result,
            Some(("999.3".to_string(), "queue-editor-and-reorder".to_string()))
        );
    }

    #[test]
    fn test_parse_backlog_dir_name_with_brace() {
        assert_eq!(
            parse_backlog_dir_name("999.1-{\n  \"slug\": \"test\"\n}"),
            None
        );
    }

    #[test]
    fn test_parse_backlog_dir_name_no_prefix() {
        assert_eq!(parse_backlog_dir_name("05-state-reader"), None);
    }

    #[test]
    fn test_humanize_slug() {
        assert_eq!(
            humanize_slug("queue-editor-and-reorder"),
            "Queue editor and reorder"
        );
    }

    /// Sort `names` with the LIVE comparator, from a given starting rotation.
    fn sorted_from(names: &[&'static str], rotate: usize) -> Vec<&'static str> {
        let mut values: Vec<&'static str> = names.to_vec();
        values.rotate_left(rotate % names.len().max(1));
        values.sort_by(|a, b| backlog_number_ordering(a, b));
        values
    }

    /// The sorted sequence of KEYS, formatted so `NaN` is comparable at all
    /// (`NaN != NaN`, so a `Vec<f64>` equality would be vacuously false under
    /// the pre-fix comparator and this control would have no teeth).
    fn sorted_keys_from(names: &[&'static str], rotate: usize) -> Vec<String> {
        sorted_from(names, rotate)
            .iter()
            .map(|name| format!("{:?}", backlog_sort_key(name)))
            .collect()
    }

    /// **WR-07, asserted on the ORDER — not on the absence of a panic**
    /// (T-21-29-04, D-21-45).
    ///
    /// `.planning/phases/` directory names are chosen by whoever wrote the
    /// repository the operator cloned, which is exactly SAFE-07's declared trust
    /// boundary, so `999.NaN-something` is an INPUT. It parses cleanly:
    /// `parse_backlog_dir_name` yields `("999.NaN", "something")` and
    /// `"NaN".parse::<f64>()` succeeds.
    ///
    /// Under the pre-fix comparator that `NaN` compared `Equal` to every other
    /// key, which is not transitive, and the Backlog tab's order became a
    /// function of the order `read_dir` happened to return entries in. This
    /// sorts the SAME set from every starting rotation and requires one answer.
    ///
    /// **The committed RED, verbatim**, produced by this test against the
    /// pre-fix `partial_cmp(..).unwrap_or(Equal)` body:
    ///
    /// ```text
    /// thread 'state_reader::backlog::tests::a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation' (2153211) panicked at src/state_reader/backlog.rs:299:13:
    /// assertion `left == right` failed: sorting the same set from rotation 1 produced ["999.NaN", "999.1", "999.2", "999.3", "999.4", "999.10"], but from rotation 0 it produced ["999.1", "999.NaN", "999.2", "999.3", "999.4", "999.10"]. A comparator whose answer depends on the input permutation is not a total order, and the Backlog tab's order is then a function of whatever `read_dir` happened to return first.
    ///   left: ["999.NaN", "999.1", "999.2", "999.3", "999.4", "999.10"]
    ///  right: ["999.1", "999.NaN", "999.2", "999.3", "999.4", "999.10"]
    /// ```
    ///
    /// Two different answers for the same six directories, and neither run
    /// complained. That is the defect: a wrong ORDER, silently.
    ///
    /// Note what that output is and is not: a WRONG ORDER, quietly, and not a
    /// panic. Verification pass 10 measured the panic claim false on this
    /// toolchain; [`super::backlog_number_ordering`]'s doc quotes the
    /// measurement.
    #[test]
    fn a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation() {
        // One non-numeric entry, so every key in the set is DISTINCT and the
        // whole element sequence is therefore permutation-independent.
        let distinct = ["999.1", "999.NaN", "999.2", "999.3", "999.10", "999.4"];
        let reference = sorted_from(&distinct, 0);
        for rotate in 1..distinct.len() {
            assert_eq!(
                sorted_from(&distinct, rotate),
                reference,
                "sorting the same set from rotation {rotate} produced {:?}, but \
                 from rotation 0 it produced {reference:?}. A comparator whose \
                 answer depends on the input permutation is not a total order, \
                 and the Backlog tab's order is then a function of whatever \
                 `read_dir` happened to return first.",
                sorted_from(&distinct, rotate)
            );
        }
        assert_eq!(
            reference,
            vec!["999.NaN", "999.1", "999.2", "999.3", "999.4", "999.10"],
            "a non-finite suffix must sort as its documented fallback key, \
             deterministically, rather than wherever the sort's internal state \
             leaves it"
        );

        // **Three non-numeric entries, which TIE on the KEY.** This block
        // asserts the sequence of KEYS, which is the property the WR-07 total
        // -order fix delivered, and it is what goes red for the pre-fix
        // comparator — under which `999.1` itself moved.
        //
        // **Corrected by 21-33 (WR-05).** This comment used to argue that the
        // element sequence could not be asserted, because `slice::sort_by` is
        // stable and rotating the input therefore reorders a tied group. That
        // was true of the comparator as it stood, and it was precisely the
        // defect: the elements' order was inherited from `read_dir`. It is no
        // longer true. `backlog_number_ordering` now appends a tiebreak on the
        // element, so tied keys are ordered by the numbers themselves and the
        // ELEMENT sequence is permutation-independent too — asserted by
        // `tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order`.
        // The key-sequence assertion is kept here rather than widened, so the
        // two controls stay separable: this one fails if the KEY order regresses,
        // that one fails if the tiebreak is removed.
        let with_ties = [
            "999.1", "999.NaN", "999.2", "999.3", "999.NaN", "999.10", "999.nan", "999.4",
        ];
        let reference_keys = sorted_keys_from(&with_ties, 0);
        for rotate in 1..with_ties.len() {
            assert_eq!(
                sorted_keys_from(&with_ties, rotate),
                reference_keys,
                "the sorted sequence of KEYS changed with the input permutation \
                 (rotation {rotate}), which no total order permits"
            );
        }
        assert_eq!(
            reference_keys,
            vec!["0.0", "0.0", "0.0", "1.0", "2.0", "3.0", "4.0", "10.0"],
            "every non-finite suffix takes the fallback key and the finite ones \
             ascend; a `NaN` surviving into this list would mean the finite \
             filter stopped filtering"
        );
    }

    /// **WR-05: the order is total over ELEMENTS, not merely over KEYS.**
    ///
    /// The control above asserts that the sequence of KEYS is
    /// permutation-independent, and deliberately does not assert the element
    /// sequence for the tied group — because before the tiebreak that would
    /// have been asserting a falsehood. That gap IS the defect verification
    /// pass 11 recorded as WR-05: every unusable suffix maps to the same
    /// fallback key, `slice::sort_by` is stable, so tied ELEMENTS kept
    /// whatever order `read_dir` handed them in. The Backlog tab's order was
    /// still a function of the filesystem — the very thing
    /// [`super::backlog_number_ordering`]'s own failure message describes as
    /// the defect.
    ///
    /// This asserts the ELEMENT sequence, which is what an operator actually
    /// sees, over two entries whose keys tie and whose names do not.
    ///
    /// **The committed RED, verbatim**, produced by this test against the
    /// pre-tiebreak comparator body
    /// (`backlog_sort_key(a).total_cmp(&backlog_sort_key(b))`, with no
    /// `.then_with(..)` arm):
    ///
    /// ```text
    /// thread 'state_reader::backlog::tests::tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order' (2961201) panicked at src/state_reader/backlog.rs:370:13:
    /// assertion `left == right` failed: two items whose keys tie came out in a different ELEMENT order from rotation 1 than from rotation 0 (["999.alpha", "999.zebra"] vs ["999.zebra", "999.alpha"]). The key sequence being permutation-independent is not enough: the operator reads elements, and a tied group that inherits `read_dir`'s order is the filesystem deciding the display order.
    ///   left: ["999.alpha", "999.zebra"]
    ///  right: ["999.zebra", "999.alpha"]
    /// ```
    ///
    /// Two entries, two rotations, two different answers — and nothing
    /// complained. That is what "total over keys but not over elements" costs
    /// an operator.
    #[test]
    fn tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order() {
        // Both suffixes are unusable, so both take the same fallback key. The
        // names are chosen so alphabetical order is the REVERSE of the input
        // order, which is what makes rotation 0 and rotation 1 disagree under
        // a comparator that is total only over keys.
        let tied = ["999.zebra", "999.alpha"];
        assert_eq!(
            backlog_sort_key(tied[0]),
            backlog_sort_key(tied[1]),
            "the fixture is only meaningful if the two keys actually TIE; if \
             these ever differ, this control is exercising the numeric path \
             and proves nothing about tied elements"
        );

        let reference = sorted_from(&tied, 0);
        for rotate in 1..tied.len() {
            assert_eq!(
                sorted_from(&tied, rotate),
                reference,
                "two items whose keys tie came out in a different ELEMENT order \
                 from rotation {rotate} than from rotation 0 ({:?} vs \
                 {reference:?}). The key sequence being permutation-independent \
                 is not enough: the operator reads elements, and a tied group \
                 that inherits `read_dir`'s order is the filesystem deciding \
                 the display order.",
                sorted_from(&tied, rotate)
            );
        }
        assert_eq!(
            reference,
            vec!["999.alpha", "999.zebra"],
            "tied keys must fall back to comparing the elements themselves, so \
             the order is total over elements and reproducible"
        );
    }

    /// **A total order, asserted as one** — antisymmetry and transitivity swept
    /// over every pair and triple of a fixture set that includes the hostile
    /// input. This is what "total order by construction" means, and it is the
    /// property `partial_cmp(..).unwrap_or(Equal)` did not have:
    /// `NaN == 1` and `NaN == 2` while `1 < 2`.
    #[test]
    fn the_backlog_comparator_is_antisymmetric_and_transitive_including_over_a_nan_suffix() {
        use std::cmp::Ordering;
        let names = [
            "999.1", "999.NaN", "999.2", "999.10", "999.nan", "999.x", "999.inf", "999.0",
        ];

        for a in names {
            assert_eq!(
                backlog_number_ordering(a, a),
                Ordering::Equal,
                "{a:?} must compare equal to itself"
            );
            for b in names {
                assert_eq!(
                    backlog_number_ordering(a, b),
                    backlog_number_ordering(b, a).reverse(),
                    "antisymmetry failed for ({a:?}, {b:?})"
                );
            }
        }

        for a in names {
            for b in names {
                for c in names {
                    let (ab, bc, ac) = (
                        backlog_number_ordering(a, b),
                        backlog_number_ordering(b, c),
                        backlog_number_ordering(a, c),
                    );
                    if ab == Ordering::Less && bc == Ordering::Less {
                        assert_eq!(
                            ac,
                            Ordering::Less,
                            "transitivity failed: {a:?} < {b:?} < {c:?} but \
                             {a:?} vs {c:?} is {ac:?}"
                        );
                    }
                    if ab == Ordering::Equal && bc == Ordering::Equal {
                        assert_eq!(
                            ac,
                            Ordering::Equal,
                            "transitivity of equality failed: {a:?} == {b:?} == \
                             {c:?} but {a:?} vs {c:?} is {ac:?}. This is the \
                             exact shape the pre-fix comparator had — `NaN` \
                             compared Equal to everything while the finite keys \
                             around it did not compare equal to each other."
                        );
                    }
                }
            }
        }
    }

    /// The no-regression direction. A fix that reorders ordinary backlog items
    /// is a user-visible change this plan does not intend, and an order test
    /// that only exercised the hostile input could not see it.
    #[test]
    fn well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix() {
        let names = ["999.10", "999.2", "999.1", "999.3", "999.21"];
        assert_eq!(
            sorted_from(&names, 0),
            vec!["999.1", "999.2", "999.3", "999.10", "999.21"],
            "well-formed items are ordered by their numeric value, exactly as \
             they were before the comparator changed"
        );
        for rotate in 1..names.len() {
            assert_eq!(sorted_from(&names, rotate), sorted_from(&names, 0));
        }
    }

    /// The finite filter and the failed-parse fallback are ONE case, asserted
    /// rather than left to the reader of `backlog_sort_key`'s body.
    #[test]
    fn a_non_finite_or_unparsable_suffix_takes_the_same_fallback_key() {
        for unusable in ["999.NaN", "999.nan", "999.inf", "999.-inf", "999.x", "999."] {
            assert_eq!(
                backlog_sort_key(unusable),
                0.0,
                "{unusable:?} cannot name a position on the number line, so it \
                 must take the same fallback as a suffix that does not parse at \
                 all — two unrepresentable cases, one answer"
            );
        }
        assert_eq!(backlog_sort_key("999.7"), 7.0);
        assert_eq!(backlog_sort_key("999.10"), 10.0);
    }

    // --- debug backlog-content-empty: where a backlog item's content lives ---
    //
    // GSD's `/gsd-capture --backlog` (`gsd-core/workflows/add-backlog.md`)
    // writes the item as a `### Phase 999.N: … (BACKLOG)` section under
    // ROADMAP.md's `## Backlog` and creates `.planning/phases/999.N-<slug>/`
    // holding ONLY `.gitkeep`. Measured 2026-09-24 across every registered
    // project: 11 of 11 `999.*` directories hold only `.gitkeep`, so a loader
    // that reads the directory alone shows every item as empty.

    /// The ROADMAP.md of [`backlog_fixture`] — sanitized text, shaped like a
    /// real GSD roadmap: an ordinary phase, a Progress table, a `## Backlog`
    /// section whose second entry carries a non-`(BACKLOG)` suffix and a
    /// blockquote, and a `---` footer after the last entry.
    const FIXTURE_ROADMAP: &str = "\
# Roadmap: sample

## Phases

- [x] **Phase 1: Foundation** - base

### Phase 1: Foundation

**Goal:** Build the base.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation | 1/1 | Complete | 2026-01-01 |

## Backlog

### Phase 999.1: Alternate transport fallback (BACKLOG)

**Goal:** [Captured for future planning] Read the device another way when the
primary transport is unavailable.
**Requirements:** REQ-05, REQ-06
**Plans:** 0 plans

**Why deferred:** measure the primary path first.

- **Slower.** The fallback reads at a fraction of the primary rate.

Plans:

- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.2: Desktop notification with live progress (PROMOTED AND DELIVERED)

> **DELIVERED by a quick task.** Kept as the capture that was acted on.

**Goal:** [Captured for future planning] Show one notification that updates in place.
**Requirements:** TBD
**Plans:** 0 plans

---
*Roadmap created: 2026-01-01*
*Phase 999.2 added: 2026-01-02*
";

    /// A `.planning/` mirroring a real project's backlog layout: two `999.x`
    /// directories each holding only `.gitkeep`, and their content in
    /// ROADMAP.md. Returns the `TempDir` (hold it) and the planning dir.
    fn backlog_fixture() -> (tempfile::TempDir, std::path::PathBuf) {
        let td = tempfile::TempDir::new().expect("temp dir");
        let planning = td.path().join(".planning");
        for dir in [
            "999.1-alternate-transport-fallback",
            "999.2-desktop-notification-live-progress",
        ] {
            let item = planning.join("phases").join(dir);
            std::fs::create_dir_all(&item).unwrap();
            std::fs::write(item.join(".gitkeep"), "").unwrap();
        }
        std::fs::write(planning.join("ROADMAP.md"), FIXTURE_ROADMAP).unwrap();
        (td, planning)
    }

    /// quick-260924-drx: the edit key opens ROADMAP.md at the item's heading —
    /// the file the item lives in — even when a directory `.md` also exists.
    #[test]
    fn the_edit_target_is_the_roadmap_section_line_when_one_exists() {
        let (_td, planning) = backlog_fixture();
        let dir = "999.2-desktop-notification-live-progress";
        let dir_md = planning.join("phases").join(dir).join("999.2-CONTEXT.md");
        std::fs::write(&dir_md, "# Context\n").unwrap();

        let (path, line) = backlog_edit_target(&planning, dir, Some(&dir_md))
            .expect("the item has a ROADMAP.md section");
        assert_eq!(path, planning.join("ROADMAP.md"));
        let line = line.expect("a heading line");
        assert!(
            FIXTURE_ROADMAP
                .lines()
                .nth(line - 1)
                .is_some_and(|l| l.starts_with("### Phase 999.2")),
            "line {line} is not the 999.2 heading"
        );
    }

    /// No ROADMAP.md entry: the directory `.md` at the top; neither: `None`.
    #[test]
    fn the_edit_target_falls_back_to_the_dir_md_then_to_none() {
        let (_td, planning) = backlog_fixture();
        std::fs::write(planning.join("ROADMAP.md"), "# Roadmap\n").unwrap();
        let dir = "999.1-alternate-transport-fallback";
        let dir_md = planning.join("phases").join(dir).join("999.1-CONTEXT.md");

        assert_eq!(
            backlog_edit_target(&planning, dir, Some(&dir_md)),
            Some((dir_md.clone(), None))
        );
        assert_eq!(backlog_edit_target(&planning, dir, None), None);
    }

    /// The reported symptom: a `.gitkeep`-only backlog directory loaded
    /// nothing, and the pane drew "Empty — no .md files in this backlog
    /// directory" for every item in every project.
    #[test]
    fn a_gitkeep_only_backlog_item_loads_its_roadmap_section() {
        let (_td, planning) = backlog_fixture();

        let content = load_backlog_content(&planning, "999.1-alternate-transport-fallback")
            .expect("the item's ROADMAP.md section is its content");

        assert!(
            content.starts_with("### Phase 999.1: Alternate transport fallback (BACKLOG)"),
            "the section opens with its own heading: {content:?}"
        );
        for body in [
            "**Goal:** [Captured for future planning] Read the device another way",
            "**Requirements:** REQ-05, REQ-06",
            "**Why deferred:** measure the primary path first.",
            "- [ ] TBD (promote with /gsd-review-backlog when ready)",
        ] {
            assert!(content.contains(body), "missing {body:?} in {content:?}");
        }
        assert!(
            !content.contains("Phase 999.2") && !content.contains("## Backlog"),
            "the section ends at the next heading: {content:?}"
        );
    }

    /// The heading suffix is not always `(BACKLOG)` — a delivered item keeps
    /// its section with another marker — and the last entry must stop at the
    /// thematic break, not run into the document footer.
    #[test]
    fn the_last_backlog_section_stops_before_the_document_footer() {
        let (_td, planning) = backlog_fixture();

        let content = load_backlog_content(&planning, "999.2-desktop-notification-live-progress")
            .expect("a PROMOTED-AND-DELIVERED item still has its section");

        assert!(content.contains("(PROMOTED AND DELIVERED)"), "{content:?}");
        assert!(
            content.contains("> **DELIVERED by a quick task.**"),
            "{content:?}"
        );
        assert!(content.contains("**Plans:** 0 plans"), "{content:?}");
        assert!(
            !content.contains("Roadmap created") && !content.contains("---"),
            "the footer after the thematic break is not part of the item: {content:?}"
        );
    }

    /// Accumulated artifacts — what `/gsd-discuss-phase 999.N` writes into the
    /// directory — follow the ROADMAP section, each under its file name, in
    /// name order.
    #[test]
    fn md_files_in_the_backlog_dir_follow_the_roadmap_section_in_name_order() {
        let (_td, planning) = backlog_fixture();
        let item = planning.join("phases/999.1-alternate-transport-fallback");
        std::fs::write(
            item.join("999.1-RESEARCH.md"),
            "# Research\n\nMeasured rates.\n",
        )
        .unwrap();
        std::fs::write(
            item.join("999.1-CONTEXT.md"),
            "# Context\n\nDecisions so far.\n",
        )
        .unwrap();

        let content = load_backlog_content(&planning, "999.1-alternate-transport-fallback")
            .expect("section plus files");

        let at = |needle: &str| {
            content
                .find(needle)
                .unwrap_or_else(|| panic!("missing {needle:?} in {content:?}"))
        };
        assert!(at("### Phase 999.1:") < at("999.1-CONTEXT.md"));
        assert!(at("999.1-CONTEXT.md") < at("Decisions so far."));
        assert!(at("Decisions so far.") < at("999.1-RESEARCH.md"));
        assert!(at("999.1-RESEARCH.md") < at("Measured rates."));
        assert!(!content.contains(".gitkeep"), "only .md files are content");
    }

    /// A directory holding a `.md` file still shows it when ROADMAP.md has no
    /// entry for the item (or no ROADMAP.md exists at all).
    #[test]
    fn md_files_alone_load_when_the_roadmap_has_no_entry() {
        let (_td, planning) = backlog_fixture();
        std::fs::remove_file(planning.join("ROADMAP.md")).unwrap();
        let item = planning.join("phases/999.1-alternate-transport-fallback");
        std::fs::write(
            item.join("999.1-BACKLOG.md"),
            "# Queue editor\n\nMake it reorderable.\n",
        )
        .unwrap();

        let content = load_backlog_content(&planning, "999.1-alternate-transport-fallback")
            .expect("the .md file is content on its own");
        assert!(content.contains("Make it reorderable."), "{content:?}");
        assert!(content.contains("999.1-BACKLOG.md"), "{content:?}");
    }

    /// Truly empty — no ROADMAP entry (e.g. the entry was removed on promotion
    /// and the directory left behind) and no `.md` file — stays `None`, so the
    /// pane's empty state is still reachable.
    #[test]
    fn an_item_with_neither_a_roadmap_entry_nor_md_files_is_empty() {
        let (_td, planning) = backlog_fixture();
        let orphan = planning.join("phases/999.3-left-behind-after-promotion");
        std::fs::create_dir_all(&orphan).unwrap();
        std::fs::write(orphan.join(".gitkeep"), "").unwrap();

        assert_eq!(
            load_backlog_content(&planning, "999.3-left-behind-after-promotion"),
            None
        );
    }

    /// Boundary neighbours of the number match: `999.1` must not take
    /// `999.10`'s section, nor `999.10` take `999.1`'s — whichever is written
    /// first.
    #[test]
    fn a_backlog_number_matches_its_own_section_not_a_numeric_neighbour() {
        let td = tempfile::TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        for dir in ["999.1-one", "999.10-ten"] {
            std::fs::create_dir_all(planning.join("phases").join(dir)).unwrap();
        }
        std::fs::write(
            planning.join("ROADMAP.md"),
            "## Backlog\n\n### Phase 999.10: Ten (BACKLOG)\n\nTEN BODY\n\n\
             ### Phase 999.1: One (BACKLOG)\n\nONE BODY\n",
        )
        .unwrap();

        let one = load_backlog_content(&planning, "999.1-one").expect("999.1 has a section");
        assert!(
            one.contains("ONE BODY") && !one.contains("TEN BODY"),
            "{one:?}"
        );
        let ten = load_backlog_content(&planning, "999.10-ten").expect("999.10 has a section");
        assert!(
            ten.contains("TEN BODY") && !ten.contains("ONE BODY"),
            "{ten:?}"
        );
    }
}
