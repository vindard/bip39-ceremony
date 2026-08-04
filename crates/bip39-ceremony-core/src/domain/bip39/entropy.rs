use core::fmt;

use zeroize::Zeroize;

use super::{Bip39Error, EntropyTarget};

/// Secret BIP-39 entropy with a validated standard length.
#[derive(Eq, PartialEq)]
pub struct Entropy {
    target: EntropyTarget,
    bytes: Vec<u8>,
}

impl Entropy {
    /// Creates entropy matching its BIP-39 target.
    ///
    /// # Errors
    ///
    /// Returns [`Bip39Error::InvalidEntropyLength`] when `bytes` does not
    /// contain exactly 16 or 32 bytes for the selected target.
    pub fn new(target: EntropyTarget, mut bytes: Vec<u8>) -> Result<Self, Bip39Error> {
        if bytes.len() == target.entropy_bytes() {
            Ok(Self { target, bytes })
        } else {
            bytes.zeroize();
            Err(Bip39Error::InvalidEntropyLength)
        }
    }

    pub(crate) fn from_protocol_bytes(target: EntropyTarget, bytes: Vec<u8>) -> Self {
        Self::new(target, bytes).expect("protocol entropy must match its target")
    }

    #[must_use]
    pub const fn target(&self) -> EntropyTarget {
        self.target
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Drop for Entropy {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl fmt::Debug for Entropy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Entropy")
            .field("target", &self.target)
            .field("bytes", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_requires_target_length_and_redacts_debug() {
        assert!(Entropy::new(EntropyTarget::Words12, vec![0; 16]).is_ok());
        assert!(Entropy::new(EntropyTarget::Words24, vec![0; 32]).is_ok());
        assert_eq!(
            Entropy::new(EntropyTarget::Words12, vec![0; 15]).unwrap_err(),
            Bip39Error::InvalidEntropyLength
        );

        let entropy = Entropy::new(EntropyTarget::Words12, vec![42; 16]).unwrap();
        let debug = format!("{entropy:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("42"));
    }
}
