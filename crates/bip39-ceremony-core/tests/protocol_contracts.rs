use bip39_ceremony_core::{
    CalculationError, CalculationOutcome, Capture, CoinFlip, ConversionProtocol, D20Face,
    D20RollSequence, DieFace, EntropyTarget, FlipSequence, ProtocolError, RollSequence, calculate,
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
fn krux_requires_its_minimum_d20_roll_count() {
    for (target, expected) in [(EntropyTarget::Words12, 30), (EntropyTarget::Words24, 60)] {
        let mut rolls = D20RollSequence::new();
        for _ in 1..expected {
            rolls.push(D20Face::new(1).unwrap());
        }
        assert!(matches!(
            calculate(
                target,
                ConversionProtocol::KruxD20V1,
                Capture::D20(&rolls)
            ),
            Err(CalculationError::Protocol(
                ProtocolError::WrongObservationCount {
                    expected: error_expected,
                    actual: error_actual,
                }
            )) if error_expected == expected && error_actual == expected - 1
        ));
    }
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
fn coldcard_rejects_skewed_complete_distributions_after_count_validation() {
    let short = rolls(&"1".repeat(49));
    assert!(matches!(
        calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ColdcardV1,
            Capture::Dice(&short)
        ),
        Err(CalculationError::Protocol(
            ProtocolError::WrongObservationCount {
                expected: 50,
                actual: 49,
            }
        ))
    ));

    for (target, observations) in [
        (
            EntropyTarget::Words12,
            "11111111111111112222222333333344444445555555666666",
        ),
        (
            EntropyTarget::Words24,
            "111111111111111111111111111111222222222222223333333333333344444444444444555555555555556666666666666",
        ),
    ] {
        let rolls = rolls(observations);
        assert!(matches!(
            calculate(
                target,
                ConversionProtocol::ColdcardV1,
                Capture::Dice(&rolls)
            ),
            Ok(CalculationOutcome::ColdcardDistributionRejected)
        ));
    }
}

#[test]
fn keystone_legacy_contract_enforces_legacy_scope() {
    let short = rolls(&"1".repeat(49));
    assert!(matches!(
        calculate(
            EntropyTarget::Words24,
            ConversionProtocol::KeystoneLegacyV1,
            Capture::Dice(&short)
        ),
        Err(CalculationError::Protocol(
            ProtocolError::WrongObservationCount {
                expected: 50,
                actual: 49,
            }
        ))
    ));

    let minimum = rolls(&"1".repeat(50));
    assert!(matches!(
        calculate(
            EntropyTarget::Words24,
            ConversionProtocol::KeystoneLegacyV1,
            Capture::Dice(&minimum)
        ),
        Ok(CalculationOutcome::Accepted(_))
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
