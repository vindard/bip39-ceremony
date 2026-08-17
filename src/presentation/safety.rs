use crate::application::SafetyAttestation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SafetyContent {
    label: &'static str,
    detail: &'static str,
}

impl SafetyContent {
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.label
    }
    #[must_use]
    pub const fn detail(self) -> &'static str {
        self.detail
    }
}

#[must_use]
pub const fn safety_content(item: SafetyAttestation) -> SafetyContent {
    match item {
        SafetyAttestation::DeviceIsolated => SafetyContent {
            label: "Device isolated",
            detail: "Disconnect network paths and avoid remote shells; this does not prove the operating system is clean.",
        },
        SafetyAttestation::RecordingDisabled => SafetyContent {
            label: "Recording disabled",
            detail: "Disable terminal or tmux logging, screen recording, screenshots, and crash capture where practical.",
        },
        SafetyAttestation::CamerasRemoved => SafetyContent {
            label: "Cameras and observers removed",
            detail: "Include phones, webcams, security cameras, remote sessions, people, and reflective surfaces.",
        },
        SafetyAttestation::PrivateTranscriptionReady => SafetyContent {
            label: "Private transcription ready",
            detail: "Prepare a deliberately chosen offline paper or metal transcription workflow before revealing words.",
        },
        SafetyAttestation::CopyChannelsAvoided => SafetyContent {
            label: "No clipboard, photo, or printer",
            detail: "Clipboard managers, cameras, and networked printers commonly create uncontrolled secret copies.",
        },
        SafetyAttestation::PhysicalRollPolicyFixed => SafetyContent {
            label: "Physical outcome policy fixed",
            detail: "Define invalid rolls or flips now; never replace a valid result because its outcome looks unusual.",
        },
        SafetyAttestation::ThrowMethodFixed => SafetyContent {
            label: "Throwing method fixed",
            detail: "Commit to a tumbling throw now — a closed box works well. A die set down gently keeps its starting face over half the time, and no check here can detect that.",
        },
        SafetyAttestation::DerivedValuesSecret => SafetyContent {
            label: "All derived values are secret",
            detail: "Treat rolls or flips, entropy, indices, words, and every derivation value as wallet-secret material.",
        },
    }
}
