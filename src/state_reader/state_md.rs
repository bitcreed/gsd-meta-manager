use serde_yml::{Mapping, Value};

/// GSD's STATE.md frontmatter, read field-by-field rather than all-or-nothing.
///
/// **Why this is not `#[derive(Deserialize)]`.** A derived struct is a single
/// unit of failure: one value of an unexpected shape anywhere aborts the whole
/// deserialize, `parse_state_md` returns `None`, and *every* field is lost —
/// indistinguishably from an absent STATE.md. That is exactly how a
/// `gsd_state_version: "1.0"` (quoted, which is what GSD actually writes)
/// against a `f64` field blanked the phase label for every registered project.
///
/// `#[serde(default)]` does not prevent it: a default covers an **absent**
/// field, never a **present** field of the wrong shape.
///
/// So the frontmatter is read into a [`Mapping`] first and each field is pulled
/// out through the lenient accessors below ([`scalar_string`], [`scalar_u32`]).
/// A value that cannot be read degrades **that field** to its default and
/// nothing else. The tolerance is structural, not opt-in: reading a mapping is
/// the only way to populate a field, so a field added later is tolerant by
/// construction rather than by remembering an attribute.
#[derive(Debug, Default, Clone)]
pub struct StateFrontmatter {
    /// GSD's schema-version marker. Opaque and comparable — see [`StateVersion`].
    /// `None` when absent or unreadable; never a reason to reject the document.
    pub gsd_state_version: Option<StateVersion>,
    pub milestone: String,
    pub milestone_name: String,
    pub status: String,
    pub stopped_at: String,
    pub last_updated: String,
    pub last_activity: Option<String>,
    /// ADR-2207: current phase number (GSD 1.8.0 frontmatter). Stored as a
    /// string but accepts a YAML string OR number.
    pub current_phase: Option<String>,
    /// ADR-2207: human-readable current phase name (GSD 1.8.0 frontmatter).
    pub current_phase_name: Option<String>,
    /// ADR-2207: current plan identifier (GSD 1.8.0 frontmatter). Accepts a
    /// YAML string OR number (e.g. `"0.3"` or `14`).
    pub current_plan: Option<String>,
    pub progress: ProgressInfo,
}

/// The `gsd_state_version` this reader was written against.
///
/// Not a floor and not a ceiling — only the point of reference for
/// [`StateVersion::is_newer_than_supported`]. A STATE.md above it still reads.
pub const SUPPORTED_STATE_VERSION: (u64, u64) = (1, 0);

/// GSD's `gsd_state_version`, kept as an **opaque, comparable** value.
///
/// GSD writes it quoted (`"1.0"`) in every STATE.md this tool has ever seen,
/// but has also written it bare (`1.0`); both are the same version and neither
/// is a reason to refuse the file. The raw text is preserved verbatim so an
/// unrecognised form round-trips into diagnostics unchanged, while the numeric
/// components — as many as the string carries — drive ordering.
///
/// A version this reader does not recognise is **not** an error. Refusing to
/// read a newer GSD's state file would reintroduce the very class of bug this
/// type exists to close; the reader degrades to whatever fields it understands.
///
/// Equality is **semantic, not textual**: `"1.0"`, `1.0` (which YAML normalises
/// to `1`) and `1.0.0` are one version, and comparing them by their spelling
/// would recreate the quoting sensitivity in a new place.
#[derive(Debug, Clone)]
pub struct StateVersion {
    raw: String,
    parts: Vec<u64>,
}

impl StateVersion {
    /// Read a version from its textual form. Infallible by design: a value that
    /// carries no numbers at all still yields a `StateVersion` that compares
    /// below every numbered one and prints its original text.
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        let parts = trimmed
            .split('.')
            .map(|seg| {
                let digits: String = seg.chars().take_while(char::is_ascii_digit).collect();
                digits.parse::<u64>().unwrap_or(0)
            })
            .collect();
        Self {
            raw: trimmed.to_string(),
            parts,
        }
    }

    /// The version as the YAML scalar delivered it — verbatim for a quoted
    /// value, YAML-normalised for a bare one (`1.0` arrives as `1`). Use it for
    /// diagnostics; use the comparison operators for decisions.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The leading numeric component, or 0 when the value carries none.
    pub fn major(&self) -> u64 {
        self.parts.first().copied().unwrap_or(0)
    }

    /// True when this version sorts above [`SUPPORTED_STATE_VERSION`].
    ///
    /// Advisory only. It exists so a caller can *say* the file came from a
    /// newer GSD, never so a caller can decline to read it.
    pub fn is_newer_than_supported(&self) -> bool {
        let (major, minor) = SUPPORTED_STATE_VERSION;
        *self > Self::from_parts(major, minor)
    }

    fn from_parts(major: u64, minor: u64) -> Self {
        Self {
            raw: format!("{}.{}", major, minor),
            parts: vec![major, minor],
        }
    }
}

impl PartialEq for StateVersion {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for StateVersion {}

impl PartialOrd for StateVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StateVersion {
    /// Component-wise, shorter forms zero-extended, so `1.0` == `1.0.0` and
    /// `1.10` > `1.9`.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let len = self.parts.len().max(other.parts.len());
        for i in 0..len {
            let a = self.parts.get(i).copied().unwrap_or(0);
            let b = other.parts.get(i).copied().unwrap_or(0);
            match a.cmp(&b) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            }
        }
        std::cmp::Ordering::Equal
    }
}

impl std::fmt::Display for StateVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

/// Read any YAML **scalar** as its string form.
///
/// String, integer, float and bool all have an unambiguous textual reading, and
/// GSD quotes fields inconsistently between versions, so accepting all of them
/// costs nothing. A sequence, mapping or null has no scalar reading and yields
/// `None` — the field degrades, the document does not.
fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Read any YAML scalar as a `u32` count.
///
/// Accepts integers, integral-valued floats (`4.0`), and numeric strings
/// (`"4"`) — the three shapes a count has been written in. A negative or
/// fractional number, or anything non-numeric, yields `None` rather than a
/// wrong count.
fn scalar_u32(value: &Value) -> Option<u32> {
    // Via the textual form deliberately: it is the one reading that is the same
    // for every scalar shape the value could arrive in, so this helper does not
    // have to track the YAML library's integer/float representation choices.
    let text = scalar_string(value)?;
    let text = text.trim();
    if let Ok(n) = text.parse::<u32>() {
        return Some(n);
    }
    let f = text.parse::<f64>().ok()?;
    if f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f <= f64::from(u32::MAX) {
        Some(f as u32)
    } else {
        None
    }
}

