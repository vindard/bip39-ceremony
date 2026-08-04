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
        ReadinessKind::WordExact => vec![
            B::Paragraph("✓ ALL ENTROPY POSITIONS + FINAL TAIL ACCEPTED".to_owned()),
            B::Paragraph("○ BIP-39 checksum will be calculated, not rolled.".to_owned()),
        ],
        ReadinessKind::NativeHash => vec![B::Paragraph(format!(
            "✓ {} ROLLS · version + target + count bound",
            readiness.recorded()
        ))],
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
