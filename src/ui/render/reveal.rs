use std::fmt::Write;

use zeroize::Zeroizing;

use crate::{
    application::ReproductionReceipt,
    domain::ceremony::Phase,
    presentation::{transcription_instructions, verified_recovery},
};

use super::frame::render_card;
use super::{
    App, Lines, clip, footer_line, lines, push, push_output, push_owned, render_quit, stage_line,
    title_line, wrap_words,
};

pub(super) fn compose_reveal_workspace(
    app: &App,
    width: usize,
    height: usize,
) -> Zeroizing<String> {
    let mut output = Zeroizing::new(String::with_capacity(
        width.saturating_mul(height).saturating_add(height),
    ));
    let separator = "─".repeat(width);
    let concealed = app.quit_pending();
    let hidden = app.mnemonic_hidden();
    let verifying = app.mnemonic_verification().is_some();
    let title_status = if concealed || hidden || verifying {
        "● SECRET CONCEALED"
    } else {
        "● SECRET EXPOSED"
    };
    push_output(&mut output, &clip(&title_line(width, title_status), width));
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&stage_line(app, width), width));
    push_output(&mut output, "");

    if concealed {
        render_reveal_exit(app, &mut output, width, height);
    } else if verifying {
        render_mnemonic_verification(app, &mut output, width, height);
    } else if hidden {
        render_quick_hidden(&mut output, width, height);
    } else {
        render_exposed_mnemonic(app, &mut output, width, height);
    }

    let status = reveal_status(app, concealed, hidden, verifying);
    push_output(&mut output, &clip(status, width));
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&footer_line(app), width));
    output
}

fn render_reveal_exit(app: &App, output: &mut String, width: usize, height: usize) {
    let return_text = if app.mnemonic_hidden() {
        "Press Esc or n to return to the quick-hidden screen."
    } else {
        "Press Esc or n to return to the mnemonic."
    };
    let banner = lines(&[
        "The recovery words are hidden while exit confirmation is open.",
        return_text,
    ]);
    render_secret_card(output, "SECRET TEMPORARILY CONCEALED", &banner, 3, width);
    let quit = if app.mnemonic_hidden() {
        lines(&[
            "END CEREMONY?",
            "",
            "All in-memory input history and generated secret material will be",
            "destroyed. Confirm that transcription is complete before exiting.",
            "",
            "[q] or Enter   End and exit",
            "[n] or Esc     Return to quick-hidden screen",
        ])
    } else {
        render_quit(Phase::Revealed, width.saturating_sub(4))
    };
    render_card(
        output,
        "END CEREMONY",
        &quit,
        height.saturating_sub(14),
        0,
        false,
        width,
    );
}

fn render_mnemonic_verification(app: &App, output: &mut String, width: usize, height: usize) {
    let Some((position, entry_len)) = app.mnemonic_verification() else {
        return;
    };
    let total = app
        .generation()
        .map_or(0, |generation| generation.mnemonic().words().len());
    let mut body = lines(&[
        "MNEMONIC HIDDEN · BACKUP COMPARISON",
        "",
        &format!("WORD POSITION {:02} OF {total:02}", position + 1),
        "Read this numbered position from the written backup, then type it.",
        &format!("INPUT  {}  · {entry_len} letters", "•".repeat(entry_len)),
        "Characters are masked; the temporary input is cleared after comparison.",
        "",
        "[Enter] compare word   [Backspace] correct   [Esc] cancel verification",
        "! Re-entry adds keyboard exposure; it still cannot inspect the physical backup.",
    ]);
    if let Some(message) = app.message() {
        push(&mut body, "");
        push(&mut body, &format!("× {message}"));
    }
    render_secret_card(
        output,
        "WORD-BY-WORD BACKUP VERIFICATION",
        &body,
        height.saturating_sub(9),
        width,
    );
}

fn render_quick_hidden(output: &mut String, width: usize, height: usize) {
    let body = lines(&[
        "MNEMONIC QUICK-HIDDEN",
        "",
        "The recovery words remain in memory but are not drawn on screen.",
        "This does not clear terminal recordings or earlier screen captures.",
        "",
        "[h] or Esc   Show mnemonic again",
        "[q]          Finish securely",
    ]);
    render_secret_card(
        output,
        "SECRET TEMPORARILY CONCEALED",
        &body,
        height.saturating_sub(9),
        width,
    );
}

