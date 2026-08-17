use std::{env, fmt::Write as _};

use bip39_ceremony_core::{
    BitBoxCapture, CalculationError, CalculationOutcome, Capture, CoinFlip, ConversionProtocol,
    D8Face, D16Face, D20Face, D20RollSequence, DieFace, EntropyTarget, FlipSequence, JadeCapture,
    ProtocolError, RollSequence, calculate,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    let protocol = match arguments.next().as_deref() {
        Some("exact-v1") => ConversionProtocol::ExactV1,
        Some("coldcard-v1") => ConversionProtocol::ColdcardV1,
        Some("seedsigner-coins-v1") => ConversionProtocol::SeedSignerCoinsV1,
        Some("krux-d20-v1") => ConversionProtocol::KruxD20V1,
        Some("bitbox02-direct-v1") => ConversionProtocol::BitBox02DirectV1,
        Some("keystone-legacy-v1") => ConversionProtocol::KeystoneLegacyV1,
        Some("jade-direct-v1") => ConversionProtocol::JadeDirectV1,
        Some("bitcoinlib-base6-v1") => ConversionProtocol::BitcoinLibBase6V1,
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

    let outcome = match protocol {
        ConversionProtocol::SeedSignerCoinsV1 => {
            let Some(flips) = parse_flips(&observations) else {
                return Ok(());
            };
            calculate(target, protocol, Capture::Coins(&flips))
        }
        ConversionProtocol::KruxD20V1 => {
            let Some(rolls) = parse_d20_rolls(&observations) else {
                return Ok(());
            };
            calculate(target, protocol, Capture::D20(&rolls))
        }
        ConversionProtocol::BitBox02DirectV1 => {
            let Some(capture) = parse_bitbox(&observations) else {
                return Ok(());
            };
            calculate(target, protocol, Capture::BitBox(&capture))
        }
        ConversionProtocol::JadeDirectV1 => {
            let Some(capture) = parse_jade(&observations) else {
                return Ok(());
            };
            calculate(target, protocol, Capture::Jade(&capture))
        }
        _ => {
            let Some(rolls) = parse_rolls(&observations) else {
                return Ok(());
            };
            calculate(target, protocol, Capture::Dice(&rolls))
        }
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
        Ok(CalculationOutcome::Base6WidthRejected) => {
            println!("rejected\tbase6-width");
        }
        Ok(CalculationOutcome::ColdcardDistributionRejected) => {
            println!("rejected\tdice-distribution");
        }
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

fn parse_jade(observations: &str) -> Option<JadeCapture> {
    let mut capture = JadeCapture::new();
    for (index, observation) in observations.split(',').enumerate() {
        let (kind, value) = observation.split_at_checked(1)?;
        let Ok(value) = value.parse::<u8>() else {
            println!("invalid\tobservation\t{index}");
            return None;
        };
        match kind {
            "a" => {
                let Ok(face) = D16Face::new(value) else {
                    println!("invalid\tobservation\t{index}");
                    return None;
                };
                capture.push_d16(face);
            }
            "b" => {
                let Ok(face) = D8Face::new(value) else {
                    println!("invalid\tobservation\t{index}");
                    return None;
                };
                capture.push_d8(face);
            }
            _ => {
                println!("invalid\tobservation\t{index}");
                return None;
            }
        }
    }
    Some(capture)
}

fn parse_bitbox(observations: &str) -> Option<BitBoxCapture> {
    let mut capture = BitBoxCapture::new();
    for (index, observation) in observations.split(',').enumerate() {
        let mut characters = observation.chars();
        let kind = characters.next();
        let value = characters.next();
        if characters.next().is_some() {
            println!("invalid\tobservation\t{index}");
            return None;
        }
        match (kind, value) {
            (Some('d'), Some(value)) => {
                let Ok(face) = DieFace::try_from(value) else {
                    println!("invalid\tobservation\t{index}");
                    return None;
                };
                capture.push_d6(face);
            }
            (Some('c'), Some(value)) => {
                let Ok(flip) = CoinFlip::try_from(value) else {
                    println!("invalid\tobservation\t{index}");
                    return None;
                };
                capture.push_coin(flip);
            }
            _ => {
                println!("invalid\tobservation\t{index}");
                return None;
            }
        }
    }
    Some(capture)
}

fn parse_d20_rolls(observations: &str) -> Option<D20RollSequence> {
    let mut rolls = D20RollSequence::new();
    for (index, observation) in observations.split(',').enumerate() {
        let Ok(value) = observation.parse::<u8>() else {
            println!("invalid\tobservation\t{index}");
            return None;
        };
        let Ok(face) = D20Face::new(value) else {
            println!("invalid\tobservation\t{index}");
            return None;
        };
        rolls.push(face);
    }
    Some(rolls)
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
        CalculationError::Protocol(ProtocolError::WrongObservationKind) => {
            println!("invalid\tobservation-kind");
        }
        CalculationError::CaptureKind => println!("invalid\tcapture-kind"),
        CalculationError::Bip39(_) => println!("error\tbip39"),
    }
}
