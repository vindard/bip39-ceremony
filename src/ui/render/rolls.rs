mod bitbox;
mod coin_four_d6;
mod d20;
mod jade;
mod word_exact;

use crate::{
    application::CaptureReadiness,
    domain::protocol::{CaptureAssessment, CaptureProgress, ConversionProtocol, WordExactProgress},
    presentation::{capture_protocol_context, readiness_document},
};
use word_exact::render_word_exact_assignment;

use super::{App, Lines, lines, push, push_owned, push_wrapped};

pub(super) fn render_roll_entry(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    if state.protocol() == Some(ConversionProtocol::JadeDirectV1) {
        return jade::render_jade_entry(app, width);
    }
    if state.protocol() == Some(ConversionProtocol::BitBox02DirectV1) {
        return bitbox::render_bitbox_entry(app, width);
    }
    if state.protocol() == Some(ConversionProtocol::CoinFourD6DirectV1) {
        return coin_four_d6::render_coin_four_d6_entry(app, width);
    }
    if state.protocol() == Some(ConversionProtocol::KruxD20V1) {
        return d20::render_d20_entry(app, width);
    }
    let count = state.capture_count();
    let required = state.required_rolls().unwrap_or(0);
    // Bit packing has no fixed tape length, so measuring the bar in rolls would
    // read "74 / 64". Its progress is the entropy width it has filled.
    let packed = match state.capture_assessment().map(CaptureAssessment::progress) {
        Some(CaptureProgress::BitPack(bitpack)) => Some((bitpack.bits(), bitpack.required_bits())),
        _ => None,
    };
    let (bar_count, bar_required) = packed.unwrap_or((count, required));
    let coin_capture = state.protocol() == Some(ConversionProtocol::SeedSignerCoinsV1);
    let word_exact = state.protocol() == Some(ConversionProtocol::WordExactV1);
    let ledger_title = capture_ledger_title(app, word_exact, coin_capture);
    let protocol = state.protocol().expect("roll capture requires a protocol");
    let protocol_context = capture_protocol_context(protocol);
    let protocol_line = format!(
        "[e] PROTOCOL DETAILS · {} · {}",
        protocol_context.name(),
        protocol.id()
    );
    let rule_line = format!("CONVERSION · {}", protocol_context.detail());
    // A closed capture must not advertise a roll it will refuse to record.
    let open = state
        .capture_assessment()
        .is_none_or(CaptureAssessment::can_record_more);
    let cursor = zeroize::Zeroizing::new(if coin_capture {
        flip_capture_cursor(state.flips(), open)
    } else {
        capture_cursor(state.rolls(), open)
    });
    let capture_heading = if coin_capture {
        "PHYSICAL COIN CAPTURE · FLIPS ARE SECRET"
    } else {
        "PHYSICAL D6 CAPTURE · ROLLS ARE SECRET"
    };
    let capture_instruction = match (open, coin_capture) {
        (true, true) => "Flip once, observe, then press: [0] tails  [1] heads",
        (true, false) => "Roll once, observe, then press: [1] [2] [3] [4] [5] [6]",
        (false, true) => "This capture is closed; no further flip is recorded.",
        (false, false) => "This capture is closed; no further roll is recorded.",
    };
    let mut output = lines(&[
        capture_heading,
        &protocol_line,
        "[t] PHYSICAL ENTROPY · throwing method",
        &rule_line,
        &cursor,
        capture_instruction,
        "",
        &progress(bar_count, bar_required, width),
        &roll_count_status(count, state.capture_assessment()),
    ]);

    if let Some(preview) = app.coldcard_hash_preview() {
        render_coldcard_hash_preview(
            &mut output,
            preview.digest_hex(),
            preview.roll_count(),
            required,
        );
    }
    push(&mut output, ledger_title);

    render_capture_ledger(&mut output, app, &state, width, word_exact, coin_capture);
    push(&mut output, "");

    if state.can_confirm_rolls() {
        render_roll_completion(&mut output, &state, width);
    } else {
        render_incomplete_guidance(&mut output, app, &state, width, word_exact, coin_capture);
    }
    output
}

fn capture_ledger_title(app: &App, word_exact: bool, coin_capture: bool) -> &'static str {
    if word_exact && !app.word_exact_raw_ledger() {
        "POSITION ASSIGNMENT · SECRET"
    } else if coin_capture && app.rolls_hidden() {
        "FLIP LEDGER · SECRET · PRIOR FLIPS HIDDEN"
    } else if coin_capture {
        "FLIP LEDGER · SECRET · ALL FLIPS VISIBLE"
    } else if app.rolls_hidden() {
        "ROLL LEDGER · SECRET · PRIOR ROLLS HIDDEN"
    } else {
        "ROLL LEDGER · SECRET · ALL ROLLS VISIBLE"
    }
}

