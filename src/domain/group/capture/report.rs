//! Value objects a group evaluation produces: a protocol's status against a set
//! of rolls, and the entropy sets (roll-count checkpoints) that group them.

use bip39_ceremony_core::Calculation;

use crate::domain::protocol::ConversionProtocol;

/// A protocol's status against a set of rolls.
///
/// A protocol *completes* at a set length only when it consumes exactly those
/// rolls with every roll contributing — that is `Accepted`, or an attempt that
/// was content-rejected. `Incomplete` (needs more), `InvalidSurplus` (finished
/// before the end), and `Unsupported` mean it does not belong to that set.
pub enum GroupStatus {
    /// Consumed all the set's rolls, none rejected, and produced a mnemonic.
    Accepted(Calculation),
    /// Not enough rolls yet to complete; more rolls could still use them all.
    Incomplete { have: usize, need: usize },
    /// `ExactV1`: the encoded value is out of the uniform range.
    InvalidOutOfRange,
    /// `ColdcardV1`: a single face exceeds 30% of the rolls.
    InvalidDistribution,
    /// `WordExactV1`: a candidate was rejected, so part of the entropy is
    /// discarded — it can never use the whole set.
    InvalidRejected,
    /// The protocol finished before the end of the set, leaving rolls unused.
    InvalidSurplus { used: usize, provided: usize },
    /// The protocol does not support this entropy target.
    Unsupported,
}

impl GroupStatus {
    #[must_use]
    pub fn calculation(&self) -> Option<&Calculation> {
        match self {
            Self::Accepted(c) => Some(c),
            _ => None,
        }
    }

    /// Whether the protocol completes at this set's length — i.e. it belongs in
    /// the set (accepted, or attempted and content-rejected).
    #[must_use]
    pub fn completes(&self) -> bool {
        matches!(
            self,
            Self::Accepted(_)
                | Self::InvalidOutOfRange
                | Self::InvalidDistribution
                | Self::InvalidRejected
        )
    }

    /// Whether this completing status is a content rejection rather than a
    /// clean acceptance.
    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(
            self,
            Self::InvalidOutOfRange | Self::InvalidDistribution | Self::InvalidRejected
        )
    }
}

/// One roll-count checkpoint: the protocols that complete using exactly this
/// many rolls, each with its status.
pub struct EntropySet {
    pub rolls: usize,
    pub protocols: Vec<(ConversionProtocol, GroupStatus)>,
}

/// The entropy sets for a capture, largest (current) first.
pub struct GroupReport {
    sets: Vec<EntropySet>,
}

impl GroupReport {
    #[must_use]
    pub(crate) fn new(sets: Vec<EntropySet>) -> Self {
        Self { sets }
    }

    #[must_use]
    pub fn sets(&self) -> &[EntropySet] {
        &self.sets
    }
}
