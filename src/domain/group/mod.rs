//! Group-comparison subdomain.
//!
//! [`GroupComparison`] is the domain service: it owns the [`GroupCapture`]
//! entities rolled this session and is the gateway to operating on them —
//! recording rolls, starting a fresh set, and evaluating a capture into the
//! entropy sets a comparison shows. Rolls always extend the current (newest)
//! capture; earlier captures are frozen for comparison. Which capture the UI is
//! *browsing* is view state and lives in the UI, not here.
//!
//! Evaluation groups protocols by **entropy set** — a roll-count checkpoint. For
//! a capture of N rolls the sets are the current N plus each length where an
//! exact-count protocol completes (the strict count and word-exact's minimum),
//! largest first. Each set lists the protocols that consume *exactly* those
//! rolls, per the entity invariant [`GroupCapture::status_under`].

mod capture;

pub use capture::{EntropySet, GroupCapture, GroupReport, GroupStatus};

use bip39_ceremony_core::bitpack_progress;

use crate::domain::{bip39::EntropyTarget, protocol::ConversionProtocol};

use ConversionProtocol::{
    BitcoinLibBase6V1, BlueWalletBitPackV1, ColdcardV1, ExactV1, KeystoneLegacyV1, WordExactV1,
};

/// The dice-based wallet protocols the comparison covers, in display order.
pub const GROUP_PROTOCOLS: [ConversionProtocol; 6] = [
    ExactV1,
    ColdcardV1,
    KeystoneLegacyV1,
    WordExactV1,
    BitcoinLibBase6V1,
    BlueWalletBitPackV1,
];

/// The set lengths for a capture of `n` rolls, largest first: the current `n`
/// plus each checkpoint (the strict count, word-exact's minimum, and the
/// unhashed base-6 reading's count) at or below `n`. Open-ended protocols do not
/// add checkpoints; they populate every set at or above their own minimum.
///
/// Known limitation: word-exact only checkpoints at its *minimum*. If mid-tape
/// rejections push its actual completion past that minimum, no checkpoint lands
/// on the longer length, so that (valid) word-exact seed is never surfaced as an
/// `Accepted` set — it reads `Incomplete` at the minimum and `InvalidSurplus` at
/// `n`. Acceptable because the comparison is about clean, rejection-free tapes.
fn checkpoint_lengths(capture: &GroupCapture) -> Vec<usize> {
    let target = capture.target();
    let n = capture.len();
    let strict = target.strict_roll_count();
    let word_exact = WordExactV1.minimum_observations(target);
    let base6 = BitcoinLibBase6V1.minimum_observations(target);
    let mut lengths: Vec<usize> = [strict, word_exact, base6, bitpack_length(capture), n]
        .into_iter()
        .filter(|&length| length <= n)
        .collect();
    lengths.sort_unstable();
    lengths.dedup();
    lengths.reverse();
    lengths
}

fn bitpack_length(capture: &GroupCapture) -> usize {
    let progress = bitpack_progress(capture.target(), capture.tape());
    if progress.is_filled() {
        progress.consumed()
    } else {
        capture.len()
    }
}

/// Evaluate a capture into its entropy sets. Each set lists the protocols that
/// complete using exactly its rolls (accepted, or attempted and content-
/// rejected); protocols that would leave rolls unused, or need more, are omitted
/// from that set.
fn evaluate(capture: &GroupCapture) -> GroupReport {
    let sets = checkpoint_lengths(capture)
        .into_iter()
        .map(|length| {
            let prefix = capture.prefix(length);
            let protocols = GROUP_PROTOCOLS
                .into_iter()
                .map(|protocol| (protocol, prefix.status_under(protocol)))
                .filter(|(_, status)| status.completes())
                .collect();
            EntropySet {
                rolls: length,
                protocols,
            }
        })
        .collect();
    GroupReport::new(sets)
}

/// The domain service: owns the captures rolled this session and is the gateway
/// to operating on them. Writes always target the current (newest) capture.
pub struct GroupComparison {
    /// In-memory repository of the capture entities rolled this session:
    /// append-only, so a capture's ordinal position is its stable identity.
    captures: Vec<GroupCapture>,
}

impl GroupComparison {
    /// Starts a comparison with a single empty capture for `target`.
    #[must_use]
    pub fn new(target: EntropyTarget) -> Self {
        Self {
            captures: vec![GroupCapture::new(target)],
        }
    }

    fn current(&self) -> &GroupCapture {
        self.captures
            .last()
            .expect("a comparison always holds at least one capture")
    }

