use core::fmt;

use zeroize::{Zeroize, Zeroizing};

use super::DieFace;

/// Ordered secret dice observations.
#[derive(Default, Eq, PartialEq)]
pub struct RollSequence(Vec<DieFace>);

impl RollSequence {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, face: DieFace) {
        self.0.push(face);
    }

    /// Removes and zeroizes the latest observation.
    pub fn remove_last(&mut self) -> bool {
        let Some(last) = self.0.last_mut() else {
            return false;
        };
        last.zeroize();
        let removed = self.0.pop();
        debug_assert!(removed.is_some());
        true
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
    pub fn faces(&self) -> &[DieFace] {
        &self.0
    }

    #[must_use]
    /// Copies the secret observations into a zeroizing ASCII buffer.
    pub fn ascii_bytes(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(self.0.iter().copied().map(DieFace::ascii).collect())
    }

    #[must_use]
    /// Copies the secret observations into a zeroizing ASCII string.
    pub fn ascii_string(&self) -> Zeroizing<String> {
        Zeroizing::new(
            self.0
                .iter()
                .copied()
                .map(DieFace::ascii)
                .map(char::from)
                .collect(),
        )
    }
}

impl Drop for RollSequence {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for RollSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RollSequence")
            .field("len", &self.len())
            .field("faces", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_preserves_order_and_supports_undo() {
        let mut sequence = RollSequence::new();
        sequence.push(DieFace::new(2).unwrap());
        sequence.push(DieFace::new(6).unwrap());

        assert_eq!(sequence.ascii_bytes().as_slice(), b"26");
        assert!(sequence.remove_last());
        assert_eq!(sequence.ascii_bytes().as_slice(), b"2");
    }

    #[test]
    fn sequence_debug_output_redacts_faces() {
        let mut sequence = RollSequence::new();
        sequence.push(DieFace::new(4).unwrap());
        let debug = format!("{sequence:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("faces: [4]"));
    }
}