fn render_capture_ledger(
    output: &mut Lines,
    app: &App,
    state: &crate::domain::ceremony::CeremonyState,
    width: usize,
    word_exact: bool,
    coin_capture: bool,
) {
    if word_exact && !app.word_exact_raw_ledger() {
        render_word_exact_assignment(output, app, width);
        return;
    }
    let capture_digits = if coin_capture {
        state.flips().ascii_string()
    } else {
        state.rolls().ascii_string()
    };
    if app.rolls_hidden() && coin_capture {
        render_hidden_flip_ledger(output, capture_digits.as_str(), 4);
    } else if app.rolls_hidden() {
        render_hidden_roll_ledger(output, capture_digits.as_str(), 4);
    } else {
        let ledger = roll_rows(capture_digits.as_str());
        render_roll_ledger(output, &ledger, app.roll_scroll(), 4);
    }
}

fn render_incomplete_guidance(
    output: &mut Lines,
    app: &App,
    state: &crate::domain::ceremony::CeremonyState,
    width: usize,
    word_exact: bool,
    coin_capture: bool,
) {
    if word_exact && !app.word_exact_raw_ledger() {
        push(
            output,
            "Rejected candidates retry only their position; accepted positions stay kept.",
        );
    } else if let Some(CaptureProgress::WordExact(progress)) =
        state.capture_assessment().map(CaptureAssessment::progress)
    {
        render_word_exact_roll_guidance(output, progress);
    } else {
        push(output, "Record every valid physical result exactly once.");
    }
    push_wrapped(
        output,
        "",
        if coin_capture {
            "Repeats and patterns are valid—never replace a surprising flip."
        } else {
            "Repeats and patterns are valid—never reroll a surprising result."
        },
        width,
    );
    if coin_capture {
        push(
            output,
            "! The app validates 0/1; it cannot observe the physical coin.",
        );
        push(
            output,
            "Backspace corrects the latest entry—never replace a valid flip.",
        );
    } else {
        push(
            output,
            "! The app validates 1–6; it cannot match the key to the die.",
        );
        push(
            output,
            "Backspace corrects the latest entry—never replace a valid face.",
        );
    }
}

fn render_coldcard_hash_preview(
    output: &mut Lines,
    digest_hex: &str,
    recorded: usize,
    minimum: usize,
) {
    push(output, "");
    push(output, "COLDCARD RUNNING SHA-256 · SECRET");
    if recorded == 0 {
        push(output, "CURRENT PREFIX · EMPTY ROLL STRING");
    } else {
        push(
            output,
            &format!("CURRENT PREFIX · {recorded} ASCII ROLL DIGIT(S)"),
        );
    }
    // The Coldcard digest IS the 256-bit entropy; show only a short
    // fingerprint so the operator can cross-check against the device without
    // painting the full seed on screen.
    push(
        output,
        &format!("  {}  · full hash withheld", fingerprint(digest_hex)),
    );
    if recorded < minimum {
        push(
            output,
            &format!(
                "○ CANDIDATE ONLY · {} more roll(s) before generation",
                minimum - recorded
            ),
        );
    } else {
        push(
            output,
            "✓ MINIMUM MET · [Enter] generate or [1–6] keep rolling",
        );
    }
    push(output, "");
}

/// A short leading…trailing view of a hex digest. Enough to compare against a
/// device's displayed hash; too little to reconstruct the seed entropy.
fn fingerprint(digest_hex: &str) -> String {
    const LEAD: usize = 6;
    const TAIL: usize = 6;
    if digest_hex.len() > LEAD + TAIL {
        format!(
            "{}…{}",
            &digest_hex[..LEAD],
            &digest_hex[digest_hex.len() - TAIL..]
        )
    } else {
        digest_hex.to_owned()
    }
}

fn flip_capture_cursor(flips: &crate::domain::coin::FlipSequence, open: bool) -> String {
    let next = next_marker(flips.len(), open, "FLIP");
    flips.flips().last().map_or_else(
        || format!("NEXT PHYSICAL FLIP · #{:03}", flips.len() + 1),
        |flip| {
            let side = if flip.get() == 0 { "TAILS" } else { "HEADS" };
            format!(
                "LATEST RECORDED · #{:03} · {side} {}{next}",
                flips.len(),
                flip.get()
            )
        },
    )
}

