use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::{DieFace, RollSequence},
    protocol::ProtocolError,
};

/// Bits a face contributes under the base-6 table, after the 6-to-0 rewrite.
///
/// The rewrite maps face 6 to digit 0, so the table is shifted by one against
/// implementations that pack faces directly: face 1 carries `01` here where
/// `BlueWallet`'s face 1 carries `00`.
#[must_use]
pub const fn raw_face_bits(face: DieFace) -> (u8, u8) {
    match face.get() {
        1 => (0b01, 2),
        2 => (0b10, 2),
        3 => (0b11, 2),
        4 => (0, 1),
        5 => (1, 1),
        // Face 6 becomes digit 0, which is the only two-bit zero.
        _ => (0b00, 2),
    }
}

/// Progress of a raw-mode capture, whose word count follows the faces rolled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IanColemanRawProgress {
    recorded: usize,
    bits: usize,
    required_bits: usize,
}

impl IanColemanRawProgress {
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }
    #[must_use]
    pub const fn bits(self) -> usize {
        self.bits
    }
    #[must_use]
    pub const fn required_bits(self) -> usize {
        self.required_bits
    }
    /// Whole 32-bit groups, which is what upstream keeps.
    #[must_use]
    pub const fn usable_bits(self) -> usize {
        (self.bits / 32) * 32
    }
    /// Whether the tape currently yields exactly the target word count.
    #[must_use]
    pub const fn is_on_target(self) -> bool {
        self.usable_bits() == self.required_bits
    }
    /// Whether another roll could still leave the tape on target.
    #[must_use]
    pub const fn is_overrun(self) -> bool {
        self.usable_bits() > self.required_bits
    }
}

#[must_use]
pub fn iancoleman_raw_progress(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> IanColemanRawProgress {
    let bits = rolls
        .faces()
        .iter()
        .map(|face| usize::from(raw_face_bits(*face).1))
        .sum();
    IanColemanRawProgress {
        recorded: rolls.len(),
        bits,
        required_bits: target.entropy_bits(),
    }
}

/// Converts an ordered D6 sequence the way iancoleman's raw mode does.
///
/// Faces are packed most-significant bit first, then the leading remainder is
/// discarded so that only whole 32-bit groups are kept. Upstream keeps the
/// *trailing* groups, so the first rolls of a tape can fall out of the result
/// entirely — the opposite end from implementations that truncate the final
/// roll.
///
/// # Errors
///
/// Returns [`ProtocolError::WrongObservationCount`] unless the packed bits
/// reduce to exactly the target width.
pub fn iancoleman_raw_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    let progress = iancoleman_raw_progress(target, rolls);
    if !progress.is_on_target() {
        return Err(ProtocolError::WrongObservationCount {
            expected: progress.required_bits(),
            actual: progress.usable_bits(),
        });
    }

    let mut packed = Zeroizing::new(Vec::with_capacity(progress.bits()));
    for face in rolls.faces() {
        let (value, width) = raw_face_bits(*face);
        for shift in (0..width).rev() {
            packed.push((value >> shift) & 1);
        }
    }

    let start = packed.len() - progress.required_bits();
    let mut bytes = Zeroizing::new(vec![0_u8; target.entropy_bytes()]);
    for (index, bit) in packed[start..].iter().enumerate() {
        bytes[index / 8] |= bit << (7 - (index % 8));
    }

    Ok(Entropy::from_protocol_bytes(target, bytes.to_vec()))
}

/// The packed bits actually used, as ASCII `0`/`1`, for inspection.
#[must_use]
pub fn iancoleman_raw_bits(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    let mut packed = Zeroizing::new(Vec::new());
    for face in rolls.faces() {
        let (value, width) = raw_face_bits(*face);
        for shift in (0..width).rev() {
            packed.push(b'0' + ((value >> shift) & 1));
        }
    }
    let usable = (packed.len() / 32) * 32;
    let start = packed.len() - usable;
    Zeroizing::new(packed[start..].to_vec())
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

    fn cycled(count: usize) -> RollSequence {
        rolls(&"123456".chars().cycle().take(count).collect::<String>())
    }

    fn hex(entropy: &Entropy) -> String {
        use core::fmt::Write as _;
        entropy.bytes().iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").expect("writing to String cannot fail");
            out
        })
    }

    #[test]
    fn the_table_is_shifted_by_the_six_to_zero_rewrite() {
        // Face 6 becomes digit 0, so it carries two bits, not one.
        assert_eq!(raw_face_bits(DieFace::new(6).unwrap()), (0b00, 2));
        assert_eq!(raw_face_bits(DieFace::new(1).unwrap()), (0b01, 2));
        assert_eq!(raw_face_bits(DieFace::new(4).unwrap()), (0, 1));
        assert_eq!(raw_face_bits(DieFace::new(5).unwrap()), (1, 1));
    }

    #[test]
    fn a_tape_off_the_word_boundary_has_no_conversion() {
        // 10 bits per six rolls, so 48 rolls give 80 bits: two whole groups.
        assert_eq!(
            iancoleman_raw_entropy(EntropyTarget::Words12, &cycled(48)),
            Err(ProtocolError::WrongObservationCount {
                expected: 128,
                actual: 64,
            })
        );
    }

    #[test]
    fn the_leading_remainder_is_discarded_not_the_trailing_one() {
        // 78 rolls of the cycle give 130 bits; upstream keeps the last 128.
        let progress = iancoleman_raw_progress(EntropyTarget::Words12, &cycled(78));
        assert_eq!((progress.bits(), progress.usable_bits()), (130, 128));

        let entropy = iancoleman_raw_entropy(EntropyTarget::Words12, &cycled(78)).unwrap();
        let bits = iancoleman_raw_bits(&cycled(78));
        assert_eq!(bits.len(), 128);

        // Dropping the two leading bits shifts everything, so the result
        // differs from the same tape read from the front.
        let from_front = iancoleman_raw_entropy(EntropyTarget::Words12, &cycled(77)).unwrap();
        assert_ne!(hex(&entropy), hex(&from_front));
    }

    #[test]
    fn an_all_one_bit_tape_needs_twice_as_many_rolls() {
        // Faces 4 and 5 carry one bit each.
        let low = iancoleman_raw_entropy(EntropyTarget::Words12, &rolls(&"4".repeat(128))).unwrap();
        assert_eq!(hex(&low), "00000000000000000000000000000000");
        let high =
            iancoleman_raw_entropy(EntropyTarget::Words12, &rolls(&"5".repeat(128))).unwrap();
        assert_eq!(hex(&high), "ffffffffffffffffffffffffffffffff");
    }
}
