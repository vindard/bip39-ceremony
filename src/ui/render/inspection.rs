use std::fmt::Write;

use zeroize::Zeroizing;

use crate::{
    application::BuildCapabilities,
    domain::{
        bip39::EntropyTarget,
        ceremony::Phase,
        inspection::{InspectionSnapshot, timeline},
        protocol::ConversionProtocol,
    },
    presentation::{
        derivation_guidance, phase_guidance, protocol_choices, protocol_menu_explanation,
        safety_content, target_choices, trust_boundary,
    },
};

use super::{
    App, InspectorView, Lines, content_choices, lines,
    protocol::{render_document, render_protocol_explanation},
    push, push_wrapped,
    rolls::progress,
};

pub(super) fn render_inspector(app: &App, width: usize) -> Lines {
    match app.inspector().map(|inspector| inspector.view) {
        Some(InspectorView::Timeline) => {
            let mut output = Lines::new(Vec::new());
            render_timeline(&mut output, app);
            output
        }
        Some(InspectorView::Derivation) => {
            let mut output = Lines::new(Vec::new());
            render_derivation(&mut output, app, width);
            output
        }
        Some(InspectorView::ProtocolExplanation) => {
            let state = app.ceremony().state();
            let target = state.target().unwrap_or(EntropyTarget::Words12);
            if state.phase() == Phase::ChooseProtocol {
                let choice = app.selected_protocol_choice();
                if let Some(protocol) = choice.implemented_protocol(target) {
                    render_protocol_explanation(protocol, target, width)
                } else {
                    render_document(&protocol_menu_explanation(choice, target), width)
                }
            } else if let Some(protocol) = state.protocol() {
                render_protocol_explanation(protocol, target, width)
            } else {
                lines(&["Protocol explanation unavailable."])
            }
        }
        Some(InspectorView::Help) => {
            let mut output = Lines::new(Vec::new());
            render_help(&mut output, app.ceremony().state().phase(), width);
            output
        }
        Some(InspectorView::Snapshot) => render_projected_ceremony(app, width),
        None => lines(&["Inspection unavailable."]),
    }
}

pub(super) fn render_projected_ceremony(app: &App, width: usize) -> Lines {
    let Some(snapshot) = app.inspected_snapshot() else {
        return lines(&["Inspection unavailable."]);
    };
    projection(
        app,
        match snapshot.phase() {
            Phase::ChooseTarget => content_choices(
                "Choose mnemonic length",
                "Both are standard BIP-39; length does not fix device or backup exposure.",
                0,
                usize::MAX,
                &target_choices(),
            ),
            Phase::ChooseProtocol => content_choices(
                "Choose conversion protocol",
                "All produce standard BIP-39. Protocol only matters to reproduce from rolls.",
                1,
                usize::MAX,
                &protocol_choices(snapshot.target().unwrap_or(EntropyTarget::Words12)),
            ),
            Phase::Safety => render_historical_safety(),
            Phase::EnterRolls => render_snapshot_rolls(&snapshot, width),
            Phase::ReadyToGenerate => lines(&["Generating mnemonic…"]),
            Phase::ExactAttemptRejected => lines(&["Exact conversion rejected this attempt"]),
            Phase::Result => lines(&[
                "✓ MNEMONIC GENERATED",
                "◆ SECRET CONCEALED · derived values remain hidden",
            ]),
            Phase::Revealed => render_inspection_mnemonic_concealment(),
            Phase::Cancelled => lines(&["Ceremony cancelled"]),
        },
    )
}

fn render_inspection_mnemonic_concealment() -> Lines {
    lines(&[
        "○ RECOVERY WORDS CONCEALED",
        "",
        "The mnemonic is not drawn during inspection.",
        "This prevents the recovery phrase remaining visible behind details.",
        "",
        "[Tab/i] Return to the mnemonic workspace",
    ])
}

fn projection(app: &App, mut output: Lines) -> Lines {
    let label = if app
        .inspector()
        .is_some_and(|inspector| inspector.position == app.ceremony().events().len())
    {
        "LIVE CEREMONY PROJECTION · READ ONLY"
    } else {
        "HISTORICAL CEREMONY PROJECTION · READ ONLY"
    };
    output.insert(0, label.to_owned());
    output
}

fn render_snapshot_rolls(snapshot: &InspectionSnapshot, width: usize) -> Lines {
    let count = snapshot.roll_count();
    let required = snapshot
        .target()
        .zip(snapshot.protocol())
        .map_or(0, |(target, protocol)| {
            protocol.minimum_observations(target)
        });
    let (heading, concealed, empty, noun, boundary) = match snapshot.protocol() {
        Some(ConversionProtocol::SeedSignerCoinsV1) => (
            "PHYSICAL COIN SNAPSHOT",
            "◆ FLIP VALUES CONCEALED",
            "  No flips recorded at this position.",
            "outcomes",
            "Snapshots show ceremony structure, not secret flip values.",
        ),
        Some(ConversionProtocol::JadeDirectV1) => (
            "PHYSICAL MIXED-DICE SNAPSHOT",
            "◆ D16/D8 VALUES CONCEALED",
            "  No rolls recorded at this position.",
            "faces",
            "Snapshots show structure, not secret mixed-dice values.",
        ),
        Some(ConversionProtocol::BitBox02DirectV1) => (
            "PHYSICAL D6 + COIN SNAPSHOT",
            "◆ D6/COIN VALUES CONCEALED",
            "  No outcomes recorded at this position.",
            "outcomes",
            "Snapshots show structure, not secret D6 or coin values.",
        ),
        _ => (
            "PHYSICAL D6 SNAPSHOT",
            "◆ ROLL VALUES CONCEALED",
            "  No rolls recorded at this position.",
            "faces",
            "Snapshots show ceremony structure, not secret roll values.",
        ),
    };
    let mut output = Lines::new(Vec::new());
    push(&mut output, heading);
    push(&mut output, &progress(count, required, width));
    push(&mut output, concealed);
    if count == 0 {
        push(&mut output, empty);
    } else {
        push(
            &mut output,
            &format!("  Positions #001–#{count:03} recorded · {noun} not drawn"),
        );
    }
    push(&mut output, boundary);
    output
}

