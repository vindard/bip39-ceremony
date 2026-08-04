use crate::application::{BuildCapabilities, Capability, ReproductionReceipt};

use super::{ContentBlock as B, Document};

#[must_use]
pub fn trust_boundary(capabilities: BuildCapabilities) -> Document {
    let network = match capabilities.network_io() {
        Capability::Absent => "no application network adapter",
    };
    let persistence = match capabilities.ceremony_persistence() {
        Capability::Absent => "no ceremony persistence adapter",
    };
    Document::new(
        "APPLICATION TRUST BOUNDARY".to_owned(),
        vec![
            B::Paragraph(format!(
                "✓ Build version {} · {network} · {persistence}",
                capabilities.version()
            )),
            B::Paragraph("! Binary provenance, actual offline status, and operating-system integrity are not verified.".to_owned()),
        ],
    )
}

#[must_use]
pub fn reproduction_receipt(receipt: ReproductionReceipt) -> Document {
    let (input_label, secret_label) = match receipt.protocol() {
        crate::domain::protocol::ConversionProtocol::SeedSignerCoinsV1 => ("flips", "coin sides"),
        crate::domain::protocol::ConversionProtocol::BitBox02DirectV1 => {
            ("outcomes", "D6 faces and coin sides")
        }
        crate::domain::protocol::ConversionProtocol::JadeDirectV1 => ("rolls", "mixed-dice faces"),
        crate::domain::protocol::ConversionProtocol::KruxD20V1 => ("rolls", "D20 faces"),
        _ => ("rolls", "faces"),
    };
    Document::new(
        "REPRODUCTION METADATA · SECRET VALUES OMITTED".to_owned(),
        vec![
            B::Paragraph(format!(
                "Protocol {} · Target {} words · Recorded {input_label} {}",
                receipt.protocol().id(),
                receipt.target().word_count(),
                receipt.input_count()
            )),
            B::Paragraph(format!(
                "Secret {secret_label} omitted. Counts can reveal rejection history; preserve the exact protocol ID for independent reproduction."
            )),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_copy_reflects_declared_capabilities() {
        let document = trust_boundary(BuildCapabilities::current());
        let text = format!("{:?}", document.blocks());
        assert!(text.contains("no application network adapter"));
        assert!(text.contains("no ceremony persistence adapter"));
        assert!(text.contains("actual offline status"));
    }
}
