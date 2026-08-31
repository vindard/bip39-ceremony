//! Group-compare rendering: the roll-collection and results screens for the
//! "one entropy set, many wallets" flow. Reads the report from the
//! [`GroupSession`] application service and the view state (screen, reveal,
//! help) from the app. Collection shows the live capture cursor and the
//! checkpoint progress bars; results lay out one section per entropy set, each
//! listing the protocols that complete at that roll count with accepted seeds
//! behind a reveal gate.

use zeroize::Zeroizing;

use crate::application::GroupSession;
use crate::domain::group::{EntropySet, GROUP_PROTOCOLS, GroupReport, GroupStatus};
use crate::domain::protocol::ConversionProtocol;

use crate::ui::app::GroupScreen;

use super::inspection::render_derivation_projection;
use super::protocol::render_protocol_explanation;
use super::rolls::progress;
use super::{
    App, Lines, clip, lines, push, push_output, push_owned, push_wrapped, render_card, title_line,
};

pub(super) fn compose_group_workspace(
    app: &App,
    session: &GroupSession,
    width: usize,
    height: usize,
) -> Zeroizing<String> {
    let mut output = Zeroizing::new(String::with_capacity(
        width.saturating_mul(height).saturating_add(height),
    ));
    let separator = "─".repeat(width);
    let corner = if app.group_derivation().is_some() {
        "● SECRET EXPOSED"
    } else if app.group_help() || app.group_details().is_some() {
        "○ NO SECRET YET"
    } else {
        status_corner(app, session)
    };
    push_output(&mut output, &clip(&title_line(width, corner), width));
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&stage_line(app), width));
    push_output(&mut output, "");

    let notice_lines = usize::from(app.message().is_some());
    let rows = height.saturating_sub(8 + notice_lines);
    let body = group_body(app, session, width.saturating_sub(4));
    render_card(
        &mut output,
        card_title(app),
        &body,
        rows,
        app.roll_scroll(),
        true,
        width,
    );

    if let Some(message) = app.message() {
        push_output(&mut output, &format!("! {message}"));
    }
    push_output(&mut output, &separator);
    push_output(&mut output, &clip(&footer(app), width));
    output
}

pub(super) fn group_body(app: &App, session: &GroupSession, width: usize) -> Lines {
    if let Some(index) = app.group_derivation() {
        return group_derivation_body(session, app.group_viewing(), index, width);
    }
    if let Some(index) = app.group_details() {
        return group_details_body(session, index, width);
    }
    if app.group_help() {
        return group_help_body();
    }
    match app.group_screen() {
        GroupScreen::Rolls => group_rolls(session, width),
        GroupScreen::Results => group_results(app, session, width),
    }
}

/// The derivation overlay: the same numbered BIP-39 stages a single-protocol
/// ceremony shows under `[d]`, stepping through every accepted seed in the
/// browsed capture. All values are secret.
fn group_derivation_body(
    session: &GroupSession,
    viewing: usize,
    index: usize,
    width: usize,
) -> Lines {
    let derivations = session.derivations(viewing);
    let Some(total) = (!derivations.is_empty()).then_some(derivations.len()) else {
        return lines(&[
            "GROUP COMPARE · DERIVATION",
            "",
            "No protocol accepted this entropy set, so there is nothing to",
            "derive. Press [d/Esc] to go back.",
        ]);
    };
    let index = index.min(total - 1);
    let entry = &derivations[index];
    let mut output = lines(&[
        &format!("GROUP COMPARE · DERIVATION · {} OF {total}", index + 1),
        &format!(
            "◆ ENTROPY SET · {} ROLLS · {}",
            entry.set_rolls,
            protocol_label(entry.protocol)
        ),
        "",
    ]);
    render_derivation_projection(&mut output, &entry.projection, width);
    push(&mut output, "");
    push(
        &mut output,
        "[←/→] step through accepted seeds · [d/Esc] back to compare",
    );
    output
}

/// The protocol-explanation overlay: the same document the main flow's `[e]`
/// details card shows, framed with a selector across the group protocols.
fn group_details_body(session: &GroupSession, index: usize, width: usize) -> Lines {
    let protocol = GROUP_PROTOCOLS[index];
    let mut output = lines(&[
        &format!(
            "GROUP COMPARE · PROTOCOL DETAILS · {} OF {}",
            index + 1,
            GROUP_PROTOCOLS.len()
        ),
        &details_selector(index),
        "",
    ]);
    output.extend(
        render_protocol_explanation(protocol, session.target(), width)
            .iter()
            .cloned(),
    );
    push(&mut output, "");
    push(
        &mut output,
        "[←/→] step through each protocol · [e/Esc] back to compare",
    );
    output
}

fn details_selector(selected: usize) -> String {
    format!(
        "▶ {} · ←/→ change protocol",
        protocol_label(GROUP_PROTOCOLS[selected])
    )
}

