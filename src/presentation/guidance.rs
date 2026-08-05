use crate::{
    application::{
        AssuranceSummary, CeremonyGuidance, ClaimId, EnvironmentEvidence, EvidenceKind,
        SoftwareEvidence,
    },
    domain::ceremony::Phase,
};

use super::{ContentBlock as B, Document};

#[must_use]
pub fn phase_guidance(phase: Phase) -> Document {
    let guidance = CeremonyGuidance::for_phase(phase);
    let blocks = guidance
        .claims()
        .iter()
        .map(|claim| {
            let text = match (claim.id(), claim.kind()) {
                (ClaimId::TargetChoice, EvidenceKind::SoftwareVerified) => {
                    "You are choosing the BIP-39 entropy target and backup length."
                }
                (ClaimId::TargetChoice, EvidenceKind::NotEstablished) => {
                    "More words do not repair a compromised device or exposed backup."
                }
                (ClaimId::ProtocolChoice, EvidenceKind::SoftwareVerified) => {
                    "You are choosing the deterministic rule that maps physical outcomes to entropy."
                }
                (ClaimId::ProtocolChoice, EvidenceKind::NotEstablished) => {
                    "Hashing cannot create entropy or certify a fair physical source."
                }
                (ClaimId::SafetyAttestation, EvidenceKind::SoftwareVerified) => {
                    "The checklist records deliberate privacy attestations."
                }
                (ClaimId::SafetyAttestation, EvidenceKind::NotEstablished) => {
                    "The application cannot verify the room, operating system, or network."
                }
                (ClaimId::RollStructure, EvidenceKind::SoftwareVerified) => {
                    "The application verifies valid protocol input keys and structure."
                }
                (ClaimId::RollStructure, EvidenceKind::NotEstablished) => {
                    "It cannot know whether a key matched the physical outcome."
                }
                (ClaimId::GenerationReadiness, EvidenceKind::SoftwareVerified) => {
                    "The selected protocol has enough structurally valid input."
                }
                (ClaimId::GenerationReadiness, EvidenceKind::NotEstablished) => {
                    "Readiness does not establish source fairness or correct physical entry."
                }
                (ClaimId::AttemptRejection, EvidenceKind::SoftwareVerified) => {
                    "Rejection follows the selected protocol's whole-sequence rule."
                }
                (ClaimId::AttemptRejection, EvidenceKind::NotEstablished) => {
                    "It does not certify the physical source or diagnose an operator."
                }
                (ClaimId::ConcealedGeneration, EvidenceKind::SoftwareVerified) => {
                    "Deterministic conversion completed without exposing derived secrets."
                }
                (ClaimId::ConcealedGeneration, EvidenceKind::NotEstablished) => {
                    "Generation does not verify transcription or downstream wallet recovery."
                }
                (ClaimId::RevealedMnemonic, EvidenceKind::SoftwareVerified) => {
                    "The mnemonic is visible for deliberate private transcription."
                }
                (ClaimId::RevealedMnemonic, EvidenceKind::NotEstablished) => {
                    "Visibility alone is not proof that the backup is correct."
                }
                (ClaimId::Cancelled, EvidenceKind::SoftwareVerified) => {
                    "The ceremony ended and owned secret buffers will be dropped."
                }
                (ClaimId::Cancelled, EvidenceKind::NotEstablished) => {
                    "Terminal, operating-system, camera, or external copies are not controlled."
                }
                (_, EvidenceKind::UserAttested) => "This condition was attested by the user.",
            };
            B::Paragraph(format!("{} {text}", evidence_symbol(claim.kind())))
        })
        .collect();
    Document::new("ABOUT THIS STEP".to_owned(), blocks)
}

#[must_use]
pub fn assurance_line(summary: AssuranceSummary) -> String {
    let software = match summary.software() {
        SoftwareEvidence::SelectionPending => "pending",
        SoftwareEvidence::TargetSelected => "target selected",
        SoftwareEvidence::ProtocolSelected => "protocol selected",
        SoftwareEvidence::CaptureValidated => "capture structure",
        SoftwareEvidence::GenerationComplete => "generation complete",
        SoftwareEvidence::CeremonyEnded => "ceremony ended",
        SoftwareEvidence::BackupVerified => "backup matched",
    };
    let environment = match summary.environment() {
        EnvironmentEvidence::NotAttested => "not attested",
        EnvironmentEvidence::UserAttested => "user attested",
    };
    format!("✓ SOFTWARE: {software} · ◇ ENV: {environment} · ! UNKNOWN: die/device")
}

const fn evidence_symbol(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::SoftwareVerified => "✓",
        EvidenceKind::UserAttested => "◇",
        EvidenceKind::NotEstablished => "!",
    }
}
