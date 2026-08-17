mod frame;
mod group;
mod inspection;
mod protocol;
mod reveal;
mod rolls;

use frame::{render_card, stage_line, title_line};
use inspection::render_inspector;
use protocol::render_document;
use reveal::{compose_reveal_workspace, render_mnemonic};
use rolls::render_roll_entry;
#[cfg(test)]
use rolls::roll_rows;
use zeroize::Zeroizing;

use crate::domain::{
    bip39::EntropyTarget,
    ceremony::Phase,
    protocol::{CaptureAssessment, ConversionProtocol},
};
use crate::{
    application::SafetyAttestation,
    presentation::{
        ChoiceContent, ProtocolMenuChoice, attempt_rejection, concealed_generation,
        finish_confirmation, protocol_choices, safety_content, target_choices,
    },
};

use super::app::{App, InspectorView, SAFETY_ITEM_COUNT};

pub(super) const MIN_WIDTH: u16 = 52;
pub(super) const MIN_HEIGHT: u16 = 40;
const MAX_CONTENT_WIDTH: usize = 88;

pub(super) type Lines = Zeroizing<Vec<String>>;

#[must_use]
pub fn render(app: &App, width: u16, height: u16) -> Zeroizing<String> {
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        return Zeroizing::new(format!(
            "bip39-ceremony\n\nTerminal too small ({width}×{height}).\n\
             Resize to at least {MIN_WIDTH}×{MIN_HEIGHT}.\n\n\
             Secret values remain hidden."
        ));
    }

    let width = usize::from(width).min(MAX_CONTENT_WIDTH);
    let height = usize::from(height);
    if !app.quit_pending()
        && let Some(session) = app.group()
    {
        return group::compose_group_workspace(app, session, width, height);
    }
    if app.inspector().is_none() && app.ceremony().state().phase() == Phase::Revealed {
        compose_reveal_workspace(app, width, height)
    } else {
        compose_workspace(app, width, height)
    }
}

pub(super) fn scroll_limit(app: &App, width: u16, height: u16) -> usize {
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        return 0;
    }
    let width = usize::from(width).min(MAX_CONTENT_WIDTH);
    let height = usize::from(height);
    let notice_lines = usize::from(app.message().is_some());
    let rows = height.saturating_sub(8 + notice_lines);

    if let Some(session) = app.group() {
        let body = group::group_body(app, session, width.saturating_sub(4));
        return body.len().saturating_sub(rows.saturating_sub(1));
    }
    if app.inspector().is_some() {
        let body = render_inspector(app, width.saturating_sub(4));
        let rows = if protocol_detail_open(app) {
            let context = protocol_detail_context(app, width.saturating_sub(4));
            split_protocol_detail_rows(
                context.len(),
                height.saturating_sub(10 + notice_lines),
                protocol_detail_minimum_rows(app),
            )
            .1
        } else {
            rows
        };
        return body.len().saturating_sub(rows.saturating_sub(1));
    }
    if app.ceremony().state().phase() == Phase::EnterRolls {
        let protocol = app.ceremony().state().protocol();
        let word_exact_assignments =
            protocol == Some(ConversionProtocol::WordExactV1) && !app.word_exact_raw_ledger();
        if word_exact_assignments
            || matches!(
                protocol,
                Some(
                    ConversionProtocol::JadeDirectV1
                        | ConversionProtocol::BitBox02DirectV1
                        | ConversionProtocol::KruxD20V1
                        | ConversionProtocol::CoinFourD6DirectV1
                )
            )
        {
            let body = render_live(app, width.saturating_sub(4));
            return body.len().saturating_sub(rows.saturating_sub(1));
        }
        if !app.rolls_hidden() {
            return app
                .ceremony()
                .state()
                .rolls()
                .len()
                .div_ceil(25)
                .saturating_sub(4);
        }
    }
    0
}

fn compose_workspace(app: &App, width: usize, height: usize) -> Zeroizing<String> {
    let mut output = Zeroizing::new(String::with_capacity(
        width.saturating_mul(height).saturating_add(height),
    ));
    let separator = "─".repeat(width);
    push_output(
        &mut output,
        &clip(&title_line(width, workspace_status(app)), width),
    );
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&stage_line(app, width), width));
    push_output(&mut output, "");

    let notice_lines = usize::from(app.message().is_some());
    if protocol_detail_open(app) && !app.quit_pending() {
        let context = protocol_detail_context(app, width.saturating_sub(4));
        let detail = render_inspector(app, width.saturating_sub(4));
        let available = height.saturating_sub(10 + notice_lines);
        let (context_rows, detail_rows) =
            split_protocol_detail_rows(context.len(), available, protocol_detail_minimum_rows(app));
        let (context_title, context_scroll) = if app.ceremony().state().phase() == Phase::EnterRolls
        {
            ("ROLL CAPTURE", app.roll_scroll())
        } else {
            ("PROTOCOL SELECTION", 0)
        };
        render_card(
            &mut output,
            context_title,
            &context,
            context_rows,
            context_scroll,
            false,
            width,
        );
        render_card(
            &mut output,
            workspace_card_title(app),
            &detail,
            detail_rows,
            app.inspector().map_or(0, |inspector| inspector.scroll),
            true,
            width,
        );
    } else {
        let rows = height.saturating_sub(8 + notice_lines);
        let body = if app.quit_pending() {
            render_quit(app.ceremony().state().phase(), width.saturating_sub(4))
        } else if app.inspector().is_some() {
            render_inspector(app, width.saturating_sub(4))
        } else {
            render_live(app, width.saturating_sub(4))
        };
        let scroll = app
            .inspector()
            .map_or(app.roll_scroll(), |inspector| inspector.scroll);
        render_card(
            &mut output,
            workspace_card_title(app),
            &body,
            rows,
            scroll,
            true,
            width,
        );
    }

    if let Some(message) = app.message() {
        push_output(&mut output, &format!("! {message}"));
    }
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&footer_line(app), width));
    output
}

fn protocol_detail_open(app: &App) -> bool {
    app.inspector()
        .is_some_and(|inspector| inspector.view == InspectorView::ProtocolExplanation)
}

fn protocol_detail_context(app: &App, width: usize) -> Lines {
    if app.ceremony().state().phase() == Phase::EnterRolls {
        render_roll_entry(app, width)
    } else {
        render_protocol_step(app)
    }
}

fn protocol_detail_minimum_rows(app: &App) -> usize {
    if app.ceremony().state().phase() == Phase::EnterRolls {
        10
    } else {
        8
    }
}

fn split_protocol_detail_rows(
    context_len: usize,
    available: usize,
    minimum_detail_rows: usize,
) -> (usize, usize) {
    let context = context_len.min(available.saturating_sub(minimum_detail_rows));
    (context, available.saturating_sub(context))
}

fn workspace_status(app: &App) -> &'static str {
    // The corner reflects only secret state the app actually controls, never
    // an environment claim (offline, airgap) it cannot verify. Operator-owned
    // guidance lives in the safety preflight.
    let state = app.ceremony().state();
    if state.phase() == Phase::EnterRolls && state.capture_count() > 0 {
        // The ledger always shows at least the latest outcome during entry.
        "● SECRET EXPOSED"
    } else if state.phase() == Phase::Result || state.capture_count() > 0 {
        "● SECRET CONCEALED"
    } else {
        "○ NO SECRET YET"
    }
}

fn workspace_card_title(app: &App) -> &'static str {
    if app.quit_pending() {
        return "CONFIRM SECURE EXIT · FOCUS";
    }
    match app.inspector().map(|inspector| inspector.view) {
        Some(InspectorView::Derivation) => "DERIVATION · ALL VALUES SECRET · FOCUS",
        Some(InspectorView::ProtocolExplanation) => "PROTOCOL DETAILS · FOCUS",
        Some(InspectorView::Help) => "HELP · FOCUS",
        None if app.ceremony().state().phase() == Phase::Safety => "SAFETY PREFLIGHT · FOCUS",
        None if app.ceremony().state().phase() == Phase::EnterRolls => "ROLL CAPTURE · FOCUS",
        None if app.ceremony().state().phase() == Phase::Result => "MNEMONIC READY · FOCUS",
        None => "CEREMONY · FOCUS",
    }
}

