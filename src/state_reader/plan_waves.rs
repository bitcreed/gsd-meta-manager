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

/// A `waves.json` parallelism manifest (GSD 1.8.0 claude-orchestration),
/// normalised for display.
///
/// Moved here from the Pipeline renderer (quick 260926-2l4, D-04): the file is
/// now read once per refresh inside
/// [`crate::state_reader::disk_status::infer_disk_status`] and cached on
/// `DiskInference::waves_manifest`, so rendering performs no file I/O. The form
/// carries no `serde_json::Value`, so it derives `PartialEq` for the
/// unchanged-state suppression the rest of `DiskInference` feeds.
///
/// Every `label` and plan `id` is third-party text from another project's
/// `.planning/` directory: a renderer puts it through the UI's `shown()`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WavesManifest {
    pub waves: Vec<ManifestWave>,
}

/// One wave of a [`WavesManifest`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestWave {
    /// The manifest's own wave id (`"w1"`), its numeric `wave`, or `wave N`
    /// (1-based position) when it carries neither.
    pub label: String,
    /// The wave's plans that carry an id; a plan entry with no id says nothing
    /// a row could show and is dropped.
    pub plans: Vec<ManifestPlan>,
}

/// One plan of a [`ManifestWave`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestPlan {
    pub id: String,
    /// How many files the manifest says the plan modifies.
    pub files: usize,
}

/// The on-disk shape, deserialized leniently: unknown fields are ignored and
/// any missing field defaults, so a partial or evolving manifest still parses.
/// `{ "waves": [ { "id": "w1", "plans": [ { "id": "p1", "files_modified": [..] } ] } ] }`.
#[derive(Debug, serde::Deserialize)]
struct RawManifest {
    #[serde(default)]
    waves: Vec<RawWave>,
}

#[derive(Debug, serde::Deserialize)]
struct RawWave {
    /// Wave identifier — usually a string id (`"w1"`) but tolerated as a number too.
    #[serde(default)]
    id: Option<serde_json::Value>,
    /// Alternate wave key some manifests use instead of `id`.
    #[serde(default)]
    wave: Option<serde_json::Value>,
    #[serde(default)]
    plans: Vec<RawPlan>,
}

#[derive(Debug, serde::Deserialize)]
struct RawPlan {
    #[serde(default)]
    id: Option<String>,
    /// Alternate plan-identifier key.
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    files_modified: Vec<String>,
}

impl RawWave {
    /// A short display label for the wave, falling back to a 1-based index.
    fn label(&self, index: usize) -> String {
        match self.id.as_ref().or(self.wave.as_ref()) {
            Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
            Some(serde_json::Value::Null) | None => format!("wave {}", index + 1),
            Some(other) => other.to_string(),
        }
    }
}

impl RawPlan {
    /// The plan's identifier, if the manifest carried one.
    fn label(&self) -> Option<String> {
        self.id
            .clone()
            .or_else(|| self.plan.clone())
            .filter(|s| !s.is_empty())
    }
}

/// Deserialize and normalise a `waves.json` manifest. `None` on unparsable
/// input, so the scan silently records nothing rather than surfacing noise.
pub fn parse_waves_manifest(raw: &str) -> Option<WavesManifest> {
    let parsed = serde_json::from_str::<RawManifest>(raw).ok()?;
    Some(WavesManifest {
        waves: parsed
            .waves
            .iter()
            .enumerate()
            .map(|(index, wave)| ManifestWave {
                label: wave.label(index),
                plans: wave
                    .plans
                    .iter()
                    .filter_map(|p| {
                        p.label().map(|id| ManifestPlan {
                            id,
                            files: p.files_modified.len(),
                        })
                    })
                    .collect(),
            })
            .collect(),
    })
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

    #[test]
    fn test_parse_waves_manifest_two_waves() {
        let raw = r#"{
            "waves": [
                {
                    "id": "w1",
                    "plans": [
                        { "id": "p1", "files_modified": ["src/foo.rs"] },
                        { "id": "p2", "files_modified": ["src/bar.rs"] }
                    ]
                },
                {
                    "id": "w2",
                    "plans": [
                        { "id": "p3", "files_modified": ["src/baz.rs", "src/qux.rs"] }
                    ]
                }
            ]
        }"#;
        let manifest = parse_waves_manifest(raw).expect("valid manifest parses");
        assert_eq!(manifest.waves.len(), 2);
        assert_eq!(manifest.waves[0].label, "w1");
        assert_eq!(manifest.waves[0].plans.len(), 2);
        assert_eq!(
            manifest.waves[0].plans[0],
            ManifestPlan {
                id: "p1".to_string(),
                files: 1
            }
        );
        assert_eq!(manifest.waves[1].label, "w2");
        assert_eq!(manifest.waves[1].plans[0].files, 2);
    }

    #[test]
    fn test_parse_waves_manifest_garbage_is_none() {
        assert!(parse_waves_manifest("not json at all").is_none());
        assert!(parse_waves_manifest("{ oops ]").is_none());
    }

    #[test]
    fn test_parse_waves_manifest_lenient_defaults() {
        // Numeric wave id and missing plan ids are tolerated.
        let raw = r#"{ "waves": [ { "wave": 1, "plans": [ { "files_modified": [] } ] }, { "plans": [ { "plan": "p9" } ] } ] }"#;
        let manifest = parse_waves_manifest(raw).expect("lenient parse");
        assert_eq!(manifest.waves.len(), 2);
        assert_eq!(manifest.waves[0].label, "1");
        // A plan with no id says nothing a row could show, and is dropped.
        assert!(manifest.waves[0].plans.is_empty());
        // No id and no wave: the 1-based position; `plan` is the alternate key.
        assert_eq!(manifest.waves[1].label, "wave 2");
        assert_eq!(manifest.waves[1].plans[0].id, "p9");

        // Empty object → empty waves, still Some (the scan keeps only non-empty).
        let empty = parse_waves_manifest("{}").expect("empty object parses");
        assert!(empty.waves.is_empty());
    }
}
