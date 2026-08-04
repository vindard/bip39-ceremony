use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CeremonyError {
    WrongPhase,
    Cancelled,
    RollLimitReached,
    NoRollsToUndo,
    InsufficientRolls,
    UnsupportedTarget,
    InvalidHistoryPosition,
}

impl fmt::Display for CeremonyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::WrongPhase => "that action is not available in the current phase",
            Self::Cancelled => "the ceremony has been cancelled",
            Self::RollLimitReached => "the selected protocol has reached its input limit",
            Self::NoRollsToUndo => "there are no captured outcomes to undo",
            Self::InsufficientRolls => "more physical outcomes are required before review",
            Self::UnsupportedTarget => {
                "the selected protocol does not support this mnemonic length"
            }
            Self::InvalidHistoryPosition => "history position is beyond the event journal",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CeremonyError {}
