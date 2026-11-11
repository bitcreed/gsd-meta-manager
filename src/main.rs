mod driver_cli;
mod driverctl_cli;
mod event;
mod next_cli;
mod status_cli;
mod task_cli;
mod taskctl_cli;
mod tui;

use event::EventBus;
use gsd_meta_manager::app::App;
use gsd_meta_manager::cli::{Cli, Commands, EnvelopeAction};
use gsd_meta_manager::config::{load_config, save_config, Config};
use gsd_meta_manager::input::run_with_terminal_suspended;
use gsd_meta_manager::main_loop::{pump, PumpOutcome};
use gsd_meta_manager::registry::{add_project, list_projects, remove_project};
use gsd_meta_manager::watcher::FileWatcher;

/// Install the file-logging subscriber, or install nothing.
///
/// **Fallible on purpose (D-25, T-27-11).** This used to be
/// `tracing_appender::rolling::daily`, which PANICS (rc 101) when the log
/// directory cannot be created. Under an unwritable data dir that turned every
/// invocation of the binary into a crash, and an in-sandbox `driverctl hook`
/// that crashes is a PreToolUse gate that fails open. The builder returns a
/// `Result`; on `Err` no subscriber is installed and the `tracing` macros become
/// no-ops, so logging is lost and nothing else is. The TUI still starts; a
/// missing log file is a smaller harm than a refused launch.
fn init_tracing() {
    use tracing_appender::rolling::{RollingFileAppender, Rotation};

    let log_dir = dirs::data_local_dir()
        .map(|d| d.join("gsd-meta-manager"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    let Ok(file_appender) = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("gsd-meta-manager.log")
        .build(&log_dir)
    else {
        return;
    };
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

/// A synchronous entry point, so `driverctl` runs before anything that can fail
/// in the environment it is spawned into (D-25).
///
/// `driverctl log|show|hook` is re-entered by the sandboxed driver and by the
/// Claude hooks, where the data dir may be unwritable and no tokio runtime is
/// needed. It is therefore dispatched here, FIRST: before tracing init,
/// `color_eyre::install`, config loading and the runtime. The exit code is the
/// control, so `driverctl_cli::run` returns it and this function exits with it.
fn main() -> anyhow::Result<()> {
    let mut cli = Cli::parse_checked();
    match cli.command.take() {
        Some(Commands::Driverctl { action }) => std::process::exit(driverctl_cli::run(action)),
        // The one-off agent's report CLI (quick 261103-f5y): before `init_tracing`
        // for the reason `driverctl` is, it runs inside the sandbox and must create
        // no log file.
        Some(Commands::Taskctl { action }) => std::process::exit(taskctl_cli::run(action)),
        // The in-sandbox bridge (28.1-05, D-08): before `init_tracing` for the
        // same reason as `driverctl`, it creates no file and writes no stdout.
        // Its exit code is the driver's, handed back to bwrap.
        Some(Commands::Bridge {
            port,
            sock,
            command,
        }) => {
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            std::process::exit(gsd_meta_manager::launcher::bridge::run(
                port, &sock, &command,
            ));
            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            {
                let _ = (port, sock, command);
                eprintln!("gsd-meta-manager bridge: needs Linux or macOS");
                std::process::exit(1);
            }
        }
        // `next` is the read-only router hint (Phase 29, D-21): dispatched here,
        // before `init_tracing`, because tracing creates a log file and `next`
        // must write nothing, in or out of the sandbox.
        Some(Commands::Next {
            project,
            target_phase,
            ..
        }) => std::process::exit(next_cli::run(
            &project,
            target_phase.as_deref(),
            cli.config.clone(),
        )),
        // `status` is the read-only query for scripts and agents (quick 261110-qap):
        // before `init_tracing` for the same reason as `next`, it writes no log file.
        Some(Commands::Status { projects, .. }) => {
            std::process::exit(status_cli::run(&projects, cli.config.clone()))
        }
        other => cli.command = other,
    }

    // Initialize tracing subscriber with file appender before anything else
    init_tracing();

    // `driver` is synchronous too (the launcher is std threads, never an `async
    // fn`), so it runs here, after tracing is up and before the runtime exists.
    // Its exit code is the control.
    match cli.command.take() {
        Some(Commands::Driver { action }) => {
            std::process::exit(driver_cli::run(action, cli.config.clone()))
        }
        // `task` (quick 261103-f5y) is the one-off launcher's command line:
        // synchronous for the reason `driver` is.
        Some(Commands::Task { action }) => {
            std::process::exit(task_cli::run(action, cli.config.clone()))
        }
        other => cli.command = other,
    }

    color_eyre::install().map_err(|e| anyhow::anyhow!("{}", e))?;

    // Built explicitly, with every driver enabled and the multi-thread flavour,
    // so the runtime is exactly what `#[tokio::main]` used to build.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> anyhow::Result<()> {
    let config_path = cli.config.unwrap_or_else(Config::default_path);

    match cli.command {
        Some(Commands::Add { path, alias }) => {
            let canonical_path = path.canonicalize().unwrap_or(path);
            let alias = alias.unwrap_or_else(|| {
                canonical_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string()
            });
            // The argv-to-identity conversion, before any function that could
            // create or answer for an identity runs (D-17-2). A derived alias
            // goes through the same judgment as a typed one, which is why the
            // hint names the explicit-alias form.
            let alias = match gsd_meta_manager::registry::Alias::new(&alias) {
                Ok(alias) => alias,
                Err(refusal) => {
                    eprintln!("Error: {refusal}");
                    eprintln!(
                        "Provide an explicit alias: gsd-meta-manager add {} <alias>",
                        canonical_path.display()
                    );
                    std::process::exit(1);
                }
            };
            // Pruning loader: stale git-worktree entries are removed (and the
            // config saved once) before the new entry is judged (quick 260925-x0v).
            let (config, _pruned) =
                gsd_meta_manager::registry::load_config_pruning_worktrees(&config_path)?;
            // The membership check keeps the RAW bytes (`as_str`) — it is a
            // lookup into `config.projects` and an escaped key would miss every
            // entry. The sentence beneath it is READ, so it carries the escaped
            // form. Same value, one line apart, two different answers; that is
            // the whole split the withdrawn `Display` impl forces you to make.
            if config.projects.contains_key(alias.as_str()) {
                // `render_for_terminal`, not `display_identity`: ONE composition
                // for every CLI echo, so no site decides for itself which half
                // of the class applies (WR-01, `21-24`). This particular value
                // has already passed `Alias::new`, so the finite identity
                // alphabet means neither class can be present and the call is a
                // no-op today — it is here so the answer does not depend on
                // that remaining true, and so `grep display_identity src/main.rs`
                // has nothing to find.
                eprintln!(
                    "Error: alias '{}' already exists. Provide an explicit alias: gsd-manager add {} <alias>",
                    gsd_meta_manager::text::render_for_terminal(alias.as_str()),
                    canonical_path.display()
                );
                std::process::exit(1);
            }
            let mut config = config;
            // RAW into `add_project`: that is the value becoming the key.
            if let Err(e) = add_project(&mut config, &alias, &canonical_path) {
                // A linked worktree is refused with the main worktree to add
                // instead (quick 260925-x0v). Both paths come from the
                // filesystem, so both are escaped before reaching the terminal.
                if let Some(refusal) =
                    e.downcast_ref::<gsd_meta_manager::registry::LinkedWorktreeRefusal>()
                {
                    eprintln!(
                        "Error: {}",
                        gsd_meta_manager::text::render_for_terminal(&refusal.to_string())
                    );
                    eprintln!(
                        "Register the main worktree instead: gsd-meta-manager add {}",
                        gsd_meta_manager::text::render_for_terminal(
                            &refusal.main_worktree.display().to_string()
                        )
                    );
                    std::process::exit(1);
                }
                return Err(e);
            }
            save_config(&config, &config_path)?;
            // READ by a human, so escaped. A confirmation that renders a name
            // other than the one just written is a confirmation of the wrong
            // thing.
            // Same composition, same reason as the duplicate-alias echo above:
            // this value passed `Alias::new` so both classes are already
            // impossible, and the call is here so that fact is not what the
            // display honesty depends on.
            println!(
                "Added project '{}' at {}",
                gsd_meta_manager::text::render_for_terminal(alias.as_str()),
                canonical_path.display()
            );
        }
        // **The ONE deliberately-raw alias consumer (D-17-3), recorded loudly
        // rather than left to be discovered as an oversight.** Removal is a
        // membership-checked lookup that CREATES NOTHING, and it is the recovery
        // path for exactly the entries this build's registration refusals
        // orphan: pass 6 measured that an older build did register invisible and
        // look-alike aliases, and those rows are still in `config.json` and
        // still rendered by `list`. A removal that could not name what an older
        // build registered would make a bad entry permanent — itself a
        // WR-06-class falsehood generator ("Project not found" about an entry
        // `list` prints). Classified `raw-by-design` in guard ten's table with
        // this reason.
        //
        // **Accepting and echoing are now separated STRUCTURALLY rather than
        // remembered** (21-21, T-21-21-03). The argv string is bound into
        // `registry::LegacyRegistryKey`, which judges nothing — removal still
        // accepts byte-for-byte what an older build registered, which is
        // D-17-3's whole content — but which implements no `Display` and no
        // conversion into a string-like type. So `remove_project` keeps getting
        // the raw bytes while the echo below CANNOT be written unescaped without
        // a deliberate, visible choice. Before this, the arm bound a bare
        // `String` and the echo printed a raw legacy alias; verification pass 8
        // measured a bidi spoof through that line.
        Some(Commands::Remove { alias }) => {
            let key = gsd_meta_manager::registry::LegacyRegistryKey::from_argv(alias);
            let mut config = load_config(&config_path)?;
            // The key goes in WHOLE, since `21-24` (CR-01). It used to be
            // unwrapped here — `remove_project(&mut config,
            // key.as_raw_for_lookup_only())` — which handed the function a bare
            // `&str` and let its own failure path write `bail!("Project not
            // found: {}", alias)` one line above this echo. So `remove`'s
            // SUCCESS echo had been made to ask whether to escape and its
            // FAILURE echo, one line up, had never been asked. The lookup is
            // still on the RAW bytes; it now happens inside `remove_project`,
            // where the type is what makes the raw `bail!` unwritable.
            remove_project(&mut config, &key)?;
            save_config(&config, &config_path)?;
            // ESCAPED — this is read by a person. `escaped_for_display` now
            // delegates to `Untrusted::shown`, i.e. `text::render_for_terminal`:
            // BOTH the invisible-formatting class and the ESC/C0/DEL/C1 control
            // class. Before `21-24` it applied only the first, and a legacy key
            // carrying `\u{1b}[31m` printed a live ANSI colour sequence through
            // this very line — measured at the built binary (WR-01).
            //
            // That the raw route is not merely discouraged but IMPOSSIBLE is
            // certified by a control that can FAIL, not by this comment:
            // `text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions`
            // asserts the absent conversions at runtime and was observed red by
            // planting them. The verbatim compile error for the raw `bail!` is
            // quoted where it belongs — in `registry::remove_project`'s own doc,
            // beside the signature that produces it.
            println!("Removed project '{}'", key.escaped_for_display());
        }
        Some(Commands::List) => {
            // Pruning loader (quick 260925-x0v); `remove` deliberately keeps
            // plain `load_config` so `remove agent-…` still finds its entry
            // [inferred].
            let (config, _pruned) =
                gsd_meta_manager::registry::load_config_pruning_worktrees(&config_path)?;
            let projects = list_projects(&config);
            if projects.is_empty() {
                println!("No projects registered.");
            } else {
                println!("{:<20} {:<50} ADDED", "ALIAS", "PATH");
                println!("{}", "-".repeat(90));
                for (alias, project) in projects {
                    // **Escaped, not raw** (D-19-5). New registrations cannot
                    // carry invisible bytes, but rows an older build accepted
                    // are still here — and pass 7 measured this exact loop
                    // printing a legacy `gsd-\u{202e}nur` as `gsd-run`. Trojan
                    // Source (CVE-2021-42574) in the tool's own project list.
                    // This is the ONE site in this file that reads raw
                    // `config.projects` keys rather than an `Alias`, so it is
                    // the one where legacy rows actually arrive.
                    //
                    // **BOTH classes, since `21-24` (WR-01).** This used to be
                    // `display_identity` alone, which answers only
                    // `General_Category=Cf` union `Default_Ignorable_Code_Point`.
                    // `ESC` is `Cc` and in NEITHER of those, so the other half
                    // of the class walked straight through: measured at the
                    // built binary, a legacy key `ev\u{1b}[31mil` printed
                    // `ev^[[31mil` here — a live ANSI colour sequence the
                    // terminal honours, from a row on disk. The tree already
                    // said so at `ui/screens/driver.rs:874-878`:
                    //
                    //   "Neither subsumes the other, and this is the only place
                    //    a registry key is drawn on this path."
                    //
                    // What was missing was a single place that COMPOSED them.
                    // `text::render_for_terminal` is that place, so this loop no
                    // longer decides the question for itself. It is the class
                    // WITHOUT the `DRIVER_OUTPUT_LINE_CELLS` cap on purpose:
                    // `sanitize_render_line` would silently truncate a long
                    // alias at 512 characters and append an ellipsis, which is a
                    // display cap for agent prose, not for a project name.
                    //
                    // The `{:<20}` below is honoured because `Rendered`'s
                    // `Display` uses `f.pad(&self.0)`, not `write_str` —
                    // `write_str` ignores the formatter's width and fill, so
                    // this table silently lost its columns (`clean /tmp` where
                    // it had printed `clean                /tmp`). 21-24 found
                    // that by binary measurement and worked around it here with
                    // a `.to_string()`, because `src/text.rs` was outside its
                    // plan's diff; commit `7bf8f6b` fixed it at the source and
                    // added `rendered_display_honours_the_format_spec_in_both_directions`
                    // to certify it, so the workaround is gone and the padding
                    // is now the type's own behaviour.
                    println!(
                        "{:<20} {:<50} {}",
                        gsd_meta_manager::text::render_for_terminal(alias),
                        project.path.display(),
                        project.added
                    );
                }
            }
        }
        // This arm is BEFORE `tui::init()` by construction, and the position is
        // doing real work. A git hook's stdout and stderr belong to the pushing
        // git process; a terminal put into raw mode by ratatui on the way past
        // would corrupt the very output that carries the refusal.
        //
        // **The envelope re-entry arms judge their alias and FAIL CLOSED**
        // (D-17-2, T-21-17-07). The generated hook stubs pass the alias they
        // were installed with; a stub installed by an older build can carry a
        // value this build no longer admits, and the honest answer to "I cannot
        // tell which project this names" is each arm's existing refusal exit
        // code — 1 for the hooks and askpass, 2 for the guard — never a pass.
        // Each refusal NAMES THE RECOVERY ROUTE, because the first user to meet
        // this meets it as `exit 1` on `git push`, and a refusal a user cannot
        // act on is a bug report rather than an error message.
        // Never reached: `main` dispatches `driverctl` before this runtime
        // exists (D-25). The arm exists so the match stays exhaustive, and it
        // returns an error rather than panicking.
        Some(Commands::Driverctl { .. }) => {
            return Err(anyhow::anyhow!(
                "driverctl is dispatched in `main` before the async runtime exists"
            ));
        }
        // Never reached: `main` dispatches `taskctl` before tracing and this
        // runtime exist (quick 261103-f5y).
        Some(Commands::Taskctl { .. }) => {
            return Err(anyhow::anyhow!(
                "taskctl is dispatched in `main` before the async runtime exists"
            ));
        }
        // Never reached: `main` dispatches `bridge` before tracing and this
        // runtime exist (28.1-05).
        Some(Commands::Bridge { .. }) => {
            return Err(anyhow::anyhow!(
                "bridge is dispatched in `main` before the async runtime exists"
            ));
        }
        // Never reached either: `main` runs `driver` synchronously, before this
        // runtime exists, so the launcher never lives inside an `async fn`.
        Some(Commands::Driver { .. }) => {
            return Err(anyhow::anyhow!(
                "driver is dispatched in `main` before the async runtime exists"
            ));
        }
        // Never reached: `main` runs `task` synchronously, like `driver`.
        Some(Commands::Task { .. }) => {
            return Err(anyhow::anyhow!(
                "task is dispatched in `main` before the async runtime exists"
            ));
        }
        // And `next`, which runs even earlier (before tracing) so it can write nothing.
        Some(Commands::Next { .. }) => {
            return Err(anyhow::anyhow!(
                "next is dispatched in `main` before tracing and the async runtime exist"
            ));
        }
        // And `status`, dispatched at the same early point as `next`.
        Some(Commands::Status { .. }) => {
            return Err(anyhow::anyhow!(
                "status is dispatched in `main` before tracing and the async runtime exist"
            ));
        }
        Some(Commands::Envelope { action }) => match action {
            EnvelopeAction::Scan { alias, root } => {
                // Was a manual `is_plain_path_component` check — a fifth site
                // spelling the alias judgment for itself. `Alias::new` is that
                // judgment, and it also refuses the look-alike shapes the bare
                // predicate accepted before D-17-1.
                let alias = match gsd_meta_manager::registry::Alias::new(&alias) {
                    Ok(alias) => alias,
                    Err(refusal) => {
                        eprintln!("Error: {refusal}");
                        eprintln!("No envelope sanctions a scan for it.");
                        std::process::exit(1);
                    }
                };
                let report = gsd_meta_manager::envelope::scan::scan_with_external(
                    &root,
                    gsd_meta_manager::envelope::scan::ScanLimits::default(),
                );
                // Stdout, because this entry point is read by a human. The
                // report is safe to print by construction: it carries file,
                // line and rule, and has nowhere to hold a matched secret.
                // READ by a human — this entry point prints to stdout for a
                // person, as the comment above says — so the alias is escaped.
                // The scan itself was handed `&root`, not this string.
                // One composition for every CLI echo (WR-01, `21-24`). Like the
                // `add` arm's two sites, this value passed `Alias::new`, so both
                // classes are already impossible and the call is a no-op today.
                println!(
                    "alias={} root={}",
                    gsd_meta_manager::text::render_for_terminal(alias.as_str()),
                    root.display()
                );
                print!("{}", report.render());
                // The exit code IS the control (D-25): a finding exits non-zero.
                std::process::exit(if report.is_clean() { 0 } else { 1 });
            }
        },
        None => {
            // TUI mode
            //
            // `App::new` runs BEFORE `tui::init()` (quick 260930-vvk): the
            // launch target is resolved against the loaded registry and a
            // refusal must exit before any terminal setup (D-03). As a side
            // effect, a config-load error no longer leaves the terminal in raw
            // mode on the alternate screen.
            let mut app = App::new(config_path)?;
            let launch_alias = match cli.target.as_deref() {
                None => None,
                Some(target) => {
                    let cwd = match std::env::current_dir() {
                        Ok(cwd) => cwd,
                        Err(err) => {
                            eprintln!("Error: cannot resolve the current directory: {err}");
                            std::process::exit(1);
                        }
                    };
                    match gsd_meta_manager::launch_target::resolve_launch_target(
                        &app.ctx.config,
                        target,
                        &cwd,
                    ) {
                        Ok(alias) => Some(alias),
                        Err(gsd_meta_manager::launch_target::LaunchTargetError::Unregistered {
                            path,
                            target,
                        }) => {
                            // A GSD project folder that is not registered yet
                            // is added under its folder name, then opened.
                            match gsd_meta_manager::launch_target::auto_register(
                                &mut app.ctx.config,
                                &path,
                            ) {
                                Some(alias) => {
                                    save_config(&app.ctx.config, &app.ctx.config_path)?;
                                    Some(alias)
                                }
                                None => {
                                    let err = gsd_meta_manager::launch_target::LaunchTargetError::Unregistered {
                                        path,
                                        target,
                                    };
                                    eprintln!("Error: {err}");
                                    eprintln!("{}", err.suggestion());
                                    std::process::exit(1);
                                }
                            }
                        }
                        Err(err) => {
                            // `Display` escapes the target and path (D-21-3).
                            eprintln!("Error: {err}");
                            eprintln!("{}", err.suggestion());
                            std::process::exit(1);
                        }
                    }
                }
            };
            let mut terminal = tui::init();
            app.load_project_states();

            app.init_change_tracker();
            // One startup status line: the installed gsd-core against the
            // version this build is synced to. All fs reads live in the App
            // method, so this async body stays free of inline fs calls.
            app.announce_gsd_install();

            let event_bus = EventBus::new();

            // Store event_tx on App context so creation flow can send actions back
            app.ctx.event_tx = Some(event_bus.tx.clone());

            // Initialize file watcher for all registered projects
            let mut watcher = FileWatcher::new(event_bus.tx.clone())?;
            for project in app.ctx.config.projects.values() {
                let planning_dir = project.path.join(".planning");
                if planning_dir.is_dir() {
                    if let Err(e) = watcher.watch(&planning_dir) {
                        tracing::warn!("Could not watch {}: {}", planning_dir.display(), e);
                    }
                }
            }

            // Store watcher on App context so new projects can be watched dynamically
            app.ctx.watcher = Some(watcher);

            // One-off cleanup (quick 261103-o2r, D-B): recover dead runs, slim landed
            // ones, drop orphans. Off the render thread; one status line comes back.
            app.start_oneoff_recycle();

            // Startup scan: auto-register any active Claude sessions whose
            // working_dir is an unregistered GSD project. The session poll
            // tick handles the same logic ongoing (every ~5s), but this
            // closes the gap between launch and the first poll.
            let initial_sessions = gsd_meta_manager::session_detector::detect_sessions();
            app.active_sessions = initial_sessions.clone();
            app.ctx.active_sessions = initial_sessions;
            app.auto_register_new_sessions();

            // The launch target opens through the overview's own Enter path,
            // after every piece of startup state is in place, so the first
            // frame is the project view and Esc behaves as if drilled in (D-05).
            if let Some(alias) = launch_alias {
                if !app.open_project_view(&alias) {
                    tracing::warn!(
                        "launch target {} resolved but its project view could not be opened",
                        gsd_meta_manager::text::render_for_terminal(&alias)
                    );
                    app.ctx.status_message = Some((
                        format!(
                            "Could not open project '{}'",
                            gsd_meta_manager::text::render_for_terminal(&alias)
                        ),
                        std::time::Instant::now(),
                    ));
                }
            }

            let input = match event_bus.spawn_input_reader() {
                Ok(r) => r,
                Err(e) => {
                    tui::restore();
                    return Err(e.into());
                }
            };
            // Keep the 250ms tick. It drives the 20-tick session poll and the
            // 3s status-message expiry; `pump`'s 16ms redraw interval is a
            // separate concern and purely additive. Removing this would
            // silently break session detection.
            event_bus.spawn_tick(250);

            let mut rx = event_bus.rx;

            let result = run_tui_loop(&mut terminal, &mut app, &mut rx, &input).await;

            tui::restore();

            result?;
        }
    }

    Ok(())
}

/// Draw, [`pump`], the editor shell-out (through `input`, which is parked while
/// the editor owns the terminal), the quit check.
///
/// Everything that used to be a bare `rx.recv().await` here now lives in
/// `gsd_meta_manager::main_loop::pump`, in the **library**, because this file is
/// the binary and nothing in it is reachable from a `cargo test --lib` target.
/// That placement is what makes TRANS-03 testable at all.
async fn run_tui_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<gsd_meta_manager::action::Action>,
    input: &gsd_meta_manager::input::InputReader,
) -> anyhow::Result<()> {
    loop {
        // Apply the desired mouse capture (quick 260926-dyf, I-14). This is
        // also the session's FIRST enable: it runs after every fallible startup
        // `?` in main's TUI arm, and every exit from this function is followed
        // by `tui::restore()`, which turns capture off.
        if app.mouse_capture != tui::mouse_captured() {
            if let Err(e) = tui::set_mouse_capture(app.mouse_capture) {
                app.mouse_capture = tui::mouse_captured();
                app.ctx.status_message = Some((
                    format!("Mouse capture unavailable: {e}"),
                    std::time::Instant::now(),
                ));
                app.needs_redraw = true;
            }
        }

        // Sync needs_redraw from ctx (screens set ctx.needs_redraw)
        if app.ctx.needs_redraw {
            app.needs_redraw = true;
            app.ctx.needs_redraw = false;
        }

        if app.needs_redraw {
            terminal.draw(|frame| gsd_meta_manager::ui::render(frame, app))?;
            app.needs_redraw = false;
        }

        if pump(app, rx).await == PumpOutcome::Idle {
            // Every arm disabled: the action channel is closed, so no further
            // message can ever arrive and continuing would spin. Unreachable
            // while the redraw timer arm exists, but breaking is the only
            // non-spinning response if that ever changes.
            tracing::warn!("event loop: all message sources closed, exiting");
            break;
        }

        // Check if a screen action requested an editor launch
        if let Some((path, line)) = app.pending_editor.take() {
            // Determine editor: $VISUAL > $EDITOR > vi
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".to_string());

            // The library decides the program and argv (WR-06: `code -w` runs
            // through `sh`, the path always a separate argument); this is the one
            // place the process is built.
            let (program, argv) = gsd_meta_manager::app::editor_command(&editor, &path, line);

            // Suspend the TUI. D-05: mouse capture goes off before the terminal
            // is handed to the editor (`Handoff::release_terminal` -> `restore`),
            // and comes back on return only if it was on. The input reader is
            // parked first, so it cannot race the editor for keystrokes
            // (quick 261103-tn4).
            let status =
                run_with_terminal_suspended(input, &mut tui::Handoff::new(terminal), || {
                    std::process::Command::new(&program).args(&argv).status()
                });

            // The exit status also reaches the library: a DIRECTION.md edit temp is
            // applied (or kept, on a failed editor) by `App::after_editor`, not here.
            let exit = match status {
                Ok(s) if s.success() => {
                    app.ctx.status_message = Some((
                        format!(
                            "Editor closed: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        std::time::Instant::now(),
                    ));
                    gsd_meta_manager::app::EditorExit::Success
                }
                Ok(s) => {
                    app.ctx.status_message = Some((
                        format!("Editor exited with: {}", s),
                        std::time::Instant::now(),
                    ));
                    gsd_meta_manager::app::EditorExit::Failed(s.to_string())
                }
                Err(e) => {
                    app.ctx.status_message = Some((
                        format!("Failed to launch {}: {}", editor, e),
                        std::time::Instant::now(),
                    ));
                    gsd_meta_manager::app::EditorExit::SpawnError(format!("{editor}: {e}"))
                }
            };

            // The edit may have changed the file the browser is showing; a known edit
            // temp is applied.
            app.after_editor(&path, exit);

            app.needs_redraw = true;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
