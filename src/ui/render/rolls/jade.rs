use std::fmt::Write;

use zeroize::Zeroizing;

use crate::domain::{
    jade::JadeCapture,
    protocol::{JadeDieKind, JadeStage, jade_progress},
};

use super::super::{App, Lines, lines, push, push_owned, push_wrapped};

pub(super) fn render_jade_entry(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    let target = state.target().expect("capture requires a target");
    let capture = state.jade();
    let progress = jade_progress(target, capture);
    let cursor = Zeroizing::new(cursor(capture, progress.stage()));
    let instruction = instruction(progress.stage());
    let mut output = lines(&[
        "PHYSICAL MIXED-DICE CAPTURE · ROLLS ARE SECRET",
        "[e] PROTOCOL DETAILS · Jade direct words · jade-direct-v1",
        "CONVERSION · D16/D16/D8 direct indices + physical entropy tail → BIP-39",
        &cursor,
        &instruction,
        "",
        &super::progress(capture.len(), progress.required(), width),
        &format!(
            "{} recorded · {} remaining",
            capture.len(),
            progress.required().saturating_sub(capture.len())
        ),
        "",
        if app.rolls_hidden() {
            "MIXED-DICE LEDGER · SECRET · PRIOR ROLLS HIDDEN"
        } else {
            "MIXED-DICE LEDGER · SECRET · ALL ROLLS VISIBLE"
        },
    ]);

    render_ledger(&mut output, capture, app.rolls_hidden(), width);
    push(&mut output, "");
    render_stage(
        &mut output,
        progress.stage(),
        progress.completed_words(),
        progress.required_words(),
    );
    if state.can_confirm_rolls() {
        super::render_roll_completion(&mut output, &state, width);
    } else {
        push_wrapped(
            &mut output,
            "",
            "Roll the requested die once and record every valid physical result exactly once.",
            width,
        );
        push(
            &mut output,
            "! The app validates the range; it cannot observe the physical die.",
        );
        push(
            &mut output,
            "Backspace corrects only the latest entry—never reroll a valid face.",
        );
    }
    output
}

fn cursor(capture: &JadeCapture, stage: JadeStage) -> String {
    let next = capture.len() + 1;
    capture.observations().last().map_or_else(
        || format!("NEXT PHYSICAL ROLL · #{next:03} · {}", stage_die(stage)),
        |observation| {
            format!(
                "LATEST · #{:03} · D{} FACE {}  |  NEXT · #{next:03} · {}",
                capture.len(),
                observation.sides(),
                observation.face(),
                stage_die(stage)
            )
        },
    )
}

fn instruction(stage: JadeStage) -> String {
    match stage {
        JadeStage::DirectWord {
            die: JadeDieKind::D16,
            ..
        }
        | JadeStage::EntropyTail {
            die: JadeDieKind::D16,
        } => "Roll D16, then press [1–9] or uppercase [A–G] for faces 10–16".to_owned(),
        JadeStage::DirectWord {
            die: JadeDieKind::D8,
            ..
        }
        | JadeStage::EntropyTail {
            die: JadeDieKind::D8,
        } => "Roll D8, then press its face [1–8]".to_owned(),
        JadeStage::Complete => "Capture complete · press Enter to generate".to_owned(),
    }
}

fn stage_die(stage: JadeStage) -> &'static str {
    match stage {
        JadeStage::DirectWord {
            die: JadeDieKind::D16,
            ..
        }
        | JadeStage::EntropyTail {
            die: JadeDieKind::D16,
        } => "D16",
        JadeStage::DirectWord {
            die: JadeDieKind::D8,
            ..
        }
        | JadeStage::EntropyTail {
            die: JadeDieKind::D8,
        } => "D8",
        JadeStage::Complete => "COMPLETE",
    }
}

fn render_stage(output: &mut Lines, stage: JadeStage, completed: usize, required: usize) {
    match stage {
        JadeStage::DirectWord {
            position,
            observation_in_word,
            ..
        } => push(
            output,
            &format!(
                "DIRECT WORD {position:02} OF {required:02} · DIE {observation_in_word} OF 3 · {completed} COMPLETE"
            ),
        ),
        JadeStage::EntropyTail { die } => {
            push(
                output,
                &format!("✓ {required} DIRECT WORD POSITIONS COMPLETE"),
            );
            push(
                output,
                &format!(
                    "FINAL ENTROPY TAIL · NEXT {} · checksum is calculated afterward",
                    match die {
                        JadeDieKind::D16 => "D16",
                        JadeDieKind::D8 => "D8",
                    }
                ),
            );
        }
        JadeStage::Complete => {
            push(
                output,
                &format!("✓ {required} DIRECT WORD POSITIONS + ENTROPY TAIL COMPLETE"),
            );
        }
    }
}

fn render_ledger(output: &mut Lines, capture: &JadeCapture, hidden: bool, width: usize) {
    if hidden {
        if let Some(observation) = capture.observations().last() {
            push(
                output,
                &format!(
                    "  #{:03} · D{}={}",
                    capture.len(),
                    observation.sides(),
                    observation.face()
                ),
            );
            if capture.len() > 1 {
                push(
                    output,
                    &format!("  {} prior roll(s) concealed", capture.len() - 1),
                );
            }
        } else {
            push(output, "  No rolls recorded");
        }
        return;
    }

    let mut ledger = Zeroizing::new(String::new());
    for (index, observation) in capture.observations().iter().enumerate() {
        if index > 0 {
            ledger.push_str("  ");
        }
        write!(
            ledger,
            "#{:03}:D{}={}",
            index + 1,
            observation.sides(),
            observation.face()
        )
        .expect("writing to String cannot fail");
    }
    let available = width.saturating_sub(4).max(16);
    for chunk in ledger.as_bytes().chunks(available) {
        let text = core::str::from_utf8(chunk).expect("mixed-dice ledger is ASCII");
        push_owned(output, format!("  {text}"));
    }
}
