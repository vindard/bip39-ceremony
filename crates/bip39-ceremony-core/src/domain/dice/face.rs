use core::fmt;

use zeroize::Zeroize;

/// A validated physical six-sided die result.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct DieFace(u8);

impl DieFace {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 6;

    /// Creates a validated D6 face.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidDieFace`] unless `value` is between one and six.
    pub fn new(value: u8) -> Result<Self, InvalidDieFace> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(InvalidDieFace)
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn base6_digit(self) -> u8 {
        self.0 - 1
    }

    #[must_use]
    pub const fn ascii(self) -> u8 {
        b'0' + self.0
    }
}

impl fmt::Debug for DieFace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DieFace([REDACTED])")
    }
}

impl Zeroize for DieFace {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl TryFrom<char> for DieFace {
    type Error = InvalidDieFace;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        value
            .to_digit(10)
            .and_then(|digit| u8::try_from(digit).ok())
            .ok_or(InvalidDieFace)
            .and_then(Self::new)
    }
}

/// Intentionally reveals no rejected value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidDieFace;

impl fmt::Display for InvalidDieFace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("die face must be between 1 and 6")
    }
}

impl std::error::Error for InvalidDieFace {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_face_accepts_only_one_through_six() {
        for value in 1..=6 {
            let face = DieFace::new(value).expect("valid D6 face");
            assert_eq!(face.get(), value);
            assert_eq!(face.base6_digit(), value - 1);
            assert_eq!(face.ascii(), b'0' + value);
        }

        for value in [0, 7, u8::MAX] {
            assert_eq!(DieFace::new(value), Err(InvalidDieFace));
        }
    }

    #[test]
    fn debug_redacts_the_observation() {
        assert_eq!(
            format!("{:?}", DieFace::new(4).unwrap()),
            "DieFace([REDACTED])"
        );
    }

    #[test]
    fn character_conversion_rejects_non_faces() {
        for value in ['1', '6'] {
            assert!(DieFace::try_from(value).is_ok());
        }
        for value in ['0', '7', 'x', '\n', '１'] {
            assert_eq!(DieFace::try_from(value), Err(InvalidDieFace));
        }
    }
}
