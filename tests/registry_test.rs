use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::path::PathBuf;

// Import the library's modules
use gsd_meta_manager::config::{load_config, save_config, Config, CONFIG_SCHEMA_VERSION};
use gsd_meta_manager::registry::{
    add_project, clear_opt_in, is_opted_in, list_projects, record_opt_in, remove_project, Alias,
    AliasRefusal, LegacyRegistryKey,
};

/// A fixture alias, judged the way a real one is. Since 21-17 the registration
/// functions take a `registry::Alias`, so a test cannot hand them a value the
/// production entry points would have refused (D-17-2).
fn visible(raw: &str) -> Alias {
    Alias::new(raw).expect("a visible test alias")
}

#[test]
fn add_project_with_valid_alias_and_planning_dir_succeeds() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    let result = add_project(&mut config, &visible("myapp"), temp.path());
    assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
    assert!(config.projects.contains_key("myapp"));
    assert_eq!(config.projects["myapp"].path, temp.path());
}

#[test]
fn add_project_with_duplicate_alias_returns_error() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("myapp"), temp.path()).unwrap();

    let result = add_project(&mut config, &visible("myapp"), temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("already exists"),
        "Expected 'already exists' in error, got: {}",
        err_msg
    );
}

/// **The refusal MOVED rather than disappeared, and it moved earlier** (D-17-2).
///
/// This test used to hand `add_project` a raw `""` and read its `bail!`. Since
/// 21-17 a blank string cannot be turned into a `registry::Alias` at all, so it
/// cannot reach the registration signature — which is the point of the newtype:
/// the value is judged where it enters, not where it lands.
#[test]
fn an_empty_alias_cannot_be_constructed_so_it_never_reaches_registration() {
    let refusal = Alias::new("").expect_err("an empty alias is not a name");
    assert!(
        matches!(refusal, AliasRefusal::NotVisible),
        "got {refusal:?}"
    );
    assert!(
        refusal.to_string().contains("visible"),
        "the message must say what is wrong with the value, got: {refusal}"
    );
}

/// The whitespace rule, likewise moved to the constructor. It is kept as a
/// separate clause from blankness because it is STRICTER than the
/// path-component question: `"my app"` is a fine directory name and a bad thing
/// to type at a shell.
#[test]
fn a_whitespace_carrying_alias_cannot_be_constructed_either() {
    let refusal = Alias::new("my app").expect_err("an alias may not contain whitespace");
    assert!(
        matches!(refusal, AliasRefusal::Whitespace),
        "got {refusal:?}"
    );
    assert!(
        refusal.to_string().contains("whitespace"),
        "Expected 'whitespace' in error, got: {refusal}"
    );
}

/// End to end, through the real registration path: two aliases that render
/// identically cannot both name a project.
///
/// Consumes the shared fixture shape rather than hand-spelling the pairs —
/// reachable from an integration crate since 21-17 dropped `test_support`'s
/// `#[cfg(test)]` gate (D-17-5).
#[test]
fn a_look_alike_alias_cannot_join_its_visible_twin_in_the_registry() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("demo"), temp.path()).expect("the visible twin registers");

    for (visible_member, look_alike) in gsd_meta_manager::test_support::LOOK_ALIKE_PAIRS {
        let refusal = Alias::new(look_alike).expect_err(
            "an alias that renders exactly like its twin must not be constructible",
        );
        assert!(
            matches!(refusal, AliasRefusal::InvisibleFormatting { .. }),
            "{look_alike:?} renders as {visible_member:?}; the refusal must name \
             that harm, got {refusal:?}"
        );
    }

    assert_eq!(
        config.projects.len(),
        1,
        "exactly one project may exist for a name that renders one way"
    );
}

#[test]
fn add_project_with_path_missing_planning_returns_error() {
    let temp = TempDir::new().unwrap();
    // No .planning/ directory created

    let mut config = Config::new();
    let result = add_project(&mut config, &visible("myapp"), temp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No .planning/ directory found"),
        "Expected 'No .planning/ directory found' in error, got: {}",
        err_msg
    );
}

#[test]
fn add_project_with_nonexistent_path_returns_error() {
    let mut config = Config::new();
    let fake_path = PathBuf::from("/nonexistent/path/to/project");
    let result = add_project(&mut config, &visible("myapp"), &fake_path);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("does not exist"),
        "Expected 'does not exist' in error, got: {}",
        err_msg
    );
}

