use core::fmt;

use zeroize::Zeroize;

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct D20Face(u8);

impl D20Face {
    /// Creates a one-based D20 face.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidD20Face`] unless `value` is in `1..=20`.
    pub const fn new(value: u8) -> Result<Self, InvalidD20Face> {
        if value >= 1 && value <= 20 {
            Ok(Self(value))
        } else {
            Err(InvalidD20Face)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Zeroize for D20Face {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for D20Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D20Face([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidD20Face;

impl fmt::Display for InvalidD20Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D20 face must be between 1 and 20")
    }
}

impl std::error::Error for InvalidD20Face {}

#[derive(Eq, PartialEq)]
pub struct D20RollSequence(Vec<D20Face>);

impl D20RollSequence {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::with_capacity(60))
    }

    pub fn push(&mut self, face: D20Face) {
        self.0.push(face);
    }

    /// Removes and zeroizes the latest face.
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

    /// Explicitly reveals the ordered secret faces.
    #[must_use]
    pub fn faces(&self) -> &[D20Face] {
        &self.0
    }
}

impl Default for D20RollSequence {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for D20RollSequence {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for D20RollSequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("D20RollSequence")
            .field("len", &self.len())
            .field("faces", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_validate_bounds_and_sequence_redacts_values() {
        assert!(D20Face::new(0).is_err());
        assert!(D20Face::new(20).is_ok());
        assert!(D20Face::new(21).is_err());
        let mut rolls = D20RollSequence::new();
        for face in [1, 10, 20] {
            rolls.push(D20Face::new(face).unwrap());
        }
        assert_eq!(rolls.faces().len(), 3);
        assert!(format!("{rolls:?}").contains("REDACTED"));
    }
}
