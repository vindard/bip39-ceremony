use crate::domain::{
    bip39::{Entropy, EntropyTarget},
    coin::FlipSequence,
    protocol::{ConversionProtocol, ProtocolError, hash_entropy},
};

pub(crate) fn seedsigner_coin_entropy(
    target: EntropyTarget,
    flips: &FlipSequence,
) -> Result<Entropy, ProtocolError> {
    let protocol = ConversionProtocol::SeedSignerCoinsV1;
    if !protocol.assess_coin_capture(target, flips).is_complete() {
        return Err(ProtocolError::WrongObservationCount {
            expected: protocol.minimum_observations(target),
            actual: flips.len(),
        });
    }
    let ascii = flips.ascii_bytes();
    Ok(hash_entropy(target, &[ascii.as_slice()]))
}
