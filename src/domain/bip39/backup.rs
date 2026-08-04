use core::fmt;

use zeroize::{Zeroize, Zeroizing};

use super::MnemonicPhrase;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedMnemonicBackup(());

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WordSubmission {
    Mismatch { position: usize },
    Next { position: usize },
    Complete(VerifiedMnemonicBackup),
}

pub(crate) struct BackupVerifier {
    position: usize,
    entry: Zeroizing<String>,
}

impl BackupVerifier {
    #[must_use]
    pub fn new() -> Self {
        Self {
            position: 0,
            entry: Zeroizing::new(String::new()),
        }
    }

    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[must_use]
    pub fn entry_len(&self) -> usize {
        self.entry.len()
    }

    pub fn push(&mut self, character: char) -> bool {
        if character.is_ascii_lowercase() && self.entry.len() < 16 {
            self.entry.push(character);
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) {
        let Some(next_len) = self.entry.len().checked_sub(1) else {
            return;
        };
        let next = Zeroizing::new(self.entry[..next_len].to_owned());
        self.entry.zeroize();
        self.entry = next;
    }

    pub fn submit(&mut self, expected: &MnemonicPhrase) -> WordSubmission {
        let position = self.position;
        if expected
            .words()
            .get(position)
            .is_none_or(|word| word != self.entry.as_str())
        {
            self.entry.zeroize();
            return WordSubmission::Mismatch { position };
        }
        self.entry.zeroize();
        if position + 1 == expected.words().len() {
            WordSubmission::Complete(VerifiedMnemonicBackup(()))
        } else {
            self.position += 1;
            WordSubmission::Next {
                position: self.position,
            }
        }
    }
}

impl Default for BackupVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for BackupVerifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BackupVerifier")
            .field("position", &self.position)
            .field("entry", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip39_ceremony_core::{
        CalculationOutcome, Capture, ConversionProtocol, DieFace, EntropyTarget, RollSequence,
        calculate,
    };

    fn calculation() -> bip39_ceremony_core::Calculation {
        let mut rolls = RollSequence::new();
        for _ in 0..50 {
            rolls.push(DieFace::new(1).unwrap());
        }
        let CalculationOutcome::Accepted(calculation) = calculate(
            EntropyTarget::Words12,
            ConversionProtocol::ExactV1,
            Capture::Dice(&rolls),
        )
        .unwrap() else {
            panic!("zero exact value is accepted");
        };
        calculation
    }

    #[test]
    fn receipt_exists_only_after_every_position_matches() {
        let expected = calculation();
        let mut verifier = BackupVerifier::new();
        verifier.push('x');
        assert_eq!(
            verifier.submit(expected.mnemonic()),
            WordSubmission::Mismatch { position: 0 }
        );
        for position in 0..12 {
            for character in expected.mnemonic().words()[position].chars() {
                verifier.push(character);
            }
            let submission = verifier.submit(expected.mnemonic());
            if position == 11 {
                assert!(matches!(submission, WordSubmission::Complete(_)));
            } else {
                assert_eq!(
                    submission,
                    WordSubmission::Next {
                        position: position + 1
                    }
                );
            }
        }
    }

    #[test]
    fn debug_redacts_temporary_word() {
        let mut verifier = BackupVerifier::new();
        for character in "abandon".chars() {
            verifier.push(character);
        }
        let debug = format!("{verifier:?}");
        assert!(!debug.contains("abandon"));
        assert!(debug.contains("[REDACTED]"));
    }
}
