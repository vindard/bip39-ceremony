use core::fmt;

use zeroize::Zeroize;

use crate::domain::{
    bip39::EntropyTarget,
    coin::CoinFlip,
    d20::D20Face,
    dice::DieFace,
    jade::{D8Face, D16Face},
    protocol::ConversionProtocol,
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
    JadeD16Recorded(D16Face),
    JadeD8Recorded(D8Face),
    BitBoxD6Recorded(DieFace),
    BitBoxCoinRecorded(CoinFlip),
    CoinFourD6CoinRecorded(CoinFlip),
    CoinFourD6D6Recorded(DieFace),
    D20Recorded(D20Face),
    RollUndone,
    FlipUndone,
    JadeUndone,
    BitBoxUndone,
    CoinFourD6Undone,
    D20Undone,
    RollsConfirmed,
    GenerationSucceeded,
    AttemptRejected,
    AttemptRestarted,
    MnemonicRevealed,
    MnemonicBackupVerified,
    CeremonyCancelled,
}

impl Zeroize for Event {
    fn zeroize(&mut self) {
        match self {
            Self::RollRecorded(face)
            | Self::BitBoxD6Recorded(face)
            | Self::CoinFourD6D6Recorded(face) => face.zeroize(),
            Self::FlipRecorded(flip)
            | Self::BitBoxCoinRecorded(flip)
            | Self::CoinFourD6CoinRecorded(flip) => flip.zeroize(),
            Self::JadeD16Recorded(face) => face.zeroize(),
            Self::JadeD8Recorded(face) => face.zeroize(),
            Self::D20Recorded(face) => face.zeroize(),
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
            Self::JadeD16Recorded(_) => formatter.write_str("JadeD16Recorded([REDACTED])"),
            Self::JadeD8Recorded(_) => formatter.write_str("JadeD8Recorded([REDACTED])"),
            Self::BitBoxD6Recorded(_) => formatter.write_str("BitBoxD6Recorded([REDACTED])"),
            Self::BitBoxCoinRecorded(_) => formatter.write_str("BitBoxCoinRecorded([REDACTED])"),
            Self::CoinFourD6CoinRecorded(_) => {
                formatter.write_str("CoinFourD6CoinRecorded([REDACTED])")
            }
            Self::CoinFourD6D6Recorded(_) => {
                formatter.write_str("CoinFourD6D6Recorded([REDACTED])")
            }
            Self::D20Recorded(_) => formatter.write_str("D20Recorded([REDACTED])"),
            Self::RollUndone => formatter.write_str("RollUndone"),
            Self::FlipUndone => formatter.write_str("FlipUndone"),
            Self::JadeUndone => formatter.write_str("JadeUndone"),
            Self::BitBoxUndone => formatter.write_str("BitBoxUndone"),
            Self::CoinFourD6Undone => formatter.write_str("CoinFourD6Undone"),
            Self::D20Undone => formatter.write_str("D20Undone"),
            Self::RollsConfirmed => formatter.write_str("RollsConfirmed"),
            Self::GenerationSucceeded => formatter.write_str("GenerationSucceeded"),
            Self::AttemptRejected => formatter.write_str("AttemptRejected"),
            Self::AttemptRestarted => formatter.write_str("AttemptRestarted"),
            Self::MnemonicRevealed => formatter.write_str("MnemonicRevealed"),
            Self::MnemonicBackupVerified => formatter.write_str("MnemonicBackupVerified"),
            Self::CeremonyCancelled => formatter.write_str("CeremonyCancelled"),
        }
    }
}
