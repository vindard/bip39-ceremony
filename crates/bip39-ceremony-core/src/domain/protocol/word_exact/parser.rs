use zeroize::Zeroizing;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::{DieFace, RollSequence},
    protocol::ProtocolError,
};

use super::{
    CandidatePurpose, CandidateStatus, CompletedWordExact, WORD_CANDIDATE_LIMIT,
    WORD_CANDIDATE_ROLLS, WordExactCandidate, WordExactParse, WordExactProgress, WordExactTrace,
    tail_parameters,
};

struct WordExactAnalysis {
    parse: WordExactParse,
    trace: WordExactTrace,
}

struct AcceptedWords {
    indices: Zeroizing<Vec<u16>>,
    offset: usize,
    rejected_candidates: usize,
    candidates: Vec<WordExactCandidate>,
    required_words: usize,
}

#[must_use]
pub fn parse_word_exact(target: EntropyTarget, rolls: &RollSequence) -> WordExactParse {
    analyze_word_exact(target, rolls).parse
}

#[must_use]
pub fn trace_word_exact(target: EntropyTarget, rolls: &RollSequence) -> WordExactTrace {
    analyze_word_exact(target, rolls).trace
}

fn analyze_word_exact(target: EntropyTarget, rolls: &RollSequence) -> WordExactAnalysis {
    let required_words = target.word_count() - 1;
    let mut offset = 0;
    let mut rejected_candidates = 0;
    let mut candidates = Vec::new();
    let mut indices = Zeroizing::new(Vec::with_capacity(required_words));

    while indices.len() < required_words {
        let position = indices.len() + 1;
        let attempt = candidates
            .iter()
            .rev()
            .take_while(|candidate: &&WordExactCandidate| {
                candidate.purpose() == CandidatePurpose::WordPosition(position)
            })
            .count()
            + 1;
        let remaining = rolls.len().saturating_sub(offset);
        let recorded = remaining.min(WORD_CANDIDATE_ROLLS);
        if recorded < WORD_CANDIDATE_ROLLS {
            candidates.push(WordExactCandidate::new(
                CandidatePurpose::WordPosition(position),
                attempt,
                offset + 1,
                recorded,
                WORD_CANDIDATE_ROLLS,
                CandidateStatus::Collecting,
            ));
            return collecting_analysis(
                candidates,
                required_words,
                WordExactProgress::WordCandidate {
                    accepted_words: indices.len(),
                    required_words,
                    candidate_rolls: recorded,
                    rejected_candidates,
                },
            );
        }

        let value = Zeroizing::new(base6_value(
            &rolls.faces()[offset..offset + WORD_CANDIDATE_ROLLS],
        ));
        let accepted = *value < WORD_CANDIDATE_LIMIT;
        candidates.push(WordExactCandidate::new(
            CandidatePurpose::WordPosition(position),
            attempt,
            offset + 1,
            WORD_CANDIDATE_ROLLS,
            WORD_CANDIDATE_ROLLS,
            candidate_status(accepted),
        ));
        offset += WORD_CANDIDATE_ROLLS;
        if accepted {
            indices.push(lower_u16(*value % 2_048));
        } else {
            rejected_candidates += 1;
        }
    }

    analyze_entropy_tail(
        target,
        rolls,
        AcceptedWords {
            indices,
            offset,
            rejected_candidates,
            candidates,
            required_words,
        },
    )
}