#[test]
fn remove_project_with_existing_alias_succeeds() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("myapp"), temp.path()).unwrap();

    let result = remove_project(&mut config, &LegacyRegistryKey::from_argv("myapp".to_string()));
    assert!(result.is_ok());
    assert!(!config.projects.contains_key("myapp"));
}

#[test]
fn remove_project_with_nonexistent_alias_returns_error() {
    let mut config = Config::new();
    let result = remove_project(
        &mut config,
        &LegacyRegistryKey::from_argv("nonexistent".to_string()),
    );
    assert!(result.is_err());
}

#[test]
fn save_config_then_load_config_roundtrips() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.json");

    // Create a config with a project
    let project_temp = TempDir::new().unwrap();
    project_temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("roundtrip"), project_temp.path()).unwrap();

    // Save and reload
    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    // Against the constant, not a literal: a config this build writes carries
    // this build's schema version, and pinning the number here would need
    // editing on every future bump for no gain.
    assert_eq!(loaded.version, CONFIG_SCHEMA_VERSION);
    assert!(loaded.projects.contains_key("roundtrip"));
    assert_eq!(loaded.projects["roundtrip"].path, project_temp.path());
}

#[test]
fn a_round_trip_through_save_and_load_preserves_an_opt_in_record() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.json");

    let project_temp = TempDir::new().unwrap();
    project_temp.child(".planning").create_dir_all().unwrap();
    project_temp
        .child("CLAUDE.md")
        .write_str("# Project instructions\n")
        .unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("opted"), project_temp.path()).unwrap();
    record_opt_in(&mut config, "opted").unwrap();
    let recorded = config.projects["opted"].driver_opt_in.clone().unwrap();

    save_config(&config, &config_path).unwrap();
    let loaded = load_config(&config_path).unwrap();

    assert!(
        is_opted_in(&loaded, "opted"),
        "an opt-in that does not survive a save and load is not an opt-in"
    );
    assert_eq!(
        loaded.projects["opted"].driver_opt_in.as_ref(),
        Some(&recorded),
        "both fields of the record round-trip verbatim, digest included"
    );

    // And the withdrawal round-trips too, as an absence rather than a marker.
    let mut loaded = loaded;
    clear_opt_in(&mut loaded, "opted").unwrap();
    save_config(&loaded, &config_path).unwrap();
    let reloaded = load_config(&config_path).unwrap();
    assert!(!is_opted_in(&reloaded, "opted"));
}

#[test]
fn load_config_on_nonexistent_file_returns_default() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("does_not_exist.json");

    let config = load_config(&config_path).unwrap();
    assert_eq!(config.version, CONFIG_SCHEMA_VERSION);
    assert!(config.projects.is_empty());
}

#[test]
fn list_projects_returns_sorted_by_alias() {
    let temp_a = TempDir::new().unwrap();
    temp_a.child(".planning").create_dir_all().unwrap();
    let temp_b = TempDir::new().unwrap();
    temp_b.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    add_project(&mut config, &visible("zulu"), temp_a.path()).unwrap();
    add_project(&mut config, &visible("alpha"), temp_b.path()).unwrap();

    let projects = list_projects(&config);
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].0, "alpha");
    assert_eq!(projects[1].0, "zulu");
}

#[test]
fn end_to_end_add_then_list_via_cli() {
    let project_temp = TempDir::new().unwrap();
    project_temp.child(".planning").create_dir_all().unwrap();
    let config_temp = TempDir::new().unwrap();
    let config_path = config_temp.path().join("config.json");

    // Run `add`
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "add",
            project_temp.path().to_str().unwrap(),
            "testalias",
        ])
        .output()
        .expect("Failed to run cargo");

    assert!(
        output.status.success(),
        "Add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let add_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        add_stdout.contains("testalias"),
        "Expected 'testalias' in add output: {}",
        add_stdout
    );

    // Run `list`
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "list",
        ])
        .output()
        .expect("Failed to run cargo");

    assert!(
        output.status.success(),
        "List failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let list_stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        list_stdout.contains("testalias"),
        "Expected 'testalias' in list output: {}",
        list_stdout
    );

    // And the refusing direction end to end, with a member from OUTSIDE the
    // pre-round-7 ranges. Pass 7 measured `add` accepting eight such aliases
    // beside `demo`, producing nine registry keys that all rendered as `demo`.
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "--config",
            config_path.to_str().unwrap(),
            "add",
            project_temp.path().to_str().unwrap(),
            "testalias\u{202e}",
        ])
        .output()
        .expect("Failed to run cargo");

    assert!(
        !output.status.success(),
        "an alias carrying U+202E renders identically to `testalias` and must \
         not become a second registry key"
    );

    let config = load_config(&config_path).expect("the config still loads");
    assert_eq!(
        config.projects.len(),
        1,
        "the refused add must write nothing: exactly one key may exist for a \
         name that renders one way"
    );
}

