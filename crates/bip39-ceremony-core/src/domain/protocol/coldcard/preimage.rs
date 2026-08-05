use zeroize::Zeroizing;

use crate::domain::dice::RollSequence;

/// Exact headerless ASCII input documented by COLDCARD.
#[must_use]
pub fn ascii_rolls(rolls: &RollSequence) -> Zeroizing<Vec<u8>> {
    rolls.ascii_bytes()
}

/// Mirrors Coldcard's enforced distribution guard.
#[must_use]
pub fn distribution_is_rejected(rolls: &RollSequence) -> bool {
    let mut counts = [0_usize; 6];
    for face in rolls.faces() {
        counts[usize::from(face.get() - 1)] += 1;
    }
    counts
        .iter()
        .any(|count| count.saturating_mul(10) > rolls.len().saturating_mul(3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    fn rolls(groups: &[(char, usize)]) -> RollSequence {
        let mut rolls = RollSequence::new();
        for (face, count) in groups {
            for _ in 0..*count {
                rolls.push(DieFace::try_from(*face).unwrap());
            }
        }
        rolls
    }

    #[test]
    fn distribution_guard_uses_a_strict_thirty_percent_limit() {
        for (accepted, rejected) in [
            (
                [('1', 15), ('2', 7), ('3', 7), ('4', 7), ('5', 7), ('6', 7)],
                [('1', 16), ('2', 7), ('3', 7), ('4', 7), ('5', 7), ('6', 6)],
            ),
            (
                [
                    ('1', 30),
                    ('2', 14),
                    ('3', 14),
                    ('4', 14),
                    ('5', 14),
                    ('6', 14),
                ],
                [
                    ('1', 31),
                    ('2', 14),
                    ('3', 14),
                    ('4', 14),
                    ('5', 14),
                    ('6', 13),
                ],
            ),
        ] {
            assert!(!distribution_is_rejected(&rolls(&accepted)));
            assert!(distribution_is_rejected(&rolls(&rejected)));
        }
    }
}
