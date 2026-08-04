mod document;
mod evidence;
mod guidance;
mod protocol;
mod readiness;
mod recovery;
mod safety;
mod setup;

pub use document::{ContentBlock, Document};
pub use evidence::{reproduction_receipt, trust_boundary};
pub use guidance::{assurance_line, phase_guidance};
pub use protocol::{protocol_explanation, protocol_menu_explanation};
pub use readiness::readiness_document;
pub use recovery::{
    concealed_generation, derivation_guidance, exact_rejection, finish_confirmation,
    transcription_instructions, verified_recovery,
};
pub use safety::{SafetyContent, safety_content};
pub use setup::{
    ChoiceContent, ProtocolMenuChoice, capture_protocol_context, protocol_choices, target_choices,
};
