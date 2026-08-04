use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    WrongObservationCount { expected: usize, actual: usize },
    UnsupportedTarget,
    IncompleteWordExact,
    WrongObservationKind,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongObservationCount { expected, actual } => {
                write!(
                    formatter,
                    "expected {expected} observations, received {actual}"
                )
            }
            Self::UnsupportedTarget => {
                formatter.write_str("protocol does not support the selected mnemonic length")
            }
            Self::IncompleteWordExact => {
                formatter.write_str("word-by-word exact conversion needs more rolls")
            }
            Self::WrongObservationKind => {
                formatter.write_str("observation die does not match the protocol position")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}
