//! Group-compare application service.
//!
//! The UI drives group compare through this seam. Its surface is the use-cases
//! — record a roll, start a fresh set, read roll progress, compare a capture —
//! not a mirror of the domain service's getters. It composes the read model the
//! screens need (folding in the entropy-set thresholds) so presentation only
//! formats. View state (screen, browsed capture, reveal, help) lives in the UI.

use crate::domain::{
    bip39::EntropyTarget,
    dice::DieFace,
    group::{GroupComparison, GroupReport},
    protocol::ConversionProtocol,
};

use super::DerivationProjection;

/// One accepted seed inside a capture, projected to the same derivation
/// evidence a single-protocol ceremony exposes (entropy, checksum, word
/// indices, recovery words) and tagged with the entropy set it came from.
pub(crate) struct GroupDerivation {
    pub set_rolls: usize,
    pub protocol: ConversionProtocol,
    pub projection: DerivationProjection,
}

/// Roll-collection progress on the current capture, with the roll counts the
/// two entropy sets complete at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RollProgress {
    pub recorded: usize,
    pub latest_face: Option<u8>,
    pub strict_target: usize,
    pub full_target: usize,
}

/// Coordinates one group-compare session for the UI.
pub(crate) struct GroupSession {
    comparison: GroupComparison,
}

impl GroupSession {
    #[must_use]
    pub fn new(target: EntropyTarget) -> Self {
        Self {
            comparison: GroupComparison::new(target),
        }
    }

    // ── use-case operations ──────────────────────────────────────────────────

    /// Records one physical roll into the current entropy set.
    pub fn record_roll(&mut self, face: DieFace) {
        self.comparison.record_roll(face);
    }

    /// Undoes the latest roll in the current entropy set.
    pub fn undo_roll(&mut self) {
        self.comparison.undo_roll();
    }

    /// Starts a fresh entropy set — the only way to clear an invalid result.
    pub fn start_fresh_set(&mut self) {
        self.comparison.new_capture();
    }

    // ── read model ─────────────────────────────────────────────────────────────

    /// Roll-collection progress on the current set, including the roll counts at
    /// which the strict and full entropy sets complete.
    #[must_use]
    pub fn roll_progress(&self) -> RollProgress {
        let target = self.comparison.target();
        RollProgress {
            recorded: self.comparison.current_rolls(),
            latest_face: self.comparison.current_latest_face(),
            strict_target: target.strict_roll_count(),
            full_target: ConversionProtocol::WordExactV1.minimum_observations(target),
        }
    }

    /// The comparison report for the entropy set at `index`.
    #[must_use]
    pub fn comparison_at(&self, index: usize) -> GroupReport {
        self.comparison.report_at(index)
    }

    /// The entropy target shared by every capture in this session — used to
    /// render target-specific protocol explanations.
    #[must_use]
    pub fn target(&self) -> EntropyTarget {
        self.comparison.target()
    }

    /// How many accepted seeds capture `index` produced across its entropy sets.
    #[must_use]
    pub fn accepted_count(&self, index: usize) -> usize {
        self.comparison
            .report_at(index)
            .sets()
            .iter()
            .flat_map(|set| set.protocols.iter())
            .filter(|(_, status)| status.calculation().is_some())
            .count()
    }

    /// Every accepted seed in capture `index`, flattened across its entropy sets
    /// in display order and projected to derivation evidence. Each projection
    /// zeroizes its secret material on drop.
    #[must_use]
    pub fn derivations(&self, index: usize) -> Vec<GroupDerivation> {
        self.comparison
            .report_at(index)
            .sets()
            .iter()
            .flat_map(|set| {
                set.protocols
                    .iter()
                    .filter_map(|(protocol, status)| {
                        status.calculation().map(|calculation| GroupDerivation {
                            set_rolls: set.rolls,
                            protocol: *protocol,
                            projection: DerivationProjection::build(calculation),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// How many entropy sets have been rolled this session.
    #[must_use]
    pub fn set_count(&self) -> usize {
        self.comparison.capture_count()
    }
}