/// Look up a key in a frontmatter mapping, ignoring a null value the way an
/// absent key is ignored (GSD writes `field:` with no value for "not set").
fn field<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    match map.get(key) {
        Some(Value::Null) | None => None,
        Some(v) => Some(v),
    }
}

/// A field read as a plain `String`, defaulting to empty.
fn string_field(map: &Mapping, key: &str) -> String {
    field(map, key).and_then(scalar_string).unwrap_or_default()
}

/// A field read as `Option<String>`; `None` when absent, null or unreadable.
fn opt_string_field(map: &Mapping, key: &str) -> Option<String> {
    field(map, key).and_then(scalar_string)
}

/// ADR-2207: true when `status` denotes milestone termination — the milestone
/// is fully done (`<version> milestone complete`) or the project is between
/// milestones (`Awaiting next milestone`). Case-insensitive.
pub fn is_milestone_terminal(status: &str) -> bool {
    let s = status.to_lowercase();
    s.contains("milestone complete") || s.contains("awaiting next milestone")
}

/// ADR-2207: true when `status` is the intermediate `All phases complete`
/// state — every phase is done but the milestone has NOT been terminated
/// (the project awaits `/gsd:complete-milestone`). Case-insensitive.
pub fn is_all_phases_complete(status: &str) -> bool {
    status.to_lowercase().contains("all phases complete")
}

/// G11: true when a project's `status` is the hard-stop `error` or `failed`.
///
/// A **predicate over the existing field, not a new field.** Whether an error
/// status parks a run is the router's disposition to make; the reader's job is
/// only to make the fact readable as a gate rather than as ordinary status text
/// (`next.md:60-69`).
///
/// Matched exactly, case-insensitively, after trimming. Deliberately not a
/// substring test: a `stopped_at` of "failed to reach the registry" or a status
/// of "recovered from error" is prose about a past failure, not a project in
/// one, and a substring match would park on both.
pub fn is_error_status(status: &str) -> bool {
    let s = status.trim().to_ascii_lowercase();
    s == "error" || s == "failed"
}

/// G15: the phases named by the `## Deferred Verification` table in STATE.md.
///
/// GSD's autonomous workflow routes a `verification_deferred_human` row to
/// `handle_blocker` (`autonomous.md:478-481`). The table's first column is the
/// phase; the header row (first cell `Phase`) and the alignment row are skipped,
/// and a row with an empty first cell contributes nothing.
///
/// Fail-safe: an absent section, a malformed table or a table with no data rows
/// all yield an empty vector, never an error.
pub fn deferred_verification_phases(content: &str) -> Vec<String> {
    let mut phases: Vec<String> = Vec::new();
    let mut in_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Any heading opens or closes the section scope, so a table further
        // down the document is never read as this section's.
        if trimmed.starts_with('#') {
            in_section = trimmed
                .trim_start_matches('#')
                .trim()
                .eq_ignore_ascii_case("Deferred Verification");
            continue;
        }
        if !in_section || !trimmed.starts_with('|') {
            continue;
        }

        let cells: Vec<String> = trimmed
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_string())
            .collect();

        // Alignment row (`|---|---|`).
        if cells
            .iter()
            .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            continue;
        }

        let Some(first) = cells.first() else { continue };
        let phase = first.trim().trim_matches('*').trim();
        // Header row, identified by NAME rather than by position, so a table
        // written without a header still contributes its rows.
        if phase.eq_ignore_ascii_case("Phase") || phase.is_empty() {
            continue;
        }
        phases.push(phase.to_string());
    }

    phases
}

/// The `progress:` sub-mapping, read with the same per-field tolerance as its
/// parent: an unreadable count is 0, and it does not take the others with it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProgressInfo {
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
    pub percent: u32,
}

impl ProgressInfo {
    fn from_mapping(map: &Mapping) -> Self {
        let count = |key: &str| field(map, key).and_then(scalar_u32).unwrap_or(0);
        Self {
            total_phases: count("total_phases"),
            completed_phases: count("completed_phases"),
            total_plans: count("total_plans"),
            completed_plans: count("completed_plans"),
            percent: count("percent"),
        }
    }
}

/// Extract YAML frontmatter from a GSD STATE.md file.
/// Returns None if the file doesn't start with `---`.
pub fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    // Skip the opening ---
    let after_open = &content[3..];
    let after_open = after_open.trim_start_matches(['\r', '\n']);

    // Find closing --- (must be on its own line)
    if let Some(end) = after_open.find("\n---") {
        Some(&after_open[..end])
    } else {
        None
    }
}

/// What reading a STATE.md's frontmatter produced.
///
/// **`Absent` and `Unreadable` are separate on purpose.** Collapsing them into
/// one `None` is what let the original bug ship: a STATE.md that existed but
/// failed to parse was indistinguishable from a project that has none, so the
/// dashboard printed a plausible-looking wrong number instead of admitting it
/// could not read the file.
#[derive(Debug, Clone)]
pub enum FrontmatterOutcome {
    /// Frontmatter read. Individual fields may have degraded to their defaults;
    /// the document as a whole was understood.
    Parsed(Box<StateFrontmatter>),
    /// No frontmatter block at all — the file does not open with `---`.
    Absent,
    /// Frontmatter read, but only after [`repair_frontmatter_block`] rewrote
    /// `repaired_lines` of it **in memory**. The file on disk is unchanged.
    ///
    /// **The third state this enum's own argument demands.** `Absent` and
    /// `Unreadable` are separate because collapsing outcomes is what let the
    /// original bug ship; a repaired document is a third thing again — not
    /// clean, because this tool rewrote it before believing it, and not
    /// unreadable, because the values did arrive. A `bool` on `Parsed` would
    /// let a caller that never reads the flag present a repaired file as a
    /// clean one; a variant makes every exhaustive match site a compile error
    /// until it has classified the new state.
    Recovered {
        frontmatter: Box<StateFrontmatter>,
        /// How many lines the repair rewrote. Always ≥ 1 — a repair that
        /// changed nothing yields `Parsed`.
        repaired_lines: u32,
    },
    /// A frontmatter block is present but could not be read as a YAML mapping.
    Unreadable {
        fault: FrontmatterFault,
        /// Where the fault is, when the parser could say — FILE-relative.
        /// `None` for every fault this crate classified itself, and for the
        /// parser errors that carry no location.
        position: Option<FrontmatterFaultPosition>,
    },
}

