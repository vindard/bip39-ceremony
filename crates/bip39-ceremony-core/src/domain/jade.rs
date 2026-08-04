use core::fmt;

use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct D16Face(u8);

impl D16Face {
    /// Creates a one-based D16 face.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidD16Face`] unless `value` is in `1..=16`.
    pub const fn new(value: u8) -> Result<Self, InvalidD16Face> {
        if value >= 1 && value <= 16 {
            Ok(Self(value))
        } else {
            Err(InvalidD16Face)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn zero_based(self) -> u8 {
        self.0 - 1
    }
}

impl Zeroize for D16Face {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for D16Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D16Face([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidD16Face;

impl fmt::Display for InvalidD16Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D16 face must be between 1 and 16")
    }
}

impl std::error::Error for InvalidD16Face {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct D8Face(u8);

impl D8Face {
    /// Creates a one-based D8 face.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidD8Face`] unless `value` is in `1..=8`.
    pub const fn new(value: u8) -> Result<Self, InvalidD8Face> {
        if value >= 1 && value <= 8 {
            Ok(Self(value))
        } else {
            Err(InvalidD8Face)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn zero_based(self) -> u8 {
        self.0 - 1
    }
}

impl Zeroize for D8Face {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for D8Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D8Face([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidD8Face;

impl fmt::Display for InvalidD8Face {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("D8 face must be between 1 and 8")
    }
}

impl std::error::Error for InvalidD8Face {}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum JadeObservation {
    D16(D16Face),
    D8(D8Face),
}

impl JadeObservation {
    #[must_use]
    pub const fn face(self) -> u8 {
        match self {
            Self::D16(face) => face.get(),
            Self::D8(face) => face.get(),
        }
    }

    #[must_use]
    pub const fn sides(self) -> u8 {
        match self {
            Self::D16(_) => 16,
            Self::D8(_) => 8,
        }
    }
}

impl Zeroize for JadeObservation {
    fn zeroize(&mut self) {
        match self {
            Self::D16(face) => face.zeroize(),
            Self::D8(face) => face.zeroize(),
        }
    }
}

impl fmt::Debug for JadeObservation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JadeObservation([REDACTED])")
    }
}

#[derive(Eq, PartialEq)]
pub struct JadeCapture(Vec<JadeObservation>);

impl JadeCapture {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::with_capacity(70))
    }

    pub fn push_d16(&mut self, face: D16Face) {
        self.0.push(JadeObservation::D16(face));
    }

    pub fn push_d8(&mut self, face: D8Face) {
        self.0.push(JadeObservation::D8(face));
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

    /// Explicitly reveals the ordered secret observations.
    #[must_use]
    pub fn observations(&self) -> &[JadeObservation] {
        &self.0
    }

    /// Copies the typed audit encoding `[sides, face]` into a zeroizing buffer.
    #[must_use]
    pub fn audit_bytes(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(
            self.0
                .iter()
                .flat_map(|observation| [observation.sides(), observation.face()])
                .collect(),
        )
    }
}

impl Default for JadeCapture {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JadeCapture {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for JadeCapture {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JadeCapture")
            .field("len", &self.len())
            .field("observations", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_validate_bounds_and_redact_debug() {
        assert!(D16Face::new(0).is_err());
        assert!(D16Face::new(16).is_ok());
        assert!(D16Face::new(17).is_err());
        assert!(D8Face::new(0).is_err());
        assert!(D8Face::new(8).is_ok());
        assert!(D8Face::new(9).is_err());
        assert!(format!("{:?}", D16Face::new(10).unwrap()).contains("REDACTED"));
    }

    #[test]
    fn capture_preserves_types_and_zeroizes_on_undo() {
        let mut capture = JadeCapture::new();
        capture.push_d16(D16Face::new(10).unwrap());
        capture.push_d8(D8Face::new(8).unwrap());
        assert_eq!(capture.audit_bytes().as_slice(), &[16, 10, 8, 8]);
        assert!(capture.remove_last());
        assert_eq!(capture.len(), 1);
        assert!(format!("{capture:?}").contains("REDACTED"));
    }
}
