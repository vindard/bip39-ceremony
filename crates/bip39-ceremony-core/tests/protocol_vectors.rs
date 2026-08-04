use std::fmt::Write;

use bip39::{Language, Mnemonic};
use bip39_ceremony_core::{
    CalculationError, CalculationOutcome, Capture, CoinFlip, ConversionProtocol, D8Face, D16Face,
    DieFace, Entropy, EntropyTarget, FlipSequence, JadeCapture, JadeDieKind, ProtocolError,
    RollSequence, calculate, jade_expected_die, jade_required_observations,
};
use bitcoin_hashes::{Hash, sha256};

fn flips(value: &str) -> FlipSequence {
    let mut flips = FlipSequence::new();
    for character in value.chars() {
        flips.push(CoinFlip::try_from(character).unwrap());
    }
    flips
}

fn jade_all_ones(target: EntropyTarget) -> JadeCapture {
    let mut capture = JadeCapture::new();
    for offset in 0..jade_required_observations(target) {
        match jade_expected_die(target, offset).unwrap() {
            JadeDieKind::D16 => capture.push_d16(D16Face::new(1).unwrap()),
            JadeDieKind::D8 => capture.push_d8(D8Face::new(1).unwrap()),
        }
    }
    capture
}

fn jade_all_max(target: EntropyTarget) -> JadeCapture {
    let mut capture = JadeCapture::new();
    for offset in 0..jade_required_observations(target) {
        match jade_expected_die(target, offset).unwrap() {
            JadeDieKind::D16 => capture.push_d16(D16Face::new(16).unwrap()),
            JadeDieKind::D8 => capture.push_d8(D8Face::new(8).unwrap()),
        }
    }
    capture
}

fn rolls(value: &str) -> RollSequence {
    let mut rolls = RollSequence::new();
    for character in value.chars() {
        rolls.push(DieFace::try_from(character).unwrap());
    }
    rolls
}

fn entropy_hex(entropy: &Entropy) -> String {
    entropy.bytes().iter().fold(String::new(), |mut hex, byte| {
        write!(hex, "{byte:02x}").unwrap();
        hex
    })
}

fn accepted(
    target: EntropyTarget,
    protocol: ConversionProtocol,
    capture: Capture<'_>,
) -> bip39_ceremony_core::Calculation {
    let CalculationOutcome::Accepted(calculation) = calculate(target, protocol, capture).unwrap()
    else {
        panic!("vector must be accepted");
    };
    calculation
}

#[test]
fn exact_zero_crosses_conversion_and_bip39_boundaries() {
    let rolls = rolls(&"1".repeat(50));
    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::ExactV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "00000000000000000000000000000000"
    );
    assert_eq!(
        calculation.mnemonic().words().join(" "),
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    );
}

#[test]
fn word_exact_zero_crosses_rejection_and_bip39_boundaries() {
    let rolls = rolls(&"1".repeat(70));
    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::WordExactV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "00000000000000000000000000000000"
    );
    assert_eq!(calculation.mnemonic().words()[11], "about");
}

#[test]
fn native_hash_crosses_sha256_and_bip39_boundaries() {
    let rolls = rolls(&"1".repeat(50));
    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::NativeHashV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "c6ea190b0a8106d07e8d8c0ef5ca33d5"
    );
    assert_eq!(calculation.mnemonic().words().len(), 12);
}

#[test]
fn coldcard_documented_short_example_matches_sha256_and_bip39() {
    let digest = sha256::Hash::hash(b"123456").to_byte_array();
    assert_eq!(
        digest,
        [
            0x8d, 0x96, 0x9e, 0xef, 0x6e, 0xca, 0xd3, 0xc2, 0x9a, 0x3a, 0x62, 0x92, 0x80, 0xe6,
            0x86, 0xcf, 0x0c, 0x3f, 0x5d, 0x5a, 0x86, 0xaf, 0xf3, 0xca, 0x12, 0x02, 0x0c, 0x92,
            0x3a, 0xdc, 0x6c, 0x92,
        ]
    );
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &digest[..16]).unwrap();
    assert_eq!(
        mnemonic.to_string(),
        "mirror reject rookie talk pudding throw happy era myth already payment owner"
    );
}

#[test]
fn keystone_legacy_documented_vector_matches_mapping_and_bip39() {
    let source = "51236422654236551235532545533355551153256611442361";
    let digest = sha256::Hash::hash(source.replace('6', "0").as_bytes()).to_byte_array();
    assert_eq!(
        entropy_hex(&Entropy::new(EntropyTarget::Words24, digest.to_vec()).unwrap()),
        "19b28b07cae0a7c219b03bc22d01faf1f397c6bc2574605b73dbdd79b4ad960f"
    );
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &digest).unwrap();
    assert_eq!(
        mnemonic.to_string(),
        "boost nephew sea noise apology three grocery alter season gym leaf token defense today vacuum purse gate swear want road opera fine flag twice"
    );
}

