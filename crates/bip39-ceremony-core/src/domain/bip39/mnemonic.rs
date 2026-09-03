use core::fmt;

use bip39::{Language, Mnemonic};
use zeroize::{Zeroize, Zeroizing};

use super::{Bip39Error, Entropy, EntropyTarget};

pub(crate) struct Bip39Encoding {
    mnemonic: EnglishMnemonic,
    checksum_bits: Vec<u8>,
    word_indices: Vec<u16>,
}

impl Bip39Encoding {
    pub(crate) fn from_entropy(entropy: &Entropy) -> Result<Self, Bip39Error> {
        let encoded = Mnemonic::from_entropy_in(Language::English, entropy.bytes())
            .map_err(|_| Bip39Error::EncodingFailed)?;
        Self::from_encoded(entropy, &encoded)
    }

    pub(super) fn from_encoded(entropy: &Entropy, encoded: &Mnemonic) -> Result<Self, Bip39Error> {
        let (round_trip, round_trip_len) = encoded.to_entropy_array();
        let round_trip = Zeroizing::new(round_trip);
        if &round_trip[..round_trip_len] != entropy.bytes() {
            return Err(Bip39Error::EncodingFailed);
        }

        let words = encoded.words().map(str::to_owned).collect();
        let mnemonic = EnglishMnemonic::from_verified_words(entropy.target(), words)?;
        let word_indices = encoded
            .word_indices()
            .map(|index| u16::try_from(index).map_err(|_| Bip39Error::EncodingFailed))
            .collect::<Result<_, _>>()?;
        let checksum_width = entropy.target().entropy_bits() / 32;
        let checksum = encoded.checksum();
        let checksum_bits = (0..checksum_width)
            .map(|offset| (checksum >> (checksum_width - offset - 1)) & 1)
            .collect();

        Ok(Self {
            mnemonic,
            checksum_bits,
            word_indices,
        })
    }

    pub(crate) fn into_parts(self) -> (EnglishMnemonic, Vec<u8>, Vec<u16>) {
        (self.mnemonic, self.checksum_bits, self.word_indices)
    }
}

/// Secret English BIP-39 words derived from validated entropy.
#[derive(Eq, PartialEq)]
pub struct EnglishMnemonic {
    target: EntropyTarget,
    words: Vec<String>,
}

impl EnglishMnemonic {
    fn from_verified_words(
        target: EntropyTarget,
        mut words: Vec<String>,
    ) -> Result<Self, Bip39Error> {
        if words.len() == target.word_count() {
            Ok(Self { target, words })
        } else {
            words.zeroize();
            Err(Bip39Error::EncodingFailed)
        }
    }

    #[must_use]
    pub const fn target(&self) -> EntropyTarget {
        self.target
    }

    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }
}

impl Drop for EnglishMnemonic {
    fn drop(&mut self) {
        self.words.zeroize();
    }
}

impl fmt::Debug for EnglishMnemonic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnglishMnemonic")
            .field("target", &self.target)
            .field("words", &"[REDACTED]")
            .finish()
    }
}
