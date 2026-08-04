#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidatePurpose {
    WordPosition(usize),
    EntropyTail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentStatus {
    Waiting,
    Collecting,
    Accepted { retried: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateStatus {
    Collecting,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct WordExactCandidate {
    purpose: CandidatePurpose,
    attempt: usize,
    first_roll: usize,
    recorded_rolls: usize,
    required_rolls: usize,
    status: CandidateStatus,
}

impl WordExactCandidate {
    pub(super) const fn new(
        purpose: CandidatePurpose,
        attempt: usize,
        first_roll: usize,
        recorded_rolls: usize,
        required_rolls: usize,
        status: CandidateStatus,
    ) -> Self {
        Self {
            purpose,
            attempt,
            first_roll,
            recorded_rolls,
            required_rolls,
            status,
        }
    }

    #[must_use]
    pub const fn purpose(self) -> CandidatePurpose {
        self.purpose
    }

    #[must_use]
    pub const fn attempt(self) -> usize {
        self.attempt
    }

    #[must_use]
    pub const fn first_roll(self) -> usize {
        self.first_roll
    }

    #[must_use]
    pub const fn recorded_rolls(self) -> usize {
        self.recorded_rolls
    }

    #[must_use]
    pub const fn required_rolls(self) -> usize {
        self.required_rolls
    }

    #[must_use]
    pub const fn status(self) -> CandidateStatus {
        self.status
    }
}

impl core::fmt::Debug for WordExactCandidate {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("WordExactCandidate([REDACTED])")
    }
}

#[derive(Eq, PartialEq)]
pub struct WordExactTrace {
    candidates: Vec<WordExactCandidate>,
    required_words: usize,
}

impl core::fmt::Debug for WordExactTrace {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("WordExactTrace")
            .field("required_words", &self.required_words)
            .field("candidates", &"[REDACTED]")
            .finish()
    }
}

impl WordExactTrace {
    pub(super) const fn new(candidates: Vec<WordExactCandidate>, required_words: usize) -> Self {
        Self {
            candidates,
            required_words,
        }
    }

    #[must_use]
    pub fn candidates(&self) -> &[WordExactCandidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn required_words(&self) -> usize {
        self.required_words
    }

    #[must_use]
    pub fn assignment_status(&self, purpose: CandidatePurpose) -> AssignmentStatus {
        let mut matching = self
            .candidates
            .iter()
            .filter(|candidate| candidate.purpose == purpose);
        let accepted = matching
            .clone()
            .any(|candidate| candidate.status == CandidateStatus::Accepted);
        let collecting = matching
            .clone()
            .any(|candidate| candidate.status == CandidateStatus::Collecting);
        let retried = matching.any(|candidate| candidate.status == CandidateStatus::Rejected);
        if accepted {
            AssignmentStatus::Accepted { retried }
        } else if collecting {
            AssignmentStatus::Collecting
        } else {
            AssignmentStatus::Waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_status_encapsulates_retry_history() {
        let trace = WordExactTrace::new(
            vec![
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
            ],
            11,
        );

        assert_eq!(
            trace.assignment_status(CandidatePurpose::WordPosition(1)),
            AssignmentStatus::Accepted { retried: true }
        );
        assert_eq!(
            trace.assignment_status(CandidatePurpose::WordPosition(2)),
            AssignmentStatus::Collecting
        );
        assert_eq!(
            trace.assignment_status(CandidatePurpose::WordPosition(3)),
            AssignmentStatus::Waiting
        );
    }
}
