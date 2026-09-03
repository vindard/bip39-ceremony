use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::{DieFace, RollSequence},
    protocol::ProtocolError,
};

/// How many bits a face contributes, and their value.
///
/// Six is not a power of two, so a fixed width per face would be biased. Four
/// faces carry two bits and two faces carry one, which keeps every emitted bit
/// unbiased under fair dice at a cost of about 1.67 bits per roll instead of
/// log2(6) ≈ 2.58.
#[must_use]
pub const fn face_bits(face: DieFace) -> (u8, u8) {
    match face.get() {
        1 => (0b00, 2),
        2 => (0b01, 2),
        3 => (0b10, 2),
        4 => (0b11, 2),
        5 => (0, 1),
        _ => (1, 1),
    }
}

/// Progress of a bit-packing capture, whose length depends on the faces rolled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitPackProgress {
    recorded: usize,
    consumed: usize,
    bits: usize,
    required_bits: usize,
}

impl BitPackProgress {
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }
    /// Rolls that contributed bits. A roll arriving after the target width is
    /// reached contributes nothing.
    #[must_use]
    pub const fn consumed(self) -> usize {
        self.consumed
    }
    #[must_use]
    pub const fn bits(self) -> usize {
        self.bits
    }
    #[must_use]
    pub const fn required_bits(self) -> usize {
        self.required_bits
    }
    /// Whether the packed bits exactly fill the target width.
    #[must_use]
    pub const fn is_filled(self) -> bool {
        self.bits == self.required_bits
    }
    /// Whether every recorded roll contributed. A longer tape means the source
    /// implementation silently discarded the surplus.
    #[must_use]
    pub const fn uses_every_roll(self) -> bool {
        self.consumed == self.recorded
    }
}

/// Packs an ordered D6 sequence into bits, most-significant bit first.
///
/// The roll that would overshoot the target width keeps only its leading bits;
/// rolls after the width is reached are ignored. Both behaviours are the source
/// implementation's, and both mean the tape length alone does not determine
/// where a capture completes.
#[must_use]
pub fn bitpack_progress(target: EntropyTarget, rolls: &RollSequence) -> BitPackProgress {
    let required_bits = target.entropy_bits();
    let mut bits = 0_usize;
    let mut consumed = 0_usize;
    for face in rolls.faces() {
        if bits == required_bits {
            break;
        }
        let (_, width) = face_bits(*face);
        bits = (bits + usize::from(width)).min(required_bits);
        consumed += 1;
    }
    BitPackProgress {
        recorded: rolls.len(),
        consumed,
        bits,
        required_bits,
    }
}

/// Converts an ordered D6 sequence into entropy by packing face bits directly.
///
/// # Errors
///
/// Returns [`ProtocolError::WrongObservationCount`] unless the packed bits
/// exactly fill the target width using every recorded roll.
pub fn bluewallet_bitpack_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    let progress = bitpack_progress(target, rolls);
    if !progress.is_filled() {
        return Err(ProtocolError::WrongObservationCount {
            expected: progress.recorded().saturating_add(1),
            actual: progress.recorded(),
        });
    }
    if !progress.uses_every_roll() {
        return Err(ProtocolError::WrongObservationCount {
            expected: progress.consumed(),
            actual: progress.recorded(),
        });
    }

    Ok(calculate_entropy(target, rolls))
}

fn calculate_entropy(target: EntropyTarget, rolls: &RollSequence) -> Entropy {
    let required_bits = target.entropy_bits();
    let mut bytes = Zeroizing::new(vec![0_u8; target.entropy_bytes()]);
    let mut placed = 0_usize;
    for face in rolls.faces() {
        let (value, width) = face_bits(*face);
        for shift in (0..width).rev() {
            if placed == required_bits {
                break;
            }
            let bit = (value >> shift) & 1;
            bytes[placed / 8] |= bit << (7 - (placed % 8));
            placed += 1;
        }
    }

    Entropy::from_protocol_bytes(target, bytes.to_vec())
}

