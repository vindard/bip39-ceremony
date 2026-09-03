use core::fmt;

use zeroize::Zeroize;

use crate::domain::{
    bip39::{Bip39Encoding, Bip39Error, EnglishMnemonic, Entropy, EntropyTarget},
    bitbox::BitBoxCapture,
    coin::FlipSequence,
    coin_four_d6::CoinFourD6Capture,
    d20::D20RollSequence,
    dice::RollSequence,
    jade::JadeCapture,
    protocol::{
        BitcoinLibBase6EntropyOutcome as BitcoinLibOutcome, CanonicalInput, ColdcardEntropyOutcome,
        ConversionProtocol, ExactEntropyOutcome, ProtocolError, bitbox_entropy,
        bitcoinlib_base6_entropy, bluewallet_bitpack_entropy, coin_four_d6_entropy,
        coldcard_entropy, exact_entropy, iancoleman_dice_entropy, iancoleman_raw_entropy,
        jade_entropy, keystone_legacy_entropy, krux_d20_entropy, seedsigner_coins_entropy,
        word_exact_entropy,
    },
};

/// A typed physical observation stream supplied to a conversion protocol.
#[derive(Clone, Copy)]
pub enum Capture<'a> {
    Dice(&'a RollSequence),
    Jade(&'a JadeCapture),
    BitBox(&'a BitBoxCapture),
    CoinFourD6(&'a CoinFourD6Capture),
    D20(&'a D20RollSequence),
    Coins(&'a FlipSequence),
}

impl fmt::Debug for Capture<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, count) = match self {
            Self::Dice(rolls) => ("dice", rolls.len()),
            Self::Jade(capture) => ("jade-mixed-dice", capture.len()),
            Self::BitBox(capture) => ("bitbox-d6-coins", capture.len()),
            Self::CoinFourD6(capture) => ("coin-four-d6", capture.len()),
            Self::D20(rolls) => ("d20", rolls.len()),
            Self::Coins(flips) => ("coins", flips.len()),
        };
        formatter
            .debug_struct("Capture")
            .field("kind", &kind)
            .field("count", &count)
            .field("observations", &"[REDACTED]")
            .finish()
    }
}

/// An accepted deterministic conversion and its BIP-39 result.
pub struct Calculation {
    protocol: ConversionProtocol,
    entropy: Entropy,
    mnemonic: EnglishMnemonic,
    evidence: CalculationEvidence,
}

impl Calculation {
    #[must_use]
    pub const fn protocol(&self) -> ConversionProtocol {
        self.protocol
    }

    /// Explicitly reveals the calculated entropy.
    #[must_use]
    pub const fn entropy(&self) -> &Entropy {
        &self.entropy
    }

    /// Explicitly reveals the verified English BIP-39 mnemonic.
    #[must_use]
    pub const fn mnemonic(&self) -> &EnglishMnemonic {
        &self.mnemonic
    }

    /// Explicitly reveals structured intermediate calculation values.
    #[must_use]
    pub const fn evidence(&self) -> &CalculationEvidence {
        &self.evidence
    }
}

impl fmt::Debug for Calculation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Calculation")
            .field("protocol", &self.protocol)
            .field("secret_values", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Structured, secret-bearing intermediate values for independent inspection.
pub struct CalculationEvidence {
    canonical_input: CanonicalInput,
    checksum_bits: Vec<u8>,
    word_indices: Vec<u16>,
}

impl CalculationEvidence {
    /// Explicitly reveals the protocol's canonical input representation.
    #[must_use]
    pub const fn canonical_input(&self) -> &CanonicalInput {
        &self.canonical_input
    }

    /// Explicitly reveals checksum bits as ordered zero-or-one values.
    #[must_use]
    pub fn checksum_bits(&self) -> &[u8] {
        &self.checksum_bits
    }

    /// Explicitly reveals zero-based English BIP-39 word indices.
    #[must_use]
    pub fn word_indices(&self) -> &[u16] {
        &self.word_indices
    }
}

impl Drop for CalculationEvidence {
    fn drop(&mut self) {
        self.checksum_bits.zeroize();
        self.word_indices.zeroize();
    }
}

impl fmt::Debug for CalculationEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CalculationEvidence([REDACTED])")
    }
}

#[derive(Debug)]
pub enum CalculationOutcome {
    Accepted(Calculation),
    ExactRejected,
    ColdcardDistributionRejected,
    Base6WidthRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalculationError {
    Protocol(ProtocolError),
    Bip39(Bip39Error),
    CaptureKind,
}

impl From<ProtocolError> for CalculationError {
    fn from(error: ProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<Bip39Error> for CalculationError {
    fn from(error: Bip39Error) -> Self {
        Self::Bip39(error)
    }
}

impl fmt::Display for CalculationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => error.fmt(formatter),
            Self::Bip39(error) => error.fmt(formatter),
            Self::CaptureKind => formatter.write_str("capture kind does not match protocol"),
        }
    }
}

impl std::error::Error for CalculationError {}

/// Deterministically converts a typed physical capture into English BIP-39.
///
/// # Errors
///
/// Returns [`CalculationError`] when the capture kind, count, target, or BIP-39
/// result violates the selected versioned protocol.
pub fn calculate(
    target: EntropyTarget,
    protocol: ConversionProtocol,
    capture: Capture<'_>,
) -> Result<CalculationOutcome, CalculationError> {
    let entropy = match (protocol, capture) {
        (ConversionProtocol::JadeDirectV1, Capture::Jade(capture)) => {
            jade_entropy(target, capture)?
        }
        (ConversionProtocol::BitBox02DirectV1, Capture::BitBox(capture)) => {
            bitbox_entropy(target, capture)?
        }
        (ConversionProtocol::KruxD20V1, Capture::D20(rolls)) => krux_d20_entropy(target, rolls)?,
        (ConversionProtocol::CoinFourD6DirectV1, Capture::CoinFourD6(capture)) => {
            coin_four_d6_entropy(target, capture)?
        }
        (ConversionProtocol::SeedSignerCoinsV1, Capture::Coins(flips)) => {
            seedsigner_coins_entropy(target, flips)?
        }
        (
            ConversionProtocol::JadeDirectV1,
            Capture::Dice(_)
            | Capture::BitBox(_)
            | Capture::CoinFourD6(_)
            | Capture::D20(_)
            | Capture::Coins(_),
        )
        | (
            ConversionProtocol::BitBox02DirectV1,
            Capture::Dice(_)
            | Capture::Jade(_)
            | Capture::CoinFourD6(_)
            | Capture::D20(_)
            | Capture::Coins(_),
        )
        | (
            ConversionProtocol::SeedSignerCoinsV1,
            Capture::Dice(_)
            | Capture::Jade(_)
            | Capture::BitBox(_)
            | Capture::CoinFourD6(_)
            | Capture::D20(_),
        )
        | (
            ConversionProtocol::KruxD20V1,
            Capture::Dice(_)
            | Capture::Jade(_)
            | Capture::BitBox(_)
            | Capture::CoinFourD6(_)
            | Capture::Coins(_),
        )
        | (
            ConversionProtocol::CoinFourD6DirectV1,
            Capture::Dice(_)
            | Capture::Jade(_)
            | Capture::BitBox(_)
            | Capture::D20(_)
            | Capture::Coins(_),
        )
        | (
            _,
            Capture::Coins(_)
            | Capture::Jade(_)
            | Capture::BitBox(_)
            | Capture::CoinFourD6(_)
            | Capture::D20(_),
        ) => {
            return Err(CalculationError::CaptureKind);
        }
        (ConversionProtocol::ExactV1, Capture::Dice(rolls)) => {
            match exact_entropy(target, rolls)? {
                ExactEntropyOutcome::Accepted(entropy) => entropy,
                ExactEntropyOutcome::Rejected => return Ok(CalculationOutcome::ExactRejected),
            }
        }
        (ConversionProtocol::BitcoinLibBase6V1, Capture::Dice(rolls)) => {
            match bitcoinlib_base6_entropy(target, rolls)? {
                BitcoinLibOutcome::Accepted(entropy) => entropy,
                BitcoinLibOutcome::Rejected => return Ok(CalculationOutcome::Base6WidthRejected),
            }
        }
        (
            protocol @ (ConversionProtocol::BlueWalletBitPackV1
            | ConversionProtocol::IanColemanRawV1),
            Capture::Dice(rolls),
        ) => packed_entropy(protocol, target, rolls)?,
        (ConversionProtocol::IanColemanDiceV1, Capture::Dice(rolls)) => {
            iancoleman_dice_entropy(target, rolls)?
        }
        (ConversionProtocol::WordExactV1, Capture::Dice(rolls)) => {
            word_exact_entropy(target, rolls)?
        }
        (ConversionProtocol::ColdcardV1, Capture::Dice(rolls)) => {
            match coldcard_entropy(target, rolls)? {
                ColdcardEntropyOutcome::Accepted(entropy) => entropy,
                ColdcardEntropyOutcome::DistributionRejected => {
                    return Ok(CalculationOutcome::ColdcardDistributionRejected);
                }
            }
        }
        (ConversionProtocol::KeystoneLegacyV1, Capture::Dice(rolls)) => {
            keystone_legacy_entropy(target, rolls)?
        }
    };

    finish_calculation(protocol, target, capture, entropy)
}

fn packed_entropy(
    protocol: ConversionProtocol,
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    match protocol {
        ConversionProtocol::BlueWalletBitPackV1 => bluewallet_bitpack_entropy(target, rolls),
        ConversionProtocol::IanColemanRawV1 => iancoleman_raw_entropy(target, rolls),
        _ => unreachable!("only packed protocols are dispatched here"),
    }
}

fn finish_calculation(
    protocol: ConversionProtocol,
    target: EntropyTarget,
    capture: Capture<'_>,
    entropy: Entropy,
) -> Result<CalculationOutcome, CalculationError> {
    let encoding = Bip39Encoding::from_entropy(&entropy)?;
    let (mnemonic, checksum_bits, word_indices) = encoding.into_parts();
    let canonical_input = match capture {
        Capture::Dice(rolls) => CanonicalInput::from_dice(protocol, target, rolls),
        Capture::Jade(capture) => CanonicalInput::from_jade(capture),
        Capture::BitBox(capture) => CanonicalInput::from_bitbox(capture),
        Capture::CoinFourD6(capture) => CanonicalInput::from_coin_four_d6(capture),
        Capture::D20(capture) => CanonicalInput::from_d20(capture),
        Capture::Coins(capture) => CanonicalInput::from_coins(capture),
    };
    Ok(CalculationOutcome::Accepted(Calculation {
        protocol,
        entropy,
        mnemonic,
        evidence: CalculationEvidence {
            canonical_input,
            checksum_bits,
            word_indices,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DieFace;

    fn rolls(value: u8, count: usize) -> RollSequence {
        let mut rolls = RollSequence::new();
        for _ in 0..count {
            rolls.push(DieFace::new(value).unwrap());
        }
        rolls
    }

    #[test]
    fn exact_zero_exposes_verified_bip39_evidence() {
        let rolls = rolls(1, 50);
        let CalculationOutcome::Accepted(calculation) = calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ExactV1,
            Capture::Dice(&rolls),
        )
        .unwrap() else {
            panic!("zero exact value is accepted");
        };

        assert_eq!(calculation.entropy().bytes(), &[0; 16]);
        assert_eq!(calculation.evidence().checksum_bits(), &[0, 0, 1, 1]);
        assert_eq!(
            calculation.evidence().word_indices(),
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3]
        );
        assert_eq!(calculation.mnemonic().words()[11], "about");
    }

    #[test]
    fn capture_kind_is_part_of_the_public_contract() {
        assert_eq!(
            calculate(
                EntropyTarget::Words12,
                ConversionProtocol::ColdcardV1,
                Capture::Coins(&FlipSequence::new()),
            )
            .unwrap_err(),
            CalculationError::CaptureKind
        );
    }

    #[test]
    fn debug_redacts_capture_calculation_and_evidence() {
        let rolls = rolls(1, 50);
        let CalculationOutcome::Accepted(calculation) = calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ExactV1,
            Capture::Dice(&rolls),
        )
        .unwrap() else {
            panic!("zero exact value is accepted");
        };
        assert!(format!("{:?}", Capture::Dice(&rolls)).contains("REDACTED"));
        assert!(format!("{calculation:?}").contains("REDACTED"));
        assert!(format!("{:?}", calculation.evidence()).contains("REDACTED"));
    }
}
