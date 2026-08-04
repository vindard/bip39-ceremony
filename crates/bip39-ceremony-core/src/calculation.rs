use core::fmt;

use bip39::{Language, Mnemonic};
use bitcoin_hashes::{Hash, HashEngine, sha256};
use zeroize::Zeroize;

use crate::domain::{
    bip39::{Bip39Error, EnglishMnemonic, Entropy, EntropyTarget},
    coin::FlipSequence,
    dice::RollSequence,
    jade::JadeCapture,
    protocol::{
        CanonicalInput, ConversionProtocol, ExactOutcome, ProtocolError, coldcard_ascii_rolls,
        exact_entropy, jade_entropy, keystone_legacy_ascii_rolls, native_hash_header,
        word_exact_entropy,
    },
};

/// A typed physical observation stream supplied to a conversion protocol.
#[derive(Clone, Copy)]
pub enum Capture<'a> {
    Dice(&'a RollSequence),
    Jade(&'a JadeCapture),
    Coins(&'a FlipSequence),
}

impl fmt::Debug for Capture<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, count) = match self {
            Self::Dice(rolls) => ("dice", rolls.len()),
            Self::Jade(capture) => ("jade-mixed-dice", capture.len()),
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
        (ConversionProtocol::SeedSignerCoinsV1, Capture::Coins(flips)) => {
            seedsigner_coin_entropy(target, flips)?
        }
        (ConversionProtocol::JadeDirectV1, Capture::Dice(_) | Capture::Coins(_))
        | (ConversionProtocol::SeedSignerCoinsV1, Capture::Dice(_) | Capture::Jade(_))
        | (_, Capture::Coins(_) | Capture::Jade(_)) => {
            return Err(CalculationError::CaptureKind);
        }
        (ConversionProtocol::ExactV1, Capture::Dice(rolls)) => {
            match exact_entropy(target, rolls)? {
                ExactOutcome::Accepted(entropy) => entropy,
                ExactOutcome::Rejected => return Ok(CalculationOutcome::ExactRejected),
            }
        }
        (ConversionProtocol::WordExactV1, Capture::Dice(rolls)) => {
            word_exact_entropy(target, rolls)?
        }
        (ConversionProtocol::NativeHashV1, Capture::Dice(rolls)) => {
            native_hash_entropy(target, rolls)?
        }
        (ConversionProtocol::ColdcardV1, Capture::Dice(rolls)) => coldcard_entropy(target, rolls)?,
        (ConversionProtocol::KeystoneLegacyV1, Capture::Dice(rolls)) => {
            keystone_legacy_entropy(target, rolls)?
        }
    };

    let mnemonic = encode_english(&entropy)?;
    let evidence = evidence(protocol, target, capture, &entropy);
    Ok(CalculationOutcome::Accepted(Calculation {
        protocol,
        entropy,
        mnemonic,
        evidence,
    }))
}

fn native_hash_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    require_complete_capture(ConversionProtocol::NativeHashV1, target, rolls)?;
    let header = native_hash_header(target);
    let ascii = rolls.ascii_bytes();
    Ok(hash_entropy(target, &[&header, ascii.as_slice()]))
}

fn coldcard_entropy(target: EntropyTarget, rolls: &RollSequence) -> Result<Entropy, ProtocolError> {
    require_complete_capture(ConversionProtocol::ColdcardV1, target, rolls)?;
    let ascii = coldcard_ascii_rolls(rolls);
    Ok(hash_entropy(target, &[ascii.as_slice()]))
}

fn keystone_legacy_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    if !ConversionProtocol::KeystoneLegacyV1.supports_target(target) {
        return Err(ProtocolError::UnsupportedTarget);
    }
    require_complete_capture(ConversionProtocol::KeystoneLegacyV1, target, rolls)?;
    let ascii = keystone_legacy_ascii_rolls(rolls);
    Ok(hash_entropy(target, &[ascii.as_slice()]))
}

