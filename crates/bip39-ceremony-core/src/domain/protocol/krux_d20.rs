use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    d20::D20RollSequence,
    protocol::{ConversionProtocol, ProtocolError, sha256_prefix_entropy},
};

/// Krux canonical D20 input: unpadded decimal faces joined by ASCII hyphens.
#[must_use]
pub(crate) fn ascii_rolls(rolls: &D20RollSequence) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(rolls.len().saturating_mul(3)));
    for (index, face) in rolls.faces().iter().enumerate() {
        if index > 0 {
            bytes.push(b'-');
        }
        if face.get() >= 10 {
            bytes.push(b'0' + face.get() / 10);
        }
        bytes.push(b'0' + face.get() % 10);
    }
    bytes
}

pub(crate) fn krux_d20_entropy(
    target: EntropyTarget,
    rolls: &D20RollSequence,
) -> Result<Entropy, ProtocolError> {
    let protocol = ConversionProtocol::KruxD20V1;
    if !protocol.assess_d20_capture(target, rolls).is_complete() {
        return Err(ProtocolError::WrongObservationCount {
            expected: protocol.minimum_observations(target),
            actual: rolls.len(),
        });
    }
    let ascii = ascii_rolls(rolls);
    Ok(sha256_prefix_entropy(target, &[ascii.as_slice()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::d20::D20Face;

    #[test]
    fn serialization_has_no_leading_or_trailing_separator() {
        let mut rolls = D20RollSequence::new();
        for value in [1, 9, 10, 20] {
            rolls.push(D20Face::new(value).unwrap());
        }
        assert_eq!(ascii_rolls(&rolls).as_slice(), b"1-9-10-20");
    }
}
