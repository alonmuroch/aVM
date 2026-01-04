use crate::require;

// program/utils.rs
pub fn read_u32(bytes: &[u8]) -> u32 {
    require(bytes.len() == 4, b"insufficient data for u32");
    let mut array = [0u8; 4];
    array.copy_from_slice(&bytes[0..4]);
    u32::from_le_bytes(array)
}
