use core::fmt;

use bip39_ceremony_core::{CalculationError, CalculationOutcome, Capture, calculate};

use crate::domain::{
    bip39::{BackupVerifier, WordSubmission},
    ceremony::{Ceremony, CeremonyError, Command},
    protocol::ConversionProtocol,
};

use super::{ColdcardHashPreview, DerivationProjection, Generation, ports::Sha256};

pub(crate) struct CeremonySession {
    ceremony: Ceremony,
    generation: Option<Generation>,
    sha256: Box<dyn Sha256>,
}

impl CeremonySession {
    #[must_use]
    pub fn new(sha256: Box<dyn Sha256>) -> Self {
        Self {
            ceremony: Ceremony::new(),
            generation: None,
            sha256,
        }
    }

    #[must_use]
    pub const fn ceremony(&self) -> &Ceremony {
        &self.ceremony
    }

    #[must_use]
    pub const fn generation(&self) -> Option<&Generation> {
        self.generation.as_ref()
    }

    #[must_use]
    pub fn derivation(&self, position: usize) -> Option<DerivationProjection> {
        let generation = self.generation.as_ref()?;
        self.ceremony.state_at(position).ok()?;
        Some(DerivationProjection::build(generation))
    }

    #[must_use]
    pub fn coldcard_hash_preview(&self) -> Option<ColdcardHashPreview> {
        let state = self.ceremony.state();
        (state.protocol() == Some(ConversionProtocol::ColdcardV1))
            .then(|| ColdcardHashPreview::build(state.rolls(), self.sha256.as_ref()))
    }

    /// Executes one semantic aggregate command.
    ///
    /// # Errors
    ///
    /// Returns the aggregate guard failure when the command is invalid now.
    pub(crate) fn execute(&mut self, command: Command) -> Result<(), CeremonyError> {
        self.ceremony.handle(command)
    }

    /// Cancels the ceremony and immediately clears session-owned generation.
    ///
    /// # Errors
    ///
    /// Returns the aggregate guard failure if the ceremony already ended.
    pub(crate) fn cancel(&mut self) -> Result<(), CeremonyError> {
        self.ceremony.cancel()?;
        self.generation = None;
        Ok(())
    }

    /// Compares one concealed backup word against this session's generation.
    ///
    /// A completed receipt is consumed here so it cannot be applied to a
    /// different session by the caller.
    ///
    /// # Errors
    ///
    /// Returns an incomplete-generation error when no mnemonic exists, or an
    /// aggregate error if completion is invalid in the current phase.
    pub(crate) fn submit_backup_word(
        &mut self,
        verifier: &mut BackupVerifier,
    ) -> Result<BackupSubmission, SessionError> {
        let generation = self
            .generation
            .as_ref()
            .ok_or(SessionError::MissingGeneration)?;
        match verifier.submit(generation.mnemonic()) {
            WordSubmission::Mismatch { position } => Ok(BackupSubmission::Mismatch { position }),
            WordSubmission::Next { position } => Ok(BackupSubmission::Next { position }),
            WordSubmission::Complete(receipt) => {
                self.ceremony.verify_mnemonic_backup(receipt)?;
                Ok(BackupSubmission::Complete)
            }
        }
    }

