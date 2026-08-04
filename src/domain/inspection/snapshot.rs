use core::fmt;

use crate::domain::{
    bip39::EntropyTarget,
    ceremony::{Ceremony, CeremonyError, Phase},
    protocol::ConversionProtocol,
};

/// Read-only projection of one event prefix.
pub struct InspectionSnapshot {
    position: usize,
    phase: Phase,
    target: Option<EntropyTarget>,
    protocol: Option<ConversionProtocol>,
    roll_count: usize,
    live: bool,
}

impl InspectionSnapshot {
    /// Projects the state at an event position without changing live state.
    ///
    /// # Errors
    ///
    /// Returns [`CeremonyError::InvalidHistoryPosition`] when `position` lies
    /// beyond the journal.
    pub fn at(ceremony: &Ceremony, position: usize) -> Result<Self, CeremonyError> {
        let state = ceremony.state_at(position)?;
        Ok(Self {
            position,
            phase: state.phase(),
            target: state.target(),
            protocol: state.protocol(),
            roll_count: state.capture_count(),
            live: position == ceremony.events().len(),
        })
    }

    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub const fn target(&self) -> Option<EntropyTarget> {
        self.target
    }

    #[must_use]
    pub const fn protocol(&self) -> Option<ConversionProtocol> {
        self.protocol
    }

    #[must_use]
    pub const fn roll_count(&self) -> usize {
        self.roll_count
    }

    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }
}

impl fmt::Debug for InspectionSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("InspectionSnapshot")
            .field("position", &self.position)
            .field("phase", &self.phase)
            .field("target", &self.target)
            .field("protocol", &self.protocol)
            .field("roll_count", &self.roll_count)
            .field("live", &self.live)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::InspectionSnapshot;

    use crate::domain::{
        bip39::EntropyTarget,
        ceremony::{Ceremony, Command},
        dice::DieFace,
        protocol::ConversionProtocol,
    };

    fn ceremony_with_roll() -> Ceremony {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony
            .handle(Command::SelectProtocol(ConversionProtocol::ExactV1))
            .unwrap();
        ceremony.handle(Command::AcknowledgeSafety).unwrap();
        ceremony
            .handle(Command::RecordRoll(DieFace::new(4).unwrap()))
            .unwrap();
        ceremony
    }

    #[test]
    fn history_snapshot_does_not_change_live_state() {
        let ceremony = ceremony_with_roll();
        let historical = InspectionSnapshot::at(&ceremony, 3).unwrap();
        let live = InspectionSnapshot::at(&ceremony, 4).unwrap();

        assert!(!historical.is_live());
        assert_eq!(historical.roll_count(), 0);
        assert!(live.is_live());
        assert_eq!(ceremony.state().rolls().len(), 1);
    }

    #[test]
    fn debug_redacts_roll_content() {
        let ceremony = ceremony_with_roll();
        let snapshot = InspectionSnapshot::at(&ceremony, 4).unwrap();
        assert!(!format!("{snapshot:?}").contains("rolls"));
    }
}
