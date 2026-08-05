use std::{env, fmt::Write as _};

use bip39_ceremony_core::{
    CalculationError, CalculationOutcome, Capture, CoinFlip, ConversionProtocol, DieFace,
    EntropyTarget, FlipSequence, ProtocolError, RollSequence, calculate,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    let protocol = match arguments.next().as_deref() {
        Some("exact-v1") => ConversionProtocol::ExactV1,
        Some("coldcard-v1") => ConversionProtocol::ColdcardV1,
        Some("keystone-legacy-v1") => ConversionProtocol::KeystoneLegacyV1,
        Some("seedsigner-coins-v1") => ConversionProtocol::SeedSignerCoinsV1,
        _ => return Err("expected a supported protocol".into()),
    };
    let target = match arguments.next().as_deref() {
        Some("12") => EntropyTarget::Words12,
        Some("24") => EntropyTarget::Words24,
        _ => return Err("expected word count 12 or 24".into()),
    };
    let observations = arguments.next().ok_or("expected observations")?;
    if arguments.next().is_some() {
        return Err("unexpected extra arguments".into());
    }

    let outcome = if protocol == ConversionProtocol::SeedSignerCoinsV1 {
        let Some(flips) = parse_flips(&observations) else {
            return Ok(());
        };
        calculate(target, protocol, Capture::Coins(&flips))
    } else {
        let Some(rolls) = parse_rolls(&observations) else {
            return Ok(());
        };
        calculate(target, protocol, Capture::Dice(&rolls))
    };

    match outcome {
        Ok(CalculationOutcome::Accepted(calculation)) => {
            let mut entropy = String::with_capacity(calculation.entropy().bytes().len() * 2);
            for byte in calculation.entropy().bytes() {
                write!(entropy, "{byte:02x}")?;
            }
            println!(
                "accepted\t{entropy}\t{}",
                calculation.mnemonic().words().join(" ")
            );
        }
        Ok(CalculationOutcome::ExactRejected) => println!("rejected\texact-range"),
        Err(error) => print_error(error),
    }
    Ok(())
}

fn parse_flips(observations: &str) -> Option<FlipSequence> {
    let mut flips = FlipSequence::new();
    for (index, character) in observations.chars().enumerate() {
        let Ok(flip) = CoinFlip::try_from(character) else {
            println!("invalid\tobservation\t{index}");
            return None;
        };
        flips.push(flip);
    }
    Some(flips)
}

fn parse_rolls(observations: &str) -> Option<RollSequence> {
    let mut rolls = RollSequence::new();
    for (index, character) in observations.chars().enumerate() {
        let Ok(face) = DieFace::try_from(character) else {
            println!("invalid\tobservation\t{index}");
            return None;
        };
        rolls.push(face);
    }
    Some(rolls)
}

fn print_error(error: CalculationError) {
    match error {
        CalculationError::Protocol(ProtocolError::WrongObservationCount { expected, actual }) => {
            println!("invalid\tobservation-count\t{expected}\t{actual}");
        }
        CalculationError::Protocol(ProtocolError::UnsupportedTarget) => {
            println!("invalid\tunsupported-target");
        }
        CalculationError::Protocol(ProtocolError::IncompleteWordExact) => {
            println!("error\tincomplete-word-exact");
        }
        CalculationError::CaptureKind => println!("invalid\tcapture-kind"),
        CalculationError::Bip39(_) => println!("error\tbip39"),
    }
}
