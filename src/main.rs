mod event;
mod tui;

use gsd_meta_manager::app::App;
use gsd_meta_manager::watcher::FileWatcher;
use clap::Parser;
use gsd_meta_manager::cli::{Cli, Commands, EnvelopeAction};
use gsd_meta_manager::config::{load_config, save_config, Config};
use gsd_meta_manager::driver::{drive, DriveArgs, RawDriveArgs};
use event::EventBus;
use gsd_meta_manager::main_loop::{
    pump, ExecEvent, PumpOutcome, EXEC_BATCH, EXEC_CHANNEL_CAPACITY,
};
use gsd_meta_manager::registry::{add_project, list_projects, remove_project};

/// Judge an alias arriving on an envelope re-entry, or fail closed with
/// `refusal_code` after naming the way out.
///
/// **T-21-17-07, the consequence D-17-1/D-17-2 create and this function is the
/// route out of.** Pass 6 measured that an older build DID register invisible
/// and look-alike aliases. Those entries are still in `config.json`, `list`
/// still renders them, and the hook stubs installed for them still pass their
/// alias on re-entry — but this build no longer admits the value. The refusal is
/// correct (the tool genuinely cannot tell which project the value names), and
/// on its own it is a dead end: the user meets it as a bare `exit 1` on
/// `git push` for a project that worked yesterday. So the message names the
/// recovery route, which is `remove` — deliberately left raw for exactly this
/// (D-17-3) — followed by a re-add under a visible alias.
///
/// It emits no credential and echoes no payload beyond the alias itself, so the
/// askpass arm's redaction contract holds through it.
fn judged_alias_or_exit(raw: &str, refusal_code: i32) -> gsd_meta_manager::registry::Alias {
    match gsd_meta_manager::registry::Alias::new(raw) {
        Ok(alias) => alias,
        Err(refusal) => {
            // The refused value is UNTRUSTED and the refusal message quotes it,
            // so it is escaped before it reaches a terminal (D-19-5). A refusal
            // that reported a bidi override by rendering one would let the
            // rejected value reorder the sentence explaining why it was
            // rejected.
            //
            // **The escaping is no longer done HERE** (D-21-3, 21-21). It is
            // done once, in `Display for AliasRefusal`, which is the one
            // producer every echo of this type goes through. This site used to
            // wrap `refusal.to_string()` in a second `display_identity` call — a
            // second spelling of one judgment, which is precisely the defect
            // D-19-2 removed from `Alias::new`, and which left the OTHER two
            // echo sites (`:91` and `:359` before this edit) depending on
            // whoever remembered to copy it. Removing it is behaviour-preserving
            // and that is not assumed: `display_identity` is pinned idempotent
            // over its own output by
            // `registry::tests::escaping_an_already_escaped_identity_changes_nothing`.
            eprintln!("Error: {refusal}");
            eprintln!(
                "This alias was accepted by an older build and no longer names a valid \
                 identity, so this hook refuses rather than guessing which project it \
                 means. To recover: `gsd-meta-manager remove <alias>` (removal still \
                 accepts it), then re-add the project under a visible alias."
            );
            std::process::exit(refusal_code);
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with file appender before anything else
    let log_dir = dirs::data_local_dir()
        .map(|d| d.join("gsd-meta-manager"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    let file_appender = tracing_appender::rolling::daily(&log_dir, "gsd-meta-manager.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    color_eyre::install().map_err(|e| anyhow::anyhow!("{}", e))?;

    let cli = Cli::parse();
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
            let config = load_config(&config_path)?;
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
            add_project(&mut config, &alias, &canonical_path)?;
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
            let config = load_config(&config_path)?;
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
                    // **`.to_string()` is load-bearing here and is not
                    // cosmetic.** `Rendered`'s `Display` is
                    // `f.write_str(&self.0)`, which IGNORES the formatter's
                    // width and fill — so `{:<20}` applied to a `Rendered`
                    // silently emits no padding at all and this table loses its
                    // columns. Measured: the first `render_for_terminal` here
                    // printed `clean /tmp` where it had printed
                    // `clean                /tmp`. The correct fix is
                    // `f.pad(&self.0)` in `impl Display for Rendered`, but
                    // `src/text.rs` belongs to plan `21-23` and is off-limits to
                    // this plan's diff, so it is reported as a wave-conflict
                    // finding and worked around at this one call site instead.
                    println!(
                        "{:<20} {:<50} {}",
                        gsd_meta_manager::text::render_for_terminal(alias).to_string(),
                        project.path.display(),
                        project.added
                    );
                }
            }
        }
        // The two `#[cfg(debug_assertions)]` fields below are D-30/WR-16: the
        // agent-override flags have no parser entry in a release build, so this
        // arm must not name them there either. The attribute rides the pattern
        // field and the struct-expression field rather than forking this arm in
        // two — one arm that goes out of sync with the other is exactly the bug
        // a cfg is supposed to prevent.
        Some(Commands::Drive {
            alias,
            command,
            target_phase,
            max_steps,
            wall_clock_cap_secs,
            max_escalations,
            approved_plan,
            run_id,
            dry_run,
            goal,
            #[cfg(debug_assertions)]
            claude_program,
            #[cfg(debug_assertions)]
            claude_args,
        }) => {
            // This arm is by construction BEFORE `tui::init()` in the `None`
            // arm below, which is the whole of "the driver never touches
            // ratatui" — no new mechanism, just the position in this match.
            let config = load_config(&config_path)?;
            // **The parse boundary, and it is the ONLY route from argv into a
            // `DriveArgs`.** Clap's fields are raw `String`s — clap parses a
            // command line, it does not judge payloads — so `RawDriveArgs` is
            // what crosses, and `DriveArgs::from_argv` is what judges. A value
            // carrying nothing visible is refused HERE, before `drive` is
            // entered and therefore before any file, lock, journal or run
            // directory can exist, identically for `--dry-run` and a real run
            // because the refusal fires before a `DriveArgs` exists for
            // `dry_run` to be read off.
            let raw = RawDriveArgs {
                alias,
                command,
                target_phase,
                max_steps,
                wall_clock_cap_secs,
                max_escalations,
                approved_plan,
                run_id,
                dry_run,
                goal,
                #[cfg(debug_assertions)]
                claude_program,
                #[cfg(debug_assertions)]
                claude_args,
            };
            let args = match DriveArgs::from_argv(raw) {
                Ok(args) => args,
                Err(err) => {
                    // The same house shape the `drive` refusal below uses:
                    // user-facing refusals in this binary print and exit, they
                    // do not bubble as an anyhow chain.
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            };
            if let Err(err) = drive(args, &config).await {
                // The `Add` arm's house shape: user-facing refusals in this
                // binary print and exit, they do not bubble as an anyhow chain.
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
        // Like the `Drive` arm above, this one is BEFORE `tui::init()` by
        // construction — and here the position is doing more work than there. A
        // git hook's stdout and stderr belong to the pushing git process; a
        // terminal put into raw mode by ratatui on the way past would corrupt
        // the very output that carries the refusal.
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
        Some(Commands::Envelope { action }) => match action {
            EnvelopeAction::PrePush { alias, hook_path } => {
                let alias = judged_alias_or_exit(&alias, 1);
                let stdin = std::io::stdin();
                // git runs a hook with the working directory at the top of the
                // worktree, which is what makes the scan's root and the path
                // checks' root the same root without a flag to get wrong.
                let repo_root = match std::env::current_dir() {
                    Ok(dir) => dir,
                    Err(err) => {
                        eprintln!("Error: cannot resolve the repository root: {err}");
                        std::process::exit(1);
                    }
                };
                match gsd_meta_manager::envelope::hooks::pre_push(
                    &alias,
                    stdin.lock(),
                    &hook_path,
                    &repo_root,
                ) {
                    // The exit code IS the control (D-25): git blocks the push
                    // on any non-zero exit, and nothing downstream has to parse
                    // a message to learn that it was blocked.
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // The `Add` arm's house shape, and fail-closed: a hook
                        // that could not do its job must not let the push
                        // through.
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::PreCommit { alias, hook_path } => {
                let alias = judged_alias_or_exit(&alias, 1);
                let repo_root = match std::env::current_dir() {
                    Ok(dir) => dir,
                    Err(err) => {
                        eprintln!("Error: cannot resolve the repository root: {err}");
                        std::process::exit(1);
                    }
                };
                match gsd_meta_manager::envelope::hooks::pre_commit(
                    &alias,
                    &hook_path,
                    &repo_root,
                ) {
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed, exactly as the push hook does: a hook
                        // that could not do its job must not let the commit
                        // through.
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::Askpass {
                alias,
                host,
                prompt,
            } => {
                // Fail-closed emits NO credential, which is the redaction
                // contract: the hint may name the recovery command, and names
                // nothing else.
                let alias = judged_alias_or_exit(&alias, 1);
                // The credential goes to stdout and the refusal goes to stderr,
                // both inside the handler — nothing is printed here, because a
                // second print site is a second place a secret could be echoed.
                match gsd_meta_manager::envelope::cred::askpass_with_config(
                    &config_path,
                    &alias,
                    &prompt,
                    &host,
                ) {
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed, and redacted: an askpass that cannot do
                        // its job emits no credential, and its diagnostic passes
                        // through the already-shipped redaction path rather than
                        // a forked one.
                        eprintln!(
                            "Error: {}",
                            gsd_meta_manager::journal::redact::redact(&err.to_string())
                        );
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::Guard { alias } => {
                // Exit 2 is this arm's deny, so its fail-closed code is 2.
                let alias = judged_alias_or_exit(&alias, 2);
                // Before `tui::init()` for the same reason the hook arms are:
                // this process's stdout carries the permission decision the
                // agent CLI reads, and a terminal put into raw mode on the way
                // past would corrupt the very answer that does the blocking.
                let stdin = std::io::stdin();
                match gsd_meta_manager::envelope::hooks::guard(&alias, stdin.lock()) {
                    // The exit code is the second carrier (2 blocks); the JSON
                    // on stdout is the first. Neither depends on the other.
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed and redacted: a guard that cannot do its
                        // job denies, and its diagnostic passes through the
                        // already-shipped redaction path rather than a forked
                        // one — the request it could not judge is a command
                        // line, which is a thing that can carry a token.
                        eprintln!(
                            "Error: {}",
                            gsd_meta_manager::journal::redact::redact(&err.to_string())
                        );
                        std::process::exit(2);
                    }
                }
            }
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
                // The exit code IS the control (D-25), here as much as in the
                // hook: a finding exits non-zero.
                std::process::exit(if report.is_clean() { 0 } else { 1 });
            }
        },
        None => {
            // TUI mode
            let mut terminal = tui::init();
            let mut app = App::new(config_path)?;
            app.load_project_states();

            app.init_change_tracker();

            let event_bus = EventBus::new();

            // The executor event channel is created ONCE for the process
            // lifetime, is SEPARATE from the Action FIFO, and is BOUNDED
            // (D-17). Each of those three properties guards a distinct failure:
            //
            // * once-for-the-lifetime + the sender clone stashed below means it
            //   never closes, so its `select!` arm can never permanently
            //   disable itself between runs (Pitfall C, T-15-27);
            // * separate means bulk stream traffic cannot starve control keys
            //   behind the biased-first Action arm (TRANS-03, T-15-25);
            // * bounded means a render loop parked in the blocking editor
            //   shell-out applies backpressure to the reader instead of growing
            //   without limit (T-15-26).
            let (exec_tx, mut exec_rx) =
                tokio::sync::mpsc::channel::<ExecEvent>(EXEC_CHANNEL_CAPACITY);

            // Store event_tx on App context so creation flow can send actions back
            app.ctx.event_tx = Some(event_bus.tx.clone());
            app.ctx.exec_tx = Some(exec_tx.clone());

            // Initialize file watcher for all registered projects
            let mut watcher = FileWatcher::new(event_bus.tx.clone())?;
            for project in app.ctx.config.projects.values() {
                let planning_dir = project.path.join(".planning");
                if planning_dir.is_dir() {
                    if let Err(e) = watcher.watch(&planning_dir) {
                        tracing::warn!(
                            "Could not watch {}: {}",
                            planning_dir.display(),
                            e
                        );
                    }
                }
            }

            // Store watcher on App context so new projects can be watched dynamically
            app.ctx.watcher = Some(watcher);

            // Startup scan: auto-register any active Claude sessions whose
            // working_dir is an unregistered GSD project. The session poll
            // tick handles the same logic ongoing (every ~5s), but this
            // closes the gap between launch and the first poll.
            let initial_sessions = gsd_meta_manager::session_detector::detect_sessions();
            app.active_sessions = initial_sessions.clone();
            app.ctx.active_sessions = initial_sessions;
            app.auto_register_new_sessions();

            // Startup scan, second half: find driver runs that outlived a
            // previous TUI session. The 20-tick block handles the same logic
            // ongoing (every ~5s), but this closes the gap between launch and
            // the first poll — which is the whole of "reopening the TUI shows
            // that run still live" (CTRL-04, D-13).
            //
            // Synchronous for the same reason the session scan above is: the TUI
            // is not yet in its loop, so there is no render thread to block.
            //
            // The scan writes nothing (D-12). A run whose driver died without an
            // `ended_at` comes back with `live == false` and its record is left
            // exactly as the dead driver left it.
            app.ctx.observed_runs =
                gsd_meta_manager::driver::reconcile::reconcile_all(&app.ctx.config.projects)
                    .into_iter()
                    .map(|run| (run.alias.clone(), run))
                    .collect();
            // Its sibling, read at the same moment and for the same reason
            // (WR-02): D-14's finished-run arm has to be answerable on the very
            // first frame, or a project whose last run failed shows no badge
            // until the first 20-tick scan lands five seconds later.
            app.ctx.last_outcomes =
                gsd_meta_manager::driver::reconcile::last_ended_outcomes(&app.ctx.config.projects);

            event_bus.spawn_crossterm_reader();
            // Keep the 250ms tick. It drives the 20-tick session poll and the
            // 3s status-message expiry; `pump`'s 16ms redraw interval is a
            // separate concern and purely additive. Removing this would
            // silently break session detection.
            event_bus.spawn_tick(250);

            let mut rx = event_bus.rx;

            let result = run_tui_loop(&mut terminal, &mut app, &mut rx, &mut exec_rx).await;

            tui::restore();

            // Held to here on purpose: this binding plus the clone on
            // `app.ctx` are what keep the executor channel open for the whole
            // process lifetime.
            drop(exec_tx);

            result?;
        }
    }

    Ok(())
}

/// Draw, [`pump`], the editor shell-out, the quit check.
///
/// Everything that used to be a bare `rx.recv().await` here now lives in
/// `gsd_meta_manager::main_loop::pump`, in the **library**, because this file is
/// the binary and nothing in it is reachable from a `cargo test --lib` target.
/// That placement is what makes TRANS-03 testable at all.
async fn run_tui_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<gsd_meta_manager::action::Action>,
    exec_rx: &mut tokio::sync::mpsc::Receiver<ExecEvent>,
) -> anyhow::Result<()> {
    loop {
        // Sync needs_redraw from ctx (screens set ctx.needs_redraw)
        if app.ctx.needs_redraw {
            app.needs_redraw = true;
            app.ctx.needs_redraw = false;
        }

        if app.needs_redraw {
            terminal.draw(|frame| gsd_meta_manager::ui::render(frame, app))?;
            app.needs_redraw = false;
        }

        if pump(app, rx, exec_rx, EXEC_BATCH).await == PumpOutcome::Idle {
            // Every arm disabled: both channels are closed, so no further
            // message can ever arrive and continuing would spin. Unreachable
            // while the redraw timer arm exists, but breaking is the only
            // non-spinning response if that ever changes.
            tracing::warn!("event loop: all message sources closed, exiting");
            break;
        }

        // Check if a screen action requested an editor launch
        if let Some(path) = app.pending_editor.take() {
            // Suspend the TUI
            ratatui::restore();

            // Determine editor: $VISUAL > $EDITOR > vi
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".to_string());

            // Spawn editor and wait for it to exit
            let status = std::process::Command::new(&editor).arg(&path).status();

            match status {
                Ok(s) if s.success() => {
                    app.ctx.status_message = Some((
                        format!(
                            "Editor closed: {}",
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ),
                        std::time::Instant::now(),
                    ));
                }
                Ok(s) => {
                    app.ctx.status_message = Some((
                        format!("Editor exited with: {}", s),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    app.ctx.status_message = Some((
                        format!("Failed to launch {}: {}", editor, e),
                        std::time::Instant::now(),
                    ));
                }
            }

            // Re-initialize TUI
            *terminal = ratatui::init();
            app.needs_redraw = true;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