    fn current_mut(&mut self) -> &mut GroupCapture {
        self.captures
            .last_mut()
            .expect("a comparison always holds at least one capture")
    }

    // ── entity operations (always on the current, newest capture) ─────────────

    /// Records one physical roll on the current capture.
    pub fn record_roll(&mut self, face: crate::domain::dice::DieFace) {
        self.current_mut().push(face);
    }

    /// Undoes the latest roll on the current capture.
    pub fn undo_roll(&mut self) {
        self.current_mut().remove_last();
    }

    /// Starts a fresh entropy set and makes it current — the only way to clear
    /// an invalid result. Earlier captures stay for comparison.
    pub fn new_capture(&mut self) {
        let target = self.current().target();
        self.captures.push(GroupCapture::new(target));
    }

    // ── reads ─────────────────────────────────────────────────────────────────

    /// The entropy target — the same for every capture in the comparison.
    #[must_use]
    pub fn target(&self) -> EntropyTarget {
        self.current().target()
    }

    #[must_use]
    pub fn capture_count(&self) -> usize {
        self.captures.len()
    }

    /// Rolls recorded on the current capture.
    #[must_use]
    pub fn current_rolls(&self) -> usize {
        self.current().len()
    }

    /// The face of the most recent roll on the current capture, if any.
    #[must_use]
    pub fn current_latest_face(&self) -> Option<u8> {
        self.current().tape().faces().last().map(|face| face.get())
    }

