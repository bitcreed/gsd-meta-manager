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
    for (id, wave) in entries {
        if let Some(n) = wave {
            numbered.entry(n).or_default().push(id);
        }
    }
    if numbered.is_empty() {
        return Vec::new();
    }
    numbered
        .into_iter()
        .map(|(wave, mut plans)| {
            plans.sort();
            PlanWave {
                wave: Some(wave),
                plans,
            }
        })
        .collect()
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
}
