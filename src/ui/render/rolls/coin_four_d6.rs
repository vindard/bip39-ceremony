use std::fmt::Write;

use zeroize::{Zeroize, Zeroizing};

use crate::domain::{
    coin_four_d6::{CoinFourD6Capture, CoinFourD6Observation},
    protocol::{CoinFourD6Progress, CoinFourD6Stage, coin_four_d6_progress},
};

use super::super::{App, Lines, lines, push, push_owned, push_wrapped};

pub(super) fn render_coin_four_d6_entry(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    let target = state.target().expect("capture requires a target");
    let capture = state.coin_four_d6();
    let progress = coin_four_d6_progress(target, capture);
    let cursor = Zeroizing::new(cursor(capture, progress.stage()));
    let instruction = instruction(progress.stage());
    let mut output = lines(&[
        "PHYSICAL COIN + FOUR-D6 CAPTURE · OUTCOMES ARE SECRET",
        "[e] PROTOCOL DETAILS · Coin + four-D6 direct words · coin-four-d6-direct-v1",
        "CONVERSION · whole-candidate rejection + direct indices + calculated checksum",
        &cursor,
        &instruction,
        "",
        &super::progress(progress.completed_candidates(), 12, width),
        &format!(
            "{} recorded · {}/12 accepted candidates · {} rejected candidates",
            progress.recorded(),
            progress.completed_candidates(),
            progress.rejected_candidates()
        ),
        "",
        if app.rolls_hidden() {
            "COIN + D6 LEDGER · SECRET · PRIOR OUTCOMES HIDDEN"
        } else {
            "COIN + D6 LEDGER · SECRET · ALL OUTCOMES VISIBLE"
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
            "A rejected tails tuple after 4,3,6,2 discards this coin and all four rolls. Start the next candidate with a fresh coin flip.",
            width,
        );
        push(
            &mut output,
            "! The app validates entries; it cannot observe the physical dice or coin.",
        );
        push(
            &mut output,
            "Backspace corrects only the latest entry—never replace a valid outcome.",
        );
    }
    output
}

fn cursor(capture: &CoinFourD6Capture, stage: CoinFourD6Stage) -> String {
    if stage == CoinFourD6Stage::Complete {
        return format!("LATEST · #{:03} · CAPTURE COMPLETE", capture.len());
    }
    let next = capture.len() + 1;
    capture.observations().last().map_or_else(
        || format!("NEXT PHYSICAL OUTCOME · #{next:03} · {}", stage_kind(stage)),
        |observation| {
            let latest = label(*observation);
            format!(
                "LATEST · #{:03} · {}  |  NEXT · #{next:03} · {}",
                capture.len(),
                latest.as_str(),
                stage_kind(stage)
            )
        },
    )
}

fn instruction(stage: CoinFourD6Stage) -> String {
    match stage {
        CoinFourD6Stage::Coin { .. } => {
            "Flip a fresh coin, then press [0] tails or [1] heads".to_owned()
        }
        CoinFourD6Stage::D6 { .. } => "Roll the next ordered D6, then press [1–6]".to_owned(),
        CoinFourD6Stage::Complete => "Capture complete · press Enter to generate".to_owned(),
    }
}

fn stage_kind(stage: CoinFourD6Stage) -> &'static str {
    match stage {
        CoinFourD6Stage::Coin { .. } => "COIN",
        CoinFourD6Stage::D6 { .. } => "D6",
        CoinFourD6Stage::Complete => "COMPLETE",
    }
}

fn render_stage(output: &mut Lines, progress: CoinFourD6Progress) {
    match progress.stage() {
        CoinFourD6Stage::Coin { position } => push(
            output,
            &format!("TABLE CANDIDATE {position:02} OF 12 · FRESH COIN REQUIRED"),
        ),
        CoinFourD6Stage::D6 { position, recorded } => push(
            output,
            &format!(
                "TABLE CANDIDATE {position:02} OF 12 · COIN RECORDED · D6 {}/4 NEXT",
                recorded + 1
            ),
        ),
        CoinFourD6Stage::Complete => push(
            output,
            "✓ 12 TABLE CANDIDATES COMPLETE · FINAL LOW NIBBLE BECOMES CHECKSUM",
        ),
    }
}

fn render_ledger(output: &mut Lines, capture: &CoinFourD6Capture, hidden: bool, width: usize) {
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
        let text = core::str::from_utf8(chunk).expect("coin-four-D6 ledger is ASCII");
        push_owned(output, format!("  {text}"));
    }
}

fn label(mut observation: CoinFourD6Observation) -> Zeroizing<String> {
    let label = Zeroizing::new(match observation {
        CoinFourD6Observation::Coin(flip) => format!(
            "C={}{}",
            flip.get(),
            if flip.get() == 0 { "T" } else { "H" }
        ),
        CoinFourD6Observation::D6(face) => format!("D6={}", face.get()),
    });
    observation.zeroize();
    label
}