    /// Evaluates the capture at `index` into its entropy sets.
    #[must_use]
    pub fn report_at(&self, index: usize) -> GroupReport {
        evaluate(&self.captures[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    fn fair(target: EntropyTarget, n: usize) -> GroupCapture {
        let mut capture = GroupCapture::new(target);
        for i in 0..n {
            let face = u8::try_from((i % 6) + 1).expect("1..=6 fits u8");
            capture.push(DieFace::new(face).expect("valid face"));
        }
        capture
    }

    fn protocols_in(report: &GroupReport, rolls: usize) -> Vec<ConversionProtocol> {
        report
            .sets()
            .iter()
            .find(|set| set.rolls == rolls)
            .map(|set| set.protocols.iter().map(|(p, _)| *p).collect())
            .unwrap_or_default()
    }

    #[test]
    fn checkpoints_are_current_plus_completion_lengths() {
        assert_eq!(
            checkpoint_lengths(&fair(EntropyTarget::Words24, 166)),
            [166, 153, 140, 100, 99]
        );
        assert_eq!(
            checkpoint_lengths(&fair(EntropyTarget::Words24, 140)),
            [140, 100, 99]
        );
        assert_eq!(
            checkpoint_lengths(&fair(EntropyTarget::Words24, 105)),
            [105, 100, 99]
        );
        assert_eq!(
            checkpoint_lengths(&fair(EntropyTarget::Words24, 100)),
            [100, 99]
        );
        assert_eq!(checkpoint_lengths(&fair(EntropyTarget::Words24, 99)), [99]);
        assert_eq!(checkpoint_lengths(&fair(EntropyTarget::Words24, 90)), [90]);
        assert_eq!(checkpoint_lengths(&fair(EntropyTarget::Words12, 50)), [50]);
    }

    #[test]
    fn the_bit_packing_checkpoint_depends_on_the_faces_rolled() {
        let uniform = |target: EntropyTarget, face: u8, n: usize| {
            let mut capture = GroupCapture::new(target);
            for _ in 0..n {
                capture.push(DieFace::new(face).expect("valid face"));
            }
            capture
        };
        assert_eq!(bitpack_length(&uniform(EntropyTarget::Words12, 1, 100)), 64);
        assert_eq!(
            bitpack_length(&uniform(EntropyTarget::Words12, 5, 100)),
            100
        );
        assert_eq!(
            bitpack_length(&uniform(EntropyTarget::Words12, 5, 128)),
            128
        );
    }

    #[test]
    fn one_hundred_sixty_six_yields_five_sets() {
        let report = evaluate(&fair(EntropyTarget::Words24, 166));
        let set_lengths: Vec<usize> = report.sets().iter().map(|s| s.rolls).collect();
        assert_eq!(set_lengths, [166, 153, 140, 100, 99]);

        assert_eq!(protocols_in(&report, 166), [ColdcardV1, KeystoneLegacyV1]);
        assert_eq!(
            protocols_in(&report, 153),
            [ColdcardV1, KeystoneLegacyV1, BlueWalletBitPackV1]
        );
        assert_eq!(
            protocols_in(&report, 140),
            [ColdcardV1, KeystoneLegacyV1, WordExactV1]
        );
        assert_eq!(
            protocols_in(&report, 100),
            [ExactV1, ColdcardV1, KeystoneLegacyV1]
        );
        // 99 is the count the unhashed reading needs and no other fixed-count
        // protocol accepts.
        assert_eq!(
            protocols_in(&report, 99),
            [ColdcardV1, KeystoneLegacyV1, BitcoinLibBase6V1]
        );
    }

    #[test]
    fn hundred_and_hundred_five_differ_only_by_the_open_ended_set() {
        // At 100, the fixed-count protocols are in the 100 set.
        let at100 = evaluate(&fair(EntropyTarget::Words24, 100));
        assert_eq!(
            protocols_in(&at100, 100),
            [ExactV1, ColdcardV1, KeystoneLegacyV1]
        );

        // At 105, the fixed-count protocols stay in the 100 set (not the 105
        // set) — rolling past 100 never quotes them against 105.
        let at105 = evaluate(&fair(EntropyTarget::Words24, 105));
        assert_eq!(protocols_in(&at105, 105), [ColdcardV1, KeystoneLegacyV1]);
        assert_eq!(
            protocols_in(&at105, 100),
            [ExactV1, ColdcardV1, KeystoneLegacyV1]
        );
    }

    #[test]
    fn coldcard_seed_differs_across_sets() {
        let report = evaluate(&fair(EntropyTarget::Words24, 166));
        let coldcard_seed = |rolls: usize| {
            report
                .sets()
                .iter()
                .find(|s| s.rolls == rolls)
                .unwrap()
                .protocols
                .iter()
                .find(|(p, _)| *p == ColdcardV1)
                .unwrap()
                .1
                .calculation()
                .unwrap()
                .mnemonic()
                .words()
                .join(" ")
        };
        assert_ne!(coldcard_seed(100), coldcard_seed(140));
        assert_ne!(coldcard_seed(140), coldcard_seed(166));
    }

    /// The point of putting the unhashed reading in the comparison: at 99 rolls
    /// it and COLDCARD both complete on one tape and disagree on the seed.
    #[test]
    fn the_unhashed_reading_and_coldcard_disagree_inside_one_set() {
        let report = evaluate(&fair(EntropyTarget::Words24, 99));
        let set = report.sets().iter().find(|s| s.rolls == 99).unwrap();
        let seed = |protocol: ConversionProtocol| {
            set.protocols
                .iter()
                .find(|(p, _)| *p == protocol)
                .unwrap()
                .1
                .calculation()
                .map(|c| c.mnemonic().words().join(" "))
        };

        let base6 = seed(BitcoinLibBase6V1).expect("this tape lands on the target width");
        let coldcard = seed(ColdcardV1).expect("hashing always produces a seed");
        assert_ne!(base6, coldcard);
    }

    #[test]
    fn below_the_first_checkpoint_the_only_set_is_empty() {
        let report = evaluate(&fair(EntropyTarget::Words24, 40));
        assert_eq!(report.sets().len(), 1);
        assert_eq!(report.sets()[0].rolls, 40);
        assert!(report.sets()[0].protocols.is_empty());
    }

    fn roll(comparison: &mut GroupComparison, count: usize) {
        for i in 0..count {
            let face = u8::try_from((i % 6) + 1).expect("1..=6 fits u8");
            comparison.record_roll(DieFace::new(face).expect("valid face"));
        }
    }

    #[test]
    fn writes_always_target_the_current_capture() {
        let mut comparison = GroupComparison::new(EntropyTarget::Words24);
        roll(&mut comparison, 5);
        assert_eq!(comparison.current_rolls(), 5);
        assert_eq!(comparison.current_latest_face(), Some(5));
        comparison.undo_roll();
        assert_eq!(comparison.current_rolls(), 4);
    }

    #[test]
    fn new_capture_freezes_prior_and_starts_empty_current() {
        let mut comparison = GroupComparison::new(EntropyTarget::Words24);
        roll(&mut comparison, 10);

        comparison.new_capture();

        assert_eq!(comparison.capture_count(), 2);
        assert_eq!(comparison.current_rolls(), 0);
        roll(&mut comparison, 3);
        assert_eq!(comparison.current_rolls(), 3);
        // The prior capture is untouched: its 100 set is still absent (10 rolls).
        assert!(comparison.report_at(0).sets()[0].protocols.is_empty());
    }
}