fn render_live(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    match state.phase() {
        Phase::ChooseTarget => content_choices(
            "Choose mnemonic length",
            "Both are standard BIP-39; length does not fix device or backup exposure.",
            0,
            app.target_cursor(),
            &target_choices(),
        ),
        Phase::ChooseProtocol => render_protocol_step(app),
        Phase::Safety => render_safety(app, width),
        Phase::EnterRolls => render_roll_entry(app, width),
        Phase::ReadyToGenerate => lines(&[
            "Generating mnemonic…",
            "",
            "The conversion is deterministic and runs entirely in memory.",
        ]),
        Phase::AttemptRejected => render_attempt_rejected(app),
        Phase::Result => render_concealed_result(app, width),
        Phase::Revealed => render_mnemonic(app, width),
        Phase::Cancelled => lines(&[
            "Ceremony cancelled",
            "",
            "Secret in-memory state is being destroyed.",
        ]),
    }
}

fn render_concealed_result(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    let Some((target, protocol)) = state.target().zip(state.protocol()) else {
        return lines(&["Generation metadata unavailable."]);
    };
    let document = concealed_generation(protocol, target);
    let rendered = render_document(&document, width);
    let mut output = Lines::new(
        rendered
            .iter()
            .filter(|line| !line.is_empty())
            .cloned()
            .collect(),
    );
    push(&mut output, "");
    push(&mut output, "> [r] REVEAL RECOVERY WORDS");
    push(
        &mut output,
        "  The complete mnemonic will appear immediately.",
    );
    output
}

fn render_protocol_step(app: &App) -> Lines {
    let target = app
        .ceremony()
        .state()
        .target()
        .unwrap_or(EntropyTarget::Words12);
    let mut output = if protocol_detail_open(app) {
        compact_protocol_choices(app.protocol_cursor(), target)
    } else {
        content_choices(
            "Choose conversion protocol",
            "Available choices generate BIP-39; target-limited rows explain their scope.",
            1,
            app.protocol_cursor(),
            &protocol_choices(target),
        )
    };
    push(&mut output, "");
    push(
        &mut output,
        if protocol_detail_open(app) {
            "Details open below · [e/Esc/Tab] close"
        } else {
            "[e] Explain selected protocol · [g] Group compare"
        },
    );
    output
}

fn compact_protocol_choices(selected: usize, target: EntropyTarget) -> Lines {
    let mut output = lines(&[
        "SETUP · STEP 2 OF 2 · Protocol",
        "Choose conversion protocol",
    ]);
    for (index, choice) in ProtocolMenuChoice::ALL.into_iter().enumerate() {
        let marker = if index == selected { '▶' } else { '○' };
        let status = if choice.implemented_protocol(target).is_some() {
            ""
        } else {
            match choice {
                ProtocolMenuChoice::KeystoneLegacyDice => " · UNSUPPORTED FOR 12 WORDS",
                ProtocolMenuChoice::CoinFourD6Direct => " · UNSUPPORTED FOR 24 WORDS",
                _ => unreachable!("all other menu choices support both targets"),
            }
        };
        push(
            &mut output,
            &format!("  {marker} {}{status}", choice.name()),
        );
    }
    output
}

fn content_choices(
    title: &str,
    description: &str,
    step: usize,
    selected: usize,
    content: &[ChoiceContent],
) -> Lines {
    let items = content
        .iter()
        .map(|choice| (choice.name(), choice.detail()))
        .collect::<Vec<_>>();
    choices(title, description, step, selected, &items)
}

fn choices(
    title: &str,
    description: &str,
    step: usize,
    selected: usize,
    choices: &[(&str, &str)],
) -> Lines {
    let mut output = Lines::new(Vec::new());
    let label = if step == 0 { "Length" } else { "Protocol" };
    push(
        &mut output,
        &format!("SETUP · STEP {} OF 2 · {label}", step + 1),
    );
    push(&mut output, title);
    push(&mut output, description);
    push(&mut output, "");
    for (index, (name, detail)) in choices.iter().enumerate() {
        let marker = if index == selected { '▶' } else { '○' };
        push(&mut output, &format!("  {marker} {name}"));
        push(&mut output, &format!("    {detail}"));
        if index + 1 < choices.len() {
            push(&mut output, "");
        }
    }
    output
}

fn render_attempt_rejected(app: &App) -> Lines {
    let required = app.ceremony().state().required_rolls().unwrap_or(0);
    let protocol = app
        .ceremony()
        .state()
        .protocol()
        .unwrap_or(ConversionProtocol::ExactV1);
    let document = attempt_rejection(protocol, required);
    let mut output = render_document(&document, MAX_CONTENT_WIDTH);
    push(&mut output, "");
    push(
        &mut output,
        "Press [r] or Enter to start a fresh roll attempt.",
    );
    output
}

fn render_safety(app: &App, width: usize) -> Lines {
    let checked = app.safety_check_count();
    let mut output = lines(&[
        "SAFETY CHECKLIST",
        "Checks are acknowledgements, not verification.",
        &format!(
            "[{}{}]  {checked} / {SAFETY_ITEM_COUNT} checked",
            "█".repeat(checked * 2),
            "░".repeat((SAFETY_ITEM_COUNT - checked) * 2)
        ),
    ]);
    for (index, item) in SafetyAttestation::ALL.iter().enumerate() {
        let cursor = if index == app.safety_cursor() {
            '▶'
        } else {
            ' '
        };
        let mark = if app.safety_checked(index) {
            '✓'
        } else {
            '□'
        };
        push(
            &mut output,
            &format!("  {cursor} {mark} {}", safety_content(*item).label()),
        );
    }
    let selected = safety_content(SafetyAttestation::ALL[app.safety_cursor()]);
    push_wrapped(&mut output, "", selected.detail(), width);
    push_wrapped(
        &mut output,
        "! ",
        "The tool cannot detect unfair dice or a compromised system.",
        width,
    );
    if app.safety_all_checked() {
        push(
            &mut output,
            "Ready. Press Enter to acknowledge and begin rolling.",
        );
    } else {
        push(&mut output, "[Space] toggle selected · [c] check all");
    }
    output
}

fn render_quit(phase: Phase, width: usize) -> Lines {
    let revealed = phase == Phase::Revealed;
    let document = finish_confirmation(revealed);
    let mut output = lines(&[document.title(), ""]);
    let content = render_document(&document, width);
    output.extend(content.iter().cloned());
    push(&mut output, "");
    push(&mut output, "[q] or Enter   Clear and exit");
    push(
        &mut output,
        if revealed {
            "[n] or Esc     Return to mnemonic"
        } else {
            "[n] or Esc     Continue ceremony"
        },
    );
    output
}

fn footer_line(app: &App) -> String {
    if app.quit_pending() {
        return if app.ceremony().state().phase() == Phase::Revealed {
            if app.mnemonic_hidden() {
                "[q/Enter] end and exit    [n/Esc] return hidden".to_owned()
            } else {
                "[q/Enter] end and exit    [n/Esc] return to mnemonic".to_owned()
            }
        } else {
            "[q/Enter] cancel and exit    [n/Esc] continue".to_owned()
        };
    }
    if let Some(inspector) = app.inspector() {
        return inspector_footer(
            inspector.view,
            app.derivation_available(),
            app.ceremony().state().phase(),
        );
    }
    match app.ceremony().state().phase() {
        Phase::ChooseTarget => {
            "[j/k or ↑/↓] choose   [l/→/Enter] next   [?] help   [q] cancel".to_owned()
        }
        Phase::ChooseProtocol => {
            if app.selected_protocol().is_some() {
                "[↑/↓] choose  [e] explain  [Enter] next  [g] group  [q] cancel".to_owned()
            } else {
                "[↑/↓] choose  [e] explain  [Enter] unavailable  [g] group".to_owned()
            }
        }
        Phase::Safety => "[q] cancel  [↑/↓] move  [Space] toggle  [c] all".to_owned(),
        Phase::EnterRolls => roll_footer(app),
        Phase::AttemptRejected => "[r/Enter] fresh attempt    [q] cancel".to_owned(),
        Phase::Result => "[r] reveal    [q] cancel".to_owned(),
        Phase::Revealed => {
            if app.mnemonic_verification().is_some() {
                "[letters] type   [Backspace] correct   [Enter] compare   [Esc] cancel".to_owned()
            } else if app.mnemonic_hidden() {
                "[h/Esc] show mnemonic    [q] finish securely".to_owned()
            } else if app.ceremony().state().mnemonic_backup_verified() {
                "[h] hide   [Enter/q] finish   [d] derive".to_owned()
            } else {
                "[h] hide   [v] verify backup   [q] finish   [d] derive".to_owned()
            }
        }
        Phase::ReadyToGenerate | Phase::Cancelled => "[q] exit".to_owned(),
    }
}

