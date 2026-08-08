#![forbid(unsafe_code)]

//! Deterministic, inspectable physical-capture to English BIP-39 calculations.
//!
//! Scope: this crate is strictly the conversion math — physical capture to
//! entropy, and entropy to a BIP-39 mnemonic — plus the metadata describing
//! each conversion's input requirements. It is meant to stay lean and auditable
//! by a reader validating the math, since that is where the security lives, and
//! to eventually stand alone as a reusable reference for how projects convert
//! entropy into mnemonics. It takes on no non-security responsibilities (no UX,
//! presentation, comparison, or orchestration); those belong to the app layer.
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
    bitbox::{BitBoxCapture, BitBoxObservation},
    coin::{CoinFlip, FlipSequence, InvalidCoinFlip},
    coin_four_d6::{CoinFourD6Capture, CoinFourD6Observation},
    d20::{D20Face, D20RollSequence, InvalidD20Face},
    dice::{DieFace, InvalidDieFace, RollSequence},
    jade::{D8Face, D16Face, InvalidD8Face, InvalidD16Face, JadeCapture, JadeObservation},
    protocol::{
        AssignmentStatus, BitBoxObservationKind, BitBoxProgress, BitBoxStage, CandidatePurpose,
        CandidateStatus, CanonicalInput, CanonicalInputKind, CaptureAssessment, CaptureProgress,
        CoinFourD6ObservationKind, CoinFourD6Progress, CoinFourD6Stage, Compatibility,
        ConversionProtocol, JadeDieKind, JadeProgress, JadeStage, ProtocolError,
        ProtocolSpecification, RejectionPolicy, WordExactCandidate, WordExactProgress,
        WordExactTrace, bitbox_progress, bitbox_required_observations, bitbox_tail_bits,
        bitbox_word_index, coin_four_d6_progress, coin_four_d6_word_index, jade_expected_die,
        jade_progress, jade_required_observations, jade_word_index, trace_word_exact,
    },
};
