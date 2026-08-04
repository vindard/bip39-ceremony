use core::fmt;
use std::fmt::Write;

use zeroize::Zeroize;

use crate::domain::dice::RollSequence;

use super::ports::Sha256;

/// Secret-bearing SHA-256 candidate for the current COLDCARD roll prefix.
pub struct ColdcardHashPreview {
    digest_hex: String,
    roll_count: usize,
}

impl ColdcardHashPreview {
    #[must_use]
    pub fn build(rolls: &RollSequence, sha256: &(impl Sha256 + ?Sized)) -> Self {
        let ascii_rolls = rolls.ascii_bytes();
        let mut digest = sha256.hash(&[ascii_rolls.as_slice()]);
        let digest_hex = digest.iter().fold(String::new(), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to String cannot fail");
            output
        });
        digest.zeroize();
        Self {
            digest_hex,
            roll_count: rolls.len(),
        }
    }

    #[must_use]
    pub fn digest_hex(&self) -> &str {
        &self.digest_hex
    }

    #[must_use]
    pub const fn roll_count(&self) -> usize {
        self.roll_count
    }
}

impl Drop for ColdcardHashPreview {
    fn drop(&mut self) {
        self.digest_hex.zeroize();
    }
}

impl fmt::Debug for ColdcardHashPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ColdcardHashPreview")
            .field("roll_count", &self.roll_count)
            .field("digest_hex", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adapters::BitcoinSha256, domain::dice::DieFace};

    #[test]
    fn preview_starts_at_empty_and_tracks_the_complete_prefix() {
        let rolls = RollSequence::new();
        let empty = ColdcardHashPreview::build(&rolls, &BitcoinSha256);
        assert_eq!(empty.roll_count(), 0);
        assert!(empty.digest_hex().starts_with("e3b0c44298fc1c14"));

        let mut rolls = RollSequence::new();
        rolls.push(DieFace::new(1).unwrap());
        rolls.push(DieFace::new(2).unwrap());
        let prefix = ColdcardHashPreview::build(&rolls, &BitcoinSha256);
        assert_eq!(prefix.roll_count(), 2);
        assert!(prefix.digest_hex().starts_with("6b51d431df5d7f14"));
    }

    #[test]
    fn debug_redacts_the_candidate_digest() {
        let preview = ColdcardHashPreview::build(&RollSequence::new(), &BitcoinSha256);
        let debug = format!("{preview:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("e3b0c442"));
    }
}
