use crate::domain::bip39::EntropyTarget;

/// A stable, versioned physical-capture-to-entropy protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversionProtocol {
    ExactV1,
    WordExactV1,
    ColdcardV1,
    KeystoneLegacyV1,
    JadeDirectV1,
    BitBox02DirectV1,
    KruxD20V1,
    CoinFourD6DirectV1,
    SeedSignerCoinsV1,
    BitcoinLibBase6V1,
    BlueWalletBitPackV1,
    IanColemanDiceV1,
    IanColemanRawV1,
}

impl ConversionProtocol {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::ExactV1 => "exact-v1",
            Self::WordExactV1 => "word-exact-v1",
            Self::ColdcardV1 => "coldcard-v1",
            Self::KeystoneLegacyV1 => "keystone-legacy-v1",
            Self::JadeDirectV1 => "jade-direct-v1",
            Self::BitBox02DirectV1 => "bitbox02-direct-v1",
            Self::KruxD20V1 => "krux-d20-v1",
            Self::CoinFourD6DirectV1 => "coin-four-d6-direct-v1",
            Self::SeedSignerCoinsV1 => "seedsigner-coins-v1",
            Self::BitcoinLibBase6V1 => "bitcoinlib-base6-v1",
            Self::BlueWalletBitPackV1 => "bluewallet-bitpack-v1",
            Self::IanColemanDiceV1 => "iancoleman-dice-v1",
            Self::IanColemanRawV1 => "iancoleman-raw-v1",
        }
    }

    #[must_use]
    pub const fn minimum_observations(self, target: EntropyTarget) -> usize {
        match (self, target) {
            (Self::WordExactV1, EntropyTarget::Words12)
            | (Self::JadeDirectV1, EntropyTarget::Words24) => 70,
            (Self::WordExactV1, EntropyTarget::Words24) => 140,
            (Self::ColdcardV1 | Self::BitcoinLibBase6V1, EntropyTarget::Words24) => 99,
            (Self::KeystoneLegacyV1, EntropyTarget::Words24) => 50,
            (Self::JadeDirectV1, EntropyTarget::Words12) => 35,
            (Self::BitBox02DirectV1, EntropyTarget::Words12) => 73,
            (Self::BitBox02DirectV1, EntropyTarget::Words24) => 141,
            (Self::KruxD20V1, EntropyTarget::Words12) => 30,
            (Self::KruxD20V1, EntropyTarget::Words24) | (Self::CoinFourD6DirectV1, _) => 60,
            (Self::SeedSignerCoinsV1, target) => target.entropy_bits(),
            (Self::BlueWalletBitPackV1 | Self::IanColemanRawV1, target) => {
                target.entropy_bits() / 2
            }
            // The web tool's fixed-length path enforces no minimum at all and
            // warns instead, so it falls through to the count that fills the
            // target under fair dice.
            (_, target) => target.strict_roll_count(),
        }
    }

    #[must_use]
    pub const fn supports_target(self, target: EntropyTarget) -> bool {
        !matches!(
            (self, target),
            (Self::KeystoneLegacyV1, EntropyTarget::Words12)
                | (Self::CoinFourD6DirectV1, EntropyTarget::Words24)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocols_define_stable_ids_and_capture_requirements() {
        let cases = [
            (ConversionProtocol::ExactV1, "exact-v1", 50, 100),
            (ConversionProtocol::WordExactV1, "word-exact-v1", 70, 140),
            (ConversionProtocol::ColdcardV1, "coldcard-v1", 50, 99),
            (
                ConversionProtocol::KeystoneLegacyV1,
                "keystone-legacy-v1",
                50,
                50,
            ),
            (ConversionProtocol::JadeDirectV1, "jade-direct-v1", 35, 70),
            (
                ConversionProtocol::BitBox02DirectV1,
                "bitbox02-direct-v1",
                73,
                141,
            ),
            (ConversionProtocol::KruxD20V1, "krux-d20-v1", 30, 60),
            (
                ConversionProtocol::CoinFourD6DirectV1,
                "coin-four-d6-direct-v1",
                60,
                60,
            ),
            (
                ConversionProtocol::SeedSignerCoinsV1,
                "seedsigner-coins-v1",
                128,
                256,
            ),
            (
                ConversionProtocol::BitcoinLibBase6V1,
                "bitcoinlib-base6-v1",
                50,
                99,
            ),
            (
                ConversionProtocol::BlueWalletBitPackV1,
                "bluewallet-bitpack-v1",
                64,
                128,
            ),
            (
                ConversionProtocol::IanColemanDiceV1,
                "iancoleman-dice-v1",
                50,
                100,
            ),
            (
                ConversionProtocol::IanColemanRawV1,
                "iancoleman-raw-v1",
                64,
                128,
            ),
        ];

        for (protocol, id, rolls12, rolls24) in cases {
            assert_eq!(protocol.id(), id);
            assert_eq!(
                protocol.minimum_observations(EntropyTarget::Words12),
                rolls12
            );
            assert_eq!(
                protocol.minimum_observations(EntropyTarget::Words24),
                rolls24
            );
        }
        assert!(!ConversionProtocol::KeystoneLegacyV1.supports_target(EntropyTarget::Words12));
        assert!(ConversionProtocol::KeystoneLegacyV1.supports_target(EntropyTarget::Words24));
        assert!(ConversionProtocol::CoinFourD6DirectV1.supports_target(EntropyTarget::Words12));
        assert!(!ConversionProtocol::CoinFourD6DirectV1.supports_target(EntropyTarget::Words24));
    }
}
