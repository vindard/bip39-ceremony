use core::fmt;

use zeroize::Zeroize;

use super::{Bip39Error, EntropyTarget};

/// Secret English BIP-39 words derived from validated entropy.
#[derive(Eq, PartialEq)]
pub struct EnglishMnemonic {
    target: EntropyTarget,
    words: Vec<String>,
}

impl EnglishMnemonic {
    pub(crate) fn from_verified_words(
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
