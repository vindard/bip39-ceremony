use std::fmt::Write;

use zeroize::Zeroizing;

use crate::{
    application::{BuildCapabilities, DerivationProjection},
    domain::{bip39::EntropyTarget, ceremony::Phase},
    presentation::{
        derivation_guidance, phase_guidance, physical_entropy_guidance, protocol_menu_explanation,
        trust_boundary,
    },
};

use super::{
    App, InspectorView, Lines, lines,
    protocol::{render_document, render_protocol_explanation},
    push, push_wrapped,
};

pub(super) fn render_inspector(app: &App, width: usize) -> Lines {
    match app.inspector().map(|inspector| inspector.view) {
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
        Some(InspectorView::PhysicalEntropy) => {
            render_document(&physical_entropy_guidance(), width)
        }
        Some(InspectorView::Help) => {
            let mut output = Lines::new(Vec::new());
            render_help(&mut output, app.ceremony().state().phase(), width);
            output
        }
        None => lines(&["Inspection unavailable."]),
    }
}

fn render_derivation(output: &mut Lines, app: &App, width: usize) {
    let Some(derivation) = app.derivation() else {
        push(output, "DERIVATION");
        push(output, "");
        push(output, "Available only after mnemonic reveal.");
        return;
    };

    push(output, "◆ DERIVATION · ALL VALUES SECRET");
    let guidance = render_document(&derivation_guidance(), width);
    output.extend(guidance.iter().cloned());
    push(output, "");
    render_derivation_projection(output, &derivation, width);
}

/// Renders the numbered BIP-39 derivation stages (protocol, canonical input,
/// entropy, checksum, word indices, recovery words) for a projection. Shared by
/// the ceremony inspector and the group-compare derivation overlay.
pub(super) fn render_derivation_projection(
    output: &mut Lines,
    derivation: &DerivationProjection,
    width: usize,
) {
    push(output, "PROTOCOL");
    push(output, &format!("  {}", derivation.protocol()));
    push(output, "");
    push(output, "01 · CANONICAL INPUT");
    push(
        output,
        &format!("  encoding · {}", derivation.canonical_input_encoding()),
    );
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
