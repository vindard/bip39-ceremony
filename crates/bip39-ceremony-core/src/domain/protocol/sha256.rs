use bitcoin_hashes::{Hash, HashEngine, sha256};
use zeroize::Zeroize;

use crate::domain::bip39::{Entropy, EntropyTarget};

pub(crate) fn sha256_prefix_entropy(target: EntropyTarget, chunks: &[&[u8]]) -> Entropy {
    let mut digest = sha256_digest(chunks);
    let entropy = Entropy::from_protocol_bytes(target, digest[..target.entropy_bytes()].to_vec());
    digest.zeroize();
    entropy
}

pub(crate) fn sha256_digest(chunks: &[&[u8]]) -> [u8; 32] {
    let mut engine = sha256::Hash::engine();
    for chunk in chunks {
        engine.input(chunk);
    }
    sha256::Hash::from_engine(engine).to_byte_array()
}
