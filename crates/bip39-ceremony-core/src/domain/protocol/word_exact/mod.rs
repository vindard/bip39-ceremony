mod model;
mod parser;
mod trace;

pub(super) const WORD_CANDIDATE_ROLLS: usize = 6;
pub(super) const WORD_CANDIDATE_LIMIT: u32 = 45_056;

pub(super) const fn tail_parameters(
    target: crate::domain::bip39::EntropyTarget,
) -> (usize, u32, u32, usize) {
    match target {
        crate::domain::bip39::EntropyTarget::Words12 => (4, 1_280, 128, 7),
        crate::domain::bip39::EntropyTarget::Words24 => (2, 32, 8, 3),
    }
}

pub use model::{CompletedWordExact, WordExactParse, WordExactProgress};
pub use parser::trace_word_exact;
pub use parser::{parse_word_exact, word_exact_entropy};
pub use trace::{
    AssignmentStatus, CandidatePurpose, CandidateStatus, WordExactCandidate, WordExactTrace,
};
