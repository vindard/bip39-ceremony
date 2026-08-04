use std::fmt::Write;

use zeroize::{Zeroize, Zeroizing};

use crate::domain::{
    bitbox::{BitBoxCapture, BitBoxObservation},
    protocol::{BitBoxProgress, BitBoxStage, bitbox_progress},
};

use super::super::{App, Lines, lines, push, push_owned, push_wrapped};

pub(super) fn render_bitbox_entry(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    let target = state.target().expect("capture requires a target");
    let capture = state.bitbox();
    let progress = bitbox_progress(target, capture);
    let cursor = Zeroizing::new(cursor(capture, progress.stage()));
    let instruction = instruction(progress.stage());
    let mut output = lines(&[
        "PHYSICAL D6 + COIN CAPTURE · OUTCOMES ARE SECRET",
        "[e] PROTOCOL DETAILS · BitBox02 Diceware · bitbox02-direct-v1",
        "CONVERSION · local D6 rejection + coin-selected direct indices and entropy tail",
        &cursor,
        &instruction,
        "",
        &super::progress(
            capture.len().saturating_sub(progress.rejected_faces()),
            progress.minimum(),
            width,
        ),
        &format!(
            "{} recorded · {}/{} direct positions · {} rejected D6",
            progress.recorded(),
            progress.completed_words(),
            progress.required_words(),
            progress.rejected_faces()
        ),
        "",
        if app.rolls_hidden() {
            "D6 + COIN LEDGER · SECRET · PRIOR OUTCOMES HIDDEN"
        } else {
            "D6 + COIN LEDGER · SECRET · ALL OUTCOMES VISIBLE"
        },
    ]);

    render_ledger(&mut output, capture, app.rolls_hidden(), width);
    push(&mut output, "");
    render_stage(&mut output, progress);
    if state.can_confirm_rolls() {
        super::render_roll_completion(&mut output, &state, width);
    } else {
        push_wrapped(
            &mut output,
            "",
            "Record the requested physical outcome. D6 faces 5 and 6 are kept as rejected attempts, then only that digit is rerolled.",
            width,
        );
        push(
            &mut output,
            "! The app validates entries; it cannot observe the physical die or coin.",
        );
        push(
            &mut output,
            "Backspace corrects only the latest entry—never replace a valid outcome.",
        );
    }
    output
}

fn cursor(capture: &BitBoxCapture, stage: BitBoxStage) -> String {
    let next = capture.len() + 1;
    capture.observations().last().map_or_else(
        || format!("NEXT PHYSICAL OUTCOME · #{next:03} · {}", stage_kind(stage)),
        |observation| {
            let latest = Zeroizing::new(match observation {
                BitBoxObservation::D6(face) if face.get() > 4 => {
                    format!("D6 FACE {} · REJECTED", face.get())
                }
                BitBoxObservation::D6(face) => format!("D6 FACE {}", face.get()),
                BitBoxObservation::Coin(flip) => format!(
                    "COIN {} {}",
                    if flip.get() == 0 { "TAILS" } else { "HEADS" },
                    flip.get()
                ),
            });
            format!(
                "LATEST · #{:03} · {}  |  NEXT · #{next:03} · {}",
                capture.len(),
                latest.as_str(),
                stage_kind(stage)
            )
        },
    )
}

fn instruction(stage: BitBoxStage) -> String {
    match stage {
        BitBoxStage::DirectWordD6 { .. } => {
            "Roll D6, then press [1–6] · 5/6 are recorded and locally retried".to_owned()
        }
        BitBoxStage::DirectWordCoin { .. } | BitBoxStage::EntropyTail { .. } => {
            "Flip coin, then press [0] tails or [1] heads".to_owned()
        }
        BitBoxStage::Complete => "Capture complete · press Enter to generate".to_owned(),
    }
}

fn stage_kind(stage: BitBoxStage) -> &'static str {
    match stage {
        BitBoxStage::DirectWordD6 { .. } => "D6",
        BitBoxStage::DirectWordCoin { .. } | BitBoxStage::EntropyTail { .. } => "COIN",
        BitBoxStage::Complete => "COMPLETE",
    }
}

fn render_stage(output: &mut Lines, progress: BitBoxProgress) {
    match progress.stage() {
        BitBoxStage::DirectWordD6 {
            position,
            accepted_faces,
        } => push(
            output,
            &format!(
                "DIRECT WORD {position:02} OF {:02} · ACCEPTED D6 {accepted_faces}/5",
                progress.required_words()
            ),
        ),
        BitBoxStage::DirectWordCoin { position } => {
            push(
                output,
                &format!(
                    "DIRECT WORD {position:02} OF {:02} · FIVE D6 DIGITS ACCEPTED",
                    progress.required_words()
                ),
            );
            push(output, "COIN SELECTOR · heads→0 · tails→1 in lookup order");
        }
        BitBoxStage::EntropyTail { recorded, required } => {
            push(
                output,
                &format!(
                    "✓ {} DIRECT WORD POSITIONS COMPLETE",
                    progress.required_words()
                ),
            );
            push(
                output,
                &format!(
                    "FINAL ENTROPY TAIL · COIN {}/{} · heads→0 · tails→1",
                    recorded + 1,
                    required
                ),
            );
        }
        BitBoxStage::Complete => push(
            output,
            &format!(
                "✓ {} DIRECT WORD POSITIONS + ENTROPY TAIL COMPLETE",
                progress.required_words()
            ),
        ),
    }
}

fn render_ledger(output: &mut Lines, capture: &BitBoxCapture, hidden: bool, width: usize) {
    if hidden {
        if let Some(observation) = capture.observations().last() {
            let latest = label(*observation);
            push_owned(
                output,
                format!("  #{:03} · {}", capture.len(), latest.as_str()),
            );
            if capture.len() > 1 {
                push(
                    output,
                    &format!("  {} prior outcome(s) concealed", capture.len() - 1),
                );
            }
        } else {
            push(output, "  No outcomes recorded");
        }
        return;
    }

    let mut ledger = Zeroizing::new(String::new());
    for (index, observation) in capture.observations().iter().enumerate() {
        if index > 0 {
            ledger.push_str("  ");
        }
        let observation = label(*observation);
        write!(ledger, "#{:03}:{}", index + 1, observation.as_str())
            .expect("writing to String cannot fail");
    }
    let available = width.saturating_sub(4).max(16);
    for chunk in ledger.as_bytes().chunks(available) {
        let text = core::str::from_utf8(chunk).expect("BitBox ledger is ASCII");
        push_owned(output, format!("  {text}"));
    }
}

fn label(mut observation: BitBoxObservation) -> Zeroizing<String> {
    let label = Zeroizing::new(match observation {
        BitBoxObservation::D6(face) if face.get() > 4 => format!("D6={}X", face.get()),
        BitBoxObservation::D6(face) => format!("D6={}", face.get()),
        BitBoxObservation::Coin(flip) => format!(
            "C={}{}",
            flip.get(),
            if flip.get() == 0 { "T" } else { "H" }
        ),
    });
    observation.zeroize();
    label
}