/// Where a YAML fault is, as **numbers**: 1-based line and column, counted in
/// the FILE rather than in the frontmatter body the parser was handed.
///
/// **The only thing the parser knows that may cross into a rendered string.**
/// The message it came from quotes the third-party document — that is the
/// reasoning [`FrontmatterFault`]'s own doc makes one field over, and the
/// reason this is a pair of `u32`s rather than the message that carried them.
/// A number cannot carry an instruction, and it cannot be prose an untrusted
/// repository wrote.
///
/// A position that cannot name a line in the block it came from is dropped
/// rather than clamped ([`file_relative_position`]), which also bounds the
/// width of any label built from it by the file's own line count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontmatterFaultPosition {
    pub line: u32,
    pub column: u32,
}

/// Why a present frontmatter block could not be read.
///
/// **A classification rather than the parser's message, deliberately.** A
/// serde error quotes the document that produced it, and that document is
/// third-party text (`src/driver/untrusted.rs` is the census of every such
/// string this tool holds). Carrying it as a `String` on `ProjectState` would
/// put unclassified third-party prose one field-access away from a model seam
/// to buy a line number. The full parser message still goes to the log; what
/// the UI carries is a fixed phrase this crate wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontmatterFault {
    /// Opened with `---` and never closed.
    Unterminated,
    /// Valid YAML, but a sequence or a bare scalar rather than a mapping.
    NotAMapping,
    /// Not valid YAML at all.
    InvalidYaml,
}

impl FrontmatterFault {
    /// A fixed, this-crate-authored description. Never third-party text.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Unterminated => "frontmatter block is never closed by a `---` line",
            Self::NotAMapping => "frontmatter is not a YAML mapping",
            Self::InvalidYaml => "frontmatter is not valid YAML",
        }
    }
}

impl std::fmt::Display for FrontmatterFault {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.describe())
    }
}

/// Translate a parser-reported, BODY-relative position into a FILE-relative
/// one, or drop it.
///
/// - `first_body_file_line` is the FILE line that body line 1 occupies — 2 for
///   an ordinary `---\n…` file, more when the file opens with blank lines.
/// - `body_lines` is how many lines the frontmatter block itself has, which is
///   what makes the validity test a check against measured reality rather than
///   an invented ceiling.
///
/// Yields `None` for a 0 line, for a line past the block's own end, and on any
/// narrowing overflow — a position that wrapped would be a confident wrong
/// number, which is the class of answer this whole module refuses to give.
fn file_relative_position(
    first_body_file_line: u64,
    body_lines: u64,
    body_line: u64,
    column: u64,
) -> Option<FrontmatterFaultPosition> {
    if body_line == 0 {
        return None;
    }
    let file_line = body_line.checked_add(first_body_file_line)?.checked_sub(1)?;
    // The last nameable line of the block, in file coordinates.
    let last_file_line = body_lines.checked_add(first_body_file_line)?.checked_sub(1)?;
    if file_line > last_file_line {
        return None;
    }
    Some(FrontmatterFaultPosition {
        line: u32::try_from(file_line).ok()?,
        column: u32::try_from(column).ok()?,
    })
}

/// The FILE-relative position of a `serde_yml` error raised against `body`, a
/// subslice of `content`.
///
/// The derivation, written down because it is the part that is easy to get
/// subtly wrong: `body` is a slice **into** `content`, so the distance between
/// their start pointers is the byte length of everything before it — the
/// leading whitespace, the opening `---` and its newline. Counting the `\n` in
/// that prefix and adding 1 gives the FILE line that body line 1 occupies.
/// (`content.len() - body.len()` would NOT do: it also counts the document
/// body after the closing `---`, which is most of the file.)
fn yaml_fault_position(
    content: &str,
    body: &str,
    error: &serde_yml::Error,
) -> Option<FrontmatterFaultPosition> {
    let location = error.location()?;
    let prefix_len = (body.as_ptr() as usize).checked_sub(content.as_ptr() as usize)?;
    let prefix = content.get(..prefix_len)?;
    let first_body_file_line = u64::try_from(prefix.chars().filter(|&c| c == '\n').count())
        .ok()?
        .checked_add(1)?;
    let body_lines = u64::try_from(body.lines().count()).ok()?;
    file_relative_position(
        first_body_file_line,
        body_lines,
        u64::try_from(location.line()).ok()?,
        u64::try_from(location.column()).ok()?,
    )
}

