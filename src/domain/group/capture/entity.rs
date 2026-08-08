//! The [`GroupCapture`] entity: one atomic entropy set, and the invariant that
//! classifies how a protocol consumes it.

use bip39_ceremony_core::{
    CalculationError, CalculationOutcome, CandidateStatus, Capture, calculate, trace_word_exact,
};

use crate::domain::{
    bip39::EntropyTarget,
    dice::{DieFace, RollSequence},
    protocol::{ConversionProtocol, ProtocolError},
};

use super::report::GroupStatus;

/// One atomic entropy set: a growing D6 tape and the target it feeds.
///
/// Every protocol evaluated against this capture derives from the same rolls.
/// The entity owns the invariant [`GroupCapture::status_under`]: a status is
/// `Accepted` only when the protocol consumes *every* roll of the capture with
/// nothing rejected — so the rule holds by construction, because the entity is
/// the sole place a `GroupStatus` is minted.
pub struct GroupCapture {
    target: EntropyTarget,
    tape: RollSequence,
}

impl GroupCapture {
    #[must_use]
    pub fn new(target: EntropyTarget) -> Self {
        Self {
            target,
            tape: RollSequence::new(),
        }
    }

    #[must_use]
    pub const fn target(&self) -> EntropyTarget {
        self.target
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tape.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tape.is_empty()
    }

    #[must_use]
    pub const fn tape(&self) -> &RollSequence {
        &self.tape
    }

    /// Appends one observation to the growing tape.
    pub fn push(&mut self, face: DieFace) {
        self.tape.push(face);
    }

    /// Removes and zeroizes the latest observation; returns whether one existed.
    pub fn remove_last(&mut self) -> bool {
        self.tape.remove_last()
    }

    /// A capture over the first `len` rolls of this one (same target). Used to
    /// evaluate the group at a shorter entropy set.
    ///
    /// # Panics
    ///
    /// Panics if `len` exceeds the current roll count.
    #[must_use]
    pub fn prefix(&self, len: usize) -> Self {
        let mut tape = RollSequence::new();
        for face in &self.tape.faces()[..len] {
            tape.push(DieFace::new(face.get()).expect("face already validated"));
        }
        Self {
            target: self.target,
            tape,
        }
    }

    /// This capture's status under `protocol` — the invariant. `Accepted` is
    /// minted only when the protocol consumes every roll of the capture with
    /// nothing rejected.
    #[must_use]
    pub fn status_under(&self, protocol: ConversionProtocol) -> GroupStatus {
        if !protocol.supports_target(self.target) {
            return GroupStatus::Unsupported;
        }
        if protocol == ConversionProtocol::WordExactV1 {
            return self.word_exact_status();
        }
        match calculate(self.target, protocol, Capture::Dice(&self.tape)) {
            Ok(CalculationOutcome::Accepted(c)) => GroupStatus::Accepted(c),
            Ok(CalculationOutcome::ExactRejected) => GroupStatus::InvalidOutOfRange,
            Ok(CalculationOutcome::ColdcardDistributionRejected) => {
                GroupStatus::InvalidDistribution
            }
            Err(CalculationError::Protocol(ProtocolError::WrongObservationCount {
                expected,
                actual,
            })) => {
                if actual < expected {
                    GroupStatus::Incomplete {
                        have: actual,
                        need: expected,
                    }
                } else {
                    GroupStatus::InvalidSurplus {
                        used: expected,
                        provided: actual,
                    }
                }
            }
            Err(CalculationError::Protocol(ProtocolError::UnsupportedTarget)) => {
                GroupStatus::Unsupported
            }
            Err(_) => GroupStatus::InvalidSurplus {
                used: 0,
                provided: self.tape.len(),
            },
        }
    }

