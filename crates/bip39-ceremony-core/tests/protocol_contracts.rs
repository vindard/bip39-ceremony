use bip39_ceremony_core::{
    CalculationError, Capture, CoinFlip, ConversionProtocol, DieFace, EntropyTarget, FlipSequence,
    ProtocolError, RollSequence, calculate,
};

fn flips(value: &str) -> FlipSequence {
    let mut flips = FlipSequence::new();
    for character in value.chars() {
        flips.push(CoinFlip::try_from(character).unwrap());
    }
    flips
}

fn rolls(value: &str) -> RollSequence {
    let mut rolls = RollSequence::new();
    for character in value.chars() {
        rolls.push(DieFace::try_from(character).unwrap());
    }
    rolls
}

#[test]
fn seedsigner_requires_the_target_entropy_width() {
    for (target, expected) in [(EntropyTarget::Words12, 128), (EntropyTarget::Words24, 256)] {
        for actual in [expected - 1, expected + 1] {
            let flips = flips(&"0".repeat(actual));
            assert!(matches!(
                calculate(
                    target,
                    ConversionProtocol::SeedSignerCoinsV1,
                    Capture::Coins(&flips)
                ),
                Err(CalculationError::Protocol(
                    ProtocolError::WrongObservationCount {
                        expected: error_expected,
                        actual: error_actual,
                    }
                )) if error_expected == expected && error_actual == actual
            ));
        }
    }
}

#[test]
fn keystone_legacy_contract_rejects_short_and_unsupported_capture() {
    let short = rolls(&"1".repeat(98));
    assert!(matches!(
        calculate(
            EntropyTarget::Words24,
            ConversionProtocol::KeystoneLegacyV1,
            Capture::Dice(&short)
        ),
        Err(CalculationError::Protocol(
            ProtocolError::WrongObservationCount {
                expected: 99,
                actual: 98,
            }
        ))
    ));

    let unsupported = rolls(&"1".repeat(50));
    assert!(matches!(
        calculate(
            EntropyTarget::Words12,
            ConversionProtocol::KeystoneLegacyV1,
            Capture::Dice(&unsupported)
        ),
        Err(CalculationError::Protocol(ProtocolError::UnsupportedTarget))
    ));
}