/// One conservative quote-in-place pass over a frontmatter block that failed to
/// parse.
///
/// Returns the rewritten block and the number of lines it changed, or `None`
/// when it changed nothing. **`None` is the contract that protects every valid
/// file**: a block this function declines is byte-identical to the one it was
/// given, so no valid document's meaning can be altered by the repair path
/// existing. The tests assert `None` directly rather than merely observing that
/// the error branch never fired.
///
/// The fault it exists for is the one measured on a real foreign STATE.md: a
/// top-level PLAIN scalar containing a colon followed by a space, which YAML
/// reads as a nested mapping key and then refuses ("mapping values are not
/// allowed in this context"). Quoting that one value makes the whole document
/// parse.
///
/// A line is a candidate only when EVERY one of these holds. Anything uncertain
/// is left exactly as it is — the repair may only ever fix the one shape it
/// understands:
///
/// - no leading whitespace (a top-level mapping entry; an indented key is out
///   of scope, and an indented line may be the continuation of a quoted scalar);
/// - it does not open with `#` (a comment) or `-` (a list item);
/// - it splits on the FIRST `:` that is followed by a space or ends the line;
/// - the key is a plain identifier-shaped token — this is what keeps a
///   continuation line of a multi-line quoted scalar from being rewritten;
/// - the value, trimmed, is non-empty (so `progress:` opening a nested mapping
///   is never touched);
/// - the value does not open with a quote (`"` `'`), a block-scalar indicator
///   (`|` `>`), a flow opener (`[` `{`), a comment (`#`) or an anchor/alias/tag
///   sigil (`&` `*` `!`);
/// - and the value is genuinely not a legal plain scalar: it carries a
///   colon-space, or it ends on a bare colon.
///
/// All of it is done over `char`s. The value is third-party text and a
/// multi-byte character is exactly what arrives without warning — the defect
/// class `crate::driver::untrusted::bounded` documents.
fn repair_frontmatter_block(yaml: &str) -> Option<(String, u32)> {
    let mut repaired_lines = 0u32;
    let mut out = String::with_capacity(yaml.len() + 16);
    for (index, raw_line) in yaml.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        // A `\r` belongs to the line terminator, not to the value; it is
        // stripped for the decision and restored verbatim on re-emission.
        let (line, carriage_return) = match raw_line.strip_suffix('\r') {
            Some(stripped) => (stripped, true),
            None => (raw_line, false),
        };
        match repair_line(line) {
            Some(fixed) => {
                repaired_lines += 1;
                out.push_str(&fixed);
            }
            None => out.push_str(line),
        }
        if carriage_return {
            out.push('\r');
        }
    }
    (repaired_lines > 0).then_some((out, repaired_lines))
}

/// The candidate rule for a single line. `None` means "leave it alone".
fn repair_line(line: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let first = *chars.first()?;
    if first.is_whitespace() || first == '#' || first == '-' {
        return None;
    }
    // The FIRST `:` that is followed by a space or ends the line.
    let split_at = chars
        .iter()
        .position(|&c| c == ':')
        .filter(|&i| chars.get(i + 1).is_none_or(|&c| c == ' '))?;
    let key: String = chars[..split_at].iter().collect();
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return None;
    }
    let value: String = chars[split_at + 1..].iter().collect();
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let opener = value.chars().next()?;
    if matches!(opener, '"' | '\'' | '|' | '>' | '[' | '{' | '&' | '*' | '!' | '#') {
        return None;
    }
    // A value with none of these is a legal plain scalar; rewriting it would
    // change a document that was never at fault on this line.
    if !value.contains(": ") && !value.ends_with(':') {
        return None;
    }
    Some(format!("{key}: {}", yaml_double_quoted(value)))
}

/// A value wrapped as a YAML double-quoted scalar.
///
/// Char-wise, never byte-wise: a naive byte scan for `"` or `\` over a value
/// carrying em-dashes and 4-byte characters is the panic class this crate has
/// already paid for twice (`untrusted::bounded`, `ui::roadmap_widget`).
fn yaml_double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Build a [`StateFrontmatter`] from a frontmatter mapping.
///
/// One construction site for both the clean and the repaired path, so a field
/// added to one cannot go missing from the other.
fn frontmatter_from_mapping(map: &Mapping) -> StateFrontmatter {
    let progress = match field(map, "progress") {
        Some(Value::Mapping(m)) => ProgressInfo::from_mapping(m),
        _ => ProgressInfo::default(),
    };
    StateFrontmatter {
        gsd_state_version: opt_string_field(map, "gsd_state_version")
            .as_deref()
            .map(StateVersion::parse),
        milestone: string_field(map, "milestone"),
        milestone_name: string_field(map, "milestone_name"),
        status: string_field(map, "status"),
        stopped_at: string_field(map, "stopped_at"),
        last_updated: string_field(map, "last_updated"),
        last_activity: opt_string_field(map, "last_activity"),
        current_phase: opt_string_field(map, "current_phase"),
        current_phase_name: opt_string_field(map, "current_phase_name"),
        current_plan: opt_string_field(map, "current_plan"),
        progress,
    }
}

/// Read STATE.md content, distinguishing "no frontmatter" from "broken
/// frontmatter".
///
/// Only two things can make frontmatter unreadable now: an unterminated block,
/// or YAML that is not a mapping. Everything else — an unknown key, a value of
/// an unexpected shape, a version from a newer GSD — degrades the individual
/// field and leaves the rest intact.
pub fn read_frontmatter(content: &str) -> FrontmatterOutcome {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return FrontmatterOutcome::Absent;
    }
    // `---\n---` is a *closed but empty* block. `extract_frontmatter` cannot see
    // it (it searches for a `\n---` that an empty body never contains), and
    // "empty" must not be reported as "broken" — nothing failed to be read.
    let body = trimmed[3..].trim_start_matches(['\r', '\n']);
    if body == "---" || body.starts_with("---\n") || body.starts_with("---\r\n") {
        return FrontmatterOutcome::Parsed(Box::default());
    }
    let Some(yaml) = extract_frontmatter(content) else {
        let fault = FrontmatterFault::Unterminated;
        tracing::warn!("Unreadable STATE.md frontmatter: {}", fault);
        // No parser ran, so there is nothing to locate.
        return FrontmatterOutcome::Unreadable {
            fault,
            position: None,
        };
    };

    let (value, repaired_lines): (Value, u32) = match serde_yml::from_str(yaml) {
        Ok(v) => (v, 0),
        Err(e) => {
            // The parser's message quotes the document, so it goes to the log
            // and stops there; the caller gets the classification.
            tracing::warn!("Unreadable STATE.md frontmatter: {}", e);
            // Taken from the FIRST error, so the position names where the
            // document as written broke — not where the rewritten one did.
            let unreadable = FrontmatterOutcome::Unreadable {
                fault: FrontmatterFault::InvalidYaml,
                position: yaml_fault_position(content, yaml, &e),
            };
            // ONE repair attempt and ONE reparse. Not a loop: a second pass
            // would be this tool arguing with a document it already failed to
            // understand, and every additional rewrite widens what it can
            // silently change.
            match repair_frontmatter_block(yaml) {
                Some((repaired, count)) => match serde_yml::from_str::<Value>(&repaired) {
                    Ok(v @ Value::Mapping(_)) => (v, count),
                    _ => return unreadable,
                },
                None => return unreadable,
            }
        }
    };

    let map = match value {
        // `---\n---` is an empty block, not a broken one.
        Value::Null => Mapping::new(),
        Value::Mapping(m) => m,
        _ => {
            let fault = FrontmatterFault::NotAMapping;
            tracing::warn!("Unreadable STATE.md frontmatter: {}", fault);
            // This crate's own classification of a document the parser
            // accepted; there is no fault position to report.
            return FrontmatterOutcome::Unreadable {
                fault,
                position: None,
            };
        }
    };

    let frontmatter = Box::new(frontmatter_from_mapping(&map));
    if repaired_lines > 0 {
        FrontmatterOutcome::Recovered {
            frontmatter,
            repaired_lines,
        }
    } else {
        FrontmatterOutcome::Parsed(frontmatter)
    }
}

