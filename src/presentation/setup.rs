use crate::domain::{bip39::EntropyTarget, protocol::ConversionProtocol};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceContent {
    name: String,
    detail: String,
}

impl ChoiceContent {
    #[must_use]
    pub const fn new(name: String, detail: String) -> Self {
        Self { name, detail }
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

#[must_use]
pub fn target_choices() -> Vec<ChoiceContent> {
    [EntropyTarget::Words12, EntropyTarget::Words24]
        .into_iter()
        .map(|target| {
            ChoiceContent::new(
                format!("{} words", target.word_count()),
                format!(
                    "{} entropy bits + {} checksum bits · {} backup",
                    target.entropy_bits(),
                    target.checksum_bits(),
                    if target == EntropyTarget::Words12 {
                        "shorter"
                    } else {
                        "longer"
                    }
                ),
            )
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolMenuChoice {
    Coldcard,
    WordExact,
    Exact,
    KeystoneLegacyDice,
    JadeDirectWords,
    BitBoxDiceware,
    KruxD20,
    CoinFourD6Direct,
    SeedSignerCoins,
}

impl ProtocolMenuChoice {
    pub const ALL: [Self; 9] = [
        Self::Coldcard,
        Self::WordExact,
        Self::Exact,
        Self::KeystoneLegacyDice,
        Self::JadeDirectWords,
        Self::BitBoxDiceware,
        Self::KruxD20,
        Self::CoinFourD6Direct,
        Self::SeedSignerCoins,
    ];

    #[must_use]
    pub const fn implemented_protocol(self, target: EntropyTarget) -> Option<ConversionProtocol> {
        match self {
            Self::Coldcard => Some(ConversionProtocol::ColdcardV1),
            Self::WordExact => Some(ConversionProtocol::WordExactV1),
            Self::Exact => Some(ConversionProtocol::ExactV1),
            Self::KeystoneLegacyDice if matches!(target, EntropyTarget::Words24) => {
                Some(ConversionProtocol::KeystoneLegacyV1)
            }
            Self::JadeDirectWords => Some(ConversionProtocol::JadeDirectV1),
            Self::BitBoxDiceware => Some(ConversionProtocol::BitBox02DirectV1),
            Self::KruxD20 => Some(ConversionProtocol::KruxD20V1),
            Self::CoinFourD6Direct if matches!(target, EntropyTarget::Words12) => {
                Some(ConversionProtocol::CoinFourD6DirectV1)
            }
            Self::SeedSignerCoins => Some(ConversionProtocol::SeedSignerCoinsV1),
            Self::KeystoneLegacyDice | Self::CoinFourD6Direct => None,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Coldcard => "COLDCARD",
            Self::WordExact => "Word-by-word Exact",
            Self::Exact => "Exact",
            Self::KeystoneLegacyDice => "Keystone legacy dice",
            Self::JadeDirectWords => "Jade direct words",
            Self::BitBoxDiceware => "BitBox02 Diceware",
            Self::KruxD20 => "Krux D20",
            Self::CoinFourD6Direct => "Coin + four-D6 direct words",
            Self::SeedSignerCoins => "SeedSigner coin flips",
        }
    }
}

#[must_use]
pub fn capture_protocol_context(protocol: ConversionProtocol) -> ChoiceContent {
    let (name, detail) = match protocol {
        ConversionProtocol::ColdcardV1 => (
            "COLDCARD",
            "ASCII face digits → SHA-256 → target-width BIP-39 entropy",
        ),
        ConversionProtocol::WordExactV1 => (
            "Word-by-word Exact",
            "localized base-6 rejection → BIP-39 entropy positions",
        ),
        ConversionProtocol::ExactV1 => (
            "Exact",
            "whole-stream base-6 rejection → target-width BIP-39 entropy",
        ),
        ConversionProtocol::KeystoneLegacyV1 => (
            "Keystone legacy dice",
            "map face 6 to ASCII 0 → SHA-256 → 256-bit BIP-39 entropy",
        ),
        ConversionProtocol::JadeDirectV1 => (
            "Jade direct words",
            "D16/D16/D8 direct indices + physical entropy tail → BIP-39",
        ),
        ConversionProtocol::BitBox02DirectV1 => (
            "BitBox02 Diceware",
            "local D6 rejection + coin-selected direct indices and entropy tail",
        ),
        ConversionProtocol::KruxD20V1 => (
            "Krux D20",
            "hyphen-separated decimal D20 faces → SHA-256 → BIP-39 entropy",
        ),
        ConversionProtocol::CoinFourD6DirectV1 => (
            "Coin + four-D6 direct words",
            "whole-candidate rejection → direct indices → calculated checksum",
        ),
        ConversionProtocol::SeedSignerCoinsV1 => (
            "SeedSigner coin flips",
            "ASCII 0/1 flip string → SHA-256 → target-width BIP-39 entropy",
        ),
    };
    ChoiceContent::new(name.to_owned(), detail.to_owned())
}

#[must_use]
pub fn protocol_choices(target: EntropyTarget) -> Vec<ChoiceContent> {
    ProtocolMenuChoice::ALL
        .into_iter()
        .map(|choice| {
            let detail = match choice.implemented_protocol(target) {
                Some(protocol) => {
                    let specification = protocol.specification(target);
                    let summary = match choice {
                        ProtocolMenuChoice::Coldcard => {
                            "Best compatibility · externally reproducible SHA-256"
                        }
                        ProtocolMenuChoice::WordExact => {
                            "Best transparency · accepted positions stay kept"
                        }
                        ProtocolMenuChoice::Exact => {
                            "Cleanest exact proof · may need a full re-roll"
                        }
                        ProtocolMenuChoice::KeystoneLegacyDice => {
                            "Legacy compatibility · face 6 maps to ASCII 0"
                        }
                        ProtocolMenuChoice::JadeDirectWords => {
                            "Published Jade table · physical final entropy selector"
                        }
                        ProtocolMenuChoice::BitBoxDiceware => {
                            "Published BitBox02 table · local D6 rejection + coin"
                        }
                        ProtocolMenuChoice::KruxD20 => {
                            "Krux compatibility · hyphenated D20 SHA-256"
                        }
                        ProtocolMenuChoice::CoinFourD6Direct => {
                            "Physical direct table · whole coin-plus-four-D6 rejection"
                        }
                        ProtocolMenuChoice::SeedSignerCoins => {
                            "Coin compatibility · fixed ASCII binary capture"
                        }
                    };
                    let roll_requirement = if protocol == ConversionProtocol::SeedSignerCoinsV1 {
                        format!("exactly {} flips", specification.minimum_observations())
                    } else if protocol == ConversionProtocol::JadeDirectV1 {
                        format!(
                            "exactly {} mixed-die rolls",
                            specification.minimum_observations()
                        )
                    } else if protocol == ConversionProtocol::CoinFourD6DirectV1 {
                        format!(
                            "min {} outcomes + rejected five-outcome retries",
                            specification.minimum_observations()
                        )
                    } else if protocol == ConversionProtocol::BitBox02DirectV1 {
                        format!(
                            "min {} outcomes + rejected D6 retries",
                            specification.minimum_observations()
                        )
                    } else if protocol == ConversionProtocol::ExactV1 {
                        format!("exactly {} rolls", specification.minimum_observations())
                    } else {
                        format!("min {} rolls", specification.minimum_observations())
                    };
                    format!("{summary} · {roll_requirement}")
                }
                None => match choice {
                    ProtocolMenuChoice::KeystoneLegacyDice => {
                        "! UNSUPPORTED · available for 24 words only".to_owned()
                    }
                    ProtocolMenuChoice::CoinFourD6Direct => {
                        "! UNSUPPORTED · sourced table defines 12 words only".to_owned()
                    }
                    _ => unreachable!("all other menu choices support both targets"),
                },
            };
            ChoiceContent::new(choice.name().to_owned(), detail)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_context_names_the_active_conversion() {
        let coldcard = capture_protocol_context(ConversionProtocol::ColdcardV1);
        assert_eq!(coldcard.name(), "COLDCARD");
        assert!(coldcard.detail().contains("ASCII face digits"));

        let keystone = capture_protocol_context(ConversionProtocol::KeystoneLegacyV1);
        assert!(keystone.detail().contains("face 6 to ASCII 0"));
    }

    #[test]
    fn setup_copy_is_derived_from_domain_targets_and_protocols() {
        let targets = target_choices();
        assert!(
            targets[0]
                .detail()
                .contains("128 entropy bits + 4 checksum bits")
        );
        let unsupported = protocol_choices(EntropyTarget::Words12);
        assert_eq!(
            unsupported[3].detail(),
            "! UNSUPPORTED · available for 24 words only"
        );

        let protocols = protocol_choices(EntropyTarget::Words24);
        assert!(protocols[0].detail().contains("min 99 rolls"));
        assert!(protocols[1].detail().contains("Best transparency"));
        assert!(protocols[2].detail().contains("exactly 100 rolls"));
        assert_eq!(protocols.len(), ProtocolMenuChoice::ALL.len());
        assert_eq!(protocols[3].name(), "Keystone legacy dice");
        assert!(protocols[3].detail().contains("Legacy compatibility"));
        assert!(protocols[8].detail().contains("exactly 256 flips"));
    }

    #[test]
    fn coin_four_d6_choice_is_twelve_word_only() {
        let twelve = protocol_choices(EntropyTarget::Words12);
        assert!(twelve[7].detail().contains("min 60 outcomes"));
        let twenty_four = protocol_choices(EntropyTarget::Words24);
        assert_eq!(
            twenty_four[7].detail(),
            "! UNSUPPORTED · sourced table defines 12 words only"
        );
    }

    #[test]
    fn bitbox_choice_exposes_rejection_aware_minimum() {
        let twelve = protocol_choices(EntropyTarget::Words12);
        let twenty_four = protocol_choices(EntropyTarget::Words24);
        assert!(
            twelve[5]
                .detail()
                .contains("min 73 outcomes + rejected D6 retries")
        );
        assert!(twenty_four[5].detail().contains("min 141 outcomes"));
    }

    #[test]
    fn krux_choice_exposes_target_specific_open_minimum() {
        let twelve = protocol_choices(EntropyTarget::Words12);
        let twenty_four = protocol_choices(EntropyTarget::Words24);
        assert!(twelve[6].detail().contains("min 30 rolls"));
        assert!(twenty_four[6].detail().contains("min 60 rolls"));
    }

    #[test]
    fn jade_choice_exposes_target_specific_mixed_dice_count() {
        let twelve = protocol_choices(EntropyTarget::Words12);
        let twenty_four = protocol_choices(EntropyTarget::Words24);
        assert!(twelve[4].detail().contains("exactly 35 mixed-die rolls"));
        assert!(
            twenty_four[4]
                .detail()
                .contains("exactly 70 mixed-die rolls")
        );
    }
}
