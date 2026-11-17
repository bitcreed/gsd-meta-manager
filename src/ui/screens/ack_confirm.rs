//! The confirm popup in front of `A` (37 D-16): it shows the owner exactly what
//! gsd-core is about to write, and only Yes runs anything.
//!
//! **Default No, Yellow accent.** The write is one frontmatter key that
//! `git checkout` undoes, so nothing here is drawn Red. The popup names the
//! project, the file, the category and the milestone, says that the marker is
//! file-level (it hides every item of the file in every lane, blocked rows
//! included) and lapses when the file changes, that gsd-core edits the
//! frontmatter and commits nothing, and shows the exact argv that Yes runs.
//!
//! **A cut-off popup cannot be confirmed** (the existing `confirm_popup`
//! guarantee): at a size where the body does not fit, `y` and Enter-on-Yes do
//! nothing and `n` / `Esc` still cancel.
//!
//! **Yes re-checks before it runs** (T-37-20): the group is looked up again by its
//! identity, and the target it resolves to now must equal the one that was shown.
//! Anything else says why and starts nothing.

use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::detail::shown;
use super::inbox::InboxScreen;
use super::owner_actions::{self, AckRequest};
use super::owner_block;
use super::{AppContext, Screen, ScreenAction};
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

/// The screen the popup was opened over, and so the background it dims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AckHost {
    /// The Owner inbox; its state is on `ctx.inbox_view`.
    Inbox,
    /// A project's detail view (the Owner block's `A`).
    Detail(String),
}

pub struct AckConfirmScreen {
    host: AckHost,
    request: AckRequest,
    popup: ConfirmPopup,
    /// The runner Yes hands to `start_owner_acknowledge_with`; `None` is the real
    /// one. Only tests set it, so none of them starts a real child.
    runner: Option<fn(&AckRequest) -> String>,
}

impl AckConfirmScreen {
    pub(crate) fn new(host: AckHost, request: AckRequest) -> Self {
        Self {
            host,
            request,
            popup: ConfirmPopup::default(),
            runner: None,
        }
    }

    #[cfg(test)]
    fn with_runner(mut self, runner: fn(&AckRequest) -> String) -> Self {
        self.runner = Some(runner);
        self
    }

    /// Yes: the status line says why nothing started, or the child starts.
    fn confirm(&self, ctx: &mut AppContext) {
        let request = &self.request;
        let say = |ctx: &mut AppContext, message: String| {
            ctx.status_message = Some((message, std::time::Instant::now()));
        };
        let group = ctx
            .owner_digest(&request.alias)
            .and_then(|digest| owner_block::group_for(digest, &request.key.key));
        let Some(group) = group else {
            say(
                ctx,
                "Group is no longer listed; nothing acknowledged".to_owned(),
            );
            return;
        };
        if let Err(message) = owner_actions::ack_recheck(ctx, request, &group) {
            say(ctx, message);
            return;
        }
        match self.runner {
            Some(runner) => ctx.start_owner_acknowledge_with(request.clone(), runner),
            None => ctx.start_owner_acknowledge(request.clone()),
        }
    }
}

/// `{N} item(s)` in the popup's own wording.
fn items_phrase(count: usize) -> String {
    if count == 1 {
        "1 item".to_owned()
    } else {
        format!("{count} items")
    }
}

/// `text` cut into rows of at most `width` cells at `/` and spaces, so a path or a
/// command never breaks inside a name. A piece longer than a whole row (no `/` or
/// space to break at) is cut at the cell boundary as the last resort. A break at a
/// space drops that space; a break at `/` keeps it on the row it ends.
pub(crate) fn wrap_at_separators(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for piece in text.split_inclusive(['/', ' ']) {
        let trimmed = piece.trim_end_matches(' ');
        let mut rest = piece;
        // A row that cannot hold the whole piece starts a new one first.
        if !row.is_empty() && owner_block::cells(&row) + owner_block::cells(trimmed) > width {
            rows.push(std::mem::take(&mut row).trim_end().to_owned());
        }
        // A piece wider than a row is cut at the cell boundary.
        while owner_block::cells(rest.trim_end_matches(' ')) > width {
            let mut cut = String::new();
            let mut used = 0;
            for ch in rest.chars() {
                let w = owner_block::cells(&ch.to_string());
                if used + w > width {
                    break;
                }
                cut.push(ch);
                used += w;
            }
            if cut.is_empty() {
                break;
            }
            rest = &rest[cut.len()..];
            if !row.is_empty() {
                rows.push(std::mem::take(&mut row).trim_end().to_owned());
            }
            rows.push(cut);
        }
        row.push_str(rest);
    }
    if !row.is_empty() || rows.is_empty() {
        rows.push(row.trim_end().to_owned());
    }
    rows
}

