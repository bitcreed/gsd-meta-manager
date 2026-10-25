//! The `GSDMM_EXPERIMENTAL_FEATURES` startup flag — **one env read, one parse,
//! one bool** (quick task 260917-fko, decision D1).
//!
//! The driver drives a real repository with a real agent. A user who never asked
//! for that must not discover it by pressing a key, so every driver surface in
//! the TUI hangs off the single `bool` this module resolves, stored once on
//! [`crate::ui::screens::AppContext::experimental`].
//!
//! **Why a module of its own rather than a `const` in `app.rs`.** The parse has
//! to be exercised without mutating the process environment —
//! `std::env::set_var` is unsound under the parallel test harness — so the
//! parse and the env read are deliberately two functions, and only the pure one
//! is tested.

/// The environment variable that gates every experimental surface.
///
/// Named as a constant so tests and `docs/CONFIGURATION.md` reference one
/// literal rather than each respelling the name.
pub const EXPERIMENTAL_FEATURES_ENV: &str = "GSDMM_EXPERIMENTAL_FEATURES";

/// Whether `raw` — the variable's value, or `None` when it is unset — turns the
/// experimental surfaces on.
///
/// Accepted truthy values, ASCII-case-insensitive with surrounding whitespace
/// trimmed: `1`, `true`, `yes`, `on`. Everything else — unset, empty, `0`,
/// `false`, `no`, `off`, and any unrecognised value — is OFF.
///
/// **Deliberately not a presence test.** `GSDMM_EXPERIMENTAL_FEATURES=0` has to
/// mean off, and a presence test cannot express that: it would read an explicit
/// opt-*out* as an opt-in, which for a surface that spawns an agent against the
/// user's repository is the one direction the default must never fail in.
///
/// Unrecognised values are off rather than on for the same reason — a typo
/// (`GSDMM_EXPERIMENTAL_FEATURES=ture`) must not enable anything.
pub fn experimental_features_enabled_from(raw: Option<&str>) -> bool {
    let Some(value) = raw else {
        return false;
    };
    let value = value.trim();
    ["1", "true", "yes", "on"]
        .iter()
        .any(|truthy| value.eq_ignore_ascii_case(truthy))
}

/// Read [`EXPERIMENTAL_FEATURES_ENV`] from the process environment.
///
/// **The only production site in this crate that touches the environment for
/// this flag** (threat T-fko-02). It is called exactly once, from
/// `App::from_config`, and the result is stored on `AppContext`; a second read
/// site would let two surfaces disagree with each other mid-session.
pub fn experimental_features_enabled() -> bool {
    experimental_features_enabled_from(std::env::var(EXPERIMENTAL_FEATURES_ENV).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pure parse is what these drive: `std::env::set_var` is unsound under
    /// the parallel test harness, and the whole truthy set is observable
    /// without it.
    #[test]
    fn the_four_documented_truthy_spellings_turn_the_flag_on() {
        for raw in ["1", "true", "yes", "on"] {
            assert!(
                experimental_features_enabled_from(Some(raw)),
                "{raw:?} is one of the four documented truthy values"
            );
        }
    }

    #[test]
    fn the_truthy_set_is_case_insensitive_and_whitespace_trimmed() {
        for raw in ["TRUE", "True", "On", "YES", "  yes  ", "\ttrue\n", " 1 "] {
            assert!(
                experimental_features_enabled_from(Some(raw)),
                "{raw:?} must parse truthy: the set is ASCII-case-insensitive \
                 and surrounding whitespace is trimmed"
            );
        }
    }

    /// An unset variable is the DEFAULT, and the default is off — this is the
    /// assertion that says a user who never asked for the driver does not get
    /// it.
    #[test]
    fn an_unset_variable_leaves_the_experimental_surfaces_off() {
        assert!(!experimental_features_enabled_from(None));
    }

    #[test]
    fn the_explicit_falsey_spellings_turn_the_flag_off() {
        for raw in ["0", "false", "FALSE", "no", "off", "Off", " 0 "] {
            assert!(
                !experimental_features_enabled_from(Some(raw)),
                "{raw:?} must parse falsey — this is why the flag is not a \
                 presence test"
            );
        }
    }

    /// The empty string is what `FOO=` gives, and it is off: setting a variable
    /// to nothing is not asking for anything.
    #[test]
    fn an_empty_or_whitespace_only_value_is_off() {
        assert!(!experimental_features_enabled_from(Some("")));
        assert!(!experimental_features_enabled_from(Some("   ")));
    }

    /// A typo must not enable a surface that spawns an agent against a real
    /// repository, so the set is closed rather than "anything that is not
    /// falsey".
    #[test]
    fn an_unrecognised_value_is_off_rather_than_on() {
        for raw in ["maybe", "ture", "1 1", "yes please", "enabled", "2"] {
            assert!(
                !experimental_features_enabled_from(Some(raw)),
                "{raw:?} is not in the closed truthy set and must be off"
            );
        }
    }

    /// The name lives in exactly one literal, which is what keeps the docs, the
    /// tests and the reader in agreement.
    #[test]
    fn the_env_var_is_named_once_and_carries_the_gsdmm_prefix() {
        assert_eq!(EXPERIMENTAL_FEATURES_ENV, "GSDMM_EXPERIMENTAL_FEATURES");
    }
}