/// Parse STATE.md content into a [`StateFrontmatter`].
///
/// Returns `None` only when there is no readable frontmatter block at all.
/// Callers that must tell an absent block from a broken one want
/// [`read_frontmatter`].
pub fn parse_state_md(content: &str) -> Option<StateFrontmatter> {
    match read_frontmatter(content) {
        FrontmatterOutcome::Parsed(fm) => Some(*fm),
        // A repaired file IS readable through this entry point — the caller
        // that needs to know it was repaired uses `read_frontmatter`.
        FrontmatterOutcome::Recovered { frontmatter, .. } => Some(*frontmatter),
        FrontmatterOutcome::Absent | FrontmatterOutcome::Unreadable { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter_basic() {
        let content = "---\nstatus: planning\n---\n# Body";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("status: planning"));
    }

    #[test]
    fn test_extract_frontmatter_with_hr_in_body() {
        let content = "---\nstatus: planning\n---\n# Body\n\n---\n\nMore content";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("status: planning"));
        assert!(!fm.contains("More content"));
    }

    #[test]
    fn test_extract_frontmatter_empty() {
        assert!(extract_frontmatter("").is_none());
    }

    #[test]
    fn test_extract_frontmatter_no_delimiters() {
        assert!(extract_frontmatter("# Just markdown").is_none());
    }

    /// The shape GSD actually writes, copied from a real `.planning/STATE.md`:
    /// `gsd_state_version` is QUOTED, `current_phase` is bare. The fixture below
    /// this one keeps the unquoted-version form, so both are covered.
    ///
    /// This is the fixture the original bug needed and did not have — every
    /// pre-existing test wrote a version form GSD never emits, so they all
    /// passed while every real project's phase label was blank.
    const REAL_GSD_STATE_MD: &str = "---\ngsd_state_version: \"1.0\"\nmilestone: v2.0\nmilestone_name: Autonomous Orchestration\ncurrent_phase: 19\ncurrent_phase_name: GITSAFE — Git & Blast-Radius Envelope\nstatus: verifying\nstopped_at: Completed 19-33-PLAN.md\nlast_updated: \"2026-09-10T02:35:05.863Z\"\nlast_activity: 2026-09-08\nstate_head: ae76ce98b0b6ecd90cb48efa68cb624b8d4e3c57\nprogress:\n  total_phases: 10\n  completed_phases: 6\n  total_plans: 112\n  completed_plans: 112\n  percent: 60\n---\n\n# Project State\n";

    #[test]
    fn the_quoted_version_gsd_actually_writes_does_not_discard_the_frontmatter() {
        let fm = parse_state_md(REAL_GSD_STATE_MD).expect("real GSD frontmatter must parse");
        // The field that used to abort the whole parse.
        assert_eq!(fm.gsd_state_version.as_ref().unwrap().raw(), "1.0");
        // And everything that used to be lost along with it.
        assert_eq!(fm.status, "verifying");
        assert_eq!(fm.current_phase.as_deref(), Some("19"));
        assert_eq!(
            fm.current_phase_name.as_deref(),
            Some("GITSAFE — Git & Blast-Radius Envelope")
        );
        assert_eq!(fm.milestone, "v2.0");
        assert_eq!(fm.milestone_name, "Autonomous Orchestration");
        assert_eq!(fm.progress.total_phases, 10);
        assert_eq!(fm.progress.completed_phases, 6);
    }

    #[test]
    fn test_parse_state_md_real_content() {
        // Unquoted version — the older form. Still accepted.
        let content = "---\ngsd_state_version: 1.0\nmilestone: v1.0\nmilestone_name: milestone\nstatus: planning\nstopped_at: Phase 1 context gathered\nlast_updated: \"2026-03-25T04:22:49.224Z\"\nprogress:\n  total_phases: 4\n  completed_phases: 0\n  total_plans: 0\n  completed_plans: 0\n  percent: 0\n---\n# Project State\n\n---\n\nSome body content";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "planning");
        assert_eq!(fm.progress.total_phases, 4);
        assert_eq!(fm.progress.completed_phases, 0);
        assert_eq!(fm.milestone, "v1.0");
        assert_eq!(
            fm.gsd_state_version,
            Some(StateVersion::parse("1.0")),
            "the bare form is the same version as the quoted one"
        );
    }

    #[test]
    fn quoted_and_unquoted_versions_are_the_same_version() {
        let quoted = parse_state_md("---\ngsd_state_version: \"1.0\"\nstatus: a\n---\n").unwrap();
        let bare = parse_state_md("---\ngsd_state_version: 1.0\nstatus: a\n---\n").unwrap();
        assert_eq!(quoted.gsd_state_version, bare.gsd_state_version);
    }

    #[test]
    fn a_version_from_a_newer_gsd_is_read_not_refused() {
        let fm = parse_state_md("---\ngsd_state_version: \"2.4\"\nstatus: executing\ncurrent_phase: 7\n---\n")
            .expect("an unrecognised version must not abort the parse");
        let v = fm.gsd_state_version.as_ref().unwrap();
        assert_eq!(v.raw(), "2.4");
        assert!(v.is_newer_than_supported());
        // Degrading sensibly means the fields it DOES understand still arrive.
        assert_eq!(fm.status, "executing");
        assert_eq!(fm.current_phase.as_deref(), Some("7"));
    }

    #[test]
    fn a_missing_version_is_not_a_failure() {
        let fm = parse_state_md("---\nstatus: executing\ncurrent_phase: 3\n---\n").unwrap();
        assert!(fm.gsd_state_version.is_none());
        assert_eq!(fm.current_phase.as_deref(), Some("3"));
    }

    #[test]
    fn a_garbage_version_leaves_every_other_field_intact() {
        let fm = parse_state_md(
            "---\ngsd_state_version: [1, 0]\nstatus: executing\ncurrent_phase: 5\nmilestone: v3.0\n---\n",
        )
        .expect("a garbage version must degrade itself, not the document");
        assert!(fm.gsd_state_version.is_none());
        assert_eq!(fm.status, "executing");
        assert_eq!(fm.current_phase.as_deref(), Some("5"));
        assert_eq!(fm.milestone, "v3.0");
    }

    #[test]
    fn a_garbage_value_in_one_field_does_not_take_the_others_with_it() {
        // Every field the tool reads, each given a shape it cannot possibly
        // want. Each must degrade alone.
        let content = "---\ngsd_state_version: \"1.0\"\nstatus: executing\nmilestone:\n  nested: mapping\ncurrent_phase_name: [a, list]\ncurrent_phase: 12\nstopped_at: 42\nprogress:\n  total_phases: 8\n  completed_phases: not-a-number\n  percent: 12.5\n---\n";
        let fm = parse_state_md(content).expect("one bad field must not discard the rest");
        assert_eq!(fm.status, "executing");
        assert_eq!(fm.current_phase.as_deref(), Some("12"));
        assert_eq!(fm.progress.total_phases, 8);
        // Degraded, individually.
        assert_eq!(fm.milestone, "");
        assert_eq!(fm.current_phase_name, None);
        assert_eq!(fm.progress.completed_phases, 0);
        // A scalar of the wrong primitive type is still readable as text.
        assert_eq!(fm.stopped_at, "42");
        // ...and an integral float count reads as the count it plainly is.
        assert_eq!(fm.progress.percent, 0); // 12.5 is not a whole percent
    }

    #[test]
    fn counts_written_as_quoted_strings_or_whole_floats_still_count() {
        let fm = parse_state_md(
            "---\nprogress:\n  total_phases: \"8\"\n  completed_phases: 3.0\n  total_plans: 31\n---\n",
        )
        .unwrap();
        assert_eq!(fm.progress.total_phases, 8);
        assert_eq!(fm.progress.completed_phases, 3);
        assert_eq!(fm.progress.total_plans, 31);
    }

    #[test]
    fn an_absent_frontmatter_and_a_broken_one_are_distinguishable() {
        assert!(matches!(
            read_frontmatter("# Just a heading\n"),
            FrontmatterOutcome::Absent
        ));
        assert!(matches!(read_frontmatter(""), FrontmatterOutcome::Absent));
        // Opened but never closed.
        assert!(matches!(
            read_frontmatter("---\nstatus: executing\n"),
            FrontmatterOutcome::Unreadable { .. }
        ));
        // Closed, but not a mapping.
        assert!(matches!(
            read_frontmatter("---\n- one\n- two\n---\n"),
            FrontmatterOutcome::Unreadable { .. }
        ));
        // Genuinely malformed YAML.
        assert!(matches!(
            read_frontmatter("---\nstatus: [unclosed\n---\n"),
            FrontmatterOutcome::Unreadable { .. }
        ));
        // And the real thing still parses.
        assert!(matches!(
            read_frontmatter(REAL_GSD_STATE_MD),
            FrontmatterOutcome::Parsed(_)
        ));
    }

    #[test]
    fn an_empty_frontmatter_block_is_empty_not_broken() {
        assert!(matches!(
            read_frontmatter("---\n---\n# Body"),
            FrontmatterOutcome::Parsed(_)
        ));
    }

    #[test]
    fn state_versions_order_component_wise() {
        assert!(StateVersion::parse("1.10") > StateVersion::parse("1.9"));
        assert_eq!(StateVersion::parse("1.0"), StateVersion::parse("1.0"));
        assert!(StateVersion::parse("1.0.0") == StateVersion::parse("1.0"));
        assert!(StateVersion::parse("2.0").is_newer_than_supported());
        assert!(!StateVersion::parse("1.0").is_newer_than_supported());
        assert!(!StateVersion::parse("0.9").is_newer_than_supported());
        // A value carrying no numbers sorts below everything and keeps its text.
        let junk = StateVersion::parse("beta");
        assert_eq!(junk.raw(), "beta");
        assert_eq!(junk.major(), 0);
        assert!(!junk.is_newer_than_supported());
    }

    #[test]
    fn the_real_planning_state_md_of_this_repository_parses() {
        // Reads the repo's OWN STATE.md — the fixture that cannot drift from
        // what GSD writes, because GSD writes it.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".planning/STATE.md");
        let Ok(content) = std::fs::read_to_string(&path) else {
            return; // not a checkout with planning artifacts; nothing to assert
        };
        let fm = parse_state_md(&content).expect("this repo's own STATE.md must parse");
        assert!(
            fm.current_phase.is_some() || fm.current_phase_name.is_some(),
            "a real STATE.md carries a current phase; got status={:?}",
            fm.status
        );
        assert!(!fm.status.is_empty(), "a real STATE.md carries a status");
    }

    #[test]
    fn test_parse_state_md_empty() {
        assert!(parse_state_md("").is_none());
    }

    #[test]
    fn test_parse_state_md_no_frontmatter() {
        assert!(parse_state_md("# Just a heading").is_none());
    }

    #[test]
    fn test_parse_state_md_missing_optional_fields() {
        let content = "---\nstatus: active\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "active");
        assert_eq!(fm.progress.total_phases, 0); // default
        assert_eq!(fm.milestone, ""); // default
    }

    #[test]
    fn test_frontmatter_current_phase_keys_string_form() {
        let content = "---\nstatus: executing\ncurrent_phase: \"14\"\ncurrent_phase_name: Dashboard\ncurrent_plan: \"0.3\"\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.current_phase.as_deref(), Some("14"));
        assert_eq!(fm.current_phase_name.as_deref(), Some("Dashboard"));
        assert_eq!(fm.current_plan.as_deref(), Some("0.3"));
    }

    #[test]
    fn test_frontmatter_current_phase_numeric_form() {
        // A bare (unquoted) numeric current_phase / current_plan must NOT abort
        // frontmatter parsing — it is coerced to its string form.
        let content = "---\nstatus: executing\ncurrent_phase: 14\ncurrent_plan: 2\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "executing");
        assert_eq!(fm.current_phase.as_deref(), Some("14"));
        assert_eq!(fm.current_plan.as_deref(), Some("2"));
    }

    #[test]
    fn test_frontmatter_current_phase_absent_is_none() {
        let content = "---\nstatus: active\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.current_phase, None);
        assert_eq!(fm.current_phase_name, None);
        assert_eq!(fm.current_plan, None);
    }

    #[test]
    fn test_is_milestone_terminal() {
        assert!(is_milestone_terminal("v1.5.0 milestone complete"));
        assert!(is_milestone_terminal("Awaiting next milestone"));
        assert!(is_milestone_terminal("V1.5.0 MILESTONE COMPLETE")); // case-insensitive
        assert!(!is_milestone_terminal("All phases complete"));
        assert!(!is_milestone_terminal("executing"));
    }

    #[test]
    fn test_is_all_phases_complete() {
        assert!(is_all_phases_complete("All phases complete"));
        assert!(is_all_phases_complete("all phases complete")); // case-insensitive
        assert!(!is_all_phases_complete("v1.5.0 milestone complete"));
        assert!(!is_all_phases_complete("Awaiting next milestone"));
        assert!(!is_all_phases_complete("executing"));
    }

    // ========================================================================
    // The quote-in-place repair pass
    // ========================================================================

    /// The one fault the repair exists for: a top-level PLAIN scalar carrying a
    /// colon-space, which YAML reads as a nested mapping key and then refuses.
    #[test]
    fn a_plain_value_carrying_a_colon_space_is_quoted_in_place() {
        let (repaired, count) =
            repair_frontmatter_block("stopped_at: a. Earlier: b\n").expect("a repairable line");
        assert_eq!(count, 1);
        let value: Value = serde_yml::from_str(&repaired).expect("the repair must reparse");
        let map = value.as_mapping().expect("a mapping");
        assert_eq!(
            map.get("stopped_at")
                .and_then(Value::as_str),
            Some("a. Earlier: b"),
            "the value must round-trip with its colon intact"
        );
    }

    /// Everything the candidate rule must decline to touch. Each of these is a
    /// line shape the real sentriq STATE.md contains, and quoting any of them
    /// would change a valid document's meaning.
    #[test]
    fn every_uncertain_line_shape_is_left_exactly_as_it_was() {
        for untouched in [
            "k: \"x: y\"\n",         // already double-quoted
            "k: 'x: y'\n",           // already single-quoted
            "k: |\n  a: b\n",        // block scalar (literal)
            "k: >\n  a: b\n",        // block scalar (folded)
            "k: [a, b]\n",           // flow sequence
            "k: {a: 1}\n",           // flow mapping
            "- x: y\n",              // list item
            "# a: comment\n",        // comment
            "k:\n  nested: 1\n",     // empty value opening a nested mapping
            "  k: a: b\n",           // indented (not a top-level entry)
            "k: &anchor\n",          // anchor
            "k: *alias\n",           // alias
            "k: !tag a: b\n",        // tag
            "k: plain value\n",      // a legal plain scalar
            "k: 2026-09-11T02:30\n", // a timestamp: colon, but no colon-SPACE
        ] {
            assert!(
                repair_frontmatter_block(untouched).is_none(),
                "the repair must decline {untouched:?} — quoting it would change a \
                 valid document's meaning"
            );
        }
    }

    /// A repaired value carrying the two characters a double-quoted YAML scalar
    /// gives meaning to must come back byte-identical. Without this an escape
    /// bug degrades to `Unreadable` and reads as "not recoverable".
    #[test]
    fn a_value_containing_a_quote_and_a_backslash_round_trips_through_the_repair() {
        let original = r#"he said "no": C:\path\to"#;
        let block = format!("stopped_at: {original}\n");
        let (repaired, _) = repair_frontmatter_block(&block).expect("a repairable line");
        let value: Value = serde_yml::from_str(&repaired).expect("the repair must reparse");
        assert_eq!(
            value
                .as_mapping()
                .unwrap()
                .get("stopped_at")
                .and_then(Value::as_str),
            Some(original)
        );
    }

    /// The sentriq shape, carrying the structural features measured on the real
    /// file: ONE plain `stopped_at` carrying `. Earlier: ` and em-dashes, an
    /// ALREADY-QUOTED sibling whose quoted value also carries a colon-space, a
    /// bare `progress:` opening an indented mapping, and a plain numeric
    /// `current_phase`.
    ///
    /// **The key order is the real file's**, so `stopped_at` — the one faulty
    /// line — sits on FILE LINE 7 exactly as it does in the file this was
    /// measured from. That is what lets the position tests pin the
    /// body-relative-to-file-relative arithmetic against real geometry instead
    /// of restating it.
    const SENTRIQ_SHAPED_STATE_MD: &str = "---\ngsd_state_version: 1.0\nmilestone: v0.12\ncurrent_phase: 9\ncurrent_phase_name: Routine Event Logging\nstatus: planning\nstopped_at: Completed 260910-p8v (DTC history is vehicle-scoped — the trace recorder covers the connect path) and a follow-up. Earlier: 260910-x8d — the discovery keystone.\nlast_updated: \"2026-09-11T02:30:00.000Z\"\nlast_activity: 2026-09-10\nlast_activity_desc: \"260910-x8d, the discovery keystone: Phase 5 builds its worklist PER MODULE\"\nstate_head: a3a060725c82eb2a8fa1b5324ba4c13085e4c86f\nprogress:\n  total_phases: 4\n  completed_phases: 0\n  total_plans: 2\n  completed_plans: 0\nmilestone_name: Actuation Routines\n---\n\n# Project State\n";

    /// The FILE line `SENTRIQ_SHAPED_STATE_MD` puts its one fault on — counted
    /// in the fixture itself rather than restated, so the offset arithmetic is
    /// pinned by the fixture's own geometry.
    fn sentriq_fault_file_line() -> u32 {
        let line = SENTRIQ_SHAPED_STATE_MD
            .lines()
            .position(|l| l.starts_with("stopped_at:"))
            .expect("the fixture carries the faulty line")
            + 1;
        assert_eq!(line, 7, "the real file puts this line at file line 7");
        u32::try_from(line).unwrap()
    }

    /// The exact `stopped_at` the fixture above carries. Asserted by equality
    /// rather than by `contains`, so a repair that mangled or truncated the
    /// value fails here instead of passing on a surviving substring.
    const SENTRIQ_STOPPED_AT: &str = "Completed 260910-p8v (DTC history is vehicle-scoped — the trace recorder covers the connect path) and a follow-up. Earlier: 260910-x8d — the discovery keystone.";

    #[test]
    fn the_sentriq_shape_recovers_with_every_value_intact() {
        let FrontmatterOutcome::Recovered {
            frontmatter,
            repaired_lines,
        } = read_frontmatter(SENTRIQ_SHAPED_STATE_MD)
        else {
            panic!("the sentriq shape must recover, not fail and not read as clean");
        };
        assert_eq!(repaired_lines, 1, "exactly one line was at fault");
        assert_eq!(frontmatter.stopped_at, SENTRIQ_STOPPED_AT);
        assert_eq!(
            frontmatter.stopped_at.matches('—').count(),
            2,
            "the em-dashes must survive a char-wise repair"
        );
        assert_eq!(
            frontmatter.last_activity.as_deref(),
            Some("2026-09-10"),
            "every sibling of the repaired line arrives as it was written"
        );
        assert_eq!(frontmatter.last_updated, "2026-09-11T02:30:00.000Z");
        assert_eq!(frontmatter.milestone, "v0.12");
        assert_eq!(frontmatter.milestone_name, "Actuation Routines");
        assert_eq!(
            frontmatter.current_phase_name.as_deref(),
            Some("Routine Event Logging")
        );
        assert_eq!(frontmatter.status, "planning");
        assert_eq!(frontmatter.current_phase.as_deref(), Some("9"));
        assert_eq!(frontmatter.progress.total_phases, 4);
        assert_eq!(frontmatter.progress.total_plans, 2);
    }

    /// The already-quoted sibling must arrive byte-identical — it is the line
    /// the candidate rule exists to decline.
    #[test]
    fn the_already_quoted_sibling_of_a_repaired_line_is_untouched() {
        let yaml = extract_frontmatter(SENTRIQ_SHAPED_STATE_MD).unwrap();
        let (repaired, count) = repair_frontmatter_block(yaml).expect("one repairable line");
        assert_eq!(count, 1);
        let value: Value = serde_yml::from_str(&repaired).unwrap();
        assert_eq!(
            value
                .as_mapping()
                .unwrap()
                .get("last_activity_desc")
                .and_then(Value::as_str),
            Some("260910-x8d, the discovery keystone: Phase 5 builds its worklist PER MODULE")
        );
    }

    /// A file with no fault must never take the repair path — `Recovered` is a
    /// claim about the document, and claiming it falsely is the spoofing risk
    /// the distinct variant exists to close.
    #[test]
    fn a_clean_block_is_parsed_and_never_recovered() {
        assert!(matches!(
            read_frontmatter(REAL_GSD_STATE_MD),
            FrontmatterOutcome::Parsed(_)
        ));
    }

    // ========================================================================
    // The YAML fault position — FILE-relative numbers, and nothing else
    // ========================================================================

    fn fault_position(content: &str) -> Option<FrontmatterFaultPosition> {
        match read_frontmatter(content) {
            FrontmatterOutcome::Unreadable { position, .. } => position,
            other => panic!("expected Unreadable, got {other:?}"),
        }
    }

    /// The body handed to the YAML parser starts AFTER the opening `---`, so
    /// every line the parser names is one line short of the file's own count.
    /// Here the fault is on the file's line 2, and the parser calls it line 1.
    #[test]
    fn a_reported_line_is_translated_from_the_body_to_the_file() {
        assert_eq!(
            fault_position("---\nstatus: [unclosed\n---\n").map(|p| p.line),
            Some(2)
        );
    }

    /// The same translation against the real file's geometry: the one fault
    /// sits on file line 7, which the parser reports as body line 6.
    #[test]
    fn the_sentriq_shape_made_unrecoverable_names_file_line_seven() {
        // The repair rewrites the `stopped_at` line, but the document carries a
        // second fault the repair is not allowed to touch (an unclosed flow
        // collection on an indented line), so it stays unreadable — and the
        // position still names the FIRST fault, where the parser stopped.
        let unrecoverable =
            SENTRIQ_SHAPED_STATE_MD.replace("  total_phases: 4", "  total_phases: [4");
        let position = fault_position(&unrecoverable).expect("a located fault");
        assert_eq!(position.line, sentriq_fault_file_line());
        assert!(position.column > 0, "columns are 1-based");
    }

    /// Not every fault has a location, and an invented one would be worse than
    /// none: the label falls back to its bare form.
    #[test]
    fn a_fault_the_parser_cannot_locate_carries_no_position() {
        // Not a mapping — this crate's own classification, never the parser's.
        assert_eq!(fault_position("---\n- one\n- two\n---\n"), None);
        // Unterminated — the block never reaches the parser at all.
        assert_eq!(fault_position("---\nstatus: executing\n"), None);
    }

    /// A line number that cannot name a line in the block it came from is not
    /// information. It is DROPPED rather than clamped to an invented ceiling.
    #[test]
    fn a_position_beyond_the_blocks_own_line_count_is_dropped_not_clamped() {
        // body line 1 is file line 2; the block has 3 lines, so file lines 2-4
        // are the only nameable ones.
        assert_eq!(
            file_relative_position(2, 3, 3, 5),
            Some(FrontmatterFaultPosition { line: 4, column: 5 })
        );
        assert_eq!(file_relative_position(2, 3, 4, 5), None, "one past the end");
        assert_eq!(file_relative_position(2, 3, 0, 5), None, "a 0 line");
        assert_eq!(
            file_relative_position(2, 3, u64::from(u32::MAX) + 9, 5),
            None,
            "no wrap on overflow"
        );
    }
}