fn capture_cursor(rolls: &crate::domain::dice::RollSequence, open: bool) -> String {
    let next = next_marker(rolls.len(), open, "ROLL");
    rolls.faces().last().map_or_else(
        || format!("NEXT PHYSICAL ROLL · #{:03}", rolls.len() + 1),
        |face| {
            format!(
                "LATEST RECORDED · #{:03} · FACE {}{next}",
                rolls.len(),
                face.get()
            )
        },
    )
}

/// The trailing cursor segment: the next position while the capture still
/// accepts one, and an explicit close otherwise.
fn next_marker(recorded: usize, open: bool, unit: &str) -> String {
    if open {
        format!("  |  NEXT · #{:03}", recorded + 1)
    } else {
        format!("  |  NO FURTHER {unit}")
    }
}

pub(super) fn render_word_exact_roll_guidance(output: &mut Lines, progress: WordExactProgress) {
    match progress {
        WordExactProgress::WordCandidate {
            accepted_words,
            required_words,
            candidate_rolls,
            ..
        } => {
            push(
                output,
                &format!(
                    "WORD POSITION {:02} OF {required_words:02} · CANDIDATE {candidate_rolls} / 6",
                    accepted_words + 1,
                ),
            );
            push(output, "Complete this six-roll candidate.");
        }
        WordExactProgress::EntropyTail {
            candidate_rolls,
            candidate_size,
            ..
        } => {
            push(
                output,
                &format!("FINAL ENTROPY BITS · CANDIDATE {candidate_rolls} / {candidate_size}"),
            );
            push(
                output,
                "The checksum is calculated after these entropy bits.",
            );
        }
    }
    let rejected_candidates = progress.rejected_candidates();
    if rejected_candidates > 0 {
        push(
            output,
            &format!(
                "{rejected_candidates} localized candidate rejection(s) · accepted positions kept"
            ),
        );
    }
}

pub(super) fn roll_count_status(count: usize, assessment: Option<CaptureAssessment>) -> String {
    let Some(assessment) = assessment else {
        return format!("{count} recorded");
    };
    match assessment.progress() {
        CaptureProgress::Fixed { recorded, required } if recorded < required => {
            format!("{recorded} recorded · {} remaining", required - recorded)
        }
        CaptureProgress::Fixed { recorded, .. } => {
            format!("{recorded} recorded · requirement met")
        }
        CaptureProgress::OpenEnded { recorded, minimum } if recorded < minimum => {
            format!("{recorded} recorded · {} remaining", minimum - recorded)
        }
        CaptureProgress::OpenEnded { recorded, .. } => {
            format!("{recorded} recorded · documented minimum met")
        }
        CaptureProgress::Jade(progress) => format!(
            "{} recorded · {}/{} direct positions complete",
            progress.recorded(),
            progress.completed_words(),
            progress.required_words()
        ),
        CaptureProgress::BitBox(progress) => format!(
            "{} recorded · {}/{} direct positions · {} rejected D6",
            progress.recorded(),
            progress.completed_words(),
            progress.required_words(),
            progress.rejected_faces()
        ),
        CaptureProgress::BitPack(progress) => format!(
            "{} recorded · {}/{} entropy bits packed",
            progress.recorded(),
            progress.bits(),
            progress.required_bits()
        ),
        CaptureProgress::IanColemanRaw(progress) => format!(
            "{} recorded · {} bits packed · {}/{} kept",
            progress.recorded(),
            progress.bits(),
            progress.usable_bits(),
            progress.required_bits()
        ),
        CaptureProgress::CoinFourD6(progress) => format!(
            "{} recorded · {}/12 accepted · {} rejected candidates",
            progress.recorded(),
            progress.completed_candidates(),
            progress.rejected_candidates()
        ),
        CaptureProgress::WordExact(progress) => format!(
            "{count} recorded · {}/{} word positions accepted",
            progress.accepted_words(),
            progress.required_words()
        ),
        CaptureProgress::WordExactComplete {
            accepted_words,
            required_words,
            ..
        } => {
            format!("{count} recorded · {accepted_words}/{required_words} word positions accepted")
        }
    }
}

