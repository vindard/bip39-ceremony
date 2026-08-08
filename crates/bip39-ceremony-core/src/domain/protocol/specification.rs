use crate::domain::bip39::EntropyTarget;

use super::{ConversionProtocol, word_exact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalInputKind {
    Base6Integer,
    LocalizedBase6Candidates,
    AsciiFaceDigits,
    AsciiFacesWithSixAsZero,
    TypedMixedDice,
    TypedD6AndCoins,
    AsciiHyphenatedD20,
    AsciiCoinFlips,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectionPolicy {
    None,
    WholeSequence,
    LocalizedD6ToBase4,
    LocalizedCoinFourD6 {
        accepted_candidates: usize,
        candidate_limit: u16,
    },
    Localized {
        word_candidate_rolls: usize,
        word_candidate_limit: u32,
        tail_candidate_rolls: usize,
        tail_candidate_limit: u32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compatibility {
    Bip39Ceremony,
    Coldcard,
    KeystoneLegacy,
    JadeTable,
    BitBoxTable,
    Krux,
    CoinFourD6Table,
    SeedSigner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolSpecification {
    id: &'static str,
    supports_target: bool,
    minimum_observations: usize,
    accepts_optional_rolls: bool,
    canonical_input: CanonicalInputKind,
    rejection: RejectionPolicy,
    compatibility: Compatibility,
}

impl ProtocolSpecification {
    #[must_use]
    pub const fn id(self) -> &'static str {
        self.id
    }
    #[must_use]
    pub const fn supports_target(self) -> bool {
        self.supports_target
    }
    #[must_use]
    pub const fn minimum_observations(self) -> usize {
        self.minimum_observations
    }
    #[must_use]
    pub const fn accepts_optional_rolls(self) -> bool {
        self.accepts_optional_rolls
    }
    #[must_use]
    pub const fn canonical_input(self) -> CanonicalInputKind {
        self.canonical_input
    }
    #[must_use]
    pub const fn rejection(self) -> RejectionPolicy {
        self.rejection
    }
    #[must_use]
    pub const fn compatibility(self) -> Compatibility {
        self.compatibility
    }
}

impl ConversionProtocol {
    #[must_use]
    pub const fn specification(self, target: EntropyTarget) -> ProtocolSpecification {
        let (canonical_input, rejection, compatibility, accepts_optional_rolls) = match self {
            Self::ExactV1 => (
                CanonicalInputKind::Base6Integer,
                RejectionPolicy::WholeSequence,
                Compatibility::Bip39Ceremony,
                false,
            ),
            Self::WordExactV1 => {
                let (tail_rolls, tail_limit, _, _) = word_exact::tail_parameters(target);
                (
                    CanonicalInputKind::LocalizedBase6Candidates,
                    RejectionPolicy::Localized {
                        word_candidate_rolls: word_exact::WORD_CANDIDATE_ROLLS,
                        word_candidate_limit: word_exact::WORD_CANDIDATE_LIMIT,
                        tail_candidate_rolls: tail_rolls,
                        tail_candidate_limit: tail_limit,
                    },
                    Compatibility::Bip39Ceremony,
                    false,
                )
            }
            Self::ColdcardV1 => (
                CanonicalInputKind::AsciiFaceDigits,
                RejectionPolicy::WholeSequence,
                Compatibility::Coldcard,
                true,
            ),
            Self::KeystoneLegacyV1 => (
                CanonicalInputKind::AsciiFacesWithSixAsZero,
                RejectionPolicy::None,
                Compatibility::KeystoneLegacy,
                true,
            ),
            Self::JadeDirectV1 => (
                CanonicalInputKind::TypedMixedDice,
                RejectionPolicy::None,
                Compatibility::JadeTable,
                false,
            ),
            Self::BitBox02DirectV1 => (
                CanonicalInputKind::TypedD6AndCoins,
                RejectionPolicy::LocalizedD6ToBase4,
                Compatibility::BitBoxTable,
                false,
            ),
            Self::KruxD20V1 => (
                CanonicalInputKind::AsciiHyphenatedD20,
                RejectionPolicy::None,
                Compatibility::Krux,
                true,
            ),
            Self::CoinFourD6DirectV1 => (
                CanonicalInputKind::TypedD6AndCoins,
                RejectionPolicy::LocalizedCoinFourD6 {
                    accepted_candidates: 12,
                    candidate_limit: 2_048,
                },
                Compatibility::CoinFourD6Table,
                false,
            ),
            Self::SeedSignerCoinsV1 => (
                CanonicalInputKind::AsciiCoinFlips,
                RejectionPolicy::None,
                Compatibility::SeedSigner,
                false,
            ),
        };
        ProtocolSpecification {
            id: self.id(),
            supports_target: self.supports_target(target),
            minimum_observations: self.minimum_observations(target),
            accepts_optional_rolls,
            canonical_input,
            rejection,
            compatibility,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specifications_expose_auditable_protocol_facts() {
        let word_exact = ConversionProtocol::WordExactV1.specification(EntropyTarget::Words12);
        assert_eq!(word_exact.minimum_observations(), 70);
        assert_eq!(
            word_exact.canonical_input(),
            CanonicalInputKind::LocalizedBase6Candidates
        );
        assert_eq!(
            word_exact.rejection(),
            RejectionPolicy::Localized {
                word_candidate_rolls: 6,
                word_candidate_limit: 45_056,
                tail_candidate_rolls: 4,
                tail_candidate_limit: 1_280,
            }
        );

        let coldcard = ConversionProtocol::ColdcardV1.specification(EntropyTarget::Words24);
        assert_eq!(coldcard.minimum_observations(), 99);
        assert!(coldcard.accepts_optional_rolls());
        assert_eq!(coldcard.rejection(), RejectionPolicy::WholeSequence);
        assert_eq!(
            coldcard.canonical_input(),
            CanonicalInputKind::AsciiFaceDigits
        );
        assert_eq!(coldcard.compatibility(), Compatibility::Coldcard);
    }

    #[test]
    fn jade_specification_exposes_typed_mixed_dice() {
        let specification = ConversionProtocol::JadeDirectV1.specification(EntropyTarget::Words12);
        assert_eq!(specification.minimum_observations(), 35);
        assert_eq!(
            specification.canonical_input(),
            CanonicalInputKind::TypedMixedDice
        );
        assert_eq!(specification.compatibility(), Compatibility::JadeTable);
    }

    #[test]
    fn bitbox_specification_exposes_local_rejection_and_typed_capture() {
        let specification =
            ConversionProtocol::BitBox02DirectV1.specification(EntropyTarget::Words24);
        assert_eq!(specification.minimum_observations(), 141);
        assert_eq!(
            specification.canonical_input(),
            CanonicalInputKind::TypedD6AndCoins
        );
        assert_eq!(
            specification.rejection(),
            RejectionPolicy::LocalizedD6ToBase4
        );
        assert_eq!(specification.compatibility(), Compatibility::BitBoxTable);
    }

    #[test]
    fn krux_specification_exposes_hyphenated_open_capture() {
        let specification = ConversionProtocol::KruxD20V1.specification(EntropyTarget::Words12);
        assert_eq!(specification.minimum_observations(), 30);
        assert_eq!(
            specification.canonical_input(),
            CanonicalInputKind::AsciiHyphenatedD20
        );
        assert_eq!(specification.compatibility(), Compatibility::Krux);
        assert!(specification.accepts_optional_rolls());
    }

    #[test]
    fn coin_four_d6_specification_exposes_whole_candidate_rejection() {
        let specification =
            ConversionProtocol::CoinFourD6DirectV1.specification(EntropyTarget::Words12);
        assert_eq!(specification.minimum_observations(), 60);
        assert_eq!(
            specification.rejection(),
            RejectionPolicy::LocalizedCoinFourD6 {
                accepted_candidates: 12,
                candidate_limit: 2_048,
            }
        );
        assert_eq!(
            specification.compatibility(),
            Compatibility::CoinFourD6Table
        );
        assert!(!specification.accepts_optional_rolls());
    }

    #[test]
    fn seedsigner_specification_exposes_ascii_coin_hash_input() {
        let specification =
            ConversionProtocol::SeedSignerCoinsV1.specification(EntropyTarget::Words12);
        assert_eq!(specification.minimum_observations(), 128);
        assert_eq!(
            specification.canonical_input(),
            CanonicalInputKind::AsciiCoinFlips
        );
        assert_eq!(specification.compatibility(), Compatibility::SeedSigner);
        assert!(!specification.accepts_optional_rolls());
    }

    #[test]
    fn keystone_specification_exposes_the_legacy_mapping() {
        let keystone = ConversionProtocol::KeystoneLegacyV1.specification(EntropyTarget::Words24);
        assert!(keystone.supports_target());
        assert!(
            !ConversionProtocol::KeystoneLegacyV1
                .specification(EntropyTarget::Words12)
                .supports_target()
        );
        assert_eq!(keystone.minimum_observations(), 50);
        assert!(keystone.accepts_optional_rolls());
        assert_eq!(
            keystone.canonical_input(),
            CanonicalInputKind::AsciiFacesWithSixAsZero
        );
        assert_eq!(keystone.compatibility(), Compatibility::KeystoneLegacy);
    }
}
