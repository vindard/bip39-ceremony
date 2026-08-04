use crate::domain::{
    bip39::EntropyTarget,
    bitbox::BitBoxCapture,
    coin::FlipSequence,
    d20::D20RollSequence,
    dice::RollSequence,
    jade::{JadeCapture, JadeObservation},
    protocol::{
        BitBoxProgress, BitBoxStage, ConversionProtocol, JadeDieKind, JadeProgress, WordExactParse,
        WordExactProgress, bitbox_progress, jade_expected_die, jade_progress, parse_word_exact,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureProgress {
    Fixed {
        recorded: usize,
        required: usize,
    },
    OpenEnded {
        recorded: usize,
        minimum: usize,
    },
    Jade(JadeProgress),
    BitBox(BitBoxProgress),
    WordExact(WordExactProgress),
    WordExactComplete {
        accepted_words: usize,
        required_words: usize,
        rejected_candidates: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureAssessment {
    Collecting(CaptureProgress),
    Complete {
        progress: CaptureProgress,
        accepts_optional_rolls: bool,
    },
    Invalid(CaptureProgress),
}

impl CaptureAssessment {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete { .. })
    }

    #[must_use]
    pub const fn can_record_more(self) -> bool {
        matches!(
            self,
            Self::Collecting(_)
                | Self::Complete {
                    accepts_optional_rolls: true,
                    ..
                }
        )
    }

    #[must_use]
    pub const fn accepts_optional_rolls(self) -> bool {
        matches!(
            self,
            Self::Complete {
                accepts_optional_rolls: true,
                ..
            }
        )
    }

    #[must_use]
    pub const fn progress(self) -> CaptureProgress {
        match self {
            Self::Collecting(progress)
            | Self::Complete { progress, .. }
            | Self::Invalid(progress) => progress,
        }
    }
}

impl ConversionProtocol {
    #[must_use]
    pub fn assess_capture(self, target: EntropyTarget, rolls: &RollSequence) -> CaptureAssessment {
        let recorded = rolls.len();
        if !self.supports_target(target) {
            return CaptureAssessment::Invalid(CaptureProgress::Fixed {
                recorded,
                required: self.minimum_observations(target),
            });
        }
        match self {
            Self::WordExactV1 => match parse_word_exact(target, rolls) {
                WordExactParse::Collecting(progress) => {
                    CaptureAssessment::Collecting(CaptureProgress::WordExact(progress))
                }
                WordExactParse::Complete(completed) => {
                    let progress = CaptureProgress::WordExactComplete {
                        accepted_words: completed.accepted_words(),
                        required_words: completed.required_words(),
                        rejected_candidates: completed.rejected_candidates(),
                    };
                    if completed.consumed_rolls() == recorded {
                        CaptureAssessment::Complete {
                            progress,
                            accepts_optional_rolls: false,
                        }
                    } else {
                        CaptureAssessment::Invalid(progress)
                    }
                }
            },
            Self::ColdcardV1 | Self::KeystoneLegacyV1 => {
                let minimum = self.minimum_observations(target);
                let progress = CaptureProgress::OpenEnded { recorded, minimum };
                if recorded >= minimum {
                    CaptureAssessment::Complete {
                        progress,
                        accepts_optional_rolls: true,
                    }
                } else {
                    CaptureAssessment::Collecting(progress)
                }
            }
            Self::JadeDirectV1
            | Self::BitBox02DirectV1
            | Self::KruxD20V1
            | Self::SeedSignerCoinsV1 => CaptureAssessment::Invalid(CaptureProgress::Fixed {
                recorded,
                required: self.minimum_observations(target),
            }),
            Self::ExactV1 | Self::NativeHashV1 => {
                let required = self.minimum_observations(target);
                let progress = CaptureProgress::Fixed { recorded, required };
                match recorded.cmp(&required) {
                    core::cmp::Ordering::Less => CaptureAssessment::Collecting(progress),
                    core::cmp::Ordering::Equal => CaptureAssessment::Complete {
                        progress,
                        accepts_optional_rolls: false,
                    },
                    core::cmp::Ordering::Greater => CaptureAssessment::Invalid(progress),
                }
            }
        }
    }

    #[must_use]
    pub fn assess_d20_capture(
        self,
        target: EntropyTarget,
        rolls: &D20RollSequence,
    ) -> CaptureAssessment {
        let recorded = rolls.len();
        let minimum = self.minimum_observations(target);
        let progress = CaptureProgress::OpenEnded { recorded, minimum };
        if self != Self::KruxD20V1 {
            return CaptureAssessment::Invalid(progress);
        }
        if recorded >= minimum {
            CaptureAssessment::Complete {
                progress,
                accepts_optional_rolls: true,
            }
        } else {
            CaptureAssessment::Collecting(progress)
        }
    }

    #[must_use]
    pub fn assess_jade_capture(
        self,
        target: EntropyTarget,
        capture: &JadeCapture,
    ) -> CaptureAssessment {
        let progress = CaptureProgress::Jade(jade_progress(target, capture));
        if self != Self::JadeDirectV1 {
            return CaptureAssessment::Invalid(progress);
        }
        let kinds_valid = capture
            .observations()
            .iter()
            .enumerate()
            .all(|(offset, observation)| {
                let kind = match observation {
                    JadeObservation::D16(_) => JadeDieKind::D16,
                    JadeObservation::D8(_) => JadeDieKind::D8,
                };
                jade_expected_die(target, offset) == Some(kind)
            });
        if !kinds_valid {
            return CaptureAssessment::Invalid(progress);
        }
        match capture.len().cmp(&self.minimum_observations(target)) {
            core::cmp::Ordering::Less => CaptureAssessment::Collecting(progress),
            core::cmp::Ordering::Equal => CaptureAssessment::Complete {
                progress,
                accepts_optional_rolls: false,
            },
            core::cmp::Ordering::Greater => CaptureAssessment::Invalid(progress),
        }
    }

    #[must_use]
    pub fn assess_bitbox_capture(
        self,
        target: EntropyTarget,
        capture: &BitBoxCapture,
    ) -> CaptureAssessment {
        let bitbox = bitbox_progress(target, capture);
        let progress = CaptureProgress::BitBox(bitbox);
        if self != Self::BitBox02DirectV1 || !bitbox.is_valid() {
            return CaptureAssessment::Invalid(progress);
        }
        if bitbox.stage() == BitBoxStage::Complete {
            CaptureAssessment::Complete {
                progress,
                accepts_optional_rolls: false,
            }
        } else {
            CaptureAssessment::Collecting(progress)
        }
    }

    #[must_use]
    pub fn assess_coin_capture(
        self,
        target: EntropyTarget,
        flips: &FlipSequence,
    ) -> CaptureAssessment {
        let recorded = flips.len();
        let required = self.minimum_observations(target);
        let progress = CaptureProgress::Fixed { recorded, required };
        if self != Self::SeedSignerCoinsV1 {
            return CaptureAssessment::Invalid(progress);
        }
        match recorded.cmp(&required) {
            core::cmp::Ordering::Less => CaptureAssessment::Collecting(progress),
            core::cmp::Ordering::Equal => CaptureAssessment::Complete {
                progress,
                accepts_optional_rolls: false,
            },
            core::cmp::Ordering::Greater => CaptureAssessment::Invalid(progress),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dice::DieFace;

    fn repeated_rolls(face: u8, count: usize) -> RollSequence {
        let mut rolls = RollSequence::new();
        for _ in 0..count {
            rolls.push(DieFace::new(face).unwrap());
        }
        rolls
    }

    #[test]
    fn capture_assessment_encapsulates_fixed_optional_and_localized_policies() {
        let fixed = ConversionProtocol::ExactV1
            .assess_capture(EntropyTarget::Words12, &repeated_rolls(1, 50));
        assert!(fixed.is_complete());
        assert!(!fixed.can_record_more());

        let optional = ConversionProtocol::ColdcardV1
            .assess_capture(EntropyTarget::Words12, &repeated_rolls(1, 50));
        assert!(optional.is_complete());
        assert!(optional.can_record_more());

        let keystone = ConversionProtocol::KeystoneLegacyV1
            .assess_capture(EntropyTarget::Words24, &repeated_rolls(1, 99));
        assert!(keystone.is_complete());
        assert!(keystone.can_record_more());

        let localized = ConversionProtocol::WordExactV1
            .assess_capture(EntropyTarget::Words12, &repeated_rolls(1, 70));
        assert!(localized.is_complete());
        assert!(!localized.can_record_more());
    }

    #[test]
    fn jade_capture_enforces_kind_order_and_target_count() {
        use crate::domain::jade::{D8Face, D16Face, JadeCapture};

        let mut capture = JadeCapture::new();
        capture.push_d16(D16Face::new(1).unwrap());
        assert!(matches!(
            ConversionProtocol::JadeDirectV1.assess_jade_capture(EntropyTarget::Words12, &capture),
            CaptureAssessment::Collecting(_)
        ));
        capture.push_d16(D16Face::new(1).unwrap());
        capture.push_d16(D16Face::new(1).unwrap());
        assert!(matches!(
            ConversionProtocol::JadeDirectV1.assess_jade_capture(EntropyTarget::Words12, &capture),
            CaptureAssessment::Invalid(_)
        ));

        let mut complete = JadeCapture::new();
        for offset in 0..35 {
            match jade_expected_die(EntropyTarget::Words12, offset).unwrap() {
                JadeDieKind::D16 => complete.push_d16(D16Face::new(1).unwrap()),
                JadeDieKind::D8 => complete.push_d8(D8Face::new(1).unwrap()),
            }
        }
        assert!(
            ConversionProtocol::JadeDirectV1
                .assess_jade_capture(EntropyTarget::Words12, &complete)
                .is_complete()
        );
        assert_eq!(
            complete.observations().last(),
            Some(&JadeObservation::D8(D8Face::new(1).unwrap()))
        );
    }

    #[test]
    fn bitbox_capture_uses_parser_completion_after_local_rejections() {
        use crate::domain::{bitbox::BitBoxCapture, coin::CoinFlip, dice::DieFace};

        let mut capture = BitBoxCapture::new();
        capture.push_d6(DieFace::new(6).unwrap());
        for _ in 0..11 {
            for _ in 0..5 {
                capture.push_d6(DieFace::new(1).unwrap());
            }
            capture.push_coin(CoinFlip::new(1).unwrap());
        }
        for _ in 0..7 {
            capture.push_coin(CoinFlip::new(1).unwrap());
        }
        let assessment = ConversionProtocol::BitBox02DirectV1
            .assess_bitbox_capture(EntropyTarget::Words12, &capture);
        assert!(assessment.is_complete());
        let CaptureProgress::BitBox(progress) = assessment.progress() else {
            panic!("BitBox progress expected");
        };
        assert_eq!(progress.recorded(), 74);
        assert_eq!(progress.rejected_faces(), 1);
    }

    #[test]
    fn krux_d20_capture_accepts_rolls_beyond_its_minimum() {
        use crate::domain::d20::{D20Face, D20RollSequence};

        let mut rolls = D20RollSequence::new();
        for _ in 0..31 {
            rolls.push(D20Face::new(20).unwrap());
        }
        let assessment =
            ConversionProtocol::KruxD20V1.assess_d20_capture(EntropyTarget::Words12, &rolls);
        assert!(assessment.is_complete());
        assert!(assessment.accepts_optional_rolls());
    }

    #[test]
    fn seedsigner_coin_capture_is_fixed_to_target_entropy_width() {
        let mut flips = FlipSequence::new();
        for _ in 0..128 {
            flips.push(crate::domain::coin::CoinFlip::new(0).unwrap());
        }
        let assessment = ConversionProtocol::SeedSignerCoinsV1
            .assess_coin_capture(EntropyTarget::Words12, &flips);
        assert!(assessment.is_complete());
        assert!(!assessment.can_record_more());
    }

    #[test]
    fn unsupported_target_capture_is_invalid() {
        assert!(matches!(
            ConversionProtocol::KeystoneLegacyV1
                .assess_capture(EntropyTarget::Words12, &repeated_rolls(1, 50)),
            CaptureAssessment::Invalid(_)
        ));
    }

    #[test]
    fn fixed_capture_overrun_is_invalid() {
        let overrun = ConversionProtocol::NativeHashV1
            .assess_capture(EntropyTarget::Words12, &repeated_rolls(1, 51));

        assert!(matches!(overrun, CaptureAssessment::Invalid(_)));
        assert!(!overrun.is_complete());
        assert!(!overrun.can_record_more());
    }
}
