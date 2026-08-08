//! Use-case orchestration across domain and adapter ports.

mod derive;
mod evidence;
mod group;
mod guidance;
mod hash_preview;
pub mod ports;
mod readiness;
mod session;

pub use bip39_ceremony_core::Calculation as Generation;
pub use derive::DerivationProjection;
pub use evidence::{BuildCapabilities, Capability, ReproductionReceipt};
pub(crate) use group::{GroupSession, RollProgress};
pub use guidance::{
    AssuranceSummary, CeremonyGuidance, ClaimId, EnvironmentEvidence, EvidenceClaim, EvidenceKind,
    SafetyAttestation, SoftwareEvidence, assurance_summary,
};
pub use hash_preview::ColdcardHashPreview;
pub use readiness::{CaptureReadiness, ReadinessKind};
pub(crate) use session::{BackupSubmission, CeremonySession};
