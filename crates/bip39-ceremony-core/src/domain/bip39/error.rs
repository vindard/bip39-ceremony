use core::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Bip39Error {
    InvalidEntropyLength,
    EncodingFailed,
}

impl fmt::Display for Bip39Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEntropyLength => {
                formatter.write_str("entropy length is not valid for target")
            }
            Self::EncodingFailed => formatter.write_str("BIP-39 encoding failed"),
        }
    }
}

impl std::error::Error for Bip39Error {}
