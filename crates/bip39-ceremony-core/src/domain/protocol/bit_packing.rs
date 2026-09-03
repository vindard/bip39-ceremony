pub(crate) fn append_bits(bytes: &mut [u8], offset: &mut usize, value: u16, bits: usize) {
    for shift in (0..bits).rev() {
        let bit = (value >> shift).to_le_bytes()[0] & 1;
        bytes[*offset / 8] |= bit << (7 - *offset % 8);
        *offset += 1;
    }
}
