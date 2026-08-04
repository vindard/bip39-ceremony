use super::InspectionSnapshot;

#[derive(Debug, Eq, PartialEq)]
pub struct SnapshotComparison {
    pub phase_changed: bool,
    pub target_changed: bool,
    pub protocol_changed: bool,
    pub roll_count_delta: isize,
}

#[must_use]
pub fn compare(selected: &InspectionSnapshot, live: &InspectionSnapshot) -> SnapshotComparison {
    SnapshotComparison {
        phase_changed: selected.phase() != live.phase(),
        target_changed: selected.target() != live.target(),
        protocol_changed: selected.protocol() != live.protocol(),
        roll_count_delta: signed_delta(selected.roll_count(), live.roll_count()),
    }
}

fn signed_delta(earlier: usize, later: usize) -> isize {
    match later.cmp(&earlier) {
        core::cmp::Ordering::Less => -isize::try_from(earlier - later).unwrap_or(isize::MAX),
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => isize::try_from(later - earlier).unwrap_or(isize::MAX),
    }
}

#[cfg(test)]
mod tests {
    use super::{SnapshotComparison, compare};
    use crate::domain::inspection::InspectionSnapshot;

    use crate::domain::{
        bip39::EntropyTarget,
        ceremony::{Ceremony, Command},
        dice::DieFace,
        protocol::ConversionProtocol,
    };

    fn ceremony_with_roll() -> Ceremony {
        let mut ceremony = Ceremony::new();
        ceremony
            .handle(Command::SelectTarget(EntropyTarget::Words12))
            .unwrap();
        ceremony
            .handle(Command::SelectProtocol(ConversionProtocol::ExactV1))
            .unwrap();
        ceremony.handle(Command::AcknowledgeSafety).unwrap();
        ceremony
            .handle(Command::RecordRoll(DieFace::new(4).unwrap()))
            .unwrap();
        ceremony
    }

    #[test]
    fn describes_change_toward_live_state() {
        let ceremony = ceremony_with_roll();
        let historical = InspectionSnapshot::at(&ceremony, 2).unwrap();
        let live = InspectionSnapshot::at(&ceremony, 4).unwrap();

        assert_eq!(
            compare(&historical, &live),
            SnapshotComparison {
                phase_changed: true,
                target_changed: false,
                protocol_changed: false,
                roll_count_delta: 1,
            }
        );
    }
}