    /// Confirms completed capture and performs deterministic generation.
    ///
    /// # Errors
    ///
    /// Returns a ceremony or generation failure without inventing a result.
    pub fn confirm_rolls_and_generate(&mut self) -> Result<SessionGeneration, SessionError> {
        self.ceremony.validate_roll_confirmation()?;
        let state = self.ceremony.state();
        let target = state.target().ok_or(SessionError::IncompleteSelection)?;
        let protocol = state.protocol().ok_or(SessionError::IncompleteSelection)?;
        let capture = match protocol {
            ConversionProtocol::SeedSignerCoinsV1 => Capture::Coins(state.flips()),
            ConversionProtocol::JadeDirectV1 => Capture::Jade(state.jade()),
            ConversionProtocol::BitBox02DirectV1 => Capture::BitBox(state.bitbox()),
            ConversionProtocol::KruxD20V1 => Capture::D20(state.d20()),
            ConversionProtocol::CoinFourD6DirectV1 => Capture::CoinFourD6(state.coin_four_d6()),
            _ => Capture::Dice(state.rolls()),
        };
        let outcome = calculate(target, protocol, capture)?;

        // Commit state changes only after every fallible adapter call succeeds.
        self.ceremony.handle(Command::ConfirmRolls)?;
        match outcome {
            CalculationOutcome::Accepted(generation) => {
                self.ceremony.record_generation_succeeded()?;
                self.generation = Some(generation);
                Ok(SessionGeneration::Generated)
            }
            CalculationOutcome::ExactRejected
            | CalculationOutcome::ColdcardDistributionRejected
            | CalculationOutcome::Base6WidthRejected => {
                self.ceremony.record_attempt_rejected()?;
                Ok(SessionGeneration::AttemptRejected)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackupSubmission {
    Mismatch { position: usize },
    Next { position: usize },
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionGeneration {
    Generated,
    AttemptRejected,
}

#[derive(Debug)]
pub(crate) enum SessionError {
    Ceremony(CeremonyError),
    Calculation(CalculationError),
    IncompleteSelection,
    MissingGeneration,
}

impl From<CeremonyError> for SessionError {
    fn from(error: CeremonyError) -> Self {
        Self::Ceremony(error)
    }
}

impl From<CalculationError> for SessionError {
    fn from(error: CalculationError) -> Self {
        Self::Calculation(error)
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ceremony(error) => error.fmt(formatter),
            Self::Calculation(error) => error.fmt(formatter),
            Self::IncompleteSelection => {
                formatter.write_str("target and protocol must be selected")
            }
            Self::MissingGeneration => formatter.write_str("mnemonic generation is incomplete"),
        }
    }
}

impl std::error::Error for SessionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{bip39::EntropyTarget, dice::DieFace, protocol::ConversionProtocol};

    struct Hash;
    impl Sha256 for Hash {
        fn hash(&self, _chunks: &[&[u8]]) -> [u8; 32] {
            [0; 32]
        }
    }

    fn ready_session() -> CeremonySession {
        let mut session = CeremonySession::new(Box::new(Hash));
        session
            .execute(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        session
            .execute(Command::SelectProtocol(ConversionProtocol::ExactV1))
            .unwrap();
        session.execute(Command::AcknowledgeSafety).unwrap();
        for _ in 0..50 {
            session
                .execute(Command::RecordRoll(DieFace::new(1).unwrap()))
                .unwrap();
        }
        session
    }

    #[test]
    fn session_orchestrates_generation_and_aggregate_transition() {
        let mut session = ready_session();

        assert_eq!(
            session.confirm_rolls_and_generate().unwrap(),
            SessionGeneration::Generated
        );
        assert!(session.generation().is_some());
        assert_eq!(
            session.ceremony().state().phase(),
            crate::domain::ceremony::Phase::Result
        );
    }

    #[test]
    fn coldcard_distribution_rejection_requires_a_fresh_attempt() {
        let mut session = CeremonySession::new(Box::new(Hash));
        session
            .execute(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        session
            .execute(Command::SelectProtocol(ConversionProtocol::ColdcardV1))
            .unwrap();
        session.execute(Command::AcknowledgeSafety).unwrap();
        for _ in 0..50 {
            session
                .execute(Command::RecordRoll(DieFace::new(1).unwrap()))
                .unwrap();
        }

        assert_eq!(
            session.confirm_rolls_and_generate().unwrap(),
            SessionGeneration::AttemptRejected
        );
        assert!(session.generation().is_none());
        assert_eq!(
            session.ceremony().state().phase(),
            crate::domain::ceremony::Phase::AttemptRejected
        );

        session.execute(Command::RestartAttempt).unwrap();
        assert_eq!(
            session.ceremony().state().phase(),
            crate::domain::ceremony::Phase::EnterRolls
        );
        assert!(session.ceremony().state().rolls().is_empty());
        assert!(session.ceremony().events().len() > 50);
        let retained_audit = format!("{:?}", session.ceremony().events());
        assert!(retained_audit.contains("RollRecorded([REDACTED])"));
    }

    #[test]
    fn cancellation_clears_generated_secret_state() {
        let mut session = ready_session();
        session.confirm_rolls_and_generate().unwrap();
        assert!(session.generation().is_some());

        session.cancel().unwrap();

        assert!(session.generation().is_none());
        assert_eq!(session.ceremony().events().len(), 1);
        assert_eq!(
            session.ceremony().state().phase(),
            crate::domain::ceremony::Phase::Cancelled
        );
    }

    #[test]
    fn backup_completion_is_consumed_by_its_owning_session() {
        let mut session = ready_session();
        session.confirm_rolls_and_generate().unwrap();
        session.execute(Command::RevealMnemonic).unwrap();
        let mut verifier = BackupVerifier::new();

        for position in 0..12 {
            let word = &session.generation().unwrap().mnemonic().words()[position];
            for character in word.chars() {
                verifier.push(character);
            }
            let submission = session.submit_backup_word(&mut verifier).unwrap();
            if position == 11 {
                assert_eq!(submission, BackupSubmission::Complete);
            }
        }

        assert!(session.ceremony().state().mnemonic_backup_verified());
    }
}
