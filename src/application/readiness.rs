use crate::domain::{bip39::EntropyTarget, ceremony::CeremonyState, protocol::ConversionProtocol};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessKind {
    Exact,
    WordExact,
    NativeHash,
    Coldcard { strict_capacity_reached: bool },
    KeystoneLegacy,
    JadeDirect,
    BitBox02Direct,
    KruxD20,
    SeedSignerCoins,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureReadiness {
    kind: ReadinessKind,
    protocol: ConversionProtocol,
    target: EntropyTarget,
    recorded: usize,
}

impl CaptureReadiness {
    #[must_use]
    pub fn from_state(state: &CeremonyState) -> Option<Self> {
        let protocol = state.protocol()?;
        let target = state.target()?;
        let recorded = state.capture_count();
        if !state.capture_assessment()?.is_complete() {
            return None;
        }
        let kind = match protocol {
            ConversionProtocol::ExactV1 => ReadinessKind::Exact,
            ConversionProtocol::WordExactV1 => ReadinessKind::WordExact,
            ConversionProtocol::NativeHashV1 => ReadinessKind::NativeHash,
            ConversionProtocol::ColdcardV1 => ReadinessKind::Coldcard {
                strict_capacity_reached: recorded >= target.strict_roll_count(),
            },
            ConversionProtocol::KeystoneLegacyV1 => ReadinessKind::KeystoneLegacy,
            ConversionProtocol::JadeDirectV1 => ReadinessKind::JadeDirect,
            ConversionProtocol::BitBox02DirectV1 => ReadinessKind::BitBox02Direct,
            ConversionProtocol::KruxD20V1 => ReadinessKind::KruxD20,
            ConversionProtocol::SeedSignerCoinsV1 => ReadinessKind::SeedSignerCoins,
        };
        Some(Self {
            kind,
            protocol,
            target,
            recorded,
        })
    }

    #[must_use]
    pub const fn kind(self) -> ReadinessKind {
        self.kind
    }
    #[must_use]
    pub const fn protocol(self) -> ConversionProtocol {
        self.protocol
    }
    #[must_use]
    pub const fn target(self) -> EntropyTarget {
        self.target
    }
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::{DieFace, RollSequence};

    #[test]
    fn readiness_is_emitted_only_for_complete_capture() {
        let mut rolls = RollSequence::new();
        for _ in 0..50 {
            rolls.push(DieFace::new(1).unwrap());
        }
        let protocol = ConversionProtocol::ExactV1;
        let target = EntropyTarget::Words12;
        let mut ceremony = crate::domain::ceremony::Ceremony::new();
        ceremony
            .handle(crate::domain::ceremony::Command::SelectTarget(target))
            .unwrap();
        ceremony
            .handle(crate::domain::ceremony::Command::SelectProtocol(protocol))
            .unwrap();
        ceremony
            .handle(crate::domain::ceremony::Command::AcknowledgeSafety)
            .unwrap();
        for face in rolls.faces() {
            ceremony
                .handle(crate::domain::ceremony::Command::RecordRoll(*face))
                .unwrap();
        }
        assert_eq!(
            CaptureReadiness::from_state(&ceremony.state())
                .unwrap()
                .kind(),
            ReadinessKind::Exact
        );

        ceremony
            .handle(crate::domain::ceremony::Command::UndoRoll)
            .unwrap();
        assert!(CaptureReadiness::from_state(&ceremony.state()).is_none());
    }
}