fn analyze_entropy_tail(
    target: EntropyTarget,
    rolls: &RollSequence,
    mut accepted: AcceptedWords,
) -> WordExactAnalysis {
    let (tail_rolls, tail_limit, tail_space, tail_bits) = tail_parameters(target);
    let mut attempt = 1;
    loop {
        let remaining = rolls.len().saturating_sub(accepted.offset);
        let recorded = remaining.min(tail_rolls);
        if recorded < tail_rolls {
            accepted.candidates.push(WordExactCandidate::new(
                CandidatePurpose::EntropyTail,
                attempt,
                accepted.offset + 1,
                recorded,
                tail_rolls,
                CandidateStatus::Collecting,
            ));
            return collecting_analysis(
                accepted.candidates,
                accepted.required_words,
                WordExactProgress::EntropyTail {
                    accepted_words: accepted.indices.len(),
                    required_words: accepted.required_words,
                    candidate_rolls: recorded,
                    candidate_size: tail_rolls,
                    rejected_candidates: accepted.rejected_candidates,
                },
            );
        }

        let value = Zeroizing::new(base6_value(
            &rolls.faces()[accepted.offset..accepted.offset + tail_rolls],
        ));
        let is_accepted = *value < tail_limit;
        accepted.candidates.push(WordExactCandidate::new(
            CandidatePurpose::EntropyTail,
            attempt,
            accepted.offset + 1,
            tail_rolls,
            tail_rolls,
            candidate_status(is_accepted),
        ));
        accepted.offset += tail_rolls;
        if is_accepted {
            let tail = lower_u16(*value % tail_space);
            let entropy = assemble_entropy(target, &accepted.indices, tail, tail_bits);
            return WordExactAnalysis {
                parse: WordExactParse::Complete(CompletedWordExact::new(
                    entropy,
                    accepted.offset,
                    accepted.indices.len(),
                    accepted.required_words,
                    accepted.rejected_candidates,
                )),
                trace: WordExactTrace::new(accepted.candidates, accepted.required_words),
            };
        }
        accepted.rejected_candidates += 1;
        attempt += 1;
    }
}

fn collecting_analysis(
    candidates: Vec<WordExactCandidate>,
    required_words: usize,
    progress: WordExactProgress,
) -> WordExactAnalysis {
    WordExactAnalysis {
        parse: WordExactParse::Collecting(progress),
        trace: WordExactTrace::new(candidates, required_words),
    }
}

const fn candidate_status(accepted: bool) -> CandidateStatus {
    if accepted {
        CandidateStatus::Accepted
    } else {
        CandidateStatus::Rejected
    }
}

/// Converts a complete localized-rejection stream into BIP-39 entropy.
///
/// # Errors
///
/// Returns [`ProtocolError::IncompleteWordExact`] while a candidate is still
/// collecting, or [`ProtocolError::WrongObservationCount`] if rolls follow completion.
pub fn word_exact_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<Entropy, ProtocolError> {
    match parse_word_exact(target, rolls) {
        WordExactParse::Collecting(_) => Err(ProtocolError::IncompleteWordExact),
        WordExactParse::Complete(completed) if completed.consumed_rolls() != rolls.len() => {
            Err(ProtocolError::WrongObservationCount {
                expected: completed.consumed_rolls(),
                actual: rolls.len(),
            })
        }
        WordExactParse::Complete(completed) => Ok(completed.into_entropy()),
    }
}

