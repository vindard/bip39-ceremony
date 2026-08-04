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
    jade::{D8Face, D16Face, InvalidD8Face, InvalidD16Face, JadeCapture, JadeObservation},
    protocol::{
        AssignmentStatus, CandidatePurpose, CandidateStatus, CanonicalInput, CanonicalInputKind,
        CaptureAssessment, CaptureProgress, Compatibility, ConversionProtocol, JadeDieKind,
        JadeProgress, JadeStage, ProtocolError, ProtocolSpecification, RejectionPolicy,
        WordExactCandidate, WordExactProgress, WordExactTrace, jade_expected_die, jade_progress,
        jade_required_observations, jade_word_index, trace_word_exact,
    },
};
