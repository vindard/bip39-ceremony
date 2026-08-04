use core::fmt;

use zeroize::Zeroize;

use crate::domain::{
    bip39::EntropyTarget, coin::CoinFlip, dice::DieFace, protocol::ConversionProtocol,
};

/// An accepted domain fact. Secret-bearing variants redact their debug output.
#[derive(Eq, PartialEq)]
pub(crate) enum Event {
    TargetSelected(EntropyTarget),
    TargetSelectionReopened,
    ProtocolSelected(ConversionProtocol),
    ProtocolSelectionReopened,
    SafetyAcknowledged,
    RollRecorded(DieFace),
    FlipRecorded(CoinFlip),
    RollUndone,
    FlipUndone,
    RollsConfirmed,
    GenerationSucceeded,
    ExactAttemptRejected,
    ExactAttemptRestarted,
    MnemonicRevealed,
    MnemonicBackupVerified,
    CeremonyCancelled,
}

impl Zeroize for Event {
    fn zeroize(&mut self) {
        match self {
            Self::RollRecorded(face) => face.zeroize(),
            Self::FlipRecorded(flip) => flip.zeroize(),
            _ => {}
        }
        *self = Self::CeremonyCancelled;
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetSelected(target) => formatter
                .debug_tuple("TargetSelected")
                .field(target)
                .finish(),
            Self::TargetSelectionReopened => formatter.write_str("TargetSelectionReopened"),
            Self::ProtocolSelected(protocol) => formatter
                .debug_tuple("ProtocolSelected")
                .field(protocol)
                .finish(),
            Self::ProtocolSelectionReopened => formatter.write_str("ProtocolSelectionReopened"),
            Self::SafetyAcknowledged => formatter.write_str("SafetyAcknowledged"),
            Self::RollRecorded(_) => formatter.write_str("RollRecorded([REDACTED])"),
            Self::FlipRecorded(_) => formatter.write_str("FlipRecorded([REDACTED])"),
            Self::RollUndone => formatter.write_str("RollUndone"),
            Self::FlipUndone => formatter.write_str("FlipUndone"),
            Self::RollsConfirmed => formatter.write_str("RollsConfirmed"),
            Self::GenerationSucceeded => formatter.write_str("GenerationSucceeded"),
            Self::ExactAttemptRejected => formatter.write_str("ExactAttemptRejected"),
            Self::ExactAttemptRestarted => formatter.write_str("ExactAttemptRestarted"),
            Self::MnemonicRevealed => formatter.write_str("MnemonicRevealed"),
            Self::MnemonicBackupVerified => formatter.write_str("MnemonicBackupVerified"),
            Self::CeremonyCancelled => formatter.write_str("CeremonyCancelled"),
        }
    }
}