/// The packed bits as ASCII `0`/`1`, for inspection.
#[must_use]
pub fn packed_bits(target: EntropyTarget, rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    let required_bits = target.entropy_bits();
    let mut output = Zeroizing::new(Vec::with_capacity(required_bits));
    for face in rolls.faces() {
        let (value, width) = face_bits(*face);
        for shift in (0..width).rev() {
            if output.len() == required_bits {
                return output;
            }
            output.push(b'0' + ((value >> shift) & 1));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolls(faces: &[u8]) -> RollSequence {
        let mut result = RollSequence::new();
        for face in faces {
            result.push(DieFace::new(*face).unwrap());
        }
        result
    }

    fn repeated(face: u8, count: usize) -> RollSequence {
        rolls(&vec![face; count])
    }

    fn cycled(count: usize) -> RollSequence {
        let faces: Vec<u8> = (0..count)
            .map(|index| u8::try_from(index % 6 + 1).unwrap())
            .collect();
        rolls(&faces)
    }

    fn hex(entropy: &Entropy) -> String {
        use core::fmt::Write as _;
        entropy.bytes().iter().fold(String::new(), |mut out, byte| {
            write!(out, "{byte:02x}").expect("writing to String cannot fail");
            out
        })
    }

    #[test]
    fn face_widths_follow_the_four_two_split() {
        for (face, expected) in [(1, (0, 2)), (2, (1, 2)), (3, (2, 2)), (4, (3, 2))] {
            assert_eq!(face_bits(DieFace::new(face).unwrap()), expected);
        }
        assert_eq!(face_bits(DieFace::new(5).unwrap()), (0, 1));
        assert_eq!(face_bits(DieFace::new(6).unwrap()), (1, 1));
    }

    /// Two-bit faces only: 64 rolls fill 128 bits exactly.
    #[test]
    fn all_two_bit_faces_fill_the_width_in_sixty_four_rolls() {
        let progress = bitpack_progress(EntropyTarget::Words12, &repeated(1, 64));
        assert_eq!((progress.bits(), progress.consumed()), (128, 64));
        assert!(progress.is_filled() && progress.uses_every_roll());

        let zero = bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(1, 64)).unwrap();
        assert_eq!(hex(&zero), "00000000000000000000000000000000");
        let ones = bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(4, 64)).unwrap();
        assert_eq!(hex(&ones), "ffffffffffffffffffffffffffffffff");
    }

    /// One-bit faces only: it takes 128 rolls to fill the same width.
    #[test]
    fn all_one_bit_faces_need_twice_as_many_rolls() {
        let zero = bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(5, 128)).unwrap();
        assert_eq!(hex(&zero), "00000000000000000000000000000000");
        let ones = bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(6, 128)).unwrap();
        assert_eq!(hex(&ones), "ffffffffffffffffffffffffffffffff");
    }

    /// Oracle vector: 1..6 repeating carries 10 bits per six rolls, so 76 rolls
    /// land exactly on 128 bits.
    #[test]
    fn repeating_tape_matches_the_source_reading() {
        let progress = bitpack_progress(EntropyTarget::Words12, &cycled(77));
        assert_eq!((progress.bits(), progress.consumed()), (128, 76));
        assert!(
            !progress.uses_every_roll(),
            "the 77th roll contributes nothing"
        );

        let entropy = bluewallet_bitpack_entropy(EntropyTarget::Words12, &cycled(76)).unwrap();
        assert_eq!(hex(&entropy), "1b46d1b46d1b46d1b46d1b46d1b46d1b");
    }

    #[test]
    fn repeating_tape_matches_the_source_reading_at_twenty_four_words() {
        let entropy = bluewallet_bitpack_entropy(EntropyTarget::Words24, &cycled(153)).unwrap();
        assert_eq!(
            hex(&entropy),
            "1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46d1b46"
        );
    }

    /// The roll that overshoots keeps its leading bit, not its trailing one.
    /// Face 4 is `11`; arriving with one bit of room it contributes `1`.
    #[test]
    fn an_overshooting_roll_keeps_its_leading_bit() {
        let mut tape = repeated(5, 127);
        tape.push(DieFace::new(4).unwrap());
        let entropy = bluewallet_bitpack_entropy(EntropyTarget::Words12, &tape).unwrap();
        assert_eq!(hex(&entropy), "00000000000000000000000000000001");
    }

    #[test]
    fn a_tape_short_of_the_width_has_no_conversion() {
        assert_eq!(
            bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(1, 63)),
            Err(ProtocolError::WrongObservationCount {
                expected: 64,
                actual: 63,
            })
        );
    }

    #[test]
    fn a_tape_past_the_width_reports_its_unused_surplus() {
        assert_eq!(
            bluewallet_bitpack_entropy(EntropyTarget::Words12, &repeated(1, 70)),
            Err(ProtocolError::WrongObservationCount {
                expected: 64,
                actual: 70,
            })
        );
    }

    #[test]
    fn packed_bits_stop_at_the_target_width() {
        let bits = packed_bits(EntropyTarget::Words12, &repeated(6, 200));
        assert_eq!(bits.len(), 128);
        assert!(bits.iter().all(|bit| *bit == b'1'));
    }
}
