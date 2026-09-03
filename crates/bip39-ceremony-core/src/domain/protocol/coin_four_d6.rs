use zeroize::Zeroize;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    coin::CoinFlip,
    coin_four_d6::{CoinFourD6Capture, CoinFourD6Observation},
    dice::DieFace,
};

use super::{ProtocolError, append_bits};

pub const REQUIRED_CANDIDATES: usize = 12;
pub const OBSERVATIONS_PER_CANDIDATE: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoinFourD6ObservationKind {
    Coin,
    D6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoinFourD6Stage {
    Coin { position: usize },
    D6 { position: usize, recorded: usize },
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoinFourD6Progress {
    recorded: usize,
    completed_candidates: usize,
    rejected_candidates: usize,
    valid: bool,
    stage: CoinFourD6Stage,
}

impl CoinFourD6Progress {
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }

    #[must_use]
    pub const fn minimum(self) -> usize {
        REQUIRED_CANDIDATES * OBSERVATIONS_PER_CANDIDATE
    }

    #[must_use]
    pub const fn completed_candidates(self) -> usize {
        self.completed_candidates
    }

    #[must_use]
    pub const fn rejected_candidates(self) -> usize {
        self.rejected_candidates
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.valid
    }

    #[must_use]
    pub const fn stage(self) -> CoinFourD6Stage {
        self.stage
    }

    #[must_use]
    pub const fn expected_kind(self) -> Option<CoinFourD6ObservationKind> {
        match self.stage {
            CoinFourD6Stage::Coin { .. } => Some(CoinFourD6ObservationKind::Coin),
            CoinFourD6Stage::D6 { .. } => Some(CoinFourD6ObservationKind::D6),
            CoinFourD6Stage::Complete => None,
        }
    }
}

struct ParsedCapture {
    progress: CoinFourD6Progress,
    indices: Vec<u16>,
}

impl Drop for ParsedCapture {
    fn drop(&mut self) {
        self.indices.zeroize();
    }
}

/// Maps one coin side and four ordered D6 faces to the pinned direct-word table.
///
/// Product-wide coin values are `heads = 1`, `tails = 0`. Heads occupies table
/// indices 0..1295; tails occupies 1296..2047 and rejects later tuples.
#[must_use]
pub fn coin_four_d6_word_index(mut coin: CoinFlip, mut faces: [DieFace; 4]) -> Option<u16> {
    let rank = calculate_rank(&faces);
    let index = if coin.get() == 1 || rank < 752 {
        Some(calculate_word_index(coin, rank))
    } else {
        None
    };
    coin.zeroize();
    faces.zeroize();
    index
}

fn calculate_rank(faces: &[DieFace]) -> u16 {
    faces
        .iter()
        .fold(0_u16, |value, face| value * 6 + u16::from(face.get() - 1))
}

fn calculate_word_index(coin: CoinFlip, rank: u16) -> u16 {
    if coin.get() == 1 { rank } else { 1_296 + rank }
}

#[must_use]
pub fn coin_four_d6_progress(
    target: EntropyTarget,
    capture: &CoinFourD6Capture,
) -> CoinFourD6Progress {
    parse_capture(target, capture).progress
}

fn parse_capture(target: EntropyTarget, capture: &CoinFourD6Capture) -> ParsedCapture {
    let mut stage = CoinFourD6Stage::Coin { position: 1 };
    let mut coin = None;
    let mut faces = zeroize::Zeroizing::new(Vec::with_capacity(4));
    let mut indices = Vec::with_capacity(REQUIRED_CANDIDATES);
    let mut rejected_candidates = 0;
    let mut valid = target == EntropyTarget::Words12;

    if valid {
        for observation in capture.observations() {
            match (stage, observation) {
                (CoinFourD6Stage::Coin { position }, CoinFourD6Observation::Coin(flip)) => {
                    coin = Some(*flip);
                    stage = CoinFourD6Stage::D6 {
                        position,
                        recorded: 0,
                    };
                }
                (CoinFourD6Stage::D6 { position, recorded }, CoinFourD6Observation::D6(face)) => {
                    faces.push(*face);
                    if recorded + 1 < 4 {
                        stage = CoinFourD6Stage::D6 {
                            position,
                            recorded: recorded + 1,
                        };
                        continue;
                    }

                    let candidate_faces: [DieFace; 4] = faces
                        .as_slice()
                        .try_into()
                        .expect("candidate completes after four D6 faces");
                    let candidate_coin = coin.take().expect("D6 stage follows a coin");
                    if let Some(index) = coin_four_d6_word_index(candidate_coin, candidate_faces) {
                        indices.push(index);
                        stage = if position == REQUIRED_CANDIDATES {
                            CoinFourD6Stage::Complete
                        } else {
                            CoinFourD6Stage::Coin {
                                position: position + 1,
                            }
                        };
                    } else {
                        rejected_candidates += 1;
                        stage = CoinFourD6Stage::Coin { position };
                    }
                    faces.zeroize();
                }
                _ => {
                    valid = false;
                    break;
                }
            }
        }
    }

    coin.zeroize();
    ParsedCapture {
        progress: CoinFourD6Progress {
            recorded: capture.len(),
            completed_candidates: indices.len(),
            rejected_candidates,
            valid,
            stage,
        },
        indices,
    }
}

pub(crate) fn coin_four_d6_entropy(
    target: EntropyTarget,
    capture: &CoinFourD6Capture,
) -> Result<Entropy, ProtocolError> {
    if target != EntropyTarget::Words12 {
        return Err(ProtocolError::UnsupportedTarget);
    }
    let parsed = parse_capture(target, capture);
    if !parsed.progress.is_valid() {
        return Err(ProtocolError::WrongObservationKind);
    }
    if parsed.progress.stage() != CoinFourD6Stage::Complete {
        return Err(ProtocolError::WrongObservationCount {
            expected: REQUIRED_CANDIDATES * OBSERVATIONS_PER_CANDIDATE,
            actual: capture.len(),
        });
    }

    Ok(calculate_entropy(target, &parsed.indices))
}

fn calculate_entropy(target: EntropyTarget, indices: &[u16]) -> Entropy {
    let mut bytes = vec![0_u8; target.entropy_bytes()];
    let mut bit_offset = 0;
    for &index in &indices[..11] {
        append_bits(&mut bytes, &mut bit_offset, index, 11);
    }
    append_bits(&mut bytes, &mut bit_offset, indices[11] >> 4, 7);
    assert_eq!(bit_offset, 128);
    Entropy::from_protocol_bytes(target, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face(value: u8) -> DieFace {
        DieFace::new(value).unwrap()
    }

    fn candidate(capture: &mut CoinFourD6Capture, coin: u8, values: [u8; 4]) {
        capture.push_coin(CoinFlip::new(coin).unwrap());
        for value in values {
            capture.push_d6(face(value));
        }
    }

    #[test]
    fn table_boundaries_fix_coin_polarity_and_d6_order() {
        assert_eq!(
            coin_four_d6_word_index(CoinFlip::new(1).unwrap(), [face(1); 4]),
            Some(0)
        );
        assert_eq!(
            coin_four_d6_word_index(CoinFlip::new(1).unwrap(), [face(6); 4]),
            Some(1_295)
        );
        assert_eq!(
            coin_four_d6_word_index(CoinFlip::new(0).unwrap(), [face(1); 4]),
            Some(1_296)
        );
        assert_eq!(
            coin_four_d6_word_index(
                CoinFlip::new(0).unwrap(),
                [face(4), face(3), face(6), face(2)]
            ),
            Some(2_047)
        );
        assert_eq!(
            coin_four_d6_word_index(
                CoinFlip::new(0).unwrap(),
                [face(4), face(3), face(6), face(3)]
            ),
            None
        );
    }

    #[test]
    fn accepted_table_has_one_physical_preimage_per_index() {
        let mut counts = [0_u8; 2_048];
        let mut rejected = 0;
        for coin in 0..=1 {
            for d1 in 1..=6 {
                for d2 in 1..=6 {
                    for d3 in 1..=6 {
                        for d4 in 1..=6 {
                            match coin_four_d6_word_index(
                                CoinFlip::new(coin).unwrap(),
                                [face(d1), face(d2), face(d3), face(d4)],
                            ) {
                                Some(index) => counts[usize::from(index)] += 1,
                                None => rejected += 1,
                            }
                        }
                    }
                }
            }
        }
        assert!(counts.iter().all(|count| *count == 1));
        assert_eq!(rejected, 544);
    }

    #[test]
    fn rejection_retries_the_complete_candidate_and_replay_supports_undo() {
        let mut capture = CoinFourD6Capture::new();
        candidate(&mut capture, 0, [4, 3, 6, 3]);
        let progress = coin_four_d6_progress(EntropyTarget::Words12, &capture);
        assert_eq!(progress.rejected_candidates(), 1);
        assert_eq!(progress.stage(), CoinFourD6Stage::Coin { position: 1 });
        assert!(capture.remove_last());
        assert_eq!(
            coin_four_d6_progress(EntropyTarget::Words12, &capture).stage(),
            CoinFourD6Stage::D6 {
                position: 1,
                recorded: 3
            }
        );
    }

    #[test]
    fn zero_and_maximum_streams_produce_entropy_boundaries() {
        for (coin, faces, expected) in [(1, [1; 4], vec![0; 16]), (0, [4, 3, 6, 2], vec![0xff; 16])]
        {
            let mut capture = CoinFourD6Capture::new();
            for _ in 0..12 {
                candidate(&mut capture, coin, faces);
            }
            assert_eq!(
                coin_four_d6_entropy(EntropyTarget::Words12, &capture)
                    .unwrap()
                    .bytes(),
                expected
            );
            assert_eq!(capture.len(), 60);
        }
    }

    #[test]
    fn wrong_kind_and_unsupported_target_are_explicit() {
        let mut capture = CoinFourD6Capture::new();
        capture.push_d6(face(1));
        assert!(!coin_four_d6_progress(EntropyTarget::Words12, &capture).is_valid());
        assert_eq!(
            coin_four_d6_entropy(EntropyTarget::Words24, &capture),
            Err(ProtocolError::UnsupportedTarget)
        );
    }
}