fn inspector_footer(view: InspectorView, derivation_available: bool, phase: Phase) -> String {
    let quit = if phase == Phase::Revealed {
        "[q] finish"
    } else {
        "[q] cancel"
    };
    if view == InspectorView::Derivation {
        return format!("[h] hide   {quit}   [↑/↓] scroll   [Tab/Esc] live");
    }
    if view == InspectorView::ProtocolExplanation {
        let destination = if phase == Phase::ChooseProtocol {
            "protocols"
        } else {
            "roll capture"
        };
        return format!("{quit}  [↑/↓] scroll  [e/Esc/Tab] {destination}");
    }
    let derivation = if derivation_available {
        "   [d] derivation"
    } else {
        ""
    };
    format!("{quit}   [↑/↓] scroll{derivation}   [Tab/Esc] live")
}

fn roll_footer(app: &App) -> String {
    let state = app.ceremony().state();
    if matches!(
        state.protocol(),
        Some(
            ConversionProtocol::JadeDirectV1
                | ConversionProtocol::BitBox02DirectV1
                | ConversionProtocol::KruxD20V1
                | ConversionProtocol::CoinFourD6DirectV1
        )
    ) {
        return if state.can_confirm_rolls() {
            "[q] cancel  [h] ledger  [↑/↓] scroll  [Enter] generate  [⌫] undo".to_owned()
        } else {
            "[q] cancel  [h] ledger  [↑/↓] scroll  [keys] outcome  [⌫] undo".to_owned()
        };
    }
    if state.protocol() == Some(ConversionProtocol::WordExactV1) {
        let visibility = if app.rolls_hidden() {
            "[h] show faces"
        } else {
            "[h] hide faces"
        };
        let view = if app.word_exact_raw_ledger() {
            "[l] assignments"
        } else {
            "[l] raw ledger"
        };
        return if state.can_confirm_rolls() {
            format!("[q] cancel  {visibility}  {view}  [Enter] generate")
        } else {
            format!("[q] cancel  {visibility}  {view}  [1–6] roll  [⌫] undo")
        };
    }
    let visibility = if app.rolls_hidden() {
        "[h] show all"
    } else {
        "[h] hide prior"
    };
    if state.can_confirm_rolls()
        && state
            .capture_assessment()
            .is_some_and(CaptureAssessment::accepts_optional_rolls)
    {
        format!("[q] cancel  {visibility}  [1–6] extra  [Enter] generate")
    } else if state.can_confirm_rolls() {
        format!("[q] cancel  {visibility}  [Enter] generate  [Backspace] undo")
    } else {
        format!("[q] cancel  {visibility}  [1–6] record  [Backspace] undo")
    }
}

pub(super) fn push_wrapped(output: &mut Lines, label: &str, value: &str, width: usize) {
    let continuation = " ".repeat(label.chars().count());
    let available = width.saturating_sub(label.chars().count()).max(10);
    let chunks = if value.contains(' ') {
        wrap_words(value, available)
    } else {
        Lines::new(
            value
                .as_bytes()
                .chunks(available)
                .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
                .collect(),
        )
    };
    for (index, chunk) in chunks.iter().enumerate() {
        let prefix = if index == 0 { label } else { &continuation };
        push_owned(output, format!("{prefix}{chunk}"));
    }
}

