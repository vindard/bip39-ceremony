use bip39::{Language, Mnemonic};

use super::{Bip39Error, Entropy, EntropyTarget, mnemonic::Bip39Encoding};

#[test]
fn mnemonic_encoding_rejects_a_different_round_trip_entropy() {
    let entropy = Entropy::new(EntropyTarget::Words12, vec![0; 16]).unwrap();
    let different = Mnemonic::from_entropy_in(Language::English, &[1; 16]).unwrap();

    assert!(matches!(
        Bip39Encoding::from_encoded(&entropy, &different),
        Err(Bip39Error::EncodingFailed)
    ));
}
