# Deferred Items — quick-260512-ecm

Out-of-scope discoveries during execution; NOT fixed in this task.

## Pre-existing clippy warning (unrelated to SECURITY.md detection)

- **File:** `src/project_creator.rs:146`
- **Lint:** `clippy::cmp_owned`
- **Snippet:** `assert!(result != PathBuf::from("~") || dirs::home_dir().is_none());`
- **Fix (deferred):** replace `PathBuf::from("~")` with `"~"` so no owned `PathBuf` is allocated
- **Why deferred:** outside the scope of this quick task (only `disk_status.rs` and `detail.rs` are in scope per the plan's `files_modified`)