pub(super) fn wrap_words(value: &str, width: usize) -> Lines {
    let mut rows = Lines::new(Vec::new());
    let mut row = String::new();
    for word in value.split_whitespace() {
        let extra = word.len() + usize::from(!row.is_empty());
        if !row.is_empty() && row.len() + extra > width {
            rows.push(row);
            row = String::new();
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

pub(super) fn lines(values: &[&str]) -> Lines {
    Lines::new(values.iter().map(|value| (*value).to_owned()).collect())
}

pub(super) fn push(output: &mut Lines, value: &str) {
    output.push(value.to_owned());
}

pub(super) fn push_owned(output: &mut Lines, value: String) {
    output.push(value);
}

fn push_output(output: &mut String, value: &str) {
    output.push_str(value);
    output.push('\n');
}

fn clip(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    let visible = width.saturating_sub(1);
    format!("{}…", value.chars().take(visible).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use termion::event::Key;

    fn concealed_twenty_four_words() -> App {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..100 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('\n'));
        app
    }

    fn revealed_twenty_four_words() -> App {
        let mut app = concealed_twenty_four_words();
        app.update(Key::Char('r'));
        app
    }

    fn verify_mnemonic(app: &mut App) {
        let words = Zeroizing::new(app.generation().unwrap().mnemonic().words().to_vec());
        app.update(Key::Char('v'));
        for word in words.iter() {
            for character in word.chars() {
                app.update(Key::Char(character));
            }
            app.update(Key::Char('\n'));
        }
    }

    fn safety_app() -> App {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('\n'));
        app
    }

    fn roll_entry_app(mode: usize, count: usize) -> App {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        let steps = match mode {
            0 | 2 => 2, // Exact; 2 preserves generic fixed-capture fixtures.
            1 => 1,     // Word-by-word Exact
            3 => 0,     // COLDCARD
            _ => unreachable!(),
        };
        for _ in 0..steps {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..count {
            app.update(Key::Char('1'));
        }
        app
    }

    #[test]
    fn status_corner_reports_secret_lifecycle_not_environment() {
        // Before any capture the corner must not assert an environment claim.
        let setup = render(&App::default(), 52, 40);
        let setup_title = setup.lines().next().unwrap();
        assert!(setup_title.ends_with("○ NO SECRET YET"));
        assert!(!setup.contains("OFFLINE"));

        // During roll entry the latest outcome is on screen: secret exposed.
        let entering = render(&roll_entry_app(0, 1), 52, 40);
        assert!(
            entering
                .lines()
                .next()
                .unwrap()
                .ends_with("● SECRET EXPOSED")
        );

        // A generated-but-hidden mnemonic is concealed.
        let concealed = render(&concealed_twenty_four_words(), 52, 40);
        assert!(
            concealed
                .lines()
                .next()
                .unwrap()
                .ends_with("● SECRET CONCEALED")
        );
    }

    #[test]
    fn concealed_result_has_a_deliberate_reveal_gate() {
        let app = concealed_twenty_four_words();
        let output = render(&app, 52, 40);

        assert!(
            output
                .lines()
                .next()
                .is_some_and(|line| line.ends_with("● SECRET CONCEALED"))
        );
        assert!(output.contains("┏━ MNEMONIC READY · FOCUS"));
        assert!(output.contains("✓ GENERATION COMPLETE · 24-WORD BIP-39"));
        assert!(output.contains("exact-v1 · no hidden randomness"));
        assert!(output.contains("◆ SECRET CONCEALED"));
        assert!(output.contains("BEFORE REVEAL"));
        assert!(output.contains("> [r] REVEAL RECOVERY WORDS"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn concealed_result_cannot_leak_through_derivation() {
        let mut app = concealed_twenty_four_words();
        let first_word = app.generation().unwrap().mnemonic().words()[0].clone();

        app.update(Key::Char('d'));
        let output = render(&app, 80, 40);
        assert!(app.inspector().is_none());
        assert!(!output.contains(&first_word));
        assert!(!output.contains("DERIVATION · ALL VALUES SECRET"));
        assert!(!output.contains("Entropy"));

        app.update(Key::Char('i'));
        app.update(Key::Char('d'));
        assert!(app.inspector().is_none());
    }

    #[test]
    fn derivation_hide_key_returns_to_the_concealed_workspace() {
        let mut app = revealed_twenty_four_words();
        let first_word = app.generation().unwrap().mnemonic().words()[0].clone();
        app.update(Key::Char('d'));
        assert!(render(&app, 80, 70).contains("DERIVATION · ALL VALUES SECRET"));

        app.update(Key::Char('h'));
        let output = render(&app, 80, 40);

        assert!(output.contains("MNEMONIC QUICK-HIDDEN"));
        assert!(!output.contains("DERIVATION · ALL VALUES SECRET"));
        assert!(!output.contains(&first_word));

        app.update(Key::Char('h'));
        let output = render(&app, 80, 70);
        assert!(output.contains("DERIVATION · ALL VALUES SECRET"));

        app.update(Key::Char('q'));
        let output = render(&app, 80, 40);
        assert!(output.contains("SECRET TEMPORARILY CONCEALED"));
        assert!(!output.contains("DERIVATION · ALL VALUES SECRET"));
        assert!(!output.contains(&first_word));
    }

    #[test]
    fn small_terminal_never_renders_secret_state() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('6'));

        let output = render(&app, 20, 5);
        assert!(output.contains("Terminal too small"));
        assert!(!output.contains("RECORDED ROLLS"));
        assert!(!output.contains("SECRET"));
    }

    #[test]
    fn setup_options_keep_visual_separation() {
        let body = choices(
            "Choose mnemonic length",
            "Description",
            0,
            0,
            &[("12 words", "128-bit"), ("24 words", "256-bit")],
        );
        let first_detail = body.iter().position(|line| line == "    128-bit").unwrap();
        assert!(body[first_detail + 1].is_empty());
        assert_eq!(body[first_detail + 2], "  ○ 24 words");
    }

    #[test]
    fn unselected_options_carry_a_distinct_marker() {
        let body = choices(
            "Choose mnemonic length",
            "Description",
            0,
            0,
            &[("12 words", "128-bit"), ("24 words", "256-bit")],
        );
        assert!(body.iter().any(|line| line == "  ▶ 12 words"));
        assert!(body.iter().any(|line| line == "  ○ 24 words"));
    }

    #[test]
    fn default_screen_has_clear_hierarchy_and_controls() {
        let output = render(&App::default(), 80, 40);
        assert!(output.contains("● SETUP  ›  ○ SAFETY"));
        assert!(output.contains("Choose mnemonic length"));
        assert!(output.contains("▶ 12 words"));
        assert!(output.contains("[q] cancel"));
        assert!(output.contains("SETUP · STEP 1 OF 2 · Length"));
        assert!(
            output.find("┏━ CEREMONY").unwrap() < output.find("Choose mnemonic length").unwrap()
        );
        assert!(!output.contains(" STATE"));
    }

    #[test]
    fn safety_preflight_formats_selection_and_progress() {
        let mut app = safety_app();
        let initial = render(&app, 52, 40);

        assert!(initial.contains("┏━ SAFETY PREFLIGHT · FOCUS"));
        assert!(initial.contains("0 / 7 checked"));
        assert!(initial.contains("▶ □ Device isolated"));
        assert!(initial.contains("[Space] toggle selected"));
        assert_eq!(initial.lines().count(), 40);

        app.update(Key::Down);
        app.update(Key::Char(' '));
        let checked = render(&app, 52, 40);
        assert!(checked.contains("▶ ✓ Recording disabled"));
        assert!(checked.contains("Disable terminal or tmux logging"));
        assert!(checked.contains("1 / 7 checked"));
    }

    #[test]
    fn safety_preflight_shows_ready_only_after_all_checks() {
        let mut app = safety_app();
        app.update(Key::Char('c'));
        let ready = render(&app, 52, 40);

        assert!(ready.contains("7 / 7 checked"));
        assert!(ready.contains("Ready. Press Enter"));
        assert!(!ready.contains("more — use Page"));
    }

    #[test]
    fn footer_remains_visible_when_body_is_long() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..100 {
            app.update(Key::Char('6'));
        }
        let output = render(&app, 80, 40);
        assert!(output.contains("[Backspace] undo"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn target_choice_separates_entropy_checksum_and_operational_risk() {
        let output = render(&App::default(), 80, 40);

        assert!(output.contains("128 entropy bits + 4 checksum bits"));
        assert!(output.contains("256 entropy bits + 8 checksum bits"));
        assert!(output.contains("length does not fix device or backup exposure"));
        assert!(!output.contains("maximum security"));
    }

    #[test]
    fn help_explains_current_evidence_boundary() {
        let mut app = App::default();
        app.update(Key::Char('?'));
        let target_help = render(&app, 80, 40);
        assert!(target_help.contains("choosing the BIP-39 entropy target"));
        assert!(target_help.contains("More words do not repair"));

        app.update(Key::Esc);
        app.update(Key::Char('\n'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('?'));
        let roll_help = render(&app, 80, 40);
        assert!(roll_help.contains("verifies valid protocol input keys"));
        assert!(roll_help.contains("cannot know whether a key matched"));
    }

    #[test]
    fn help_states_application_trust_boundary() {
        let mut app = App::default();
        app.update(Key::Char('?'));
        let output = render(&app, 80, 40);

        assert!(output.contains("APPLICATION TRUST BOUNDARY"));
        assert!(output.contains("no application network adapter"));
        assert!(output.contains("actual offline status"));
        assert!(output.contains("operating-system integrity"));
    }

    #[test]
    fn minimum_terminal_shows_both_progress_steps_and_all_protocols() {
        let mut app = App::default();
        app.update(Key::Right);
        let output = render(&app, 52, 40);

        assert!(output.contains("SETUP · STEP 2 OF 2 · Protocol"));
        assert!(output.contains("COLDCARD"));
        assert!(output.contains("SeedSigner coin flips"));
        assert!(!output.contains("↓ more — use Page Down"));
    }

    #[test]
    fn protocol_step_shows_all_modes_without_scrolling_on_a_desktop() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        let output = render(&app, 80, 40);

        for expected in [
            "Choose conversion protocol",
            "COLDCARD",
            "Word-by-word Exact",
            "Exact",
            "Keystone legacy dice",
            "Jade direct words",
            "BitBox02 Diceware",
            "Krux D20",
            "Coin + four-D6 direct words",
            "SeedSigner coin flips",
        ] {
            assert!(output.contains(expected), "missing {expected}");
        }
        assert!(!output.contains(" v1"));
        assert!(!output.contains("↓ more — use Page Down"));
    }

    #[test]
    fn unsupported_target_renders_as_unavailable_with_explanation() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..3 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ Keystone legacy dice"));
        assert!(menu.contains("! UNSUPPORTED · available for 24 words only"));
        assert!(menu.contains("[Enter] unavailable"));

        app.update(Key::Char('e'));
        let details = render(&app, 80, 40);
        assert!(details.contains("KEYSTONE LEGACY DICE · UNSUPPORTED TARGET"));
        assert!(details.contains("available only for 24-word output"));
        app.update(Key::PageDown);
        assert!(render(&app, 80, 40).contains("cannot be selected for a ceremony"));
    }

    #[test]
    fn jade_protocol_uses_typed_mixed_dice_workspace() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..4 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('A'));
        let capture = render(&app, 80, 40);
        assert!(capture.contains("PHYSICAL MIXED-DICE CAPTURE · ROLLS ARE SECRET"));
        assert!(capture.contains("D16 FACE 10"));
        assert!(capture.contains("uppercase [A–G]"));
        assert!(capture.contains("DIRECT WORD 01 OF 11"));
        assert!(capture.contains("34 remaining"));

        let minimum = render(&app, 52, 40);
        assert_eq!(minimum.lines().count(), 40);
        assert!(minimum.contains("PHYSICAL MIXED-DICE CAPTURE"));
    }

    #[test]
    fn jade_choice_is_operational_in_protocol_menu() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..4 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ Jade direct words"));
        assert!(menu.contains("[Enter] next"));
    }

    #[test]
    fn bitbox_choice_and_rejection_aware_workspace_are_operational() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..5 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ BitBox02 Diceware"));
        assert!(menu.contains("[Enter] next"));

        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('6'));
        for _ in 0..5 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('h'));
        let capture = render(&app, 80, 40);
        assert!(capture.contains("PHYSICAL D6 + COIN CAPTURE"));
        assert!(capture.contains("D6=6X"));
        assert!(capture.contains("Flip coin, then press [0] tails or [1] heads"));
        assert!(capture.contains("COIN SELECTOR · heads→0 · tails→1"));
        assert!(capture.contains("1 rejected D6"));
    }

    #[test]
    fn krux_d20_choice_and_capture_workspace_are_operational() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..6 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ Krux D20"));
        assert!(menu.contains("min 30 rolls"));
        assert!(menu.contains("[Enter] next"));

        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('K'));
        let capture = render(&app, 80, 40);
        assert!(capture.contains("PHYSICAL D20 CAPTURE · ROLLS ARE SECRET"));
        assert!(capture.contains("D20 FACE 20"));
        assert!(capture.contains("uppercase [A–K]"));
        assert!(capture.contains("29 remaining"));
    }

    #[test]
    fn jade_twenty_four_word_ledger_scrolls_at_minimum_size() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..4 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..70 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('h'));

        let limit = scroll_limit(&app, 52, 40);
        assert!(limit > 0);
        let top = render(&app, 52, 40);
        assert!(top.contains("[↑/↓] scroll"));
        for _ in 0..limit.div_ceil(4) {
            app.update_bounded(Key::PageDown, limit);
        }
        let bottom = render(&app, 52, 40);
        assert!(bottom.contains("ALL DIRECT POSITIONS + ENTROPY TAIL"));
        assert!(bottom.contains("[Enter] confirm and generate"));
    }

    #[test]
    fn bitbox_twenty_four_word_ledger_scrolls_to_tail_completion() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..5 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..23 {
            for _ in 0..5 {
                app.update(Key::Char('1'));
            }
            app.update(Key::Char('1'));
        }
        for _ in 0..3 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('h'));

        let limit = scroll_limit(&app, 52, 40);
        assert!(limit > 0);
        for _ in 0..limit.div_ceil(4) {
            app.update_bounded(Key::PageDown, limit);
        }
        let bottom = render(&app, 52, 40);
        assert!(bottom.contains("ALL BITBOX TABLE POSITIONS + ENTROPY TAIL"));
        assert!(bottom.contains("[Enter] confirm and generate"));
    }

    #[test]
    fn krux_twenty_four_word_ledger_scrolls_to_open_completion() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..6 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..60 {
            app.update(Key::Char('1'));
        }
        app.update(Key::Char('h'));

        let limit = scroll_limit(&app, 52, 40);
        assert!(limit > 0);
        for _ in 0..limit.div_ceil(4) {
            app.update_bounded(Key::PageDown, limit);
        }
        let bottom = render(&app, 52, 40);
        assert!(bottom.contains("KRUX D20 MINIMUM MET"));
        assert!(bottom.contains("Additional Krux-compatible"));
    }

    #[test]
    fn coin_four_d6_protocol_uses_typed_rejection_workspace() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..7 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ Coin + four-D6 direct words"));
        assert!(menu.contains("[Enter] next"));

        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('1'));
        app.update(Key::Char('6'));
        app.update(Key::Char('h'));
        let capture = render(&app, 80, 40);
        assert!(capture.contains("PHYSICAL COIN + FOUR-D6 CAPTURE"));
        assert!(capture.contains("C=1H"));
        assert!(capture.contains("D6=6"));
        assert!(capture.contains("D6 2/4 NEXT"));
        assert!(capture.contains("whole-candidate rejection"));
    }

    #[test]
    fn seedsigner_coin_protocol_uses_binary_capture_workspace() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        for _ in 0..8 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ SeedSigner coin flips"));
        assert!(menu.contains("exactly 128 flips"));
        assert!(menu.contains("[Enter] next"));

        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        app.update(Key::Char('1'));
        let capture = render(&app, 80, 40);
        assert!(capture.contains("PHYSICAL COIN CAPTURE · FLIPS ARE SECRET"));
        assert!(capture.contains("[0] tails  [1] heads"));
        assert!(capture.contains("LATEST RECORDED · #001 · HEADS 1"));
        assert!(capture.contains("127 remaining"));
        assert!(!capture.contains("[2] [3] [4] [5] [6]"));
    }

    #[test]
    fn keystone_legacy_is_available_and_explained_for_24_words() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..3 {
            app.update(Key::Down);
        }
        let menu = render(&app, 80, 40);
        assert!(menu.contains("▶ Keystone legacy dice"));
        assert!(menu.contains("Legacy compatibility"));
        assert!(menu.contains("[Enter] next"));

        app.update(Key::Char('e'));
        let details = render(&app, 80, 40);
        assert!(details.contains("KEYSTONE LEGACY HASH · 24 WORDS ONLY"));
        assert!(details.contains("mapped text:   123450"));
    }

    #[test]
    fn keystone_legacy_generates_after_its_enforced_minimum() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..3 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..50 {
            app.update(Key::Char('6'));
        }

        let ready = render(&app, 80, 40);
        assert!(ready.contains("LEGACY KEYSTONE MINIMUM MET · 50 recorded"));
        assert!(ready.contains("Face 6 maps to ASCII 0"));

        app.update(Key::Char('\n'));
        assert!(app.generation().is_some_and(
            |generation| generation.protocol() == ConversionProtocol::KeystoneLegacyV1
        ));
        assert_eq!(
            app.generation()
                .map(|generation| generation.mnemonic().words().len()),
            Some(24)
        );
    }

    #[test]
    fn protocol_detail_keeps_a_visible_back_path_at_minimum_size() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('e'));
        let output = render(&app, 52, 40);

        for expected in [
            "PROTOCOL SELECTION",
            "▶ COLDCARD",
            "○ Word-by-word Exact",
            "○ Exact",
            "PROTOCOL DETAILS · FOCUS",
            "COLDCARD HASH · DEVICE COMPATIBILITY",
            "[e/Esc/Tab] protocols",
        ] {
            assert!(
                output.contains(expected),
                "missing {expected}\n{}",
                output.as_str()
            );
        }
        assert_eq!(
            output
                .lines()
                .filter(|line| line.starts_with(['┌', '┏']))
                .count(),
            2
        );
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn exact_explanation_uses_the_contextual_detail_card() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('e'));
        let first = render(&app, 80, 40);

        assert!(first.contains("┌─ PROTOCOL SELECTION"));
        assert!(first.contains("▶ Exact"));
        assert!(first.contains("┏━ PROTOCOL DETAILS · FOCUS"));
        assert!(first.contains("SMALL EXAMPLE · TWO DICE TO EIGHT VALUES"));
        assert!(first.contains("(1, 1) → 0"));
        assert!(first.contains("↓ more — use Page Down"));
        assert!(first.contains('█'));
        assert!(first.contains('░'));
    }

    #[test]
    fn protocol_detail_scroll_updates_content_and_rail() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('e'));
        let first = render(&app, 80, 40);

        for _ in 0..5 {
            app.update(Key::PageDown);
        }
        let later = render(&app, 80, 40);
        assert_ne!(first, later);
        assert!(later.contains("↑ more — use Page Up"));
        let rail = |screen: &str| {
            screen
                .lines()
                .filter_map(|line| line.chars().last())
                .filter(|marker| matches!(marker, '█' | '░'))
                .collect::<String>()
        };
        assert_ne!(rail(&first), rail(&later));
    }

    #[test]
    fn word_exact_explanation_documents_local_rejection() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Char('e'));
        let output = render(&app, 80, 40);

        assert!(output.contains("┏━ PROTOCOL DETAILS · FOCUS"));
        assert!(output.contains("WORD-BY-WORD EXACT · LOCALIZED REJECTION"));
        assert!(output.contains("reroll only these six dice"));
    }

    #[test]
    fn default_protocol_explanation_covers_external_compatibility() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('e'));
        let coldcard = render(&app, 80, 40);

        assert!(coldcard.contains("┏━ PROTOCOL DETAILS · FOCUS"));
        assert!(coldcard.contains("COLDCARD HASH · DEVICE COMPATIBILITY"));
        assert!(coldcard.contains("COMPATIBILITY BOUNDARY"));
        assert!(coldcard.contains("SeedSigner and Krux D6"));
    }

    #[test]
    fn coldcard_example_scrolls_through_the_complete_pipeline() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Char('e'));
        assert!(render(&app, 80, 40).contains("COMPATIBILITY BOUNDARY"));

        for _ in 0..20 {
            app.update(Key::PageDown);
        }
        let end = render(&app, 80, 40);
        assert!(end.contains("PUBLIC 12-WORD RESULT"));
        assert!(end.contains("payment owner"));
        assert!(end.contains("at least 50 rolls"));
    }

    #[test]
    fn exact_rejection_explains_scope_audit_and_secrecy() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        app.update(Key::Down);
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..50 {
            app.update(Key::Char('6'));
        }
        app.update(Key::Char('\n'));
        let output = render(&app, 80, 40);

        assert!(output.contains("NO ENTROPY RESULT"));
        assert!(output.contains("No part is reusable"));
        assert!(output.contains("Do not change one face"));
        assert!(output.contains("rejected faces concealed and still secret"));
    }

    #[test]
    fn exact_option_warns_about_the_re_roll_cost() {
        let mut app = App::default();
        app.update(Key::Char('\n'));
        let output = render(&app, 80, 40);
        assert!(output.contains("may need a full re-roll"));
    }

    #[test]
    fn narrow_terminals_keep_the_whole_phase_path() {
        let output = render(&App::default(), 52, 40);
        let stage = output.lines().nth(2).unwrap();
        assert!(stage.contains("REVEAL"));
        assert!(!stage.contains('…'));
    }

    #[test]
    fn roll_capture_distinguishes_latest_next_and_unverified_entry() {
        let mut app = roll_entry_app(0, 0);
        let initial = render(&app, 80, 40);
        assert!(initial.contains("NEXT PHYSICAL ROLL · #001"));

        app.update(Key::Char('4'));
        let recorded = render(&app, 80, 40);
        assert!(recorded.contains("LATEST RECORDED · #001 · FACE 4"));
        assert!(recorded.contains("NEXT · #002"));
        assert!(recorded.contains("cannot match the key to the die"));
        assert!(recorded.contains("never replace a valid face"));
    }

    #[test]
    fn roll_rows_include_stable_positions_and_latest_cursor() {
        assert_eq!(
            roll_rows("1234561234").as_slice(),
            ["> #001–010  12345 61234▌"]
        );
    }

    #[test]
    fn roll_rows_hold_strictly_twenty_five_positions() {
        let rows = roll_rows(&"1".repeat(51));

        assert_eq!(rows.len(), 3);
        assert!(rows[0].starts_with("  #001–025"));
        assert!(rows[1].starts_with("  #026–050"));
        assert!(rows[2].starts_with("> #051–051"));
    }

    #[test]
    fn word_exact_capture_shows_local_candidate_progress() {
        let mut app = roll_entry_app(1, 0);
        for _ in 0..6 {
            app.update(Key::Char('6'));
        }
        for _ in 0..3 {
            app.update(Key::Char('1'));
        }
        let output = render(&app, 80, 70);

        for expected in [
            "9 recorded · 0/23 word positions accepted",
            "ASSIGNMENT MAP",
            "01 ◐",
            "POSITION 01 · ATTEMPT 2 · #007–012",
            "[• • 1 ○ ○ ○]",
            "POSITION 01 · ATTEMPT 1 · #001–006 · × REJECTED",
            "KEPT IN AUDIT",
        ] {
            assert!(output.contains(expected));
        }
        assert!(!output.contains("[6 6 6 6 6 6]"));

        app.update(Key::Char('h'));
        let visible = render(&app, 80, 70);
        for expected in ["[6 6 6 6 6 6]", "[1 1 1 ○ ○ ○]"] {
            assert!(visible.contains(expected));
        }
    }

    #[test]
    fn word_exact_assignment_workspace_fits_the_minimum_terminal() {
        let app = roll_entry_app(1, 3);
        let output = render(&app, 52, 40);

        assert!(output.contains("ASSIGNMENT MAP"));
        assert!(output.contains("CURRENT CANDIDATE"));
        assert!(output.contains("POSITION 01 · ATTEMPT 1"));
        assert!(output.contains("[• • 1 ○ ○ ○]"));
        assert!(output.contains("[l] raw ledger"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn word_exact_assignment_history_scrolls_at_minimum_size() {
        let mut app = roll_entry_app(1, 60);
        let limit = scroll_limit(&app, 52, 40);
        assert!(limit > 0);

        for _ in 0..limit.div_ceil(4) {
            app.update_bounded(Key::PageDown, limit);
        }
        let output = render(&app, 52, 40);

        assert_eq!(app.roll_scroll(), limit);
        assert!(output.contains("KEPT FOR ENTROPY"));
    }

    #[test]
    fn word_exact_can_switch_to_the_raw_roll_ledger() {
        let mut app = roll_entry_app(1, 9);
        let assignments = render(&app, 80, 70);
        assert!(assignments.contains("POSITION ASSIGNMENT · SECRET"));
        assert!(!assignments.contains("ROLL LEDGER · SECRET"));

        app.update(Key::Char('l'));
        let ledger = render(&app, 80, 40);
        assert!(ledger.contains("ROLL LEDGER · SECRET · PRIOR ROLLS HIDDEN"));
        assert!(ledger.contains("[l] assignments"));
    }

    #[test]
    fn roll_capture_has_stable_sections_at_minimum_size() {
        let app = roll_entry_app(2, 36);
        let output = render(&app, 52, 40);

        assert!(output.contains("┏━ ROLL CAPTURE · FOCUS"));
        assert!(output.contains("PHYSICAL D6 CAPTURE · ROLLS ARE SECRET"));
        assert!(output.contains("[e] PROTOCOL DETAILS · Exact · exact-v1"));
        assert!(output.contains("CONVERSION · whole-stream base-6 rejection"));
        assert!(output.contains("36 recorded · 64 remaining"));
        assert!(output.contains("NEXT · #037"));
        assert!(output.contains("Backspace corrects the latest"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn roll_capture_opens_the_active_protocol_explanation() {
        let mut app = roll_entry_app(2, 1);
        app.update(Key::Char('e'));
        let details = render(&app, 80, 40);

        assert!(details.contains("┌─ ROLL CAPTURE"));
        assert!(details.contains("LATEST RECORDED · #001"));
        assert!(details.contains("┏━ PROTOCOL DETAILS · FOCUS"));
        assert!(details.contains("SMALL EXAMPLE · TWO DICE TO EIGHT VALUES"));
        assert!(details.contains("[e/Esc/Tab] roll capture"));
        assert!(!details.contains("PROTOCOL SELECTION"));
        assert_eq!(
            details
                .lines()
                .filter(|line| line.starts_with(['┌', '┏']))
                .count(),
            2
        );

        app.update(Key::Esc);
        let capture = render(&app, 80, 40);
        assert!(capture.contains("┏━ ROLL CAPTURE · FOCUS"));
        assert!(capture.contains("LATEST RECORDED · #001"));
    }

    #[test]
    fn roll_capture_defaults_to_numbered_latest_only_view() {
        let app = roll_entry_app(2, 36);
        let output = render(&app, 52, 40);

        assert!(output.contains("ROLL LEDGER · SECRET · PRIOR ROLLS HIDDEN"));
        assert!(output.contains("Rolls #001–#035 concealed"));
        assert!(output.contains("ROLL #036 REVEALED · FACE 1"));
        assert!(output.contains("[h] show all"));
        assert!(output.contains("never reroll a"));
        assert!(output.contains("surprising result"));
    }

    #[test]
    fn roll_capture_can_toggle_the_complete_ledger() {
        let mut app = roll_entry_app(2, 36);
        app.update(Key::Char('h'));
        let visible = render(&app, 52, 40);

        assert!(visible.contains("ROLL LEDGER · SECRET · ALL ROLLS VISIBLE"));
        assert!(visible.contains("> #026–036"));
        assert!(visible.contains("[h] hide prior"));
    }

    #[test]
    fn overflowing_roll_ledger_follows_latest_and_scrolls_independently() {
        let mut app = roll_entry_app(3, 126);
        app.update(Key::Char('h'));
        let latest = render(&app, 52, 40);
        assert!(latest.contains("↑ earlier rolls"));
        assert!(latest.contains("> #126–126"));
        assert!(latest.contains("126 recorded · documented minimum met"));

        app.update(Key::PageUp);
        let earlier = render(&app, 52, 40);
        assert!(earlier.contains("↓ later rolls"));
        assert!(earlier.contains("126 recorded · documented minimum met"));
    }

    /// A capture that will refuse the next roll must not advertise one.
    #[test]
    fn a_closed_capture_stops_advertising_a_next_roll() {
        let output = render(&roll_entry_app(2, 100), 80, 40);

        assert!(output.contains("NO FURTHER ROLL"));
        assert!(output.contains("This capture is closed"));
        assert!(!output.contains("NEXT · #101"));
        assert!(!output.contains("Roll once, observe"));
        // The latest roll is still reported; only the invitation goes away.
        assert!(output.contains("LATEST RECORDED · #100"));
    }

    /// COLDCARD accepts rolls past its minimum, so its invitation stays.
    #[test]
    fn an_open_capture_still_invites_the_next_roll() {
        let output = render(&roll_entry_app(3, 100), 80, 40);

        assert!(output.contains("NEXT · #101"));
        assert!(output.contains("Roll once, observe"));
        assert!(!output.contains("NO FURTHER ROLL"));
    }

    #[test]
    fn a_closed_coin_capture_stops_advertising_a_next_flip() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        for _ in 0..8 {
            app.update(Key::Down);
        }
        app.update(Key::Char('\n'));
        app.update(Key::Char('c'));
        app.update(Key::Char('\n'));
        for _ in 0..256 {
            app.update(Key::Char('1'));
        }

        let output = render(&app, 80, 40);
        assert!(output.contains("NO FURTHER FLIP"));
        assert!(output.contains("no further flip is recorded"));
        assert!(!output.contains("NEXT · #257"));
    }

    #[test]
    fn fixed_roll_completion_replaces_record_control() {
        let app = roll_entry_app(2, 100);
        let output = render(&app, 52, 40);

        assert!(output.contains("EXACT REQUIRED COUNT · 100 recorded"));
        assert!(output.contains("[Enter] confirm and generate"));
        assert!(output.contains("[Enter] generate"));
    }

    #[test]
    fn exact_and_word_exact_show_distinct_readiness_evidence() {
        let exact = render(&roll_entry_app(0, 100), 80, 40);
        assert!(exact.contains("EXACT REQUIRED COUNT · 100 recorded"));
        assert!(exact.contains("may accept or reject"));

        let word_exact = render_roll_entry(&roll_entry_app(1, 140), 80).join("\n");
        assert!(word_exact.contains("ALL ENTROPY POSITIONS + FINAL TAIL ACCEPTED"));
        assert!(word_exact.contains("checksum will be calculated, not rolled"));
    }

    #[test]
    fn coldcard_roll_capture_updates_hash_from_empty_prefix() {
        let empty = render_roll_entry(&roll_entry_app(3, 0), 80).join("\n");
        for expected in [
            "COLDCARD RUNNING SHA-256 · SECRET",
            "CURRENT PREFIX · EMPTY ROLL STRING",
            "e3b0c4…52b855  · full hash withheld",
            "99 more roll(s) before generation",
        ] {
            assert!(empty.contains(expected));
        }
        // The full 64-char digest must never reach the screen.
        assert!(!empty.contains("e3b0c44298fc1c149afbf4c8996fb924"));
        assert!(!empty.contains("27ae41e4649b934ca495991b7852b855"));

        let one_roll = render_roll_entry(&roll_entry_app(3, 1), 80).join("\n");
        assert!(one_roll.contains("CURRENT PREFIX · 1 ASCII ROLL DIGIT(S)"));
        assert!(one_roll.contains("6b86b2…875b4b  · full hash withheld"));
        assert!(!one_roll.contains("6b86b273ff34fce19d6b804eff5a3f57"));
    }

    #[test]
    fn coldcard_roll_capture_can_generate_or_keep_rolling_after_minimum() {
        let ready = render_roll_entry(&roll_entry_app(3, 99), 80).join("\n");
        for expected in [
            "✓ MINIMUM MET · [Enter] generate or [1–6] keep rolling",
            "[Enter] generate · [1–6] add extra",
            "Additional COLDCARD-compatible rolls are optional",
        ] {
            assert!(ready.contains(expected));
        }

        let extra = render_roll_entry(&roll_entry_app(3, 100), 80).join("\n");
        assert!(extra.contains("CURRENT PREFIX · 100 ASCII ROLL DIGIT(S)"));
        assert!(extra.contains("✓ MINIMUM MET"));
    }

    #[test]
    fn coldcard_distribution_rejection_explains_fresh_attempt() {
        let mut app = roll_entry_app(3, 99);
        app.update(Key::Char('\n'));
        let output = render(&app, 80, 40);

        assert!(output.contains("COLDCARD ATTEMPT REJECTED"));
        assert!(output.contains("more than 30% of the time"));
        assert!(output.contains("No part is reusable"));
        assert!(output.contains("fresh attempt"));
    }

    #[test]
    fn coldcard_completion_explains_optional_and_hundred_roll_states() {
        let documented = render(&roll_entry_app(3, 99), 80, 40);
        assert!(documented.contains("DOCUMENTED MINIMUM MET · 99 recorded"));
        assert!(documented.contains("Additional COLDCARD-compatible rolls are optional"));

        let hundred = render(&roll_entry_app(3, 100), 52, 40);
        assert!(hundred.contains("100 fair independent D6 rolls have >256 bits"));
        assert!(hundred.contains("[1–6] add extra"));
        assert!(hundred.contains("[1–6] extra"));
        assert_eq!(hundred.lines().count(), 40);
    }

    #[test]
    fn minimum_terminal_shows_the_complete_stable_mnemonic() {
        let app = revealed_twenty_four_words();
        let output = render(&app, 52, 40);
        assert!(output.contains("01  abandon"));
        assert!(output.contains("24  art"));
        assert!(!output.contains("more — use Page"));
        assert_eq!(output.lines().count(), 40);
        assert!(output.lines().all(|line| line.chars().count() <= 52));
    }

    #[test]
    fn ceremony_uses_one_full_height_workspace_card() {
        let output = render(&App::default(), 80, 40);
        assert_eq!(
            output.lines().filter(|line| line.starts_with('┏')).count(),
            1
        );
        assert_eq!(
            output.lines().filter(|line| line.starts_with('┗')).count(),
            1
        );
        assert!(!output.contains(" STATE"));
    }

    #[test]
    fn details_replace_the_ceremony_in_the_same_workspace() {
        let mut app = revealed_twenty_four_words();
        app.update(Key::Char('d'));
        let output = render(&app, 80, 40);
        assert!(output.contains("┏━ DERIVATION · ALL VALUES SECRET · FOCUS"));
        assert_eq!(
            output.lines().filter(|line| line.starts_with('┏')).count(),
            1
        );
        assert!(!output.contains("┏━ CEREMONY"));
    }

    #[test]
    fn taller_terminals_expand_derivation_details() {
        let mut app = revealed_twenty_four_words();
        app.update(Key::Char('d'));
        let output = render(&app, 80, 70);
        let state = output
            .split("┏━ DERIVATION · ALL VALUES SECRET · FOCUS")
            .nth(1)
            .unwrap();

        for expected in [
            "◆ DERIVATION · ALL VALUES SECRET",
            "Press h to conceal",
            "DETERMINISTIC PIPELINE",
            "01 · CANONICAL INPUT",
            "02 · BIP-39 ENTROPY",
            "03 · CALCULATED CHECKSUM",
            "04 · 11-BIT WORD INDICES",
            "05 · BIP-39 RECOVERY WORDS",
            "VERIFY THE CHAIN",
        ] {
            assert!(state.contains(expected));
        }
        assert!(output.contains("[h] hide"));
        assert!(output.contains("[q] finish"));
    }

    #[test]
    fn reveal_keeps_secret_exposure_and_device_warnings_visible() {
        let app = revealed_twenty_four_words();
        let output = render(&app, 80, 40);
        assert!(output.contains("SECRET NOW VISIBLE"));
        assert!(output.contains("networked device"));
        assert!(!output.contains("↓ more — use Page Down"));
    }

    #[test]
    fn reveal_can_be_quick_hidden_without_rendering_words() {
        let mut app = revealed_twenty_four_words();
        app.update(Key::Char('h'));
        let hidden = render(&app, 80, 40);

        assert!(hidden.contains("MNEMONIC QUICK-HIDDEN"));
        assert!(hidden.contains("[h/Esc] show mnemonic"));
        assert!(!hidden.contains("01  abandon"));

        app.update(Key::Char('h'));
        assert!(render(&app, 80, 40).contains("01  abandon"));
    }

    #[test]
    fn completed_ceremony_uses_end_not_cancel_confirmation() {
        let mut app = revealed_twenty_four_words();
        app.update(Key::Char('q'));
        let output = render(&app, 80, 40);

        assert!(output.contains("END CEREMONY?"));
        assert!(output.contains("SECRET TEMPORARILY CONCEALED"));
        assert!(!output.contains("01  abandon"));
        assert!(!output.contains("CANCEL CEREMONY?"));
        assert!(output.contains("OWNED BUFFERS WILL BE CLEARED"));
        assert!(output.contains("NOT CONTROLLED BY THIS APPLICATION"));
        assert!(output.contains("swap, crash dumps, cameras, or OS copies"));
        assert!(output.contains("return to mnemonic"));
    }

    #[test]
    fn reveal_explains_checksum_limit() {
        let output = render(&revealed_twenty_four_words(), 80, 40);

        assert!(output.contains("checksum catches many mistakes, not every wrong word"));
    }

    #[test]
    fn backup_verification_conceals_words_and_masks_input() {
        let mut app = revealed_twenty_four_words();
        app.update(Key::Char('v'));
        app.update(Key::Char('a'));
        app.update(Key::Char('b'));
        let output = render(&app, 52, 40);

        assert!(output.contains("WORD-BY-WORD BACKUP VERIFICATION"));
        assert!(output.contains("WORD POSITION 01 OF 24"));
        assert!(output.contains("INPUT  ••  · 2 letters"));
        assert!(!output.contains("01  abandon"));
        assert!(!output.contains("INPUT  ab"));
        assert!(output.contains("[Enter] compare"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn checked_reveal_sets_downstream_recovery_boundary() {
        let mut app = revealed_twenty_four_words();
        verify_mnemonic(&mut app);
        let output = render(&app, 100, 40);

        assert!(output.contains("REPRODUCTION METADATA · SECRET VALUES OMITTED"));
        assert!(output.contains("Protocol exact-v1 · Target 24 words · Recorded rolls 100"));
        assert!(output.contains("Secret faces omitted"));
        assert!(output.contains("GENERATION IS NOT VERIFIED WALLET RECOVERY"));
        assert!(output.contains("passphrase, network, script, derivation, or descriptor"));
        assert!(output.contains("restore and verify addresses"));
    }

    #[test]
    fn reveal_workspace_prioritizes_words_check_and_finish_controls() {
        let mut app = revealed_twenty_four_words();
        let output = render(&app, 80, 40);
        assert!(output.contains("BIP-39 RECOVERY WORDS · 24 WORDS · SECRET"));
        assert!(output.contains("01  abandon"));
        assert!(output.contains("24  art"));
        assert!(output.contains("TRANSCRIPTION CHECK"));
        assert!(output.contains("[v] verify backup"));

        verify_mnemonic(&mut app);
        let checked = render(&app, 80, 40);
        assert!(checked.contains("BACKUP WORDS VERIFIED IN ORDER"));
        assert!(checked.contains("[Enter/q] finish"));
        assert_eq!(checked.lines().count(), 40);
        assert!(checked.lines().all(|line| line.chars().count() <= 80));
    }

    fn group_results_app() -> App {
        let mut app = App::default();
        app.update(Key::Down); // 24 words
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        for index in 0..100 {
            let face = u8::try_from((index % 6) + 1).unwrap();
            app.update(Key::Char(char::from(b'0' + face)));
        }
        app.update(Key::Char('\n'));
        app
    }

    #[test]
    fn group_collect_screen_shows_entry_guidance() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        let output = render(&app, 80, 40);

        assert!(output.contains("◆ GROUP COMPARE  ›  COLLECT ROLLS"));
        assert!(output.contains("┏━ GROUP COMPARE · COLLECT · FOCUS"));
        assert!(output.contains("PHYSICAL D6 CAPTURE · ROLLS ARE SECRET"));
        assert!(output.contains("NEXT PHYSICAL ROLL · #001"));
        assert!(output.contains("[Enter] compare"));
        assert_eq!(output.lines().count(), 40);
    }

    #[test]
    fn group_results_conceal_seeds_until_reveal() {
        let mut app = group_results_app();
        let concealed = render(&app, 80, 40);

        // 100-roll capture: one entropy set at 100 (word-exact needs 140, so it
        // isn't in any set yet).
        assert!(concealed.contains("◆ ENTROPY SET · 100 ROLLS"));
        assert!(concealed.contains("✓ Exact"));
        assert!(concealed.contains("accepted"));
        assert!(concealed.contains("[r] reveals the accepted seeds."));

        let report = app.group().unwrap().comparison_at(0);
        let first_word = report
            .sets()
            .iter()
            .find(|s| s.rolls == 100)
            .unwrap()
            .protocols[0]
            .1
            .calculation()
            .unwrap()
            .mnemonic()
            .words()[0]
            .clone();
        assert!(!concealed.contains(&format!("      {first_word}")));

        app.update(Key::Char('r'));
        let revealed = render(&app, 80, 40);
        assert!(!revealed.contains("[r] reveals the accepted seeds."));
        assert!(revealed.contains(&first_word));
    }

    #[test]
    fn group_help_panel_explains_the_entropy_set_model() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        app.update(Key::Char('?'));
        let output = render(&app, 80, 44);

        assert!(output.contains("◆ GROUP COMPARE  ›  HELP"));
        assert!(output.contains("GROUP COMPARE · HOW IT WORKS"));
        assert!(output.contains("checkpoint"));
        assert!(output.contains("ENTROPY SETS"));
        assert!(output.contains("[?/Esc] close"));
    }

    #[test]
    fn group_details_card_shows_the_protocol_explanation() {
        let mut app = App::default();
        app.update(Key::Down); // 24 words
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        app.update(Key::Char('e')); // open details on the first protocol (Exact)
        let exact = render(&app, 80, 44);

        assert!(exact.contains("◆ GROUP COMPARE  ›  PROTOCOL DETAILS"));
        assert!(exact.contains("┏━ GROUP COMPARE · DETAILS · FOCUS"));
        assert!(exact.contains("PROTOCOL DETAILS · 1 OF 4"));
        assert!(exact.contains("▶ Exact"));
        assert!(exact.contains("[←/→] protocol"));

        // Stepping right renders the next protocol's document (COLDCARD).
        app.update(Key::Right);
        let coldcard = render(&app, 80, 44);
        assert!(coldcard.contains("PROTOCOL DETAILS · 2 OF 4"));
        assert!(coldcard.contains("▶ COLDCARD"));
        assert!(coldcard.contains("COLDCARD HASH"));
    }

    #[test]
    fn group_derivation_card_shows_entropy_and_word_stages() {
        let mut app = group_results_app(); // 100 fair rolls, on the results screen
        app.update(Key::Char('d'));
        let output = render(&app, 80, 44);

        // Same numbered BIP-39 stages as the single-protocol derivation view.
        assert!(output.contains("◆ GROUP COMPARE  ›  DERIVATION"));
        assert!(output.contains("● SECRET EXPOSED"));
        assert!(output.contains("GROUP COMPARE · DERIVATION · 1 OF"));
        assert!(output.contains("◆ ENTROPY SET · 100 ROLLS"));
        assert!(output.contains("01 · CANONICAL INPUT"));
        assert!(output.contains("02 · BIP-39 ENTROPY"));
        assert!(output.contains("04 · 11-BIT WORD INDICES"));
        assert!(output.contains("05 · BIP-39 RECOVERY WORDS"));
        assert!(output.contains("[←/→] seed"));
    }

    #[test]
    fn derivation_puts_the_encoding_label_above_the_unlabelled_input() {
        let output = {
            let mut app = group_results_app(); // first accepted seed: Exact (base6)
            app.update(Key::Char('d'));
            render(&app, 80, 44)
        };
        // The encoding sits on its own line above the input…
        assert!(
            output.contains("encoding · base-6 (0-5), msb-first (left-to-right), global rejection")
        );
        // …and the input line no longer carries the "label:content" prefix.
        assert!(!output.contains("global rejection:"));
    }

    #[test]
    fn group_invalid_set_explains_the_fresh_set_remedy() {
        let mut app = App::default();
        app.update(Key::Down);
        app.update(Key::Char('\n'));
        app.update(Key::Char('g'));
        for _ in 0..100 {
            app.update(Key::Char('6'));
        }
        app.update(Key::Char('\n'));
        let output = render(&app, 80, 40);

        assert!(output.contains("✗ Exact"));
        assert!(output.contains("rejected · value out of range"));
        assert!(output.contains("for a fresh set"));
        assert!(output.contains("[n] fresh"));
    }
}
