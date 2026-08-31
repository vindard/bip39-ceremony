use crate::{
    application::{CaptureReadiness, ReadinessKind},
    domain::protocol::CanonicalInputKind,
};

use super::{ContentBlock as B, Document};

#[must_use]
pub fn readiness_document(readiness: CaptureReadiness) -> Document {
    let specification = readiness.protocol().specification(readiness.target());
    let blocks = match readiness.kind() {
        ReadinessKind::Exact => vec![
            B::Paragraph(format!(
                "✓ EXACT REQUIRED COUNT · {} recorded",
                readiness.recorded()
            )),
            B::Paragraph("! Whole-sequence conversion may accept or reject.".to_owned()),
        ],
        ReadinessKind::BitcoinLibBase6 => bitcoinlib_readiness(readiness.recorded()),
        ReadinessKind::BlueWalletBitPack => bluewallet_readiness(readiness.recorded()),
        ReadinessKind::IanColemanDice | ReadinessKind::IanColemanRaw => {
            return Document::new("CAPTURE READY".to_owned(), iancoleman_blocks(readiness));
        }
        ReadinessKind::WordExact => vec![
            B::Paragraph("✓ ALL ENTROPY POSITIONS + FINAL TAIL ACCEPTED".to_owned()),
            B::Paragraph("○ BIP-39 checksum will be calculated, not rolled.".to_owned()),
        ],
        ReadinessKind::Coldcard {
            strict_capacity_reached,
        } => {
            let mut values = vec![B::Paragraph(if strict_capacity_reached {
                format!(
                    "✓ {}+ ROLLS RECORDED",
                    readiness.target().strict_roll_count()
                )
            } else {
                format!(
                    "✓ DOCUMENTED MINIMUM MET · {} recorded",
                    readiness.recorded()
                )
            })];
            if strict_capacity_reached {
                values.push(B::Paragraph(format!(
                    "{} fair independent D6 rolls have >{} bits of raw outcome capacity.",
                    readiness.target().strict_roll_count(),
                    readiness.target().entropy_bits()
                )));
            }
            let input = match specification.canonical_input() {
                CanonicalInputKind::AsciiFaceDigits => "ordered ASCII face digits",
                _ => "the specified canonical input",
            };
            values.push(B::Paragraph(format!("✓ Canonical input is {input}.")));
            values.push(B::Paragraph(
                "Additional COLDCARD-compatible rolls are optional.".to_owned(),
            ));
            return Document::new("CAPTURE READY".to_owned(), values);
        }
        ReadinessKind::KeystoneLegacy => vec![
            B::Paragraph(format!(
                "✓ LEGACY KEYSTONE MINIMUM MET · {} recorded",
                readiness.recorded()
            )),
            B::Paragraph("✓ Face 6 maps to ASCII 0 before SHA-256.".to_owned()),
            B::Paragraph("Additional mapped D6 rolls are optional.".to_owned()),
        ],
        ReadinessKind::JadeDirect => vec![
            B::Paragraph(format!(
                "✓ ALL DIRECT POSITIONS + ENTROPY TAIL · {} rolls recorded",
                readiness.recorded()
            )),
            B::Paragraph("✓ D16/D16/D8 triples map directly to BIP-39 indices.".to_owned()),
            B::Paragraph(
                "○ The checksum and final word will be calculated, not rolled.".to_owned(),
            ),
        ],
        ReadinessKind::BitBox02Direct => vec![
            B::Paragraph(format!(
                "✓ ALL BITBOX TABLE POSITIONS + ENTROPY TAIL · {} outcomes recorded",
                readiness.recorded()
            )),
            B::Paragraph("✓ Rejected D6 faces were retried only at their local digit.".to_owned()),
            B::Paragraph(
                "○ The checksum and final word will be calculated, not chosen.".to_owned(),
            ),
        ],
        ReadinessKind::KruxD20 => vec![
            B::Paragraph(format!(
                "✓ KRUX D20 MINIMUM MET · {} rolls recorded",
                readiness.recorded()
            )),
            B::Paragraph("✓ Decimal D20 faces are separated by ASCII hyphens.".to_owned()),
            B::Paragraph("Additional Krux-compatible D20 rolls are optional.".to_owned()),
        ],
        ReadinessKind::CoinFourD6Direct => coin_four_d6_readiness(readiness.recorded()),
        ReadinessKind::SeedSignerCoins => vec![
            B::Paragraph(format!(
                "✓ EXACT SEEDSIGNER COUNT · {} flips recorded",
                readiness.recorded()
            )),
            B::Paragraph("✓ Ordered 0/1 values become ASCII bytes before SHA-256.".to_owned()),
        ],
    };
    Document::new("CAPTURE READY".to_owned(), blocks)
}

fn bitcoinlib_readiness(recorded: usize) -> Vec<B> {
    vec![
        B::Paragraph(format!("✓ SOURCE REQUIRED COUNT · {recorded} recorded")),
        B::Paragraph(
            "! The base-6 reading is used unhashed and may miss the target width.".to_owned(),
        ),
    ]
}

fn bluewallet_readiness(recorded: usize) -> Vec<B> {
    vec![
        B::Paragraph(format!("✓ ENTROPY WIDTH PACKED · {recorded} rolls used")),
        B::Paragraph("✓ Every entropy bit traces to one roll; nothing is hashed.".to_owned()),
        B::Paragraph("! Further rolls would not be used. Record this tape length.".to_owned()),
    ]
}

fn coin_four_d6_readiness(recorded: usize) -> Vec<B> {
    vec![
        B::Paragraph(format!(
            "✓ 12 DIRECT TABLE CANDIDATES · {recorded} outcomes recorded"
        )),
        B::Paragraph("✓ Rejected coin-plus-four-D6 candidates were retried in full.".to_owned()),
        B::Paragraph(
            "○ The final candidate supplies seven entropy bits; checksum is calculated.".to_owned(),
        ),
    ]
}

fn iancoleman_blocks(readiness: CaptureReadiness) -> Vec<B> {
    if readiness.kind() == ReadinessKind::IanColemanRaw {
        return vec![
            B::Paragraph(format!(
                "✓ WHOLE GROUPS REDUCE TO THE TARGET · {} recorded",
                readiness.recorded()
            )),
            B::Paragraph(
                "✓ The leading remainder is discarded; the last rolls decide the seed.".to_owned(),
            ),
            B::Paragraph(
                "! One more whole 32-bit group would be a longer phrase upstream.".to_owned(),
            ),
        ];
    }
    vec![
        B::Paragraph(format!(
            "✓ TARGET-FILLING COUNT MET · {} recorded",
            readiness.recorded()
        )),
        B::Paragraph("✓ Face 6 maps to ASCII 0 before SHA-256.".to_owned()),
        B::Paragraph("The web tool itself enforces no minimum.".to_owned()),
    ]
}
