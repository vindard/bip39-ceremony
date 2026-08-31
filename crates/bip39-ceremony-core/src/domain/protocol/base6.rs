use crate::domain::dice::{DieFace, RollSequence};

/// Accumulates an ordered D6 sequence as one big-endian base-6 integer.
///
/// The first roll is the most-significant digit and each face contributes
/// `face - 1`. `value` must be wide enough to hold `6^rolls.len() - 1`; the
/// caller sizes it, and an overflow is a caller bug rather than a rejection.
pub(super) fn accumulate(value: &mut [u8], rolls: &RollSequence) {
    for face in rolls.faces() {
        multiply_add(value, *face);
    }
}

fn multiply_add(value: &mut [u8], face: DieFace) {
    let mut carry = u16::from(face.base6_digit());
    for byte in value.iter_mut().rev() {
        let next = u16::from(*byte) * 6 + carry;
        *byte = u8::try_from(next & 0xff).expect("masked byte fits u8");
        carry = next >> 8;
    }
    assert_eq!(carry, 0, "fixed buffer holds the configured roll count");
}

/// Byte width of the minimal big-endian encoding of an accumulated value.
///
/// Leading zero bytes are not part of that encoding, so a value below
/// `256^(value.len() - 1)` is narrower than the buffer holding it.
pub(super) fn significant_bytes(value: &[u8]) -> usize {
    value.len() - value.iter().take_while(|byte| **byte == 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolls(value: &str) -> RollSequence {
        let mut result = RollSequence::new();
        for character in value.chars() {
            result.push(DieFace::try_from(character).unwrap());
        }
        result
    }

    #[test]
    fn first_roll_is_the_most_significant_digit() {
        let mut value = [0_u8; 2];
        accumulate(&mut value, &rolls("21"));

        assert_eq!(value, [0, 6]);
    }

    #[test]
    fn significant_width_ignores_leading_zero_bytes() {
        assert_eq!(significant_bytes(&[0, 0, 0]), 0);
        assert_eq!(significant_bytes(&[0, 0, 1]), 1);
        assert_eq!(significant_bytes(&[0, 1, 0]), 2);
        assert_eq!(significant_bytes(&[1, 0, 0]), 3);
    }
}
