use core::fmt;

use zeroize::Zeroize;

use crate::domain::ceremony::{Ceremony, Event};

/// One user-readable accepted fact in the ceremony timeline.
pub struct TimelineEntry {
    position: usize,
    description: String,
    secret_bearing: bool,
}

impl TimelineEntry {
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub const fn is_secret_bearing(&self) -> bool {
        self.secret_bearing
    }
}

impl Drop for TimelineEntry {
    fn drop(&mut self) {
        self.description.zeroize();
    }
}

impl fmt::Debug for TimelineEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TimelineEntry")
            .field("position", &self.position)
            .field("description", &"[REDACTED]")
            .field("secret_bearing", &self.secret_bearing)
            .finish()
    }
}

#[must_use]
pub fn timeline(ceremony: &Ceremony) -> Vec<TimelineEntry> {
    ceremony
        .events()
        .iter()
        .enumerate()
        .map(|(index, event)| timeline_entry(index + 1, event))
        .collect()
}

fn timeline_entry(position: usize, event: &Event) -> TimelineEntry {
    let (description, secret_bearing) = match event {
        Event::TargetSelected(target) => (format!("selected {} words", target.word_count()), false),
        Event::TargetSelectionReopened => {
            ("returned to mnemonic length selection".to_owned(), false)
        }
        Event::ProtocolSelected(protocol) => (format!("selected {}", protocol.id()), false),
        Event::ProtocolSelectionReopened => ("returned to protocol selection".to_owned(), false),
        Event::SafetyAcknowledged => ("acknowledged safety assumptions".to_owned(), false),
        Event::RollRecorded(_) => ("recorded secret roll".to_owned(), true),
        Event::FlipRecorded(_) => ("recorded secret coin flip".to_owned(), true),
        Event::RollUndone => ("undid the latest active roll".to_owned(), true),
        Event::FlipUndone => ("undid the latest active coin flip".to_owned(), true),
        Event::RollsConfirmed => ("confirmed roll transcription".to_owned(), true),
        Event::GenerationSucceeded => ("generated concealed result".to_owned(), true),
        Event::ExactAttemptRejected => ("exact conversion rejected attempt".to_owned(), true),
        Event::ExactAttemptRestarted => ("started a fresh exact attempt".to_owned(), true),
        Event::MnemonicRevealed => ("revealed mnemonic".to_owned(), true),
        Event::MnemonicBackupVerified => ("marked mnemonic transcription checked".to_owned(), true),
        Event::CeremonyCancelled => ("cancelled ceremony".to_owned(), true),
    };
    TimelineEntry {
        position,
        description,
        secret_bearing,
    }
}

#[cfg(test)]
mod tests {
    use super::timeline;

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
    fn explains_events_and_marks_secret_entries() {
        let ceremony = ceremony_with_roll();
        let entries = timeline(&ceremony);
        assert_eq!(entries[0].description(), "selected 12 words");
        assert!(!entries[0].is_secret_bearing());
        assert_eq!(entries[3].description(), "recorded secret roll");
        assert!(entries[3].is_secret_bearing());
        assert!(!format!("{:?}", entries[3]).contains("recorded roll 4"));
    }
}
