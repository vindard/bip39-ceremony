use core::fmt;

use zeroize::{Zeroize, Zeroizing};

use super::{coin::CoinFlip, dice::DieFace};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum BitBoxObservation {
    D6(DieFace),
    Coin(CoinFlip),
}

impl BitBoxObservation {
    #[must_use]
    pub const fn value(self) -> u8 {
        match self {
            Self::D6(face) => face.get(),
            Self::Coin(flip) => flip.get(),
        }
    }

    #[must_use]
    pub const fn kind_tag(self) -> u8 {
        match self {
            Self::D6(_) => 6,
            Self::Coin(_) => 2,
        }
    }
}

impl Zeroize for BitBoxObservation {
    fn zeroize(&mut self) {
        match self {
            Self::D6(face) => face.zeroize(),
            Self::Coin(flip) => flip.zeroize(),
        }
    }
}

impl fmt::Debug for BitBoxObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BitBoxObservation([REDACTED])")
    }
}

#[derive(Eq, PartialEq)]
pub struct BitBoxCapture(Vec<BitBoxObservation>);

impl BitBoxCapture {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::with_capacity(141))
    }

    pub fn push_d6(&mut self, face: DieFace) {
        self.0.push(BitBoxObservation::D6(face));
    }

    pub fn push_coin(&mut self, flip: CoinFlip) {
        self.0.push(BitBoxObservation::Coin(flip));
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
    pub fn observations(&self) -> &[BitBoxObservation] {
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

impl Default for BitBoxCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BitBoxCapture {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for BitBoxCapture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BitBoxCapture")
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
        let mut capture = BitBoxCapture::new();
        capture.push_d6(DieFace::new(5).unwrap());
        capture.push_coin(CoinFlip::new(1).unwrap());
        assert_eq!(capture.audit_bytes().as_slice(), &[6, 5, 2, 1]);
        assert!(capture.remove_last());
        assert_eq!(capture.len(), 1);
        assert!(format!("{capture:?}").contains("REDACTED"));
    }
}
