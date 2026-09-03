use crate::domain::protocol::ConversionProtocol;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceScope {
    CompleteModule,
    SelectedFunctions(&'static [&'static str]),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProtocolSourceView {
    label: &'static str,
    path: &'static str,
    contents: &'static str,
    scope: SourceScope,
}

impl ProtocolSourceView {
    pub(crate) const fn label(self) -> &'static str {
        self.label
    }

    pub(crate) const fn path(self) -> &'static str {
        self.path
    }

    pub(crate) const fn scope(self) -> SourceScope {
        self.scope
    }

    pub(crate) fn lines(self) -> Vec<(usize, &'static str)> {
        match self.scope {
            SourceScope::CompleteModule => self
                .contents
                .lines()
                .enumerate()
                .map(|(line, contents)| (line + 1, contents))
                .collect(),
            SourceScope::SelectedFunctions(functions) => functions
                .iter()
                .enumerate()
                .flat_map(|(index, function)| {
                    let mut lines = self.function_lines(function);
                    if index > 0 {
                        lines.insert(0, (0, ""));
                    }
                    lines
                })
                .collect(),
        }
    }

    fn function_lines(self, function: &str) -> Vec<(usize, &'static str)> {
        let lines: Vec<_> = self.contents.lines().collect();
        let needle = format!("fn {function}(");
        let start = lines
            .iter()
            .position(|line| line.contains(&needle))
            .unwrap_or_else(|| panic!("{function} is missing from {}", self.path));
        let mut depth = 0_usize;
        let mut opened = false;

        for (offset, line) in lines[start..].iter().enumerate() {
            for character in line.chars() {
                match character {
                    '{' => {
                        opened = true;
                        depth += 1;
                    }
                    '}' if opened => depth -= 1,
                    _ => {}
                }
            }
            if opened && depth == 0 {
                return lines[start..=start + offset]
                    .iter()
                    .enumerate()
                    .map(|(line_offset, line)| (start + line_offset + 1, *line))
                    .collect();
            }
        }

        panic!("{function} is incomplete in {}", self.path);
    }
}

const fn complete_module(
    label: &'static str,
    path: &'static str,
    contents: &'static str,
) -> ProtocolSourceView {
    ProtocolSourceView {
        label,
        path,
        contents,
        scope: SourceScope::CompleteModule,
    }
}

const fn selected_functions(
    label: &'static str,
    path: &'static str,
    contents: &'static str,
    functions: &'static [&'static str],
) -> ProtocolSourceView {
    ProtocolSourceView {
        label,
        path,
        contents,
        scope: SourceScope::SelectedFunctions(functions),
    }
}

pub(crate) const BIP39_VERSION: &str = "2.2.2";
pub(crate) const BIP39_CHECKSUM: &str =
    "90dbd31c98227229239363921e60fcf5e558e43ec69094d46fc4996f08d1d5bc";
pub(crate) const BITCOIN_HASHES_VERSION: &str = "0.14.101";
pub(crate) const BITCOIN_HASHES_CHECKSUM: &str =
    "bca4c7abb40c8817d77403c880988cfd484f23ab2365726afb2f798363e2c4a2";

const MNEMONIC: ProtocolSourceView = complete_module(
    "COMMON ENTROPY → MNEMONIC",
    "crates/bip39-ceremony-core/src/domain/bip39/mnemonic.rs",
    include_str!("../../crates/bip39-ceremony-core/src/domain/bip39/mnemonic.rs"),
);
const BASE6: ProtocolSourceView = selected_functions(
    "SHARED BASE-6 INTEGER",
    "crates/bip39-ceremony-core/src/domain/protocol/base6.rs",
    include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/base6.rs"),
    &["accumulate", "multiply_add"],
);
const BIT_PACKING: ProtocolSourceView = complete_module(
    "SHARED MSB-FIRST PACKING",
    "crates/bip39-ceremony-core/src/domain/protocol/bit_packing.rs",
    include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/bit_packing.rs"),
);
const SHA256: ProtocolSourceView = selected_functions(
    "SHARED SHA-256 TRANSFORMATION",
    "crates/bip39-ceremony-core/src/domain/protocol/sha256.rs",
    include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/sha256.rs"),
    &["sha256_prefix_entropy", "sha256_digest"],
);

