use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::RollSequence,
    protocol::{ConversionProtocol, ProtocolError, base6},
};

/// Result of the unhashed base-6 reading, which can reject a complete attempt.
#[derive(Debug, Eq, PartialEq)]
pub enum BitcoinLibBase6EntropyOutcome {
    Accepted(Entropy),
    Rejected,
}

/// Reads an ordered D6 sequence as a base-6 integer and uses it as entropy.
///
/// The first roll is the most-significant digit and each face maps to
/// `face - 1`. There is no hash and no rejection sampling, so the reading is
/// only defined when the integer's minimal big-endian encoding is exactly the
/// target entropy width. Values that overflow that width, or that are small
/// enough to encode in fewer bytes, do not describe a `target`-width BIP-39
/// input and are rejected rather than padded or truncated.
///
/// # Errors
///
/// Returns [`ProtocolError::WrongObservationCount`] unless the sequence
/// contains exactly 50 rolls for 128 bits or 99 rolls for 256 bits.
pub fn bitcoinlib_base6_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<BitcoinLibBase6EntropyOutcome, ProtocolError> {
    let expected = ConversionProtocol::BitcoinLibBase6V1.minimum_observations(target);
    if rolls.len() != expected {
        return Err(ProtocolError::WrongObservationCount {
            expected,
            actual: rolls.len(),
        });
    }

    let width = target.entropy_bytes();
    let mut value = Zeroizing::new(vec![0_u8; width + 1]);
    base6::accumulate(&mut value, rolls);

    if base6::significant_bytes(&value) != width {
        return Ok(BitcoinLibBase6EntropyOutcome::Rejected);
    }

    Ok(BitcoinLibBase6EntropyOutcome::Accepted(
        Entropy::from_protocol_bytes(target, value[1..].to_vec()),
    ))
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

    /// The repeating `1..6` tape both bitcoinlib entropy tests roll.
    fn source_tape(count: usize) -> RollSequence {
        let tape: String = "123456".chars().cycle().take(count).collect();
        rolls(&tape)
    }

    /// Primary-source vector: bitcoinlib's own 50-roll entropy test.
    #[test]
    fn fifty_roll_source_vector_matches_the_published_integer() {
        let outcome = bitcoinlib_base6_entropy(EntropyTarget::Words12, &source_tape(50)).unwrap();

        let BitcoinLibBase6EntropyOutcome::Accepted(entropy) = outcome else {
            panic!("published vector is accepted");
        };
        assert_eq!(
            entropy.bytes(),
            [
                0x18, 0x4e, 0xc4, 0xbe, 0xd5, 0x6e, 0xb8, 0x6a, 0xac, 0xaa, 0xa2, 0x24, 0xb5, 0x67,
                0x2f, 0x45,
            ]
        );
    }

    /// Primary-source vector: bitcoinlib's own 99-roll entropy test.
    #[test]
    fn ninety_nine_roll_source_vector_matches_the_published_integer() {
        let outcome = bitcoinlib_base6_entropy(EntropyTarget::Words24, &source_tape(99)).unwrap();

        let BitcoinLibBase6EntropyOutcome::Accepted(entropy) = outcome else {
            panic!("published vector is accepted");
        };
        assert_eq!(
            entropy.bytes(),
            [
                0x09, 0x9f, 0x84, 0x37, 0xb4, 0x99, 0x6f, 0x90, 0x32, 0x67, 0xce, 0x4a, 0x8c, 0x0f,
                0x47, 0xad, 0xd7, 0x8d, 0xfe, 0x53, 0x88, 0x7d, 0xc3, 0x96, 0x51, 0x97, 0xcc, 0xdc,
                0x40, 0x6b, 0x1b, 0xa0,
            ]
        );
    }

    #[test]
    fn all_ones_is_rejected_because_zero_has_no_target_width_encoding() {
        assert_eq!(
            bitcoinlib_base6_entropy(EntropyTarget::Words12, &rolls(&"1".repeat(50))),
            Ok(BitcoinLibBase6EntropyOutcome::Rejected)
        );
    }

    #[test]
    fn a_value_wider_than_the_target_is_rejected_rather_than_truncated() {
        // 6^50 exceeds 2^128, so the largest 50-roll reading does not fit.
        assert_eq!(
            bitcoinlib_base6_entropy(EntropyTarget::Words12, &rolls(&"6".repeat(50))),
            Ok(BitcoinLibBase6EntropyOutcome::Rejected)
        );
    }

    #[test]
    fn the_reading_requires_its_exact_roll_count() {
        assert_eq!(
            bitcoinlib_base6_entropy(EntropyTarget::Words24, &rolls(&"6".repeat(100))),
            Err(ProtocolError::WrongObservationCount {
                expected: 99,
                actual: 100,
            })
        );
    }
}