fn group_help_body() -> Lines {
    lines(&[
        "GROUP COMPARE · HOW IT WORKS",
        "",
        "You roll ONE physical D6 tape. Every dice protocol is replayed",
        "against that same entropy, so you compare the seeds each wallet's",
        "method produces from identical rolls.",
        "",
        "A protocol appears in a set only if it consumes EXACTLY those",
        "rolls. Because protocols finish at different lengths, results group",
        "into ENTROPY SETS at roll-count checkpoints — the current tape, plus",
        "each length where a fixed-count protocol completes (99, 100, and",
        "140 at 24 words), plus where either packed method reaches its target.",
        "COLDCARD, Keystone, and Ian Coleman fixed use every roll, so they",
        "appear in each set at or above their minimum with a different seed.",
        "",
        "IN A SET, A PROTOCOL IS",
        "  ✓ accepted   a seed came out — press [r] to reveal it.",
        "  ✗ rejected   these rolls fail a content check; rolling cannot",
        "               fix it, so the remedy is [n] a fresh set.",
        "",
        "[?/Esc] close this help.",
    ])
}

fn status_corner(app: &App, session: &GroupSession) -> &'static str {
    match app.group_screen() {
        GroupScreen::Rolls => {
            if session.roll_progress().recorded == 0 {
                "○ NO SECRET YET"
            } else {
                "● SECRET EXPOSED"
            }
        }
        GroupScreen::Results => {
            if app.group_revealed() {
                "● SECRET EXPOSED"
            } else {
                "● SECRET CONCEALED"
            }
        }
    }
}

fn stage_line(app: &App) -> String {
    let screen = if app.group_derivation().is_some() {
        "DERIVATION"
    } else if app.group_details().is_some() {
        "PROTOCOL DETAILS"
    } else if app.group_help() {
        "HELP"
    } else {
        match app.group_screen() {
            GroupScreen::Rolls => "COLLECT ROLLS",
            GroupScreen::Results => "COMPARE RESULTS",
        }
    };
    format!("◆ GROUP COMPARE  ›  {screen}")
}

const fn card_title(app: &App) -> &'static str {
    if app.group_derivation().is_some() {
        return "GROUP COMPARE · DERIVATION · ALL VALUES SECRET · FOCUS";
    }
    if app.group_details().is_some() {
        return "GROUP COMPARE · DETAILS · FOCUS";
    }
    if app.group_help() {
        return "GROUP COMPARE · HELP · FOCUS";
    }
    match app.group_screen() {
        GroupScreen::Rolls => "GROUP COMPARE · COLLECT · FOCUS",
        GroupScreen::Results => "GROUP COMPARE · RESULTS · FOCUS",
    }
}

fn footer(app: &App) -> String {
    if app.group_derivation().is_some() {
        return "[←/→] seed  [↑/↓] scroll  [d/Esc] back  [q] cancel".to_owned();
    }
    if app.group_details().is_some() {
        return "[←/→] protocol  [↑/↓] scroll  [e/Esc] back  [q] cancel".to_owned();
    }
    if app.group_help() {
        return "[?/Esc] close help  [q] cancel".to_owned();
    }
    match app.group_screen() {
        GroupScreen::Rolls => {
            "[1–6] roll  [⌫] undo  [Enter] compare  [e] details  [?] help  [q] cancel".to_owned()
        }
        GroupScreen::Results => {
            "[r] reveal  [d] derive  [e] details  [c] more  [n] fresh  [←/→] sets  [q] cancel"
                .to_owned()
        }
    }
}

fn group_rolls(session: &GroupSession, width: usize) -> Lines {
    let state = session.roll_progress();

    // The current (newest) capture is always the last one being built.
    let count = session.set_count();
    let cursor = match state.latest_face {
        Some(face) => format!(
            "LATEST RECORDED · #{:03} · FACE {face}  |  NEXT · #{:03}",
            state.recorded,
            state.recorded + 1
        ),
        None => "NEXT PHYSICAL ROLL · #001".to_owned(),
    };
    let mut output = lines(&[
        "PHYSICAL D6 CAPTURE · ROLLS ARE SECRET",
        &format!("GROUP COMPARE · CAPTURE {count} OF {count}"),
        &cursor,
        "Roll once, observe, then press: [1] [2] [3] [4] [5] [6]",
        "",
        "One physical tape, replayed across every dice protocol so you can",
        "compare the seeds that fall out of the very same entropy.",
        "",
        "CHECKPOINTS · a protocol joins a set once it uses exactly the tape",
        "  STRICT · Exact · COLDCARD · Keystone · Ian Coleman fixed",
    ]);
    let bar_width = width.saturating_sub(2);
    push_owned(
        &mut output,
        format!(
            "  {}",
            progress(state.recorded, state.strict_target, bar_width)
        ),
    );
    push(&mut output, "  FULL · adds Word-by-word Exact");
    push_owned(
        &mut output,
        format!(
            "  {}",
            progress(state.recorded, state.full_target, bar_width)
        ),
    );
    push(
        &mut output,
        "  OTHER · bitcoinlib at 99 · packed methods depend on the faces",
    );
    push(&mut output, "");
    push(
        &mut output,
        "! The app validates 1–6; it cannot match the key to the die.",
    );
    push(
        &mut output,
        "Backspace corrects the latest entry—never replace a valid face.",
    );
    push(
        &mut output,
        "[Enter] compares every protocol on this entropy set.",
    );
    push(
        &mut output,
        "[e] explains how each protocol converts the tape.",
    );
    output
}

