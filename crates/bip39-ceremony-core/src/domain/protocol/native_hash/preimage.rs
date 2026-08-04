use crate::domain::bip39::EntropyTarget;

/// Canonical non-secret header for native hash conditioning.
#[must_use]
pub fn header(target: EntropyTarget) -> Vec<u8> {
    const DOMAIN: &[u8] = b"bip39-ceremony/native-hash/v1\0";
    let (target_bits, count) = match target {
        EntropyTarget::Words12 => ([0, 128], [0, 50]),
        EntropyTarget::Words24 => ([1, 0], [0, 100]),
    };

    let mut header = Vec::with_capacity(DOMAIN.len() + 4);
    header.extend_from_slice(DOMAIN);
    header.extend_from_slice(&target_bits);
    header.extend_from_slice(&count);
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_binds_version_target_and_count() {
        assert_eq!(
            header(EntropyTarget::Words12),
            b"bip39-ceremony/native-hash/v1\0\0\x80\0\x32".to_vec()
        );
        assert_ne!(
            header(EntropyTarget::Words12),
            header(EntropyTarget::Words24)
        );
    }
}
