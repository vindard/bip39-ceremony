mod preimage;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::RollSequence,
    protocol::{ConversionProtocol, ProtocolError, hash_entropy, require_complete_capture},
};

pub(crate) use preimage::{ascii_rolls, distribution_is_rejected};

pub(crate) enum ColdcardOutcome {
    Accepted(Entropy),
    DistributionRejected,
}

pub(crate) fn coldcard_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<ColdcardOutcome, ProtocolError> {
    require_complete_capture(ConversionProtocol::ColdcardV1, target, rolls)?;
    if distribution_is_rejected(rolls) {
        return Ok(ColdcardOutcome::DistributionRejected);
    }
    let ascii = ascii_rolls(rolls);
    Ok(ColdcardOutcome::Accepted(hash_entropy(
        target,
        &[ascii.as_slice()],
    )))
}
