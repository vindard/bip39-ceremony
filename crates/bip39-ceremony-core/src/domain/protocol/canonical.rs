use core::fmt;

use zeroize::Zeroizing;

use crate::domain::{
    bitbox::BitBoxCapture, coin::FlipSequence, coin_four_d6::CoinFourD6Capture,
    d20::D20RollSequence, dice::RollSequence, jade::JadeCapture,
};

use super::{
    ConversionProtocol, coldcard_ascii_rolls, keystone_legacy_ascii_rolls, krux_d20_ascii_rolls,
};

/// Secret protocol input represented without presentation wording.
pub enum CanonicalInput {
    Base6Integer(Zeroizing<Vec<u8>>),
    LocalizedBase6Candidates(Zeroizing<Vec<u8>>),
    AsciiFaceDigits(Zeroizing<Vec<u8>>),
    AsciiFacesWithSixAsZero(Zeroizing<Vec<u8>>),
    TypedMixedDice(Zeroizing<Vec<u8>>),
    TypedD6AndCoins(Zeroizing<Vec<u8>>),
    AsciiHyphenatedD20(Zeroizing<Vec<u8>>),
    AsciiCoinFlips(Zeroizing<Vec<u8>>),
}

impl CanonicalInput {
    #[must_use]
    pub(crate) fn from_dice(protocol: ConversionProtocol, rolls: &RollSequence) -> Self {
        match protocol {
            ConversionProtocol::ExactV1 | ConversionProtocol::BitcoinLibBase6V1 => {
                Self::Base6Integer(base6_digits(rolls))
            }
            ConversionProtocol::WordExactV1 => Self::LocalizedBase6Candidates(base6_digits(rolls)),
            ConversionProtocol::ColdcardV1 => Self::AsciiFaceDigits(coldcard_ascii_rolls(rolls)),
            ConversionProtocol::KeystoneLegacyV1 => {
                Self::AsciiFacesWithSixAsZero(keystone_legacy_ascii_rolls(rolls))
            }
            _ => unreachable!("typed protocols use typed canonical constructors"),
        }
    }

    #[must_use]
    pub(crate) fn from_jade(capture: &JadeCapture) -> Self {
        Self::TypedMixedDice(capture.audit_bytes())
    }

    #[must_use]
    pub(crate) fn from_bitbox(capture: &BitBoxCapture) -> Self {
        Self::TypedD6AndCoins(capture.audit_bytes())
    }

    #[must_use]
    pub(crate) fn from_coin_four_d6(capture: &CoinFourD6Capture) -> Self {
        Self::TypedD6AndCoins(capture.audit_bytes())
    }

    #[must_use]
    pub(crate) fn from_d20(capture: &D20RollSequence) -> Self {
        Self::AsciiHyphenatedD20(krux_d20_ascii_rolls(capture))
    }

    #[must_use]
    pub(crate) fn from_coins(capture: &FlipSequence) -> Self {
        Self::AsciiCoinFlips(capture.ascii_bytes())
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

        let CanonicalInput::Base6Integer(digits) =
            CanonicalInput::from_dice(ConversionProtocol::ExactV1, &rolls)
        else {
            panic!("base-6 representation expected");
        };
        assert_eq!(digits.as_slice(), b"05");
    }
}
