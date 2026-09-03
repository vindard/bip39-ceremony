mod document;
mod evidence;
mod guidance;
mod physical;
mod protocol;
mod readiness;
mod recovery;
mod safety;
mod setup;
mod source;

pub use document::{ContentBlock, Document};
pub use evidence::{reproduction_receipt, trust_boundary};
pub use guidance::{assurance_line, phase_guidance};
pub use physical::physical_entropy_guidance;
pub use protocol::{protocol_explanation, protocol_menu_explanation};
pub use readiness::readiness_document;
pub use recovery::{
    attempt_rejection, concealed_generation, derivation_guidance, finish_confirmation,
    transcription_instructions, verified_recovery,
};
pub use safety::{SafetyContent, safety_content};
pub use setup::{
    ChoiceContent, ProtocolMenuChoice, capture_protocol_context, protocol_choices, target_choices,
};
pub(crate) use source::{
    BIP39_CHECKSUM, BIP39_VERSION, BITCOIN_HASHES_CHECKSUM, BITCOIN_HASHES_VERSION, SourceScope,
    protocol_source_files,
};
