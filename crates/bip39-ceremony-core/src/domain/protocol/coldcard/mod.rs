mod preimage;

use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    dice::RollSequence,
    protocol::{
        ConversionProtocol, ProtocolError, require_complete_dice_capture, sha256_prefix_entropy,
    },
};

pub(crate) use preimage::{ascii_rolls, distribution_is_rejected};

pub(crate) enum ColdcardEntropyOutcome {
    Accepted(Entropy),
    DistributionRejected,
}

pub(crate) fn coldcard_entropy(
    target: EntropyTarget,
    rolls: &RollSequence,
) -> Result<ColdcardEntropyOutcome, ProtocolError> {
    require_complete_dice_capture(ConversionProtocol::ColdcardV1, target, rolls)?;
    if distribution_is_rejected(rolls) {
        return Ok(ColdcardEntropyOutcome::DistributionRejected);
    }
    Ok(ColdcardEntropyOutcome::Accepted(calculate_entropy(
        target, rolls,
    )))
}

fn calculate_entropy(target: EntropyTarget, rolls: &RollSequence) -> Entropy {
    let ascii = ascii_rolls(rolls);
    sha256_prefix_entropy(target, &[ascii.as_slice()])
}
