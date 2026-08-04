use crate::domain::{
    bip39::EntropyTarget,
    bitbox::BitBoxCapture,
    coin::FlipSequence,
    d20::D20RollSequence,
    dice::RollSequence,
    jade::JadeCapture,
    protocol::{CaptureAssessment, ConversionProtocol},
};

use super::Event;

/// User-facing progress through the ceremony.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    ChooseTarget,
    ChooseProtocol,
    Safety,
    EnterRolls,
    ReadyToGenerate,
    ExactAttemptRejected,
    Result,
    Revealed,
    Cancelled,
}

/// State derived solely by folding accepted events.
#[derive(Debug)]
pub struct CeremonyState {
    phase: Phase,
    target: Option<EntropyTarget>,
    protocol: Option<ConversionProtocol>,
    rolls: RollSequence,
    flips: FlipSequence,
    jade: JadeCapture,
    bitbox: BitBoxCapture,
    d20: D20RollSequence,
    safety_acknowledged: bool,
    generation_succeeded: bool,
    mnemonic_backup_verified: bool,
}

impl Default for CeremonyState {
    fn default() -> Self {
        Self {
            phase: Phase::ChooseTarget,
            target: None,
            protocol: None,
            rolls: RollSequence::new(),
            flips: FlipSequence::new(),
            jade: JadeCapture::new(),
            bitbox: BitBoxCapture::new(),
            d20: D20RollSequence::new(),
            safety_acknowledged: false,
            generation_succeeded: false,
            mnemonic_backup_verified: false,
        }
    }
}

impl CeremonyState {
    #[must_use]
    pub const fn target(&self) -> Option<EntropyTarget> {
        self.target
    }

    #[must_use]
    pub const fn protocol(&self) -> Option<ConversionProtocol> {
        self.protocol
    }

    #[must_use]
    pub fn rolls(&self) -> &RollSequence {
        &self.rolls
    }

    #[must_use]
    pub fn flips(&self) -> &FlipSequence {
        &self.flips
    }

    #[must_use]
    pub fn jade(&self) -> &JadeCapture {
        &self.jade
    }

    #[must_use]
    pub fn bitbox(&self) -> &BitBoxCapture {
        &self.bitbox
    }

    #[must_use]
    pub fn d20(&self) -> &D20RollSequence {
        &self.d20
    }

    #[must_use]
    pub fn capture_count(&self) -> usize {
        match self.protocol {
            Some(ConversionProtocol::SeedSignerCoinsV1) => self.flips.len(),
            Some(ConversionProtocol::JadeDirectV1) => self.jade.len(),
            Some(ConversionProtocol::BitBox02DirectV1) => self.bitbox.len(),
            Some(ConversionProtocol::KruxD20V1) => self.d20.len(),
            _ => self.rolls.len(),
        }
    }

    #[must_use]
    pub const fn safety_acknowledged(&self) -> bool {
        self.safety_acknowledged
    }

    #[must_use]
    pub const fn generation_succeeded(&self) -> bool {
        self.generation_succeeded
    }

    #[must_use]
    pub const fn mnemonic_backup_verified(&self) -> bool {
        self.mnemonic_backup_verified
    }

    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        matches!(self.phase, Phase::Cancelled)
    }

    #[must_use]
    pub const fn phase(&self) -> Phase {
        self.phase
    }

    #[must_use]
    pub fn required_rolls(&self) -> Option<usize> {
        self.target
            .zip(self.protocol)
            .map(|(target, protocol)| protocol.minimum_observations(target))
    }

    #[must_use]
    pub fn capture_assessment(&self) -> Option<CaptureAssessment> {
        self.target
            .zip(self.protocol)
            .map(|(target, protocol)| match protocol {
                ConversionProtocol::SeedSignerCoinsV1 => {
                    protocol.assess_coin_capture(target, &self.flips)
                }
                ConversionProtocol::JadeDirectV1 => {
                    protocol.assess_jade_capture(target, &self.jade)
                }
                ConversionProtocol::BitBox02DirectV1 => {
                    protocol.assess_bitbox_capture(target, &self.bitbox)
                }
                ConversionProtocol::KruxD20V1 => protocol.assess_d20_capture(target, &self.d20),
                _ => protocol.assess_capture(target, &self.rolls),
            })
    }

    #[must_use]
    pub fn can_confirm_rolls(&self) -> bool {
        self.capture_assessment()
            .is_some_and(CaptureAssessment::is_complete)
    }

    pub(super) fn apply(&mut self, event: &Event) {
        match event {
            Event::TargetSelected(target) => {
                self.target = Some(*target);
                self.phase = Phase::ChooseProtocol;
            }
            Event::TargetSelectionReopened => {
                self.target = None;
                self.phase = Phase::ChooseTarget;
            }
            Event::ProtocolSelected(protocol) => {
                self.protocol = Some(*protocol);
                self.phase = Phase::Safety;
            }
            Event::ProtocolSelectionReopened => {
                self.protocol = None;
                self.phase = Phase::ChooseProtocol;
            }
            Event::SafetyAcknowledged => {
                self.safety_acknowledged = true;
                self.phase = Phase::EnterRolls;
            }
            Event::RollRecorded(face) => self.rolls.push(*face),
            Event::FlipRecorded(flip) => self.flips.push(*flip),
            Event::JadeD16Recorded(face) => self.jade.push_d16(*face),
            Event::JadeD8Recorded(face) => self.jade.push_d8(*face),
            Event::BitBoxD6Recorded(face) => self.bitbox.push_d6(*face),
            Event::BitBoxCoinRecorded(flip) => self.bitbox.push_coin(*flip),
            Event::D20Recorded(face) => self.d20.push(*face),
            Event::RollUndone => {
                let removed = self.rolls.remove_last();
                assert!(removed, "undo events require an active roll");
            }
            Event::FlipUndone => {
                let removed = self.flips.remove_last();
                assert!(removed, "undo events require an active flip");
            }
            Event::JadeUndone => {
                let removed = self.jade.remove_last();
                assert!(removed, "undo events require an active Jade observation");
            }
            Event::BitBoxUndone => {
                let removed = self.bitbox.remove_last();
                assert!(removed, "undo events require an active BitBox observation");
            }
            Event::D20Undone => {
                let removed = self.d20.remove_last();
                assert!(removed, "undo events require an active D20 roll");
            }
            Event::RollsConfirmed => self.phase = Phase::ReadyToGenerate,
            Event::GenerationSucceeded => {
                self.generation_succeeded = true;
                self.phase = Phase::Result;
            }
            Event::ExactAttemptRejected => self.phase = Phase::ExactAttemptRejected,
            Event::ExactAttemptRestarted => self.restart_attempt(),
            Event::MnemonicRevealed => self.phase = Phase::Revealed,
            Event::MnemonicBackupVerified => {
                self.mnemonic_backup_verified = true;
            }
            Event::CeremonyCancelled => self.phase = Phase::Cancelled,
        }
    }

    fn restart_attempt(&mut self) {
        self.rolls = RollSequence::new();
        self.flips = FlipSequence::new();
        self.jade = JadeCapture::new();
        self.bitbox = BitBoxCapture::new();
        self.d20 = D20RollSequence::new();
        self.phase = Phase::EnterRolls;
    }
}