/// **The migration recovery route, CERTIFIED rather than asserted in prose.**
///
/// D-19-2 rates the alphabet narrowing `costly` and explicitly NOT one-way, and
/// T-21-19-04 accepts legacy entries failing closed — both on the strength of
/// one claim: an entry this build refuses is still REMOVABLE, so a user is never
/// stuck with a project they can neither use nor delete. That route is real at
/// HEAD (`remove_project` takes a raw `&str`; neither CLI arm nor the TUI path
/// wraps it in `Alias::new`; `judged_alias_or_exit` prints the hint), but
/// nothing tested it — the two removal tests above only remove `"myapp"`, a
/// value the alphabet accepts. So nothing but a comment stopped a later cleanup
/// from wrapping the remove path in `Alias::new` and making every legacy entry
/// permanent.
///
/// This goes red the day someone does that, which is the only thing that could
/// turn D-19-2 from `costly` into one-way.
#[test]
fn a_legacy_alias_the_alphabet_refuses_is_still_removable() {
    for raw in [
        "\u{434}\u{435}\u{43c}\u{43e}", // Cyrillic — a plain non-ASCII alias an older build took
        "demo\u{202e}",                 // and a look-alike one
    ] {
        let temp = TempDir::new().unwrap();
        temp.child(".planning").create_dir_all().unwrap();

        let mut config = Config::new();
        add_project(&mut config, &visible("legacy"), temp.path()).expect("the seed registers");

        // Re-key the entry under the raw value. This reaches `Config.projects`
        // exactly as an older build's `add_project` did — behind `Alias::new`,
        // which is the whole point — without hand-building a `RegisteredProject`
        // literal. That literal is deliberately breakage-prone by
        // `src/config.rs`'s design so field additions surface at the call sites
        // that must handle them, and an integration test is not one of those.
        let entry = config
            .projects
            .remove("legacy")
            .expect("the seed entry exists");
        config.projects.insert(raw.to_string(), entry);

        // ARRIVAL BEFORE PROPERTY: the key is genuinely in the registry before
        // anything is asserted about removing it, or this test could pass
        // vacuously against a config that never held it.
        assert!(
            list_projects(&config)
                .iter()
                .any(|(alias, _)| alias.as_str() == raw),
            "{raw:?} must be present before this test says anything about \
             removing it"
        );

        // And it is genuinely a value THIS build refuses, not an incidental one.
        let refusal = Alias::new(raw)
            .expect_err("the fixture must be an alias this build's alphabet refuses");
        assert!(
            matches!(
                refusal,
                AliasRefusal::OutsideIdentityAlphabet { .. }
                    | AliasRefusal::InvisibleFormatting { .. }
            ),
            "{raw:?} must be refused by the alphabet or the earlier, more \
             specific invisible-formatting clause: got {refusal:?}"
        );

        // The claim D-19-2's reversibility rating rests on.
        //
        // **Only the CONSTRUCTOR CALL is wrapped for `21-24`; not one assertion
        // below or above changed.** `LegacyRegistryKey::from_argv` judges
        // nothing and `remove_project` looks up on
        // `as_raw_for_lookup_only()`, so `raw` still reaches the removal
        // byte-for-byte — which is exactly what D-17-3's accept half demands
        // and what this test exists to pin.
        remove_project(&mut config, &LegacyRegistryKey::from_argv(raw.to_string())).expect(
            "an entry this build refuses must still be removable — otherwise \
             the alphabet narrowing is one-way and a user is stuck with a \
             project they can neither use nor delete",
        );
        assert!(
            !config.projects.contains_key(raw),
            "and the key must actually be gone"
        );
    }
}
