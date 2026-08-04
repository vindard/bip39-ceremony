use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    WrongObservationCount { expected: usize, actual: usize },
    UnsupportedTarget,
    IncompleteWordExact,
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
        }
    }
}

impl std::error::Error for ProtocolError {}