fn render_exposed_mnemonic(app: &App, output: &mut String, width: usize, height: usize) {
    let banner_text = "Anyone who can see or record these words may recover the wallet. \
        Keep cameras, screen sharing, and terminal recording off.";
    let banner = wrap_words(banner_text, width.saturating_sub(4));
    let banner_rows = banner.len().max(2);
    render_secret_card(output, "SECRET NOW VISIBLE", &banner, banner_rows, width);

    let mut words = Lines::new(Vec::new());
    push(&mut words, "");
    let mnemonic_rows = mnemonic_word_rows(app, width.saturating_sub(4));
    words.extend(mnemonic_rows.iter().cloned());
    push(&mut words, "");
    let words_rows = words.len();
    let word_count = app
        .generation()
        .map_or(0, |generation| generation.mnemonic().words().len());
    render_secret_card(
        output,
        &format!("BIP-39 RECOVERY WORDS · {word_count} WORDS · SECRET"),
        &words,
        words_rows,
        width,
    );

    let check = transcription_check(app, width.saturating_sub(4));
    let fixed = 13 + banner_rows + words_rows;
    let check_rows = height.saturating_sub(fixed).max(check.len());
    render_card(
        output,
        "TRANSCRIPTION CHECK",
        &check,
        check_rows,
        0,
        false,
        width,
    );
}

fn transcription_check(app: &App, width: usize) -> Lines {
    let instructions = transcription_instructions();
    let rendered = super::protocol::render_document(&instructions, width);
    let mut check = Lines::new(
        rendered
            .iter()
            .filter(|line| !line.is_empty())
            .cloned()
            .collect(),
    );
    if app.ceremony().state().mnemonic_backup_verified() {
        if let Some(receipt) = ReproductionReceipt::from_state(&app.ceremony().state()) {
            let document = verified_recovery(receipt);
            let rendered = super::protocol::render_document(&document, width);
            check.extend(rendered.iter().filter(|line| !line.is_empty()).cloned());
        }
        push(
            &mut check,
            "Press Enter or q to clear owned buffers and finish.",
        );
    } else {
        push(&mut check, "[v] Verify written backup word by word");
        push(
            &mut check,
            "Verification conceals this list and compares every numbered position.",
        );
    }
    check
}

fn reveal_status(app: &App, concealed: bool, hidden: bool, verifying: bool) -> &'static str {
    if concealed {
        "○ SECRET TEMPORARILY CONCEALED · exit confirmation"
    } else if verifying {
        "◇ LIVE · mnemonic concealed · comparing temporary typed word"
    } else if hidden {
        "○ LIVE · mnemonic quick-hidden · secret remains in memory"
    } else if app.ceremony().state().mnemonic_backup_verified() {
        "✓ LIVE · mnemonic revealed · backup words verified · in memory only"
    } else {
        "● LIVE · mnemonic revealed · inputs and secret remain in memory only"
    }
}

fn render_secret_card(output: &mut String, title: &str, body: &Lines, rows: usize, width: usize) {
    let prefix = format!("╔═ {title} ");
    let fill = width.saturating_sub(prefix.chars().count() + 1);
    push_output(output, &format!("{prefix}{}╗", "═".repeat(fill)));
    for value in body.iter().take(rows) {
        let inner_width = width.saturating_sub(4);
        let value = Zeroizing::new(clip(value, inner_width));
        let padding = inner_width.saturating_sub(value.chars().count());
        let line = Zeroizing::new(format!("║ {}{} ║", value.as_str(), " ".repeat(padding)));
        push_output(output, &line);
    }
    for _ in body.len().min(rows)..rows {
        push_output(
            output,
            &format!("║ {} ║", " ".repeat(width.saturating_sub(4))),
        );
    }
    push_output(
        output,
        &format!("╚{}╝", "═".repeat(width.saturating_sub(2))),
    );
}

/// Renders the stable numbered mnemonic body for the reveal workspace.
pub(super) fn render_mnemonic(app: &App, width: usize) -> Lines {
    let mut output = lines(&[
        "BIP-39 RECOVERY WORDS · SECRET",
        "Write in numbered order. Compare your copy twice.",
        "",
    ]);
    let mnemonic_rows = mnemonic_word_rows(app, width);
    output.extend(mnemonic_rows.iter().cloned());
    output
}

fn mnemonic_word_rows(app: &App, width: usize) -> Lines {
    let Some(generation) = app.generation() else {
        return lines(&["Mnemonic unavailable."]);
    };
    let mut output = Lines::new(Vec::new());
    let words = generation.mnemonic().words();
    let columns = 3;
    let rows = words.len().div_ceil(columns);
    let column_width = width / columns;
    for row in 0..rows {
        let mut value = Zeroizing::new(String::with_capacity(width));
        for column in 0..columns {
            let index = row + column * rows;
            if let Some(word) = words.get(index) {
                let cell = Zeroizing::new(format!("{:02}  {word}", index + 1));
                write!(value, "{:<column_width$}", cell.as_str())
                    .expect("writing mnemonic cell to String cannot fail");
            }
        }
        push_owned(&mut output, value.trim_end().to_owned());
    }
    output
}
