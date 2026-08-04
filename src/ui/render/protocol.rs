use crate::{
    domain::{bip39::EntropyTarget, protocol::ConversionProtocol},
    presentation::{ContentBlock, protocol_explanation},
};

use super::{Lines, push, push_wrapped};

pub(super) fn render_protocol_explanation(
    protocol: ConversionProtocol,
    target: EntropyTarget,
    width: usize,
) -> Lines {
    let document = protocol_explanation(protocol, target);
    render_document(&document, width)
}

pub(super) fn render_document(document: &crate::presentation::Document, width: usize) -> Lines {
    let mut output = Lines::new(Vec::new());
    for (index, block) in document.blocks().iter().enumerate() {
        if index > 0 {
            push(&mut output, "");
        }
        match block {
            ContentBlock::Heading(text) => push(&mut output, text),
            ContentBlock::Paragraph(text) => push_wrapped(&mut output, "", text, width),
            ContentBlock::Verbatim(text) => {
                for line in text.lines() {
                    push(&mut output, &format!("  {line}"));
                }
            }
        }
    }
    output
}