    /// Word-by-word Exact needs its own classification: it consumes a variable
    /// prefix and rejects candidates, so a rejection means part of the entropy
    /// is discarded (unrecoverable → invalid), and completing before the end
    /// leaves a surplus.
    fn word_exact_status(&self) -> GroupStatus {
        let trace = trace_word_exact(self.target, &self.tape);
        if trace
            .candidates()
            .iter()
            .any(|candidate| candidate.status() == CandidateStatus::Rejected)
        {
            return GroupStatus::InvalidRejected;
        }
        let need = ConversionProtocol::WordExactV1.minimum_observations(self.target);
        match calculate(
            self.target,
            ConversionProtocol::WordExactV1,
            Capture::Dice(&self.tape),
        ) {
            Ok(CalculationOutcome::Accepted(c)) => GroupStatus::Accepted(c),
            Err(CalculationError::Protocol(ProtocolError::IncompleteWordExact)) => {
                GroupStatus::Incomplete {
                    have: self.tape.len(),
                    need,
                }
            }
            Err(CalculationError::Protocol(ProtocolError::WrongObservationCount {
                expected,
                actual,
            })) => GroupStatus::InvalidSurplus {
                used: expected,
                provided: actual,
            },
            _ => GroupStatus::InvalidSurplus {
                used: 0,
                provided: self.tape.len(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ConversionProtocol::{ColdcardV1, ExactV1, KeystoneLegacyV1, WordExactV1};

    /// A capture whose first roll is face 1 (keeps Exact in range) then cycles,
    /// so a completed protocol reads as Accepted rather than content-rejected.
    fn fair(target: EntropyTarget, n: usize) -> GroupCapture {
        let mut capture = GroupCapture::new(target);
        for i in 0..n {
            let face = u8::try_from((i % 6) + 1).expect("1..=6 fits u8");
            capture.push(DieFace::new(face).expect("valid face"));
        }
        capture
    }

    #[test]
    fn exact_accepts_only_its_exact_count() {
        assert!(
            fair(EntropyTarget::Words24, 100)
                .status_under(ExactV1)
                .calculation()
                .is_some(),
            "exact should accept at exactly 100"
        );
        assert!(
            matches!(
                fair(EntropyTarget::Words24, 99).status_under(ExactV1),
                GroupStatus::Incomplete {
                    have: 99,
                    need: 100
                }
            ),
            "99 is short → incomplete"
        );
        assert!(
            matches!(
                fair(EntropyTarget::Words24, 105).status_under(ExactV1),
                GroupStatus::InvalidSurplus {
                    used: 100,
                    provided: 105
                }
            ),
            "105 leaves 5 unused → invalid surplus"
        );
    }

    #[test]
    fn open_ended_protocols_use_every_roll() {
        // COLDCARD (min 99) and Keystone (min 50) accept at any length >= min.
        for &n in &[100usize, 105, 140, 166] {
            assert!(
                fair(EntropyTarget::Words24, n)
                    .status_under(ColdcardV1)
                    .calculation()
                    .is_some(),
                "coldcard should accept at {n}"
            );
            assert!(
                fair(EntropyTarget::Words24, n)
                    .status_under(KeystoneLegacyV1)
                    .calculation()
                    .is_some(),
                "keystone should accept at {n}"
            );
        }
        assert!(matches!(
            fair(EntropyTarget::Words24, 60).status_under(ColdcardV1),
            GroupStatus::Incomplete { .. }
        ));
    }

    #[test]
    fn coldcard_rejects_a_skewed_distribution() {
        let mut capture = GroupCapture::new(EntropyTarget::Words24);
        for _ in 0..100 {
            capture.push(DieFace::new(6).expect("valid face"));
        }
        assert!(matches!(
            capture.status_under(ColdcardV1),
            GroupStatus::InvalidDistribution
        ));
    }

    #[test]
    fn exact_out_of_range_is_invalid_not_accepted() {
        let mut capture = GroupCapture::new(EntropyTarget::Words24);
        for _ in 0..100 {
            capture.push(DieFace::new(6).expect("valid face"));
        }
        assert!(matches!(
            capture.status_under(ExactV1),
            GroupStatus::InvalidOutOfRange
        ));
    }

    #[test]
    fn word_exact_accepts_only_at_its_exact_clean_length() {
        assert!(
            matches!(
                fair(EntropyTarget::Words24, 100).status_under(WordExactV1),
                GroupStatus::Incomplete {
                    have: 100,
                    need: 140
                }
            ),
            "short of 140 → incomplete"
        );
        assert!(
            fair(EntropyTarget::Words24, 140)
                .status_under(WordExactV1)
                .calculation()
                .is_some(),
            "exactly 140, no rejection → accepted"
        );
        assert!(
            matches!(
                fair(EntropyTarget::Words24, 166).status_under(WordExactV1),
                GroupStatus::InvalidSurplus { .. }
            ),
            "past 140 with no rejection → surplus invalid"
        );
    }

    #[test]
    fn word_exact_rejection_is_invalid_even_when_long_enough() {
        // All sixes make every 6-roll candidate exceed the uniform ceiling, so
        // candidates are rejected — discarded entropy → invalid regardless.
        let mut capture = GroupCapture::new(EntropyTarget::Words24);
        for _ in 0..200 {
            capture.push(DieFace::new(6).expect("valid face"));
        }
        assert!(matches!(
            capture.status_under(WordExactV1),
            GroupStatus::InvalidRejected
        ));
    }

    #[test]
    fn keystone_is_unsupported_for_twelve_words() {
        assert!(matches!(
            fair(EntropyTarget::Words12, 50).status_under(KeystoneLegacyV1),
            GroupStatus::Unsupported
        ));
    }

    #[test]
    fn prefix_takes_the_leading_rolls() {
        let capture = fair(EntropyTarget::Words24, 120);
        let prefix = capture.prefix(100);
        assert_eq!(prefix.len(), 100);
        assert!(prefix.status_under(ExactV1).calculation().is_some());
    }
}