/// The popup body (UI-SPEC "A confirm popup"): BOLD labels, every dynamic value
/// through `shown`. The argv is the program and every element of the request,
/// space-joined, each element escaped on its own, in normal weight because it is
/// what the owner consents to. `inner_w` is the popup's inner width: the label
/// and value share a row when they fit, a longer value wraps at `/` and spaces
/// under a hanging indent, and the argv continues two cells in.
pub(crate) fn ack_body(req: &AckRequest, inner_w: u16) -> Vec<Line<'static>> {
    const LABEL_W: usize = 11;
    let inner_w = usize::from(inner_w);
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let field = |label: &str, value: String| -> Vec<Line<'static>> {
        let parts = wrap_at_separators(&value, inner_w.saturating_sub(LABEL_W));
        parts
            .into_iter()
            .enumerate()
            .map(|(i, part)| {
                let head = if i == 0 {
                    Span::styled(format!("{label:<LABEL_W$}"), bold)
                } else {
                    Span::raw(" ".repeat(LABEL_W))
                };
                Line::from(vec![head, Span::raw(part)])
            })
            .collect()
    };
    let mut category = req.target.category.to_owned();
    if let Some(label) = &req.target.archived_milestone {
        category.push_str(&format!(" \u{b7} archived {}", shown(label)));
    }
    let argv = req
        .command_line()
        .into_iter()
        .map(shown)
        .collect::<Vec<_>>()
        .join(" ");
    let mut body: Vec<Line<'static>> = Vec::new();
    body.extend(field("Project", shown(&req.alias)));
    body.extend(field("File", shown(&req.target.rel_file)));
    body.extend(field("Category", category));
    body.extend(field("Milestone", shown(&req.target.milestone)));
    body.push(Line::raw(""));
    body.push(Line::raw(format!(
        "Hides all {} of this file in every lane, blocked rows included. The marker lapses by itself when the file changes.",
        items_phrase(req.target.items)
    )));
    body.push(Line::raw(""));
    body.push(Line::raw(format!(
        "gsd-core edits the file's frontmatter. Nothing is committed: {} gets one modified file in its working tree.",
        shown(&req.alias)
    )));
    body.push(Line::raw(""));
    for (i, part) in wrap_at_separators(&argv, inner_w.saturating_sub(2))
        .into_iter()
        .enumerate()
    {
        let indent = if i == 0 { "" } else { "  " };
        body.push(Line::raw(format!("{indent}{part}")));
    }
    body
}

crate::ui::screens::adjudicate_screen!(
    AckConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registered alias, the project-relative file, the archived \
     milestone label, the milestone and the whole argv (including the \
     gsd-tools path) read out of `.planning/` and the install, each through \
     `shown`, over the inbox or the detail view as its background. Fixture \
     states: one popup with hostile values at the sizes the guard renders.",
);

