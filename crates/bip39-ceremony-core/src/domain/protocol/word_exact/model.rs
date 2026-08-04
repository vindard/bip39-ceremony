use crate::domain::bip39::Entropy;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WordExactProgress {
    WordCandidate {
        accepted_words: usize,
        required_words: usize,
        candidate_rolls: usize,
        rejected_candidates: usize,
    },
    EntropyTail {
        accepted_words: usize,
        required_words: usize,
        candidate_rolls: usize,
        candidate_size: usize,
        rejected_candidates: usize,
    },
}

impl WordExactProgress {
    #[must_use]
    pub const fn accepted_words(self) -> usize {
        match self {
            Self::WordCandidate { accepted_words, .. }
            | Self::EntropyTail { accepted_words, .. } => accepted_words,
        }
    }

    #[must_use]
    pub const fn required_words(self) -> usize {
        match self {
            Self::WordCandidate { required_words, .. }
            | Self::EntropyTail { required_words, .. } => required_words,
        }
    }

    #[must_use]
    pub const fn rejected_candidates(self) -> usize {
        match self {
            Self::WordCandidate {
                rejected_candidates,
                ..
            }
            | Self::EntropyTail {
                rejected_candidates,
                ..
            } => rejected_candidates,
        }
    }
}

pub struct CompletedWordExact {
    entropy: Entropy,
    consumed_rolls: usize,
    accepted_words: usize,
    required_words: usize,
    rejected_candidates: usize,
}

impl CompletedWordExact {
    pub(super) fn new(
        entropy: Entropy,
        consumed_rolls: usize,
        accepted_words: usize,
        required_words: usize,
        rejected_candidates: usize,
    ) -> Self {
        Self {
            entropy,
            consumed_rolls,
            accepted_words,
            required_words,
            rejected_candidates,
        }
    }

    #[must_use]
    pub const fn consumed_rolls(&self) -> usize {
        self.consumed_rolls
    }

    #[must_use]
    pub const fn accepted_words(&self) -> usize {
        self.accepted_words
    }

    #[must_use]
    pub const fn required_words(&self) -> usize {
        self.required_words
    }

    #[must_use]
    pub const fn rejected_candidates(&self) -> usize {
        self.rejected_candidates
    }

    #[must_use]
    pub fn into_entropy(self) -> Entropy {
        self.entropy
    }
}

impl core::fmt::Debug for CompletedWordExact {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("CompletedWordExact")
            .field("entropy", &"[REDACTED]")
            .field("consumed_rolls", &self.consumed_rolls)
            .field("accepted_words", &self.accepted_words)
            .field("required_words", &self.required_words)
            .field("rejected_candidates", &self.rejected_candidates)
            .finish()
    }
}

#[derive(Debug)]
pub enum WordExactParse {
    Collecting(WordExactProgress),
    Complete(CompletedWordExact),
}
