use crate::domain::ceremony::{CeremonyState, Phase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SafetyAttestation {
    DeviceIsolated,
    RecordingDisabled,
    CamerasRemoved,
    PrivateTranscriptionReady,
    CopyChannelsAvoided,
    PhysicalRollPolicyFixed,
    ThrowMethodFixed,
    DerivedValuesSecret,
}

impl SafetyAttestation {
    pub const ALL: [Self; 8] = [
        Self::DeviceIsolated,
        Self::RecordingDisabled,
        Self::CamerasRemoved,
        Self::PrivateTranscriptionReady,
        Self::CopyChannelsAvoided,
        Self::PhysicalRollPolicyFixed,
        Self::ThrowMethodFixed,
        Self::DerivedValuesSecret,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    SoftwareVerified,
    UserAttested,
    NotEstablished,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimId {
    TargetChoice,
    ProtocolChoice,
    SafetyAttestation,
    RollStructure,
    GenerationReadiness,
    AttemptRejection,
    ConcealedGeneration,
    RevealedMnemonic,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvidenceClaim {
    id: ClaimId,
    kind: EvidenceKind,
}

impl EvidenceClaim {
    #[must_use]
    pub const fn new(id: ClaimId, kind: EvidenceKind) -> Self {
        Self { id, kind }
    }
    #[must_use]
    pub const fn id(self) -> ClaimId {
        self.id
    }
    #[must_use]
    pub const fn kind(self) -> EvidenceKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftwareEvidence {
    SelectionPending,
    TargetSelected,
    ProtocolSelected,
    CaptureValidated,
    GenerationComplete,
    CeremonyEnded,
    BackupVerified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentEvidence {
    NotAttested,
    UserAttested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssuranceSummary {
    software: SoftwareEvidence,
    environment: EnvironmentEvidence,
}

impl AssuranceSummary {
    #[must_use]
    pub const fn software(self) -> SoftwareEvidence {
        self.software
    }
    #[must_use]
    pub const fn environment(self) -> EnvironmentEvidence {
        self.environment
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CeremonyGuidance {
    claims: [EvidenceClaim; 2],
}

impl CeremonyGuidance {
    #[must_use]
    pub fn for_phase(phase: Phase) -> Self {
        let id = match phase {
            Phase::ChooseTarget => ClaimId::TargetChoice,
            Phase::ChooseProtocol => ClaimId::ProtocolChoice,
            Phase::Safety => ClaimId::SafetyAttestation,
            Phase::EnterRolls => ClaimId::RollStructure,
            Phase::ReadyToGenerate => ClaimId::GenerationReadiness,
            Phase::AttemptRejected => ClaimId::AttemptRejection,
            Phase::Result => ClaimId::ConcealedGeneration,
            Phase::Revealed => ClaimId::RevealedMnemonic,
            Phase::Cancelled => ClaimId::Cancelled,
        };
        Self {
            claims: [
                EvidenceClaim::new(id, EvidenceKind::SoftwareVerified),
                EvidenceClaim::new(id, EvidenceKind::NotEstablished),
            ],
        }
    }

    #[must_use]
    pub const fn claims(&self) -> &[EvidenceClaim] {
        &self.claims
    }
}

#[must_use]
pub fn assurance_summary(state: &CeremonyState) -> AssuranceSummary {
    let software = if state.mnemonic_backup_verified() {
        SoftwareEvidence::BackupVerified
    } else if state.generation_succeeded() {
        SoftwareEvidence::GenerationComplete
    } else {
        match state.phase() {
            Phase::ChooseTarget => SoftwareEvidence::SelectionPending,
            Phase::ChooseProtocol => SoftwareEvidence::TargetSelected,
            Phase::Safety => SoftwareEvidence::ProtocolSelected,
            Phase::EnterRolls | Phase::ReadyToGenerate | Phase::AttemptRejected => {
                SoftwareEvidence::CaptureValidated
            }
            Phase::Result | Phase::Revealed => SoftwareEvidence::GenerationComplete,
            Phase::Cancelled => SoftwareEvidence::CeremonyEnded,
        }
    };
    let environment = if state.safety_acknowledged() {
        EnvironmentEvidence::UserAttested
    } else {
        EnvironmentEvidence::NotAttested
    };
    AssuranceSummary {
        software,
        environment,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        bip39::EntropyTarget,
        ceremony::{Ceremony, Command},
        protocol::ConversionProtocol,
    };

    #[test]
    fn assurance_changes_only_when_evidence_exists() {
        let mut ceremony = Ceremony::new();
        assert_eq!(
            assurance_summary(&ceremony.state()).software(),
            SoftwareEvidence::SelectionPending
        );
        assert_eq!(
            assurance_summary(&ceremony.state()).environment(),
            EnvironmentEvidence::NotAttested
        );
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony
            .handle(Command::SelectProtocol(ConversionProtocol::ExactV1))
            .unwrap();
        assert_eq!(
            assurance_summary(&ceremony.state()).software(),
            SoftwareEvidence::ProtocolSelected
        );
        assert_eq!(
            assurance_summary(&ceremony.state()).environment(),
            EnvironmentEvidence::NotAttested
        );
        ceremony.handle(Command::AcknowledgeSafety).unwrap();
        assert_eq!(
            assurance_summary(&ceremony.state()).environment(),
            EnvironmentEvidence::UserAttested
        );
    }

    #[test]
    fn early_cancellation_does_not_claim_generation_completed() {
        let mut ceremony = Ceremony::new();
        ceremony.cancel().unwrap();

        let summary = assurance_summary(&ceremony.state());
        assert_eq!(summary.software(), SoftwareEvidence::CeremonyEnded);
        assert_eq!(summary.environment(), EnvironmentEvidence::NotAttested);
    }
}
