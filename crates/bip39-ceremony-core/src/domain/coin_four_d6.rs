use core::fmt;

use zeroize::{Zeroize, Zeroizing};

use super::{coin::CoinFlip, dice::DieFace};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CoinFourD6Observation {
    Coin(CoinFlip),
    D6(DieFace),
}

impl CoinFourD6Observation {
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::Coin(flip) => flip.get(),
            Self::D6(face) => face.get(),
        }
    }

    #[must_use]
    pub const fn kind_tag(self) -> u8 {
        match self {
            Self::Coin(_) => 2,
            Self::D6(_) => 6,
        }
    }
}

impl Zeroize for CoinFourD6Observation {
    fn zeroize(&mut self) {
        match self {
            Self::Coin(flip) => flip.zeroize(),
            Self::D6(face) => face.zeroize(),
        }
    }
}

impl fmt::Debug for CoinFourD6Observation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CoinFourD6Observation([REDACTED])")
    }
}

#[derive(Eq, PartialEq)]
pub struct CoinFourD6Capture(Vec<CoinFourD6Observation>);

impl CoinFourD6Capture {
    #[must_use]
    pub fn new() -> Self {
        // Leave practical rejection headroom so secret-bearing observations do
        // not normally cross a heap reallocation before drop-time zeroization.
        Self(Vec::with_capacity(128))
    }

    pub fn push_coin(&mut self, flip: CoinFlip) {
        self.0.push(CoinFourD6Observation::Coin(flip));
    }

    pub fn push_d6(&mut self, face: DieFace) {
        self.0.push(CoinFourD6Observation::D6(face));
    }

    /// Removes and zeroizes the latest raw observation.
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

    /// Explicitly reveals the ordered secret observations.
    #[must_use]
    pub fn observations(&self) -> &[CoinFourD6Observation] {
        &self.0
    }

    /// Copies typed `[kind, value]` pairs into a zeroizing audit buffer.
    #[must_use]
    pub fn audit_bytes(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            self.0
                .iter()
                .flat_map(|observation| [observation.kind_tag(), observation.value()])
                .collect(),
        )
    }
}

impl Default for CoinFourD6Capture {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CoinFourD6Capture {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for CoinFourD6Capture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoinFourD6Capture")
            .field("len", &self.len())
            .field("observations", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_preserves_kinds_and_redacts_values() {
        let mut capture = CoinFourD6Capture::new();
        capture.push_coin(CoinFlip::new(1).unwrap());
        capture.push_d6(DieFace::new(5).unwrap());
        assert_eq!(capture.audit_bytes().as_slice(), &[2, 1, 6, 5]);
        assert!(capture.remove_last());
        assert_eq!(capture.len(), 1);
        assert!(format!("{capture:?}").contains("REDACTED"));
    }
}