impl Screen for AckConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                self.confirm(ctx);
                ScreenAction::Pop
            }
            ConfirmOutcome::Cancel => ScreenAction::Pop,
            ConfirmOutcome::Pending => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        match &self.host {
            AckHost::Inbox => InboxScreen::new().render(frame, area, ctx),
            AckHost::Detail(alias) => {
                super::detail::DetailScreen::new(alias.clone()).render_main_only(frame, area, ctx)
            }
        }
        confirm_popup::dim_background(frame, area);
        self.popup.record(confirm_popup::render_confirm_popup(
            frame,
            area,
            "Acknowledge via gsd-core",
            ack_body(&self.request, confirm_popup::body_room(area).0),
            Color::Yellow,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "ack_confirm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{Kind, Lane, OwnerDigest, OwnerGroup};
    use crate::ui::screens::tests::ctx_with_aliases;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ALIAS: &str = "alder-api";
    const FILE: &str = ".planning/phases/24-x/24-UAT.md";

    fn group() -> OwnerGroup {
        let mut g = crate::ui::screens::owner_block::sample_owner_digest()
            .groups
            .into_iter()
            .find(|g| g.kind == Kind::Uat && g.lane == Lane::Test && g.phase == "24")
            .expect("a TEST UAT group of phase 24");
        g.file = FILE.to_owned();
        g.counts.checks = 6;
        g
    }

    /// A context holding one project with one acknowledgeable group, and the
    /// request `A` would build for it. No test depends on this machine's install.
    fn setup() -> (AppContext, AckRequest) {
        let mut ctx = ctx_with_aliases(&[ALIAS]);
        ctx.project_states.get_mut(ALIAS).unwrap().milestone = "v2.0".to_owned();
        let g = group();
        ctx.owner_digests.insert(
            ALIAS.to_owned(),
            OwnerDigest {
                groups: vec![g.clone()],
                ..OwnerDigest::default()
            },
        );
        owner_actions::force_launcher(Some(owner_actions::test_launcher()));
        let request = owner_actions::ack_request(&ctx, ALIAS, &g).expect("a request");
        (ctx, request)
    }

    /// The request as a `node` install builds it: the shim path ahead of the argv.
    fn through_node(mut request: AckRequest) -> AckRequest {
        request.program = "node".to_owned();
        request.argv.insert(
            0,
            "/home/alder/.claude/gsd-core/bin/gsd-tools.cjs".to_owned(),
        );
        request
    }

    fn rows(screen: &AckConfirmScreen, ctx: &AppContext, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn press(screen: &mut AckConfirmScreen, ctx: &mut AppContext, code: KeyCode) -> ScreenAction {
        screen.handle_key(code, KeyModifiers::NONE, ctx)
    }

    static YES_RUNS: AtomicUsize = AtomicUsize::new(0);
    fn yes_runner(_: &AckRequest) -> String {
        YES_RUNS.fetch_add(1, Ordering::SeqCst);
        "ran".to_owned()
    }
    static GONE_RUNS: AtomicUsize = AtomicUsize::new(0);
    fn gone_runner(_: &AckRequest) -> String {
        GONE_RUNS.fetch_add(1, Ordering::SeqCst);
        "ran".to_owned()
    }
    static BUSY_RUNS: AtomicUsize = AtomicUsize::new(0);
    fn busy_runner(_: &AckRequest) -> String {
        BUSY_RUNS.fetch_add(1, Ordering::SeqCst);
        "ran".to_owned()
    }
    static CANCEL_RUNS: AtomicUsize = AtomicUsize::new(0);
    fn cancel_runner(_: &AckRequest) -> String {
        CANCEL_RUNS.fetch_add(1, Ordering::SeqCst);
        "ran".to_owned()
    }
    static CLIP_RUNS: AtomicUsize = AtomicUsize::new(0);
    fn clip_runner(_: &AckRequest) -> String {
        CLIP_RUNS.fetch_add(1, Ordering::SeqCst);
        "ran".to_owned()
    }

    #[test]
    fn enter_defaults_to_no() {
        let (mut ctx, request) = setup();
        let mut screen = AckConfirmScreen::new(AckHost::Inbox, request).with_runner(cancel_runner);
        let action = press(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(matches!(action, ScreenAction::Pop));
        assert!(ctx.status_message.is_none(), "No says nothing");
        assert!(ctx.owner_ack_busy.is_empty());
        assert_eq!(CANCEL_RUNS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn escape_cancels_and_starts_nothing() {
        let (mut ctx, request) = setup();
        for code in [KeyCode::Esc, KeyCode::Char('n')] {
            let mut screen =
                AckConfirmScreen::new(AckHost::Inbox, request.clone()).with_runner(cancel_runner);
            assert!(matches!(
                press(&mut screen, &mut ctx, code),
                ScreenAction::Pop
            ));
        }
        assert!(ctx.status_message.is_none());
        assert!(ctx.owner_ack_busy.is_empty());
        assert_eq!(CANCEL_RUNS.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn yes_starts_one_bounded_child() {
        let (mut ctx, request) = setup();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        let mut screen = AckConfirmScreen::new(AckHost::Inbox, request).with_runner(yes_runner);
        let action = press(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert!(matches!(action, ScreenAction::Pop));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Acknowledging 24-UAT.md via gsd-core...")
        );
        assert!(ctx
            .owner_ack_busy
            .contains(&(ALIAS.to_owned(), FILE.to_owned())));
        let Some(crate::action::Action::OwnerAcknowledged {
            alias,
            file,
            outcome,
        }) = rx.recv().await
        else {
            panic!("the child reports through OwnerAcknowledged");
        };
        assert_eq!(
            (alias.as_str(), file.as_str(), outcome.as_str()),
            (ALIAS, FILE, "ran")
        );
        assert_eq!(YES_RUNS.load(Ordering::SeqCst), 1, "exactly one child");
    }

    #[tokio::test]
    async fn a_vanished_group_is_not_acknowledged() {
        let (mut ctx, request) = setup();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        // The refresh between `A` and Yes dropped the group.
        ctx.owner_digests
            .insert(ALIAS.to_owned(), OwnerDigest::default());
        let mut screen = AckConfirmScreen::new(AckHost::Inbox, request).with_runner(gone_runner);
        let action = press(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert!(matches!(action, ScreenAction::Pop));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Group is no longer listed; nothing acknowledged")
        );
        assert!(ctx.owner_ack_busy.is_empty());
        assert_eq!(GONE_RUNS.load(Ordering::SeqCst), 0);
    }

    /// IN-01: busy is per file path, not per basename, so the same-named file of
    /// another workstream or milestone is not blocked.
    #[tokio::test]
    async fn a_same_named_file_elsewhere_is_not_busy() {
        let (mut ctx, _request) = setup();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        ctx.owner_ack_busy.insert((
            ALIAS.to_owned(),
            ".planning/workstreams/beta/phases/24-x/24-UAT.md".to_owned(),
        ));
        assert!(
            owner_actions::ack_open(&ctx, ALIAS, &group(), AckHost::Inbox).is_ok(),
            "another file with the same basename is not busy"
        );
    }

    #[tokio::test]
    async fn a_busy_file_is_not_acknowledged_twice() {
        let (mut ctx, request) = setup();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        ctx.owner_ack_busy
            .insert((ALIAS.to_owned(), FILE.to_owned()));
        // `A` on a busy file never opens the popup.
        let g = group();
        assert_eq!(
            owner_actions::ack_open(&ctx, ALIAS, &g, AckHost::Inbox)
                .err()
                .as_deref(),
            Some("Already acknowledging 24-UAT.md")
        );
        // And a popup that was open already refuses at Yes.
        let mut screen = AckConfirmScreen::new(AckHost::Inbox, request).with_runner(busy_runner);
        let action = press(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert!(matches!(action, ScreenAction::Pop));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Already acknowledging 24-UAT.md")
        );
        assert_eq!(BUSY_RUNS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_popup_fits_80x24_with_the_exact_argv() {
        let (ctx, request) = setup();
        let request = through_node(request);
        let command_line: Vec<String> = request
            .command_line()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let screen = AckConfirmScreen::new(AckHost::Inbox, request);
        let text = rows(&screen, &ctx, 80, 24);
        assert!(!screen.popup.clipped(), "{text}");
        for want in [
            "Acknowledge via gsd-core",
            "Project    alder-api",
            "File       .planning/phases/24-x/24-UAT.md",
            "Category   uat_gaps",
            "Milestone  v2.0",
            "Hides all 6 items of this file in every lane, blocked rows included.",
            "The marker lapses by itself when the file changes.",
            "gsd-core edits the file's frontmatter. Nothing is committed:",
            "[ Yes ]",
            "[ No ]",
        ] {
            assert!(text.contains(want), "{want:?} missing from\n{text}");
        }
        assert_eq!(screen.popup.focus, confirm_popup::ConfirmFocus::No);
        // The exact argv: every element is on screen, in order.
        let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut from = flat.find("node").expect("the program is drawn");
        for element in &command_line {
            let at = flat[from..]
                .find(element.as_str())
                .unwrap_or_else(|| panic!("{element:?} is not drawn in order in\n{text}"));
            from += at + element.len();
        }
        assert!(
            command_line.iter().any(|e| e == "--phase") && command_line.iter().any(|e| e == "24"),
            "{command_line:?}"
        );
    }

    /// UI review 37 #3: a long file path and launcher path wrap at `/` and
    /// spaces under a hanging indent, never inside a name, at 100 columns.
    #[test]
    fn long_paths_wrap_at_separators_with_a_hanging_indent_at_100x30() {
        let (ctx, mut request) = setup();
        request.target.rel_file =
            ".planning/workstreams/v0.16-long-name/phases/42.1-some-long-title-here/42.1-VERIFICATION.md"
                .to_owned();
        request.program = "node".to_owned();
        request.argv.insert(
            0,
            "/home/alder/.local/share/claude-profiles/some-profile/gsd-core/bin/gsd-tools.cjs"
                .to_owned(),
        );
        let screen = AckConfirmScreen::new(AckHost::Inbox, request);
        let text = rows(&screen, &ctx, 100, 30);
        assert!(!screen.popup.clipped(), "{text}");
        let inner: Vec<String> = text
            .lines()
            .filter_map(|l| {
                let c: Vec<char> = l.chars().collect();
                let from = c.iter().position(|ch| *ch == '\u{2502}')?;
                let to = c.iter().rposition(|ch| *ch == '\u{2502}')?;
                (to > from).then(|| c[from + 1..to].iter().collect::<String>())
            })
            .collect();
        // The label and the first segment share a row.
        let file_at = inner
            .iter()
            .position(|l| l.starts_with("File       .planning/"))
            .unwrap_or_else(|| panic!("File label alone or missing:\n{text}"));
        // Continuations hang under the value column and start at a name.
        let next = &inner[file_at + 1];
        assert!(next.starts_with(&format!("{:<11}", "")), "{next:?}");
        assert!(
            inner[file_at].trim_end().ends_with('/'),
            "{:?}",
            inner[file_at]
        );
        let joined: String = inner[file_at..]
            .iter()
            .take(2)
            .map(|l| l.trim().trim_start_matches("File").trim().to_owned())
            .collect();
        assert!(
            joined.ends_with("42.1-some-long-title-here/42.1-VERIFICATION.md"),
            "{joined}"
        );
        // The argv: `node` is not alone on its row, and gsd-tools.cjs is whole.
        assert!(
            inner.iter().any(|l| l.contains("node /home/alder/")),
            "{text}"
        );
        assert!(inner.iter().any(|l| l.contains("gsd-tools.cjs")), "{text}");
        for l in &inner {
            assert_ne!(l.trim(), "node", "{text}");
        }
    }

    #[test]
    fn the_popup_clips_at_60x20_and_cannot_confirm() {
        let (mut ctx, request) = setup();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        let mut screen =
            AckConfirmScreen::new(AckHost::Inbox, through_node(request)).with_runner(clip_runner);
        let text = rows(&screen, &ctx, 60, 20);
        assert!(screen.popup.clipped(), "{text}");
        assert!(
            text.contains(confirm_popup::CLIPPED_HINT.trim_end()),
            "{text}"
        );
        // `y` and Enter-on-Yes do nothing while the body is cut off.
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Char('y')),
            ScreenAction::None
        ));
        press(&mut screen, &mut ctx, KeyCode::Left);
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Enter),
            ScreenAction::None
        ));
        assert!(ctx.owner_ack_busy.is_empty());
        assert_eq!(CLIP_RUNS.load(Ordering::SeqCst), 0);
        // `n` and Esc still cancel.
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Char('n')),
            ScreenAction::Pop
        ));
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
    }
    /// OWN-11 backstop: a hostile alias, file, label and launcher path (ESC, C0,
    /// bidi, zero-width) leave no raw control cell, at the sizes the popup is
    /// specified for.
    #[test]
    fn a_hostile_popup_draws_no_raw_control_cell() {
        let hostile = "x\u{1b}[2J\u{7}\u{202e}y\u{200b}z\u{85}";
        let (mut ctx, mut request) = setup();
        request.alias = hostile.to_owned();
        request.target.rel_file = hostile.to_owned();
        request.target.milestone = hostile.to_owned();
        request.target.archived_milestone = Some(hostile.to_owned());
        request.program = hostile.to_owned();
        request.argv.insert(0, hostile.to_owned());
        ctx.status_message = Some((
            crate::text::render_for_terminal(&format!("Acknowledge failed: {hostile}")).to_string(),
            std::time::Instant::now(),
        ));
        let screen = AckConfirmScreen::new(AckHost::Inbox, request);
        for (w, h) in [(80, 24), (60, 20)] {
            let text = rows(&screen, &ctx, w, h);
            for c in text.chars().filter(|c| *c != '\n') {
                assert!(
                    !c.is_control() && !matches!(c, '\u{202e}' | '\u{200b}'),
                    "{w}x{h}: raw {c:?} in\n{text}"
                );
            }
        }
    }
}
