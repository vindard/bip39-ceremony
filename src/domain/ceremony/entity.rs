use core::fmt;

use zeroize::Zeroize;

use crate::domain::{
    bip39::EntropyTarget,
    coin::CoinFlip,
    d20::D20Face,
    dice::DieFace,
    jade::{D8Face, D16Face},
    protocol::{
        BitBoxObservationKind, CoinFourD6ObservationKind, ConversionProtocol, JadeDieKind,
        bitbox_progress, coin_four_d6_progress, jade_expected_die,
    },
};

use super::{CeremonyError, CeremonyState, Command, Event, Phase};

/// Ephemeral event-sourced aggregate for one generation journey.
pub struct Ceremony {
    events: Vec<Event>,
}

impl Default for Ceremony {
    fn default() -> Self {
        Self::new()
    }
}

impl Ceremony {
    #[must_use]
    pub fn new() -> Self {
        // Reserve beyond every rejection-free ceremony so secret-bearing event
        // history does not normally cross a heap reallocation before zeroizing.
        Self {
            events: Vec::with_capacity(512),
        }
    }

    #[must_use]
    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }

    #[must_use]
    pub fn state(&self) -> CeremonyState {
        Self::replay(&self.events)
    }

    /// Cancels this ceremony and immediately clears its secret event history.
    ///
    /// # Errors
    ///
    /// Returns [`CeremonyError::Cancelled`] if cancellation already occurred.
    pub fn cancel(&mut self) -> Result<(), CeremonyError> {
        if self.state().is_cancelled() {
            return Err(CeremonyError::Cancelled);
        }
        self.events.zeroize();
        self.events.push(Event::CeremonyCancelled);
        Ok(())
    }

    /// Reconstructs state after the first `position` events.
    ///
    /// # Errors
    ///
    /// Returns [`CeremonyError::InvalidHistoryPosition`] if the requested
    /// position is beyond the event journal.
    pub fn state_at(&self, position: usize) -> Result<CeremonyState, CeremonyError> {
        self.events
            .get(..position)
            .map(Self::replay)
            .ok_or(CeremonyError::InvalidHistoryPosition)
    }

    /// Validates and records a semantic command.
    ///
    /// # Errors
    ///
    /// Returns a [`CeremonyError`] when the command would violate the current
    /// phase or a domain invariant.
    pub fn handle(&mut self, command: Command) -> Result<(), CeremonyError> {
        let state = self.state();
        let event = Self::decide(&state, command)?;
        self.events.push(event);
        Ok(())
    }

    fn replay(events: &[Event]) -> CeremonyState {
        let mut state = CeremonyState::default();
        for event in events {
            state.apply(event);
        }
        state
    }

    fn decide(state: &CeremonyState, command: Command) -> Result<Event, CeremonyError> {
        if state.is_cancelled() {
            return Err(CeremonyError::Cancelled);
        }
        match command {
            Command::SelectTarget(target) => Self::select_target(state, target),
            Command::ReopenTargetSelection => Self::reopen_target_selection(state),
            Command::SelectProtocol(protocol) => Self::select_protocol(state, protocol),
            Command::ReopenProtocolSelection => Self::reopen_protocol_selection(state),
            Command::AcknowledgeSafety => Self::acknowledge_safety(state),
            Command::RecordRoll(face) => Self::record_roll(state, face),
            Command::RecordFlip(flip) => Self::record_flip(state, flip),
            Command::RecordJadeD16(face) => Self::record_jade_d16(state, face),
            Command::RecordJadeD8(face) => Self::record_jade_d8(state, face),
            Command::RecordBitBoxD6(face) => Self::record_bitbox_d6(state, face),
            Command::RecordBitBoxCoin(flip) => Self::record_bitbox_coin(state, flip),
            Command::RecordCoinFourD6Coin(flip) => Self::record_coin_four_d6_coin(state, flip),
            Command::RecordCoinFourD6D6(face) => Self::record_coin_four_d6_d6(state, face),
            Command::RecordD20(face) => Self::record_d20(state, face),
            Command::UndoRoll => Self::undo_roll(state),
            Command::UndoFlip => Self::undo_flip(state),
            Command::UndoJade => Self::undo_jade(state),
            Command::UndoBitBox => Self::undo_bitbox(state),
            Command::UndoCoinFourD6 => Self::undo_coin_four_d6(state),
            Command::UndoD20 => Self::undo_d20(state),
            Command::ConfirmRolls => Self::confirm_rolls(state),
            Command::RestartExactAttempt => Self::restart_exact_attempt(state),
            Command::RevealMnemonic => Self::reveal_mnemonic(state),
        }
    }

    fn select_target(state: &CeremonyState, target: EntropyTarget) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::ChooseTarget {
            Ok(Event::TargetSelected(target))
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    fn reopen_target_selection(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::ChooseProtocol {
            Ok(Event::TargetSelectionReopened)
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    fn select_protocol(
        state: &CeremonyState,
        protocol: ConversionProtocol,
    ) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::ChooseProtocol {
            return Err(CeremonyError::WrongPhase);
        }
        let target = state.target().ok_or(CeremonyError::WrongPhase)?;
        if !protocol.supports_target(target) {
            return Err(CeremonyError::UnsupportedTarget);
        }
        Ok(Event::ProtocolSelected(protocol))
    }

    fn reopen_protocol_selection(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::Safety {
            Ok(Event::ProtocolSelectionReopened)
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    fn acknowledge_safety(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::Safety {
            Ok(Event::SafetyAcknowledged)
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    fn record_roll(state: &CeremonyState, face: DieFace) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || matches!(
                state.protocol(),
                Some(
                    ConversionProtocol::SeedSignerCoinsV1
                        | ConversionProtocol::JadeDirectV1
                        | ConversionProtocol::BitBox02DirectV1
                        | ConversionProtocol::KruxD20V1
                        | ConversionProtocol::CoinFourD6DirectV1
                )
            )
        {
            return Err(CeremonyError::WrongPhase);
        }
        let Some(assessment) = state.capture_assessment() else {
            return Err(CeremonyError::WrongPhase);
        };
        if !assessment.can_record_more() {
            return Err(CeremonyError::RollLimitReached);
        }
        Ok(Event::RollRecorded(face))
    }

    fn record_flip(state: &CeremonyState, flip: CoinFlip) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::SeedSignerCoinsV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        let Some(assessment) = state.capture_assessment() else {
            return Err(CeremonyError::WrongPhase);
        };
        if !assessment.can_record_more() {
            return Err(CeremonyError::RollLimitReached);
        }
        Ok(Event::FlipRecorded(flip))
    }

    fn record_jade_d16(state: &CeremonyState, face: D16Face) -> Result<Event, CeremonyError> {
        Self::validate_jade_record(state, JadeDieKind::D16)?;
        Ok(Event::JadeD16Recorded(face))
    }

    fn record_jade_d8(state: &CeremonyState, face: D8Face) -> Result<Event, CeremonyError> {
        Self::validate_jade_record(state, JadeDieKind::D8)?;
        Ok(Event::JadeD8Recorded(face))
    }

    fn validate_jade_record(state: &CeremonyState, kind: JadeDieKind) -> Result<(), CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::JadeDirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        let target = state.target().ok_or(CeremonyError::WrongPhase)?;
        let expected = jade_expected_die(target, state.jade().len());
        match expected {
            Some(expected) if expected == kind => Ok(()),
            Some(_) => Err(CeremonyError::UnexpectedDie),
            None => Err(CeremonyError::RollLimitReached),
        }
    }

    fn record_bitbox_d6(state: &CeremonyState, face: DieFace) -> Result<Event, CeremonyError> {
        Self::validate_bitbox_record(state, BitBoxObservationKind::D6)?;
        Ok(Event::BitBoxD6Recorded(face))
    }

    fn record_bitbox_coin(state: &CeremonyState, flip: CoinFlip) -> Result<Event, CeremonyError> {
        Self::validate_bitbox_record(state, BitBoxObservationKind::Coin)?;
        Ok(Event::BitBoxCoinRecorded(flip))
    }

    fn validate_bitbox_record(
        state: &CeremonyState,
        kind: BitBoxObservationKind,
    ) -> Result<(), CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::BitBox02DirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        let target = state.target().ok_or(CeremonyError::WrongPhase)?;
        match bitbox_progress(target, state.bitbox()).expected_kind() {
            Some(expected) if expected == kind => Ok(()),
            Some(_) => Err(CeremonyError::UnexpectedObservation),
            None => Err(CeremonyError::RollLimitReached),
        }
    }

    fn record_coin_four_d6_coin(
        state: &CeremonyState,
        flip: CoinFlip,
    ) -> Result<Event, CeremonyError> {
        Self::validate_coin_four_d6_record(state, CoinFourD6ObservationKind::Coin)?;
        Ok(Event::CoinFourD6CoinRecorded(flip))
    }

    fn record_coin_four_d6_d6(
        state: &CeremonyState,
        face: DieFace,
    ) -> Result<Event, CeremonyError> {
        Self::validate_coin_four_d6_record(state, CoinFourD6ObservationKind::D6)?;
        Ok(Event::CoinFourD6D6Recorded(face))
    }

    fn validate_coin_four_d6_record(
        state: &CeremonyState,
        kind: CoinFourD6ObservationKind,
    ) -> Result<(), CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::CoinFourD6DirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        let target = state.target().ok_or(CeremonyError::WrongPhase)?;
        match coin_four_d6_progress(target, state.coin_four_d6()).expected_kind() {
            Some(expected) if expected == kind => Ok(()),
            Some(_) => Err(CeremonyError::UnexpectedObservation),
            None => Err(CeremonyError::RollLimitReached),
        }
    }

    fn record_d20(state: &CeremonyState, face: D20Face) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::KruxD20V1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        let Some(assessment) = state.capture_assessment() else {
            return Err(CeremonyError::WrongPhase);
        };
        if !assessment.can_record_more() {
            return Err(CeremonyError::RollLimitReached);
        }
        Ok(Event::D20Recorded(face))
    }

    fn undo_roll(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || matches!(
                state.protocol(),
                Some(
                    ConversionProtocol::JadeDirectV1
                        | ConversionProtocol::BitBox02DirectV1
                        | ConversionProtocol::SeedSignerCoinsV1
                        | ConversionProtocol::KruxD20V1
                        | ConversionProtocol::CoinFourD6DirectV1
                )
            )
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.rolls().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::RollUndone)
        }
    }

    fn undo_flip(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::SeedSignerCoinsV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.flips().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::FlipUndone)
        }
    }

    fn undo_jade(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::JadeDirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.jade().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::JadeUndone)
        }
    }

    fn undo_bitbox(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::BitBox02DirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.bitbox().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::BitBoxUndone)
        }
    }

    fn undo_coin_four_d6(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::CoinFourD6DirectV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.coin_four_d6().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::CoinFourD6Undone)
        }
    }

    fn undo_d20(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls
            || state.protocol() != Some(ConversionProtocol::KruxD20V1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        if state.d20().is_empty() {
            Err(CeremonyError::NoRollsToUndo)
        } else {
            Ok(Event::D20Undone)
        }
    }

    pub(crate) fn validate_roll_confirmation(&self) -> Result<(), CeremonyError> {
        Self::confirm_rolls(&self.state()).map(drop)
    }

    fn confirm_rolls(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() != Phase::EnterRolls {
            return Err(CeremonyError::WrongPhase);
        }
        if state.can_confirm_rolls() {
            Ok(Event::RollsConfirmed)
        } else {
            Err(CeremonyError::InsufficientRolls)
        }
    }

    pub(crate) fn record_generation_succeeded(&mut self) -> Result<(), CeremonyError> {
        let state = self.state();
        if state.phase() != Phase::ReadyToGenerate {
            return Err(CeremonyError::WrongPhase);
        }
        self.events.push(Event::GenerationSucceeded);
        Ok(())
    }

    pub(crate) fn record_exact_attempt_rejected(&mut self) -> Result<(), CeremonyError> {
        let state = self.state();
        if state.phase() != Phase::ReadyToGenerate
            || state.protocol() != Some(ConversionProtocol::ExactV1)
        {
            return Err(CeremonyError::WrongPhase);
        }
        self.events.push(Event::ExactAttemptRejected);
        Ok(())
    }

    fn restart_exact_attempt(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::ExactAttemptRejected {
            Ok(Event::ExactAttemptRestarted)
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    fn reveal_mnemonic(state: &CeremonyState) -> Result<Event, CeremonyError> {
        if state.phase() == Phase::Result {
            Ok(Event::MnemonicRevealed)
        } else {
            Err(CeremonyError::WrongPhase)
        }
    }

    pub(crate) fn verify_mnemonic_backup(
        &mut self,
        _receipt: crate::domain::bip39::VerifiedMnemonicBackup,
    ) -> Result<(), CeremonyError> {
        let state = self.state();
        if state.phase() != Phase::Revealed || state.mnemonic_backup_verified() {
            return Err(CeremonyError::WrongPhase);
        }
        self.events.push(Event::MnemonicBackupVerified);
        Ok(())
    }
}

impl Drop for Ceremony {
    fn drop(&mut self) {
        self.events.zeroize();
    }
}

impl fmt::Debug for Ceremony {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Ceremony")
            .field("event_count", &self.events.len())
            .field("events", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::Ceremony;
    use crate::domain::{
        bip39::{BackupVerifier, EntropyTarget, VerifiedMnemonicBackup, WordSubmission},
        ceremony::{CeremonyError, Command, Event, Phase},
        coin::CoinFlip,
        d20::D20Face,
        dice::DieFace,
        jade::{D8Face, D16Face},
        protocol::{
            BitBoxStage, CaptureAssessment, CaptureProgress, CoinFourD6Stage, ConversionProtocol,
            JadeDieKind, WordExactProgress, bitbox_progress, coin_four_d6_progress,
            jade_expected_die,
        },
    };

    fn verified_backup() -> VerifiedMnemonicBackup {
        use bip39_ceremony_core::{CalculationOutcome, Capture, RollSequence, calculate};

        let mut rolls = RollSequence::new();
        for _ in 0..50 {
            rolls.push(DieFace::new(1).unwrap());
        }
        let CalculationOutcome::Accepted(calculation) = calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ExactV1,
            Capture::Dice(&rolls),
        )
        .unwrap() else {
            panic!("zero exact value is accepted");
        };
        let mut verifier = BackupVerifier::new();
        for word in calculation.mnemonic().words() {
            for character in word.chars() {
                verifier.push(character);
            }
            if let WordSubmission::Complete(receipt) = verifier.submit(calculation.mnemonic()) {
                return receipt;
            }
        }
        panic!("complete verification receipt expected")
    }

    fn collecting_word_progress(ceremony: &Ceremony) -> WordExactProgress {
        let Some(CaptureAssessment::Collecting(CaptureProgress::WordExact(progress))) =
            ceremony.state().capture_assessment()
        else {
            panic!("collecting word-exact progress expected");
        };
        progress
    }

    fn configured(mode: ConversionProtocol) -> Ceremony {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony.handle(Command::SelectProtocol(mode)).unwrap();
        ceremony.handle(Command::AcknowledgeSafety).unwrap();
        ceremony
    }

    fn add_rolls(ceremony: &mut Ceremony, count: usize) {
        let face = DieFace::new(4).unwrap();
        for _ in 0..count {
            ceremony.handle(Command::RecordRoll(face)).unwrap();
        }
    }

    fn add_bitbox_zero_capture(ceremony: &mut Ceremony) {
        let heads = CoinFlip::new(1).unwrap();
        for _ in 0..11 {
            for _ in 0..5 {
                ceremony
                    .handle(Command::RecordBitBoxD6(DieFace::new(1).unwrap()))
                    .unwrap();
            }
            ceremony.handle(Command::RecordBitBoxCoin(heads)).unwrap();
        }
        for _ in 0..7 {
            ceremony.handle(Command::RecordBitBoxCoin(heads)).unwrap();
        }
    }

    #[test]
    fn happy_path_emits_facts_and_derives_state() {
        let ceremony = configured(ConversionProtocol::ExactV1);
        let state = ceremony.state();

        assert_eq!(state.phase(), Phase::EnterRolls);
        assert_eq!(state.target(), Some(EntropyTarget::Words12));
        assert_eq!(state.protocol(), Some(ConversionProtocol::ExactV1));
        assert_eq!(ceremony.events().len(), 3);
    }

    #[test]
    fn commands_are_rejected_out_of_phase_without_new_events() {
        let mut ceremony = Ceremony::new();
        let before = ceremony.events().len();

        assert_eq!(
            ceremony.handle(Command::AcknowledgeSafety),
            Err(CeremonyError::WrongPhase)
        );
        assert_eq!(ceremony.events().len(), before);
    }

    #[test]
    fn target_selection_can_be_reopened_before_mode_selection() {
        let mut ceremony = Ceremony::new();
        assert_eq!(
            ceremony.handle(Command::ReopenTargetSelection),
            Err(CeremonyError::WrongPhase)
        );
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words24))
            .unwrap();
        ceremony.handle(Command::ReopenTargetSelection).unwrap();

        assert_eq!(ceremony.state().phase(), Phase::ChooseTarget);
        assert_eq!(ceremony.state().target(), None);
        assert_eq!(
            ceremony.events().last(),
            Some(&Event::TargetSelectionReopened)
        );
    }

    #[test]
    fn mode_selection_can_be_reopened_before_safety_acknowledgement() {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony
            .handle(Command::SelectProtocol(ConversionProtocol::NativeHashV1))
            .unwrap();
        ceremony.handle(Command::ReopenProtocolSelection).unwrap();

        assert_eq!(ceremony.state().phase(), Phase::ChooseProtocol);
        assert_eq!(ceremony.state().target(), Some(EntropyTarget::Words12));
        assert_eq!(ceremony.state().protocol(), None);
        assert_eq!(
            ceremony.events().last(),
            Some(&Event::ProtocolSelectionReopened)
        );
    }

    #[test]
    fn reopening_target_is_rejected_after_leaving_mode_selection() {
        let mut safety = Ceremony::new();
        safety
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        safety
            .handle(Command::SelectProtocol(ConversionProtocol::NativeHashV1))
            .unwrap();
        assert_eq!(safety.state().phase(), Phase::Safety);
        let before = safety.events().len();
        assert_eq!(
            safety.handle(Command::ReopenTargetSelection),
            Err(CeremonyError::WrongPhase)
        );
        assert_eq!(safety.events().len(), before);

        let mut rolling = configured(ConversionProtocol::ExactV1);
        assert_eq!(
            rolling.handle(Command::ReopenTargetSelection),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn reopening_mode_is_rejected_outside_safety() {
        let mut fresh = Ceremony::new();
        assert_eq!(
            fresh.handle(Command::ReopenProtocolSelection),
            Err(CeremonyError::WrongPhase)
        );
        fresh
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        assert_eq!(fresh.state().phase(), Phase::ChooseProtocol);
        assert_eq!(
            fresh.handle(Command::ReopenProtocolSelection),
            Err(CeremonyError::WrongPhase)
        );

        let mut rolling = configured(ConversionProtocol::ExactV1);
        assert_eq!(
            rolling.handle(Command::ReopenProtocolSelection),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn reopened_target_can_be_reselected_and_advances_again() {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words24))
            .unwrap();
        ceremony.handle(Command::ReopenTargetSelection).unwrap();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();

        assert_eq!(ceremony.state().phase(), Phase::ChooseProtocol);
        assert_eq!(ceremony.state().target(), Some(EntropyTarget::Words12));
        assert_eq!(ceremony.state().protocol(), None);
    }

    #[test]
    fn selection_commands_reject_repeat_or_out_of_phase_use() {
        let mut ceremony = Ceremony::new();
        assert_eq!(
            ceremony.handle(Command::SelectProtocol(ConversionProtocol::ExactV1)),
            Err(CeremonyError::WrongPhase)
        );
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        assert_eq!(
            ceremony.handle(Command::SelectTarget(EntropyTarget::Words24)),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn protocol_selection_rejects_an_unsupported_target() {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();

        assert_eq!(
            ceremony.handle(Command::SelectProtocol(
                ConversionProtocol::KeystoneLegacyV1
            )),
            Err(CeremonyError::UnsupportedTarget)
        );
        assert_eq!(ceremony.state().phase(), Phase::ChooseProtocol);
    }

    #[test]
    fn seedsigner_accepts_only_the_fixed_coin_capture() {
        let mut ceremony = configured(ConversionProtocol::SeedSignerCoinsV1);
        assert_eq!(
            ceremony.handle(Command::RecordRoll(DieFace::new(1).unwrap())),
            Err(CeremonyError::WrongPhase)
        );
        for _ in 0..128 {
            ceremony
                .handle(Command::RecordFlip(CoinFlip::new(0).unwrap()))
                .unwrap();
        }
        assert!(ceremony.state().can_confirm_rolls());
        assert_eq!(
            ceremony.handle(Command::RecordFlip(CoinFlip::new(1).unwrap())),
            Err(CeremonyError::RollLimitReached)
        );
        ceremony.handle(Command::UndoFlip).unwrap();
        assert!(!ceremony.state().can_confirm_rolls());
    }

    #[test]
    fn roll_actions_require_the_roll_phase() {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        let face = DieFace::new(3).unwrap();
        assert_eq!(
            ceremony.handle(Command::RecordRoll(face)),
            Err(CeremonyError::WrongPhase)
        );
        assert_eq!(
            ceremony.handle(Command::UndoRoll),
            Err(CeremonyError::WrongPhase)
        );
        assert_eq!(
            ceremony.handle(Command::ConfirmRolls),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn generation_and_reveal_require_their_phases() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        assert_eq!(
            ceremony.record_generation_succeeded(),
            Err(CeremonyError::WrongPhase)
        );
        assert_eq!(
            ceremony.handle(Command::RevealMnemonic),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn restarting_exact_requires_a_rejected_attempt() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        assert_eq!(
            ceremony.handle(Command::RestartExactAttempt),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn cancellation_is_available_from_the_first_phase() {
        let mut ceremony = Ceremony::new();
        ceremony.cancel().unwrap();
        assert_eq!(ceremony.state().phase(), Phase::Cancelled);
    }

    #[test]
    fn full_prefix_position_reconstructs_the_live_state() {
        let ceremony = configured(ConversionProtocol::ExactV1);
        let full = ceremony.state_at(ceremony.events().len()).unwrap();
        assert_eq!(full.phase(), ceremony.state().phase());
        assert_eq!(full.target(), ceremony.state().target());
        assert_eq!(full.protocol(), ceremony.state().protocol());
    }

    #[test]
    fn undo_is_appended_and_reverses_the_derived_roll_state() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        add_rolls(&mut ceremony, 2);
        ceremony.handle(Command::UndoRoll).unwrap();

        assert_eq!(ceremony.state().rolls().len(), 1);
        assert_eq!(ceremony.events().last(), Some(&Event::RollUndone));
        assert_eq!(ceremony.events().len(), 6);
    }

    #[test]
    fn undo_without_rolls_is_rejected() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        assert_eq!(
            ceremony.handle(Command::UndoRoll),
            Err(CeremonyError::NoRollsToUndo)
        );
    }

    #[test]
    fn exact_mode_requires_and_limits_exact_roll_count() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        add_rolls(&mut ceremony, 49);
        assert_eq!(
            ceremony.handle(Command::ConfirmRolls),
            Err(CeremonyError::InsufficientRolls)
        );

        add_rolls(&mut ceremony, 1);
        assert_eq!(
            ceremony.handle(Command::RecordRoll(DieFace::new(1).unwrap())),
            Err(CeremonyError::RollLimitReached)
        );
        assert_eq!(ceremony.handle(Command::ConfirmRolls), Ok(()));
        assert_eq!(ceremony.state().phase(), Phase::ReadyToGenerate);
    }

    #[test]
    fn word_exact_undo_recomputes_candidate_boundaries() {
        let mut ceremony = configured(ConversionProtocol::WordExactV1);
        let six = DieFace::new(6).unwrap();
        for _ in 0..6 {
            ceremony.handle(Command::RecordRoll(six)).unwrap();
        }
        ceremony
            .handle(Command::RecordRoll(DieFace::new(1).unwrap()))
            .unwrap();

        ceremony.handle(Command::UndoRoll).unwrap();
        let after_partial_undo = collecting_word_progress(&ceremony);
        assert_eq!(after_partial_undo.rejected_candidates(), 1);
        assert!(matches!(
            after_partial_undo,
            WordExactProgress::WordCandidate {
                candidate_rolls: 0,
                ..
            }
        ));

        ceremony.handle(Command::UndoRoll).unwrap();
        let inside_prior_group = collecting_word_progress(&ceremony);
        assert_eq!(inside_prior_group.rejected_candidates(), 0);
        assert!(matches!(
            inside_prior_group,
            WordExactProgress::WordCandidate {
                candidate_rolls: 5,
                ..
            }
        ));
    }

    #[test]
    fn word_exact_allows_local_rejection_beyond_the_minimum() {
        let mut ceremony = configured(ConversionProtocol::WordExactV1);
        let six = DieFace::new(6).unwrap();
        for _ in 0..6 {
            ceremony.handle(Command::RecordRoll(six)).unwrap();
        }
        add_rolls(&mut ceremony, 70);

        assert_eq!(ceremony.state().rolls().len(), 76);
        assert!(ceremony.state().can_confirm_rolls());
        assert_eq!(
            ceremony.handle(Command::RecordRoll(six)),
            Err(CeremonyError::RollLimitReached)
        );
    }

    #[test]
    fn jade_capture_enforces_die_order_and_undo() {
        let mut ceremony = configured(ConversionProtocol::JadeDirectV1);
        assert_eq!(
            ceremony.handle(Command::RecordJadeD8(D8Face::new(1).unwrap())),
            Err(CeremonyError::UnexpectedDie)
        );
        ceremony
            .handle(Command::RecordJadeD16(D16Face::new(1).unwrap()))
            .unwrap();
        ceremony.handle(Command::UndoJade).unwrap();
        assert!(ceremony.state().jade().is_empty());

        for offset in 0..35 {
            match jade_expected_die(EntropyTarget::Words12, offset).unwrap() {
                JadeDieKind::D16 => ceremony
                    .handle(Command::RecordJadeD16(D16Face::new(1).unwrap()))
                    .unwrap(),
                JadeDieKind::D8 => ceremony
                    .handle(Command::RecordJadeD8(D8Face::new(1).unwrap()))
                    .unwrap(),
            }
        }
        assert!(ceremony.state().can_confirm_rolls());
        assert_eq!(ceremony.state().jade().len(), 35);
    }

    #[test]
    fn bitbox_capture_rejects_kinds_and_retries_only_local_d6_faces() {
        let mut ceremony = configured(ConversionProtocol::BitBox02DirectV1);
        let heads = CoinFlip::new(1).unwrap();
        assert_eq!(
            ceremony.handle(Command::RecordBitBoxCoin(heads)),
            Err(CeremonyError::UnexpectedObservation)
        );

        ceremony
            .handle(Command::RecordBitBoxD6(DieFace::new(6).unwrap()))
            .unwrap();
        let state = ceremony.state();
        let progress = bitbox_progress(EntropyTarget::Words12, state.bitbox());
        assert_eq!(progress.rejected_faces(), 1);
        assert!(matches!(progress.stage(), BitBoxStage::DirectWordD6 { .. }));
        ceremony.handle(Command::UndoBitBox).unwrap();
        assert!(ceremony.state().bitbox().is_empty());

        add_bitbox_zero_capture(&mut ceremony);
        assert!(ceremony.state().can_confirm_rolls());
        assert_eq!(ceremony.state().bitbox().len(), 73);
        assert_eq!(
            ceremony.handle(Command::RecordBitBoxCoin(heads)),
            Err(CeremonyError::RollLimitReached)
        );
    }

    #[test]
    fn coin_four_d6_capture_rejects_whole_candidates_and_replays_undo() {
        let mut ceremony = configured(ConversionProtocol::CoinFourD6DirectV1);
        ceremony
            .handle(Command::RecordCoinFourD6Coin(CoinFlip::new(0).unwrap()))
            .unwrap();
        for value in [4, 3, 6, 3] {
            ceremony
                .handle(Command::RecordCoinFourD6D6(DieFace::new(value).unwrap()))
                .unwrap();
        }
        let state = ceremony.state();
        let progress = coin_four_d6_progress(EntropyTarget::Words12, state.coin_four_d6());
        assert_eq!(progress.rejected_candidates(), 1);
        assert_eq!(progress.stage(), CoinFourD6Stage::Coin { position: 1 });

        ceremony.handle(Command::UndoCoinFourD6).unwrap();
        let state = ceremony.state();
        assert_eq!(
            coin_four_d6_progress(EntropyTarget::Words12, state.coin_four_d6()).stage(),
            CoinFourD6Stage::D6 {
                position: 1,
                recorded: 3,
            }
        );
        assert!(matches!(
            ceremony.handle(Command::RecordCoinFourD6Coin(CoinFlip::new(1).unwrap())),
            Err(CeremonyError::UnexpectedObservation)
        ));
    }

    #[test]
    fn krux_d20_capture_supports_undo_minimum_and_extra_rolls() {
        let mut ceremony = configured(ConversionProtocol::KruxD20V1);
        ceremony
            .handle(Command::RecordD20(D20Face::new(20).unwrap()))
            .unwrap();
        ceremony.handle(Command::UndoD20).unwrap();
        assert!(ceremony.state().d20().is_empty());

        for _ in 0..30 {
            ceremony
                .handle(Command::RecordD20(D20Face::new(1).unwrap()))
                .unwrap();
        }
        assert!(ceremony.state().can_confirm_rolls());
        ceremony
            .handle(Command::RecordD20(D20Face::new(20).unwrap()))
            .unwrap();
        assert_eq!(ceremony.state().d20().len(), 31);
        assert_eq!(ceremony.handle(Command::ConfirmRolls), Ok(()));
    }

    #[test]
    fn coldcard_mode_allows_rolls_beyond_the_documented_minimum() {
        let mut ceremony = configured(ConversionProtocol::ColdcardV1);
        add_rolls(&mut ceremony, 51);
        assert!(ceremony.state().can_confirm_rolls());
        assert_eq!(ceremony.handle(Command::ConfirmRolls), Ok(()));
    }

    #[test]
    fn successful_generation_can_be_revealed() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        add_rolls(&mut ceremony, 50);
        ceremony.handle(Command::ConfirmRolls).unwrap();
        ceremony.record_generation_succeeded().unwrap();
        assert_eq!(ceremony.state().phase(), Phase::Result);

        ceremony.handle(Command::RevealMnemonic).unwrap();
        assert_eq!(ceremony.state().phase(), Phase::Revealed);
        assert!(!ceremony.state().mnemonic_backup_verified());

        ceremony.verify_mnemonic_backup(verified_backup()).unwrap();
        assert_eq!(ceremony.state().phase(), Phase::Revealed);
        assert!(ceremony.state().mnemonic_backup_verified());
        assert_eq!(
            ceremony.verify_mnemonic_backup(verified_backup()),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn exact_rejection_preserves_history_and_starts_fresh_attempt() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        add_rolls(&mut ceremony, 50);
        ceremony.handle(Command::ConfirmRolls).unwrap();
        ceremony.record_exact_attempt_rejected().unwrap();
        assert_eq!(ceremony.state().phase(), Phase::ExactAttemptRejected);

        ceremony.handle(Command::RestartExactAttempt).unwrap();
        let state = ceremony.state();
        assert_eq!(state.phase(), Phase::EnterRolls);
        assert!(state.rolls().is_empty());
        assert!(ceremony.events().len() > 50);
    }

    #[test]
    fn hash_mode_cannot_record_exact_rejection() {
        let mut ceremony = configured(ConversionProtocol::NativeHashV1);
        add_rolls(&mut ceremony, 50);
        ceremony.handle(Command::ConfirmRolls).unwrap();
        assert_eq!(
            ceremony.record_exact_attempt_rejected(),
            Err(CeremonyError::WrongPhase)
        );
    }

    #[test]
    fn event_prefix_reconstructs_prior_state() {
        let mut ceremony = configured(ConversionProtocol::NativeHashV1);
        add_rolls(&mut ceremony, 2);

        let before_rolls = ceremony.state_at(3).unwrap();
        let after_first = ceremony.state_at(4).unwrap();
        assert_eq!(before_rolls.rolls().len(), 0);
        assert_eq!(after_first.rolls().len(), 1);
        assert_eq!(ceremony.state().rolls().len(), 2);
    }

    #[test]
    fn invalid_history_position_is_explicit() {
        let ceremony = Ceremony::new();
        assert!(matches!(
            ceremony.state_at(1),
            Err(CeremonyError::InvalidHistoryPosition)
        ));
    }

    #[test]
    fn cancellation_is_terminal() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        add_rolls(&mut ceremony, 2);
        ceremony.cancel().unwrap();

        let state = ceremony.state();
        assert_eq!(state.phase(), Phase::Cancelled);
        assert!(state.rolls().is_empty());
        assert_eq!(ceremony.events(), [Event::CeremonyCancelled]);
        assert_eq!(
            ceremony.handle(Command::UndoRoll),
            Err(CeremonyError::Cancelled)
        );
    }

    #[test]
    fn debug_output_redacts_event_payloads() {
        let mut ceremony = configured(ConversionProtocol::ExactV1);
        ceremony
            .handle(Command::RecordRoll(DieFace::new(5).unwrap()))
            .unwrap();

        assert_eq!(
            format!("{:?}", ceremony.events().last().unwrap()),
            "RollRecorded([REDACTED])"
        );
        let aggregate = format!("{ceremony:?}");
        assert!(aggregate.contains("REDACTED"));
        assert!(!aggregate.contains('5'));
    }
}
