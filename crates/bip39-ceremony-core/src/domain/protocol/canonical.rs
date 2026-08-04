use core::fmt;

use zeroize::Zeroizing;

use crate::domain::{bip39::EntropyTarget, coin::FlipSequence, dice::RollSequence};

use super::{
    ConversionProtocol, coldcard_ascii_rolls, keystone_legacy_ascii_rolls, native_hash_header,
};

/// Secret protocol input represented without presentation wording.
pub enum CanonicalInput {
    Base6Integer(Zeroizing<Vec<u8>>),
    LocalizedBase6Candidates(Zeroizing<Vec<u8>>),
    VersionedBinary {
        header: Vec<u8>,
        ascii_rolls: Zeroizing<Vec<u8>>,
    },
    AsciiFaceDigits(Zeroizing<Vec<u8>>),
    AsciiFacesWithSixAsZero(Zeroizing<Vec<u8>>),
    AsciiCoinFlips(Zeroizing<Vec<u8>>),
}

impl CanonicalInput {
    #[must_use]
    pub(crate) fn from_capture(
        protocol: ConversionProtocol,
        target: EntropyTarget,
        rolls: &RollSequence,
        flips: &FlipSequence,
    ) -> Self {
        match protocol {
            ConversionProtocol::ExactV1 => Self::Base6Integer(base6_digits(rolls)),
            ConversionProtocol::WordExactV1 => Self::LocalizedBase6Candidates(base6_digits(rolls)),
            ConversionProtocol::NativeHashV1 => Self::VersionedBinary {
                header: native_hash_header(target),
                ascii_rolls: rolls.ascii_bytes(),
            },
            ConversionProtocol::ColdcardV1 => Self::AsciiFaceDigits(coldcard_ascii_rolls(rolls)),
            ConversionProtocol::KeystoneLegacyV1 => {
                Self::AsciiFacesWithSixAsZero(keystone_legacy_ascii_rolls(rolls))
            }
            ConversionProtocol::SeedSignerCoinsV1 => Self::AsciiCoinFlips(flips.ascii_bytes()),
        }
    }
}

fn base6_digits(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        rolls
            .faces()
            .iter()
            .map(|face| b'0' + face.base6_digit())
            .collect(),
    )
}

impl fmt::Debug for CanonicalInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CanonicalInput([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    #[test]
    fn representation_uses_protocol_owned_face_mapping() {
        let mut rolls = RollSequence::new();
        rolls.push(DieFace::new(1).unwrap());
        rolls.push(DieFace::new(6).unwrap());

        let CanonicalInput::Base6Integer(digits) = CanonicalInput::from_capture(
            ConversionProtocol::ExactV1,
            EntropyTarget::Words12,
            &rolls,
            &FlipSequence::new(),
        ) else {
            panic!("base-6 representation expected");
        };
        assert_eq!(digits.as_slice(), b"05");
    }
}
