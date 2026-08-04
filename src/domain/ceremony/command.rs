use core::fmt;

use crate::domain::{
    bip39::EntropyTarget, coin::CoinFlip, dice::DieFace, protocol::ConversionProtocol,
};

/// Semantic intent accepted by the ceremony aggregate.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Command {
    SelectTarget(EntropyTarget),
    ReopenTargetSelection,
    SelectProtocol(ConversionProtocol),
    ReopenProtocolSelection,
    AcknowledgeSafety,
    RecordRoll(DieFace),
    RecordFlip(CoinFlip),
    UndoRoll,
    UndoFlip,
    ConfirmRolls,
    RestartExactAttempt,
    RevealMnemonic,
}

impl fmt::Debug for Command {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectTarget(target) => {
                formatter.debug_tuple("SelectTarget").field(target).finish()
            }
            Self::ReopenTargetSelection => formatter.write_str("ReopenTargetSelection"),
            Self::SelectProtocol(protocol) => formatter
                .debug_tuple("SelectProtocol")
                .field(protocol)
                .finish(),
            Self::ReopenProtocolSelection => formatter.write_str("ReopenProtocolSelection"),
            Self::AcknowledgeSafety => formatter.write_str("AcknowledgeSafety"),
            Self::RecordRoll(_) => formatter.write_str("RecordRoll([REDACTED])"),
            Self::RecordFlip(_) => formatter.write_str("RecordFlip([REDACTED])"),
            Self::UndoRoll => formatter.write_str("UndoRoll"),
            Self::UndoFlip => formatter.write_str("UndoFlip"),
            Self::ConfirmRolls => formatter.write_str("ConfirmRolls"),
            Self::RestartExactAttempt => formatter.write_str("RestartExactAttempt"),
            Self::RevealMnemonic => formatter.write_str("RevealMnemonic"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_roll_face() {
        let command = Command::RecordRoll(DieFace::new(4).unwrap());
        let debug = format!("{command:?}");
        assert_eq!(debug, "RecordRoll([REDACTED])");
        assert!(!debug.contains('4'));
    }
}
