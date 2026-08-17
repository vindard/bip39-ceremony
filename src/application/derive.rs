use core::fmt;
use std::fmt::Write;

use bip39_ceremony_core::CanonicalInput;
use zeroize::Zeroize;

use super::Generation;

/// Secret-bearing presentation projection of structured calculation evidence.
pub struct DerivationProjection {
    protocol: &'static str,
    canonical_input_encoding: &'static str,
    canonical_input: String,
    entropy_hex: String,
    checksum_bits: String,
    word_indices: Vec<u16>,
    words: Vec<String>,
}

impl DerivationProjection {
    #[must_use]
    pub fn build(generation: &Generation) -> Self {
        let evidence = generation.evidence();
        let (canonical_input_encoding, canonical_input) =
            canonical_input_parts(evidence.canonical_input());
        Self {
            protocol: generation.protocol().id(),
            canonical_input_encoding,
            canonical_input,
            entropy_hex: hex(generation.entropy().bytes()),
            checksum_bits: evidence
                .checksum_bits()
                .iter()
                .map(|bit| char::from(b'0' + bit))
                .collect(),
            word_indices: evidence.word_indices().to_vec(),
            words: generation.mnemonic().words().to_vec(),
        }
    }

    #[must_use]
    pub const fn protocol(&self) -> &str {
        self.protocol
    }

    /// The encoding of the canonical input (e.g. `ascii-rolls`). A display
    /// label describing the format of [`Self::canonical_input`]; it is not part
    /// of the input and never enters the hash.
    #[must_use]
    pub const fn canonical_input_encoding(&self) -> &str {
        self.canonical_input_encoding
    }

    #[must_use]
    pub fn canonical_input(&self) -> &str {
        &self.canonical_input
    }

    #[must_use]
    pub fn entropy_hex(&self) -> &str {
        &self.entropy_hex
    }

    #[must_use]
    pub fn checksum_bits(&self) -> &str {
        &self.checksum_bits
    }

    #[must_use]
    pub fn word_indices(&self) -> &[u16] {
        &self.word_indices
    }

    #[must_use]
    pub fn words(&self) -> &[String] {
        &self.words
    }
}

impl Drop for DerivationProjection {
    fn drop(&mut self) {
        self.canonical_input.zeroize();
        self.entropy_hex.zeroize();
        self.checksum_bits.zeroize();
        self.word_indices.zeroize();
        self.words.zeroize();
    }
}

impl fmt::Debug for DerivationProjection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DerivationProjection")
            .field("protocol", &self.protocol)
            .field("secret_values", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Splits canonical-input evidence into its encoding label and its raw content.
/// The label names the format only; the content is what the protocol actually
/// consumes (and, for hash-based protocols, the exact SHA-256 preimage). The
/// label is never part of the input or the hash.
fn canonical_input_parts(input: &CanonicalInput) -> (&'static str, String) {
    match input {
        CanonicalInput::Base6Integer(digits) => (
            "base-6 (0-5), msb-first (left-to-right), global rejection",
            ascii(digits).to_owned(),
        ),
        CanonicalInput::LocalizedBase6Candidates(digits) => (
            "base-6 (0-5), msb-first per-word candidates, local rejection",
            ascii(digits).to_owned(),
        ),
        CanonicalInput::AsciiFaceDigits(digits) => ("ascii-rolls", ascii(digits).to_owned()),
        CanonicalInput::PackedFaceBits(bits) => (
            "packed face bits (1-4 two bits, 5-6 one bit), msb-first",
            ascii(bits).to_owned(),
        ),
        CanonicalInput::AsciiFacesWithSixAsZero(digits) => {
            ("ascii-faces-6-as-0", ascii(digits).to_owned())
        }
        CanonicalInput::TypedMixedDice(observations) => {
            let mut output = String::new();
            for (index, pair) in observations.chunks_exact(2).enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write!(output, "D{}={}", pair[0], pair[1]).expect("writing to String cannot fail");
            }
            ("typed-mixed-dice", output)
        }
        CanonicalInput::TypedD6AndCoins(observations) => {
            let mut output = String::new();
            for (index, pair) in observations.chunks_exact(2).enumerate() {
                if index > 0 {
                    output.push(',');
                }
                let kind = if pair[0] == 6 { "D6" } else { "coin" };
                write!(output, "{kind}={}", pair[1]).expect("writing to String cannot fail");
            }
            ("typed-d6-coins", output)
        }
        CanonicalInput::AsciiHyphenatedD20(rolls) => {
            ("ascii-hyphenated-d20", ascii(rolls).to_owned())
        }
        CanonicalInput::AsciiCoinFlips(flips) => ("ascii-coin-flips", ascii(flips).to_owned()),
    }
}

fn ascii(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).expect("canonical protocol digits are ASCII")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
        output
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip39_ceremony_core::{
        CalculationOutcome, Capture, ConversionProtocol, DieFace, EntropyTarget, RollSequence,
        calculate,
    };

    fn zero_rolls() -> RollSequence {
        let mut rolls = RollSequence::new();
        for _ in 0..50 {
            rolls.push(DieFace::new(1).unwrap());
        }
        rolls
    }

    fn zero_calculation(rolls: &RollSequence) -> Generation {
        let CalculationOutcome::Accepted(calculation) = calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ExactV1,
            Capture::Dice(rolls),
        )
        .unwrap() else {
            panic!("zero exact value is accepted");
        };
        calculation
    }

    #[test]
    fn projection_exposes_all_bip39_stages() {
        let rolls = zero_rolls();
        let generation = zero_calculation(&rolls);
        let projection = DerivationProjection::build(&generation);
        assert_eq!(projection.protocol(), "exact-v1");
        // The encoding label is a separate display field; the content carries no
        // label prefix (it is exactly what the protocol consumes).
        assert_eq!(
            projection.canonical_input_encoding(),
            "base-6 (0-5), msb-first (left-to-right), global rejection"
        );
        assert!(!projection.canonical_input().contains(':'));
        assert_eq!(projection.entropy_hex(), "00000000000000000000000000000000");
        assert_eq!(projection.checksum_bits(), "0011");
        assert_eq!(
            projection.word_indices(),
            &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3]
        );
        assert_eq!(projection.words()[0], "abandon");
        assert_eq!(projection.words()[11], "about");
    }

    #[test]
    fn projection_debug_redacts_every_secret_stage() {
        let rolls = zero_rolls();
        let generation = zero_calculation(&rolls);
        let projection = DerivationProjection::build(&generation);
        let debug = format!("{projection:?}");
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("abandon"));
        assert!(!debug.contains("000000"));
    }
}
