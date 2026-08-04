use zeroize::Zeroizing;

use crate::domain::dice::RollSequence;

/// Exact headerless ASCII input documented by COLDCARD.
#[must_use]
pub fn ascii_rolls(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    rolls.ascii_bytes()
}