#[test]
fn keystone_legacy_generation_crosses_all_boundaries() {
    let rolls = rolls(&"6".repeat(99));
    let calculation = accepted(
        EntropyTarget::Words24,
        ConversionProtocol::KeystoneLegacyV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "37b322ffb509a620e955be4b8f252fcb089b78ffd592537399253579f82e7b13"
    );
    assert_eq!(
        calculation.mnemonic().words().join(" "),
        "dash october say head omit away pipe result entire junior episode noodle meadow round young rather fat orphan enable helmet panel blame unable joy"
    );
}

#[test]
fn jade_published_example_crosses_table_and_bip39_boundaries() {
    let target = EntropyTarget::Words12;
    let mut capture = JadeCapture::new();
    capture.push_d16(D16Face::new(10).unwrap());
    capture.push_d16(D16Face::new(9).unwrap());
    capture.push_d8(D8Face::new(8).unwrap());
    for offset in 3..jade_required_observations(target) {
        match jade_expected_die(target, offset).unwrap() {
            JadeDieKind::D16 => capture.push_d16(D16Face::new(1).unwrap()),
            JadeDieKind::D8 => capture.push_d8(D8Face::new(1).unwrap()),
        }
    }
    let calculation = accepted(
        target,
        ConversionProtocol::JadeDirectV1,
        Capture::Jade(&capture),
    );
    assert_eq!(calculation.mnemonic().words()[0], "ocean");
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "98e00000000000000000000000000000"
    );
}

#[test]
fn jade_direct_words_cross_mixed_dice_and_bip39_boundaries() {
    for target in [EntropyTarget::Words12, EntropyTarget::Words24] {
        let capture = jade_all_ones(target);
        let calculation = accepted(
            target,
            ConversionProtocol::JadeDirectV1,
            Capture::Jade(&capture),
        );
        assert_eq!(
            calculation.entropy().bytes(),
            vec![0; target.entropy_bytes()]
        );
        assert_eq!(calculation.mnemonic().words()[0], "abandon");
        assert_eq!(
            calculation.mnemonic().words()[target.word_count() - 1],
            if target == EntropyTarget::Words12 {
                "about"
            } else {
                "art"
            }
        );
    }
}

#[test]
fn jade_max_faces_fix_tail_and_byte_boundaries() {
    for (target, final_word) in [
        (EntropyTarget::Words12, "wrong"),
        (EntropyTarget::Words24, "vote"),
    ] {
        let capture = jade_all_max(target);
        let calculation = accepted(
            target,
            ConversionProtocol::JadeDirectV1,
            Capture::Jade(&capture),
        );
        assert_eq!(
            calculation.entropy().bytes(),
            vec![u8::MAX; target.entropy_bytes()]
        );
        assert_eq!(calculation.mnemonic().words()[0], "zoo");
        assert_eq!(
            calculation.mnemonic().words()[target.word_count() - 1],
            final_word
        );
    }
}

#[test]
fn jade_wrong_die_order_is_rejected() {
    let mut capture = JadeCapture::new();
    for _ in 0..35 {
        capture.push_d8(D8Face::new(1).unwrap());
    }
    assert_eq!(
        calculate(
            EntropyTarget::Words12,
            ConversionProtocol::JadeDirectV1,
            Capture::Jade(&capture),
        )
        .unwrap_err(),
        CalculationError::Protocol(ProtocolError::WrongObservationKind)
    );
}

#[test]
fn seedsigner_coin_flips_cross_sha256_and_bip39_boundaries() {
    let flips = flips(&"0".repeat(128));
    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::SeedSignerCoinsV1,
        Capture::Coins(&flips),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "45725791c47b32618cc57b88343e2bce"
    );
    assert_eq!(
        calculation.mnemonic().words().join(" "),
        "earth naive tongue material rebel cotton credit quarter market peanut memory other"
    );
}

#[test]
fn coldcard_hash_crosses_sha256_and_bip39_boundaries() {
    let rolls = rolls(&"1".repeat(100));
    let calculation = accepted(
        EntropyTarget::Words24,
        ConversionProtocol::ColdcardV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "380b4863f69ebaacc794bfa1742a8a6ddc575e8cf0ded4341ab9da158881ea2d"
    );
    assert_eq!(calculation.mnemonic().words().len(), 24);
}
