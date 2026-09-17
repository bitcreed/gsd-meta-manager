//! Execution waves, derived from the place GSD actually records them: the
//! `wave:` key in each `*-PLAN.md`'s leading frontmatter.
//!
//! `/gsd-execute-phase` assigns each plan to a wave and writes that number into
//! the plan's own frontmatter, so the frontmatter is the authoritative record of
//! which plans ran concurrently and which were serialized. A `waves.json`
//! manifest exists only under GSD 1.8.0's claude-orchestration backend and is
//! absent from most projects; frontmatter is present in every plan this
//! repository has measured (116 of 116).
//!
//! Derivation happens in the refresh-cadence scan
//! ([`crate::state_reader::disk_status::infer_disk_status`]), never at render
//! time, and it reuses the file content that scan already read for its
//! superseded check — so it costs no additional file IO anywhere.

use super::disk_status::leading_frontmatter_value;

/// One execution wave: the plans GSD assigned to the same `wave:` number.
///
/// `wave` is `None` for the trailing bucket of plans that carry no readable
/// wave number. That bucket is *not* a wave — it records plans whose wave is
/// unknown — which is why [`PlanWave::label`] spells it differently and why
/// callers counting parallelism must exclude it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanWave {
    pub wave: Option<u32>,
    pub plans: Vec<String>,
}

impl PlanWave {
    /// A short display label: `w2` for a known wave, `w?` for the unknown bucket.
    ///
    /// **Formats the stored number, never a position.** The renderer iterates
    /// waves with an index and the manifest's own `WaveEntry::label` falls back
    /// to index-plus-one, so a phase whose plans ran in waves 1 and 3 would
    /// otherwise render as waves 1 and 2 — a label that quietly contradicts the
    /// frontmatter it came from.
    pub fn label(&self) -> String {
        match self.wave {
            Some(n) => format!("w{n}"),
            None => "w?".to_string(),
        }
    }
}

/// The `wave:` number declared in a plan file's leading frontmatter.
///
/// Delegates the block anchoring to
/// [`crate::state_reader::disk_status::leading_frontmatter_value`] rather than
/// re-implementing it: that function's byte-zero block anchor and column-zero
/// key match are load-bearing (a nested `wave:` under another mapping key is a
/// different key), and a second, divergent copy is precisely the defect its doc
/// comment records.
///
/// Non-numeric, negative, empty, absent and out-of-range values all yield
/// `None`; the plan then falls into the unknown bucket instead of panicking or
/// inventing a number.
pub fn plan_wave_number(content: &str) -> Option<u32> {
    leading_frontmatter_value(content, "wave")?.parse::<u32>().ok()
}

/// Group `(plan id, wave number)` pairs into waves for display.
///
/// Numbered waves come first in ascending numeric order, each with its plan ids
/// sorted; plans carrying no number collect into exactly one trailing
/// `PlanWave` whose `wave` is `None`.
///
/// **When no entry carries a number the result is empty, deliberately.** The
/// absence of wave metadata is reported as nothing to draw, never as a single
/// anonymous bucket: the renderer would otherwise print a "Waves" heading over
/// a flat list of every plan in the phase, which is the plan list the user
/// already has one pane away. This is also the graceful degradation a project
/// whose GSD version predates wave assignment depends on.
///
/// **The sorting is what makes the result stable across refreshes.** Directory
/// iteration order is unspecified, `DiskInference` derives `PartialEq`, and
/// that comparison drives the dashboard's unchanged-state suppression (quick
/// task 260512-eyv) — an unsorted vector would make a project flap as
/// `Updated` on every refresh.
pub fn group_into_waves(entries: Vec<(String, Option<u32>)>) -> Vec<PlanWave> {
    let mut numbered: std::collections::BTreeMap<u32, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut unnumbered: Vec<String> = Vec::new();
    for (id, wave) in entries {
        match wave {
            Some(n) => numbered.entry(n).or_default().push(id),
            None => unnumbered.push(id),
        }
    }
    if numbered.is_empty() {
        return Vec::new();
    }
    let mut waves: Vec<PlanWave> = numbered
        .into_iter()
        .map(|(wave, mut plans)| {
            plans.sort();
            PlanWave {
                wave: Some(wave),
                plans,
            }
        })
        .collect();
    if !unnumbered.is_empty() {
        unnumbered.sort();
        waves.push(PlanWave {
            wave: None,
            plans: unnumbered,
        });
    }
    waves
}

#[cfg(test)]
mod tests {
    use super::super::disk_status::infer_disk_status;
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn plan_file(wave: &str) -> String {
        format!("---\nphase: 5\nplan: 01\nwave: {wave}\n---\n\nbody\n")
    }

