use bitcoin_hashes::{Hash, HashEngine, sha256};

use crate::application::ports::Sha256;

pub struct BitcoinSha256;

impl Sha256 for BitcoinSha256 {
    fn hash(&self, chunks: &[&[u8]]) -> [u8; 32] {
        let mut engine = sha256::Hash::engine();
        for chunk in chunks {
            engine.input(chunk);
        }
        sha256::Hash::from_engine(engine).to_byte_array()
    }
}