pub(super) fn render_roll_completion(
    output: &mut Lines,
    state: &crate::domain::ceremony::CeremonyState,
    width: usize,
) {
    let Some(readiness) = CaptureReadiness::from_state(state) else {
        return;
    };
    let assessment = state.capture_assessment();
    let document = readiness_document(readiness);
    let rendered = super::protocol::render_document(&document, width);
    let visible = rendered
        .iter()
        .filter(|line| !line.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if assessment.is_some_and(CaptureAssessment::accepts_optional_rolls) {
        output.extend(visible.iter().take(2).cloned());
        push(output, "[Enter] generate · [1–6] add extra");
        output.extend(visible.iter().skip(2).cloned());
    } else {
        output.extend(visible);
        push(output, "[Enter] confirm and generate");
    }
}

fn render_hidden_flip_ledger(output: &mut Lines, flips: &str, rows: usize) {
    let count = flips.len();
    let Some(latest) = flips.chars().last() else {
        push(output, "  Waiting for the first physical flip…");
        for _ in 1..rows {
            push(output, "");
        }
        return;
    };
    if count == 1 {
        push(output, "  No prior flips to conceal");
    } else {
        push(output, &format!("  Flips #001–#{:03} concealed", count - 1));
    }
    let side = if latest == '0' { "TAILS" } else { "HEADS" };
    push_owned(
        output,
        format!("> FLIP #{count:03} REVEALED · {side} {latest}"),
    );
    for _ in 2..rows {
        push(output, "");
    }
}

pub(super) fn render_hidden_roll_ledger(output: &mut Lines, rolls: &str, rows: usize) {
    let count = rolls.len();
    let Some(latest) = rolls.chars().last() else {
        push(output, "  Waiting for the first physical roll…");
        for _ in 1..rows {
            push(output, "");
        }
        return;
    };
    if count == 1 {
        push(output, "  No prior rolls to conceal");
    } else {
        push(output, &format!("  Rolls #001–#{:03} concealed", count - 1));
    }
    push_owned(
        output,
        format!("> ROLL #{count:03} REVEALED · FACE {latest}"),
    );
    for _ in 2..rows {
        push(output, "");
    }
}

pub(super) fn render_roll_ledger(
    output: &mut Lines,
    ledger: &[String],
    requested: usize,
    rows: usize,
) {
    if ledger.is_empty() {
        push(output, "  Waiting for the first physical roll…");
        for _ in 1..rows {
            push(output, "");
        }
        return;
    }
    let max_start = ledger.len().saturating_sub(rows.saturating_sub(1));
    let start = max_start.saturating_sub(requested.min(max_start));
    let has_above = start > 0;
    let mut visible = rows.saturating_sub(usize::from(has_above));
    let has_below = start + visible < ledger.len();
    visible = visible.saturating_sub(usize::from(has_below));
    let end = (start + visible).min(ledger.len());
    if has_above {
        push(output, "  ↑ earlier rolls · Page Up");
    }
    for row in &ledger[start..end] {
        push(output, row);
    }
    if has_below {
        push(output, "  ↓ later rolls · Page Down");
    }
    let used = usize::from(has_above) + end - start + usize::from(has_below);
    for _ in used..rows {
        push(output, "");
    }
}

pub(super) fn progress(count: usize, required: usize, width: usize) -> String {
    let bar_width = width.saturating_sub(22).clamp(16, 42);
    let filled = (count.min(required) * bar_width)
        .checked_div(required)
        .unwrap_or(0);
    let prefix = if count >= required { "✓ " } else { "" };
    format!(
        "{prefix}[{}{}]  {count:>3} / {required}",
        "█".repeat(filled),
        "░".repeat(bar_width - filled)
    )
}

pub(super) fn roll_rows(rolls: &str) -> Lines {
    let digits_per_row = 25;
    let chunks = rolls.as_bytes().chunks(digits_per_row).collect::<Vec<_>>();
    let last = chunks.len().saturating_sub(1);
    let mut output = Lines::new(Vec::with_capacity(chunks.len()));
    for (row, chunk) in chunks.into_iter().enumerate() {
        let start = row * digits_per_row + 1;
        let end = start + chunk.len() - 1;
        let mut grouped = zeroize::Zeroizing::new(String::with_capacity(
            chunk.len().saturating_add(chunk.len() / 5),
        ));
        for (index, group) in chunk.chunks(5).enumerate() {
            if index > 0 {
                grouped.push(' ');
            }
            grouped.push_str(core::str::from_utf8(group).expect("roll digits are ASCII"));
        }
        let line = if row == last {
            format!("> #{start:03}–{end:03}  {}▌", grouped.as_str())
        } else {
            format!("  #{start:03}–{end:03}  {}", grouped.as_str())
        };
        push_owned(&mut output, line);
    }
    output
}
