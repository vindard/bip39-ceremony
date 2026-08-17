use crate::{
    application::ReproductionReceipt,
    domain::{bip39::EntropyTarget, protocol::ConversionProtocol},
};

use super::{ContentBlock as B, Document, reproduction_receipt};

#[must_use]
pub fn concealed_generation(protocol: ConversionProtocol, target: EntropyTarget) -> Document {
    Document::new(
        "MNEMONIC READY".to_owned(),
        vec![
            B::Heading(format!(
                "✓ GENERATION COMPLETE · {}-WORD BIP-39",
                target.word_count()
            )),
            B::Paragraph(format!("✓ {} · no hidden randomness", protocol.id())),
            B::Heading("◆ SECRET CONCEALED".to_owned()),
            B::Paragraph(
                "Recovery words, entropy, and indices are not drawn on screen.".to_owned(),
            ),
            B::Paragraph("Secret material remains in memory until the ceremony ends.".to_owned()),
            B::Heading("BEFORE REVEAL".to_owned()),
            B::Paragraph(
                "1. Confirm privacy; turn off cameras, sharing, and recording.".to_owned(),
            ),
            B::Paragraph("2. Have paper and a permanent pen ready for transcription.".to_owned()),
        ],
    )
}

#[must_use]
pub fn transcription_instructions() -> Document {
    Document::new(
        "TRANSCRIPTION CHECK".to_owned(),
        vec![
            B::Paragraph("1. Write every word together with its number.".to_owned()),
            B::Paragraph("2. Compare the written copy against this list twice.".to_owned()),
            B::Paragraph(
                "3. Check the first word, last word, and every numbered position.".to_owned(),
            ),
            B::Paragraph(
                "Do not photograph, copy, paste, print, or enter them on a networked device."
                    .to_owned(),
            ),
            B::Paragraph(
                "! BIP-39 checksum catches many mistakes, not every wrong word or order."
                    .to_owned(),
            ),
        ],
    )
}

#[must_use]
pub fn verified_recovery(receipt: ReproductionReceipt) -> Document {
    let mut blocks = vec![B::Heading("✓ BACKUP WORDS VERIFIED IN ORDER".to_owned())];
    let reproduction = reproduction_receipt(receipt);
    blocks.push(B::Heading(reproduction.title().to_owned()));
    blocks.extend(reproduction.blocks().iter().cloned());
    blocks.extend([
        B::Heading("GENERATION IS NOT VERIFIED WALLET RECOVERY".to_owned()),
        B::Paragraph("Record required passphrase, network, script, derivation, or descriptor metadata separately.".to_owned()),
        B::Paragraph("Independently restore and verify addresses before relying on this backup.".to_owned()),
    ]);
    Document::new("BACKUP VERIFIED".to_owned(), blocks)
}

#[must_use]
pub fn attempt_rejection(protocol: ConversionProtocol, required_rolls: usize) -> Document {
    let (title, reason) = match protocol {
        ConversionProtocol::ColdcardV1 => (
            "COLDCARD ATTEMPT REJECTED",
            "Some face occurred more than 30% of the time, so Coldcard's enforced workflow rejects the sequence.",
        ),
        ConversionProtocol::BitcoinLibBase6V1 => (
            "BASE-6 READING REJECTED",
            "The base-6 reading of these rolls does not encode to exactly the target entropy width, and this reading pads nothing.",
        ),
        _ => (
            "EXACT ATTEMPT REJECTED",
            "Expected protocol outcome preserving a uniform conversion.",
        ),
    };
    Document::new(
        title.to_owned(),
        vec![
            B::Heading(format!("× {title} · NO ENTROPY RESULT")),
            B::Paragraph(format!("✓ {reason}")),
            B::Paragraph(format!(
                "× No part is reusable; re-roll all {required_rolls} physical results."
            )),
            B::Paragraph(
                "! Do not change one face or keep a favorable part of the sequence.".to_owned(),
            ),
            B::Paragraph(
                "Audit events are retained with rejected faces concealed and still secret."
                    .to_owned(),
            ),
        ],
    )
}

#[must_use]
pub fn finish_confirmation(revealed: bool) -> Document {
    let secret = if revealed {
        "✓ Roll history, entropy, mnemonic, and derivation values"
    } else {
        "✓ Roll history and any generated secret material"
    };
    Document::new(
        if revealed {
            "END CEREMONY?"
        } else {
            "CANCEL CEREMONY?"
        }
        .to_owned(),
        vec![
            B::Heading("OWNED BUFFERS WILL BE CLEARED".to_owned()),
            B::Paragraph(secret.to_owned()),
            B::Heading("NOT CONTROLLED BY THIS APPLICATION".to_owned()),
            B::Paragraph(
                "! Terminal recording/scrollback, swap, crash dumps, cameras, or OS copies"
                    .to_owned(),
            ),
        ],
    )
}

#[must_use]
pub fn derivation_guidance() -> Document {
    Document::new(
        "DERIVATION GUIDANCE".to_owned(),
        vec![
            B::Paragraph(
                "! This view exposes recovery-secret material. Press h to conceal it immediately."
                    .to_owned(),
            ),
            B::Heading("DETERMINISTIC PIPELINE".to_owned()),
            B::Verbatim("DICE → CANONICAL INPUT → ENTROPY → CHECKSUM → INDICES → WORDS".to_owned()),
            B::Heading("VERIFY THE CHAIN".to_owned()),
            B::Paragraph(
                "The same protocol and canonical input must reproduce every stage above exactly."
                    .to_owned(),
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concealed_copy_uses_domain_identity_and_target() {
        let document =
            concealed_generation(ConversionProtocol::WordExactV1, EntropyTarget::Words24);
        let text = format!("{:?}", document.blocks());
        assert!(text.contains("24-WORD BIP-39"));
        assert!(text.contains("word-exact-v1"));
        assert!(text.contains("no hidden randomness"));
    }
}
