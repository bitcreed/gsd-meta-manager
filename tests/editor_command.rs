#![cfg(unix)]
// ============================================================================
// WR-06 / D-17 (Phase 30, plan 30-08): a `$VISUAL` / `$EDITOR` value that carries
// arguments (`code -w`, `emacs -nw`) opens the edit temp, and a run-directory path
// reaches the editor as one literal argument. Real `sh` runs a stub editor that
// records its argv, one argument per line. No real editor, no model call (D-26).
// ============================================================================

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{Mutex, MutexGuard};

use gsd_meta_manager::app::editor_command;
use tempfile::TempDir;

/// Held from writing a stub script until the process that runs it has exited.
///
/// A test thread's `fork` inherits another thread's still-open write descriptor on a
/// just-written script until that child execs, so exec of the script fails with
/// `ETXTBSY` ("Text file busy"). Only one test in this binary may be between
/// "write the stub" and "the stub exited" at a time.
static EXEC_LOCK: Mutex<()> = Mutex::new(());

fn exec_lock() -> MutexGuard<'static, ()> {
    EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// An executable `#!/bin/sh` stub at `dir/name` that appends each argument as its own
/// line to `record`.
fn write_stub(dir: &Path, name: &str, record: &Path) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let stub = dir.join(name);
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> '{}'; done\n",
            record.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    stub
}

/// Spawn the program and argv `editor_command` returns, in `cwd`.
fn run(
    editor: &str,
    path: &Path,
    cwd: &Path,
    extra_path: Option<&Path>,
) -> std::io::Result<ExitStatus> {
    let (program, argv) = editor_command(editor, path, None);
    let mut cmd = Command::new(program);
    cmd.args(argv).current_dir(cwd);
    if let Some(dir) = extra_path {
        let mut paths = vec![dir.to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        cmd.env("PATH", std::env::join_paths(paths).unwrap());
    }
    cmd.status()
}

fn recorded(record: &Path) -> Vec<String> {
    fs::read_to_string(record)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn an_editor_with_arguments_receives_them_and_the_path() {
    let _guard = exec_lock();
    let root = TempDir::new().unwrap();
    let record = root.path().join("record");
    let stub = write_stub(&root.path().join("bin"), "ed-stub", &record);
    let target = root.path().join("DIRECTION.md");

    let status = run(
        &format!("{} -w", stub.display()),
        &target,
        root.path(),
        None,
    );

    assert!(
        status.as_ref().is_ok_and(|s| s.success()),
        "an editor string with arguments must run: {status:?}"
    );
    assert_eq!(
        recorded(&record),
        vec!["-w".to_string(), target.display().to_string()]
    );
}

#[test]
fn an_editor_path_with_a_space_is_quoted_and_runs() {
    let _guard = exec_lock();
    let root = TempDir::new().unwrap();
    let record = root.path().join("record");
    let stub = write_stub(&root.path().join("my editors"), "ed-stub", &record);
    let target = root.path().join("DIRECTION.md");

    let status = run(
        &format!("'{}' --wait", stub.display()),
        &target,
        root.path(),
        None,
    );

    assert!(
        status.as_ref().is_ok_and(|s| s.success()),
        "a quoted editor path with arguments must run: {status:?}"
    );
    assert_eq!(
        recorded(&record),
        vec!["--wait".to_string(), target.display().to_string()]
    );
}

/// The shipped direct spawn, unchanged: an unquoted value that names one existing file
/// (e.g. `/Applications/Sublime Text.app/Contents/SharedSupport/bin/subl`) is the program.
#[test]
fn an_existing_editor_file_with_a_space_runs_directly() {
    let _guard = exec_lock();
    let root = TempDir::new().unwrap();
    let record = root.path().join("record");
    let stub = write_stub(&root.path().join("Sublime Text"), "subl", &record);
    let target = root.path().join("DIRECTION.md");
    let editor = stub.display().to_string();

    let (program, argv) = editor_command(&editor, &target, None);
    assert_eq!(
        program,
        OsString::from(&editor),
        "an existing file is the program itself, not `sh`"
    );
    assert_eq!(argv, vec![target.as_os_str().to_owned()]);

    let status = run(&editor, &target, root.path(), None);
    assert!(
        status.as_ref().is_ok_and(|s| s.success()),
        "an existing editor file with a space must run: {status:?}"
    );
    assert_eq!(recorded(&record), vec![target.display().to_string()]);
}

#[test]
fn a_bare_editor_program_runs_directly() {
    let _guard = exec_lock();
    let root = TempDir::new().unwrap();
    let record = root.path().join("record");
    let bin = root.path().join("bin");
    write_stub(&bin, "gsd-ed-stub-bare", &record);
    let target = root.path().join("DIRECTION.md");

    let (program, argv) = editor_command("gsd-ed-stub-bare", &target, None);
    assert_eq!(program, OsString::from("gsd-ed-stub-bare"));
    assert_eq!(argv, vec![target.as_os_str().to_owned()]);

    let status = run("gsd-ed-stub-bare", &target, root.path(), Some(&bin));
    assert!(
        status.as_ref().is_ok_and(|s| s.success()),
        "a bare editor program must run: {status:?}"
    );
    assert_eq!(recorded(&record), vec![target.display().to_string()]);
}

/// T-30-27: the path comes from a run directory whose name a cloned repository or a
/// sibling sandbox chooses. It reaches the editor as one literal argument and runs
/// nothing, whatever shell syntax it holds.
#[test]
fn a_hostile_path_reaches_the_editor_as_one_literal_argument() {
    let _guard = exec_lock();
    let root = TempDir::new().unwrap();
    let record = root.path().join("record");
    let stub = write_stub(&root.path().join("bin"), "ed-stub", &record);
    let cwd = root.path().join("cwd");
    fs::create_dir_all(&cwd).unwrap();

    let hostile = [
        "$(touch PWNED);x.md",
        "`touch PWNED2`.md",
        "a'; touch PWNED3; echo '.md",
        "a\" ; touch PWNED4 ; \".md",
    ];
    for (i, name) in hostile.iter().enumerate() {
        let target = root.path().join(name);
        let _ = fs::remove_file(&record);

        let status = run(&format!("{} -w", stub.display()), &target, &cwd, None);

        assert!(
            status.as_ref().is_ok_and(|s| s.success()),
            "case {i} ({name}): {status:?}"
        );
        assert_eq!(
            recorded(&record),
            vec!["-w".to_string(), target.display().to_string()],
            "case {i} ({name}): the path is one literal argument"
        );
    }
    let entries: Vec<_> = fs::read_dir(&cwd)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(
        entries.is_empty(),
        "no hostile path ran a command (found {entries:?})"
    );
    for pwned in ["PWNED", "PWNED2", "PWNED3", "PWNED4"] {
        assert!(!root.path().join(pwned).exists(), "{pwned}");
    }
}
