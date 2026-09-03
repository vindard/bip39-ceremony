use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::RollSequence,
    protocol::{
        ConversionProtocol, ProtocolError, require_complete_dice_capture, sha256_prefix_entropy,
    },
};

/// Legacy Keystone's documented Ian Coleman-compatible D6 text mapping.
#[must_use]
pub fn ascii_rolls(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        rolls
            .faces()
            .iter()
            .map(|face| if face.get() == 6 { b'0' } else { face.ascii() })
            .collect(),
    )
}

pub(crate) fn keystone_legacy_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    let protocol = ConversionProtocol::KeystoneLegacyV1;
    if !protocol.supports_target(target) {
        return Err(ProtocolError::UnsupportedTarget);
    }
    require_complete_dice_capture(protocol, target, rolls)?;
    let ascii = ascii_rolls(rolls);
    Ok(sha256_prefix_entropy(target, &[ascii.as_slice()]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    #[test]
    fn maps_six_to_zero_and_preserves_other_faces() {
        let mut rolls = RollSequence::new();
        for value in 1..=6 {
            rolls.push(DieFace::new(value).unwrap());
        }

        assert_eq!(ascii_rolls(&rolls).as_slice(), b"123450");
    }
}
