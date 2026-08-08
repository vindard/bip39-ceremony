use std::fmt::Write;

use bip39::{Language, Mnemonic};
use bip39_ceremony_core::{
    BitBoxCapture, CalculationError, CalculationOutcome, CanonicalInput, Capture, CoinFlip,
    CoinFourD6Capture, ConversionProtocol, D8Face, D16Face, D20Face, D20RollSequence, DieFace,
    Entropy, EntropyTarget, FlipSequence, JadeCapture, JadeDieKind, ProtocolError, RollSequence,
    bitbox_tail_bits, calculate, coin_four_d6_progress, jade_expected_die,
    jade_required_observations,
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

fn bitbox_uniform(target: EntropyTarget, face: u8, flip: u8) -> BitBoxCapture {
    let mut capture = BitBoxCapture::new();
    for _ in 0..target.word_count() - 1 {
        for _ in 0..5 {
            capture.push_d6(DieFace::new(face).unwrap());
        }
        capture.push_coin(CoinFlip::new(flip).unwrap());
    }
    for _ in 0..bitbox_tail_bits(target) {
        capture.push_coin(CoinFlip::new(flip).unwrap());
    }
    capture
}

fn coin_four_d6_indices(indices: &[u16]) -> CoinFourD6Capture {
    let mut capture = CoinFourD6Capture::new();
    for &index in indices {
        let (coin, mut rank) = if index < 1_296 {
            (1, index)
        } else {
            (0, index - 1_296)
        };
        let mut faces = [1_u8; 4];
        for offset in (0..4).rev() {
            faces[offset] = u8::try_from(rank % 6 + 1).unwrap();
            rank /= 6;
        }
        capture.push_coin(CoinFlip::new(coin).unwrap());
        for face in faces {
            capture.push_d6(DieFace::new(face).unwrap());
        }
    }
    capture
}

fn bitbox_asymmetric(target: EntropyTarget) -> BitBoxCapture {
    let mut capture = BitBoxCapture::new();
    for position in 0..target.word_count() - 1 {
        let index = (position * 137 + 217) % 2_048;
        let mut base4 = index / 2;
        let mut faces = [1_u8; 5];
        for offset in (0..5).rev() {
            faces[offset] = u8::try_from(base4 % 4 + 1).unwrap();
            base4 /= 4;
        }
        for face in faces {
            capture.push_d6(DieFace::new(face).unwrap());
        }
        capture.push_coin(CoinFlip::new(u8::try_from(1 - index % 2).unwrap()).unwrap());
    }
    let tail = if target == EntropyTarget::Words12 {
        [1, 0, 1, 0, 1, 0, 1].as_slice()
    } else {
        [1, 0, 1].as_slice()
    };
    for selector in tail {
        capture.push_coin(CoinFlip::new(1 - selector).unwrap());
    }
    capture
}

fn d20_rolls(face: u8, count: usize) -> D20RollSequence {
    let mut rolls = D20RollSequence::new();
    for _ in 0..count {
        rolls.push(D20Face::new(face).unwrap());
    }
    rolls
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
fn bitbox_direct_words_cross_rejection_table_and_bip39_boundaries() {
    for (target, final_word) in [
        (EntropyTarget::Words12, "about"),
        (EntropyTarget::Words24, "art"),
    ] {
        let mut capture = bitbox_uniform(target, 1, 1);
        // A rejected face is retained without changing the selected index.
        let mut with_rejection = BitBoxCapture::new();
        with_rejection.push_d6(DieFace::new(6).unwrap());
        for observation in capture.observations() {
            match observation {
                bip39_ceremony_core::BitBoxObservation::D6(face) => {
                    with_rejection.push_d6(*face);
                }
                bip39_ceremony_core::BitBoxObservation::Coin(flip) => {
                    with_rejection.push_coin(*flip);
                }
            }
        }
        capture = with_rejection;
        let calculation = accepted(
            target,
            ConversionProtocol::BitBox02DirectV1,
            Capture::BitBox(&capture),
        );
        assert_eq!(
            calculation.entropy().bytes(),
            vec![0; target.entropy_bytes()]
        );
        assert_eq!(calculation.mnemonic().words()[0], "abandon");
        assert_eq!(
            calculation.mnemonic().words()[target.word_count() - 1],
            final_word
        );
        let CanonicalInput::TypedD6AndCoins(input) = calculation.evidence().canonical_input()
        else {
            panic!("typed BitBox evidence expected");
        };
        assert_eq!(&input[..4], &[6, 6, 6, 1]);
    }
}

#[test]
fn bitbox_asymmetric_vectors_fix_word_and_tail_bit_order() {
    for (target, entropy) in [
        (EntropyTarget::Words12, "1b2588f5a745fae1a07c98a436ab19d5"),
        (
            EntropyTarget::Words24,
            "1b2588f5a745fae1a07c98a436ab19ebce8bf382b8e02d27c93db0471b05a4fd",
        ),
    ] {
        let capture = bitbox_asymmetric(target);
        let calculation = accepted(
            target,
            ConversionProtocol::BitBox02DirectV1,
            Capture::BitBox(&capture),
        );
        assert_eq!(entropy_hex(calculation.entropy()), entropy);
        assert_eq!(calculation.evidence().word_indices()[0], 217);
        assert_eq!(calculation.evidence().word_indices()[1], 354);
    }
}

#[test]
fn bitbox_max_faces_fix_lookup_and_entropy_boundaries() {
    for (target, final_word) in [
        (EntropyTarget::Words12, "wrong"),
        (EntropyTarget::Words24, "vote"),
    ] {
        let capture = bitbox_uniform(target, 4, 0);
        let calculation = accepted(
            target,
            ConversionProtocol::BitBox02DirectV1,
            Capture::BitBox(&capture),
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
fn bitbox_wrong_observation_kind_is_rejected() {
    let mut capture = BitBoxCapture::new();
    capture.push_coin(CoinFlip::new(1).unwrap());
    assert_eq!(
        calculate(
            EntropyTarget::Words12,
            ConversionProtocol::BitBox02DirectV1,
            Capture::BitBox(&capture),
        )
        .unwrap_err(),
        CalculationError::Protocol(ProtocolError::WrongObservationKind)
    );
}

#[test]
fn krux_d20_pinned_vectors_cross_serialization_hash_and_bip39() {
    for (target, count, entropy, mnemonic) in [
        (
            EntropyTarget::Words12,
            30,
            "4cf6b2e58bcfee3fa6f6d0618c99bfcd",
            "erupt remain ride bleak year cabin orange sure ghost gospel husband oppose",
        ),
        (
            EntropyTarget::Words24,
            60,
            "5e0ecfd4e5c1ff5e0f2f519b09ab83af6c88ef136045b7be90c4a8d9e9bf1e87",
            "fun island vivid slide cable pyramid device tuition only essence thought gain silk jealous eternal anger response virus couple faculty ozone test key vocal",
        ),
    ] {
        let rolls = d20_rolls(1, count);
        let calculation = accepted(target, ConversionProtocol::KruxD20V1, Capture::D20(&rolls));
        assert_eq!(entropy_hex(calculation.entropy()), entropy);
        assert_eq!(calculation.mnemonic().words().join(" "), mnemonic);
        let CanonicalInput::AsciiHyphenatedD20(input) = calculation.evidence().canonical_input()
        else {
            panic!("Krux D20 canonical input expected");
        };
        assert_eq!(input.len(), count * 2 - 1);
        assert!(!input.starts_with(b"-"));
        assert!(!input.ends_with(b"-"));
    }
}

#[test]
fn krux_d20_mixed_face_oracle_fixes_decimal_boundaries() {
    let mut rolls = D20RollSequence::new();
    for _ in 0..10 {
        for face in [1, 10, 20] {
            rolls.push(D20Face::new(face).unwrap());
        }
    }
    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::KruxD20V1,
        Capture::D20(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "f206c8adb2ef5c50d7eda7a19ee69b51"
    );
    assert_eq!(
        calculation.mnemonic().words().join(" "),
        "velvet curve clock grape volume chronic garden reject pave warm plug photo"
    );
    let CanonicalInput::AsciiHyphenatedD20(input) = calculation.evidence().canonical_input() else {
        panic!("Krux D20 canonical input expected");
    };
    assert!(input.starts_with(b"1-10-20-1-10-20"));
    assert!(input.ends_with(b"-1-10-20"));
}

#[test]
fn coin_four_d6_vectors_cross_table_tail_and_bip39_boundaries() {
    for (indices, entropy, mnemonic) in [
        (
            vec![0; 12],
            "00000000000000000000000000000000",
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        ),
        (
            vec![2_047; 12],
            "ffffffffffffffffffffffffffffffff",
            "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
        ),
        (
            vec![
                0, 1_295, 1_296, 2_047, 1, 1_294, 1_297, 2_046, 2, 1_293, 1_298, 2_045,
            ],
            "00143e887ff00343a88ffe005436897f",
            "abandon peanut pear zoo ability peace peasant zone able payment pelican wrap",
        ),
    ] {
        let capture = coin_four_d6_indices(&indices);
        let calculation = accepted(
            EntropyTarget::Words12,
            ConversionProtocol::CoinFourD6DirectV1,
            Capture::CoinFourD6(&capture),
        );
        assert_eq!(entropy_hex(calculation.entropy()), entropy);
        assert_eq!(calculation.mnemonic().words().join(" "), mnemonic);
        let CanonicalInput::TypedD6AndCoins(input) = calculation.evidence().canonical_input()
        else {
            panic!("typed coin-four-D6 canonical input expected");
        };
        assert_eq!(input.len(), 120);
    }
}

#[test]
fn coin_four_d6_rejection_remains_in_typed_canonical_evidence() {
    let mut capture = CoinFourD6Capture::new();
    capture.push_coin(CoinFlip::new(0).unwrap());
    for face in [4, 3, 6, 3] {
        capture.push_d6(DieFace::new(face).unwrap());
    }
    let zeros = coin_four_d6_indices(&[0; 12]);
    for observation in zeros.observations() {
        match observation {
            bip39_ceremony_core::CoinFourD6Observation::Coin(flip) => capture.push_coin(*flip),
            bip39_ceremony_core::CoinFourD6Observation::D6(face) => capture.push_d6(*face),
        }
    }
    let progress = coin_four_d6_progress(EntropyTarget::Words12, &capture);
    assert_eq!(progress.rejected_candidates(), 1);
    assert_eq!(progress.completed_candidates(), 12);

    let calculation = accepted(
        EntropyTarget::Words12,
        ConversionProtocol::CoinFourD6DirectV1,
        Capture::CoinFourD6(&capture),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "00000000000000000000000000000000"
    );
    let CanonicalInput::TypedD6AndCoins(input) = calculation.evidence().canonical_input() else {
        panic!("typed coin-four-D6 canonical input expected");
    };
    assert_eq!(input.len(), 130);
    assert_eq!(
        &input[..20],
        &[2, 0, 6, 4, 6, 3, 6, 6, 6, 3, 2, 1, 6, 1, 6, 1, 6, 1, 6, 1]
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
    let rolls = rolls(&("123456".repeat(17))[..100]);
    let calculation = accepted(
        EntropyTarget::Words24,
        ConversionProtocol::ColdcardV1,
        Capture::Dice(&rolls),
    );
    assert_eq!(
        entropy_hex(calculation.entropy()),
        "e56403e8522ddeae1b44a1e8148b1ba4d3b4c626ccf20980056eedcc7e0c0f35"
    );
    assert_eq!(calculation.mnemonic().words().len(), 24);
}