fn group_results(app: &App, session: &GroupSession, width: usize) -> Lines {
    let viewing = app.group_viewing();
    let report = session.comparison_at(viewing);
    let recorded = report.sets().first().map_or(0, |set| set.rolls);
    let revealed = app.group_revealed();

    let mut output = lines(&[
        &format!(
            "GROUP COMPARE · CAPTURE {} OF {} · {recorded} ROLLS",
            viewing + 1,
            session.set_count()
        ),
        "One tape, every protocol — each set is a roll-count checkpoint where",
        "a protocol consumes exactly those rolls and nothing more.",
    ]);

    for set in report.sets() {
        push(&mut output, "");
        render_set(&mut output, set, revealed, width);
    }

    if has_rejection(&report) {
        push(&mut output, "");
        push(
            &mut output,
            "✗ A rejected result shares this entropy set — rolling more cannot",
        );
        push(
            &mut output,
            "  repair it. Press [n] for a fresh set to clear it.",
        );
    }
    if !revealed {
        push(&mut output, "");
        push(&mut output, "> [r] reveals the accepted seeds.");
    }
    push(&mut output, "");
    push(
        &mut output,
        "[e] explains a protocol · [d] derives an accepted seed (entropy, words).",
    );
    output
}

fn render_set(output: &mut Lines, set: &EntropySet, revealed: bool, width: usize) {
    push_owned(output, format!("◆ ENTROPY SET · {} ROLLS", set.rolls));
    if set.protocols.is_empty() {
        push(
            &mut *output,
            "  ○ collecting — no protocol completes at this length yet",
        );
        return;
    }
    for (protocol, status) in &set.protocols {
        push_owned(
            output,
            format!(
                "  {} {:<20} {}",
                status_glyph(status),
                protocol_label(*protocol),
                status_reason(status)
            ),
        );
        if revealed && let Some(calculation) = status.calculation() {
            // Zeroize the joined seed; the wrapped copies land in the Zeroizing
            // `Lines`, but this intermediate would otherwise leak to freed heap.
            let seed = Zeroizing::new(calculation.mnemonic().words().join(" "));
            push_wrapped(output, "       seed · ", &seed, width);
        }
    }
}

const fn status_glyph(status: &GroupStatus) -> char {
    match status {
        GroupStatus::Accepted(_) => '✓',
        // Filtered out of a set, but keep the match exhaustive.
        GroupStatus::Incomplete { .. } => '…',
        GroupStatus::Unsupported => '—',
        _ => '✗',
    }
}

fn status_reason(status: &GroupStatus) -> &'static str {
    match status {
        GroupStatus::Accepted(_) => "accepted",
        GroupStatus::InvalidOutOfRange => "rejected · value out of range",
        GroupStatus::InvalidDistribution => "rejected · a face exceeds 30%",
        GroupStatus::InvalidRejected => "rejected · a candidate was rejected",
        GroupStatus::Incomplete { .. } => "incomplete",
        GroupStatus::InvalidSurplus { .. } => "rejected · surplus rolls",
        GroupStatus::Unsupported => "unsupported",
    }
}

fn has_rejection(report: &GroupReport) -> bool {
    report
        .sets()
        .iter()
        .flat_map(|set| set.protocols.iter())
        .any(|(_, status)| status.is_rejected())
}

fn protocol_label(protocol: ConversionProtocol) -> &'static str {
    match protocol {
        ConversionProtocol::ExactV1 => "Exact",
        ConversionProtocol::ColdcardV1 => "COLDCARD",
        ConversionProtocol::KeystoneLegacyV1 => "Keystone legacy",
        ConversionProtocol::WordExactV1 => "Word-by-word Exact",
        ConversionProtocol::BitcoinLibBase6V1 => "bitcoinlib base-6",
        ConversionProtocol::BlueWalletBitPackV1 => "BlueWallet bit packing",
        ConversionProtocol::IanColemanDiceV1 => "Ian Coleman fixed",
        ConversionProtocol::IanColemanRawV1 => "Ian Coleman raw",
        _ => protocol.id(),
    }
}