fn seedsigner_coin_entropy(
    target: EntropyTarget,
    flips: &FlipSequence,
) -> Result<Entropy, ProtocolError> {
    let protocol = ConversionProtocol::SeedSignerCoinsV1;
    if !protocol.assess_coin_capture(target, flips).is_complete() {
        return Err(ProtocolError::WrongObservationCount {
            expected: protocol.minimum_observations(target),
            actual: flips.len(),
        });
    }
    let ascii = flips.ascii_bytes();
    Ok(hash_entropy(target, &[ascii.as_slice()]))
}

fn hash_entropy(target: EntropyTarget, chunks: &[&[u8]]) -> Entropy {
    let mut digest = sha256(chunks);
    let entropy = Entropy::from_protocol_bytes(target, digest[..target.entropy_bytes()].to_vec());
    digest.zeroize();
    entropy
}

fn require_complete_capture(
    protocol: ConversionProtocol,
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<(), ProtocolError> {
    if protocol.assess_capture(target, rolls).is_complete() {
        Ok(())
    } else {
        Err(ProtocolError::WrongObservationCount {
            expected: protocol.minimum_observations(target),
            actual: rolls.len(),
        })
    }
}

fn encode_english(entropy: &Entropy) -> Result<EnglishMnemonic, Bip39Error> {
    let mnemonic = Mnemonic::from_entropy_in(Language::English, entropy.bytes())
        .map_err(|_| Bip39Error::EncodingFailed)?;
    let words = mnemonic.words().map(str::to_owned).collect();
    EnglishMnemonic::from_verified_words(entropy.target(), words)
}

fn evidence(
    protocol: ConversionProtocol,
    target: EntropyTarget,
    capture: Capture<'_>,
    entropy: &Entropy,
) -> CalculationEvidence {
    let empty_rolls = RollSequence::new();
    let empty_flips = FlipSequence::new();
    let empty_jade = JadeCapture::new();
    let (rolls, flips, jade) = match capture {
        Capture::Dice(rolls) => (rolls, &empty_flips, &empty_jade),
        Capture::Jade(jade) => (&empty_rolls, &empty_flips, jade),
        Capture::Coins(flips) => (&empty_rolls, flips, &empty_jade),
    };
    let canonical_input = CanonicalInput::from_capture(protocol, target, rolls, flips, jade);
    let mut digest = sha256(&[entropy.bytes()]);
    let checksum_length = target.entropy_bits() / 32;
    let checksum_bits = (0..checksum_length)
        .map(|offset| (digest[0] >> (7 - offset)) & 1)
        .collect();
    let word_indices = bip39_indices(entropy.bytes(), digest[0], checksum_length);
    digest.zeroize();
    CalculationEvidence {
        canonical_input,
        checksum_bits,
        word_indices,
    }
}

fn bip39_indices(entropy: &[u8], checksum: u8, checksum_length: usize) -> Vec<u16> {
    let mut indices = Vec::with_capacity((entropy.len() * 8 + checksum_length) / 11);
    let mut accumulator = 0_u16;
    let mut bits = 0_u8;

    for byte in entropy {
        push_bits(*byte, 8, &mut accumulator, &mut bits, &mut indices);
    }
    push_bits(
        checksum >> (8 - checksum_length),
        u8::try_from(checksum_length).expect("BIP-39 checksum length fits u8"),
        &mut accumulator,
        &mut bits,
        &mut indices,
    );
    assert_eq!(bits, 0, "BIP-39 groups end on a word boundary");
    indices
}

fn push_bits(value: u8, count: u8, accumulator: &mut u16, bits: &mut u8, indices: &mut Vec<u16>) {
    for shift in (0..count).rev() {
        *accumulator = (*accumulator << 1) | u16::from((value >> shift) & 1);
        *bits += 1;
        if *bits == 11 {
            indices.push(*accumulator);
            *accumulator = 0;
            *bits = 0;
        }
    }
}

fn sha256(chunks: &[&[u8]]) -> [u8; 32] {
    let mut engine = sha256::Hash::engine();
    for chunk in chunks {
        engine.input(chunk);
    }
    sha256::Hash::from_engine(engine).to_byte_array()
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
