use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::RollSequence,
    protocol::{ProtocolError, base6},
};

/// Result of exact uniform conversion, which can reject a complete attempt.
#[derive(Debug, Eq, PartialEq)]
pub enum ExactEntropyOutcome {
    Accepted(Entropy),
    Rejected,
}

/// Converts an ordered fixed-length D6 sequence to exactly uniform entropy.
///
/// The first roll is the most-significant base-6 digit and each face maps to
/// `face - 1`. Inputs at or above the largest multiple of the target space are
/// rejected.
///
/// # Errors
///
/// Returns [`ProtocolError::WrongObservationCount`] unless the sequence contains
/// exactly 50 rolls for 128 bits or 100 rolls for 256 bits.
pub fn exact_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<ExactEntropyOutcome, ProtocolError> {
    require_roll_count(target.strict_roll_count(), rolls.len())?;

    let mut value = Zeroizing::new(vec![0_u8; target.entropy_bytes() + 1]);
    base6::accumulate(&mut value, rolls);

    let accepted_quotients = match target {
        EntropyTarget::Words12 => 2,
        EntropyTarget::Words24 => 5,
    };
    if value[0] >= accepted_quotients {
        return Ok(ExactEntropyOutcome::Rejected);
    }

    let entropy = Entropy::from_protocol_bytes(target, value[1..].to_vec());
    Ok(ExactEntropyOutcome::Accepted(entropy))
}

fn require_roll_count(expected: usize, actual: usize) -> Result<(), ProtocolError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ProtocolError::WrongObservationCount { expected, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    fn rolls(value: &str) -> RollSequence {
        let mut result = RollSequence::new();
        for character in value.chars() {
            result.push(DieFace::try_from(character).unwrap());
        }
        result
    }

    #[test]
    fn exact_mode_preserves_leading_zero_digits() {
        let result = exact_entropy(EntropyTarget::Words12, &rolls(&"1".repeat(50))).unwrap();
        let ExactEntropyOutcome::Accepted(entropy) = result else {
            panic!("zero must be accepted");
        };
        assert_eq!(entropy.bytes(), [0; 16]);
    }

    #[test]
    fn exact_128_boundary_is_unbiased() {
        let below = rolls("61262262611466652634263242642635642166662444332122");
        let at = rolls("61262262611466652634263242642635642166662444332123");

        assert!(matches!(
            exact_entropy(EntropyTarget::Words12, &below),
            Ok(ExactEntropyOutcome::Accepted(_))
        ));
        assert!(matches!(
            exact_entropy(EntropyTarget::Words12, &at),
            Ok(ExactEntropyOutcome::Rejected)
        ));
    }

    #[test]
    fn exact_256_boundary_is_unbiased() {
        let below = rolls(concat!(
            "62633655465242253144132535222554424333361342521346",
            "25323511345453215331453612416422433556351254153322"
        ));
        let at = rolls(concat!(
            "62633655465242253144132535222554424333361342521346",
            "25323511345453215331453612416422433556351254153323"
        ));

        assert!(matches!(
            exact_entropy(EntropyTarget::Words24, &below),
            Ok(ExactEntropyOutcome::Accepted(_))
        ));
        assert!(matches!(
            exact_entropy(EntropyTarget::Words24, &at),
            Ok(ExactEntropyOutcome::Rejected)
        ));
    }

    #[test]
    fn exact_mode_rejects_wrong_counts() {
        assert_eq!(
            exact_entropy(EntropyTarget::Words12, &rolls("123456")),
            Err(ProtocolError::WrongObservationCount {
                expected: 50,
                actual: 6,
            })
        );
    }
}
