use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    jade::{D8Face, D16Face, JadeCapture, JadeObservation},
};

use super::{ProtocolError, append_bits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JadeDieKind {
    D16,
    D8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JadeStage {
    DirectWord {
        position: usize,
        die: JadeDieKind,
        observation_in_word: usize,
    },
    EntropyTail {
        die: JadeDieKind,
    },
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JadeProgress {
    recorded: usize,
    required: usize,
    completed_words: usize,
    required_words: usize,
    stage: JadeStage,
}

impl JadeProgress {
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }

    #[must_use]
    pub const fn required(self) -> usize {
        self.required
    }

    #[must_use]
    pub const fn completed_words(self) -> usize {
        self.completed_words
    }

    #[must_use]
    pub const fn required_words(self) -> usize {
        self.required_words
    }

    #[must_use]
    pub const fn stage(self) -> JadeStage {
        self.stage
    }
}

#[must_use]
pub const fn jade_required_observations(target: EntropyTarget) -> usize {
    match target {
        EntropyTarget::Words12 => 35,
        EntropyTarget::Words24 => 70,
    }
}

#[must_use]
pub const fn jade_expected_die(target: EntropyTarget, offset: usize) -> Option<JadeDieKind> {
    let direct = (target.word_count() - 1) * 3;
    if offset < direct {
        return Some(if offset % 3 < 2 {
            JadeDieKind::D16
        } else {
            JadeDieKind::D8
        });
    }
    match (target, offset - direct) {
        (EntropyTarget::Words12, 0) => Some(JadeDieKind::D16),
        (EntropyTarget::Words12, 1) | (EntropyTarget::Words24, 0) => Some(JadeDieKind::D8),
        _ => None,
    }
}

#[must_use]
pub fn jade_progress(target: EntropyTarget, capture: &JadeCapture) -> JadeProgress {
    let recorded = capture.len();
    let required_words = target.word_count() - 1;
    let direct = required_words * 3;
    let completed_words = recorded.min(direct) / 3;
    let stage = if recorded < direct {
        JadeStage::DirectWord {
            position: recorded / 3 + 1,
            die: if recorded % 3 < 2 {
                JadeDieKind::D16
            } else {
                JadeDieKind::D8
            },
            observation_in_word: recorded % 3 + 1,
        }
    } else if let Some(die) = jade_expected_die(target, recorded) {
        JadeStage::EntropyTail { die }
    } else {
        JadeStage::Complete
    };
    JadeProgress {
        recorded,
        required: jade_required_observations(target),
        completed_words,
        required_words,
        stage,
    }
}

#[must_use]
pub fn jade_word_index(first: D16Face, second: D16Face, third: D8Face) -> u16 {
    u16::from(first.zero_based()) * 128
        + u16::from(second.zero_based()) * 8
        + u16::from(third.zero_based())
}

pub(crate) fn jade_entropy(
    target: EntropyTarget,
    capture: &JadeCapture,
) -> Result<Entropy, ProtocolError> {
    let expected = jade_required_observations(target);
    if capture.len() != expected {
        return Err(ProtocolError::WrongObservationCount {
            expected,
            actual: capture.len(),
        });
    }
    for (offset, observation) in capture.observations().iter().enumerate() {
        let kind = match observation {
            JadeObservation::D16(_) => JadeDieKind::D16,
            JadeObservation::D8(_) => JadeDieKind::D8,
        };
        if jade_expected_die(target, offset) != Some(kind) {
            return Err(ProtocolError::WrongObservationKind);
        }
    }

    let required_words = target.word_count() - 1;
    let mut indices = Vec::with_capacity(required_words);
    for group in capture.observations()[..required_words * 3].chunks_exact(3) {
        let [
            JadeObservation::D16(first),
            JadeObservation::D16(second),
            JadeObservation::D8(third),
        ] = group
        else {
            return Err(ProtocolError::WrongObservationKind);
        };
        indices.push(jade_word_index(*first, *second, *third));
    }

    let tail_start = required_words * 3;
    let (tail, tail_bits) = match target {
        EntropyTarget::Words12 => {
            let [JadeObservation::D16(first), JadeObservation::D8(second)] =
                &capture.observations()[tail_start..]
            else {
                return Err(ProtocolError::WrongObservationKind);
            };
            words12_entropy_tail(*first, *second)
        }
        EntropyTarget::Words24 => {
            let [JadeObservation::D8(face)] = &capture.observations()[tail_start..] else {
                return Err(ProtocolError::WrongObservationKind);
            };
            words24_entropy_tail(*face)
        }
    };
    Ok(assemble_entropy(target, &indices, tail, tail_bits))
}

fn words12_entropy_tail(first: D16Face, second: D8Face) -> (u16, usize) {
    (
        u16::from(first.zero_based()) * 8 + u16::from(second.zero_based()),
        7,
    )
}

fn words24_entropy_tail(face: D8Face) -> (u16, usize) {
    (u16::from(face.zero_based()), 3)
}

fn assemble_entropy(
    target: EntropyTarget,
    indices: &[u16],
    tail: u16,
    tail_bits: usize,
) -> Entropy {
    let mut bytes = vec![0_u8; target.entropy_bytes()];
    let mut bit_offset = 0;
    for &index in indices {
        append_bits(&mut bytes, &mut bit_offset, index, 11);
    }
    append_bits(&mut bytes, &mut bit_offset, tail, tail_bits);
    assert_eq!(bit_offset, target.entropy_bits());
    Entropy::from_protocol_bytes(target, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_ones(target: EntropyTarget) -> JadeCapture {
        let mut capture = JadeCapture::new();
        for offset in 0..jade_required_observations(target) {
            match jade_expected_die(target, offset).unwrap() {
                JadeDieKind::D16 => capture.push_d16(D16Face::new(1).unwrap()),
                JadeDieKind::D8 => capture.push_d8(D8Face::new(1).unwrap()),
            }
        }
        capture
    }

    #[test]
    fn official_example_and_boundaries_fix_mixed_radix_order() {
        assert_eq!(
            jade_word_index(
                D16Face::new(10).unwrap(),
                D16Face::new(9).unwrap(),
                D8Face::new(8).unwrap(),
            ),
            1_223
        );
        assert_eq!(
            jade_word_index(
                D16Face::new(16).unwrap(),
                D16Face::new(16).unwrap(),
                D8Face::new(8).unwrap(),
            ),
            2_047
        );
    }

    #[test]
    fn all_one_captures_produce_zero_entropy() {
        for target in [EntropyTarget::Words12, EntropyTarget::Words24] {
            assert_eq!(
                jade_entropy(target, &all_ones(target)).unwrap().bytes(),
                vec![0; target.entropy_bytes()]
            );
        }
    }

    #[test]
    fn progress_moves_from_words_to_target_specific_tail() {
        let mut capture = JadeCapture::new();
        assert!(matches!(
            jade_progress(EntropyTarget::Words12, &capture).stage(),
            JadeStage::DirectWord {
                die: JadeDieKind::D16,
                ..
            }
        ));
        for offset in 0..33 {
            match jade_expected_die(EntropyTarget::Words12, offset).unwrap() {
                JadeDieKind::D16 => capture.push_d16(D16Face::new(1).unwrap()),
                JadeDieKind::D8 => capture.push_d8(D8Face::new(1).unwrap()),
            }
        }
        assert_eq!(
            jade_progress(EntropyTarget::Words12, &capture).stage(),
            JadeStage::EntropyTail {
                die: JadeDieKind::D16
            }
        );
    }
}
