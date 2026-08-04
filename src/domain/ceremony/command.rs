use core::fmt;

use crate::domain::{
    bip39::EntropyTarget,
    coin::CoinFlip,
    d20::D20Face,
    dice::DieFace,
    jade::{D8Face, D16Face},
    protocol::ConversionProtocol,
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
    RecordJadeD16(D16Face),
    RecordJadeD8(D8Face),
    RecordBitBoxD6(DieFace),
    RecordBitBoxCoin(CoinFlip),
    RecordD20(D20Face),
    UndoRoll,
    UndoFlip,
    UndoJade,
    UndoBitBox,
    UndoD20,
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
            Self::RecordJadeD16(_) => formatter.write_str("RecordJadeD16([REDACTED])"),
            Self::RecordJadeD8(_) => formatter.write_str("RecordJadeD8([REDACTED])"),
            Self::RecordBitBoxD6(_) => formatter.write_str("RecordBitBoxD6([REDACTED])"),
            Self::RecordBitBoxCoin(_) => formatter.write_str("RecordBitBoxCoin([REDACTED])"),
            Self::RecordD20(_) => formatter.write_str("RecordD20([REDACTED])"),
            Self::UndoRoll => formatter.write_str("UndoRoll"),
            Self::UndoFlip => formatter.write_str("UndoFlip"),
            Self::UndoJade => formatter.write_str("UndoJade"),
            Self::UndoBitBox => formatter.write_str("UndoBitBox"),
            Self::UndoD20 => formatter.write_str("UndoD20"),
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

        let jade = format!("{:?}", Command::RecordJadeD16(D16Face::new(10).unwrap()));
        assert_eq!(jade, "RecordJadeD16([REDACTED])");
        assert!(!jade.contains("10"));

        let bitbox = format!("{:?}", Command::RecordBitBoxD6(DieFace::new(5).unwrap()));
        assert_eq!(bitbox, "RecordBitBoxD6([REDACTED])");
        assert!(!bitbox.contains('5'));

        let d20 = format!("{:?}", Command::RecordD20(D20Face::new(20).unwrap()));
        assert_eq!(d20, "RecordD20([REDACTED])");
    }
}
