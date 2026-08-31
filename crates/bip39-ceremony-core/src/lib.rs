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
    bitbox::{BitBoxCapture, BitBoxObservation},
    coin::{CoinFlip, FlipSequence, InvalidCoinFlip},
    coin_four_d6::{CoinFourD6Capture, CoinFourD6Observation},
    d20::{D20Face, D20RollSequence, InvalidD20Face},
    dice::{DieFace, InvalidDieFace, RollSequence},
    jade::{D8Face, D16Face, InvalidD8Face, InvalidD16Face, JadeCapture, JadeObservation},
    protocol::{
        AssignmentStatus, BitBoxObservationKind, BitBoxProgress, BitBoxStage, BitPackProgress,
        CandidatePurpose, CandidateStatus, CanonicalInput, CanonicalInputKind, CaptureAssessment,
        CaptureProgress, CoinFourD6ObservationKind, CoinFourD6Progress, CoinFourD6Stage,
        Compatibility, ConversionProtocol, IanColemanRawProgress, JadeDieKind, JadeProgress,
        JadeStage, ProtocolError, ProtocolSpecification, RejectionPolicy, WordExactCandidate,
        WordExactProgress, WordExactTrace, bitbox_progress, bitbox_required_observations,
        bitbox_tail_bits, bitbox_word_index, bitpack_progress, coin_four_d6_progress,
        coin_four_d6_word_index, face_bits, iancoleman_raw_progress, jade_expected_die,
        jade_progress, jade_required_observations, jade_word_index, raw_face_bits,
        trace_word_exact,
    },
};
