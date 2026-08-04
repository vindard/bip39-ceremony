/// The BIP-39 entropy size and corresponding mnemonic length.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntropyTarget {
    Words12,
    Words24,
}

impl EntropyTarget {
    #[must_use]
    pub const fn entropy_bits(self) -> usize {
        match self {
            Self::Words12 => 128,
            Self::Words24 => 256,
        }
    }

    #[must_use]
    pub const fn entropy_bytes(self) -> usize {
        self.entropy_bits() / 8
    }

    #[must_use]
    pub const fn checksum_bits(self) -> usize {
        self.entropy_bits() / 32
    }

    #[must_use]
    pub const fn word_count(self) -> usize {
        match self {
            Self::Words12 => 12,
            Self::Words24 => 24,
        }
    }

    #[must_use]
    pub const fn strict_roll_count(self) -> usize {
        match self {
            Self::Words12 => 50,
            Self::Words24 => 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_define_bip39_sizes() {
        let cases = [
            (EntropyTarget::Words12, 128, 16, 12, 50),
            (EntropyTarget::Words24, 256, 32, 24, 100),
        ];

        for (target, bits, bytes, words, rolls) in cases {
            assert_eq!(target.entropy_bits(), bits);
            assert_eq!(target.entropy_bytes(), bytes);
            assert_eq!(target.checksum_bits(), bits / 32);
            assert_eq!(target.word_count(), words);
            assert_eq!(target.strict_roll_count(), rolls);
        }
    }
}
