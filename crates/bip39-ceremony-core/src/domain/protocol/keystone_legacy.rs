use zeroize::Zeroizing;

use crate::domain::dice::RollSequence;

/// Legacy Keystone's documented Ian Coleman-compatible D6 text mapping.
#[must_use]
pub fn ascii_rolls(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        rolls
            .faces()
            .iter()
            .map(|face| if face.get() == 6 { b'0' } else { face.ascii() })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    #[test]
    fn maps_six_to_zero_and_preserves_other_faces() {
        let mut rolls = RollSequence::new();
        for value in 1..=6 {
            rolls.push(DieFace::new(value).unwrap());
        }

        assert_eq!(ascii_rolls(&rolls).as_slice(), b"123450");
    }
}
