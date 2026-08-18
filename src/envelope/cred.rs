//! The git configuration the envelope injects through the **environment**.
//!
//! Git reads `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n`
//! (since 2.31; this machine has 2.43) at the same precedence as `-c`, which is
//! **above repo-local config**, and every `git` invocation in the child's
//! process tree inherits it — including ones an agent makes from a nested shell
//! or from a script it wrote.

use std::ffi::OsString;
use std::path::Path;

/// The env-injected `core.hooksPath` triplet pointing git at `hooks_dir`.
///
/// ## Four things this buys over installing into the repository's hooks
/// directory, which is why the obvious approach is declined (D-09)
///
/// 1. **The user's own git commands in that repository are unaffected.** A
///    `pre-push` hook installed into `.git/hooks/` fires for the human too,
///    which would make this tool's safety envelope silently police the user's
///    own pushes. That is a bug, not a feature.
/// 2. **Nothing is left behind** when a run crashes, is SIGKILLed, or the TUI is
///    closed — the same posture as "crash reconciliation performs zero disk
///    writes".
/// 3. **The agent cannot uninstall it by editing a file in the repository**,
///    because there is no file in the repository.
/// 4. **Repo-local `core.hooksPath` cannot override it**, because env-injected
///    configuration outranks repository configuration.
///
/// ## The limit, stated rather than implied
///
/// An agent that unsets `GIT_CONFIG_COUNT` in a subshell **escapes this layer**.
/// That is not a hole this project can close client-side, and the one form that
/// outranks this injection is `git -c core.hooksPath=… push` — which plan 19-02
/// denies **by name** at the tool boundary for exactly that reason. Neither the
/// denial nor this injection is a guarantee; server-side branch protection is
/// the only boundary that does not depend on the agent's cooperation.
pub fn hooks_path_env(hooks_dir: &Path) -> Vec<(OsString, OsString)> {
    vec![
        (OsString::from("GIT_CONFIG_COUNT"), OsString::from("1")),
        (
            OsString::from("GIT_CONFIG_KEY_0"),
            OsString::from("core.hooksPath"),
        ),
        (
            OsString::from("GIT_CONFIG_VALUE_0"),
            hooks_dir.as_os_str().to_owned(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_triplet_names_the_count_the_key_and_the_directory() {
        let env = hooks_path_env(Path::new("/data/envelope/demo/hooks"));
        assert_eq!(
            env,
            vec![
                (OsString::from("GIT_CONFIG_COUNT"), OsString::from("1")),
                (
                    OsString::from("GIT_CONFIG_KEY_0"),
                    OsString::from("core.hooksPath")
                ),
                (
                    OsString::from("GIT_CONFIG_VALUE_0"),
                    OsString::from("/data/envelope/demo/hooks")
                ),
            ]
        );
    }

    #[test]
    fn the_count_matches_the_number_of_key_value_pairs() {
        // A count that disagrees with the pairs makes git read a key that is not
        // there, and git's response to that is to ignore the whole injection.
        let env = hooks_path_env(Path::new("/x"));
        let count: usize = env[0].1.to_string_lossy().parse().unwrap();
        assert_eq!(count * 2 + 1, env.len());
    }
}
