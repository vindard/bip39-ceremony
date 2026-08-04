mod canonical;
mod capture;
mod coldcard;
mod error;
mod exact;
mod identity;
mod keystone_legacy;
mod native_hash;
mod specification;
mod word_exact;

pub use canonical::CanonicalInput;
pub use capture::{CaptureAssessment, CaptureProgress};
pub use error::ProtocolError;
pub use identity::ConversionProtocol;
pub use specification::{
    CanonicalInputKind, Compatibility, ProtocolSpecification, RejectionPolicy,
};
pub use word_exact::trace_word_exact;
pub use word_exact::{
    AssignmentStatus, CandidatePurpose, CandidateStatus, WordExactCandidate, WordExactParse,
    WordExactProgress, WordExactTrace,
};

pub(crate) use coldcard::ascii_rolls as coldcard_ascii_rolls;
pub(crate) use exact::{ExactOutcome, exact_entropy};
pub(crate) use keystone_legacy::ascii_rolls as keystone_legacy_ascii_rolls;
pub(crate) use native_hash::header as native_hash_header;
pub(crate) use word_exact::{parse_word_exact, word_exact_entropy};
