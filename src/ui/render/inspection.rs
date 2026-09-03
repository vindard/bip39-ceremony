use std::fmt::Write;

use zeroize::Zeroizing;

use crate::{
    application::{BuildCapabilities, DerivationProjection},
    domain::{bip39::EntropyTarget, ceremony::Phase},
    presentation::{
        BIP39_CHECKSUM, BIP39_VERSION, BITCOIN_HASHES_CHECKSUM, BITCOIN_HASHES_VERSION,
        SourceScope, derivation_guidance, phase_guidance, physical_entropy_guidance,
        protocol_menu_explanation, protocol_source_files, trust_boundary,
    },
};

use super::{
    App, InspectorView, Lines, lines,
    protocol::{render_document, render_protocol_explanation},
    push, push_wrapped,
};

pub(super) fn render_preview(app: &App, width: usize) -> Lines {
    if app.inspector().is_some() {
        return render_inspector(app, width);
    }

    let state = app.ceremony().state();
    if state.phase() == Phase::Revealed {
        return lines(&[
            "PREVIEW CONCEALED",
            "",
            "Recovery words remain in the Task pane.",
            "Press d to deliberately open the secret derivation.",
        ]);
    }
    if state.phase() == Phase::ChooseProtocol {
        let target = state.target().unwrap_or(EntropyTarget::Words12);
        let choice = app.selected_protocol_choice();
        return choice.implemented_protocol(target).map_or_else(
            || render_document(&protocol_menu_explanation(choice, target), width),
            |protocol| render_protocol_explanation(protocol, target, width),
        );
    }
    if let Some((protocol, target)) = state.protocol().zip(state.target()) {
        return render_protocol_explanation(protocol, target, width);
    }

    let mut output = Lines::new(Vec::new());
    let guidance = render_document(&phase_guidance(state.phase()), width);
    output.extend(guidance.iter().cloned());
    output
}

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
        Some(InspectorView::ProtocolSource) => render_protocol_source(app, width),
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

fn render_protocol_source(app: &App, width: usize) -> Lines {
    let Some(protocol) = app.inspected_protocol() else {
        return lines(&["Protocol source unavailable."]);
    };
    let files = protocol_source_files(protocol);
    let selected = app
        .inspector()
        .map_or(0, |inspector| inspector.source_file)
        .min(files.len().saturating_sub(1));
    let file = files[selected];

    let mut output = Lines::new(Vec::new());
    push(&mut output, "◆ CALCULATION SOURCE");
    push(&mut output, &format!("PROTOCOL · {}", protocol.id()));
    push(
        &mut output,
        &format!(
            "STAGE {} OF {} · {}",
            selected + 1,
            files.len(),
            file.label()
        ),
    );
    push_wrapped(&mut output, "  ", file.path(), width);
    push(&mut output, "");
    match file.scope() {
        SourceScope::CompleteModule => {
            push(
                &mut output,
                "Complete local module embedded verbatim; no source lines are omitted.",
            );
            if selected == 0 {
                push_wrapped(
                    &mut output,
                    "External boundary · ",
                    &format!("bip39 {BIP39_VERSION} · crates.io SHA-256 {BIP39_CHECKSUM}"),
                    width,
                );
                push_wrapped(
                    &mut output,
                    "External SHA-256 · ",
                    &format!(
                        "bitcoin_hashes {BITCOIN_HASHES_VERSION} · crates.io SHA-256 {BITCOIN_HASHES_CHECKSUM}"
                    ),
                    width,
                );
                push(
                    &mut output,
                    "Mnemonic::from_entropy_in executes in bip39; its digest uses bitcoin_hashes.",
                );
                push(
                    &mut output,
                    "Exact dependency source is external to this view and identified above.",
                );
                push(
                    &mut output,
                    "Words, indices, and checksum come from one value; entropy is round-trip checked.",
                );
                push(
                    &mut output,
                    "This display does not prove binary provenance; verify the build separately.",
                );
            }
        }
        SourceScope::SelectedFunctions(functions) => {
            push_wrapped(
                &mut output,
                "Selected functions · ",
                &functions.join(" · "),
                width,
            );
            push(
                &mut output,
                "These are selected accepted-value formulas, not the complete capture call path.",
            );
            push(
                &mut output,
                "Validation gates, rejection branches, tests, and unrelated code are omitted.",
            );
        }
    }
    push(
        &mut output,
        "Original line numbers are display-only. Long lines may be clipped.",
    );
    push(&mut output, "");
    for (line_number, line) in file.lines() {
        if line_number == 0 {
            push(&mut output, "");
        } else {
            push(&mut output, &format!("{line_number:04} │ {line}"));
        }
    }
    output
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