fn base6_value(faces: &[DieFace]) -> u32 {
    faces
        .iter()
        .fold(0, |value, face| value * 6 + u32::from(face.base6_digit()))
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

fn lower_u16(value: u32) -> u16 {
    let bytes = value.to_be_bytes();
    u16::from_be_bytes([bytes[2], bytes[3]])
}

fn append_bits(bytes: &mut [u8], offset: &mut usize, value: u16, bits: usize) {
    for shift in (0..bits).rev() {
        let bit = (value >> shift).to_le_bytes()[0] & 1;
        bytes[*offset / 8] |= bit << (7 - *offset % 8);
        *offset += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rolls(value: &str) -> RollSequence {
        let mut result = RollSequence::new();
        for character in value.chars() {
            result.push(DieFace::try_from(character).unwrap());
        }
        result
    }

    fn encoded_base6(mut value: u32, digits: usize) -> String {
        let mut faces = vec!['1'; digits];
        for face in faces.iter_mut().rev() {
            *face = char::from_digit(value % 6 + 1, 10).unwrap();
            value /= 6;
        }
        assert_eq!(value, 0);
        faces.into_iter().collect()
    }

    #[test]
    fn candidate_boundary_is_uniform() {
        assert_eq!(
            base6_value(rolls(&encoded_base6(45_055, 6)).faces()),
            45_055
        );
        assert_eq!(
            base6_value(rolls(&encoded_base6(45_056, 6)).faces()),
            45_056
        );

        let mut counts = vec![0_u8; 2_048];
        for value in 0..WORD_CANDIDATE_LIMIT {
            counts[usize::try_from(value % 2_048).unwrap()] += 1;
        }
        assert!(counts.into_iter().all(|count| count == 22));
    }

    #[test]
    fn zero_stream_builds_zero_entropy() {
        for (target, count) in [(EntropyTarget::Words12, 70), (EntropyTarget::Words24, 140)] {
            let sequence = rolls(&"1".repeat(count));
            assert!(matches!(
                parse_word_exact(target, &sequence),
                WordExactParse::Complete(_)
            ));
            assert_eq!(
                word_exact_entropy(target, &sequence).unwrap().bytes(),
                vec![0; target.entropy_bytes()]
            );
        }
    }

    #[test]
    fn tail_spaces_have_equal_preimages() {
        let mut seven_bit_counts = [0_u8; 128];
        for value in 0..1_280 {
            seven_bit_counts[value % 128] += 1;
        }
        assert!(seven_bit_counts.into_iter().all(|count| count == 10));

        let mut three_bit_counts = [0_u8; 8];
        for value in 0..32 {
            three_bit_counts[value % 8] += 1;
        }
        assert!(three_bit_counts.into_iter().all(|count| count == 4));
    }

    #[test]
    fn packs_word_indices_and_tail_bits_msb_first() {
        let sequence = rolls(&format!(
            "{}{}{}",
            encoded_base6(2_047, 6),
            "1".repeat(60),
            encoded_base6(127, 4)
        ));
        let entropy = word_exact_entropy(EntropyTarget::Words12, &sequence).unwrap();
        let mut expected = vec![0_u8; 16];
        expected[0] = 0xff;
        expected[1] = 0xe0;
        expected[15] = 0x7f;
        assert_eq!(entropy.bytes(), expected);
    }

    #[test]
    fn rejects_only_the_local_candidate() {
        let sequence = rolls(&format!("{}{}", encoded_base6(45_056, 6), "1".repeat(70)));
        let WordExactParse::Complete(completed) =
            parse_word_exact(EntropyTarget::Words12, &sequence)
        else {
            panic!("stream should complete");
        };
        assert_eq!(completed.rejected_candidates(), 1);
        assert_eq!(completed.into_entropy().bytes(), [0; 16]);
    }

    #[test]
    fn retries_only_a_rejected_entropy_tail() {
        let sequence = rolls(&format!(
            "{}{}{}",
            "1".repeat(66),
            encoded_base6(1_280, 4),
            encoded_base6(0, 4)
        ));
        let WordExactParse::Complete(completed) =
            parse_word_exact(EntropyTarget::Words12, &sequence)
        else {
            panic!("stream should complete");
        };
        assert_eq!(completed.rejected_candidates(), 1);
        assert_eq!(completed.into_entropy().bytes(), [0; 16]);
    }

    #[test]
    fn trace_assigns_retries_to_the_same_position() {
        let sequence = rolls(&format!("{}{}111", encoded_base6(45_056, 6), "1".repeat(6)));
        let trace = trace_word_exact(EntropyTarget::Words12, &sequence);
        let candidates = trace.candidates();

        assert_eq!(
            candidates,
            [
                WordExactCandidate::new(
                    CandidatePurpose::WordPosition(1),
                    1,
                    1,
                    6,
                    6,
                    CandidateStatus::Rejected,
                ),
                WordExactCandidate::new(
                    CandidatePurpose::WordPosition(1),
                    2,
                    7,
                    6,
                    6,
                    CandidateStatus::Accepted,
                ),
                WordExactCandidate::new(
                    CandidatePurpose::WordPosition(2),
                    1,
                    13,
                    3,
                    6,
                    CandidateStatus::Collecting,
                ),
            ]
        );
    }

    #[test]
    fn reports_typed_word_and_tail_progress() {
        let WordExactParse::Collecting(WordExactProgress::WordCandidate {
            candidate_rolls, ..
        }) = parse_word_exact(EntropyTarget::Words12, &rolls("111"))
        else {
            panic!("partial word candidate expected");
        };
        assert_eq!(candidate_rolls, 3);

        let WordExactParse::Collecting(WordExactProgress::EntropyTail {
            accepted_words,
            candidate_rolls,
            candidate_size,
            ..
        }) = parse_word_exact(EntropyTarget::Words12, &rolls(&"1".repeat(68)))
        else {
            panic!("partial entropy tail expected");
        };
        assert_eq!(accepted_words, 11);
        assert_eq!(candidate_rolls, 2);
        assert_eq!(candidate_size, 4);
    }
}