static EXACT: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/exact/conversion.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/exact/conversion.rs"),
        &["calculate_entropy"],
    ),
    BASE6,
];
static WORD_EXACT: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/word_exact/parser.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/word_exact/parser.rs"),
        &[
            "base6_value",
            "accepted_word_index",
            "accepted_entropy_tail",
            "assemble_entropy",
            "lower_u16",
        ],
    ),
    selected_functions(
        "METHOD PARAMETERS",
        "crates/bip39-ceremony-core/src/domain/protocol/word_exact/mod.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/word_exact/mod.rs"),
        &["tail_parameters"],
    ),
    BIT_PACKING,
];
static COLDCARD: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/coldcard/mod.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/coldcard/mod.rs"),
        &["calculate_entropy"],
    ),
    selected_functions(
        "METHOD PREIMAGE",
        "crates/bip39-ceremony-core/src/domain/protocol/coldcard/preimage.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/coldcard/preimage.rs"),
        &["ascii_rolls"],
    ),
    SHA256,
];
static KEYSTONE: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/keystone_legacy.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/keystone_legacy.rs"),
        &["ascii_rolls", "calculate_entropy"],
    ),
    SHA256,
];
static JADE: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/jade.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/jade.rs"),
        &[
            "jade_word_index",
            "words12_entropy_tail",
            "words24_entropy_tail",
            "assemble_entropy",
        ],
    ),
    BIT_PACKING,
];
static BITBOX: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/bitbox.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/bitbox.rs"),
        &[
            "calculate_word_index",
            "append_entropy_tail",
            "bitbox_tail_bits",
            "calculate_entropy",
        ],
    ),
    BIT_PACKING,
];
static KRUX: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/krux_d20.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/krux_d20.rs"),
        &["ascii_rolls", "calculate_entropy"],
    ),
    SHA256,
];
static COIN_FOUR_D6: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/coin_four_d6.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/coin_four_d6.rs"),
        &[
            "calculate_rank",
            "calculate_word_index",
            "calculate_entropy",
        ],
    ),
    BIT_PACKING,
];
static SEEDSIGNER: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/seedsigner_coins.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/seedsigner_coins.rs"),
        &["calculate_entropy"],
    ),
    selected_functions(
        "METHOD PREIMAGE",
        "crates/bip39-ceremony-core/src/domain/coin.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/coin.rs"),
        &["ascii_bytes"],
    ),
    SHA256,
];
static BITCOINLIB: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/bitcoinlib_base6.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/bitcoinlib_base6.rs"),
        &["calculate_entropy"],
    ),
    BASE6,
];
static BLUEWALLET: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/bluewallet_bitpack.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/bluewallet_bitpack.rs"),
        &["face_bits", "calculate_entropy"],
    ),
];
static IAN_COLEMAN_DICE: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/iancoleman.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/iancoleman.rs"),
        &["calculate_dice_entropy"],
    ),
    selected_functions(
        "METHOD PREIMAGE",
        "crates/bip39-ceremony-core/src/domain/protocol/keystone_legacy.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/keystone_legacy.rs"),
        &["ascii_rolls"],
    ),
    SHA256,
];
static IAN_COLEMAN_RAW: &[ProtocolSourceView] = &[
    MNEMONIC,
    selected_functions(
        "METHOD-SPECIFIC ENTROPY FORMULAS",
        "crates/bip39-ceremony-core/src/domain/protocol/iancoleman.rs",
        include_str!("../../crates/bip39-ceremony-core/src/domain/protocol/iancoleman.rs"),
        &["raw_face_bits", "calculate_raw_entropy"],
    ),
];

