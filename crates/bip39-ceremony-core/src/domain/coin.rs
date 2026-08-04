use core::fmt;

use zeroize::{Zeroize, Zeroizing};

/// One secret physical coin observation encoded for `SeedSigner` compatibility.
#[derive(Clone, Copy, Eq, PartialEq, Zeroize)]
pub struct CoinFlip(u8);

impl CoinFlip {
    /// Creates a flip from `SeedSigner`'s binary representation.
    ///
    /// # Errors
    ///
    /// Returns an error unless `value` is zero (tails) or one (heads).
    pub const fn new(value: u8) -> Result<Self, InvalidCoinFlip> {
        if value <= 1 {
            Ok(Self(value))
        } else {
            Err(InvalidCoinFlip)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn ascii(self) -> u8 {
        b'0' + self.0
    }
}

impl TryFrom<char> for CoinFlip {
    type Error = InvalidCoinFlip;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '0' => Ok(Self(0)),
            '1' => Ok(Self(1)),
            _ => Err(InvalidCoinFlip),
        }
    }
}

impl fmt::Debug for CoinFlip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CoinFlip([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidCoinFlip;

/// Ordered secret coin observations.
#[derive(Default, Eq, PartialEq)]
pub struct FlipSequence(Vec<CoinFlip>);

impl FlipSequence {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, flip: CoinFlip) {
        self.0.push(flip);
    }

    /// Removes and zeroizes the latest observation.
    pub fn remove_last(&mut self) -> bool {
        let Some(last) = self.0.last_mut() else {
            return false;
        };
        last.zeroize();
        self.0.pop().is_some()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    /// Explicitly reveals the ordered secret observations.
    pub fn flips(&self) -> &[CoinFlip] {
        &self.0
    }

    #[must_use]
    /// Copies the secret observations into a zeroizing ASCII buffer.
    pub fn ascii_bytes(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(self.0.iter().copied().map(CoinFlip::ascii).collect())
    }

    #[must_use]
    /// Copies the secret observations into a zeroizing ASCII string.
    pub fn ascii_string(&self) -> Zeroizing<String> {
        Zeroizing::new(
            self.0
                .iter()
                .copied()
                .map(CoinFlip::ascii)
                .map(char::from)
                .collect(),
        )
    }
}

impl Drop for FlipSequence {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for FlipSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FlipSequence")
            .field("len", &self.len())
            .field("flips", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flips_accept_only_binary_values_and_preserve_order() {
        assert!(CoinFlip::new(2).is_err());
        let mut flips = FlipSequence::new();
        flips.push(CoinFlip::new(1).unwrap());
        flips.push(CoinFlip::new(0).unwrap());
        assert_eq!(flips.ascii_bytes().as_slice(), b"10");
        assert!(flips.remove_last());
        assert_eq!(flips.ascii_bytes().as_slice(), b"1");
    }

    #[test]
    fn debug_redacts_flip_values() {
        let flip = CoinFlip::new(1).unwrap();
        assert_eq!(format!("{flip:?}"), "CoinFlip([REDACTED])");
        let mut flips = FlipSequence::new();
        flips.push(flip);
        assert!(!format!("{flips:?}").contains("[1]"));
    }
}
