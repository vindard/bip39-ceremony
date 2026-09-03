use zeroize::Zeroize;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    bitbox::{BitBoxCapture, BitBoxObservation},
    coin::CoinFlip,
    dice::DieFace,
};

use super::{ProtocolError, append_bits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BitBoxObservationKind {
    D6,
    Coin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BitBoxStage {
    DirectWordD6 {
        position: usize,
        accepted_faces: usize,
    },
    DirectWordCoin {
        position: usize,
    },
    EntropyTail {
        recorded: usize,
        required: usize,
    },
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BitBoxProgress {
    recorded: usize,
    minimum: usize,
    completed_words: usize,
    required_words: usize,
    rejected_faces: usize,
    valid: bool,
    stage: BitBoxStage,
}

impl BitBoxProgress {
    #[must_use]
    pub const fn recorded(self) -> usize {
        self.recorded
    }

    #[must_use]
    pub const fn minimum(self) -> usize {
        self.minimum
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
    pub const fn rejected_faces(self) -> usize {
        self.rejected_faces
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.valid
    }

    #[must_use]
    pub const fn stage(self) -> BitBoxStage {
        self.stage
    }

    #[must_use]
    pub const fn expected_kind(self) -> Option<BitBoxObservationKind> {
        match self.stage {
            BitBoxStage::DirectWordD6 { .. } => Some(BitBoxObservationKind::D6),
            BitBoxStage::DirectWordCoin { .. } | BitBoxStage::EntropyTail { .. } => {
                Some(BitBoxObservationKind::Coin)
            }
            BitBoxStage::Complete => None,
        }
    }
}

struct ParsedBitBox {
    progress: BitBoxProgress,
    indices: Vec<u16>,
    tail: u16,
}

impl Drop for ParsedBitBox {
    fn drop(&mut self) {
        self.indices.zeroize();
        self.tail.zeroize();
    }
}

#[must_use]
pub const fn bitbox_required_observations(target: EntropyTarget) -> usize {
    match target {
        EntropyTarget::Words12 => 73,
        EntropyTarget::Words24 => 141,
    }
}

#[must_use]
pub const fn bitbox_tail_bits(target: EntropyTarget) -> usize {
    match target {
        EntropyTarget::Words12 => 7,
        EntropyTarget::Words24 => 3,
    }
}

/// Maps five accepted D6 faces and a physical coin side to a BIP-39 index.
///
/// `BitBox02`'s table orders heads before tails, so its selector bit is
/// `heads = 0`, `tails = 1`, opposite the capture key values.
#[must_use]
pub fn bitbox_word_index(mut faces: [DieFace; 5], mut coin: CoinFlip) -> Option<u16> {
    let index = if faces.iter().any(|face| face.get() > 4) {
        None
    } else {
        Some(calculate_word_index(&faces, coin))
    };
    faces.zeroize();
    coin.zeroize();
    index
}

fn calculate_word_index(faces: &[DieFace], coin: CoinFlip) -> u16 {
    let base4 = faces
        .iter()
        .fold(0_u16, |value, face| value * 4 + u16::from(face.get() - 1));
    base4 * 2 + u16::from(1 - coin.get())
}

#[must_use]
pub fn bitbox_progress(target: EntropyTarget, capture: &BitBoxCapture) -> BitBoxProgress {
    parse_bitbox(target, capture).progress
}

fn parse_bitbox(target: EntropyTarget, capture: &BitBoxCapture) -> ParsedBitBox {
    let required_words = target.word_count() - 1;
    let required_tail = bitbox_tail_bits(target);
    let mut stage = BitBoxStage::DirectWordD6 {
        position: 1,
        accepted_faces: 0,
    };
    let mut current_faces = zeroize::Zeroizing::new(Vec::with_capacity(5));
    let mut indices = Vec::with_capacity(required_words);
    let mut tail = 0_u16;
    let mut rejected_faces = 0;
    let mut valid = true;

    for observation in capture.observations() {
        match (stage, observation) {
            (
                BitBoxStage::DirectWordD6 {
                    position,
                    accepted_faces,
                },
                BitBoxObservation::D6(face),
            ) => {
                if face.get() <= 4 {
                    current_faces.push(*face);
                    stage = if accepted_faces + 1 == 5 {
                        BitBoxStage::DirectWordCoin { position }
                    } else {
                        BitBoxStage::DirectWordD6 {
                            position,
                            accepted_faces: accepted_faces + 1,
                        }
                    };
                } else {
                    rejected_faces += 1;
                }
            }
            (BitBoxStage::DirectWordCoin { position }, BitBoxObservation::Coin(coin)) => {
                let faces: [DieFace; 5] = current_faces
                    .as_slice()
                    .try_into()
                    .expect("coin follows five accepted faces");
                indices.push(
                    bitbox_word_index(faces, *coin).expect("parser retains accepted D6 faces"),
                );
                current_faces.zeroize();
                stage = if position == required_words {
                    BitBoxStage::EntropyTail {
                        recorded: 0,
                        required: required_tail,
                    }
                } else {
                    BitBoxStage::DirectWordD6 {
                        position: position + 1,
                        accepted_faces: 0,
                    }
                };
            }
            (BitBoxStage::EntropyTail { recorded, required }, BitBoxObservation::Coin(coin)) => {
                tail = append_entropy_tail(tail, *coin);
                stage = if recorded + 1 == required {
                    BitBoxStage::Complete
                } else {
                    BitBoxStage::EntropyTail {
                        recorded: recorded + 1,
                        required,
                    }
                };
            }
            _ => {
                valid = false;
                break;
            }
        }
    }

    ParsedBitBox {
        progress: BitBoxProgress {
            recorded: capture.len(),
            minimum: bitbox_required_observations(target),
            completed_words: indices.len(),
            required_words,
            rejected_faces,
            valid,
            stage,
        },
        indices,
        tail,
    }
}

fn append_entropy_tail(tail: u16, coin: CoinFlip) -> u16 {
    (tail << 1) | u16::from(1 - coin.get())
}

pub(crate) fn bitbox_entropy(
    target: EntropyTarget,
    capture: &BitBoxCapture,
) -> Result<Entropy, ProtocolError> {
    let parsed = parse_bitbox(target, capture);
    if !parsed.progress.is_valid() {
        return Err(ProtocolError::WrongObservationKind);
    }
    if parsed.progress.stage() != BitBoxStage::Complete {
        return Err(ProtocolError::WrongObservationCount {
            expected: bitbox_required_observations(target),
            actual: capture.len(),
        });
    }

    Ok(calculate_entropy(target, &parsed))
}

fn calculate_entropy(target: EntropyTarget, parsed: &ParsedBitBox) -> Entropy {
    let mut bytes = vec![0_u8; target.entropy_bytes()];
    let mut bit_offset = 0;
    for &index in &parsed.indices {
        append_bits(&mut bytes, &mut bit_offset, index, 11);
    }
    append_bits(
        &mut bytes,
        &mut bit_offset,
        parsed.tail,
        bitbox_tail_bits(target),
    );
    assert_eq!(bit_offset, target.entropy_bits());
    Entropy::from_protocol_bytes(target, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted_face(value: u8) -> DieFace {
        DieFace::new(value).unwrap()
    }

    fn all_heads(target: EntropyTarget) -> BitBoxCapture {
        let mut capture = BitBoxCapture::new();
        for _ in 0..target.word_count() - 1 {
            for _ in 0..5 {
                capture.push_d6(accepted_face(1));
            }
            capture.push_coin(CoinFlip::new(1).unwrap());
        }
        for _ in 0..bitbox_tail_bits(target) {
            capture.push_coin(CoinFlip::new(1).unwrap());
        }
        capture
    }

    #[test]
    fn lookup_boundaries_match_the_published_table() {
        assert_eq!(
            bitbox_word_index([accepted_face(1); 5], CoinFlip::new(1).unwrap()),
            Some(0)
        );
        assert_eq!(
            bitbox_word_index([accepted_face(1); 5], CoinFlip::new(0).unwrap()),
            Some(1)
        );
        assert_eq!(
            bitbox_word_index([accepted_face(4); 5], CoinFlip::new(0).unwrap()),
            Some(2_047)
        );
        assert_eq!(
            bitbox_word_index([accepted_face(5); 5], CoinFlip::new(1).unwrap()),
            None
        );
    }

    #[test]
    fn rejection_retries_only_the_current_face() {
        let mut capture = BitBoxCapture::new();
        capture.push_d6(accepted_face(1));
        capture.push_d6(accepted_face(6));
        let progress = bitbox_progress(EntropyTarget::Words12, &capture);
        assert_eq!(progress.rejected_faces(), 1);
        assert_eq!(
            progress.stage(),
            BitBoxStage::DirectWordD6 {
                position: 1,
                accepted_faces: 1,
            }
        );
        assert!(progress.is_valid());
    }

    #[test]
    fn undo_replay_crosses_die_and_coin_boundaries() {
        let mut capture = BitBoxCapture::new();
        for _ in 0..5 {
            capture.push_d6(accepted_face(1));
        }
        assert!(matches!(
            bitbox_progress(EntropyTarget::Words12, &capture).stage(),
            BitBoxStage::DirectWordCoin { position: 1 }
        ));
        assert!(capture.remove_last());
        assert_eq!(
            bitbox_progress(EntropyTarget::Words12, &capture).stage(),
            BitBoxStage::DirectWordD6 {
                position: 1,
                accepted_faces: 4,
            }
        );
        capture.push_d6(accepted_face(1));
        capture.push_coin(CoinFlip::new(1).unwrap());
        assert!(capture.remove_last());
        assert!(matches!(
            bitbox_progress(EntropyTarget::Words12, &capture).stage(),
            BitBoxStage::DirectWordCoin { position: 1 }
        ));
    }

    #[test]
    fn undo_replay_crosses_the_entropy_tail_boundary() {
        let mut complete = all_heads(EntropyTarget::Words24);
        assert!(complete.remove_last());
        assert_eq!(
            bitbox_progress(EntropyTarget::Words24, &complete).stage(),
            BitBoxStage::EntropyTail {
                recorded: 2,
                required: 3,
            }
        );
    }

    #[test]
    fn wrong_kind_is_invalid_and_all_heads_are_zero_entropy() {
        let mut invalid = BitBoxCapture::new();
        invalid.push_coin(CoinFlip::new(1).unwrap());
        assert!(!bitbox_progress(EntropyTarget::Words12, &invalid).is_valid());

        for target in [EntropyTarget::Words12, EntropyTarget::Words24] {
            let capture = all_heads(target);
            assert_eq!(capture.len(), bitbox_required_observations(target));
            assert_eq!(
                bitbox_entropy(target, &capture).unwrap().bytes(),
                vec![0; target.entropy_bytes()]
            );
        }
    }
}