pub(crate) const fn protocol_source_files(
    protocol: ConversionProtocol,
) -> &'static [ProtocolSourceView] {
    match protocol {
        ConversionProtocol::ExactV1 => EXACT,
        ConversionProtocol::WordExactV1 => WORD_EXACT,
        ConversionProtocol::ColdcardV1 => COLDCARD,
        ConversionProtocol::KeystoneLegacyV1 => KEYSTONE,
        ConversionProtocol::JadeDirectV1 => JADE,
        ConversionProtocol::BitBox02DirectV1 => BITBOX,
        ConversionProtocol::KruxD20V1 => KRUX,
        ConversionProtocol::CoinFourD6DirectV1 => COIN_FOUR_D6,
        ConversionProtocol::SeedSignerCoinsV1 => SEEDSIGNER,
        ConversionProtocol::BitcoinLibBase6V1 => BITCOINLIB,
        ConversionProtocol::BlueWalletBitPackV1 => BLUEWALLET,
        ConversionProtocol::IanColemanDiceV1 => IAN_COLEMAN_DICE,
        ConversionProtocol::IanColemanRawV1 => IAN_COLEMAN_RAW,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const METHOD_FUNCTIONS: &[(ConversionProtocol, &str, &[&str])] = &[
        (
            ConversionProtocol::ExactV1,
            "exact/conversion.rs",
            &["calculate_entropy", "accumulate"],
        ),
        (
            ConversionProtocol::WordExactV1,
            "word_exact/parser.rs",
            &[
                "base6_value",
                "accepted_word_index",
                "accepted_entropy_tail",
                "assemble_entropy",
                "lower_u16",
                "tail_parameters",
                "append_bits",
            ],
        ),
        (
            ConversionProtocol::ColdcardV1,
            "coldcard/mod.rs",
            &["calculate_entropy", "ascii_rolls", "sha256_prefix_entropy"],
        ),
        (
            ConversionProtocol::KeystoneLegacyV1,
            "keystone_legacy.rs",
            &["ascii_rolls", "calculate_entropy", "sha256_prefix_entropy"],
        ),
        (
            ConversionProtocol::JadeDirectV1,
            "jade.rs",
            &[
                "jade_word_index",
                "words12_entropy_tail",
                "words24_entropy_tail",
                "assemble_entropy",
                "append_bits",
            ],
        ),
        (
            ConversionProtocol::BitBox02DirectV1,
            "bitbox.rs",
            &[
                "calculate_word_index",
                "append_entropy_tail",
                "bitbox_tail_bits",
                "calculate_entropy",
                "append_bits",
            ],
        ),
        (
            ConversionProtocol::KruxD20V1,
            "krux_d20.rs",
            &["ascii_rolls", "calculate_entropy", "sha256_prefix_entropy"],
        ),
        (
            ConversionProtocol::CoinFourD6DirectV1,
            "coin_four_d6.rs",
            &[
                "calculate_rank",
                "calculate_word_index",
                "calculate_entropy",
                "append_bits",
            ],
        ),
        (
            ConversionProtocol::SeedSignerCoinsV1,
            "seedsigner_coins.rs",
            &["calculate_entropy", "ascii_bytes", "sha256_prefix_entropy"],
        ),
        (
            ConversionProtocol::BitcoinLibBase6V1,
            "bitcoinlib_base6.rs",
            &["calculate_entropy", "accumulate"],
        ),
        (
            ConversionProtocol::BlueWalletBitPackV1,
            "bluewallet_bitpack.rs",
            &["face_bits", "calculate_entropy"],
        ),
        (
            ConversionProtocol::IanColemanDiceV1,
            "iancoleman.rs",
            &[
                "calculate_dice_entropy",
                "ascii_rolls",
                "sha256_prefix_entropy",
            ],
        ),
        (
            ConversionProtocol::IanColemanRawV1,
            "iancoleman.rs",
            &["raw_face_bits", "calculate_raw_entropy"],
        ),
    ];

    #[test]
    fn every_protocol_starts_with_the_complete_common_mnemonic_module() {
        for &(protocol, _, _) in METHOD_FUNCTIONS {
            let first = protocol_source_files(protocol)[0];
            assert_eq!(first, MNEMONIC, "{}", protocol.id());
            assert_eq!(first.scope(), SourceScope::CompleteModule);
            let rendered = first
                .lines()
                .into_iter()
                .map(|(_, line)| line)
                .collect::<Vec<_>>()
                .join("\n");
            assert!(rendered.contains("Mnemonic::from_entropy_in"));
            assert!(rendered.contains("encoded.to_entropy_array()"));
            assert!(rendered.contains(".word_indices()"));
            assert!(rendered.contains("encoded.checksum()"));
            assert!(rendered.contains("EnglishMnemonic::from_verified_words"));
        }
    }

    #[test]
    fn every_protocol_separates_method_math_from_shared_math() {
        for &(protocol, expected_path, expected_functions) in METHOD_FUNCTIONS {
            let views = protocol_source_files(protocol);
            assert!(
                views[1].path().ends_with(expected_path),
                "{}",
                protocol.id()
            );
            assert_eq!(views[1].label(), "METHOD-SPECIFIC ENTROPY FORMULAS");
            let rendered = views
                .iter()
                .skip(1)
                .flat_map(|view| view.lines())
                .map(|(_, line)| line)
                .collect::<Vec<_>>()
                .join("\n");
            for expected_function in expected_functions {
                assert!(
                    rendered.contains(&format!("fn {expected_function}(")),
                    "{}: {expected_function}",
                    protocol.id()
                );
            }
        }
    }

    #[test]
    fn selected_method_views_exclude_validation_and_rejection_gates() {
        for &(protocol, _, _) in METHOD_FUNCTIONS {
            for view in protocol_source_files(protocol).iter().skip(1) {
                let rendered = view
                    .lines()
                    .into_iter()
                    .map(|(_, line)| line)
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(!rendered.contains("#[cfg(test)]"), "{}", protocol.id());
                assert!(!rendered.contains("mod tests"), "{}", protocol.id());
                assert!(!rendered.contains("return Err("), "{}", protocol.id());
                assert!(!rendered.contains("::Rejected"), "{}", protocol.id());
                assert!(
                    !rendered.contains("distribution_is_rejected"),
                    "{}",
                    protocol.id()
                );
            }
        }
    }

    #[test]
    fn complete_module_lines_are_unabridged() {
        let expected = MNEMONIC
            .contents
            .lines()
            .enumerate()
            .map(|(line, contents)| (line + 1, contents))
            .collect::<Vec<_>>();

        assert_eq!(MNEMONIC.lines(), expected);
    }

    #[test]
    fn displayed_dependency_identity_matches_the_trust_ledger() {
        let ledger = include_str!("../../supply-chain/trust.toml");
        assert_trusted_dependency(ledger, "bip39", BIP39_VERSION, BIP39_CHECKSUM);
        assert_trusted_dependency(
            ledger,
            "bitcoin_hashes",
            BITCOIN_HASHES_VERSION,
            BITCOIN_HASHES_CHECKSUM,
        );
    }

    fn assert_trusted_dependency(ledger: &str, name: &str, version: &str, checksum: &str) {
        let package = ledger
            .split("[[cargo-package]]")
            .find(|package| package.contains(&format!("name = \"{name}\"")))
            .unwrap_or_else(|| panic!("{name} package has a trust record"));
        assert!(package.contains(&format!("version = \"{version}\"")));
        assert!(package.contains(&format!("checksum = \"{checksum}\"")));
    }
}
