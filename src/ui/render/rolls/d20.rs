use std::fmt::Write;

use zeroize::Zeroizing;

use crate::domain::d20::D20RollSequence;

use super::super::{App, Lines, lines, push, push_owned, push_wrapped};

pub(super) fn render_d20_entry(app: &App, width: usize) -> Lines {
    let state = app.ceremony().state();
    let rolls = state.d20();
    let minimum = state.required_rolls().unwrap_or(0);
    let cursor = Zeroizing::new(cursor(rolls));
    let mut output = lines(&[
        "PHYSICAL D20 CAPTURE · ROLLS ARE SECRET",
        "[e] PROTOCOL DETAILS · Krux D20 · krux-d20-v1",
        "CONVERSION · hyphen-separated decimal D20 faces → SHA-256 → BIP-39 entropy",
        &cursor,
        "Roll D20, then press [1–9] or uppercase [A–K] for faces 10–20",
        "",
        &super::progress(rolls.len().min(minimum), minimum, width),
        &super::roll_count_status(rolls.len(), state.capture_assessment()),
        "",
        if app.rolls_hidden() {
            "D20 LEDGER · SECRET · PRIOR ROLLS HIDDEN"
        } else {
            "D20 LEDGER · SECRET · ALL ROLLS VISIBLE"
        },
    ]);

    render_ledger(&mut output, rolls, app.rolls_hidden(), width);
    push(&mut output, "");
    if state.can_confirm_rolls() {
        super::render_roll_completion(&mut output, &state, width);
    } else {
        push_wrapped(
            &mut output,
            "",
            "Roll once and record every valid physical D20 result exactly once.",
            width,
        );
        push(
            &mut output,
            "! The app validates 1–20; it cannot match the key to the die.",
        );
        push(
            &mut output,
            "Backspace corrects only the latest entry—never reroll a valid face.",
        );
    }
    output
}

fn cursor(rolls: &D20RollSequence) -> String {
    let next = rolls.len() + 1;
    rolls.faces().last().map_or_else(
        || format!("NEXT PHYSICAL D20 ROLL · #{next:03}"),
        |face| {
            format!(
                "LATEST · #{:03} · D20 FACE {}  |  NEXT · #{next:03}",
                rolls.len(),
                face.get()
            )
        },
    )
}

fn render_ledger(output: &mut Lines, rolls: &D20RollSequence, hidden: bool, width: usize) {
    if hidden {
        if let Some(face) = rolls.faces().last() {
            push_owned(
                output,
                format!("  #{:03} · D20={}", rolls.len(), face.get()),
            );
            if rolls.len() > 1 {
                push(
                    output,
                    &format!("  {} prior roll(s) concealed", rolls.len() - 1),
                );
            }
        } else {
            push(output, "  No rolls recorded");
        }
        return;
    }

    let mut ledger = Zeroizing::new(String::new());
    for (index, face) in rolls.faces().iter().enumerate() {
        if index > 0 {
            ledger.push_str("  ");
        }
        write!(ledger, "#{:03}:D20={}", index + 1, face.get())
            .expect("writing to String cannot fail");
    }
    let available = width.saturating_sub(4).max(16);
    for chunk in ledger.as_bytes().chunks(available) {
        let text = core::str::from_utf8(chunk).expect("D20 ledger is ASCII");
        push_owned(output, format!("  {text}"));
    }
}
