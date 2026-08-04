#![forbid(unsafe_code)]

//! Deterministic, inspectable physical-capture to English BIP-39 calculations.
//!
//! Secret-bearing values redact their `Debug` output and zeroize owned buffers
//! where practical. Accessors returning observations, entropy, evidence, or
//! words deliberately reveal wallet-secret material.

mod calculation;
mod domain;

pub use calculation::{
    Calculation, CalculationError, CalculationEvidence, CalculationOutcome, Capture, calculate,
};
pub use domain::{
    bip39::{Bip39Error, EnglishMnemonic, Entropy, EntropyTarget},
    coin::{CoinFlip, FlipSequence, InvalidCoinFlip},
    dice::{DieFace, InvalidDieFace, RollSequence},
    protocol::{
        AssignmentStatus, CandidatePurpose, CandidateStatus, CanonicalInput, CanonicalInputKind,
        CaptureAssessment, CaptureProgress, Compatibility, ConversionProtocol, ProtocolError,
        ProtocolSpecification, RejectionPolicy, WordExactCandidate, WordExactProgress,
        WordExactTrace, trace_word_exact,
    },
};