fn render_historical_safety() -> Lines {
    let mut output = lines(&[
        "SAFETY CHECKLIST · NOT YET ACKNOWLEDGED",
        "Checklist interaction is available only on the live state.",
        "",
    ]);
    for item in crate::application::SafetyAttestation::ALL {
        push(
            &mut output,
            &format!("  □ {}", safety_content(item).label()),
        );
    }
    push(&mut output, "");
    push(
        &mut output,
        "The tool cannot detect unfair dice or a compromised system.",
    );
    output
}

fn render_timeline(output: &mut Lines, app: &App) {
    push(output, "EVENT TIMELINE");
    push(output, "The selected event produces the inspected state.");
    let selected = app.inspector().map_or(0, |value| value.position);
    let entries = timeline(app.ceremony());
    if selected == 0 {
        push(output, ">   0  Ceremony started");
    }
    let start = selected.saturating_sub(5).max(1);
    let end = (start + 10).min(entries.len());
    if start > 1 {
        push(output, "    …  earlier events");
    }
    for entry in entries
        .iter()
        .filter(|entry| (start..=end).contains(&entry.position()))
    {
        let cursor = if entry.position() == selected {
            '>'
        } else {
            ' '
        };
        let secret = if entry.is_secret_bearing() {
            "  SECRET"
        } else {
            ""
        };
        push(
            output,
            &format!(
                "{cursor} {:>3}  {}{secret}",
                entry.position(),
                entry.description()
            ),
        );
    }
    if end < entries.len() {
        push(output, "    …  later events");
    }
}

fn render_derivation(output: &mut Lines, app: &App, width: usize) {
    let Some(derivation) = app.derivation() else {
        push(output, "DERIVATION");
        push(output, "");
        push(
            output,
            "Available only after mnemonic reveal at this state.",
        );
        return;
    };

    push(output, "◆ DERIVATION · ALL VALUES SECRET");
    let guidance = render_document(&derivation_guidance(), width);
    output.extend(guidance.iter().cloned());
    push(output, "");
    push(output, "PROTOCOL");
    push(output, &format!("  {}", derivation.protocol()));
    push(output, "");
    push(output, "01 · CANONICAL INPUT");
    push_wrapped(output, "  ", derivation.canonical_input(), width);
    push(output, "");
    push(output, "02 · BIP-39 ENTROPY");
    push_wrapped(output, "  ", derivation.entropy_hex(), width);
    push(output, "");
    push(output, "03 · CALCULATED CHECKSUM");
    push_wrapped(output, "  ", derivation.checksum_bits(), width);
    push(
        output,
        "  Derived from SHA-256 of entropy; adds no entropy.",
    );
    push(output, "");
    push(output, "04 · 11-BIT WORD INDICES");
    let mut indices = Zeroizing::new(String::with_capacity(
        derivation.word_indices().len().saturating_mul(6),
    ));
    for (position, index) in derivation.word_indices().iter().enumerate() {
        if position > 0 {
            indices.push_str("  ");
        }
        write!(indices, "{index:04}").expect("writing word index to String cannot fail");
    }
    push_wrapped(output, "  ", &indices, width);
    push(output, "");
    push(output, "05 · BIP-39 RECOVERY WORDS");
    let words = Zeroizing::new(derivation.words().join("  "));
    push_wrapped(output, "  ", &words, width);
}

fn render_help(output: &mut Lines, phase: Phase, width: usize) {
    let guidance = render_document(&phase_guidance(phase), width);
    output.extend(guidance.iter().cloned());
    push(output, "");
    let trust_document = trust_boundary(BuildCapabilities::current());
    push(output, trust_document.title());
    let trust = render_document(&trust_document, width);
    output.extend(trust.iter().cloned());
    push(output, "");
    push(output, "HOW THIS CEREMONY WORKS");
    push(
        output,
        "Every accepted action appends an in-memory domain event.",
    );
    push(
        output,
        "Historical inspection folds an earlier event prefix and",
    );
    push(output, "never changes the live state or final mnemonic.");
    push(output, "");
    push(output, "CONTROLS");
    push(output, "  Up/Down or j/k move highlighted choices");
    push(output, "  Left/Right or h/l move between setup steps");
    push(
        output,
        "  Left or h returns from safety to protocol selection",
    );
    push(output, "  Enter advances or acknowledges the current step");
    push(
        output,
        "  Use e on protocol selection for canonical protocol details",
    );
}
