use std::path::{Path, PathBuf};

use crate::text::Untrusted;

/// One `999.*` directory under a project's `.planning/phases/`.
///
/// **Every text field is [`Untrusted`]** (D-21-19). Each of them is read off
/// disk from a repository the user cloned: `dir_name` is a directory name,
/// `number` and `description` are parsed out of it and out of the first
/// heading of a `.md` file inside it, and `content` is that file's body. None
/// of it was authored by this build, which is SAFE-07's own trust boundary.
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

/// Parse all backlog items from the `.planning/phases/` directory.
/// Reads 999.* directories, extracts number/slug, finds description from first heading.
/// Does NOT load content (leaves it as None).
pub fn parse_backlog_items(planning_dir: &Path) -> Vec<BacklogItem> {
    let phases_dir = planning_dir.join("phases");
    let entries = match std::fs::read_dir(&phases_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut items: Vec<BacklogItem> = entries
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

            // Skip directories with no .md files (empty backlog placeholders)
            let md_path = find_first_md_file(&e.path())?;

            // Try to find description from first .md file's first heading
            let description = find_first_heading(&e.path()).unwrap_or_else(|| humanize_slug(&slug));

            // The ONE place these four values are created, so the ONE place
            // they are wrapped. Everything downstream inherits the carrier.
            Some(BacklogItem {
                dir_name: Untrusted::from_untrusted_source(dir_name),
                number: Untrusted::from_untrusted_source(number),
                description: Untrusted::from_untrusted_source(description),
                content: None,
                path: Some(md_path),
            })
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

/// Load the full content of a backlog item's first .md file.
pub fn load_backlog_content(planning_dir: &Path, dir_name: &str) -> Option<String> {
    let item_dir = planning_dir.join("phases").join(dir_name);
    let md_path = find_first_md_file(&item_dir)?;
    std::fs::read_to_string(md_path).ok()
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
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries.first().map(|e| e.path())
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
}