    #[test]
    fn test_plan_waves_end_to_end_from_frontmatter_without_a_manifest() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), plan_file("1")).unwrap();
        fs::write(dir.path().join("05-02-beta-PLAN.md"), plan_file("2")).unwrap();
        assert!(!dir.path().join("waves.json").exists());

        let inf = infer_disk_status(dir.path());
        assert_eq!(
            inf.plan_waves,
            vec![
                PlanWave {
                    wave: Some(1),
                    plans: vec!["05-01-alpha".to_string()],
                },
                PlanWave {
                    wave: Some(2),
                    plans: vec!["05-02-beta".to_string()],
                },
            ]
        );
    }

    #[test]
    fn test_plan_waves_label_uses_the_stored_number() {
        assert_eq!(
            PlanWave {
                wave: Some(3),
                plans: vec![],
            }
            .label(),
            "w3"
        );
    }

    #[test]
    fn test_plan_waves_absent_everywhere_group_into_nothing() {
        // The degradation the whole feature rests on: a project (or a GSD
        // version) that records no waves draws no wave section, rather than one
        // anonymous bucket holding every plan in the phase.
        let grouped = group_into_waves(vec![
            ("05-01".to_string(), None),
            ("05-02".to_string(), None),
        ]);
        assert_eq!(grouped, Vec::new());
    }

    #[test]
    fn test_plan_waves_unnumbered_plans_collect_into_one_trailing_bucket() {
        let grouped = group_into_waves(vec![
            ("05-03".to_string(), None),
            ("05-01".to_string(), Some(1)),
            ("05-04".to_string(), None),
            ("05-02".to_string(), Some(2)),
        ]);
        assert_eq!(
            grouped,
            vec![
                PlanWave {
                    wave: Some(1),
                    plans: vec!["05-01".to_string()],
                },
                PlanWave {
                    wave: Some(2),
                    plans: vec!["05-02".to_string()],
                },
                PlanWave {
                    wave: None,
                    plans: vec!["05-03".to_string(), "05-04".to_string()],
                },
            ]
        );
        assert_eq!(grouped[2].label(), "w?");
    }

    #[test]
    fn test_plan_waves_non_contiguous_numbers_keep_their_own_labels() {
        // Waves 1 and 3 must never render as 1 and 2: the label is the stored
        // number, not the entry's position.
        let grouped = group_into_waves(vec![
            ("05-02".to_string(), Some(3)),
            ("05-01".to_string(), Some(1)),
        ]);
        let labels: Vec<String> = grouped.iter().map(PlanWave::label).collect();
        assert_eq!(labels, vec!["w1".to_string(), "w3".to_string()]);
    }

    #[test]
    fn test_plan_waves_within_a_wave_are_sorted_whatever_order_the_scan_gave() {
        let forward = group_into_waves(vec![
            ("05-01".to_string(), Some(1)),
            ("05-02".to_string(), Some(1)),
        ]);
        let reversed = group_into_waves(vec![
            ("05-02".to_string(), Some(1)),
            ("05-01".to_string(), Some(1)),
        ]);
        assert_eq!(forward, reversed);
        assert_eq!(
            forward[0].plans,
            vec!["05-01".to_string(), "05-02".to_string()]
        );
    }

    #[test]
    fn test_plan_waves_reject_values_that_are_not_a_wave_number() {
        assert_eq!(plan_wave_number(&plan_file("2")), Some(2));
        assert_eq!(plan_wave_number(&plan_file("not-a-number")), None);
        assert_eq!(plan_wave_number(&plan_file("-1")), None);
        assert_eq!(plan_wave_number(&plan_file("")), None);
        assert_eq!(plan_wave_number(&plan_file("99999999999999999999")), None);
        assert_eq!(plan_wave_number("---\nphase: 5\n---\n"), None);
        assert_eq!(plan_wave_number(""), None);
    }

    #[test]
    fn test_plan_waves_inherit_the_frontmatter_anchoring_rules() {
        // A nested key is a different key (WR-05) ...
        assert_eq!(
            plan_wave_number("---\nexecution:\n  wave: 2\n---\n"),
            None
        );
        // ... and nothing below the closing marker is frontmatter at all.
        assert_eq!(plan_wave_number("---\nphase: 5\n---\nwave: 2\n"), None);
        // ... and a file that does not open with the block marker has none.
        assert_eq!(plan_wave_number("# Title\n---\nwave: 2\n---\n"), None);
    }

    #[test]
    fn test_plan_waves_skip_a_superseded_plan_exactly_as_plan_count_does() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), plan_file("1")).unwrap();
        fs::write(
            dir.path().join("05-02-beta-PLAN.md"),
            "---\nstatus: superseded\nwave: 2\n---\n",
        )
        .unwrap();

        let inf = infer_disk_status(dir.path());
        assert_eq!(inf.plan_count, 1);
        assert_eq!(
            inf.plan_waves,
            vec![PlanWave {
                wave: Some(1),
                plans: vec!["05-01-alpha".to_string()],
            }]
        );
    }

    #[test]
    fn test_plan_waves_are_empty_for_a_phase_whose_plans_record_none() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), "---\nphase: 5\n---\n").unwrap();
        fs::write(dir.path().join("05-02-beta-PLAN.md"), "no frontmatter here").unwrap();

        let inf = infer_disk_status(dir.path());
        assert_eq!(inf.plan_count, 2);
        assert!(inf.plan_waves.is_empty());
    }
}
